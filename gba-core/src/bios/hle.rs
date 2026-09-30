//! High-level emulation of BIOS SWI functions and of the BIOS IRQ dispatcher.
//! Source: GBATEK "GBA BIOS Functions" (each function's register contract), "BIOS RAM Usage".
//!
//! M0 status: framework + Halt/Stop/IntrWait/VBlankIntrWait/Div/DivArm/Sqrt/CpuSet/CpuFastSet/
//! GetBiosChecksum/RegisterRamReset/SoundBias/MidiKey2Freq. Remaining functions land in M4 and are
//! recorded in `Bios::unimplemented` until then.

use super::{swi, HLE_IRQ_RETURN_ADDR};
use crate::bus::{Access, Bus, MemoryBus};
use crate::cpu::regs::{Regs, MODE_IRQ};

/// BIOS mirror of acknowledged IRQ flags, used by IntrWait.
pub const INTR_FLAGS_MIRROR: u32 = 0x0300_7FF8;
/// User IRQ handler pointer.
pub const IRQ_HANDLER_PTR: u32 = 0x0300_7FFC;
/// Real BIOS checksum (GBATEK: BAAE187Fh for the GBA/GBA SP/DS BIOS).
pub const BIOS_CHECKSUM: u32 = 0xBAAE_187F;

impl Bus {
    /// Emulate BIOS code at `regs.r[15]`. Handles the IRQ vector and the IRQ return stub.
    pub(crate) fn hle_vector(&mut self, regs: &mut Regs) -> bool {
        match regs.r[15] {
            // Vector 0x18: the CPU has already entered IRQ mode. Emulate:
            //   stmfd r13!, {r0-r3,r12,r14}; mov r0, #0x4000000; add r14, r15, #0; ldr r15, [r0, #-4]
            crate::cpu::VEC_IRQ => {
                let mut sp = regs.r[13].wrapping_sub(6 * 4);
                regs.r[13] = sp;
                for &reg in &[0usize, 1, 2, 3, 12, 14] {
                    self.write32(sp, regs.r[reg], Access::S);
                    sp = sp.wrapping_add(4);
                }
                regs.r[0] = 0x0400_0000;
                regs.r[14] = HLE_IRQ_RETURN_ADDR;
                regs.r[15] = self.read32(IRQ_HANDLER_PTR, Access::N) & !3;
                self.bios.open_bus = super::BIOS_OPENBUS_DURING_IRQ;
                self.idle(6);
                true
            }
            // Return stub: ldmfd r13!, {r0-r3,r12,r14}; subs r15, r14, #4
            HLE_IRQ_RETURN_ADDR => {
                if regs.mode() != MODE_IRQ {
                    // A game returned here from a non-IRQ mode; nothing sensible to do.
                    return false;
                }
                let mut sp = regs.r[13];
                for &reg in &[0usize, 1, 2, 3, 12, 14] {
                    regs.r[reg] = self.read32(sp, Access::S);
                    sp = sp.wrapping_add(4);
                }
                regs.r[13] = sp;
                let target = regs.r[14].wrapping_sub(4);
                let spsr = regs.spsr();
                regs.set_cpsr(spsr);
                regs.r[15] = if regs.thumb() { target & !1 } else { target & !3 };
                self.bios.open_bus = super::BIOS_OPENBUS_AFTER_IRQ;
                self.idle(6);
                true
            }
            _ => false,
        }
    }

    /// Service `swi num`. Contract: `regs.r[15]` is the address of the instruction *after* the SWI
    /// (so a waiting function can rewind and re-execute it). Returns true when handled.
    pub(crate) fn hle_swi(&mut self, num: u8, regs: &mut Regs) -> bool {
        self.bios.swi_count += 1;
        self.bios.open_bus = super::BIOS_OPENBUS_AFTER_SWI;
        // Approximate SWI dispatch overhead (BIOS entry/exit code).
        self.idle(10);
        match num {
            swi::HALT => self.irq.halted = true,
            swi::STOP => self.irq.stopped = true,
            swi::INTR_WAIT => self.hle_intr_wait(regs, regs.r[0] != 0, regs.r[1] as u16),
            swi::VBLANK_INTR_WAIT => self.hle_intr_wait(regs, true, 1),
            swi::DIV => {
                let (q, r, a) = hle_div(regs.r[0] as i32, regs.r[1] as i32);
                regs.r[0] = q as u32;
                regs.r[1] = r as u32;
                regs.r[3] = a;
            }
            swi::DIV_ARM => {
                let (q, r, a) = hle_div(regs.r[1] as i32, regs.r[0] as i32);
                regs.r[0] = q as u32;
                regs.r[1] = r as u32;
                regs.r[3] = a;
            }
            swi::SQRT => regs.r[0] = isqrt(regs.r[0]),
            swi::CPU_SET => self.hle_cpu_set(regs),
            swi::CPU_FAST_SET => self.hle_cpu_fast_set(regs),
            swi::GET_BIOS_CHECKSUM => regs.r[0] = BIOS_CHECKSUM,
            swi::REGISTER_RAM_RESET => self.hle_register_ram_reset(regs.r[0] as u8),
            swi::SOUND_BIAS => {
                let level: u16 = if regs.r[0] == 0 { 0 } else { 0x200 };
                let cur = self.io_read16(0x0400_0088);
                self.io_write16(0x0400_0088, (cur & !0x3FE) | level);
            }
            swi::MIDI_KEY2FREQ => {
                let wave = regs.r[0];
                let freq_base = self.read32(wave.wrapping_add(4), Access::N) as f64;
                let key = regs.r[1] as f64;
                let fine = regs.r[2] as f64;
                let v = freq_base * 2f64.powf((180.0 - key - fine / 256.0) / 12.0);
                regs.r[0] = v as u32;
            }
            _ => {
                if !self.bios.unimplemented.contains(&num) {
                    self.bios.unimplemented.push(num);
                }
            }
        }
        true
    }

    /// IntrWait core. `discard`: clear the requested flags first and wait for a fresh one.
    fn hle_intr_wait(&mut self, regs: &mut Regs, discard: bool, flags: u16) {
        // Both IntrWait and VBlankIntrWait force IME=1 (GBATEK).
        self.irq.ime = true;
        let mirror_addr = INTR_FLAGS_MIRROR;
        let mut mirror = self.read16(mirror_addr, Access::N);
        if self.bios.intr_wait.is_none() && discard {
            mirror &= !flags;
            self.write16(mirror_addr, mirror, Access::N);
        }
        if mirror & flags != 0 {
            // Satisfied: acknowledge in the mirror and return.
            mirror &= !flags;
            self.write16(mirror_addr, mirror, Access::N);
            self.bios.intr_wait = None;
            return;
        }
        // Park: halt and re-execute the SWI after the next interrupt.
        self.bios.intr_wait = Some(flags);
        self.irq.halted = true;
        regs.r[15] = regs.r[15].wrapping_sub(if regs.thumb() { 2 } else { 4 });
    }

    /// CpuSet: r0 src, r1 dst, r2 = count (bits 0-20) | fixed-source fill (bit 24) | 32-bit (bit 26).
    fn hle_cpu_set(&mut self, regs: &mut Regs) {
        let mut src = regs.r[0];
        let mut dst = regs.r[1];
        let ctl = regs.r[2];
        let count = ctl & 0x1F_FFFF;
        let fill = ctl & (1 << 24) != 0;
        if ctl & (1 << 26) != 0 {
            src &= !3;
            dst &= !3;
            for _ in 0..count {
                let v = self.read32(src, Access::S);
                self.write32(dst, v, Access::S);
                if !fill {
                    src = src.wrapping_add(4);
                }
                dst = dst.wrapping_add(4);
            }
        } else {
            src &= !1;
            dst &= !1;
            for _ in 0..count {
                let v = self.read16(src, Access::S);
                self.write16(dst, v, Access::S);
                if !fill {
                    src = src.wrapping_add(2);
                }
                dst = dst.wrapping_add(2);
            }
        }
    }

    /// CpuFastSet: 32-bit units in blocks of 8 words; count rounded up to a multiple of 8.
    fn hle_cpu_fast_set(&mut self, regs: &mut Regs) {
        let mut src = regs.r[0] & !3;
        let mut dst = regs.r[1] & !3;
        let ctl = regs.r[2];
        let count = ((ctl & 0x1F_FFFF) + 7) & !7;
        let fill = ctl & (1 << 24) != 0;
        for _ in 0..count {
            let v = self.read32(src, Access::S);
            self.write32(dst, v, Access::S);
            if !fill {
                src = src.wrapping_add(4);
            }
            dst = dst.wrapping_add(4);
        }
    }

    /// RegisterRamReset flags: bit0 EWRAM, bit1 IWRAM (except the top 0x200 bytes), bit2 palette,
    /// bit3 VRAM, bit4 OAM, bit5 SIO registers, bit6 sound registers, bit7 all other registers.
    fn hle_register_ram_reset(&mut self, flags: u8) {
        if flags & 1 != 0 {
            self.ewram.fill(0);
        }
        if flags & 2 != 0 {
            self.iwram[..0x7E00].fill(0);
        }
        if flags & 4 != 0 {
            self.ppu.pal.fill(0);
        }
        if flags & 8 != 0 {
            self.ppu.vram.fill(0);
        }
        if flags & 16 != 0 {
            self.ppu.oam.fill(0);
        }
        if flags & 32 != 0 {
            self.sio.fill(0);
            self.sio[(0x134 - 0x120)..(0x136 - 0x120)].copy_from_slice(&0x8000u16.to_le_bytes());
        }
        if flags & 64 != 0 {
            for off in (0x60..0xA8).step_by(2) {
                self.io_write16(0x0400_0000 + off, 0);
            }
            self.io_write16(0x0400_0088, 0x200);
        }
        if flags & 128 != 0 {
            for off in (0x00..0x60).step_by(2) {
                if off == 0x04 || off == 0x06 {
                    continue;
                }
                self.io_write16(0x0400_0000 + off, 0);
            }
            for off in (0xB0..0xE0).step_by(2) {
                self.io_write16(0x0400_0000 + off, 0);
            }
            for off in (0x100..0x110).step_by(2) {
                self.io_write16(0x0400_0000 + off, 0);
            }
            self.io_write16(0x0400_0200, 0);
            self.io_write16(0x0400_0202, 0xFFFF);
            self.io_write16(0x0400_0208, 0);
            self.io_write16(0x0400_0204, 0);
            // DISPCNT = 0x0080 (forced blank) as the real BIOS does.
            self.io_write16(0x0400_0000, 0x0080);
        }
    }
}

/// Signed division as the BIOS does it. Returns (quotient, remainder, |quotient|).
/// Division by zero: the real BIOS hangs in an infinite loop; we return the values mGBA does
/// (r0 = sign of numerator, r1 = numerator, r3 = 1) so games do not lock up.
pub fn hle_div(num: i32, denom: i32) -> (i32, i32, u32) {
    if denom == 0 {
        return (if num < 0 { -1 } else { 1 }, num, 1);
    }
    let q = num.wrapping_div(denom);
    let r = num.wrapping_rem(denom);
    (q, r, q.unsigned_abs())
}

/// Integer square root (floor).
pub fn isqrt(v: u32) -> u32 {
    if v < 2 {
        return v;
    }
    let mut x = (v as f64).sqrt() as u32;
    while (x as u64) * (x as u64) > v as u64 {
        x -= 1;
    }
    while ((x + 1) as u64) * ((x + 1) as u64) <= v as u64 {
        x += 1;
    }
    x
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn div_matches_bios_contract() {
        assert_eq!(hle_div(7, 2), (3, 1, 3));
        assert_eq!(hle_div(-7, 2), (-3, -1, 3));
        assert_eq!(hle_div(7, -2), (-3, 1, 3));
        assert_eq!(hle_div(i32::MIN, -1), (i32::MIN, 0, 0x8000_0000));
    }

    #[test]
    fn sqrt_floor() {
        assert_eq!(isqrt(0), 0);
        assert_eq!(isqrt(1), 1);
        assert_eq!(isqrt(15), 3);
        assert_eq!(isqrt(16), 4);
        assert_eq!(isqrt(u32::MAX), 0xFFFF);
    }
}
