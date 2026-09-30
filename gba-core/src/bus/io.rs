//! IO register dispatch (0x04000000-0x040003FE plus the mGBA debug registers at 0x04FFF600).
//! Source: GBATEK "GBA I/O Map". Integrator-owned; subsystems own the semantics of their ranges.

use super::{Bus, SysCtx};

/// Is the 16-bit register at `off` (even, below 0x400) readable? Everything else in the register
/// block is write-only or unused. Source: GBATEK "GBA I/O Map" (R/W columns).
fn io_readable(off: u32) -> bool {
    matches!(
        off,
        // DISPCNT, Green Swap, DISPSTAT, VCOUNT, BG0-3CNT, WININ/WINOUT, BLDCNT/BLDALPHA.
        0x000..=0x00E | 0x048 | 0x04A | 0x050 | 0x052
        // SOUNDxCNT (not their unused trailing halfwords), SOUNDCNT_L/H/X, SOUNDBIAS, WAVE_RAM.
        | 0x060..=0x064 | 0x068 | 0x06C | 0x070..=0x074 | 0x078 | 0x07C | 0x080..=0x084 | 0x088 | 0x090..=0x09E
        // DMAxCNT_H.
        | 0x0BA | 0x0C6 | 0x0D2 | 0x0DE
        // TMxCNT_L/H.
        | 0x100..=0x10E
        // SIODATA32/SIOMULTI0-3, SIOCNT, SIODATA8; KEYINPUT, KEYCNT, RCNT; JOYCNT, JOY_RECV/TRANS, JOYSTAT.
        | 0x120..=0x12A | 0x130..=0x134 | 0x140 | 0x150..=0x158
        // IE, IF, WAITCNT, IME, POSTFLG.
        | 0x200..=0x204 | 0x208 | 0x300
    )
}

impl Bus {
    /// What a read of the unreadable IO halfword at `off` returns. GBATEK "Unpredictable Things",
    /// reading from unused or write-only I/O ports: open bus when the whole 32-bit word is unused
    /// and/or write-only (BG0HOFS, DMA0SAD, 0E0h), zero when the word contains a readable
    /// halfword (MOSAIC/04Eh style pairs, DMAxCNT_L next to DMAxCNT_H, 136h, 206h, 302h).
    #[inline]
    fn io_unmapped16(&self, off: u32) -> u16 {
        let w = off & !3;
        if io_readable(w) || io_readable(w + 2) {
            0
        } else {
            (self.open_bus >> ((off & 2) * 8)) as u16
        }
    }

    /// Side-effect-free IO read (debugger / composed reads).
    pub fn io_peek8(&self, addr: u32) -> u8 {
        let off = addr & 0x00FF_FFFF;
        if off < 0x400 && !io_readable(off & !1) {
            return (self.io_unmapped16(off & !1) >> ((off & 1) * 8)) as u8;
        }
        match off {
            0x000..=0x05F => self.ppu.read_io8(off as u16),
            0x060..=0x0AF => self.apu.read_io8(off as u16),
            0x0B0..=0x0DF => self.dma.read8(off as u16),
            0x100..=0x10F => self.timers.read8(off as u16, self.sched.now),
            0x120..=0x12F | 0x134..=0x15F => self.sio.peek8(off as u16),
            0x130..=0x133 => self.keypad.read8(off as u16),
            0x200 => self.irq.ie as u8,
            0x201 => (self.irq.ie >> 8) as u8,
            0x202 => self.irq.if_ as u8,
            0x203 => (self.irq.if_ >> 8) as u8,
            0x204 => self.waitcnt as u8,
            0x205 => (self.waitcnt >> 8) as u8,
            0x208 => self.irq.ime as u8,
            0x300 => self.postflg,
            0x800..=0x803 => (self.imc >> ((off & 3) * 8)) as u8,
            0xFF_F780 => 0xEA,
            0xFF_F781 => 0x1D,
            // Beyond the register block (and not the 4000800h port): nothing answers.
            _ => (self.open_bus >> ((off & 3) * 8)) as u8,
        }
    }

    pub fn io_read8(&mut self, addr: u32) -> u8 {
        let off = addr & 0x00FF_FFFF;
        match off {
            // Reading JOY_RECV has a side effect on JOYSTAT.
            0x150..=0x153 => self.sio.read8(off as u16),
            _ => self.io_peek8(addr),
        }
    }

    pub fn io_read16(&mut self, addr: u32) -> u16 {
        let a = addr & !1;
        self.io_read8(a) as u16 | (self.io_read8(a + 1) as u16) << 8
    }

    pub fn io_read32(&mut self, addr: u32) -> u32 {
        let a = addr & !3;
        self.io_read16(a) as u32 | (self.io_read16(a + 2) as u32) << 16
    }

    pub fn io_write8(&mut self, addr: u32, v: u8) {
        let off = addr & 0x00FF_FFFF;
        match off {
            0x000..=0x05F => {
                // Bits without storage never read back (GBATEK "LCD I/O BG Control", "Window
                // Feature", "Color Special Effects"): BG0/BG1CNT bit 13 (display area overflow is
                // BG2/BG3 only), WININ/WINOUT bits 6-7/14-15, BLDCNT bits 14-15, BLDALPHA bits
                // 5-7/13-15.
                let v = match off {
                    0x009 | 0x00B => v & 0xDF,
                    0x048..=0x04B => v & 0x3F,
                    0x051 => v & 0x3F,
                    0x052 | 0x053 => v & 0x1F,
                    _ => v,
                };
                let Bus { sched, irq, dma, ppu, .. } = self;
                ppu.write_io8(off as u16, v, &mut SysCtx { sched, irq, dma });
            }
            0x060..=0x0AF => {
                let Bus { sched, apu, .. } = self;
                apu.write_io8(off as u16, v, sched);
            }
            0x0B0..=0x0DF => {
                let Bus { sched, dma, .. } = self;
                dma.write8(off as u16, v, sched);
            }
            0x100..=0x10F => {
                let Bus { sched, timers, .. } = self;
                timers.write8(off as u16, v, sched);
            }
            0x120..=0x12F | 0x134..=0x15F => {
                let Bus { sched, sio, .. } = self;
                sio.write8(off as u16, v, sched);
            }
            0x130..=0x133 => {
                let Bus { keypad, irq, .. } = self;
                keypad.write8(off as u16, v, irq);
            }
            0x200 => self.irq.ie = (self.irq.ie & 0xFF00) | v as u16,
            0x201 => self.irq.ie = (self.irq.ie & 0x00FF) | ((v as u16 & 0x3F) << 8),
            0x202 => self.irq.write_if_ack(v as u16),
            0x203 => self.irq.write_if_ack((v as u16) << 8),
            0x204 => {
                self.waitcnt = (self.waitcnt & 0xFF00) | v as u16;
                self.update_waitstates();
            }
            0x205 => {
                self.waitcnt = (self.waitcnt & 0x00FF) | ((v as u16 & 0x5F) << 8);
                self.update_waitstates();
            }
            0x208 => self.irq.ime = v & 1 != 0,
            0x300 => self.postflg = v & 1,
            0x301 => self.irq.write_haltcnt(v),
            0x800..=0x803 => {
                let sh = (off & 3) * 8;
                self.imc = (self.imc & !(0xFF << sh)) | ((v as u32) << sh);
            }
            0xFF_F600..=0xFF_F6FF => self.debug.mgba_buf_write((off - 0xFF_F600) as usize, v),
            0xFF_F700 | 0xFF_F701 => self.debug.mgba_flags_write(off & 1, v),
            0xFF_F780 | 0xFF_F781 => self.debug.mgba_enable_write(off & 1, v),
            _ => {}
        }
    }

    pub fn io_write16(&mut self, addr: u32, v: u16) {
        let a = addr & !1;
        let off = a & 0x00FF_FFFF;
        match off {
            // Sound FIFOs take whole halfwords/words, not bytes.
            0x0A0 | 0x0A2 => self.apu.fifo_write(0, &v.to_le_bytes()),
            0x0A4 | 0x0A6 => self.apu.fifo_write(1, &v.to_le_bytes()),
            // SIOCNT must see both bytes at once: the low byte carries the start bit, the high
            // byte the mode and length that decide what that start means.
            0x120..=0x12E | 0x134..=0x15E => {
                let Bus { sched, sio, .. } = self;
                sio.write16(off as u16, v, sched);
            }
            _ => {
                self.io_write8(a, v as u8);
                self.io_write8(a + 1, (v >> 8) as u8);
            }
        }
    }

    pub fn io_write32(&mut self, addr: u32, v: u32) {
        let a = addr & !3;
        let off = a & 0x00FF_FFFF;
        match off {
            0x0A0 => self.apu.fifo_write(0, &v.to_le_bytes()),
            0x0A4 => self.apu.fifo_write(1, &v.to_le_bytes()),
            _ => {
                self.io_write16(a, v as u16);
                self.io_write16(a + 2, (v >> 16) as u16);
            }
        }
    }

    /// Human-readable dump of the named IO registers (side-effect free).
    pub fn dump_io(&self) -> String {
        let r16 = |off: u32| self.io_peek8(0x0400_0000 + off) as u16 | (self.io_peek8(0x0400_0000 + off + 1) as u16) << 8;
        let r32 = |off: u32| r16(off) as u32 | (r16(off + 2) as u32) << 16;
        let mut s = String::new();
        s.push_str(&format!(
            "DISPCNT={:04X} DISPSTAT={:04X} VCOUNT={:3} BG0CNT={:04X} BG1CNT={:04X} BG2CNT={:04X} BG3CNT={:04X}\n",
            r16(0x00), r16(0x04), r16(0x06), r16(0x08), r16(0x0A), r16(0x0C), r16(0x0E)
        ));
        s.push_str(&format!(
            "WIN0H={:04X} WIN1H={:04X} WIN0V={:04X} WIN1V={:04X} WININ={:04X} WINOUT={:04X} MOSAIC={:04X} BLDCNT={:04X} BLDALPHA={:04X} BLDY={:04X}\n",
            r16(0x40), r16(0x42), r16(0x44), r16(0x46), r16(0x48), r16(0x4A), r16(0x4C), r16(0x50), r16(0x52), r16(0x54)
        ));
        s.push_str(&format!(
            "SOUNDCNT_L={:04X} SOUNDCNT_H={:04X} SOUNDCNT_X={:04X} SOUNDBIAS={:04X}\n",
            r16(0x80), r16(0x82), r16(0x84), r16(0x88)
        ));
        for ch in 0..4u32 {
            let b = 0xB0 + ch * 12;
            s.push_str(&format!(
                "DMA{ch}: SAD={:08X} DAD={:08X} CNT_L={:04X} CNT_H={:04X}\n",
                r32(b), r32(b + 4), r16(b + 8), r16(b + 10)
            ));
        }
        for t in 0..4u32 {
            let b = 0x100 + t * 4;
            s.push_str(&format!("TM{t}: COUNT={:04X} CNT={:04X}  ", r16(b), r16(b + 2)));
        }
        s.push('\n');
        s.push_str(&format!(
            "KEYINPUT={:04X} KEYCNT={:04X} IE={:04X} IF={:04X} IME={} WAITCNT={:04X} POSTFLG={} HALT={} STOP={}\n",
            r16(0x130), r16(0x132), self.irq.ie, self.irq.if_, self.irq.ime as u8, self.waitcnt, self.postflg,
            self.irq.halted as u8, self.irq.stopped as u8
        ));
        s
    }
}
