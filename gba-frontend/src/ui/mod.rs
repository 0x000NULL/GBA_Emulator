//! Window-resolution rendering layer for the GUI ("paper, ink, one red", `docs/gui/`).
//!
//! `tokens` holds the design constants, `brand` the Strawberry mark, `canvas` draws (tiny-skia + direct pixel writes), `text`
//! rasterises the bundled faces, `widgets` are the shared components, `drawings` parses the SVG
//! line art, `fmt` applies the copy rules, `input` is the per-frame input snapshot and the
//! GBA-button-driven navigation actions.

pub mod brand;
pub mod canvas;
pub mod drawings;
pub mod fmt;
pub mod input;
pub mod text;
pub mod tokens;
pub mod widgets;

pub use canvas::Canvas;
pub use text::Fonts;
pub use tokens::{Rgb, NIGHT};

/// Largest integer scale at which 240x160 fits in `w` x `h` (at least 1).
pub fn fit_scale(w: u32, h: u32) -> u32 {
    (w / gba_core::SCREEN_W as u32).min(h / gba_core::SCREEN_H as u32).max(1)
}

/// Top-left of a `scale`x frame centred in `w` x `h`.
pub fn centred_frame(w: u32, h: u32, scale: u32) -> (i32, i32) {
    let fw = gba_core::SCREEN_W as i32 * scale as i32;
    let fh = gba_core::SCREEN_H as i32 * scale as i32;
    ((w as i32 - fw) / 2, (h as i32 - fh) / 2)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fit_and_centre() {
        assert_eq!(fit_scale(1280, 800), 5);
        assert_eq!(fit_scale(1280, 700), 4);
        assert_eq!(fit_scale(960, 640), 4);
        assert_eq!(fit_scale(720, 480), 3);
        assert_eq!(fit_scale(100, 100), 1);
        assert_eq!(centred_frame(1280, 800, 4), (160, 80));
        assert_eq!(centred_frame(720, 480, 3), (0, 0));
    }
}
