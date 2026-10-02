//! The in-game overlay: everything drawn over a running game is drawn at native 240x160 into a
//! copy of the frame, in the Pixel palette, and scaled with it (`DESIGN.md`, rule 6).
//!
//! `font5x5` is the bitmap font, `pause` the pause menu state machine; this module holds the
//! drawing primitives (dim, window, text, cursor, thumbnails, toast).

pub mod font5x5;
pub mod pause;

use crate::ui::tokens::pixel::*;
use gba_core::{FrameBuffer, SCREEN_H, SCREEN_W};

/// A 48x32 slot thumbnail: an exact 1/5 sample of a frame, BGR555.
pub type Thumb = [u16; THUMB_W * THUMB_H];

pub fn thumb_from_frame(fb: &[u16]) -> Thumb {
    let mut t = [0u16; THUMB_W * THUMB_H];
    for y in 0..THUMB_H {
        for x in 0..THUMB_W {
            t[y * THUMB_W + x] = fb[(y * 5) * SCREEN_W + x * 5] & 0x7FFF;
        }
    }
    t
}

/// Drawing target: a frame copy. Never the core's own framebuffer.
pub struct Overlay<'a> {
    pub fb: &'a mut FrameBuffer,
}

impl<'a> Overlay<'a> {
    pub fn new(fb: &'a mut FrameBuffer) -> Self {
        Overlay { fb }
    }

    #[inline]
    pub fn px(&mut self, x: i32, y: i32, c: u16) {
        if x >= 0 && y >= 0 && (x as usize) < SCREEN_W && (y as usize) < SCREEN_H {
            self.fb[y as usize * SCREEN_W + x as usize] = c;
        }
    }

    /// BLDY-style brightness decrease on the whole frame: `I - I*EVY/16` per channel.
    pub fn dim(&mut self, evy: u16) {
        for p in self.fb.iter_mut() {
            *p = dim_px(*p, evy);
        }
    }

    pub fn fill(&mut self, x: i32, y: i32, w: i32, h: i32, c: u16) {
        for yy in y..y + h {
            for xx in x..x + w {
                self.px(xx, yy, c);
            }
        }
    }

    pub fn rect(&mut self, x: i32, y: i32, w: i32, h: i32, c: u16) {
        self.fill(x, y, w, 1, c);
        self.fill(x, y + h - 1, w, 1, c);
        self.fill(x, y, 1, h, c);
        self.fill(x + w - 1, y, 1, h, c);
    }

    /// Pixel window: 1 px ink, 1 px cream, 1 px shade border, cream fill. Returns the inner
    /// content origin (after the 3 px border and 4 px padding).
    pub fn window(&mut self, x: i32, y: i32, w: i32, h: i32) -> (i32, i32) {
        self.rect(x, y, w, h, INK_555);
        self.rect(x + 1, y + 1, w - 2, h - 2, CREAM_555);
        self.rect(x + 2, y + 2, w - 4, h - 4, SHADE_555);
        self.fill(x + 3, y + 3, w - 6, h - 6, CREAM_555);
        (x + 3 + WINDOW_PADDING, y + 3 + WINDOW_PADDING)
    }

    /// One line of 5x5 text; returns the width drawn.
    pub fn text(&mut self, x: i32, y: i32, text: &str, c: u16) -> i32 {
        self.text_scaled(x, y, text, c, 1)
    }

    /// One line of 5x5 text with every glyph pixel a `scale` x `scale` block (whole numbers
    /// only, so the text stays on the 8 px grid); returns the width drawn.
    pub fn text_scaled(&mut self, x: i32, y: i32, text: &str, c: u16, scale: i32) -> i32 {
        let mut cx = x;
        for ch in text.chars() {
            let g = font5x5::glyph(ch);
            for (row, bits) in g.iter().enumerate() {
                for col in 0..font5x5::GLYPH_W {
                    if bits & (0b10000 >> col) != 0 {
                        self.fill(cx + col as i32 * scale, y + row as i32 * scale, scale, scale, c);
                    }
                }
            }
            cx += font5x5::ADVANCE * scale;
        }
        cx - x - if text.is_empty() { 0 } else { scale }
    }

    pub fn text_center_scaled(&mut self, cx: i32, y: i32, text: &str, c: u16, scale: i32) {
        let w = font5x5::width(text) * scale - (scale - 1);
        self.text_scaled(cx - w / 2, y, text, c, scale);
    }

    /// Nearest-neighbour blit of a `w` x `h` BGR555 texture at an integer scale; bit 15 set =
    /// transparent.
    pub fn sprite(&mut self, x: i32, y: i32, tex: &[u16], w: usize, h: usize, scale: i32) {
        for ty in 0..h {
            for tx in 0..w {
                let t = tex[ty * w + tx];
                if t & 0x8000 == 0 {
                    self.fill(x + tx as i32 * scale, y + ty as i32 * scale, scale, scale, t);
                }
            }
        }
    }

    pub fn text_right(&mut self, x_right: i32, y: i32, text: &str, c: u16) {
        let w = font5x5::width(text);
        self.text(x_right - w, y, text, c);
    }

    pub fn text_center(&mut self, cx: i32, y: i32, text: &str, c: u16) {
        let w = font5x5::width(text);
        self.text(cx - w / 2, y, text, c);
    }

    /// 4x8 right-pointing triangle cursor with its left edge at x and vertical centre at cy.
    pub fn cursor(&mut self, x: i32, cy: i32, c: u16) {
        for i in 0..4 {
            let half = 4 - i;
            for dy in -half + 1..half {
                self.px(x + i, cy + dy, c);
            }
        }
    }

    /// 48x32 thumbnail with a 1 px outline (ink, or red when selected).
    pub fn thumb(&mut self, x: i32, y: i32, t: &Thumb, selected: bool) {
        for yy in 0..THUMB_H {
            for xx in 0..THUMB_W {
                self.px(x + xx as i32, y + yy as i32, t[yy * THUMB_W + xx]);
            }
        }
        self.rect(x - 1, y - 1, THUMB_W as i32 + 2, THUMB_H as i32 + 2, if selected { RED_555 } else { INK_555 });
    }

    /// Empty slot thumbnail: `empty` fill with "EMPTY" in dim.
    pub fn thumb_empty(&mut self, x: i32, y: i32, selected: bool) {
        self.fill(x, y, THUMB_W as i32, THUMB_H as i32, EMPTY_555);
        self.text_center(x + THUMB_W as i32 / 2, y + THUMB_H as i32 / 2 - 2, "EMPTY", DIM_555);
        self.rect(x - 1, y - 1, THUMB_W as i32 + 2, THUMB_H as i32 + 2, if selected { RED_555 } else { INK_555 });
    }

    /// Single-line toast window at the bottom of the frame.
    pub fn toast(&mut self, line: &str) {
        let line = font5x5::truncate(line);
        let w = font5x5::width(&line) + 2 * (3 + WINDOW_PADDING);
        let h = font5x5::GLYPH_H as i32 + 2 * (3 + WINDOW_PADDING);
        let x = (SCREEN_W as i32 - w) / 2;
        let y = SCREEN_H as i32 - h - 4;
        let (cx, cy) = self.window(x, y, w, h);
        self.text(cx, cy, &line, INK_555);
    }
}

/// `I - I*EVY/16` on each 5-bit channel.
pub fn dim_px(p: u16, evy: u16) -> u16 {
    let ch = |v: u16| v - v * evy / 16;
    let r = ch(p & 0x1F);
    let g = ch((p >> 5) & 0x1F);
    let b = ch((p >> 10) & 0x1F);
    r | g << 5 | b << 10
}

#[cfg(test)]
mod tests {
    use super::*;

    fn blank() -> Box<FrameBuffer> {
        vec![0u16; SCREEN_W * SCREEN_H].into_boxed_slice().try_into().unwrap()
    }

    #[test]
    fn dim_matches_bldy_maths() {
        assert_eq!(dim_px(0x7FFF, 9), {
            let v = 31 - 31 * 9 / 16; // 14
            v | v << 5 | v << 10
        });
        assert_eq!(dim_px(0, 9), 0);
        assert_eq!(dim_px(0x001F, 16), 0);
        assert_eq!(dim_px(0x03E0, 0), 0x03E0);
    }

    #[test]
    fn window_border_and_padding() {
        let mut fb = blank();
        fb.fill(0x7FFF);
        let mut o = Overlay::new(&mut fb);
        let (cx, cy) = o.window(8, 8, 84, 112);
        assert_eq!((cx, cy), (8 + 3 + 4, 8 + 3 + 4));
        let at = |x: usize, y: usize| fb[y * SCREEN_W + x];
        assert_eq!(at(8, 8), INK_555);
        assert_eq!(at(9, 9), CREAM_555);
        assert_eq!(at(10, 10), SHADE_555);
        assert_eq!(at(11, 11), CREAM_555);
        assert_eq!(at(8 + 83, 8 + 111), INK_555);
        assert_eq!(at(7, 7), 0x7FFF);
        assert_eq!(at(8 + 84, 8), 0x7FFF);
    }

    #[test]
    fn scaled_text_and_sprite_blit() {
        let mut fb = blank();
        let mut o = Overlay::new(&mut fb);
        let w = o.text_scaled(0, 0, "AB", RED_555, 2);
        assert_eq!(w, 2 * font5x5::ADVANCE * 2 - 2);
        let tex = crate::ui::brand::mark().night_555();
        o.sprite(100, 100, &tex, 16, 16, 3);
        let at = |x: usize, y: usize| fb[y * SCREEN_W + x];
        // row 0 of 'A' is .###. so column 1 is lit, and at 2x it is a 2x2 block
        assert_eq!(at(2, 0), RED_555);
        assert_eq!(at(3, 1), RED_555);
        assert_eq!(at(0, 0), 0);
        assert_eq!(at(100 + 7 * 3, 100), CREAM_555, "stem at 3x");
        assert_eq!(at(100 + 7 * 3 + 2, 100 + 2), CREAM_555);
        assert_eq!(at(100, 100), 0, "transparent corner untouched");
        assert_eq!(at(100 + 3 * 3, 100 + 7 * 3), RED_555, "first seed");
    }

    #[test]
    fn text_cursor_and_toast_draw_in_palette() {
        let mut fb = blank();
        let mut o = Overlay::new(&mut fb);
        let w = o.text(10, 10, "RESUME", RED_555);
        assert_eq!(w, font5x5::width("RESUME"));
        o.cursor(2, 12, RED_555);
        o.toast("SAVED TO SLOT 2");
        let red = fb.iter().filter(|p| **p == RED_555).count();
        assert!(red > 20);
        let cream = fb.iter().filter(|p| **p == CREAM_555).count();
        assert!(cream > 100, "toast fill");
        // cursor: 4 columns, 7,5,3,1 pixels tall
        let col = |x: usize| (0..SCREEN_H).filter(|y| fb[y * SCREEN_W + x] == RED_555).count();
        assert_eq!((col(2), col(3), col(4), col(5)), (7, 5, 3, 1));
    }

    #[test]
    fn thumbnails_are_exact_fifth_samples() {
        let mut src = blank();
        for y in 0..SCREEN_H {
            for x in 0..SCREEN_W {
                src[y * SCREEN_W + x] = ((x / 5) as u16) | (((y / 5) as u16) << 6);
            }
        }
        let t = thumb_from_frame(&src[..]);
        for y in 0..THUMB_H {
            for x in 0..THUMB_W {
                assert_eq!(t[y * THUMB_W + x], (x as u16) | ((y as u16) << 6));
            }
        }
        let mut fb = blank();
        {
            let mut o = Overlay::new(&mut fb);
            o.thumb(20, 20, &t, true);
            o.thumb_empty(100, 20, false);
        }
        assert_eq!(fb[19 * SCREEN_W + 19], RED_555);
        assert_eq!(fb[20 * SCREEN_W + 20], t[0]);
        assert_eq!(fb[20 * SCREEN_W + 100], EMPTY_555);
        assert_eq!(fb[19 * SCREEN_W + 99], INK_555);
    }
}
