//! minifb window: BGR555 -> 0RGB conversion through a LUT, integer upscaling, title bar.

use crate::config::{SCREEN_H, SCREEN_W};
use anyhow::{anyhow, Result};
use minifb::{Scale, ScaleMode, Window, WindowOptions};

/// 32768-entry BGR555 -> 0RGB8888 table (bit 15 of the source is ignored).
pub fn build_lut() -> Vec<u32> {
    (0..0x8000u32)
        .map(|p| {
            let [r, g, b] = gba_test::bgr555_to_rgb(p as u16);
            (r as u32) << 16 | (g as u32) << 8 | b as u32
        })
        .collect()
}

/// Nearest-neighbour upscale of a 240x160 BGR555 framebuffer into `out` (240*scale x 160*scale, 0RGB).
pub fn scale_into(fb: &[u16], lut: &[u32], scale: usize, out: &mut [u32]) {
    debug_assert_eq!(fb.len(), SCREEN_W * SCREEN_H);
    let dst_w = SCREEN_W * scale;
    debug_assert_eq!(out.len(), dst_w * SCREEN_H * scale);
    for y in 0..SCREEN_H {
        let src = &fb[y * SCREEN_W..(y + 1) * SCREEN_W];
        let row0 = y * scale * dst_w;
        {
            let row = &mut out[row0..row0 + dst_w];
            for (x, &p) in src.iter().enumerate() {
                let c = lut[(p & 0x7FFF) as usize];
                row[x * scale..(x + 1) * scale].fill(c);
            }
        }
        for r in 1..scale {
            out.copy_within(row0..row0 + dst_w, row0 + r * dst_w);
        }
    }
}

pub struct Video {
    window: Window,
    scale: usize,
    buf: Vec<u32>,
    lut: Vec<u32>,
}

impl Video {
    pub fn new(title: &str, scale: u32) -> Result<Self> {
        let scale = scale.max(1) as usize;
        let (w, h) = (SCREEN_W * scale, SCREEN_H * scale);
        let opts = WindowOptions {
            resize: false,
            scale: Scale::X1,
            scale_mode: ScaleMode::Stretch,
            ..WindowOptions::default()
        };
        let mut window = Window::new(title, w, h, opts).map_err(|e| anyhow!("creating window: {e}"))?;
        // minifb rate-limits update() to 250 fps by default; pacing is ours.
        window.set_target_fps(0);
        Ok(Video { window, scale, buf: vec![0; w * h], lut: build_lut() })
    }

    /// Convert, upscale and blit one frame. Also pumps window events.
    pub fn present(&mut self, fb: &[u16]) -> Result<()> {
        scale_into(fb, &self.lut, self.scale, &mut self.buf);
        let (w, h) = (SCREEN_W * self.scale, SCREEN_H * self.scale);
        self.window.update_with_buffer(&self.buf, w, h).map_err(|e| anyhow!("presenting frame: {e}"))
    }

    /// Pump window events without redrawing.
    pub fn poll(&mut self) {
        self.window.update();
    }

    pub fn is_open(&self) -> bool {
        self.window.is_open()
    }

    pub fn set_title(&mut self, title: &str) {
        self.window.set_title(title);
    }

    pub fn window(&self) -> &Window {
        &self.window
    }

    pub fn size(&self) -> (usize, usize) {
        self.window.get_size()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lut_endpoints() {
        let lut = build_lut();
        assert_eq!(lut.len(), 0x8000);
        assert_eq!(lut[0], 0x000000);
        assert_eq!(lut[0x7FFF], 0xFFFFFF);
        assert_eq!(lut[0x001F], 0xFF0000, "pure red is the low 5 bits");
        assert_eq!(lut[0x03E0], 0x00FF00);
        assert_eq!(lut[0x7C00], 0x0000FF);
    }

    #[test]
    fn scale_replicates_pixels() {
        let lut = build_lut();
        let mut fb = vec![0u16; SCREEN_W * SCREEN_H];
        fb[0] = 0x001F; // red at (0,0)
        fb[SCREEN_W + 1] = 0x7C00; // blue at (1,1)
        let scale = 3;
        let w = SCREEN_W * scale;
        let mut out = vec![0u32; w * SCREEN_H * scale];
        scale_into(&fb, &lut, scale, &mut out);
        for y in 0..3 {
            for x in 0..3 {
                assert_eq!(out[y * w + x], 0xFF0000);
            }
        }
        for y in 3..6 {
            for x in 3..6 {
                assert_eq!(out[y * w + x], 0x0000FF);
            }
        }
        assert_eq!(out[3], 0);
        assert_eq!(out[3 * w], 0);
    }
}
