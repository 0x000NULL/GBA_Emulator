//! PPU. Owned by the PPU agent (M3) except the scanline/V-Blank timing contract in this file,
//! which the integrator wrote and which the rest of the system depends on.
//!
//! Timing contract (see `timing.rs`, GBATEK "LCD I/O Interrupts and Status", "LCD Dimensions and Timings"):
//! - `Event::HBlank` at cycle `HDRAW_CYCLES` of every line: render line `vcount` if < 160 using the
//!   registers as they are at this moment, set the H-Blank flag, raise the H-Blank IRQ (every line,
//!   0..227), trigger H-Blank DMA (only lines 0..159) and video-capture DMA (lines 2..162).
//! - `Event::LineEnd` at cycle `CYCLES_PER_LINE`: clear H-Blank flag, vcount = (vcount+1) % 228,
//!   update V-Count match flag/IRQ, set V-Blank flag + IRQ + DMA at line 160, clear V-Blank flag at
//!   line 227, latch the affine reference points at line 0, mark the frame complete at line 160.
//!
//! M0 status: rendering is a STUB (backdrop colour). Everything else in this file is final.

use crate::bus::SysCtx;
use crate::dma::DmaTrigger;
use crate::irq::Irq;
use crate::sched::Event;
use crate::timing::*;
use crate::{FrameBuffer, SCREEN_H, SCREEN_W};

pub const PAL_SIZE: usize = 0x400;
pub const VRAM_SIZE: usize = 0x1_8000;
pub const OAM_SIZE: usize = 0x400;

pub struct Ppu {
    pub pal: Box<[u8; PAL_SIZE]>,
    pub vram: Box<[u8; VRAM_SIZE]>,
    pub oam: Box<[u8; OAM_SIZE]>,
    pub fb: Box<FrameBuffer>,
    /// Raw LCD registers 0x000-0x05F as written (DISPSTAT read-only bits and VCOUNT kept separately).
    pub io: [u8; 0x60],
    /// DISPSTAT bits 3-5 and 8-15 (writable part); bits 0-2 are derived from `vcount`/`hblank`.
    pub dispstat: u16,
    pub vcount: u32,
    pub hblank: bool,
    /// Set at the start of V-Blank; the frontend/harness clears it via `take_frame_ready`.
    pub frame_ready: bool,
    pub frame_count: u64,
    /// Internal affine reference points (BG2X/Y, BG3X/Y), latched at line 0 and after writes.
    pub bg_ref: [i32; 4],
}

impl Default for Ppu {
    fn default() -> Self {
        Self::new()
    }
}

impl Ppu {
    pub fn new() -> Self {
        Ppu {
            pal: vec![0u8; PAL_SIZE].into_boxed_slice().try_into().unwrap(),
            vram: vec![0u8; VRAM_SIZE].into_boxed_slice().try_into().unwrap(),
            oam: vec![0u8; OAM_SIZE].into_boxed_slice().try_into().unwrap(),
            fb: vec![0u16; SCREEN_W * SCREEN_H].into_boxed_slice().try_into().unwrap(),
            io: [0; 0x60],
            dispstat: 0,
            vcount: 0,
            hblank: false,
            frame_ready: false,
            frame_count: 0,
            bg_ref: [0; 4],
        }
    }

    #[inline]
    pub fn reg16(&self, off: usize) -> u16 {
        u16::from_le_bytes([self.io[off], self.io[off + 1]])
    }

    #[inline]
    pub fn dispcnt(&self) -> u16 {
        self.reg16(0x00)
    }

    /// DISPSTAT as read: V-Blank flag (line 160..=226), H-Blank flag, V-Count match, plus the writable bits.
    pub fn dispstat_read(&self) -> u16 {
        let vblank = (VISIBLE_LINES..=VBLANK_FLAG_LAST_LINE).contains(&self.vcount) as u16;
        let vmatch = (self.vcount == (self.dispstat >> 8) as u32) as u16;
        self.dispstat | vblank | (self.hblank as u16) << 1 | vmatch << 2
    }

    /// Schedule the first line events. HLE boot mimics the post-BIOS state: VCOUNT = 0x7E with the
    /// next LineEnd 117 cycles away (mGBA GBASkipBIOS), which keeps early traces aligned.
    pub fn boot_schedule(&mut self, ctx: &mut SysCtx, hle: bool) {
        if hle {
            self.vcount = 0x7E;
            self.hblank = true;
            ctx.sched.schedule_in(Event::LineEnd, 117);
        } else {
            self.vcount = 0;
            ctx.sched.schedule_in(Event::HBlank, HDRAW_CYCLES);
            ctx.sched.schedule_in(Event::LineEnd, CYCLES_PER_LINE);
        }
    }

    pub fn on_hblank(&mut self, ctx: &mut SysCtx) {
        if self.vcount < VISIBLE_LINES {
            self.render_scanline(self.vcount as usize);
            ctx.dma.trigger(DmaTrigger::HBlank);
        }
        if (VIDEO_CAPTURE_FIRST_LINE..=VIDEO_CAPTURE_LAST_LINE).contains(&self.vcount) {
            ctx.dma.trigger(DmaTrigger::VideoCapture);
        }
        self.hblank = true;
        if self.dispstat & (1 << 4) != 0 {
            ctx.irq.raise(Irq::HBlank);
        }
    }

    pub fn on_line_end(&mut self, ctx: &mut SysCtx) {
        self.hblank = false;
        self.vcount = (self.vcount + 1) % LINES_PER_FRAME;
        match self.vcount {
            0 => self.latch_affine_refs(),
            VISIBLE_LINES => {
                self.frame_ready = true;
                self.frame_count += 1;
                if self.dispstat & (1 << 3) != 0 {
                    ctx.irq.raise(Irq::VBlank);
                }
                ctx.dma.trigger(DmaTrigger::VBlank);
            }
            _ => {}
        }
        if self.dispstat & (1 << 5) != 0 && self.vcount == (self.dispstat >> 8) as u32 {
            ctx.irq.raise(Irq::VCount);
        }
        let line_start = ctx.sched.now;
        ctx.sched.schedule(Event::HBlank, line_start + HDRAW_CYCLES);
        ctx.sched.schedule(Event::LineEnd, line_start + CYCLES_PER_LINE);
    }

    fn latch_affine_refs(&mut self) {
        // BG2X 0x28, BG2Y 0x2C, BG3X 0x38, BG3Y 0x3C: 28-bit signed fixed point.
        for (i, off) in [0x28usize, 0x2C, 0x38, 0x3C].iter().enumerate() {
            let v = u32::from_le_bytes([self.io[*off], self.io[off + 1], self.io[off + 2], self.io[off + 3]]);
            self.bg_ref[i] = ((v << 4) as i32) >> 4;
        }
    }

    pub fn read_io8(&self, off: u16) -> u8 {
        let off = off as usize;
        match off {
            0x04 => self.dispstat_read() as u8,
            0x05 => (self.dispstat_read() >> 8) as u8,
            0x06 => self.vcount as u8,
            0x07 => 0,
            // Write-only registers read as 0: BGxHOFS/VOFS (0x10-0x1F), BG2/3 affine (0x20-0x3F),
            // WINxH/V (0x40-0x47), MOSAIC (0x4C), BLDY (0x54).
            0x10..=0x47 | 0x4C | 0x4D | 0x54..=0x5F => 0,
            0x00..=0x5F => self.io[off],
            _ => 0,
        }
    }

    pub fn write_io8(&mut self, off: u16, v: u8, _ctx: &mut SysCtx) {
        let off = off as usize;
        match off {
            0x04 => self.dispstat = (self.dispstat & 0xFF00) | (v as u16 & 0x38),
            0x05 => self.dispstat = (self.dispstat & 0x00FF) | ((v as u16) << 8),
            0x06 | 0x07 => {}
            0x00..=0x5F => {
                self.io[off] = v;
                // Writing a reference point register reloads the internal copy immediately.
                if (0x28..0x30).contains(&off) || (0x38..0x40).contains(&off) {
                    self.latch_affine_refs();
                }
            }
            _ => {}
        }
    }

    /// Render one visible scanline into `fb`. STUB: backdrop colour (palette entry 0), or black
    /// under forced blank. Replaced by the PPU agent in M3 (modes 0-5, OBJ, windows, blending, mosaic).
    pub fn render_scanline(&mut self, y: usize) {
        let dispcnt = self.dispcnt();
        let color = if dispcnt & 0x80 != 0 { 0x7FFF } else { u16::from_le_bytes([self.pal[0], self.pal[1]]) & 0x7FFF };
        let row = &mut self.fb[y * SCREEN_W..(y + 1) * SCREEN_W];
        row.fill(color);
    }

    /// Returns true once per frame, at the start of V-Blank.
    pub fn take_frame_ready(&mut self) -> bool {
        std::mem::replace(&mut self.frame_ready, false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dma::DmaController;
    use crate::irq::IrqController;
    use crate::sched::Scheduler;

    #[test]
    fn vblank_flag_and_irq_timing() {
        let mut sched = Scheduler::new();
        let mut irq = IrqController::new();
        let mut dma = DmaController::new();
        let mut ppu = Ppu::new();
        ppu.dispstat = 1 << 3; // V-Blank IRQ enable
        {
            let mut ctx = SysCtx { sched: &mut sched, irq: &mut irq, dma: &mut dma };
            ppu.boot_schedule(&mut ctx, false);
        }
        let mut lines = 0;
        while lines < 160 {
            let deadline = sched.next_deadline();
            sched.now = deadline;
            let (ev, _) = sched.pop_due().unwrap();
            let mut ctx = SysCtx { sched: &mut sched, irq: &mut irq, dma: &mut dma };
            match ev {
                Event::HBlank => ppu.on_hblank(&mut ctx),
                Event::LineEnd => {
                    ppu.on_line_end(&mut ctx);
                    lines += 1;
                }
                _ => unreachable!(),
            }
        }
        assert_eq!(ppu.vcount, 160);
        assert_eq!(sched.now, 160 * CYCLES_PER_LINE);
        assert!(ppu.dispstat_read() & 1 != 0);
        assert_eq!(irq.if_, Irq::VBlank as u16);
        assert!(ppu.frame_ready);
    }
}
