//! Timers 0-3, event-driven. Source: GBATEK "GBA Timers".
//!
//! A running non-cascade timer has one `Event::TimerOverflow(n)` scheduled at the cycle it will
//! reach 0x10000; reads derive the counter from the current time. Cascade timers have no event
//! and are incremented by the previous timer's overflow handler.

use crate::irq::{Irq, IrqController};
use crate::sched::{Event, Scheduler};

#[derive(Clone, Copy, Debug, Default)]
pub struct Timer {
    /// TMxCNT_L write value.
    pub reload: u16,
    /// TMxCNT_H.
    pub control: u16,
    /// Counter value at `start_time` (for running timers) or the current counter (stopped/cascade).
    pub counter: u16,
    /// Cycle at which `counter` was valid.
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
}

#[derive(Clone, Debug, Default)]
pub struct Timers {
    pub t: [Timer; 4],
}

impl Timers {
    pub fn new() -> Self {
        Self::default()
    }

    /// Current counter value of timer `n` at `now`.
    pub fn counter(&self, n: usize, now: u64) -> u16 {
        let t = &self.t[n];
        if t.enabled() && !t.cascade() {
            let ticks = now.saturating_sub(t.start_time) / t.prescale();
            t.counter.wrapping_add(ticks as u16)
        } else {
            t.counter
        }
    }

    fn schedule_overflow(&mut self, n: usize, sched: &mut Scheduler) {
        let t = &self.t[n];
        let ticks_left = 0x1_0000 - t.counter as u64;
        sched.schedule(Event::TimerOverflow(n as u8), t.start_time + ticks_left * t.prescale());
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
            0 => self.t[n].reload = (self.t[n].reload & 0xFF00) | v as u16,
            1 => self.t[n].reload = (self.t[n].reload & 0x00FF) | ((v as u16) << 8),
            2 => {
                let old = self.t[n];
                let new_control = v as u16 & 0xC7;
                // Freeze the current count before changing anything.
                let cur = self.counter(n, sched.now);
                sched.cancel(Event::TimerOverflow(n as u8));
                let t = &mut self.t[n];
                t.control = new_control;
                t.counter = cur;
                t.start_time = sched.now;
                if t.enabled() {
                    if !old.enabled() {
                        // 0->1: reload. GBATEK: the counter starts one cycle later.
                        t.counter = t.reload;
                        t.start_time = sched.now + 1;
                    }
                    if !t.cascade() {
                        self.schedule_overflow(n, sched);
                    }
                }
            }
            _ => {}
        }
    }

    /// Timer `n` overflowed at its scheduled time: reload, raise IRQ, cascade, reschedule.
    pub fn on_overflow(&mut self, n: u8, sched: &mut Scheduler, irq: &mut IrqController) {
        let n = n as usize;
        let at = self.t[n].start_time + (0x1_0000 - self.t[n].counter as u64) * self.t[n].prescale();
        self.overflow_at(n, at, sched, irq);
    }

    fn overflow_at(&mut self, n: usize, at: u64, sched: &mut Scheduler, irq: &mut IrqController) {
        {
            let t = &mut self.t[n];
            t.counter = t.reload;
            t.start_time = at;
            if t.irq_enabled() {
                irq.raise(Irq::timer(n as u8));
            }
        }
        if !self.t[n].cascade() && self.t[n].enabled() {
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

    #[test]
    fn counts_with_prescaler_and_overflows() {
        let mut s = Scheduler::new();
        let mut irq = IrqController::new();
        let mut tm = Timers::new();
        tm.write8(0x100, 0xFE, &mut s);
        tm.write8(0x101, 0xFF, &mut s); // reload 0xFFFE
        tm.write8(0x102, 0xC0, &mut s); // enable, irq, prescale 1
        assert_eq!(tm.counter(0, 1), 0xFFFE);
        assert_eq!(tm.counter(0, 2), 0xFFFF);
        let at = s.time_of(Event::TimerOverflow(0)).unwrap();
        assert_eq!(at, 3);
        s.now = at;
        let (ev, _) = s.pop_due().unwrap();
        assert_eq!(ev, Event::TimerOverflow(0));
        tm.on_overflow(0, &mut s, &mut irq);
        assert_eq!(irq.if_, Irq::Timer0 as u16);
        assert_eq!(tm.counter(0, 3), 0xFFFE);
        assert_eq!(s.time_of(Event::TimerOverflow(0)), Some(5));
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
        tm.write8(0x102, 0x80, &mut s); // timer0 overflows after 1 tick
        s.now = 2;
        s.pop_due().unwrap();
        tm.on_overflow(0, &mut s, &mut irq);
        assert_eq!(tm.counter(1, 2), 1);
    }
}
