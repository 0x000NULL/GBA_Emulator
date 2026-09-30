//! IE / IF / IME / HALTCNT. Source: GBATEK "GBA Interrupt Control".
//!
//! `if_` is the register the CPU reads; `line` is the subset of it currently asserted on the
//! CPU's IRQ input. Most sources set both at once (`raise`). A source that knows its request
//! reaches the CPU a few cycles after the flag is set uses `raise_at`, and `sync` moves matured
//! requests onto the line; `Gba::run_until` calls it before every IRQ check. Acknowledging IF
//! drops the request from both, whether or not it has reached the line yet.

use crate::sched::{Event, Scheduler};

#[repr(u16)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Irq {
    VBlank = 1 << 0,
    HBlank = 1 << 1,
    VCount = 1 << 2,
    Timer0 = 1 << 3,
    Timer1 = 1 << 4,
    Timer2 = 1 << 5,
    Timer3 = 1 << 6,
    Serial = 1 << 7,
    Dma0 = 1 << 8,
    Dma1 = 1 << 9,
    Dma2 = 1 << 10,
    Dma3 = 1 << 11,
    Keypad = 1 << 12,
    GamePak = 1 << 13,
}

impl Irq {
    pub fn timer(n: u8) -> Irq {
        match n & 3 {
            0 => Irq::Timer0,
            1 => Irq::Timer1,
            2 => Irq::Timer2,
            _ => Irq::Timer3,
        }
    }
    pub fn dma(n: u8) -> Irq {
        match n & 3 {
            0 => Irq::Dma0,
            1 => Irq::Dma1,
            2 => Irq::Dma2,
            _ => Irq::Dma3,
        }
    }
}

#[cfg_attr(feature = "savestate", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Default, Clone)]
pub struct IrqController {
    /// 4000200h IE
    pub ie: u16,
    /// 4000202h IF (write 1 to acknowledge)
    pub if_: u16,
    /// 4000208h IME bit 0
    pub ime: bool,
    /// HALTCNT: CPU halted until (IE & IF) != 0 (IME not required).
    pub halted: bool,
    /// HALTCNT bit 7: stop mode (until keypad/serial/gamepak IRQ).
    pub stopped: bool,
    /// IF bits asserted on the CPU's IRQ input (subset of `if_`).
    line: u16,
    /// Requests set in `if_` that reach `line` at the given cycle. One entry per bit.
    delayed: Vec<(u16, u64)>,
}

impl IrqController {
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the IF bit and assert it on the CPU immediately.
    #[inline]
    pub fn raise(&mut self, irq: Irq) {
        self.if_ |= irq as u16;
        self.line |= irq as u16;
        if !self.delayed.is_empty() {
            self.delayed.retain(|&(b, _)| b != irq as u16);
        }
    }

    /// Set the IF bit now; the CPU sees the request from cycle `at` (an `IrqLatch` event wakes
    /// the main loop then). A request already asserted or already queued is left alone.
    pub fn raise_at(&mut self, irq: Irq, at: u64, sched: &mut Scheduler) {
        let b = irq as u16;
        self.if_ |= b;
        if self.line & b != 0 || self.delayed.iter().any(|&(d, _)| d == b) {
            return;
        }
        self.delayed.push((b, at));
        sched.schedule(Event::IrqLatch, at);
    }

    /// Move requests whose time has come onto the CPU line.
    #[inline]
    pub fn sync(&mut self, now: u64) {
        if self.delayed.is_empty() {
            return;
        }
        let mut line = self.line;
        self.delayed.retain(|&(b, at)| {
            if at <= now {
                line |= b;
                false
            } else {
                true
            }
        });
        self.line = line;
    }

    /// True when the CPU should take an IRQ exception (CPSR.I is the CPU's business).
    #[inline]
    pub fn pending(&self) -> bool {
        self.ime && (self.ie & self.line) != 0
    }

    /// True when a halted CPU must wake (IME is ignored for wake-up).
    #[inline]
    pub fn wake_condition(&self) -> bool {
        (self.ie & self.line) != 0
    }

    pub fn write_if_ack(&mut self, v: u16) {
        self.if_ &= !v;
        self.line &= !v;
        if !self.delayed.is_empty() {
            self.delayed.retain(|&(b, _)| b & v == 0);
        }
    }

    /// 4000301h HALTCNT: bit 7 = 0 halt, 1 stop.
    pub fn write_haltcnt(&mut self, v: u8) {
        if v & 0x80 != 0 {
            self.stopped = true;
        } else {
            self.halted = true;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn raise_is_immediate() {
        let mut irq = IrqController::new();
        irq.ie = Irq::VBlank as u16;
        irq.raise(Irq::VBlank);
        assert_eq!(irq.if_, Irq::VBlank as u16);
        assert!(irq.wake_condition());
        assert!(!irq.pending(), "IME off");
        irq.ime = true;
        assert!(irq.pending());
        irq.write_if_ack(Irq::VBlank as u16);
        assert_eq!(irq.if_, 0);
        assert!(!irq.pending());
    }

    #[test]
    fn raise_at_sets_if_now_and_line_later() {
        let mut s = Scheduler::new();
        let mut irq = IrqController::new();
        irq.ie = Irq::Timer1 as u16;
        irq.ime = true;
        irq.raise_at(Irq::Timer1, 10, &mut s);
        assert_eq!(irq.if_, Irq::Timer1 as u16);
        irq.sync(9);
        assert!(!irq.pending());
        assert!(!irq.wake_condition());
        assert_eq!(s.time_of(Event::IrqLatch), Some(10));
        irq.sync(10);
        assert!(irq.pending());
        assert!(irq.wake_condition());
        // A second request for an asserted bit is a no-op.
        irq.raise_at(Irq::Timer1, 20, &mut s);
        irq.write_if_ack(Irq::Timer1 as u16);
        irq.sync(100);
        assert!(!irq.pending());
    }

    #[test]
    fn ack_before_the_line_asserts_drops_the_request() {
        let mut s = Scheduler::new();
        let mut irq = IrqController::new();
        irq.ie = 0xFFFF;
        irq.ime = true;
        irq.raise_at(Irq::Timer0, 10, &mut s);
        irq.raise_at(Irq::Timer2, 12, &mut s);
        irq.write_if_ack(Irq::Timer0 as u16);
        irq.sync(11);
        assert!(!irq.pending());
        irq.sync(12);
        assert!(irq.pending());
        assert_eq!(irq.if_, Irq::Timer2 as u16);
    }

    #[test]
    fn immediate_raise_overrides_a_queued_one() {
        let mut s = Scheduler::new();
        let mut irq = IrqController::new();
        irq.ie = 0xFFFF;
        irq.raise_at(Irq::Timer0, 10, &mut s);
        irq.raise(Irq::Timer0);
        assert!(irq.wake_condition());
        irq.write_if_ack(Irq::Timer0 as u16);
        irq.sync(10);
        assert!(!irq.wake_condition(), "the queued copy was dropped with the raise");
    }
}
