//! Timers 0-3, event-driven. Source: GBATEK "GBA Timers"; cycle-level behaviour pinned by the
//! mGBA test suite ("Timer count-up tests", "Timer IRQ tests"), which are hardware-recorded.
//!
//! A running non-cascade timer has one `Event::TimerOverflow(n)` scheduled at the cycle it will
//! reach 0x10000; reads derive the counter from the current time. Cascade timers have no event
//! and are incremented by the previous timer's overflow handler.
//!
//! Timeline conventions (`now` is `Scheduler::now`, advanced by every bus access, so a CPU store
//! "lands" at the `now` reached by its data cycle):
//! - The prescaler is a free-running divider of the system clock: a timer ticks at every cycle
//!   `t > start_time` with `t % prescale == 0`; the phase is not reset when a timer starts.
//!   (GBATEK only gives the divisors; the count-up tests' 1d/2d/4d variants, whose results shift
//!   with the start cycle modulo the prescaler, pin the free-running behaviour.)
//! - A 0->1 write of the start bit at cycle `W` loads the counter from the reload register and
//!   the timer starts counting at `W + START_DELAY`: with prescaler 1 the first increment is
//!   visible to a read at `W + 3`.
//! - A write to TMxCNT_L at cycle `W` is not seen by an overflow at `W + 1`: both happen on the
//!   same hardware clock edge, so that overflow still loads the previous reload value
//!   (Timer IRQ test "FFFF": start with reload FFFF, immediately write reload 0, and the first
//!   overflow reloads FFFF, i.e. two overflows fire one cycle apart).
//! - A timer overflow at cycle `t` becomes visible to the CPU's IRQ input at `t + IRQ_DELAY`;
//!   IF reads see it immediately (see `IrqController::raise_at`).
//! - A control write at cycle `W` takes effect after the tick at `W`. If an overflow is due at
//!   `W` (or earlier) but its event has not been dispatched yet, the write is deferred until the
//!   event handler runs, so the overflow (IRQ, cascade, FIFO hook) is not lost.

use crate::irq::{Irq, IrqController};
use crate::sched::{Event, Scheduler};

/// Cycles between the start-bit write and the first counted clock.
pub const START_DELAY: u64 = 2;
/// Cycles between an overflow and the CPU seeing the IRQ request.
pub const IRQ_DELAY: u64 = 3;
/// A TMxCNT_L write at `W` is used by overflows at `W + RELOAD_VISIBLE` or later.
const RELOAD_VISIBLE: u64 = 2;

#[cfg_attr(feature = "savestate", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Copy, Debug, Default)]
pub struct Timer {
    /// TMxCNT_L write value.
    pub reload: u16,
    /// The reload value before the most recent TMxCNT_L write, still used by an overflow on the
    /// same clock edge as that write.
    pub reload_prev: u16,
    /// Cycle of the most recent TMxCNT_L write.
    pub reload_written_at: u64,
    /// TMxCNT_H.
    pub control: u16,
    /// Counter value at `start_time` (for running timers) or the current counter (stopped/cascade).
    pub counter: u16,
    /// Cycle at which `counter` was valid; ticks are counted at cycles strictly after it.
    pub start_time: u64,
}

impl Timer {
    #[inline]
    pub fn enabled(&self) -> bool {
        self.control & 0x80 != 0
    }
    #[inline]
    pub fn cascade(&self) -> bool {
        self.control & 0x04 != 0
    }
    #[inline]
    pub fn irq_enabled(&self) -> bool {
        self.control & 0x40 != 0
    }
    /// Prescaler: 1, 64, 256, 1024 cycles per tick.
    #[inline]
    pub fn prescale(&self) -> u64 {
        match self.control & 3 {
            0 => 1,
            1 => 64,
            2 => 256,
            _ => 1024,
        }
    }
    /// Free-running (prescaler-driven) in timer slot `n`. TM0CNT_H bit 2 is "not used": timer 0
    /// has no predecessor to cascade from, so it always counts on its prescaler (several games,
    /// e.g. Who Wants to Be a Millionaire, set TM0CNT_H = 0x84 for their sample timer).
    #[inline]
    fn running(&self, n: usize) -> bool {
        self.enabled() && !(n > 0 && self.cascade())
    }
    /// Ticks counted in `(start_time, now]` (free-running prescaler).
    #[inline]
    fn ticks_until(&self, now: u64) -> u64 {
        let p = self.prescale();
        (now / p).saturating_sub(self.start_time / p)
    }
    /// Cycle of the `k`-th tick after `start_time` (k >= 1).
    #[inline]
    fn tick_time(&self, k: u64) -> u64 {
        let p = self.prescale();
        (self.start_time / p + k) * p
    }
    /// Cycle at which the running timer reaches 0x10000.
    #[inline]
    fn overflow_time(&self) -> u64 {
        self.tick_time(0x1_0000 - self.counter as u64)
    }
    /// Reload value loaded by an overflow at cycle `at`.
    #[inline]
    fn reload_for(&self, at: u64) -> u16 {
        if at < self.reload_written_at + RELOAD_VISIBLE {
            self.reload_prev
        } else {
            self.reload
        }
    }
}

#[cfg_attr(feature = "savestate", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, Default)]
pub struct Timers {
    pub t: [Timer; 4],
    /// TMxCNT_H writes that landed at or after a not-yet-dispatched overflow; applied by
    /// `on_overflow` once that overflow has been processed.
    deferred_control: [Option<u8>; 4],
}

impl Timers {
    pub fn new() -> Self {
        Self::default()
    }

    /// Current counter value of timer `n` at `now`. Accounts for overflows that are due but whose
    /// event has not been dispatched yet (reads inside the instruction that crossed them).
    pub fn counter(&self, n: usize, now: u64) -> u16 {
        let t = &self.t[n];
        if !t.running(n) {
            return t.counter;
        }
        let ticks = t.ticks_until(now);
        let mut base = t.counter as u64;
        let mut done = 0u64;
        loop {
            let to_overflow = 0x1_0000 - base;
            if ticks - done < to_overflow {
                return (base + ticks - done) as u16;
            }
            done += to_overflow;
            let at = t.tick_time(done);
            base = t.reload_for(at) as u64;
            if at >= t.reload_written_at + RELOAD_VISIBLE {
                // Every later overflow reloads the same value: close the form.
                let period = 0x1_0000 - base;
                return (base + (ticks - done) % period) as u16;
            }
        }
    }

    fn schedule_overflow(&mut self, n: usize, sched: &mut Scheduler) {
        let at = self.t[n].overflow_time();
        sched.schedule(Event::TimerOverflow(n as u8), at);
    }

    pub fn read8(&self, off: u16, now: u64) -> u8 {
        if !(0x100..0x110).contains(&off) {
            return 0;
        }
        let n = ((off - 0x100) / 4) as usize;
        match (off - 0x100) % 4 {
            0 => self.counter(n, now) as u8,
            1 => (self.counter(n, now) >> 8) as u8,
            2 => self.t[n].control as u8,
            _ => 0,
        }
    }

    pub fn write8(&mut self, off: u16, v: u8, sched: &mut Scheduler) {
        if !(0x100..0x110).contains(&off) {
            return;
        }
        let n = ((off - 0x100) / 4) as usize;
        match (off - 0x100) % 4 {
            0 => self.write_reload(n, (self.t[n].reload & 0xFF00) | v as u16, sched.now),
            1 => self.write_reload(n, (self.t[n].reload & 0x00FF) | ((v as u16) << 8), sched.now),
            2 => {
                // An overflow due at or before this write must be processed first (its tick
                // precedes the write on the same edge); let the event handler apply the write.
                if self.t[n].running(n) {
                    if let Some(at) = sched.time_of(Event::TimerOverflow(n as u8)) {
                        if at <= sched.now {
                            self.deferred_control[n] = Some(v);
                            return;
                        }
                    }
                }
                self.write_control(n, v, sched);
            }
            _ => {}
        }
    }

    fn write_reload(&mut self, n: usize, reload: u16, now: u64) {
        let t = &mut self.t[n];
        // Two writes on the same edge (a 16-bit write dispatched as two bytes) collapse into one.
        if t.reload_written_at != now {
            t.reload_prev = t.reload;
            t.reload_written_at = now;
        }
        t.reload = reload;
    }

    fn write_control(&mut self, n: usize, v: u8, sched: &mut Scheduler) {
        let old = self.t[n];
        let new_control = v as u16 & 0xC7;
        // Freeze the current count (including a tick on this cycle) before changing anything.
        let cur = self.counter(n, sched.now);
        sched.cancel(Event::TimerOverflow(n as u8));
        let t = &mut self.t[n];
        t.control = new_control;
        t.counter = cur;
        t.start_time = sched.now;
        if t.enabled() {
            if !old.enabled() {
                // 0->1: load the reload value (a 32-bit write that sets both halves uses the
                // value it just wrote, GBATEK) and start counting START_DELAY cycles later.
                t.counter = t.reload;
                t.start_time = sched.now + START_DELAY;
            }
            if t.running(n) {
                self.schedule_overflow(n, sched);
            }
        }
    }

    /// Timer `n` overflowed at its scheduled time: reload, raise IRQ, cascade, reschedule.
    pub fn on_overflow(&mut self, n: u8, sched: &mut Scheduler, irq: &mut IrqController) {
        let n = n as usize;
        let at = self.t[n].overflow_time();
        self.overflow_at(n, at, sched, irq);
        if let Some(v) = self.deferred_control[n].take() {
            self.write8(0x102 + 4 * n as u16, v, sched);
        }
    }

    fn overflow_at(&mut self, n: usize, at: u64, sched: &mut Scheduler, irq: &mut IrqController) {
        {
            let t = &mut self.t[n];
            t.counter = t.reload_for(at);
            t.start_time = at;
            if t.irq_enabled() {
                irq.raise_at(Irq::timer(n as u8), at + IRQ_DELAY, sched);
            }
        }
        if self.t[n].running(n) {
            self.schedule_overflow(n, sched);
        }
        // Cascade into the next timer.
        if n < 3 {
            let next = &mut self.t[n + 1];
            if next.enabled() && next.cascade() {
                let (c, wrapped) = next.counter.overflowing_add(1);
                next.counter = c;
                if wrapped {
                    self.overflow_at(n + 1, at, sched, irq);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fire(tm: &mut Timers, s: &mut Scheduler, irq: &mut IrqController) -> u64 {
        let (ev, at) = s.pop_due().expect("an event is due");
        match ev {
            Event::TimerOverflow(n) => tm.on_overflow(n, s, irq),
            Event::IrqLatch => {}
            other => panic!("unexpected event {other:?}"),
        }
        at
    }

    /// Enable write at `now`: the counter starts START_DELAY cycles later, ticks every cycle
    /// after that, overflows at start + ticks_left and reloads.
    #[test]
    fn counts_with_prescaler_and_overflows() {
        let mut s = Scheduler::new();
        let mut irq = IrqController::new();
        let mut tm = Timers::new();
        s.now = 10;
        tm.write8(0x100, 0xFE, &mut s);
        tm.write8(0x101, 0xFF, &mut s); // reload 0xFFFE
        tm.write8(0x102, 0xC0, &mut s); // enable, irq, prescale 1
        assert_eq!(tm.counter(0, 10), 0xFFFE);
        assert_eq!(tm.counter(0, 12), 0xFFFE, "no tick before start + 1");
        assert_eq!(tm.counter(0, 13), 0xFFFF);
        let at = s.time_of(Event::TimerOverflow(0)).unwrap();
        assert_eq!(at, 14);
        assert_eq!(tm.counter(0, 14), 0xFFFE, "reads past a due overflow already show the reload");
        s.now = at;
        let fired = fire(&mut tm, &mut s, &mut irq);
        assert_eq!(fired, 14);
        assert_eq!(irq.if_, Irq::Timer0 as u16, "IF is set immediately");
        assert_eq!(tm.counter(0, 14), 0xFFFE);
        assert_eq!(s.time_of(Event::TimerOverflow(0)), Some(16));
    }

    /// Timer IRQ test "FFFC": `str reload|start` at E, `strh reload=0` at E+2, `ldrh` at E+4+N.
    #[test]
    fn timer_irq_suite_fffc_timeline() {
        let mut s = Scheduler::new();
        let mut irq = IrqController::new();
        irq.ie = Irq::Timer0 as u16;
        irq.ime = true;
        let mut tm = Timers::new();
        let e = 1000;
        s.now = e;
        tm.write8(0x100, 0xFC, &mut s);
        tm.write8(0x101, 0xFF, &mut s);
        tm.write8(0x102, 0xC0, &mut s);
        s.now = e + 2;
        tm.write8(0x100, 0x00, &mut s);
        tm.write8(0x101, 0x00, &mut s);
        // The ldrh after N nops is checked for a pending IRQ at E+2+N and reads at E+4+N.
        // Reads with 0..=6 nops give FFFE FFFF 0000 0001 0002 0003 0004; the overflow at E+6
        // reaches the CPU at E+9, so the ldrh after 7 nops is pre-empted.
        let values = [0xFFFEu16, 0xFFFF, 0, 1, 2, 3, 4];
        for n in 0..=7u64 {
            let check = e + 2 + n;
            s.now = check;
            while s.next_deadline() <= check {
                fire(&mut tm, &mut s, &mut irq);
            }
            irq.sync(check);
            assert_eq!(irq.pending(), n >= 7, "IRQ check before the ldrh after {n} nops");
            if n < 7 {
                let read = e + 4 + n;
                s.now = read;
                assert_eq!(tm.counter(0, read), values[n as usize], "{n} nops");
            }
        }
    }

    /// Timer IRQ test "FFFF": the reload write two cycles after the start lands on the same edge
    /// as the first overflow, which therefore reloads FFFF; the next cycle overflows again to 0.
    #[test]
    fn reload_write_on_overflow_edge_uses_previous_value() {
        let mut s = Scheduler::new();
        let mut irq = IrqController::new();
        let mut tm = Timers::new();
        let e = 500;
        s.now = e;
        tm.write8(0x100, 0xFF, &mut s);
        tm.write8(0x101, 0xFF, &mut s);
        tm.write8(0x102, 0xC0, &mut s);
        s.now = e + 2;
        tm.write8(0x100, 0x00, &mut s);
        tm.write8(0x101, 0x00, &mut s);
        // Reads before the events are dispatched already follow both overflows.
        assert_eq!(tm.counter(0, e + 3), 0xFFFF);
        assert_eq!(tm.counter(0, e + 4), 0x0000, "stale read: second overflow used the new reload");
        assert_eq!(tm.counter(0, e + 5), 0x0001);
        s.now = e + 3;
        assert_eq!(fire(&mut tm, &mut s, &mut irq), e + 3);
        assert_eq!(tm.counter(0, e + 3), 0xFFFF, "first overflow reloads the old value");
        s.now = e + 4;
        assert_eq!(fire(&mut tm, &mut s, &mut irq), e + 4);
        assert_eq!(tm.counter(0, e + 4), 0x0000);
        assert_eq!(tm.counter(0, e + 5), 0x0001);
        assert_eq!(tm.counter(0, e + 86), 0x0052);
    }

    /// With reload FFFF every tick overflows: the derived value never wraps to 0000/0001 even
    /// when the read happens after the scheduled overflow but before the event is dispatched.
    #[test]
    fn stale_reads_wrap_with_the_reload_period() {
        let mut s = Scheduler::new();
        let mut tm = Timers::new();
        s.now = 0;
        tm.write8(0x100, 0xFF, &mut s);
        tm.write8(0x101, 0xFF, &mut s);
        tm.write8(0x102, 0x80, &mut s);
        for now in 0..40 {
            assert_eq!(tm.counter(0, now), 0xFFFF, "now={now}");
        }
        // Reload FFF0 (period 16): value cycles FFF0..FFFF.
        let mut tm = Timers::new();
        tm.write8(0x100, 0xF0, &mut s);
        tm.write8(0x101, 0xFF, &mut s);
        tm.write8(0x102, 0x80, &mut s);
        assert_eq!(tm.counter(0, 2 + 16), 0xFFF0);
        assert_eq!(tm.counter(0, 2 + 17), 0xFFF1);
        assert_eq!(tm.counter(0, 2 + 33), 0xFFF1);
    }

    /// The prescaler divides the free-running system clock: with prescale 64 a timer started at
    /// cycle 100 ticks at 128, 192, ... regardless of the start phase.
    #[test]
    fn prescaler_is_free_running() {
        let mut s = Scheduler::new();
        let mut tm = Timers::new();
        s.now = 100;
        tm.write8(0x100, 0x00, &mut s);
        tm.write8(0x101, 0x00, &mut s);
        tm.write8(0x102, 0x81, &mut s); // enable, prescale 64
        assert_eq!(tm.counter(0, 127), 0);
        assert_eq!(tm.counter(0, 128), 1);
        assert_eq!(tm.counter(0, 191), 1);
        assert_eq!(tm.counter(0, 192), 2);
        assert_eq!(s.time_of(Event::TimerOverflow(0)), Some(128 + 0xFFFF * 64));

        // Started exactly on a tick boundary: the start cycle itself does not count (the timer
        // is not running until START_DELAY later).
        let mut tm = Timers::new();
        s.now = 128;
        tm.write8(0x102, 0x81, &mut s);
        assert_eq!(tm.counter(0, 191), 0);
        assert_eq!(tm.counter(0, 192), 1);
        // Started so that START_DELAY lands exactly on a boundary: that boundary does not count.
        let mut tm = Timers::new();
        s.now = 126;
        tm.write8(0x102, 0x81, &mut s);
        assert_eq!(tm.counter(0, 128), 0);
        assert_eq!(tm.counter(0, 192), 1);
    }

    /// Stopping a timer freezes the counter, including a tick on the write cycle.
    #[test]
    fn stop_freezes_counter() {
        let mut s = Scheduler::new();
        let mut tm = Timers::new();
        s.now = 0;
        tm.write8(0x102, 0x80, &mut s);
        s.now = 50;
        tm.write8(0x102, 0x00, &mut s);
        assert_eq!(tm.counter(0, 50), 48);
        assert_eq!(tm.counter(0, 5000), 48);
        assert_eq!(s.time_of(Event::TimerOverflow(0)), None);
    }

    /// A stop write on the cycle of a due-but-undispatched overflow is applied after that
    /// overflow: the IRQ still fires and the frozen counter is the reload value.
    #[test]
    fn control_write_after_due_overflow_is_deferred() {
        let mut s = Scheduler::new();
        let mut irq = IrqController::new();
        let mut tm = Timers::new();
        s.now = 0;
        tm.write8(0x100, 0xFE, &mut s);
        tm.write8(0x101, 0xFF, &mut s);
        tm.write8(0x102, 0xC0, &mut s); // overflow at 4
        s.now = 4;
        tm.write8(0x102, 0x00, &mut s);
        assert_eq!(tm.t[0].control, 0xC0, "write deferred until the overflow is dispatched");
        assert_eq!(fire(&mut tm, &mut s, &mut irq), 4);
        assert_eq!(irq.if_, Irq::Timer0 as u16);
        assert_eq!(tm.t[0].control, 0x00);
        assert_eq!(tm.counter(0, 4), 0xFFFE);
        assert_eq!(tm.counter(0, 400), 0xFFFE);
        assert_eq!(s.time_of(Event::TimerOverflow(0)), None);
    }

    #[test]
    fn cascade_increments_on_overflow() {
        let mut s = Scheduler::new();
        let mut irq = IrqController::new();
        let mut tm = Timers::new();
        tm.write8(0x104, 0x00, &mut s);
        tm.write8(0x105, 0x00, &mut s);
        tm.write8(0x106, 0x84, &mut s); // timer1: enabled, cascade
        tm.write8(0x100, 0xFF, &mut s);
        tm.write8(0x101, 0xFF, &mut s);
        tm.write8(0x102, 0x80, &mut s); // timer0 overflows after 1 tick, at cycle 3
        s.now = 3;
        assert_eq!(fire(&mut tm, &mut s, &mut irq), 3);
        assert_eq!(tm.counter(1, 3), 1);
    }

    /// TM0CNT_H bit 2 (count-up) is "not used" for timer 0: it still counts on its prescaler.
    /// Who Wants to Be a Millionaire (and others) program TM0CNT_H = 0x84 for the FIFO timer.
    #[test]
    fn timer0_ignores_the_count_up_bit() {
        let mut s = Scheduler::new();
        let mut irq = IrqController::new();
        let mut tm = Timers::new();
        tm.write8(0x100, 0xFF, &mut s);
        tm.write8(0x101, 0xFF, &mut s);
        tm.write8(0x102, 0x84, &mut s); // timer0: enabled, count-up bit set, prescaler F/1
        assert!(s.time_of(Event::TimerOverflow(0)).is_some(), "overflow scheduled");
        s.now = 3;
        assert_eq!(fire(&mut tm, &mut s, &mut irq), 3);
        assert_eq!(tm.counter(0, 3), 0xFFFF, "reloaded after the overflow");
        // The same bit on timer 1 does cascade (no own overflow event).
        tm.write8(0x106, 0x84, &mut s);
        assert!(s.time_of(Event::TimerOverflow(1)).is_none());
    }

    /// The overflow IRQ reaches the CPU IRQ_DELAY cycles after the overflow and is dropped if IF
    /// is acknowledged before then.
    #[test]
    fn overflow_irq_is_delayed_and_ack_cancels_it() {
        let mut s = Scheduler::new();
        let mut irq = IrqController::new();
        irq.ie = Irq::Timer0 as u16;
        irq.ime = true;
        let mut tm = Timers::new();
        s.now = 0;
        tm.write8(0x100, 0xFF, &mut s);
        tm.write8(0x101, 0xFF, &mut s);
        tm.write8(0x102, 0xC0, &mut s);
        s.now = 3;
        fire(&mut tm, &mut s, &mut irq);
        assert_eq!(irq.if_, Irq::Timer0 as u16);
        irq.sync(3 + IRQ_DELAY - 1);
        assert!(!irq.pending());
        assert!(!irq.wake_condition());
        assert_eq!(s.time_of(Event::IrqLatch), Some(3 + IRQ_DELAY), "scheduler wakes a halted CPU");
        irq.sync(3 + IRQ_DELAY);
        assert!(irq.pending());
        assert!(irq.wake_condition());

        // Acknowledge before the line asserts: nothing reaches the CPU.
        let mut irq = IrqController::new();
        irq.ie = Irq::Timer0 as u16;
        irq.ime = true;
        irq.raise_at(Irq::Timer0, 20, &mut s);
        irq.write_if_ack(Irq::Timer0 as u16);
        irq.sync(100);
        assert_eq!(irq.if_, 0);
        assert!(!irq.pending());
    }
}
