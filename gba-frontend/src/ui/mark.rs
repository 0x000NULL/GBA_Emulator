//! A pixel mark: a 16x16 sprite parsed from an SVG with the same parser as the line drawings,
//! so the SVG stays the source of truth. Each closed subpath is an axis-aligned run of cells in
//! one of the four Pixel colours (`tokens::pixel`; the Night chrome colours count as ink and red).
//!
//! Whole-number scales only, never smoothed. On paper a mark uses all four colours; on an ink
//! screen the night variant draws the outline in cream and the seeds in red and leaves the body
//! transparent.

#![allow(dead_code)]

use super::drawings::{self, Geometry, Seg};
use super::tokens::{pixel, Rgb, NIGHT};

/// Side of the sprite in pixels.
pub const SPRITE: usize = 16;
/// Transparent texel for `Canvas::blit_indexed` / `Overlay::sprite` (bit 15 set).
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

    const SVG: &str = r##"<svg viewBox="0 0 16 16"><path fill="#181818" d="M0 0 L2 0 L2 1 L0 1 z M5 5 L6 5 L6 7 L5 7 z"/><path fill="#D03828" d="M1 1 L2 1 L2 2 L1 2 z"/><path fill="#F0E8D8" d="M15 15 L17 15 L17 17 L15 17 z"/><path fill="#123456" d="M8 8 L9 8 L9 9 L8 9 z"/></svg>"##;

    #[test]
    fn closed_subpaths_fill_their_cells() {
        let s = parse_mark(SVG);
        assert_eq!((s.at(0, 0), s.at(1, 0), s.at(2, 0)), (Px::Ink, Px::Ink, Px::None));
        assert_eq!((s.at(5, 5), s.at(5, 6)), (Px::Ink, Px::Ink), "second subpath");
        assert_eq!(s.at(1, 1), Px::Red);
        assert_eq!(s.at(15, 15), Px::Cream, "clipped to the sprite");
        assert_eq!(s.at(8, 8), Px::None, "a colour outside the palette is skipped");
        assert_eq!(s.count(Px::Ink), 4);
    }

    #[test]
    fn texels_and_argb_follow_the_palette() {
        let s = parse_mark(SVG);
        let paper = s.paper_555();
        assert_eq!((paper[0], paper[17], paper[255]), (pixel::INK_555, pixel::RED_555, pixel::CREAM_555));
        assert_eq!(paper[2], CLEAR_555);
        let night = s.night_555();
        assert_eq!((night[0], night[17], night[255]), (pixel::CREAM_555, pixel::RED_555, CLEAR_555));
        let argb = s.argb_paper();
        assert_eq!((argb[0], argb[2]), (0xFF18_1818, 0));
    }
}
