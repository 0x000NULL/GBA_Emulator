//! Design tokens, transcribed from `docs/gui/tokens.json` ("paper, ink, one red").
//!
//! The unit test at the bottom parses the JSON and checks every colour and metric here against
//! it, so the two cannot drift apart silently. Sizes are CSS px at the 1280x800 design size;
//! `pixel::*` values are native GBA pixels (240x160).

// Toolkit surface: the screens use a subset of these helpers.
#![allow(dead_code)]

/// Opaque colour, `[r, g, b]`.
pub type Rgb = [u8; 3];

pub const fn hex(v: u32) -> Rgb {
    [(v >> 16) as u8, (v >> 8) as u8, v as u8]
}

/// One of the two chrome palettes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Palette {
    pub bg: Rgb,
    pub bg2: Rgb,
    pub fg: Rgb,
    pub fg2: Rgb,
    pub line: Rgb,
    pub spot: Rgb,
}

/// Menus away from a game: Library, Save states, Settings.
pub const PAPER: Palette = Palette {
    bg: hex(0xF4EFE3),
    bg2: hex(0xE7E0CF),
    fg: hex(0x1A1A1A),
    fg2: hex(0x5A5448),
    line: hex(0xD6CDB9),
    spot: hex(0xC8322A),
};

/// Anything around a running or paused game: in-game chrome, letterbox, Paused, Bench.
pub const NIGHT: Palette = Palette {
    bg: hex(0x141414),
    bg2: hex(0x1D1C1A),
    fg: hex(0xEDE6D6),
    fg2: hex(0x9C958A),
    line: hex(0x2F2C27),
    spot: hex(0xE5483C),
};

/// In-game overlay colours: every channel is a multiple of 8 so each is exact in BGR555.
pub mod pixel {
    use super::{hex, Rgb};
    pub const CREAM: Rgb = hex(0xF0E8D8);
    pub const SHADE: Rgb = hex(0xB8B0A0);
    pub const INK: Rgb = hex(0x181818);
    pub const RED: Rgb = hex(0xD03828);
    pub const DIM: Rgb = hex(0x686058);
    pub const EMPTY: Rgb = hex(0xD8D0C0);
    pub const CREAM_555: u16 = 0x6FBE;
    pub const SHADE_555: u16 = 0x52D7;
    pub const INK_555: u16 = 0x0C63;
    pub const RED_555: u16 = 0x14FA;
    pub const DIM_555: u16 = 0x2D8D;
    pub const EMPTY_555: u16 = 0x635B;
    /// BLDY-style brightness decrease applied to the frame behind the overlay.
    pub const DIM_EVY: u16 = 9;
    pub const WINDOW_PADDING: i32 = 4;
    pub const MENU_PITCH: i32 = 14;
    pub const THUMB_W: usize = 48;
    pub const THUMB_H: usize = 32;
}

/// Object colours: cartridge shell, case foam, film, loupe.
pub mod object {
    use super::{hex, Rgb};
    pub const CART_SHELL: Rgb = hex(0xA9A9B3);
    pub const CART_SHELL_SHADOW: Rgb = hex(0x8B8B95);
    pub const CART_SHELL_HIGHLIGHT: Rgb = hex(0xCFCFD6);
    pub const CART_GRIP_A: Rgb = hex(0x9C9CA6);
    pub const CART_GRIP_B: Rgb = hex(0xB4B4BD);
    pub const CASE_OUTER: Rgb = hex(0x262420);
    pub const FOAM: Rgb = hex(0x1C1B19);
    pub const FOAM_DOT: Rgb = hex(0x282622);
    pub const FILM: Rgb = hex(0x161616);
    pub const FILM_EMPTY: Rgb = hex(0x222222);
    pub const FILM_EDGE_TEXT: Rgb = hex(0xCFC6B4);
    pub const LOUPE_RING: Rgb = hex(0x1C1C1C);
    pub const LOUPE_RIM: Rgb = hex(0x3A3A3A);
}

/// Cartridge label colour pairs `(bg, ink)`, chosen by a stable hash of the game code.
pub const LABELS: [(Rgb, Rgb); 12] = [
    (hex(0xEFE4C4), hex(0x1F4D2B)),
    (hex(0x7A1F1F), hex(0xF6E7C8)),
    (hex(0x101010), hex(0xF2F2F2)),
    (hex(0xD8392C), hex(0xFFFFFF)),
    (hex(0xECE8DE), hex(0x222222)),
    (hex(0x2E4A6B), hex(0xF1E6C8)),
    (hex(0xF2C230), hex(0x1B1B1B)),
    (hex(0xE8761E), hex(0x1B1B1B)),
    (hex(0x17242F), hex(0xA6E36A)),
    (hex(0x1A1A2E), hex(0xD8C58A)),
    (hex(0x2A2A2A), hex(0xE6E6E6)),
    (hex(0xF3EFE6), hex(0x1D1D35)),
];

/// Stable label colours for a game code (FNV-1a over the bytes).
pub fn label_for(code: &str) -> (Rgb, Rgb) {
    let mut h: u32 = 0x811C_9DC5;
    for b in code.bytes() {
        h ^= b as u32;
        h = h.wrapping_mul(0x0100_0193);
    }
    LABELS[(h % LABELS.len() as u32) as usize]
}

pub mod layout {
    pub const DESIGN_W: u32 = 1280;
    pub const DESIGN_H: u32 = 800;
    pub const MIN_W: u32 = 960;
    pub const MIN_H: u32 = 640;
    pub const MARGIN: f32 = 40.0;
    pub const HEAD_H: f32 = 56.0;
    pub const HEAD_RULE: f32 = 1.5;
    pub const HEAD_GAP: f32 = 14.0;
    pub const FOOT_H: f32 = 44.0;
    pub const FOOT_RULE: f32 = 1.0;
    pub const FOOT_GAP: f32 = 22.0;
    pub const GUTTER: f32 = 44.0;
    pub const SECTION_GAP: f32 = 26.0;
}

pub mod component {
    pub const KEY_H: f32 = 22.0;
    pub const KEY_MIN_W: f32 = 22.0;
    pub const KEY_PAD_X: f32 = 6.0;
    pub const KEY_BORDER: f32 = 1.5;
    pub const KEY_BORDER_BOTTOM: f32 = 3.0;
    pub const KEY_RADIUS: f32 = 3.0;
    pub const TICK_SIZE: f32 = 14.0;
    pub const TICK_BORDER: f32 = 1.5;
    pub const TICK_INSET: f32 = 2.0;
    pub const BUTTON_H: f32 = 44.0;
    pub const BUTTON_PAD_X: f32 = 22.0;
    pub const BUTTON_OUTLINE: f32 = 1.5;
    pub const CART_W: f32 = 156.0;
    pub const CART_H: f32 = 108.0;
    pub const CART_RADIUS: [f32; 4] = [7.0, 7.0, 3.0, 3.0];
    pub const CART_GRIP_TOP: f32 = 4.0;
    pub const CART_GRIP_INSET: f32 = 22.0;
    pub const CART_GRIP_H: f32 = 11.0;
    /// top, right, bottom, left
    pub const CART_LABEL_INSET: [f32; 4] = [21.0, 9.0, 9.0, 9.0];
    pub const CART_LABEL_RADIUS: f32 = 3.0;
    pub const CART_ART_W: f32 = 52.0;
    pub const CART_HOVER_LIFT: f32 = 4.0;
    pub const CART_HOVER_MS: u32 = 120;
    pub const FILM_FRAME_W: f32 = 240.0;
    pub const FILM_FRAME_H: f32 = 160.0;
    pub const FILM_FRAME_GAP: f32 = 12.0;
    pub const FILM_STRIP_PAD_X: f32 = 12.0;
    pub const FILM_SPROCKET_BAND: f32 = 18.0;
    pub const FILM_HOLE_W: f32 = 12.0;
    pub const FILM_HOLE_H: f32 = 8.0;
    pub const FILM_HOLE_PITCH: f32 = 30.0;
    pub const FILM_TILT: [f32; 2] = [-0.4, 0.3];
    pub const LOUPE_DIAMETER: f32 = 250.0;
    pub const LOUPE_MAGNIFY: f32 = 3.0;
    pub const LOUPE_RING: f32 = 11.0;
    pub const LOUPE_RIM: f32 = 2.0;
    pub const PENCIL_RING_STROKE: f32 = 3.5;
}

/// Type sizes (px) by role.
pub mod font {
    pub const DISPLAY_SCREEN: f32 = 40.0;
    pub const DISPLAY_TITLE: f32 = 34.0;
    pub const DISPLAY_HERO: f32 = 58.0;
    pub const DISPLAY_CART: f32 = 13.0;
    pub const DISPLAY_HEAD_NUMBER: f32 = 30.0;
    pub const DISPLAY_HEAD_TITLE: f32 = 22.0;
    pub const DISPLAY_LINE_HEIGHT: f32 = 0.9;
    pub const DISPLAY_TRACKING: f32 = 0.3;
    pub const INTERFACE_BODY: f32 = 13.0;
    pub const INTERFACE_BUTTON: f32 = 14.0;
    pub const INTERFACE_EMPHASIS: f32 = 15.0;
    pub const LABEL: f32 = 10.5;
    pub const LABEL_TRACKING: f32 = 1.8;
    pub const READING_BODY: f32 = 13.5;
    pub const READING_INTRO: f32 = 15.0;
    pub const READING_LINE_HEIGHT: f32 = 1.45;
    pub const DATA_SMALL: f32 = 10.5;
    pub const DATA_BODY: f32 = 12.5;
    pub const DATA_KEY: f32 = 11.0;
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    const JSON: &str = include_str!("../../../docs/gui/tokens.json");

    fn parse_hex(s: &str) -> Rgb {
        hex(u32::from_str_radix(s.trim_start_matches('#'), 16).unwrap())
    }

    fn palette(v: &Value) -> Palette {
        let c = |k: &str| parse_hex(v[k].as_str().unwrap());
        Palette { bg: c("bg"), bg2: c("bg2"), fg: c("fg"), fg2: c("fg2"), line: c("line"), spot: c("spot") }
    }

    #[test]
    fn palettes_match_tokens_json() {
        let t: Value = serde_json::from_str(JSON).unwrap();
        assert_eq!(palette(&t["color"]["paper"]), PAPER);
        assert_eq!(palette(&t["color"]["night"]), NIGHT);
        let px = &t["color"]["pixel"];
        let check = |name: &str, rgb: Rgb, bgr: u16| {
            assert_eq!(parse_hex(px[name]["hex"].as_str().unwrap()), rgb, "{name}");
            let s = px[name]["bgr555"].as_str().unwrap();
            assert_eq!(u16::from_str_radix(s.trim_start_matches("0x"), 16).unwrap(), bgr, "{name} bgr555");
            // every channel a multiple of 8, and the BGR555 value really is that colour
            let [r, g, b] = rgb;
            assert_eq!((r as u16 >> 3) | ((g as u16 >> 3) << 5) | ((b as u16 >> 3) << 10), bgr, "{name} exact");
        };
        check("cream", pixel::CREAM, pixel::CREAM_555);
        check("shade", pixel::SHADE, pixel::SHADE_555);
        check("ink", pixel::INK, pixel::INK_555);
        check("red", pixel::RED, pixel::RED_555);
        check("dim", pixel::DIM, pixel::DIM_555);
        check("empty", pixel::EMPTY, pixel::EMPTY_555);
        assert_eq!(t["pixel"]["dim"]["evy"].as_u64().unwrap() as u16, pixel::DIM_EVY);
    }

    #[test]
    fn objects_and_labels_match_tokens_json() {
        let t: Value = serde_json::from_str(JSON).unwrap();
        let o = &t["color"]["object"];
        let c = |k: &str| parse_hex(o[k].as_str().unwrap());
        assert_eq!(c("cartShell"), object::CART_SHELL);
        assert_eq!(c("cartShellShadow"), object::CART_SHELL_SHADOW);
        assert_eq!(c("cartShellHighlight"), object::CART_SHELL_HIGHLIGHT);
        assert_eq!(c("cartGripA"), object::CART_GRIP_A);
        assert_eq!(c("cartGripB"), object::CART_GRIP_B);
        assert_eq!(c("caseOuter"), object::CASE_OUTER);
        assert_eq!(c("foam"), object::FOAM);
        assert_eq!(c("foamDot"), object::FOAM_DOT);
        assert_eq!(c("film"), object::FILM);
        assert_eq!(c("filmEmpty"), object::FILM_EMPTY);
        assert_eq!(c("filmEdgeText"), object::FILM_EDGE_TEXT);
        assert_eq!(c("loupeRing"), object::LOUPE_RING);
        assert_eq!(c("loupeRim"), object::LOUPE_RIM);
        let labels = t["color"]["label"].as_array().unwrap();
        assert_eq!(labels.len(), LABELS.len());
        for (i, l) in labels.iter().enumerate() {
            assert_eq!(parse_hex(l["bg"].as_str().unwrap()), LABELS[i].0, "label {i} bg");
            assert_eq!(parse_hex(l["ink"].as_str().unwrap()), LABELS[i].1, "label {i} ink");
        }
    }

    #[test]
    fn metrics_match_tokens_json() {
        let t: Value = serde_json::from_str(JSON).unwrap();
        let f = |v: &Value| v.as_f64().unwrap() as f32;
        let l = &t["layout"];
        assert_eq!(l["designSize"][0].as_u64().unwrap() as u32, layout::DESIGN_W);
        assert_eq!(l["designSize"][1].as_u64().unwrap() as u32, layout::DESIGN_H);
        assert_eq!(l["minSize"][0].as_u64().unwrap() as u32, layout::MIN_W);
        assert_eq!(l["minSize"][1].as_u64().unwrap() as u32, layout::MIN_H);
        assert_eq!(f(&l["margin"]), layout::MARGIN);
        assert_eq!(f(&l["head"]["height"]), layout::HEAD_H);
        assert_eq!(f(&l["head"]["rule"]), layout::HEAD_RULE);
        assert_eq!(f(&l["head"]["gap"]), layout::HEAD_GAP);
        assert_eq!(f(&l["foot"]["height"]), layout::FOOT_H);
        assert_eq!(f(&l["foot"]["rule"]), layout::FOOT_RULE);
        assert_eq!(f(&l["foot"]["gap"]), layout::FOOT_GAP);
        assert_eq!(f(&l["gutter"]), layout::GUTTER);
        assert_eq!(f(&l["sectionGap"]), layout::SECTION_GAP);
        let c = &t["component"];
        assert_eq!(f(&c["key"]["height"]), component::KEY_H);
        assert_eq!(f(&c["key"]["minWidth"]), component::KEY_MIN_W);
        assert_eq!(f(&c["key"]["padX"]), component::KEY_PAD_X);
        assert_eq!(f(&c["key"]["border"]), component::KEY_BORDER);
        assert_eq!(f(&c["key"]["borderBottom"]), component::KEY_BORDER_BOTTOM);
        assert_eq!(f(&c["key"]["radius"]), component::KEY_RADIUS);
        assert_eq!(f(&c["tick"]["size"]), component::TICK_SIZE);
        assert_eq!(f(&c["tick"]["border"]), component::TICK_BORDER);
        assert_eq!(f(&c["tick"]["inset"]), component::TICK_INSET);
        assert_eq!(f(&c["button"]["height"]), component::BUTTON_H);
        assert_eq!(f(&c["button"]["padX"]), component::BUTTON_PAD_X);
        assert_eq!(f(&c["button"]["outline"]), component::BUTTON_OUTLINE);
        assert_eq!(f(&c["cart"]["size"][0]), component::CART_W);
        assert_eq!(f(&c["cart"]["size"][1]), component::CART_H);
        for i in 0..4 {
            assert_eq!(f(&c["cart"]["radius"][i]), component::CART_RADIUS[i]);
            assert_eq!(f(&c["cart"]["label"]["inset"][i]), component::CART_LABEL_INSET[i]);
        }
        assert_eq!(f(&c["cart"]["grip"]["top"]), component::CART_GRIP_TOP);
        assert_eq!(f(&c["cart"]["grip"]["inset"]), component::CART_GRIP_INSET);
        assert_eq!(f(&c["cart"]["grip"]["height"]), component::CART_GRIP_H);
        assert_eq!(f(&c["cart"]["label"]["radius"]), component::CART_LABEL_RADIUS);
        assert_eq!(f(&c["cart"]["label"]["artWidth"]), component::CART_ART_W);
        assert_eq!(f(&c["cart"]["hoverLift"]), component::CART_HOVER_LIFT);
        assert_eq!(c["cart"]["hoverMs"].as_u64().unwrap() as u32, component::CART_HOVER_MS);
        assert_eq!(f(&c["film"]["frame"][0]), component::FILM_FRAME_W);
        assert_eq!(f(&c["film"]["frame"][1]), component::FILM_FRAME_H);
        assert_eq!(f(&c["film"]["frameGap"]), component::FILM_FRAME_GAP);
        assert_eq!(f(&c["film"]["stripPadX"]), component::FILM_STRIP_PAD_X);
        assert_eq!(f(&c["film"]["sprocketBand"]), component::FILM_SPROCKET_BAND);
        assert_eq!(f(&c["film"]["hole"][0]), component::FILM_HOLE_W);
        assert_eq!(f(&c["film"]["hole"][1]), component::FILM_HOLE_H);
        assert_eq!(f(&c["film"]["holePitch"]), component::FILM_HOLE_PITCH);
        assert_eq!(f(&c["film"]["tilt"][0]), component::FILM_TILT[0]);
        assert_eq!(f(&c["film"]["tilt"][1]), component::FILM_TILT[1]);
        assert_eq!(f(&c["loupe"]["diameter"]), component::LOUPE_DIAMETER);
        assert_eq!(f(&c["loupe"]["magnify"]), component::LOUPE_MAGNIFY);
        assert_eq!(f(&c["loupe"]["ring"]), component::LOUPE_RING);
        assert_eq!(f(&c["loupe"]["rim"]), component::LOUPE_RIM);
        assert_eq!(f(&c["pencilRing"]["stroke"]), component::PENCIL_RING_STROKE);
        let fo = &t["font"];
        assert_eq!(f(&fo["display"]["sizes"]["screen"]), font::DISPLAY_SCREEN);
        assert_eq!(f(&fo["display"]["sizes"]["title"]), font::DISPLAY_TITLE);
        assert_eq!(f(&fo["display"]["sizes"]["hero"]), font::DISPLAY_HERO);
        assert_eq!(f(&fo["display"]["sizes"]["cartTitle"]), font::DISPLAY_CART);
        assert_eq!(f(&fo["display"]["sizes"]["headNumber"]), font::DISPLAY_HEAD_NUMBER);
        assert_eq!(f(&fo["display"]["sizes"]["headTitle"]), font::DISPLAY_HEAD_TITLE);
        assert_eq!(f(&fo["display"]["lineHeight"]), font::DISPLAY_LINE_HEIGHT);
        assert_eq!(f(&fo["display"]["tracking"]), font::DISPLAY_TRACKING);
        assert_eq!(f(&fo["interface"]["sizes"]["body"]), font::INTERFACE_BODY);
        assert_eq!(f(&fo["interface"]["sizes"]["button"]), font::INTERFACE_BUTTON);
        assert_eq!(f(&fo["interface"]["sizes"]["emphasis"]), font::INTERFACE_EMPHASIS);
        assert_eq!(f(&fo["label"]["size"]), font::LABEL);
        assert_eq!(f(&fo["label"]["tracking"]), font::LABEL_TRACKING);
        assert_eq!(f(&fo["reading"]["sizes"]["body"]), font::READING_BODY);
        assert_eq!(f(&fo["reading"]["sizes"]["intro"]), font::READING_INTRO);
        assert_eq!(f(&fo["reading"]["lineHeight"]), font::READING_LINE_HEIGHT);
        assert_eq!(f(&fo["data"]["sizes"]["small"]), font::DATA_SMALL);
        assert_eq!(f(&fo["data"]["sizes"]["body"]), font::DATA_BODY);
        assert_eq!(f(&fo["data"]["sizes"]["key"]), font::DATA_KEY);
        assert_eq!(f(&t["pixel"]["window"]["padding"]), pixel::WINDOW_PADDING as f32);
        assert_eq!(f(&t["pixel"]["menu"]["itemPitch"]), pixel::MENU_PITCH as f32);
        assert_eq!(t["pixel"]["thumb"]["size"][0].as_u64().unwrap() as usize, pixel::THUMB_W);
        assert_eq!(t["pixel"]["thumb"]["size"][1].as_u64().unwrap() as usize, pixel::THUMB_H);
    }

    #[test]
    fn label_hash_is_stable_and_in_range() {
        assert_eq!(label_for("B8CE"), label_for("B8CE"));
        let distinct: std::collections::HashSet<_> = ["B8CE", "BZME", "AZLE", "AINJ", "AMKE", "BZ6E", "AFXE", "AX4E"]
            .iter()
            .map(|c| label_for(c))
            .collect();
        assert!(distinct.len() >= 4, "eight codes should spread over several labels");
    }
}
