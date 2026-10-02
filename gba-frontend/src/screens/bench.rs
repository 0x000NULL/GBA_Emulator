//! D Bench: the developer view (`docs/gui/SCREENS.md`). Night palette. The game at 3x, a
//! 228-line scanline timeline of the last frame, per-layer pictures with show/hide ticks,
//! six channel meters with on/off ticks, and the CPU (registers, CPSR flags, disassembly).
//!
//! Data comes from `gba.bench()` (the core's capture, switched on only while this screen is
//! open) plus the always-available registers, DISPCNT, VCOUNT and side-effect-free peeks.

use crate::ui::fmt::{abbrev_save, fmt_frames};
use crate::ui::input::{Action, UiInput};
use crate::ui::text::Style;
use crate::ui::tokens::{font as f, layout as l, Palette, NIGHT};
use crate::ui::{widgets, Canvas};
use gba_core::cpu::disasm;
use gba_core::cpu::regs::{MODE_ABT, MODE_FIQ, MODE_IRQ, MODE_SVC, MODE_SYS, MODE_UND, MODE_USR};
use gba_core::{Gba, SaveType, CH_FIFO_A, CH_FIFO_B, CH_NOISE, CH_SQ1, CH_SQ2, CH_WAVE, EV_DMA1_FIFO, EV_DMA2_FIFO, EV_DMA3_START, EV_TIMER0_OVF, EV_VBLANK_IRQ, SCREEN_H, SCREEN_W};

pub const GAME_SCALE: u32 = 3;
const LAYER_NAMES: [&str; 5] = ["BG0", "BG1", "BG2", "BG3", "OBJ"];
const CHANNEL_NAMES: [&str; 6] = ["SQ1", "SQ2", "WAVE", "NOISE", "FIFO A", "FIFO B"];
const SEGMENTS: u32 = 32;
const SEG_HOT: u32 = 25;

/// Everything the screen reads that is not in the machine itself.
pub struct BenchCtx<'a> {
    pub gba: &'a Gba,
    pub code: &'a str,
    pub save: SaveType,
    pub hle: bool,
    /// Host time spent emulating the last frame, and the wall time between the last two presents.
    pub host_ms: f64,
    pub paced_ms: f64,
    /// Device-side audio underruns (None without audio).
    pub audio_underruns: Option<u64>,
    /// The game is advancing (P) rather than halted on this screen.
    pub running: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BenchOutcome {
    None,
    /// A layer or channel tick changed: the app applies `layer_mute` / `channel_mute`.
    MutesChanged,
}

#[derive(Clone, Debug)]
pub struct BenchScreen {
    /// Focus over the 11 ticks: 0..5 layers, 5..11 channels.
    pub focus: usize,
    pub layer_mute: u8,
    pub channel_mute: u8,
    /// Hit boxes recorded by the last render, for mouse clicks: (x, y, w, h, tick index).
    hits: Vec<(f32, f32, f32, f32, usize)>,
}

impl Default for BenchScreen {
    fn default() -> Self {
        Self::new()
    }
}

impl BenchScreen {
    pub fn new() -> Self {
        BenchScreen { focus: 0, layer_mute: 0, channel_mute: 0, hits: Vec::new() }
    }

    fn toggle(&mut self, i: usize) {
        if i < 5 {
            self.layer_mute ^= 1 << i;
        } else if i < 11 {
            self.channel_mute ^= 1 << (i - 5);
        }
    }

    pub fn update(&mut self, input: &UiInput) -> BenchOutcome {
        let mut changed = false;
        if input.action(Action::Up) {
            self.focus = (self.focus + 10) % 11;
        }
        if input.action(Action::Down) {
            self.focus = (self.focus + 1) % 11;
        }
        if input.action(Action::Confirm) {
            self.toggle(self.focus);
            changed = true;
        }
        if input.clicked {
            if let Some(&(_, _, _, _, i)) = self.hits.iter().find(|(x, y, w, h, _)| input.hover(*x, *y, *w, *h)) {
                self.focus = i;
                self.toggle(i);
                changed = true;
            }
        }
        if changed {
            BenchOutcome::MutesChanged
        } else {
            BenchOutcome::None
        }
    }

    pub fn render(&mut self, cv: &mut Canvas, ctx: &BenchCtx) {
        let pal = &NIGHT;
        self.hits.clear();
        let w = cv.width();
        cv.clear(pal.bg);
        let bios = if ctx.hle { "HLE BIOS" } else { "LLE BIOS" };
        let context = format!("Developer view · {} · {} · {bios}", ctx.code, abbrev_save(ctx.save));
        widgets::head(cv, "D", "Bench", Some(&context), None, pal);
        self.head_status(cv, ctx, pal);
        widgets::foot(
            cv,
            &[(&["P"], if ctx.running { "Halt" } else { "Run" }), (&["N"], "Step frame"), (&["Tab"], "Fast-forward"), (&["F1", "F3"], "Save · load"), (&["F10"], "Dump IO"), (&["↑", "↓", "Z"], "Toggle layer / channel")],
            Some(("F12", "Close bench")),
            pal,
        );

        // --- the game at 3x --------------------------------------------------------------
        let gx = l::MARGIN as i32;
        let gy = (l::HEAD_H + 17.0) as i32;
        let gw = SCREEN_W as i32 * GAME_SCALE as i32;
        let gh = SCREEN_H as i32 * GAME_SCALE as i32;
        cv.blit_bgr555(&ctx.gba.framebuffer()[..], SCREEN_W, SCREEN_H, gx, gy, GAME_SCALE as usize);
        let cap = Style::data(f::DATA_SMALL).color(pal.fg2);
        let cy = (gy + gh) as f32 + 8.0;
        cv.text(gx as f32, cy, &format!("{} × {} · {}× · nearest", SCREEN_W, SCREEN_H, GAME_SCALE), &cap);
        cv.text_right((gx + gw) as f32, cy, &format!("host {:.2} ms / frame · paced {:.2} ms", ctx.host_ms, ctx.paced_ms), &cap);

        // --- scanlines -------------------------------------------------------------------
        self.scanlines(cv, ctx, gx as f32, cy + 24.0, gw as f32, pal);

        // --- right column ----------------------------------------------------------------
        let cx = (gx + gw) as f32 + 48.0;
        let cw = w - l::MARGIN - cx;
        let mut y = l::HEAD_H + 17.0;
        y = self.picture(cv, ctx, cx, y, cw, pal);
        y = self.sound(cv, ctx, cx, y + l::SECTION_GAP, cw, pal);
        self.cpu(cv, ctx, cx, y + l::SECTION_GAP, cw, pal);
    }

    fn head_status(&self, cv: &mut Canvas, ctx: &BenchCtx, pal: &Palette) {
        let halted = ctx.gba.bus.irq.halted;
        let word = if !ctx.running {
            "HALTED"
        } else if halted {
            "WAITING"
        } else {
            "RUNNING"
        };
        let text = format!("{word}  FR {}  VCOUNT {}", fmt_frames(ctx.gba.bus.ppu.frame_count), ctx.gba.bus.ppu.vcount);
        let st = Style::data(f::DATA_SMALL + 0.5).color(pal.fg2);
        let tw = cv.measure(&text, &st);
        let x = cv.width() - l::MARGIN - tw;
        let y = (l::HEAD_H - l::HEAD_RULE - st.size * 1.2) / 2.0 + 1.0;
        cv.text(x, y, &text, &st);
        cv.fill_rect(x - 16.0, y + 3.0, 8.0, 8.0, pal.spot);
    }

    fn section(&self, cv: &mut Canvas, x: f32, y: f32, title: &str, context: &str, pal: &Palette) -> f32 {
        let ts = Style::display(18.0).color(pal.fg);
        let tw = cv.text(x, y, title, &ts);
        let cs = Style::label().color(pal.fg2);
        cv.text(x + tw + 10.0, y + 4.0, context, &cs);
        y + 26.0
    }

    fn scanlines(&self, cv: &mut Canvas, ctx: &BenchCtx, x: f32, y: f32, w: f32, pal: &Palette) {
        let y = self.section(cv, x, y, "Scanlines", "ONE FRAME · 228 LINES · 1 232 CYCLES EACH", pal);
        let label_w = 80.0;
        let tx = x + label_w;
        let tw = w - label_w;
        let rows: [(&str, u8); 5] = [("IRQ VBL", EV_VBLANK_IRQ), ("DMA1 FIFO A", EV_DMA1_FIFO), ("DMA2 FIFO B", EV_DMA2_FIFO), ("DMA3", EV_DMA3_START), ("TM0 OVF", EV_TIMER0_OVF)];
        let row_h = 18.0;
        let per_line = tw / 228.0;
        // VBlank shading across all rows
        let vb_x = tx + 160.0 * per_line;
        cv.fill_rect(vb_x, y, tx + tw - vb_x, row_h * rows.len() as f32, pal.bg2);
        let ls = Style::data(f::DATA_SMALL - 1.0).color(pal.fg2);
        let bench = ctx.gba.bench();
        for (i, (name, bit)) in rows.iter().enumerate() {
            let ry = y + i as f32 * row_h;
            cv.text(x, ry + 3.0, name, &ls);
            cv.hrule(tx, tx + tw, ry + row_h - 4.0, 1.0, pal.line);
            if let Some(b) = bench {
                for (line, ev) in b.lines.iter().enumerate() {
                    if ev & bit != 0 {
                        cv.fill_rect(tx + line as f32 * per_line, ry + 3.0, per_line.max(1.0), row_h - 8.0, pal.fg);
                    }
                }
            }
        }
        // cursor at the current VCOUNT
        let vc = ctx.gba.bus.ppu.vcount.min(227) as f32;
        cv.fill_rect(tx + vc * per_line, y - 4.0, 1.5, row_h * rows.len() as f32 + 6.0, pal.spot);
        // axis
        let ay = y + row_h * rows.len() as f32 + 4.0;
        let axis = Style::data(f::DATA_SMALL - 1.0).color(pal.fg2);
        for (line, label) in [(0u32, "0"), (80, "80"), (160, "160"), (227, "227")] {
            let lx = tx + line as f32 * per_line;
            let st = if line == 160 { axis.color(pal.spot) } else { axis };
            if line == 227 {
                cv.text_right(lx + per_line, ay, label, &st);
            } else {
                cv.text(lx, ay, label, &st);
            }
        }
        cv.text(vb_x + 30.0 * per_line, ay, "VBLANK", &axis);
    }

    fn picture(&mut self, cv: &mut Canvas, ctx: &BenchCtx, x: f32, y: f32, w: f32, pal: &Palette) -> f32 {
        let dispcnt = ctx.gba.bus.ppu.dispcnt();
        let y = self.section(cv, x, y, "Picture", &format!("DISPCNT {dispcnt:04X} · MODE {}", dispcnt & 7), pal);
        let bench = ctx.gba.bench();
        let tw = 80.0;
        let th = 53.0;
        let gap = (w - 5.0 * tw) / 4.0;
        let ls = Style::data(f::DATA_SMALL).color(pal.fg);
        for i in 0..5 {
            let tx = x + i as f32 * (tw + gap);
            let hidden = self.layer_mute & (1 << i) != 0;
            let enabled = bench.map(|b| b.layers.enabled & (1 << i) != 0).unwrap_or(false);
            cv.fill_rect(tx, y, tw, th, pal.bg2);
            if let Some(b) = bench {
                if enabled {
                    let plane = &b.layers.planes[i];
                    let mut thumb = [0u16; 80 * 53];
                    for ty in 0..53 {
                        for sx in 0..80 {
                            thumb[ty * 80 + sx] = plane[(ty * 3) * SCREEN_W + sx * 3];
                        }
                    }
                    cv.blit_bgr555(&thumb, 80, 53, tx as i32, y as i32, 1);
                }
            }
            if hidden {
                cv.line(tx, y, tx + tw, y + th, 1.0, pal.spot);
            }
            cv.stroke_rect(tx, y, tw, th, 1.0, if i == self.focus { pal.spot } else { pal.line });
            let ty = y + th + 8.0;
            widgets::tick(cv, tx, ty, !hidden, pal);
            self.hits.push((tx, ty, tw, 14.0, i));
            let name = if i == 4 {
                match bench {
                    Some(b) => format!("OBJ {}/{}", b.layers.obj_drawn, b.layers.obj_active.max(b.layers.obj_drawn)),
                    None => "OBJ".to_string(),
                }
            } else {
                let prio = ctx.gba.bus.ppu.reg16(0x08 + i * 2) & 3;
                format!("{} P{prio}", LAYER_NAMES[i])
            };
            let st = if enabled { ls } else { ls.color(pal.fg2) };
            cv.text(tx + 20.0, ty + 1.0, &name, &st);
        }
        y + th + 8.0 + 20.0
    }

    fn sound(&mut self, cv: &mut Canvas, ctx: &BenchCtx, x: f32, y: f32, w: f32, pal: &Palette) -> f32 {
        let rate = crate::app::src_rate(ctx.gba).round() as u64;
        let bench = ctx.gba.bench();
        let underruns = bench.map(|b| b.sound.underruns[0] + b.sound.underruns[1]).unwrap_or(0);
        let mut ctxt = format!("{} HZ · FIFO UNDERRUNS {underruns}", fmt_thousands(rate));
        if let Some(u) = ctx.audio_underruns {
            ctxt.push_str(&format!(" · DEVICE {u}"));
        }
        let y = self.section(cv, x, y, "Sound", &ctxt, pal);
        let ls = Style::data(f::DATA_SMALL).color(pal.fg);
        let cs = Style::data(f::DATA_SMALL - 1.0).color(pal.fg2);
        let row_h = 22.0;
        let meter_x = x + 90.0;
        let cap_w = 86.0;
        let meter_w = w - 90.0 - cap_w - 8.0;
        let seg_w = (meter_w / SEGMENTS as f32).floor();
        for ch in 0..6 {
            let ry = y + ch as f32 * row_h;
            let muted = self.channel_mute & (1 << ch) != 0;
            widgets::tick(cv, x, ry + 1.0, !muted, pal);
            self.hits.push((x, ry, 80.0, row_h, 5 + ch));
            let st = if ch + 5 == self.focus { ls.color(pal.spot) } else { ls };
            cv.text(x + 20.0, ry + 2.0, CHANNEL_NAMES[ch], &st);
            let level = bench.map(|b| meter_level(ch, b.sound.peak[ch])).unwrap_or(0);
            for s in 0..SEGMENTS {
                let sx = meter_x + s as f32 * seg_w;
                let on = s < level && !muted;
                let c = if !on {
                    pal.line
                } else if s >= SEG_HOT {
                    pal.spot
                } else {
                    pal.fg
                };
                cv.fill_rect(sx, ry + 3.0, seg_w - 2.0, 10.0, c);
            }
            let caption = match (ch, bench) {
                (CH_SQ1, Some(b)) => format!("duty {}", duty_name(b.sound.duty[0])),
                (CH_SQ2, Some(b)) => format!("duty {}", duty_name(b.sound.duty[1])),
                (CH_WAVE, Some(b)) => if b.sound.wave_two_banks { "64 × 4-bit".to_string() } else { "32 × 4-bit".to_string() },
                (CH_NOISE, Some(b)) => if b.sound.noise_width7 { "7-bit LFSR".to_string() } else { "15-bit LFSR".to_string() },
                (CH_FIFO_A, Some(b)) => fifo_caption(b.sound.fifo_dma[0], b.sound.fifo_timer[0]),
                (CH_FIFO_B, Some(b)) => fifo_caption(b.sound.fifo_dma[1], b.sound.fifo_timer[1]),
                _ => String::new(),
            };
            cv.text_right(x + w, ry + 3.0, &caption, &cs);
        }
        y + 6.0 * row_h
    }

    fn cpu(&self, cv: &mut Canvas, ctx: &BenchCtx, x: f32, y: f32, w: f32, pal: &Palette) {
        let regs = &ctx.gba.cpu.regs;
        let thumb = regs.thumb();
        let mode = mode_name(regs.mode());
        let y = self.section(cv, x, y, "CPU", &format!("ARM7TDMI · {mode} · {}", if thumb { "THUMB" } else { "ARM" }), pal);
        let ls = Style::data(f::DATA_SMALL).color(pal.fg2);
        let vs = Style::data(f::DATA_SMALL).color(pal.fg);
        let col_w = w / 4.0;
        let names = ["R0", "R1", "R2", "R3", "R4", "R5", "R6", "R7", "R8", "R9", "R10", "R11", "R12", "SP", "LR", "PC"];
        for i in 0..16 {
            let cx = x + (i % 4) as f32 * col_w;
            let cy = y + (i / 4) as f32 * 18.0;
            cv.text(cx, cy, names[i], &ls);
            let v = regs.r[i];
            let st = if i == 15 {
                vs.color(pal.spot)
            } else if v == 0 {
                vs.color(pal.fg2)
            } else {
                vs
            };
            cv.text(cx + 30.0, cy, &format!("{v:08X}"), &st);
        }
        // CPSR flags
        let fy = y + 4.0 * 18.0 + 8.0;
        cv.text(x, fy + 2.0, "CPSR", &ls);
        let flags = [("N", regs.flag_n()), ("Z", regs.flag_z()), ("C", regs.flag_c()), ("V", regs.flag_v()), ("I", regs.irq_disabled()), ("F", regs.cpsr & 0x40 != 0), ("T", thumb)];
        let mut fx = x + 46.0;
        for (name, on) in flags {
            if on {
                cv.fill_rect(fx, fy, 16.0, 16.0, pal.fg);
                cv.text_center(fx + 8.0, fy + 2.0, name, &vs.color(pal.bg));
            } else {
                cv.stroke_rect(fx, fy, 16.0, 16.0, 1.0, pal.line);
                cv.text_center(fx + 8.0, fy + 2.0, name, &ls);
            }
            fx += 20.0;
        }
        cv.text(fx + 8.0, fy + 2.0, &format!("{:08X} {mode}", regs.cpsr), &vs);
        // disassembly around PC
        let pc = ctx.gba.cpu.pc();
        let size: u32 = if thumb { 2 } else { 4 };
        let dy0 = fy + 28.0;
        for (k, off) in (-2i32..=1).enumerate() {
            let addr = pc.wrapping_add((off * size as i32) as u32);
            let (op_text, text) = if thumb {
                let op = ctx.gba.bus.peek16(addr);
                (format!("{op:04X}"), disasm::thumb(op, addr))
            } else {
                let op = ctx.gba.bus.peek32(addr);
                (format!("{op:08X}"), disasm::arm(op, addr))
            };
            let ly = dy0 + k as f32 * 18.0;
            let current = off == 0;
            let st = if current { vs.color(pal.spot) } else { vs };
            if current {
                cv.fill_rect(x - 8.0, ly - 1.0, 2.0, 16.0, pal.spot);
            }
            cv.text(x, ly, &format!("{addr:08X}"), &if current { st } else { ls });
            cv.text(x + 84.0, ly, &op_text, &if current { st } else { ls });
            cv.text(x + 160.0, ly, &text, &st);
        }
    }
}

fn fmt_thousands(v: u64) -> String {
    let s = v.to_string();
    let mut out = String::new();
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (s.len() - i) % 3 == 0 {
            out.push(' ');
        }
        out.push(c);
    }
    out
}

/// Segments lit for a peak: PSG peaks are 0..=15, FIFO peaks 0..=128.
pub fn meter_level(ch: usize, peak: u8) -> u32 {
    let (max, p) = if ch >= CH_FIFO_A { (128.0, peak.min(128) as f32) } else { (15.0, peak.min(15) as f32) };
    (p / max * SEGMENTS as f32).round() as u32
}

fn duty_name(d: u8) -> &'static str {
    match d & 3 {
        0 => "12.5%",
        1 => "25%",
        2 => "50%",
        _ => "75%",
    }
}

fn fifo_caption(dma: Option<u8>, timer: u8) -> String {
    match dma {
        Some(d) => format!("DMA{d} · TM{timer}"),
        None => format!("no DMA · TM{timer}"),
    }
}

pub fn mode_name(mode: u32) -> &'static str {
    match mode {
        MODE_USR => "USR",
        MODE_FIQ => "FIQ",
        MODE_IRQ => "IRQ",
        MODE_SVC => "SVC",
        MODE_ABT => "ABT",
        MODE_UND => "UND",
        MODE_SYS => "SYS",
        _ => "???",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::Fonts;

    fn machine() -> Gba {
        let mut rom = vec![0u8; 0x1000];
        rom[0..4].copy_from_slice(&0xEAFF_FFFEu32.to_le_bytes());
        rom[0xA0..0xAC].copy_from_slice(b"BENCHTEST\0\0\0");
        rom[0xAC..0xB0].copy_from_slice(b"TSTE");
        rom[0xB2] = 0x96;
        Gba::new(rom, gba_core::Options::default()).unwrap()
    }

    #[test]
    fn meter_scaling() {
        assert_eq!(meter_level(CH_SQ1, 0), 0);
        assert_eq!(meter_level(CH_SQ1, 15), 32);
        assert_eq!(meter_level(CH_FIFO_A, 128), 32);
        assert_eq!(meter_level(CH_FIFO_B, 64), 16);
        assert_eq!(fmt_thousands(32768), "32 768");
        assert_eq!(fmt_thousands(999), "999");
        assert_eq!(mode_name(MODE_SYS), "SYS");
        assert_eq!(duty_name(2), "50%");
    }

    #[test]
    fn ticks_toggle_mutes_by_focus_and_click() {
        let mut b = BenchScreen::new();
        let up = UiInput { actions: vec![Action::Up], ..Default::default() };
        assert_eq!(b.update(&up), BenchOutcome::None);
        assert_eq!(b.focus, 10, "wraps to FIFO B");
        let confirm = UiInput { actions: vec![Action::Confirm], ..Default::default() };
        assert_eq!(b.update(&confirm), BenchOutcome::MutesChanged);
        assert_eq!(b.channel_mute, 1 << 5);
        b.focus = 2;
        b.update(&confirm);
        assert_eq!(b.layer_mute, 1 << 2);
        b.hits.push((10.0, 10.0, 50.0, 10.0, 4));
        let click = UiInput { clicked: true, mouse: (20.0, 15.0), ..Default::default() };
        assert_eq!(b.update(&click), BenchOutcome::MutesChanged);
        assert_eq!(b.layer_mute, (1 << 2) | (1 << 4));
        assert_eq!(b.focus, 4);
    }

    #[test]
    fn renders_with_and_without_the_capture() {
        let mut gba = machine();
        gba.run_frame();
        let mut cv = Canvas::new(1280, 800, Fonts::new());
        let mut b = BenchScreen::new();
        let ctx = BenchCtx { gba: &gba, code: "TSTE", save: SaveType::None, hle: true, host_ms: 0.5, paced_ms: 16.7, audio_underruns: None, running: false };
        b.render(&mut cv, &ctx);
        assert_eq!(b.hits.len(), 11);
        assert_eq!(cv.pixel(640, 55), NIGHT.fg, "head rule");
        drop(ctx);
        gba.set_bench(true);
        for _ in 0..3 {
            gba.run_frame();
        }
        assert!(gba.bench().is_some());
        let ctx = BenchCtx { gba: &gba, code: "TSTE", save: SaveType::None, hle: true, host_ms: 0.5, paced_ms: 16.7, audio_underruns: None, running: true };
        b.render(&mut cv, &ctx);
        let spot = (0..1280u32).flat_map(|x| (0..800u32).map(move |y| (x, y))).filter(|&(x, y)| cv.pixel(x, y) == NIGHT.spot).count();
        assert!(spot > 10, "status square, PC, cursor: {spot}");
    }

    #[test]
    fn pinned_bench_renders() {
        let mut gba = machine();
        gba.run_frame();
        let mut cv = Canvas::new(1280, 800, Fonts::new());
        let mut b = BenchScreen::new();
        let ctx = BenchCtx { gba: &gba, code: "TSTE", save: SaveType::None, hle: true, host_ms: 0.5, paced_ms: 16.7, audio_underruns: None, running: false };
        b.render(&mut cv, &ctx);
        crate::pinned::check_canvas("bench-idle", &cv);
        drop(ctx);
        gba.set_bench(true);
        for _ in 0..3 {
            gba.run_frame();
        }
        b.layer_mute = 0;
        let ctx = BenchCtx { gba: &gba, code: "TSTE", save: SaveType::Sram32K, hle: true, host_ms: 0.53, paced_ms: 16.74, audio_underruns: Some(0), running: true };
        b.render(&mut cv, &ctx);
        crate::pinned::check_canvas("bench-running", &cv);
    }

    #[test]
    #[ignore]
    fn render_bench() {
        let rom = std::env::var("GBA_ROM").expect("set GBA_ROM");
        let mut gba = gba_test::build_gba(std::path::Path::new(&rom), Some(gba_core::BiosMode::Hle), None).unwrap();
        for _ in 0..1500 {
            gba.run_frame();
        }
        gba.set_bench(true);
        for _ in 0..5 {
            gba.run_frame();
        }
        let mut cv = Canvas::new(1280, 800, Fonts::new());
        let mut b = BenchScreen::new();
        b.layer_mute = 0;
        let code = gba.bus.cart.header.game_code_str();
        let ctx = BenchCtx { gba: &gba, code: &code, save: gba.bus.cart.save_type, hle: true, host_ms: 0.53, paced_ms: 16.74, audio_underruns: Some(0), running: false };
        b.render(&mut cv, &ctx);
        let path = crate::render_tests::out_dir().join("D-bench.png");
        cv.save_png(&path).unwrap();
        println!("wrote {}", path.display());
    }
}
