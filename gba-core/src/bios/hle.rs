//! High-level emulation of BIOS SWI functions and of the BIOS IRQ dispatcher.
//! Source: GBATEK "BIOS Functions" (docs/gbatek/703-714), "BIOS RAM Usage" (720).
//!
//! Register contracts follow GBATEK exactly. Decompression/unpack functions read through the bus
//! and write with the WRAM (8-bit) or VRAM (16-bit unit) variants as documented.

use super::{swi, HLE_IRQ_RETURN_ADDR};
use crate::bus::{Access, Bus, MemoryBus};
use crate::cpu::regs::{Regs, MODE_IRQ, MODE_SVC, MODE_SYS};

/// BIOS mirror of acknowledged IRQ flags, used by IntrWait.
pub const INTR_FLAGS_MIRROR: u32 = 0x0300_7FF8;
/// SoftReset re-entry flag (00h = ROM 08000000h, else RAM 02000000h).
pub const SOFT_RESET_FLAG: u32 = 0x0300_7FFA;
/// User IRQ handler pointer.
pub const IRQ_HANDLER_PTR: u32 = 0x0300_7FFC;
/// Real BIOS checksum (GBATEK: BAAE187Fh for the GBA/GBA SP).
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
            swi::SOFT_RESET => self.hle_soft_reset(regs),
            swi::REGISTER_RAM_RESET => self.hle_register_ram_reset(regs.r[0] as u8),
            swi::HALT => self.irq.halted = true,
            swi::STOP => self.irq.stopped = true,
            swi::INTR_WAIT => self.hle_intr_wait(regs, regs.r[0] != 0, regs.r[1] as u16),
            swi::VBLANK_INTR_WAIT => {
                regs.r[0] = 1;
                regs.r[1] = 1;
                self.hle_intr_wait(regs, true, 1);
            }
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
            swi::ARCTAN => {
                let (r0, r1, r3) = arctan(regs.r[0] as i32);
                regs.r[0] = r0 as u32;
                regs.r[1] = r1 as u32;
                regs.r[3] = r3 as u32;
            }
            swi::ARCTAN2 => {
                let (r0, r1) = arctan2(regs.r[0] as i32, regs.r[1] as i32);
                regs.r[0] = r0 as u32;
                regs.r[1] = r1 as u32;
                regs.r[3] = 0x170;
            }
            swi::CPU_SET => self.hle_cpu_set(regs),
            swi::CPU_FAST_SET => self.hle_cpu_fast_set(regs),
            swi::GET_BIOS_CHECKSUM => regs.r[0] = BIOS_CHECKSUM,
            swi::BG_AFFINE_SET => self.hle_bg_affine_set(regs),
            swi::OBJ_AFFINE_SET => self.hle_obj_affine_set(regs),
            swi::BIT_UNPACK => self.hle_bit_unpack(regs),
            swi::LZ77_UNCOMP_WRAM => self.hle_lz77(regs, false),
            swi::LZ77_UNCOMP_VRAM => self.hle_lz77(regs, true),
            swi::HUFF_UNCOMP => self.hle_huffman(regs),
            swi::RL_UNCOMP_WRAM => self.hle_rl(regs, false),
            swi::RL_UNCOMP_VRAM => self.hle_rl(regs, true),
            swi::DIFF8_UNFILTER_WRAM => self.hle_diff8(regs, false),
            swi::DIFF8_UNFILTER_VRAM => self.hle_diff8(regs, true),
            swi::DIFF16_UNFILTER => self.hle_diff16(regs),
            swi::SOUND_BIAS => {
                let level: u16 = if regs.r[0] == 0 { 0 } else { 0x200 };
                let cur = self.io_read16(0x0400_0088);
                self.io_write16(0x0400_0088, (cur & !0x3FE) | level);
            }
            swi::MIDI_KEY2FREQ => {
                let wave = regs.r[0];
                let freq = self.read32(wave.wrapping_add(4), Access::N) as f64;
                let key = regs.r[1] as u8 as f64;
                let fine = regs.r[2] as u8 as f64;
                regs.r[0] = (freq / 2f64.powf((180.0 - key - fine / 256.0) / 12.0)) as u32;
            }
            0x27 => self.irq.write_haltcnt(regs.r[2] as u8), // CustomHalt
            // The BIOS sound driver (m4a) is not emulated: commercial games link their own copy.
            swi::SOUND_DRIVER_INIT
            | swi::SOUND_DRIVER_MODE
            | swi::SOUND_DRIVER_MAIN
            | swi::SOUND_DRIVER_VSYNC
            | swi::SOUND_CHANNEL_CLEAR
            | swi::SOUND_DRIVER_VSYNC_OFF
            | swi::SOUND_DRIVER_VSYNC_ON => self.note_unimplemented(num),
            _ => self.note_unimplemented(num),
        }
        true
    }

    fn note_unimplemented(&mut self, num: u8) {
        if !self.bios.unimplemented.contains(&num) {
            self.bios.unimplemented.push(num);
        }
    }

    /// SoftReset: clear 3007E00h..3007FFFh, reset stacks, zero r0-r12 and the svc/irq LR/SPSR,
    /// enter system mode (IRQs enabled) and jump to ROM or RAM per the re-entry flag.
    fn hle_soft_reset(&mut self, regs: &mut Regs) {
        let flag = self.read8(SOFT_RESET_FLAG, Access::N);
        for a in (0x0300_7E00..0x0300_8000).step_by(4) {
            self.write32(a, 0, Access::S);
        }
        regs.set_cpsr(MODE_SVC | crate::cpu::regs::FLAG_I | crate::cpu::regs::FLAG_F);
        regs.r[13] = 0x0300_7FE0;
        regs.r[14] = 0;
        regs.set_spsr(0);
        regs.switch_mode(MODE_IRQ);
        regs.r[13] = 0x0300_7FA0;
        regs.r[14] = 0;
        regs.set_spsr(0);
        regs.set_cpsr(MODE_SYS);
        regs.r[13] = 0x0300_7F00;
        for r in 0..13 {
            regs.r[r] = 0;
        }
        let target = if flag == 0 { 0x0800_0000 } else { 0x0200_0000 };
        regs.r[14] = target;
        regs.r[15] = target;
        self.bios.intr_wait = None;
    }

    /// IntrWait core. `discard`: clear the requested flags first and wait for a fresh one.
    fn hle_intr_wait(&mut self, regs: &mut Regs, discard: bool, flags: u16) {
        // Both IntrWait and VBlankIntrWait force IME=1 (GBATEK).
        self.irq.ime = true;
        let mut mirror = self.read16(INTR_FLAGS_MIRROR, Access::N);
        if self.bios.intr_wait.is_none() && discard {
            mirror &= !flags;
            self.write16(INTR_FLAGS_MIRROR, mirror, Access::N);
        }
        if mirror & flags != 0 {
            mirror &= !flags;
            self.write16(INTR_FLAGS_MIRROR, mirror, Access::N);
            self.bios.intr_wait = None;
            return;
        }
        // Park: halt and re-execute the SWI after the next interrupt.
        self.bios.intr_wait = Some(flags);
        self.irq.halted = true;
        regs.r[15] = regs.r[15].wrapping_sub(if regs.thumb() { 2 } else { 4 });
    }

    /// The BIOS copy routines "silently reject to do anything if the source start or end
    /// addresses are reaching into the BIOS area" (GBATEK BIOS Memory Copy). The real check is
    /// against the start of EWRAM: a source at 0x10000 or 0x01000000 is rejected too (mgba-suite
    /// "BIOS out-of-bounds load" / "Out-of-bounds load" expect the destination untouched).
    fn cpu_set_source_rejected(src: u32, span: u32) -> bool {
        src < 0x0200_0000 || src.wrapping_add(span) < 0x0200_0000
    }

    /// CpuSet: r0 src, r1 dst, r2 = count (bits 0-20) | fixed-source fill (bit 24) | 32-bit (bit 26).
    /// Addresses are passed to the bus unaligned, exactly as the BIOS loop does: the 16-bit loop
    /// is LDRH/STRH, so an odd source yields the aligned halfword rotated right by 8 and STRH
    /// stores its low half; the 32-bit loop is LDMIA/STMIA, which never rotates. RAM ignores the
    /// low address bits, the 8-bit SRAM bus uses them to pick the byte lane.
    fn hle_cpu_set(&mut self, regs: &mut Regs) {
        let mut src = regs.r[0];
        let mut dst = regs.r[1];
        let ctl = regs.r[2];
        let count = ctl & 0x1F_FFFF;
        let fill = ctl & (1 << 24) != 0;
        let word = ctl & (1 << 26) != 0;
        if Self::cpu_set_source_rejected(src, count * if word { 4 } else { 2 }) {
            return;
        }
        if word {
            for _ in 0..count {
                let v = self.read32(src, Access::S);
                self.write32(dst, v, Access::S);
                if !fill {
                    src = src.wrapping_add(4);
                }
                dst = dst.wrapping_add(4);
            }
        } else {
            for _ in 0..count {
                let v = self.read16(src, Access::S) as u32;
                let v = if src & 1 != 0 { v.rotate_right(8) } else { v };
                self.write16(dst, v as u16, Access::S);
                if !fill {
                    src = src.wrapping_add(2);
                }
                dst = dst.wrapping_add(2);
            }
        }
    }

    /// CpuFastSet: 32-bit units in blocks of 8 words; count rounded up to a multiple of 8.
    /// LDMIA/STMIA, so unaligned addresses are passed through unrotated like the 32-bit CpuSet.
    fn hle_cpu_fast_set(&mut self, regs: &mut Regs) {
        let mut src = regs.r[0];
        let mut dst = regs.r[1];
        let ctl = regs.r[2];
        let count = ((ctl & 0x1F_FFFF) + 7) & !7;
        let fill = ctl & (1 << 24) != 0;
        if Self::cpu_set_source_rejected(src, count * 4) {
            return;
        }
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
    /// Always sets DISPCNT = 0080h (forced blank).
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
        }
        self.io_write16(0x0400_0000, 0x0080);
    }

    // ------------------------------------------------------------------ affine

    /// BgAffineSet: src entries of 20 bytes (ox s32, oy s32, cx s16, cy s16, sx s16, sy s16,
    /// theta u16), dst entries of 16 bytes (pa, pb, pc, pd, x, y). Only the top 8 bits of theta count.
    fn hle_bg_affine_set(&mut self, regs: &mut Regs) {
        let mut src = regs.r[0];
        let mut dst = regs.r[1];
        let n = regs.r[2];
        for _ in 0..n {
            let ox = self.read32(src, Access::S) as i32 as f64 / 256.0;
            let oy = self.read32(src + 4, Access::S) as i32 as f64 / 256.0;
            let cx = self.read16(src + 8, Access::S) as i16 as f64;
            let cy = self.read16(src + 10, Access::S) as i16 as f64;
            let sx = self.read16(src + 12, Access::S) as i16 as f64 / 256.0;
            let sy = self.read16(src + 14, Access::S) as i16 as f64 / 256.0;
            let theta = (self.read16(src + 16, Access::S) >> 8) as f64 / 128.0 * std::f64::consts::PI;
            src = src.wrapping_add(20);
            let (sin, cos) = theta.sin_cos();
            let a = cos * sx;
            let b = -sin * sx;
            let c = sin * sy;
            let d = cos * sy;
            let rx = ox - (a * cx + b * cy);
            let ry = oy - (c * cx + d * cy);
            self.write16(dst, (a * 256.0) as i32 as i16 as u16, Access::S);
            self.write16(dst + 2, (b * 256.0) as i32 as i16 as u16, Access::S);
            self.write16(dst + 4, (c * 256.0) as i32 as i16 as u16, Access::S);
            self.write16(dst + 6, (d * 256.0) as i32 as i16 as u16, Access::S);
            self.write32(dst + 8, (rx * 256.0) as i32 as u32, Access::S);
            self.write32(dst + 12, (ry * 256.0) as i32 as u32, Access::S);
            dst = dst.wrapping_add(16);
        }
    }

    /// ObjAffineSet: src entries of 8 bytes (sx s16, sy s16, theta u16, pad), four s16 results
    /// written every r3 bytes.
    fn hle_obj_affine_set(&mut self, regs: &mut Regs) {
        let mut src = regs.r[0];
        let mut dst = regs.r[1];
        let n = regs.r[2];
        let diff = regs.r[3];
        for _ in 0..n {
            let sx = self.read16(src, Access::S) as i16 as f64 / 256.0;
            let sy = self.read16(src + 2, Access::S) as i16 as f64 / 256.0;
            let theta = (self.read16(src + 4, Access::S) >> 8) as f64 / 128.0 * std::f64::consts::PI;
            src = src.wrapping_add(8);
            let (sin, cos) = theta.sin_cos();
            let a = cos * sx;
            let b = -sin * sx;
            let c = sin * sy;
            let d = cos * sy;
            self.write16(dst, (a * 256.0) as i32 as i16 as u16, Access::S);
            self.write16(dst.wrapping_add(diff), (b * 256.0) as i32 as i16 as u16, Access::S);
            self.write16(dst.wrapping_add(diff * 2), (c * 256.0) as i32 as i16 as u16, Access::S);
            self.write16(dst.wrapping_add(diff * 3), (d * 256.0) as i32 as i16 as u16, Access::S);
            dst = dst.wrapping_add(diff * 4);
        }
    }

    // ------------------------------------------------------------------ decompression

    /// Write helper for the "Vram" variants: bytes are latched into halfwords.
    fn out16_latch(&mut self, dst: &mut u32, latch: &mut u16, b: u8) {
        if *dst & 1 == 0 {
            *latch = b as u16;
        } else {
            *latch |= (b as u16) << 8;
            self.write16(*dst & !1, *latch, Access::S);
        }
        *dst = dst.wrapping_add(1);
    }

    fn out8(&mut self, dst: &mut u32, b: u8) {
        self.write8(*dst, b, Access::S);
        *dst = dst.wrapping_add(1);
    }

    /// Read back an already-written destination byte (LZ77 back-references). For the VRAM
    /// variant the byte may still be in the latch.
    fn dest_byte(&mut self, addr: u32, cur_dst: u32, latch: u16, vram: bool) -> u8 {
        if vram && addr == (cur_dst & !1) && cur_dst & 1 == 1 {
            return latch as u8;
        }
        if vram {
            let h = self.read16(addr & !1, Access::S);
            if addr & 1 == 0 {
                h as u8
            } else {
                (h >> 8) as u8
            }
        } else {
            self.read8(addr, Access::S)
        }
    }

    /// BitUnPack: r0 src, r1 dst (word aligned), r2 -> {u16 len, u8 src_bits, u8 dst_bits, u32 offset|zero_flag<<31}.
    fn hle_bit_unpack(&mut self, regs: &mut Regs) {
        let mut src = regs.r[0];
        let mut dst = regs.r[1] & !3;
        let info = regs.r[2];
        let len = self.read16(info, Access::N) as u32;
        let src_bits = self.read8(info + 2, Access::S) as u32;
        let dst_bits = self.read8(info + 3, Access::S) as u32;
        let off_word = self.read32(info + 4, Access::S);
        let offset = off_word & 0x7FFF_FFFF;
        let zero_flag = off_word & 0x8000_0000 != 0;
        if !matches!(src_bits, 1 | 2 | 4 | 8) || !matches!(dst_bits, 1 | 2 | 4 | 8 | 16 | 32) {
            return;
        }
        let src_mask = if src_bits == 32 { u32::MAX } else { (1u32 << src_bits) - 1 };
        let mut acc: u32 = 0;
        let mut acc_bits: u32 = 0;
        for _ in 0..len {
            let byte = self.read8(src, Access::S) as u32;
            src = src.wrapping_add(1);
            let mut bit = 0;
            while bit < 8 {
                let mut unit = (byte >> bit) & src_mask;
                if unit != 0 || zero_flag {
                    unit = unit.wrapping_add(offset);
                }
                if dst_bits < 32 {
                    unit &= (1u32 << dst_bits) - 1;
                    acc |= unit << acc_bits;
                } else {
                    acc = unit;
                }
                acc_bits += dst_bits;
                if acc_bits >= 32 {
                    self.write32(dst, acc, Access::S);
                    dst = dst.wrapping_add(4);
                    acc = 0;
                    acc_bits = 0;
                }
                bit += src_bits;
            }
        }
        if acc_bits > 0 {
            self.write32(dst, acc, Access::S);
        }
    }

    /// LZ77UnComp: header (type 1, size in bits 8-31), flag byte + 8 blocks; compressed block =
    /// (len-3)<<4 | disp_hi, disp_lo; copies from dst - disp - 1.
    fn hle_lz77(&mut self, regs: &mut Regs, vram: bool) {
        let mut src = regs.r[0] & !3;
        let mut dst = regs.r[1];
        if vram {
            dst &= !1;
        }
        let header = self.read32(src, Access::N);
        if (header >> 4) & 0xF != 1 {
            return;
        }
        let mut remaining = header >> 8;
        src = src.wrapping_add(4);
        let mut latch: u16 = 0;
        while remaining > 0 {
            let flags = self.read8(src, Access::S);
            src = src.wrapping_add(1);
            for i in 0..8 {
                if remaining == 0 {
                    break;
                }
                if flags & (0x80 >> i) == 0 {
                    let b = self.read8(src, Access::S);
                    src = src.wrapping_add(1);
                    if vram {
                        self.out16_latch(&mut dst, &mut latch, b);
                    } else {
                        self.out8(&mut dst, b);
                    }
                    remaining -= 1;
                } else {
                    let b1 = self.read8(src, Access::S) as u32;
                    let b2 = self.read8(src + 1, Access::S) as u32;
                    src = src.wrapping_add(2);
                    let len = (b1 >> 4) + 3;
                    let disp = ((b1 & 0xF) << 8) | b2;
                    for _ in 0..len {
                        if remaining == 0 {
                            break;
                        }
                        let from = dst.wrapping_sub(disp + 1);
                        let b = self.dest_byte(from, dst, latch, vram);
                        if vram {
                            self.out16_latch(&mut dst, &mut latch, b);
                        } else {
                            self.out8(&mut dst, b);
                        }
                        remaining -= 1;
                    }
                }
            }
        }
    }

    /// RLUnComp: header (type 3), flag byte: bit 7 = run, bits 0-6 = length-3 (run) / length-1 (raw).
    fn hle_rl(&mut self, regs: &mut Regs, vram: bool) {
        let mut src = regs.r[0] & !3;
        let mut dst = regs.r[1];
        if vram {
            dst &= !1;
        }
        let header = self.read32(src, Access::N);
        if (header >> 4) & 0xF != 3 {
            return;
        }
        let mut remaining = header >> 8;
        src = src.wrapping_add(4);
        let mut latch: u16 = 0;
        while remaining > 0 {
            let flag = self.read8(src, Access::S);
            src = src.wrapping_add(1);
            if flag & 0x80 != 0 {
                let len = ((flag & 0x7F) as u32 + 3).min(remaining);
                let b = self.read8(src, Access::S);
                src = src.wrapping_add(1);
                for _ in 0..len {
                    if vram {
                        self.out16_latch(&mut dst, &mut latch, b);
                    } else {
                        self.out8(&mut dst, b);
                    }
                }
                remaining -= len;
            } else {
                let len = ((flag & 0x7F) as u32 + 1).min(remaining);
                for _ in 0..len {
                    let b = self.read8(src, Access::S);
                    src = src.wrapping_add(1);
                    if vram {
                        self.out16_latch(&mut dst, &mut latch, b);
                    } else {
                        self.out8(&mut dst, b);
                    }
                }
                remaining -= len;
            }
        }
    }

    /// HuffUnComp: header (type 2, data size 4 or 8 bits), tree size byte, tree table, 32-bit
    /// bitstream words (bit 31 first). Output written in 32-bit units.
    fn hle_huffman(&mut self, regs: &mut Regs) {
        let src = regs.r[0] & !3;
        let mut dst = regs.r[1] & !3;
        let header = self.read32(src, Access::N);
        if (header >> 4) & 0xF != 2 {
            return;
        }
        let data_bits = header & 0xF;
        if data_bits == 0 || data_bits > 8 {
            return;
        }
        let mut remaining_bytes = header >> 8;
        let tree_size = self.read8(src + 4, Access::S) as u32;
        let tree_base = src + 5;
        let mut bit_src = src + 4 + (tree_size + 1) * 2;
        let mut node_addr = tree_base;
        let mut acc: u32 = 0;
        let mut acc_bits: u32 = 0;
        'outer: while remaining_bytes > 0 {
            let word = self.read32(bit_src, Access::S);
            bit_src = bit_src.wrapping_add(4);
            for bit in (0..32).rev() {
                let node = self.read8(node_addr, Access::S);
                let go1 = (word >> bit) & 1 != 0;
                let child_base = (node_addr & !1) + ((node & 0x3F) as u32) * 2 + 2;
                let (next, is_data) = if go1 { (child_base + 1, node & 0x40 != 0) } else { (child_base, node & 0x80 != 0) };
                if is_data {
                    let data = self.read8(next, Access::S) as u32;
                    acc |= (data & ((1u32 << data_bits) - 1)) << acc_bits;
                    acc_bits += data_bits;
                    node_addr = tree_base;
                    if acc_bits >= 32 {
                        self.write32(dst, acc, Access::S);
                        dst = dst.wrapping_add(4);
                        acc = 0;
                        acc_bits = 0;
                        remaining_bytes = remaining_bytes.saturating_sub(4);
                        if remaining_bytes == 0 {
                            break 'outer;
                        }
                    }
                } else {
                    node_addr = next;
                }
            }
        }
    }

    /// Diff8bitUnFilter: header (type 8, size 1), first byte literal then cumulative sums.
    fn hle_diff8(&mut self, regs: &mut Regs, vram: bool) {
        let mut src = regs.r[0] & !3;
        let mut dst = regs.r[1];
        if vram {
            dst &= !1;
        }
        let header = self.read32(src, Access::N);
        if (header >> 4) & 0xF != 8 || header & 0xF != 1 {
            return;
        }
        let mut remaining = header >> 8;
        src = src.wrapping_add(4);
        let mut latch: u16 = 0;
        let mut acc: u8 = 0;
        while remaining > 0 {
            acc = acc.wrapping_add(self.read8(src, Access::S));
            src = src.wrapping_add(1);
            if vram {
                self.out16_latch(&mut dst, &mut latch, acc);
            } else {
                self.out8(&mut dst, acc);
            }
            remaining -= 1;
        }
    }

    /// Diff16bitUnFilter: header (type 8, size 2), halfword units.
    fn hle_diff16(&mut self, regs: &mut Regs) {
        let mut src = regs.r[0] & !3;
        let mut dst = regs.r[1] & !1;
        let header = self.read32(src, Access::N);
        if (header >> 4) & 0xF != 8 || header & 0xF != 2 {
            return;
        }
        let mut remaining = header >> 8;
        src = src.wrapping_add(4);
        let mut acc: u16 = 0;
        while remaining >= 2 {
            acc = acc.wrapping_add(self.read16(src, Access::S));
            src = src.wrapping_add(2);
            self.write16(dst, acc, Access::S);
            dst = dst.wrapping_add(2);
            remaining -= 2;
        }
    }
}

/// Signed division as the BIOS does it. Returns (quotient, remainder, |quotient|).
/// Division by zero: the real BIOS hangs; we return the values mGBA does so games do not lock up.
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

/// ArcTan as the BIOS computes it (polynomial in 1.14 fixed point). Returns (r0, r1, r3) with the
/// intermediate values the BIOS leaves in r1/r3.
pub fn arctan(i: i32) -> (i32, i32, i32) {
    // 32-bit ARM arithmetic throughout: the BIOS uses plain MUL/ADD, so every step wraps.
    let a = (i.wrapping_mul(i) >> 14).wrapping_neg();
    let mut b = (0xA9i32.wrapping_mul(a) >> 14).wrapping_add(0x390);
    b = (b.wrapping_mul(a) >> 14).wrapping_add(0x91C);
    b = (b.wrapping_mul(a) >> 14).wrapping_add(0xFB6);
    b = (b.wrapping_mul(a) >> 14).wrapping_add(0x16AA);
    b = (b.wrapping_mul(a) >> 14).wrapping_add(0x2081);
    b = (b.wrapping_mul(a) >> 14).wrapping_add(0x3651);
    b = (b.wrapping_mul(a) >> 14).wrapping_add(0xA2F9);
    ((i.wrapping_mul(b)) >> 16, a, b)
}

/// ArcTan2 as the BIOS computes it: quadrant correction around `arctan`. Returns (r0, r1).
pub fn arctan2(x: i32, y: i32) -> (i32, i32) {
    let at = |v: i32| -> (i32, i32) {
        let (r0, r1, _) = arctan(v);
        (r0, r1)
    };
    if y == 0 {
        return (if x >= 0 { 0 } else { 0x8000 }, 0);
    }
    if x == 0 {
        // The BIOS leaves y in r1 on this path (mgba suite "ArcTan2 00000000,y").
        return (if y >= 0 { 0x4000 } else { 0xC000 }, y);
    }
    if y >= 0 {
        if x >= 0 {
            if x >= y {
                return at((y << 14) / x);
            }
        } else if -x >= y {
            let (r0, r1) = at((y << 14) / x);
            return (r0 + 0x8000, r1);
        }
        let (r0, r1) = at((x << 14) / y);
        (0x4000 - r0, r1)
    } else {
        if x <= 0 {
            if -x > -y {
                let (r0, r1) = at((y << 14) / x);
                return (r0 + 0x8000, r1);
            }
        } else if x >= -y {
            let (r0, r1) = at((y << 14) / x);
            return (r0 + 0x10000, r1);
        }
        let (r0, r1) = at((x << 14) / y);
        (0xC000 - r0, r1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bios::Bios;
    use crate::cart::Cartridge;

    fn bus() -> Bus {
        let rom = vec![0u8; 0x100];
        Bus::new(Cartridge::new(rom, None).unwrap(), Bios::hle())
    }

    fn poke(bus: &mut Bus, addr: u32, data: &[u8]) {
        for (i, &b) in data.iter().enumerate() {
            bus.write8(addr + i as u32, b, Access::N);
        }
    }

    fn peek(bus: &Bus, addr: u32, n: usize) -> Vec<u8> {
        (0..n).map(|i| bus.peek8(addr + i as u32)).collect()
    }

    fn swi(bus: &mut Bus, num: u8, r: [u32; 4]) -> Regs {
        let mut regs = Regs::new();
        regs.set_cpsr(MODE_SYS);
        regs.r[..4].copy_from_slice(&r);
        regs.r[15] = 0x0800_0004;
        assert!(bus.hle_swi(num, &mut regs));
        regs
    }

    #[test]
    fn div_matches_bios_contract() {
        assert_eq!(hle_div(7, 2), (3, 1, 3));
        assert_eq!(hle_div(-1234, 10), (-123, -4, 123));
        assert_eq!(hle_div(7, -2), (-3, 1, 3));
        assert_eq!(hle_div(i32::MIN, -1), (i32::MIN, 0, 0x8000_0000));
    }

    #[test]
    fn sqrt_floor() {
        assert_eq!(isqrt(0), 0);
        assert_eq!(isqrt(1), 1);
        assert_eq!(isqrt(15), 3);
        assert_eq!(isqrt(16), 4);
        assert_eq!(isqrt(2 << 30), 46340);
        assert_eq!(isqrt(u32::MAX), 0xFFFF);
    }

    #[test]
    fn arctan_known_points() {
        // tan(0) = 0 -> 0; tan(pi/4) = 1.0 (0x4000) -> pi/4 = 0x2000 (within BIOS polynomial error).
        assert_eq!(arctan(0).0, 0);
        let q = arctan(0x4000).0;
        assert!((q - 0x2000).abs() <= 8, "got {q:#x}");
        let nq = arctan(-0x4000).0;
        assert!((nq + 0x2000).abs() <= 8, "got {nq:#x}");
        // ArcTan2 axes.
        assert_eq!(arctan2(0x100, 0).0, 0);
        assert_eq!(arctan2(0, 0x100).0, 0x4000);
        assert_eq!(arctan2(-0x100, 0).0, 0x8000);
        assert_eq!(arctan2(0, -0x100).0, 0xC000);
        // 45 degrees in each quadrant.
        assert!((arctan2(0x100, 0x100).0 - 0x2000).abs() <= 8);
        assert!((arctan2(-0x100, 0x100).0 - 0x6000).abs() <= 8);
        assert!((arctan2(-0x100, -0x100).0 - 0xA000).abs() <= 8);
        assert!((arctan2(0x100, -0x100).0 - 0xE000).abs() <= 8);
    }

    // ---------------------------------------------------------------- encoders for round trips

    fn lz77_encode(data: &[u8]) -> Vec<u8> {
        let mut out = vec![0x10, (data.len() & 0xFF) as u8, ((data.len() >> 8) & 0xFF) as u8, ((data.len() >> 16) & 0xFF) as u8];
        let mut i = 0;
        while i < data.len() {
            let flag_pos = out.len();
            out.push(0);
            let mut flags = 0u8;
            for b in 0..8 {
                if i >= data.len() {
                    break;
                }
                // Find the longest match within the 4096-byte window (min length 3).
                let mut best_len = 0;
                let mut best_disp = 0;
                let start = i.saturating_sub(0x1000);
                for j in start..i {
                    let mut l = 0;
                    while l < 18 && i + l < data.len() && data[j + l] == data[i + l] {
                        l += 1;
                    }
                    if l > best_len {
                        best_len = l;
                        best_disp = i - j - 1;
                    }
                }
                if best_len >= 3 {
                    flags |= 0x80 >> b;
                    out.push((((best_len - 3) as u8) << 4) | ((best_disp >> 8) as u8 & 0xF));
                    out.push((best_disp & 0xFF) as u8);
                    i += best_len;
                } else {
                    out.push(data[i]);
                    i += 1;
                }
            }
            out[flag_pos] = flags;
        }
        while out.len() % 4 != 0 {
            out.push(0);
        }
        out
    }

    fn rl_encode(data: &[u8]) -> Vec<u8> {
        let mut out = vec![0x30, (data.len() & 0xFF) as u8, ((data.len() >> 8) & 0xFF) as u8, ((data.len() >> 16) & 0xFF) as u8];
        let mut i = 0;
        while i < data.len() {
            let mut run = 1;
            while i + run < data.len() && data[i + run] == data[i] && run < 130 {
                run += 1;
            }
            if run >= 3 {
                out.push(0x80 | (run - 3) as u8);
                out.push(data[i]);
                i += run;
            } else {
                let mut n = 0;
                while i + n < data.len() && n < 128 {
                    let ahead = i + n;
                    if ahead + 2 < data.len() && data[ahead] == data[ahead + 1] && data[ahead] == data[ahead + 2] {
                        break;
                    }
                    n += 1;
                }
                let n = n.max(1);
                out.push((n - 1) as u8);
                out.extend_from_slice(&data[i..i + n]);
                i += n;
            }
        }
        while out.len() % 4 != 0 {
            out.push(0);
        }
        out
    }

    /// Minimal Huffman encoder for 8-bit symbols: builds a tree, emits the BIOS table + bitstream.
    fn huff_encode(data: &[u8]) -> Vec<u8> {
        #[derive(Clone)]
        enum Node {
            Leaf(u8),
            Inner(Box<Node>, Box<Node>),
        }
        let mut freq = [0usize; 256];
        for &b in data {
            freq[b as usize] += 1;
        }
        let mut heap: Vec<(usize, usize, Node)> = freq.iter().enumerate().filter(|(_, &f)| f > 0).map(|(s, &f)| (f, s, Node::Leaf(s as u8))).collect();
        if heap.len() == 1 {
            let (f, s, n) = heap[0].clone();
            heap.push((f, s + 1, n));
        }
        let mut seq = 256;
        while heap.len() > 1 {
            heap.sort_by_key(|(f, s, _)| (*f, *s));
            let (f0, _, n0) = heap.remove(0);
            let (f1, _, n1) = heap.remove(0);
            heap.push((f0 + f1, seq, Node::Inner(Box::new(n0), Box::new(n1))));
            seq += 1;
        }
        let root = heap.remove(0).2;
        // Codes.
        let mut codes: Vec<Vec<bool>> = vec![Vec::new(); 256];
        fn walk(n: &Node, path: &mut Vec<bool>, codes: &mut Vec<Vec<bool>>) {
            match n {
                Node::Leaf(s) => codes[*s as usize] = path.clone(),
                Node::Inner(a, b) => {
                    path.push(false);
                    walk(a, path, codes);
                    path.pop();
                    path.push(true);
                    walk(b, path, codes);
                    path.pop();
                }
            }
        }
        walk(&root, &mut Vec::new(), &mut codes);
        // Tree table in BIOS layout: breadth-first, children of node at index i placed as a pair;
        // offset field = (pair_index - (i & !1)) / 2 - 1.
        let mut table: Vec<u8> = Vec::new();
        let mut queue: std::collections::VecDeque<(usize, Node)> = std::collections::VecDeque::new();
        table.push(0); // root placeholder
        queue.push_back((0, root));
        while let Some((idx, node)) = queue.pop_front() {
            if let Node::Inner(a, b) = node {
                let pair = table.len();
                table.push(0);
                table.push(0);
                // Offsets are computed in address space: the table starts at src+5, so node i lives
                // at relative address 5+i and its children at ((5+i) & !1) + offset*2 + 2.
                let node_addr = 5 + idx;
                let pair_addr = 5 + pair;
                let offset = (pair_addr - (node_addr & !1)) / 2 - 1;
                assert!(offset < 64, "tree too deep for the BIOS format");
                let mut v = offset as u8;
                if let Node::Leaf(s) = *a {
                    v |= 0x80;
                    table[pair] = s;
                } else {
                    queue.push_back((pair, *a));
                }
                if let Node::Leaf(s) = *b {
                    v |= 0x40;
                    table[pair + 1] = s;
                } else {
                    queue.push_back((pair + 1, *b));
                }
                table[idx] = v;
            }
        }
        while (table.len() + 1) % 2 != 0 {
            table.push(0);
        }
        let tree_size_field = ((table.len() + 1) / 2 - 1) as u8;
        let mut out = vec![0x28, (data.len() & 0xFF) as u8, ((data.len() >> 8) & 0xFF) as u8, ((data.len() >> 16) & 0xFF) as u8];
        out.push(tree_size_field);
        out.extend_from_slice(&table);
        assert_eq!(out.len(), 4 + (tree_size_field as usize + 1) * 2);
        // Bitstream.
        let mut word: u32 = 0;
        let mut nbits = 0;
        for &b in data {
            for &bit in &codes[b as usize] {
                word |= (bit as u32) << (31 - nbits);
                nbits += 1;
                if nbits == 32 {
                    out.extend_from_slice(&word.to_le_bytes());
                    word = 0;
                    nbits = 0;
                }
            }
        }
        if nbits > 0 {
            out.extend_from_slice(&word.to_le_bytes());
        }
        out
    }

    fn sample_data() -> Vec<u8> {
        let mut d = Vec::new();
        for i in 0..64u32 {
            d.push((i * 7 % 13) as u8);
        }
        d.extend(std::iter::repeat(0xAB).take(40));
        d.extend_from_slice(b"Huffman and LZ77 both like repeated text. repeated text. repeated text.");
        while d.len() % 4 != 0 {
            d.push(0x11);
        }
        d
    }

    #[test]
    fn lz77_round_trip_wram_and_vram() {
        let data = sample_data();
        let enc = lz77_encode(&data);
        let mut b = bus();
        poke(&mut b, 0x0200_0000, &enc);
        swi(&mut b, swi::LZ77_UNCOMP_WRAM, [0x0200_0000, 0x0200_4000, 0, 0]);
        assert_eq!(peek(&b, 0x0200_4000, data.len()), data);
        swi(&mut b, swi::LZ77_UNCOMP_VRAM, [0x0200_0000, 0x0600_0000, 0, 0]);
        assert_eq!(peek(&b, 0x0600_0000, data.len()), data);
    }

    #[test]
    fn rl_round_trip_wram_and_vram() {
        let data = sample_data();
        let enc = rl_encode(&data);
        let mut b = bus();
        poke(&mut b, 0x0200_0000, &enc);
        swi(&mut b, swi::RL_UNCOMP_WRAM, [0x0200_0000, 0x0200_4000, 0, 0]);
        assert_eq!(peek(&b, 0x0200_4000, data.len()), data);
        swi(&mut b, swi::RL_UNCOMP_VRAM, [0x0200_0000, 0x0600_0000, 0, 0]);
        assert_eq!(peek(&b, 0x0600_0000, data.len()), data);
    }

    #[test]
    fn huffman_round_trip() {
        let data = sample_data();
        let enc = huff_encode(&data);
        let mut b = bus();
        poke(&mut b, 0x0200_0000, &enc);
        swi(&mut b, swi::HUFF_UNCOMP, [0x0200_0000, 0x0200_4000, 0, 0]);
        assert_eq!(peek(&b, 0x0200_4000, data.len()), data);
    }

    #[test]
    fn diff8_and_diff16_unfilter() {
        let data: Vec<u8> = (10u8..26).collect();
        let mut filt = vec![0x81, data.len() as u8, 0, 0, data[0]];
        for w in data.windows(2) {
            filt.push(w[1].wrapping_sub(w[0]));
        }
        while filt.len() % 4 != 0 {
            filt.push(0);
        }
        let mut b = bus();
        poke(&mut b, 0x0200_0000, &filt);
        swi(&mut b, swi::DIFF8_UNFILTER_WRAM, [0x0200_0000, 0x0200_4000, 0, 0]);
        assert_eq!(peek(&b, 0x0200_4000, data.len()), data);
        swi(&mut b, swi::DIFF8_UNFILTER_VRAM, [0x0200_0000, 0x0600_0000, 0, 0]);
        assert_eq!(peek(&b, 0x0600_0000, data.len()), data);

        let words: Vec<u16> = vec![1000, 1005, 990, 65535, 3];
        let mut f16 = vec![0x82, (words.len() * 2) as u8, 0, 0];
        f16.extend_from_slice(&words[0].to_le_bytes());
        for w in words.windows(2) {
            f16.extend_from_slice(&w[1].wrapping_sub(w[0]).to_le_bytes());
        }
        while f16.len() % 4 != 0 {
            f16.push(0);
        }
        poke(&mut b, 0x0200_8000, &f16);
        swi(&mut b, swi::DIFF16_UNFILTER, [0x0200_8000, 0x0200_C000, 0, 0]);
        let got: Vec<u16> = (0..words.len()).map(|i| b.peek16(0x0200_C000 + i as u32 * 2)).collect();
        assert_eq!(got, words);
    }

    #[test]
    fn bit_unpack_1bpp_to_4bpp_with_offset() {
        let mut b = bus();
        // 4 bytes of 1bpp data -> 32 units -> 16 bytes of 4bpp.
        let src = [0b1010_0101u8, 0xFF, 0x00, 0x81];
        poke(&mut b, 0x0200_0000, &src);
        // info: len=4, src 1 bit, dst 4 bits, offset 3 without zero flag.
        let info = [4u8, 0, 1, 4, 3, 0, 0, 0];
        poke(&mut b, 0x0200_0100, &info);
        swi(&mut b, swi::BIT_UNPACK, [0x0200_0000, 0x0200_4000, 0x0200_0100, 0]);
        let out = peek(&b, 0x0200_4000, 16);
        // Byte 0 bits LSB first: 1,0,1,0,0,1,0,1 -> units 4,0,4,0,0,4,0,4 packed two per byte.
        assert_eq!(&out[..4], &[0x04, 0x04, 0x40, 0x40]);
        assert_eq!(&out[4..8], &[0x44, 0x44, 0x44, 0x44]);
        assert_eq!(&out[8..12], &[0, 0, 0, 0]);
        assert_eq!(&out[12..16], &[0x04, 0x00, 0x00, 0x40]);
        // With the zero flag, zero units also get the offset.
        let info_z = [4u8, 0, 1, 4, 3, 0, 0, 0x80];
        poke(&mut b, 0x0200_0100, &info_z);
        swi(&mut b, swi::BIT_UNPACK, [0x0200_0000, 0x0200_5000, 0x0200_0100, 0]);
        let out = peek(&b, 0x0200_5000, 4);
        assert_eq!(out, vec![0x34, 0x34, 0x43, 0x43]);
    }

    #[test]
    fn cpu_set_and_fast_set_copy_and_fill() {
        let mut b = bus();
        let data: Vec<u8> = (0..64).collect();
        poke(&mut b, 0x0200_0000, &data);
        // 16-bit copy of 32 halfwords.
        swi(&mut b, swi::CPU_SET, [0x0200_0000, 0x0200_1000, 32, 0]);
        assert_eq!(peek(&b, 0x0200_1000, 64), data);
        // 32-bit fill of 8 words with the word at src.
        swi(&mut b, swi::CPU_SET, [0x0200_0000, 0x0200_2000, 8 | (1 << 24) | (1 << 26), 0]);
        assert_eq!(peek(&b, 0x0200_2000, 8), vec![0, 1, 2, 3, 0, 1, 2, 3]);
        // FastSet copy, count 9 rounds up to 16 words.
        swi(&mut b, swi::CPU_FAST_SET, [0x0200_0000, 0x0200_3000, 9, 0]);
        assert_eq!(peek(&b, 0x0200_3000, 64), data);
        // Source in BIOS is rejected.
        swi(&mut b, swi::CPU_SET, [0x0000_0000, 0x0200_4000, 4, 0]);
        assert_eq!(peek(&b, 0x0200_4000, 8), vec![0; 8]);
    }

    #[test]
    fn cpu_set_rejects_any_source_below_ewram() {
        let mut b = bus();
        b.write32(0x0200_4000, 0x1111_1111, Access::N);
        // BIOS mirror space, the unused area and a range whose end reaches back into them.
        for src in [0x0001_0000u32, 0x0100_0000, 0x01FF_FFF0] {
            swi(&mut b, swi::CPU_SET, [src, 0x0200_4000, 8, 0]);
            swi(&mut b, swi::CPU_SET, [src, 0x0200_4000, 8 | (1 << 26), 0]);
            swi(&mut b, swi::CPU_FAST_SET, [src, 0x0200_4000, 8, 0]);
        }
        assert_eq!(b.peek32(0x0200_4000), 0x1111_1111, "destination untouched");
        // The first EWRAM word is the lowest accepted source.
        swi(&mut b, swi::CPU_SET, [0x0200_0000, 0x0200_4000, 1 | (1 << 26), 0]);
        assert_eq!(b.peek32(0x0200_4000), b.peek32(0x0200_0000));
    }

    #[test]
    fn cpu_set_unaligned_source_follows_ldrh_and_ldm_semantics() {
        let mut b = bus();
        b.write32(0x0200_0000, 0xDEAD_BEEF, Access::N);
        b.write32(0x0200_0004, 0x3333_3333, Access::N);
        // 16-bit copy from src+1: LDRH rotates the aligned halfword (0xBEEF -> 0xEF0000BE) and
        // STRH keeps the low half, so the destination word becomes 0x00DE00BE (mgba-suite
        // "ROM load swi B 16 (unaligned)" pattern).
        swi(&mut b, swi::CPU_SET, [0x0200_0001, 0x0200_1000, 8, 0]);
        assert_eq!(b.peek32(0x0200_1000), 0x00DE_00BE);
        // 32-bit copy (LDMIA) from src+1..+3: no rotation, the aligned word is copied.
        for off in 1..4 {
            swi(&mut b, swi::CPU_SET, [0x0200_0000 + off, 0x0200_2000, 8 | (1 << 26), 0]);
            assert_eq!(b.peek32(0x0200_2000), 0xDEAD_BEEF, "CpuSet 32 unaligned {off}");
            swi(&mut b, swi::CPU_FAST_SET, [0x0200_0000 + off, 0x0200_3000, 8, 0]);
            assert_eq!(b.peek32(0x0200_3000), 0xDEAD_BEEF, "CpuFastSet unaligned {off}");
        }
        // An unaligned destination is aligned by RAM (STM/STRH ignore the low bits).
        swi(&mut b, swi::CPU_SET, [0x0200_0000, 0x0200_4002, 2 | (1 << 26), 0]);
        assert_eq!(b.peek32(0x0200_4000), 0xDEAD_BEEF);
        assert_eq!(b.peek32(0x0200_4004), 0x3333_3333);
    }

    #[test]
    fn cpu_set_on_the_sram_bus_uses_the_byte_lane_the_address_selects() {
        let rom = vec![0u8; 0x100];
        let mut b = Bus::new(Cartridge::new(rom, Some(crate::cart::SaveType::Sram32K)).unwrap(), Bios::hle());
        poke(&mut b, 0x0E00_0000, b"Game Boy");
        // 16-bit copy from SRAM+1: bytes 'a' and 'e' each replicated, rotated, low half stored.
        swi(&mut b, swi::CPU_SET, [0x0E00_0001, 0x0200_1000, 2, 0]);
        assert_eq!(b.peek32(0x0200_1000), 0x0065_0061);
        // 32-bit copy from SRAM+2: the byte at +2 replicated on all lanes.
        swi(&mut b, swi::CPU_SET, [0x0E00_0002, 0x0200_2000, 1 | (1 << 26), 0]);
        assert_eq!(b.peek32(0x0200_2000), 0x6D6D_6D6D);
        swi(&mut b, swi::CPU_FAST_SET, [0x0E00_0003, 0x0200_3000, 8, 0]);
        assert_eq!(b.peek32(0x0200_3000), 0x6565_6565);
        // 32-bit store to SRAM+1 writes only the byte lane the address selects.
        b.write32(0x0200_0000, 0xA5B6_C7D8, Access::N);
        swi(&mut b, swi::CPU_SET, [0x0200_0000, 0x0E00_0001, 1 | (1 << 26), 0]);
        assert_eq!(peek(&b, 0x0E00_0000, 2), vec![b'G', 0xC7]);
    }

    #[test]
    fn obj_affine_set_identity_and_scale() {
        let mut b = bus();
        // sx = 1.0, sy = 1.0, theta = 0.
        poke(&mut b, 0x0200_0000, &[0x00, 0x01, 0x00, 0x01, 0x00, 0x00, 0, 0]);
        swi(&mut b, swi::OBJ_AFFINE_SET, [0x0200_0000, 0x0200_1000, 1, 2]);
        assert_eq!(b.peek16(0x0200_1000), 0x0100);
        assert_eq!(b.peek16(0x0200_1002), 0x0000);
        assert_eq!(b.peek16(0x0200_1004), 0x0000);
        assert_eq!(b.peek16(0x0200_1006), 0x0100);
        // theta = 90 degrees (0x4000): pa = 0, pb = -sx, pc = sy, pd = 0; OAM stride 8.
        poke(&mut b, 0x0200_0000, &[0x00, 0x02, 0x00, 0x01, 0x00, 0x40, 0, 0]);
        swi(&mut b, swi::OBJ_AFFINE_SET, [0x0200_0000, 0x0200_2000, 1, 8]);
        assert_eq!(b.peek16(0x0200_2000) as i16, 0);
        assert_eq!(b.peek16(0x0200_2008) as i16, -0x200);
        assert_eq!(b.peek16(0x0200_2010) as i16, 0x100);
        assert_eq!(b.peek16(0x0200_2018) as i16, 0);
    }

    #[test]
    fn bg_affine_set_translation() {
        let mut b = bus();
        // ox = 100.0, oy = 50.0 (8.8), cx = 120, cy = 80, sx = sy = 1.0, theta = 0.
        let mut src = Vec::new();
        src.extend_from_slice(&(100i32 << 8).to_le_bytes());
        src.extend_from_slice(&(50i32 << 8).to_le_bytes());
        src.extend_from_slice(&120i16.to_le_bytes());
        src.extend_from_slice(&80i16.to_le_bytes());
        src.extend_from_slice(&0x100i16.to_le_bytes());
        src.extend_from_slice(&0x100i16.to_le_bytes());
        src.extend_from_slice(&0u16.to_le_bytes());
        poke(&mut b, 0x0200_0000, &src);
        swi(&mut b, swi::BG_AFFINE_SET, [0x0200_0000, 0x0200_1000, 1, 0]);
        assert_eq!(b.peek16(0x0200_1000), 0x100);
        assert_eq!(b.peek16(0x0200_1006), 0x100);
        assert_eq!(b.peek32(0x0200_1008) as i32, (100 - 120) << 8);
        assert_eq!(b.peek32(0x0200_100C) as i32, (50 - 80) << 8);
    }

    #[test]
    fn intr_wait_parks_then_returns() {
        let mut b = bus();
        b.write16(INTR_FLAGS_MIRROR, 0, Access::N);
        let regs = swi(&mut b, swi::VBLANK_INTR_WAIT, [0, 0, 0, 0]);
        assert!(b.irq.halted);
        assert!(b.irq.ime);
        assert_eq!(regs.r[15], 0x0800_0000, "PC rewound to the SWI");
        assert_eq!(b.bios.intr_wait, Some(1));
        // The user handler acknowledges V-Blank in the mirror; re-executing the SWI returns.
        b.irq.halted = false;
        b.write16(INTR_FLAGS_MIRROR, 1, Access::N);
        let regs = swi(&mut b, swi::VBLANK_INTR_WAIT, [1, 1, 0, 0]);
        assert!(!b.irq.halted);
        assert_eq!(regs.r[15], 0x0800_0004);
        assert_eq!(b.read16(INTR_FLAGS_MIRROR, Access::N), 0);
        assert_eq!(b.bios.intr_wait, None);
    }

    #[test]
    fn soft_reset_sets_stacks_and_jumps() {
        let mut b = bus();
        b.write8(SOFT_RESET_FLAG, 0, Access::N);
        let mut regs = Regs::new();
        regs.set_cpsr(MODE_SYS);
        regs.r[0] = 0xDEAD;
        b.hle_swi(swi::SOFT_RESET, &mut regs);
        assert_eq!(regs.mode(), MODE_SYS);
        assert_eq!(regs.r[13], 0x0300_7F00);
        assert_eq!(regs.sp_of(MODE_IRQ), 0x0300_7FA0);
        assert_eq!(regs.sp_of(MODE_SVC), 0x0300_7FE0);
        assert_eq!(regs.r[0], 0);
        assert_eq!(regs.r[15], 0x0800_0000);
        assert!(!regs.irq_disabled());
        b.write8(SOFT_RESET_FLAG, 1, Access::N);
        b.hle_swi(swi::SOFT_RESET, &mut regs);
        assert_eq!(regs.r[15], 0x0200_0000);
    }

    #[test]
    fn irq_vector_and_return_round_trip() {
        let mut b = bus();
        b.write32(IRQ_HANDLER_PTR, 0x0300_1000, Access::N);
        let mut regs = Regs::new();
        regs.set_cpsr(MODE_SYS | crate::cpu::regs::FLAG_T);
        regs.r[0] = 0x11;
        regs.r[3] = 0x33;
        regs.r[12] = 0xCC;
        regs.r[15] = 0x0800_0100;
        // Emulate CPU exception entry.
        let old = regs.cpsr;
        regs.switch_mode(MODE_IRQ);
        regs.set_spsr(old);
        regs.r[13] = 0x0300_7FA0;
        regs.r[14] = 0x0800_0104;
        regs.cpsr &= !crate::cpu::regs::FLAG_T;
        regs.cpsr |= crate::cpu::regs::FLAG_I;
        regs.r[15] = crate::cpu::VEC_IRQ;
        assert!(b.hle_vector(&mut regs));
        assert_eq!(regs.r[15], 0x0300_1000);
        assert_eq!(regs.r[0], 0x0400_0000);
        assert_eq!(regs.r[14], HLE_IRQ_RETURN_ADDR);
        assert_eq!(regs.r[13], 0x0300_7FA0 - 24);
        // Handler clobbers r0/r3/r12 then returns.
        regs.r[0] = 0;
        regs.r[3] = 0;
        regs.r[12] = 0;
        regs.r[15] = HLE_IRQ_RETURN_ADDR;
        assert!(b.hle_vector(&mut regs));
        assert_eq!(regs.r[0], 0x11);
        assert_eq!(regs.r[3], 0x33);
        assert_eq!(regs.r[12], 0xCC);
        assert_eq!(regs.sp_of(MODE_IRQ), 0x0300_7FA0, "IRQ stack fully popped");
        assert_eq!(regs.mode(), MODE_SYS);
        assert!(regs.thumb());
        assert_eq!(regs.r[15], 0x0800_0100);
    }
}
