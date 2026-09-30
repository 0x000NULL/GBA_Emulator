//! Serial I/O registers (4000120h-400015Fh): SIOCNT/RCNT mode model, SIODATA32/SIOMULTI0-3,
//! SIODATA8/SIOMLT_SEND, the JOY Bus registers, and normal-mode transfers driven by the internal
//! shift clock. Source: GBATEK "GBA Communication Ports" (SIO Normal Mode, SIO Multi-Player Mode,
//! SIO UART Mode, SIO JOY BUS Mode, SIO General-Purpose Mode, SIO Control Registers Summary).
//!
//! No link partner is emulated. Every input pin floats high through its internal pull-up, so:
//! - Normal mode: SI reads high; an internal-clock transfer shifts in all ones and completes after
//!   8 or 32 bits at 64 (256 KHz) or 8 (2 MHz) cycles per bit, clears the start bit and raises the
//!   serial IRQ when enabled. An external-clock transfer never completes (nobody drives SC).
//! - Multi-player mode: SI high means this unit is a child, and the start bit is read-only for
//!   slaves, so no transfer ever starts. SD reads "all ready" (pulled high).
//! - UART mode: the send path has no listener and the receive FIFO stays empty.
//! - JOY Bus mode: the GBA is always the slave; no master is emulated.
//!
//! Register storage keeps only the bits GBATEK marks writable; read-only bits are composed on read
//! by [`Sio::peek16`] so a stored value never leaks a bit hardware would not return.

use crate::irq::{Irq, IrqController};
use crate::sched::{Event, Scheduler};

/// Bits of SIOCNT with backing storage in every mode: 0-3, 7-14. Bits 4-6 are read-only status
/// (ID/error, send/receive/error flags, or "always 0") and bit 15 is read-only 0.
const SIOCNT_STORED: u16 = 0x7F8F;
/// Bits of RCNT with backing storage: 0-8 (data/direction/SI IRQ enable) and 14-15 (mode).
/// Bits 9-13 are "Always 0, read only".
const RCNT_STORED: u16 = 0xC1FF;

/// Communication mode selected by RCNT.15-14 and SIOCNT.13-12 (GBATEK "SIO Control Registers
/// Summary").
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Normal8,
    Normal32,
    Multi,
    Uart,
    Gpio,
    Joy,
}

#[cfg_attr(feature = "savestate", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Default, Clone)]
pub struct Sio {
    /// 4000120h-4000126h: SIOMULTI0-3; the first two halfwords double as SIODATA32.
    pub multi: [u16; 4],
    /// 4000128h SIOCNT, writable bits only (`SIOCNT_STORED`).
    pub siocnt: u16,
    /// 400012Ah SIODATA8 / SIOMLT_SEND.
    pub data8: u16,
    /// 4000134h RCNT, writable bits only (`RCNT_STORED`).
    pub rcnt: u16,
    /// 4000140h JOYCNT: bits 0-2 flags (write 1 to acknowledge), bit 6 reset-command IRQ enable.
    pub joycnt: u16,
    /// 4000150h JOY_RECV, written by the (unemulated) JOY Bus master.
    pub joy_recv: u32,
    /// 4000154h JOY_TRANS, the reply data a master would read.
    pub joy_trans: u32,
    /// 4000158h JOYSTAT: bit 1 set by a JOY_TRANS write, bit 3 cleared by a JOY_RECV read,
    /// bits 4-5 general purpose.
    pub joystat: u16,
    /// A normal-mode transfer is in flight (an `Event::SioComplete` is pending for it).
    pub busy: bool,
}

impl Sio {
    pub fn new() -> Self {
        Self::default()
    }

    /// Post-BIOS / `RegisterRamReset` state: everything zero except RCNT = 8000h (general
    /// purpose mode, all pins input), which is how the BIOS leaves the port.
    pub fn reset(&mut self) {
        *self = Sio { rcnt: 0x8000, ..Sio::default() };
    }

    pub fn mode(&self) -> Mode {
        match (self.rcnt >> 14) & 3 {
            0 | 1 => match (self.siocnt >> 12) & 3 {
                0 => Mode::Normal8,
                1 => Mode::Normal32,
                2 => Mode::Multi,
                _ => Mode::Uart,
            },
            2 => Mode::Gpio,
            _ => Mode::Joy,
        }
    }

    /// SIOCNT as the CPU reads it. Storage holds the writable bits; the read-only ones are the
    /// state of an unconnected port (GBATEK SIOCNT tables for each mode).
    fn siocnt_read(&self) -> u16 {
        match (self.siocnt >> 12) & 3 {
            // Normal: bit 2 SI state (1=High/None), bits 4-6 "always 0", bit 15 always 0.
            0 | 1 => (self.siocnt & (SIOCNT_STORED & !0x0004)) | 0x0004,
            // Multi-player: bit 2 SI-terminal (1=child: SI floats high without a parent), bit 3
            // SD-terminal (1=all GBAs ready: SD pulled high), bits 4-5 ID = 0, bit 6 error = 0.
            2 => (self.siocnt & (SIOCNT_STORED & !0x000C)) | 0x000C,
            // UART: bit 4 send-data full = 0, bit 5 receive-data empty = 1, bit 6 error = 0.
            _ => self.siocnt | 0x0020,
        }
    }

    /// RCNT as the CPU reads it: stored bits plus the current SC,SD,SI,SO pin state in bits 0-3
    /// ("Undocumented (current SC,SD,SI,SO state, as for General Purpose mode)").
    fn rcnt_read(&self) -> u16 {
        let pins = match self.mode() {
            // Output pins show their data bit; input pins float high (internal pull-up).
            Mode::Gpio => {
                let dir = (self.rcnt >> 4) & 0xF;
                (self.rcnt & dir) | (!dir & 0xF)
            }
            // "SI and SO pins are data lines, SC and SD are set to low."
            Mode::Joy => 0b1100,
            // SC idles high, SD is unused (low), SI floats high, SO follows "SO during inactivity".
            Mode::Normal8 | Mode::Normal32 => 0b0101 | (self.siocnt & 0x0008),
            // Multi: SD high while every unit is in multi-player mode, SO high after a transfer.
            // UART: SC/SD are the (idle) CTS lines, SI/SO idle high.
            Mode::Multi | Mode::Uart => 0b1111,
        };
        (self.rcnt & (RCNT_STORED & !0xF)) | pins
    }

    /// Side-effect-free 16-bit read of the register at `off` (400012xh-400015xh, even offset).
    /// Unused halfwords return 0; the bus decides whether they are open-bus instead.
    pub fn peek16(&self, off: u16) -> u16 {
        match off {
            0x120..=0x126 => self.multi[((off - 0x120) / 2) as usize],
            0x128 => self.siocnt_read(),
            // UART mode reads the receive shift register / FIFO, which is empty.
            0x12A => {
                if self.mode() == Mode::Uart {
                    0
                } else {
                    self.data8
                }
            }
            0x134 => self.rcnt_read(),
            0x140 => self.joycnt,
            // JOY_RECV holds what a master sent (nothing); JOY_TRANS does not read back
            // (GBATEK lists both as R/W; the mGBA suite measures 0 on hardware after a write).
            0x150 => self.joy_recv as u16,
            0x152 => (self.joy_recv >> 16) as u16,
            0x154 | 0x156 => 0,
            0x158 => self.joystat,
            _ => 0,
        }
    }

    /// 16-bit read with side effects: reading JOY_RECV clears JOYSTAT bit 3 ("Send Status Flag").
    pub fn read16(&mut self, off: u16) -> u16 {
        if off == 0x150 || off == 0x152 {
            self.joystat &= !0x0008;
        }
        self.peek16(off)
    }

    pub fn peek8(&self, off: u16) -> u8 {
        (self.peek16(off & !1) >> ((off & 1) * 8)) as u8
    }

    pub fn read8(&mut self, off: u16) -> u8 {
        (self.read16(off & !1) >> ((off & 1) * 8)) as u8
    }

    /// The stored halfword a byte write merges into (no read-only bits, no pin state).
    fn stored16(&self, off: u16) -> u16 {
        match off {
            0x120..=0x126 => self.multi[((off - 0x120) / 2) as usize],
            0x128 => self.siocnt,
            0x12A => self.data8,
            0x134 => self.rcnt,
            0x154 => self.joy_trans as u16,
            0x156 => (self.joy_trans >> 16) as u16,
            0x158 => self.joystat,
            _ => 0,
        }
    }

    pub fn write8(&mut self, off: u16, v: u8, sched: &mut Scheduler) {
        let reg = off & !1;
        if reg == 0x140 {
            // JOYCNT: the flags and the IRQ enable live in the low byte; merging the stored low
            // byte into a high-byte write would acknowledge flags the program never touched.
            if off & 1 == 0 {
                self.write16(reg, v as u16, sched);
            }
            return;
        }
        let cur = self.stored16(reg);
        let merged = if off & 1 == 0 { (cur & 0xFF00) | v as u16 } else { (cur & 0x00FF) | (v as u16) << 8 };
        self.write16(reg, merged, sched);
    }

    pub fn write16(&mut self, off: u16, v: u16, sched: &mut Scheduler) {
        match off {
            // SIODATA32 accepts data only in normal 32-bit mode ("SIOCNT/RCNT must be set to
            // 32bit normal mode <before> writing to SIODATA32"); as SIOMULTI0/1 it is receive-only.
            0x120 | 0x122 => {
                if self.mode() == Mode::Normal32 {
                    self.multi[((off - 0x120) / 2) as usize] = v;
                }
            }
            // SIOMULTI2/3 only ever hold incoming data.
            0x124 | 0x126 => {}
            0x128 => self.write_siocnt(v, sched),
            // In UART mode the write goes to the send shift register / FIFO, not to a readable
            // data register; nothing is listening.
            0x12A => {
                if self.mode() != Mode::Uart {
                    self.data8 = v;
                }
            }
            0x134 => self.rcnt = v & RCNT_STORED,
            // Bits 0-2 "work like the bits in the IF register": a 1 acknowledges. Bit 6 is R/W.
            0x140 => self.joycnt = (self.joycnt & 0x0007 & !v) | (v & 0x0040),
            // Receive data is written by the master, never by the local CPU.
            0x150 | 0x152 => {}
            0x154 => {
                self.joy_trans = (self.joy_trans & 0xFFFF_0000) | v as u32;
                self.joystat |= 0x0002;
            }
            0x156 => {
                self.joy_trans = (self.joy_trans & 0x0000_FFFF) | (v as u32) << 16;
                self.joystat |= 0x0002;
            }
            // Only the general purpose flags (bits 4-5) are writable.
            0x158 => self.joystat = (self.joystat & !0x0030) | (v & 0x0030),
            _ => {}
        }
    }

    fn write_siocnt(&mut self, v: u16, sched: &mut Scheduler) {
        self.siocnt = v & SIOCNT_STORED;
        let start = self.siocnt & 0x0080 != 0;
        match self.mode() {
            Mode::Normal8 | Mode::Normal32 => {
                if !start {
                    self.cancel(sched);
                } else if !self.busy && self.siocnt & 0x0001 != 0 {
                    // Internal shift clock: the transfer runs to completion on its own. With the
                    // external clock nothing drives SC, so the start bit simply stays set.
                    self.start(sched);
                }
            }
            // Multi-player start is read-only for slaves (SI high = child); UART, general purpose
            // and JOY Bus have no start bit. Leaving normal mode abandons an in-flight transfer.
            _ => self.cancel(sched),
        }
    }

    /// Cycles for one normal-mode transfer: 8 or 32 bits at 16.78 MHz / 64 (256 KHz) or / 8 (2 MHz).
    pub fn transfer_cycles(&self) -> u64 {
        let bits = if self.mode() == Mode::Normal32 { 32 } else { 8 };
        let per_bit = if self.siocnt & 0x0002 != 0 { 8 } else { 64 };
        bits * per_bit
    }

    fn start(&mut self, sched: &mut Scheduler) {
        sched.cancel(Event::SioComplete);
        sched.schedule_in(Event::SioComplete, self.transfer_cycles());
        self.busy = true;
    }

    fn cancel(&mut self, sched: &mut Scheduler) {
        if self.busy {
            sched.cancel(Event::SioComplete);
            self.busy = false;
        }
    }

    /// `Event::SioComplete`: all bits shifted. The start bit resets, the shifted-in bits (SI high
    /// throughout, so all ones) land in the data register, and the IRQ fires if bit 14 is set.
    pub fn on_complete(&mut self, irq: &mut IrqController) {
        if !self.busy {
            return;
        }
        self.busy = false;
        self.siocnt &= !0x0080;
        match self.mode() {
            Mode::Normal32 => {
                self.multi[0] = 0xFFFF;
                self.multi[1] = 0xFFFF;
            }
            // SIODATA8: "only lower 8bit are used".
            _ => self.data8 |= 0x00FF,
        }
        if self.siocnt & 0x4000 != 0 {
            irq.raise(Irq::Serial);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sio() -> (Sio, Scheduler, IrqController) {
        let mut s = Sio::new();
        s.reset();
        (s, Scheduler::new(), IrqController::new())
    }

    /// The mGBA suite's "SIO register R/W" procedure: set RCNT and SIOCNT, write `v` to `off`,
    /// read it back.
    fn rw(s: &mut Sio, sched: &mut Scheduler, rcnt: u16, siocnt: u16, off: u16, v: u16) -> u16 {
        s.write16(0x134, rcnt, sched);
        s.write16(0x128, siocnt, sched);
        s.write16(off, v, sched);
        let r = s.read16(off);
        if off != 0x134 {
            s.write16(off, 0, sched);
        }
        s.write16(0x128, 0, sched);
        s.write16(0x134, 0x8000, sched);
        r
    }

    #[test]
    fn reset_state() {
        let (s, ..) = sio();
        assert_eq!(s.peek16(0x134), 0x800F, "general purpose mode, all pins input and pulled high");
        assert_eq!(s.peek16(0x128), 0x0004, "SI reads high on an unconnected port");
        assert_eq!(s.mode(), Mode::Gpio);
    }

    #[test]
    fn siocnt_readback_per_mode() {
        let (mut s, mut sched, _) = sio();
        // Multi: bits 4-6 and 15 read-only 0; SI (child) and SD (ready) read 1.
        assert_eq!(rw(&mut s, &mut sched, 0, 0x2000, 0x128, 0xEFFF), 0x6F8F);
        // Normal 8/32: bits 4-6, 15 read 0; SI high; the start bit reads back set right away.
        assert_eq!(rw(&mut s, &mut sched, 0, 0x0000, 0x128, 0xCFFF), 0x4F8F);
        assert_eq!(rw(&mut s, &mut sched, 0, 0x1000, 0x128, 0xDFFF), 0x5F8F);
        // UART: send full 0, receive empty 1, error 0.
        assert_eq!(rw(&mut s, &mut sched, 0, 0x3000, 0x128, 0xFFFF), 0x7FAF);
        // General purpose / JOY Bus: SIOCNT bits still exist as for the mode its own bits select.
        assert_eq!(rw(&mut s, &mut sched, 0x8000, 0, 0x128, 0xCFFF), 0x4F8F);
        assert_eq!(rw(&mut s, &mut sched, 0xC000, 0, 0x128, 0xCFFF), 0x4F8F);
    }

    #[test]
    fn rcnt_pin_state_per_mode() {
        let (mut s, mut sched, _) = sio();
        // Multi and UART: every pin idles high.
        assert_eq!(rw(&mut s, &mut sched, 0, 0x2000, 0x134, 0x3FFF), 0x01FF);
        assert_eq!(rw(&mut s, &mut sched, 0, 0x3000, 0x134, 0x3FFF), 0x01FF);
        // Normal: SC and SI high, SD unused, SO follows SIOCNT bit 3 (0 here).
        assert_eq!(rw(&mut s, &mut sched, 0, 0x0000, 0x134, 0x3FFF), 0x01F5);
        assert_eq!(rw(&mut s, &mut sched, 0, 0x1000, 0x134, 0x3FFF), 0x01F5);
        assert_eq!(rw(&mut s, &mut sched, 0, 0x0008, 0x134, 0x3FFF), 0x01FD, "SO during inactivity");
        // General purpose, all outputs: data bits read back. JOY: SC/SD low, SI/SO high.
        assert_eq!(rw(&mut s, &mut sched, 0x8000, 0, 0x134, 0xBFFF), 0x81FF);
        assert_eq!(rw(&mut s, &mut sched, 0xC000, 0, 0x134, 0xFFFF), 0xC1FC);
        // General purpose, all inputs: the pull-ups win over the written data bits.
        assert_eq!(rw(&mut s, &mut sched, 0x8000, 0, 0x134, 0x8000), 0x800F);
        assert_eq!(rw(&mut s, &mut sched, 0x8000, 0, 0x134, 0x8010), 0x801E, "SC output low, rest input");
        // Bits 9-13 never store.
        assert_eq!(s.peek16(0x134) & 0x3E00, 0);
    }

    #[test]
    fn data_registers_per_mode() {
        let (mut s, mut sched, _) = sio();
        // SIODATA32 is writable only in normal 32-bit mode.
        assert_eq!(rw(&mut s, &mut sched, 0, 0x2000, 0x120, 0xFFFF), 0);
        assert_eq!(rw(&mut s, &mut sched, 0, 0x0000, 0x120, 0xFFFF), 0);
        assert_eq!(rw(&mut s, &mut sched, 0, 0x1000, 0x120, 0xFFFF), 0xFFFF);
        assert_eq!(rw(&mut s, &mut sched, 0, 0x1000, 0x122, 0xFFFF), 0xFFFF);
        assert_eq!(rw(&mut s, &mut sched, 0, 0x3000, 0x120, 0xFFFF), 0);
        assert_eq!(rw(&mut s, &mut sched, 0x8000, 0, 0x120, 0xFFFF), 0);
        assert_eq!(rw(&mut s, &mut sched, 0xC000, 0, 0x122, 0xFFFF), 0);
        // SIOMULTI2/3 are receive-only.
        assert_eq!(rw(&mut s, &mut sched, 0, 0x2000, 0x124, 0xFFFF), 0);
        assert_eq!(rw(&mut s, &mut sched, 0, 0x1000, 0x126, 0xFFFF), 0);
        // SIODATA8 / SIOMLT_SEND is plain R/W except in UART mode, where reads see the empty FIFO.
        assert_eq!(rw(&mut s, &mut sched, 0, 0x2000, 0x12A, 0xFFFF), 0xFFFF);
        assert_eq!(rw(&mut s, &mut sched, 0, 0x0000, 0x12A, 0xFFFF), 0xFFFF);
        assert_eq!(rw(&mut s, &mut sched, 0, 0x3000, 0x12A, 0xFFFF), 0);
        assert_eq!(rw(&mut s, &mut sched, 0x8000, 0, 0x12A, 0xFFFF), 0xFFFF);
    }

    #[test]
    fn joy_bus_registers() {
        let (mut s, mut sched, _) = sio();
        // JOYCNT: writing 1s acknowledges bits 0-2; bit 6 is stored.
        s.joycnt = 0x0007;
        s.write16(0x140, 0x0045, &mut sched);
        assert_eq!(s.peek16(0x140), 0x0042);
        s.write16(0x140, 0xFFFF, &mut sched);
        assert_eq!(s.peek16(0x140), 0x0040);
        s.write8(0x141, 0xFF, &mut sched);
        assert_eq!(s.peek16(0x140), 0x0040, "high byte holds no bits and acknowledges nothing");
        s.joycnt = 0x0047;
        s.write8(0x141, 0xFF, &mut sched);
        assert_eq!(s.peek16(0x140), 0x0047);
        // JOY_RECV ignores CPU writes; JOY_TRANS stores but does not read back and sets JOYSTAT.1.
        for m in [0x8000u16, 0xC000, 0] {
            assert_eq!(rw(&mut s, &mut sched, m, 0, 0x150, 0xFFFF), 0);
            assert_eq!(rw(&mut s, &mut sched, m, 0, 0x152, 0xFFFF), 0);
            assert_eq!(rw(&mut s, &mut sched, m, 0, 0x154, 0xFFFF), 0);
            assert_eq!(rw(&mut s, &mut sched, m, 0, 0x156, 0xFFFF), 0);
        }
        s.joystat = 0;
        s.write16(0x154, 0x1234, &mut sched);
        s.write16(0x156, 0x5678, &mut sched);
        assert_eq!(s.joy_trans, 0x5678_1234);
        assert_eq!(s.peek16(0x158), 0x0002);
        // JOYSTAT: only bits 4-5 writable; reading JOY_RECV clears bit 3.
        s.joystat = 0x0008;
        s.write16(0x158, 0xFFFF, &mut sched);
        assert_eq!(s.peek16(0x158), 0x0038);
        s.read16(0x150);
        assert_eq!(s.peek16(0x158), 0x0030);
    }

    fn run_transfer(siocnt: u16, expect_cycles: u64) -> (Sio, IrqController) {
        let (mut s, mut sched, mut irq) = sio();
        sched.now = 1000;
        s.write16(0x134, 0, &mut sched);
        s.write16(0x128, siocnt & !0x80, &mut sched);
        assert!(!s.busy);
        s.write16(0x128, siocnt | 0x80, &mut sched);
        assert!(s.busy);
        assert_eq!(sched.time_of(Event::SioComplete), Some(1000 + expect_cycles));
        assert_eq!(s.peek16(0x128) & 0x80, 0x80, "start bit reads set while busy");
        sched.now = 1000 + expect_cycles;
        let (ev, _) = sched.pop_due().unwrap();
        assert_eq!(ev, Event::SioComplete);
        s.on_complete(&mut irq);
        assert!(!s.busy);
        assert_eq!(s.peek16(0x128) & 0x80, 0, "start bit clears on completion");
        (s, irq)
    }

    #[test]
    fn normal_transfer_timing_and_result() {
        // 8 bits at 256 KHz (64 cycles/bit) and 2 MHz (8 cycles/bit); 32 bits likewise.
        let (s, irq) = run_transfer(0x4001, 8 * 64);
        assert_eq!(s.peek16(0x12A), 0x00FF, "SI high: all ones shifted in");
        assert_eq!(irq.if_ & Irq::Serial as u16, Irq::Serial as u16);
        let (s, irq) = run_transfer(0x0003, 8 * 8);
        assert_eq!(s.peek16(0x12A), 0x00FF);
        assert_eq!(irq.if_, 0, "no IRQ without bit 14");
        let (s, _) = run_transfer(0x5001, 32 * 64);
        assert_eq!(s.peek16(0x120), 0xFFFF);
        assert_eq!(s.peek16(0x122), 0xFFFF);
        let (s, _) = run_transfer(0x5003, 32 * 8);
        assert_eq!(s.peek16(0x120), 0xFFFF);
    }

    #[test]
    fn data8_upper_byte_survives_transfer() {
        let (mut s, mut sched, mut irq) = sio();
        s.write16(0x134, 0, &mut sched);
        s.write16(0x12A, 0xAA55, &mut sched);
        s.write16(0x128, 0x0083, &mut sched);
        sched.now += 64;
        s.on_complete(&mut irq);
        assert_eq!(s.peek16(0x12A), 0xAAFF);
    }

    #[test]
    fn external_clock_and_other_modes_never_complete() {
        let (mut s, mut sched, _) = sio();
        s.write16(0x134, 0, &mut sched);
        s.write16(0x128, 0x4080, &mut sched);
        assert!(!s.busy, "external clock: nothing drives SC");
        assert_eq!(sched.time_of(Event::SioComplete), None);
        assert_eq!(s.peek16(0x128) & 0x80, 0x80, "start bit stays set");
        // Multi-player parent-less start, UART, general purpose and JOY Bus.
        for (rcnt, siocnt) in [(0u16, 0x6081u16), (0, 0x7081), (0x8000, 0x4081), (0xC000, 0x4081)] {
            let (mut s, mut sched, _) = sio();
            s.write16(0x134, rcnt, &mut sched);
            s.write16(0x128, siocnt, &mut sched);
            assert!(!s.busy, "rcnt={rcnt:04X} siocnt={siocnt:04X}");
            assert!(sched.is_empty());
        }
    }

    #[test]
    fn clearing_start_or_leaving_normal_mode_cancels() {
        let (mut s, mut sched, mut irq) = sio();
        s.write16(0x134, 0, &mut sched);
        s.write16(0x128, 0x4081, &mut sched);
        assert!(s.busy);
        s.write16(0x128, 0x4001, &mut sched);
        assert!(!s.busy);
        assert!(sched.is_empty());
        s.write16(0x128, 0x4081, &mut sched);
        s.write16(0x128, 0x6081, &mut sched);
        assert!(!s.busy, "switching to multi-player abandons the transfer");
        assert!(sched.is_empty());
        // A rewrite with the start bit still set does not restart the running transfer.
        s.write16(0x128, 0x4081, &mut sched);
        let t = sched.time_of(Event::SioComplete);
        sched.now += 100;
        s.write16(0x128, 0x4081, &mut sched);
        assert_eq!(sched.time_of(Event::SioComplete), t);
        // A stale completion after reset is ignored.
        s.reset();
        s.on_complete(&mut irq);
        assert_eq!(irq.if_, 0);
    }

    #[test]
    fn byte_writes_merge_into_stored_bits() {
        let (mut s, mut sched, _) = sio();
        s.write16(0x134, 0, &mut sched);
        s.write8(0x129, 0x40, &mut sched);
        s.write8(0x128, 0x81, &mut sched);
        assert!(s.busy, "low-byte write carrying the start bit starts the transfer");
        assert_eq!(s.peek16(0x128), 0x4085);
        assert_eq!(s.peek8(0x129), 0x40);
        assert_eq!(s.peek8(0x128), 0x85);
        s.write8(0x12A, 0x34, &mut sched);
        s.write8(0x12B, 0x12, &mut sched);
        assert_eq!(s.peek16(0x12A), 0x1234);
        s.write8(0x135, 0xC1, &mut sched);
        assert_eq!(s.mode(), Mode::Joy);
        assert_eq!(s.peek16(0x134), 0xC10C);
    }
}
