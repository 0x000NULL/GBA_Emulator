//! Window-resolution rendering layer for the GUI ("paper, ink, one red", `docs/gui/`).
//!
//! `tokens` holds the design constants, `mark` parses pixel marks, `canvas` draws (tiny-skia + direct pixel writes), `text`
//! rasterises the bundled faces, `widgets` are the shared components, `drawings` parses the SVG
//! line art, `fmt` applies the copy rules, `input` is the per-frame input snapshot and the
//! GBA-button-driven navigation actions.

pub mod canvas;
pub mod drawings;
pub mod fmt;
pub mod input;
pub mod mark;
pub mod text;
pub mod tokens;
pub mod widgets;

pub use canvas::Canvas;
pub use text::Fonts;
pub use tokens::{Rgb, NIGHT};

/// The emulated picture's size in dots, set by the app (240x160 here).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Screen {
    pub w: u32,
    pub h: u32,
}

impl Screen {
    /// Largest integer scale at which the picture fits in `w` x `h` (at least 1).
    pub fn fit_scale(self, w: u32, h: u32) -> u32 {
        (w / self.w).min(h / self.h).max(1)
    }

    /// Top-left of a `scale`x picture centred in `w` x `h`.
    pub fn centred(self, w: u32, h: u32, scale: u32) -> (i32, i32) {
        let fw = self.w as i32 * scale as i32;
        let fh = self.h as i32 * scale as i32;
        ((w as i32 - fw) / 2, (h as i32 - fh) / 2)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fit_and_centre() {
        let gba = Screen { w: 240, h: 160 };
        assert_eq!(gba.fit_scale(1280, 800), 5);
        assert_eq!(gba.fit_scale(1280, 700), 4);
        assert_eq!(gba.fit_scale(960, 640), 4);
        assert_eq!(gba.fit_scale(720, 480), 3);
        assert_eq!(gba.fit_scale(100, 100), 1);
        assert_eq!(gba.centred(1280, 800, 4), (160, 80));
        assert_eq!(gba.centred(720, 480, 3), (0, 0));
        let nes = Screen { w: 256, h: 240 };
        assert_eq!(nes.fit_scale(1280, 800), 3);
        assert_eq!(nes.centred(1280, 800, 3), (256, 40));
    }
}
