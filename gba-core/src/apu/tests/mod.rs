//! APU unit tests. Each file drives an `Apu` directly through `write_io8` and the event
//! entry points (`on_frame_seq`, `on_sample`, `on_timer_overflow`).

mod fifo;
mod mixer;
mod noise;
mod regs;
mod square;
mod wave;

use super::Apu;
use crate::sched::Scheduler;

/// A fresh APU with the master enable set, plus the scheduler `write_io8` wants.
pub fn apu_on() -> (Apu, Scheduler) {
    let mut a = Apu::new();
    let mut s = Scheduler::new();
    a.write_io8(0x84, 0x80, &mut s);
    (a, s)
}

pub fn w8(a: &mut Apu, s: &mut Scheduler, off: u16, v: u8) {
    a.write_io8(off, v, s);
}

/// 16-bit write as the bus performs it: low byte then high byte.
pub fn w16(a: &mut Apu, s: &mut Scheduler, off: u16, v: u16) {
    a.write_io8(off, v as u8, s);
    a.write_io8(off + 1, (v >> 8) as u8, s);
}

pub fn r16(a: &Apu, off: u16) -> u16 {
    a.reg16(off as usize)
}

/// Run the 512 Hz frame sequencer `n` times.
pub fn frames(a: &mut Apu, n: usize) {
    for _ in 0..n {
        a.on_frame_seq();
    }
}

/// SOUNDCNT_X bits 0-3.
pub fn status(a: &Apu) -> u8 {
    a.read_io8(0x84) & 0x0F
}
