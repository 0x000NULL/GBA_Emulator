//! Visual review renders: `cargo test -p gba-frontend --release -- --ignored render_` writes
//! PNGs under `target/ui-renders/` to compare with `docs/gui/mockups/` by eye.
//!
//! Screens that need a game use the ROM in `GBA_ROM` when set (any ROM works); otherwise a
//! synthetic test pattern stands in for the frame.

#![cfg(test)]

use crate::overlay::pause::{Mode, PauseMenu, SlotInfo};
use crate::overlay::{thumb_from_frame, Overlay};
use crate::ui::tokens::pixel::DIM_EVY;
use gba_core::{FrameBuffer, SCREEN_H, SCREEN_W};
use std::path::PathBuf;
use std::time::Instant;

pub fn out_dir() -> PathBuf {
    let d = gba_test::repo_root().join("target").join("ui-renders");
    std::fs::create_dir_all(&d).unwrap();
    d
}

/// A frame to draw over: the ROM from `GBA_ROM` run for `frames`, or a test pattern.
pub fn sample_frame(frames: u32) -> Box<FrameBuffer> {
    let mut fb: Box<FrameBuffer> = vec![0u16; SCREEN_W * SCREEN_H].into_boxed_slice().try_into().unwrap();
    if let Ok(p) = std::env::var("GBA_ROM") {
        if let Ok(mut gba) = gba_test::build_gba(std::path::Path::new(&p), Some(gba_core::BiosMode::Hle), None) {
            for _ in 0..frames {
                gba.run_frame();
            }
            fb.copy_from_slice(&gba.framebuffer()[..]);
            return fb;
        }
    }
    for y in 0..SCREEN_H {
        for x in 0..SCREEN_W {
            let checker = ((x / 16) + (y / 16)) % 2 == 0;
            fb[y * SCREEN_W + x] = if checker { 0x5AD6 } else { 0x4A52 };
        }
    }
    fb
}

/// Write a 240x160 BGR555 frame as a PNG at an integer scale.
pub fn write_frame_png(name: &str, fb: &[u16], scale: usize) {
    let mut big = vec![0u16; SCREEN_W * scale * SCREEN_H * scale];
    for y in 0..SCREEN_H * scale {
        for x in 0..SCREEN_W * scale {
            big[y * SCREEN_W * scale + x] = fb[(y / scale) * SCREEN_W + x / scale];
        }
    }
    let path = out_dir().join(name);
    let mut rgb = Vec::with_capacity(big.len() * 3);
    for p in &big {
        rgb.extend_from_slice(&gba_test::bgr555_to_rgb(*p));
    }
    let file = std::fs::File::create(&path).unwrap();
    let mut enc = png::Encoder::new(std::io::BufWriter::new(file), (SCREEN_W * scale) as u32, (SCREEN_H * scale) as u32);
    enc.set_color(png::ColorType::Rgb);
    enc.set_depth(png::BitDepth::Eight);
    enc.write_header().unwrap().write_image_data(&rgb).unwrap();
    println!("wrote {}", path.display());
}

#[test]
#[ignore]
fn render_paused_overlay() {
    let base = sample_frame(1500);
    let mut slots: [Option<SlotInfo>; 8] = Default::default();
    for (i, frames) in [(0usize, 300u32), (1, 900), (2, 1500)] {
        let f = sample_frame(frames);
        slots[i] = Some(SlotInfo { thumb: Box::new(thumb_from_frame(&f[..])), play_secs: frames as u64 / 60, frame: frames as u64, broken: None });
    }
    for (name, mode, sel) in [("02-paused-main.png", Mode::Main, 1usize), ("02-paused-save.png", Mode::Save, 1), ("02-paused-load.png", Mode::Load, 2)] {
        let mut fb = base.clone();
        let mut o = Overlay::new(&mut fb);
        o.dim(DIM_EVY);
        let menu = PauseMenu { sel, mode, page: 0, slot_sel: 1, message: None };
        menu.draw(&mut o, &slots, Instant::now());
        write_frame_png(name, &fb[..], 4);
    }
    let mut fb = base.clone();
    let mut o = Overlay::new(&mut fb);
    o.toast("SAVED TO SLOT 2");
    write_frame_png("02-toast.png", &fb[..], 4);
}
