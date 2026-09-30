//! Cycle-based event scheduler. `now` is the global cycle counter; every bus access advances it.

use std::cmp::Reverse;
use std::collections::BinaryHeap;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Event {
    /// Cycle `HDRAW_CYCLES` of every line: render the scanline, set H-Blank flag, H-Blank IRQ/DMA.
    HBlank,
    /// Cycle `CYCLES_PER_LINE`: VCOUNT++, V-Blank/V-Count logic, clear H-Blank flag, reschedule.
    LineEnd,
    /// Timer `n` reached 0x10000.
    TimerOverflow(u8),
    /// Immediate-mode DMA start latency (2 cycles after the enable write).
    DmaStart(u8),
    /// Emit one stereo output sample.
    ApuSample,
    /// PSG frame sequencer tick (length/envelope/sweep), 512 Hz.
    ApuFrameSeq,
    /// Re-evaluate the keypad IRQ condition.
    KeypadPoll,
    /// One-instruction IRQ line latency, if a test demands it.
    IrqLatch,
}

#[derive(Debug, Default)]
pub struct Scheduler {
    pub now: u64,
    seq: u32,
    heap: BinaryHeap<Reverse<(u64, u32, Event)>>,
}

impl Scheduler {
    pub fn new() -> Self {
        Self::default()
    }

    /// Schedule `ev` at absolute cycle `at`. Events at equal times fire in insertion order.
    pub fn schedule(&mut self, ev: Event, at: u64) {
        self.seq = self.seq.wrapping_add(1);
        self.heap.push(Reverse((at, self.seq, ev)));
    }

    /// Schedule `ev` `dt` cycles from now.
    pub fn schedule_in(&mut self, ev: Event, dt: u64) {
        self.schedule(ev, self.now + dt);
    }

    /// Remove every pending instance of `ev`. O(n); the heap holds ~10 entries.
    pub fn cancel(&mut self, ev: Event) {
        if self.heap.iter().any(|Reverse((_, _, e))| *e == ev) {
            let kept: Vec<_> = self.heap.drain().filter(|Reverse((_, _, e))| *e != ev).collect();
            self.heap = kept.into_iter().collect();
        }
    }

    /// Absolute time of the earliest pending event, or `u64::MAX` if none.
    pub fn next_deadline(&self) -> u64 {
        self.heap.peek().map(|Reverse((at, _, _))| *at).unwrap_or(u64::MAX)
    }

    /// Pop the earliest event if it is due (`at <= now`). Returns the event and its scheduled time.
    pub fn pop_due(&mut self) -> Option<(Event, u64)> {
        match self.heap.peek() {
            Some(Reverse((at, _, _))) if *at <= self.now => {
                let Reverse((at, _, ev)) = self.heap.pop().unwrap();
                Some((ev, at))
            }
            _ => None,
        }
    }

    /// Time at which `ev` is next scheduled, if pending.
    pub fn time_of(&self, ev: Event) -> Option<u64> {
        self.heap.iter().filter(|Reverse((_, _, e))| *e == ev).map(|Reverse((at, _, _))| *at).min()
    }

    pub fn len(&self) -> usize {
        self.heap.len()
    }

    pub fn is_empty(&self) -> bool {
        self.heap.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fires_in_time_then_insertion_order() {
        let mut s = Scheduler::new();
        s.schedule(Event::LineEnd, 10);
        s.schedule(Event::HBlank, 5);
        s.schedule(Event::TimerOverflow(0), 10);
        assert_eq!(s.next_deadline(), 5);
        assert!(s.pop_due().is_none());
        s.now = 10;
        assert_eq!(s.pop_due().unwrap().0, Event::HBlank);
        assert_eq!(s.pop_due().unwrap().0, Event::LineEnd);
        assert_eq!(s.pop_due().unwrap().0, Event::TimerOverflow(0));
        assert!(s.pop_due().is_none());
    }

    #[test]
    fn cancel_removes_all_instances() {
        let mut s = Scheduler::new();
        s.schedule(Event::TimerOverflow(1), 3);
        s.schedule(Event::TimerOverflow(1), 7);
        s.schedule(Event::HBlank, 4);
        s.cancel(Event::TimerOverflow(1));
        assert_eq!(s.len(), 1);
        assert_eq!(s.time_of(Event::HBlank), Some(4));
        assert_eq!(s.time_of(Event::TimerOverflow(1)), None);
    }
}
