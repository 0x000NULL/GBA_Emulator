//! The Strawberry mark and name (`docs/branding/`). The 16x16 sprite is parsed from
//! `logo/strawberry-mark.svg` with the same parser as the line drawings, so the SVG stays the
//! source of truth; the icons are embedded for the window and the exe.
//!
//! Whole-number scales only, never smoothed (board 02). On paper the sprite uses all four
//! colours; on an ink screen the night variant draws the outline in cream and the seeds in red
//! and leaves the body transparent (`strawberry-mark-night.svg`).

#![allow(dead_code)]

use super::drawings::{self, Geometry, Seg};
use super::tokens::{pixel, Rgb, NIGHT};
use std::sync::OnceLock;

pub const NAME: &str = "Strawberry";
pub const WORDMARK: &str = "STRAWBERRY";
pub const SERIES: &str = "Fruit Basket · No. 2";

pub const MARK_SVG: &str = include_str!("../../../docs/branding/logo/strawberry-mark.svg");
pub const MARK_NIGHT_SVG: &str = include_str!("../../../docs/branding/logo/strawberry-mark-night.svg");
/// `strawberry.ico`: 16, 32, 48, 64, 128 and 256 px entries.
pub const ICO: &[u8] = include_bytes!("../../../docs/branding/icons/strawberry.ico");
pub const ICON_48_PNG: &[u8] = include_bytes!("../../../docs/branding/icons/png/strawberry-icon-48.png");
pub const ICON_128_PNG: &[u8] = include_bytes!("../../../docs/branding/icons/png/strawberry-icon-128.png");

/// Side of the sprite in pixels.
pub const SPRITE: usize = 16;
/// Transparent texel for `Canvas::blit_bgr555` / `Overlay::sprite` (bit 15 set).
pub const CLEAR_555: u16 = 0x8000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Px {
    None,
    Ink,
    Cream,
    Red,
    Shade,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Sprite {
    pub px: [Px; SPRITE * SPRITE],
}

/// Class a path fill: the Pixel palette on paper, the Night chrome colours in the night file.
fn class(fill: Rgb) -> Option<Px> {
    match fill {
        c if c == pixel::INK || c == NIGHT.fg => Some(Px::Ink),
        c if c == pixel::CREAM => Some(Px::Cream),
        c if c == pixel::RED || c == NIGHT.spot => Some(Px::Red),
        c if c == pixel::SHADE => Some(Px::Shade),
        _ => None,
    }
}

/// Parse a mark SVG: every closed subpath is an axis-aligned run of cells filled from its
/// bounding box with the path's colour class.
pub fn parse_mark(svg: &str) -> Sprite {
    let mut px = [Px::None; SPRITE * SPRITE];
    for shape in drawings::parse(svg).shapes {
        let Some(p) = shape.fill.and_then(class) else { continue };
        let Geometry::Path(segs) = &shape.geom else { continue };
        let mut bounds: Option<(f32, f32, f32, f32)> = None;
        let mut flush = |b: &mut Option<(f32, f32, f32, f32)>| {
            if let Some((x0, y0, x1, y1)) = b.take() {
                for y in (y0.round() as i32)..(y1.round() as i32) {
                    for x in (x0.round() as i32)..(x1.round() as i32) {
                        if (0..SPRITE as i32).contains(&x) && (0..SPRITE as i32).contains(&y) {
                            px[y as usize * SPRITE + x as usize] = p;
                        }
                    }
                }
            }
        };
        for seg in segs {
            match *seg {
                Seg::Move(x, y) => {
                    flush(&mut bounds);
                    bounds = Some((x, y, x, y));
                }
                Seg::Line(x, y) => {
                    if let Some(b) = bounds.as_mut() {
                        b.0 = b.0.min(x);
                        b.1 = b.1.min(y);
                        b.2 = b.2.max(x);
                        b.3 = b.3.max(y);
                    }
                }
                Seg::Close => flush(&mut bounds),
                Seg::Cubic(..) => {}
            }
        }
        flush(&mut bounds);
    }
    Sprite { px }
}

/// The paper mark, parsed once.
pub fn mark() -> &'static Sprite {
    static MARK: OnceLock<Sprite> = OnceLock::new();
    MARK.get_or_init(|| parse_mark(MARK_SVG))
}

impl Sprite {
    pub fn at(&self, x: usize, y: usize) -> Px {
        self.px[y * SPRITE + x]
    }

    pub fn count(&self, p: Px) -> usize {
        self.px.iter().filter(|c| **c == p).count()
    }

    /// BGR555 texels through `map`; unmapped cells are transparent.
    pub fn bgr555(&self, map: impl Fn(Px) -> Option<u16>) -> [u16; SPRITE * SPRITE] {
        let mut out = [CLEAR_555; SPRITE * SPRITE];
        for (o, p) in out.iter_mut().zip(self.px.iter()) {
            if let Some(c) = map(*p) {
                *o = c;
            }
        }
        out
    }

    /// On paper: all four colours (each exact in BGR555).
    pub fn paper_555(&self) -> [u16; SPRITE * SPRITE] {
        self.bgr555(|p| match p {
            Px::Ink => Some(pixel::INK_555),
            Px::Cream => Some(pixel::CREAM_555),
            Px::Red => Some(pixel::RED_555),
            Px::Shade => Some(pixel::SHADE_555),
            Px::None => None,
        })
    }

    /// On an ink screen: outline and leaves in cream, seeds in red, body transparent.
    pub fn night_555(&self) -> [u16; SPRITE * SPRITE] {
        self.bgr555(|p| match p {
            Px::Ink => Some(pixel::CREAM_555),
            Px::Red => Some(pixel::RED_555),
            _ => None,
        })
    }

    /// `0xAARRGGBB` per cell through `map` (window icons); unmapped cells are clear.
    pub fn argb(&self, map: impl Fn(Px) -> Option<Rgb>) -> [u32; SPRITE * SPRITE] {
        let mut out = [0u32; SPRITE * SPRITE];
        for (o, p) in out.iter_mut().zip(self.px.iter()) {
            if let Some([r, g, b]) = map(*p) {
                *o = 0xFF00_0000 | (r as u32) << 16 | (g as u32) << 8 | b as u32;
            }
        }
        out
    }

    /// The paper colours on a clear background.
    pub fn argb_paper(&self) -> [u32; SPRITE * SPRITE] {
        self.argb(|p| match p {
            Px::Ink => Some(pixel::INK),
            Px::Cream => Some(pixel::CREAM),
            Px::Red => Some(pixel::RED),
            Px::Shade => Some(pixel::SHADE),
            Px::None => None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mark_parses_to_the_documented_cells() {
        let s = mark();
        assert_eq!(s.count(Px::Ink), 71);
        assert_eq!(s.count(Px::Cream), 74);
        assert_eq!(s.count(Px::Red), 15, "fifteen seeds");
        assert_eq!(s.count(Px::Shade), 9);
        assert_eq!(s.count(Px::None), 87);
        assert_eq!(s.at(7, 0), Px::Ink, "stem");
        assert_eq!(s.at(3, 7), Px::Red, "first seed");
        assert_eq!(s.at(14, 7), Px::Shade, "lower-right shade");
        assert_eq!(s.at(0, 0), Px::None);
    }

    #[test]
    fn night_mark_is_the_paper_outline_and_seeds() {
        let day = mark();
        let night = parse_mark(MARK_NIGHT_SVG);
        for i in 0..SPRITE * SPRITE {
            let want = match day.px[i] {
                Px::Ink => Px::Ink,
                Px::Red => Px::Red,
                _ => Px::None,
            };
            assert_eq!(night.px[i], want, "cell {i}");
        }
        let tex = day.night_555();
        assert_eq!(tex.iter().filter(|t| **t == pixel::CREAM_555).count(), 71);
        assert_eq!(tex.iter().filter(|t| **t == pixel::RED_555).count(), 15);
        assert_eq!(tex.iter().filter(|t| **t == CLEAR_555).count(), 87 + 74 + 9);
    }

    #[test]
    fn paper_texels_are_exact_and_argb_is_opaque_where_drawn() {
        let tex = mark().paper_555();
        for t in tex {
            assert!(matches!(t, pixel::INK_555 | pixel::CREAM_555 | pixel::RED_555 | pixel::SHADE_555 | CLEAR_555));
        }
        // the 5-bit values are exact; the frontend's expansion replicates the top bits downward
        assert_eq!(gba_test::bgr555_to_rgb(pixel::CREAM_555).map(|c| c & 0xF8), pixel::CREAM);
        assert_eq!(gba_test::bgr555_to_rgb(pixel::RED_555).map(|c| c & 0xF8), pixel::RED);
        let argb = mark().argb_paper();
        assert_eq!(argb.iter().filter(|a| **a == 0).count(), 87);
        assert_eq!(argb[7], 0xFF18_1818);
    }

    #[test]
    fn icon_bytes_are_an_ico_with_six_png_entries() {
        assert_eq!(&ICO[0..4], &[0, 0, 1, 0]);
        let n = u16::from_le_bytes([ICO[4], ICO[5]]) as usize;
        assert_eq!(n, 6);
        for i in 0..n {
            let e = 6 + i * 16;
            let off = u32::from_le_bytes([ICO[e + 12], ICO[e + 13], ICO[e + 14], ICO[e + 15]]) as usize;
            assert_eq!(&ICO[off..off + 4], b"\x89PNG", "entry {i}");
        }
        assert_eq!(&ICON_48_PNG[..4], b"\x89PNG");
        assert_eq!(&ICON_128_PNG[..4], b"\x89PNG");
    }
}
