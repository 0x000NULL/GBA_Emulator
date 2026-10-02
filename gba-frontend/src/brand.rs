//! The Strawberry mark, name and icons (`docs/branding/`), and the GBA-shaped line drawings
//! (`docs/gui/drawings/handheld-*.svg`). The mark is a `ui::mark` sprite; the icons are embedded
//! for the window (`icon.rs`) and the exe (`build.rs`).

#![allow(dead_code)]

pub use crate::ui::mark::{parse_mark, Sprite, SPRITE};
use std::sync::OnceLock;

pub const NAME: &str = "Strawberry";
pub const WORDMARK: &str = "STRAWBERRY";
pub const SERIES: &str = "Fruit Basket · No. 2";

pub const MARK_SVG: &str = include_str!("../../docs/branding/logo/strawberry-mark.svg");
pub const MARK_NIGHT_SVG: &str = include_str!("../../docs/branding/logo/strawberry-mark-night.svg");
/// `strawberry.ico`: 16, 32, 48, 64, 128 and 256 px entries.
pub const ICO: &[u8] = include_bytes!("../../docs/branding/icons/strawberry.ico");
pub const ICON_48_PNG: &[u8] = include_bytes!("../../docs/branding/icons/png/strawberry-icon-48.png");
pub const ICON_128_PNG: &[u8] = include_bytes!("../../docs/branding/icons/png/strawberry-icon-128.png");

/// The window icon (`icon::install`).
pub const ICON: crate::icon::Icon = crate::icon::Icon { ico: ICO, ico_name: "strawberry.ico", pngs: &[ICON_48_PNG, ICON_128_PNG], sprite: mark };

/// The handheld figure of the Settings Controls page and the Library's console.
pub const HANDHELD_CONTROLS: &str = include_str!("../../docs/gui/drawings/handheld-controls.svg");
pub const HANDHELD_LIBRARY: &str = include_str!("../../docs/gui/drawings/handheld-library.svg");

/// The paper mark, parsed once.
pub fn mark() -> &'static Sprite {
    static MARK: OnceLock<Sprite> = OnceLock::new();
    MARK.get_or_init(|| parse_mark(MARK_SVG))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::mark::{Px, CLEAR_555};
    use crate::ui::tokens::pixel;

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

    #[test]
    fn controls_drawing_parses() {
        let d = crate::ui::drawings::parse(HANDHELD_CONTROLS);
        assert_eq!((d.width, d.height), (504.0, 280.0));
        // 4 rects + 11 circles + 8 paths (1 pad + 7 leaders)
        assert_eq!(d.shapes.len(), 23);
        let rects = d.shapes.iter().filter(|s| matches!(s.geom, crate::ui::drawings::Geometry::Rect { .. })).count();
        let circles = d.shapes.iter().filter(|s| matches!(s.geom, crate::ui::drawings::Geometry::Circle { .. })).count();
        assert_eq!((rects, circles), (4, 11));
        // root defaults: no fill, ink stroke 1.5
        assert_eq!(d.shapes[0].stroke, Some(crate::ui::tokens::PAPER.fg));
        assert_eq!(d.shapes[0].stroke_width, 1.5);
        assert_eq!(d.shapes[0].fill, None);
        // the body is filled paper
        assert_eq!(d.shapes[2].fill, Some(crate::ui::tokens::PAPER.bg));
        // leader lines inherit the group's stroke-width 1
        let leaders: Vec<_> = d.shapes.iter().filter(|s| s.stroke_width == 1.0).collect();
        assert_eq!(leaders.len(), 7);
        // red callout dots: filled spot, no stroke
        let dots: Vec<_> = d.shapes.iter().filter(|s| s.fill == Some(crate::ui::tokens::hex(0xC8322A))).collect();
        assert_eq!(dots.len(), 7);
        assert!(dots.iter().all(|s| s.stroke.is_none()));
    }

    #[test]
    fn library_drawing_parses() {
        let d = crate::ui::drawings::parse(HANDHELD_LIBRARY);
        assert_eq!((d.width, d.height), (436.0, 320.0));
        assert_eq!(d.shapes.len(), 13);
        let ring = d.shapes.iter().find(|s| matches!(s.geom, crate::ui::drawings::Geometry::Circle { r, .. } if r == 15.0)).unwrap();
        assert_eq!(ring.stroke, Some(crate::ui::tokens::hex(0xC8322A)));
        assert_eq!(ring.stroke_width, 3.0);
        let speaker: Vec<_> = d.shapes.iter().filter(|s| matches!(s.geom, crate::ui::drawings::Geometry::Circle { r, .. } if r == 1.3)).collect();
        assert_eq!(speaker.len(), 4);
        assert_eq!(speaker[0].fill, Some(crate::ui::tokens::PAPER.fg));
    }

    #[test]
    fn handheld_drawings_render() {
        use crate::ui::tokens::PAPER;
        use crate::ui::{Canvas, Fonts};
        let mut c = Canvas::new(520, 340, Fonts::new());
        c.clear(PAPER.bg);
        for svg in [HANDHELD_CONTROLS, HANDHELD_LIBRARY] {
            c.drawing(&crate::ui::drawings::parse(svg), 0.0, 0.0);
        }
        let ink = (0..520).flat_map(|x| (0..340).map(move |y| (x, y))).filter(|&(x, y)| c.pixel(x, y) != PAPER.bg).count();
        assert!(ink > 1000);
    }

    #[test]
    fn the_x11_buffer_lists_both_icon_sizes() {
        let (w, h, px) = crate::icon::decode_argb(ICON_48_PNG).expect("48 px icon");
        assert_eq!((w, h, px.len()), (48, 48, 48 * 48));
        assert!(px.iter().any(|p| p >> 24 == 0xFF), "opaque pixels");
        let buf = crate::icon::net_wm_icon(&ICON);
        assert_eq!((buf[0], buf[1]), (48, 48));
        let next = 2 + 48 * 48;
        assert_eq!((buf[next], buf[next + 1]), (128, 128));
        assert_eq!(buf.len(), next + 2 + 128 * 128);
    }
}
