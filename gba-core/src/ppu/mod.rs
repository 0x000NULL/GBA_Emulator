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
//! Rendering (M3): `render_scanline` draws the active BG layers (text, affine, bitmap) and the OBJ
//! layer into per-line buffers, then `compose` applies windows, priority, colour special effects
//! and green swap. Modules: `regs` (typed register views), `bg_text`, `bg_affine`, `bitmap`,
//! `obj`, `compose`.

mod bg_affine;
mod bg_text;
mod bitmap;
mod compose;
mod obj;
mod regs;
#[cfg(test)]
mod tests;

use crate::bus::SysCtx;
use crate::dma::DmaTrigger;
use crate::irq::Irq;
use crate::sched::Event;
use crate::timing::*;
use crate::{FrameBuffer, SCREEN_H, SCREEN_W};
use regs::{Mem, TRANSPARENT};

pub const PAL_SIZE: usize = 0x400;
pub const VRAM_SIZE: usize = 0x1_8000;
pub const OAM_SIZE: usize = 0x400;

/// Per-scanline scratch buffers (kept across lines to avoid reallocation).
struct Scratch {
    bg: [[u16; SCREEN_W]; 4],
    obj: obj::ObjLine,
    win: [u8; SCREEN_W],
}

fn new_fb() -> Box<FrameBuffer> {
    vec![0u16; SCREEN_W * SCREEN_H].into_boxed_slice().try_into().unwrap()
}

fn new_scratch() -> Box<Scratch> {
    Box::new(Scratch { bg: [[TRANSPARENT; SCREEN_W]; 4], obj: obj::ObjLine::new(), win: [0x3F; SCREEN_W] })
}

#[cfg_attr(feature = "savestate", derive(serde::Serialize, serde::Deserialize))]
pub struct Ppu {
    #[cfg_attr(feature = "savestate", serde(with = "crate::state::bytes_box"))]
    pub pal: Box<[u8; PAL_SIZE]>,
    #[cfg_attr(feature = "savestate", serde(with = "crate::state::bytes_box"))]
    pub vram: Box<[u8; VRAM_SIZE]>,
    #[cfg_attr(feature = "savestate", serde(with = "crate::state::bytes_box"))]
    pub oam: Box<[u8; OAM_SIZE]>,
    #[cfg_attr(feature = "savestate", serde(skip, default = "new_fb"))]
    pub fb: Box<FrameBuffer>,
    /// Raw LCD registers 0x000-0x05F as written (DISPSTAT read-only bits and VCOUNT kept separately).
    #[cfg_attr(feature = "savestate", serde(with = "crate::state::bytes_arr"))]
    pub io: [u8; 0x60],
    /// DISPSTAT bits 3-5 and 8-15 (writable part); bits 0-2 are derived from `vcount`/`hblank`.
    pub dispstat: u16,
    pub vcount: u32,
    pub hblank: bool,
    /// Set at the start of V-Blank; the frontend/harness clears it via `take_frame_ready`.
    pub frame_ready: bool,
    pub frame_count: u64,
    /// Internal affine reference points (BG2X/Y, BG3X/Y), latched at line 0 and after writes,
    /// advanced by PB/PD after every rendered line.
    pub bg_ref: [i32; 4],
    #[cfg_attr(feature = "savestate", serde(skip, default = "new_scratch"))]
    scratch: Box<Scratch>,
}

impl Default for Ppu {
    fn default() -> Self {
        Self::new()
    }
}

impl Ppu {
    pub fn new() -> Self {
        // Reset state: BG2PA/BG2PD/BG3PA/BG3PD hold the identity (1.0 = 0x0100); everything else 0.
        // Software (and the jsmolka PPU tests) relies on this without ever writing the parameters.
        let mut io = [0u8; 0x60];
        for off in [0x20usize, 0x26, 0x30, 0x36] {
            io[off + 1] = 0x01;
        }
        Ppu {
            pal: vec![0u8; PAL_SIZE].into_boxed_slice().try_into().unwrap(),
            vram: vec![0u8; VRAM_SIZE].into_boxed_slice().try_into().unwrap(),
            oam: vec![0u8; OAM_SIZE].into_boxed_slice().try_into().unwrap(),
            fb: vec![0u16; SCREEN_W * SCREEN_H].into_boxed_slice().try_into().unwrap(),
            io,
            dispstat: 0,
            vcount: 0,
            hblank: false,
            frame_ready: false,
            frame_count: 0,
            bg_ref: [0; 4],
            scratch: Box::new(Scratch {
                bg: [[TRANSPARENT; SCREEN_W]; 4],
                obj: obj::ObjLine::new(),
                win: [0x3F; SCREEN_W],
            }),
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

    /// `at` is the cycle the LineEnd event was scheduled for; the next line is timed from it, not
    /// from `sched.now`, so CPU/DMA overshoot never lengthens the frame.
    pub fn on_line_end(&mut self, ctx: &mut SysCtx, at: u64) {
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
        let line_start = at;
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

    /// Render one visible scanline into `fb` with the registers as they are right now.
    /// Forced blank (DISPCNT bit 7) outputs white. Afterwards the internal affine reference
    /// points advance by PB/PD (GBATEK "Internal Reference Point Registers").
    pub fn render_scanline(&mut self, y: usize) {
        debug_assert!(y < SCREEN_H);
        let Ppu { pal, vram, oam, io, fb, bg_ref, scratch, .. } = self;
        let mem = Mem { io, pal, vram, oam };
        let row = &mut fb[y * SCREEN_W..(y + 1) * SCREEN_W];
        let dispcnt = mem.dispcnt();
        if dispcnt & 0x80 != 0 {
            row.fill(0x7FFF);
            return;
        }
        let mode = dispcnt & 7;
        let mosaic = mem.mosaic();
        let Scratch { bg, obj, win } = &mut **scratch;

        // Which BGs exist in this mode (bit i = BGi), masked by the DISPCNT display enables.
        let mode_bgs: u16 = match mode {
            0 => 0b1111,
            1 => 0b0111,
            2 => 0b1100,
            3..=5 => 0b0100,
            _ => 0,
        };
        let enabled = mode_bgs & (dispcnt >> 8);
        let mut bg_active = [false; 4];
        for i in 0..4 {
            if enabled & (1 << i) == 0 {
                continue;
            }
            bg_active[i] = true;
            let cnt = mem.bgcnt(i);
            let text = mode == 0 || (mode == 1 && i < 2);
            if text {
                let ly = if cnt.mosaic { y - y % mosaic.bg_v } else { y };
                bg_text::render(&mem, i, ly, &mut bg[i]);
            } else {
                let [pa, pb, pc, pd] = mem.bg_affine_params(i);
                let r = if i == 2 { 0 } else { 2 };
                let (mut rx, mut ry) = (bg_ref[r], bg_ref[r + 1]);
                if cnt.mosaic {
                    // Line y shows what line y - y % v showed: back the reference up that many lines.
                    let back = (y % mosaic.bg_v) as i32;
                    rx = rx.wrapping_sub(pb.wrapping_mul(back));
                    ry = ry.wrapping_sub(pd.wrapping_mul(back));
                }
                if mode >= 3 {
                    bitmap::render(&mem, mode, rx, ry, pa, pc, &mut bg[i]);
                } else {
                    bg_affine::render(&mem, i, rx, ry, pa, pc, &mut bg[i]);
                }
            }
            if cnt.mosaic {
                compose::mosaic_h(&mut bg[i], mosaic.bg_h);
            }
        }

        obj::render(&mem, y, obj);
        compose::window_line(&mem, y, &obj.window, win);
        compose::compose_line(&mem, bg, bg_active, obj, win, row);
        if io[0x02] & 1 != 0 {
            compose::green_swap(row);
        }

        // Advance the internal reference points to the next line.
        let bg2 = mem.bg_affine_params(2);
        let bg3 = mem.bg_affine_params(3);
        bg_ref[0] = bg_ref[0].wrapping_add(bg2[1]);
        bg_ref[1] = bg_ref[1].wrapping_add(bg2[3]);
        bg_ref[2] = bg_ref[2].wrapping_add(bg3[1]);
        bg_ref[3] = bg_ref[3].wrapping_add(bg3[3]);
    }

    /// Returns true once per frame, at the start of V-Blank.
    pub fn take_frame_ready(&mut self) -> bool {
        std::mem::replace(&mut self.frame_ready, false)
    }
}

#[cfg(test)]
mod timing_tests {
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
                    ppu.on_line_end(&mut ctx, deadline);
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
