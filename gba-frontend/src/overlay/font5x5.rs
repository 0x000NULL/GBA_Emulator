//! The in-game pixel font: 5x5 uppercase caps on a 6 px advance and an 8 px line, drawn at
//! native 240x160 into the frame before upscaling. Glyphs are our own (`DESIGN.md`, "Type").
//! Lowercase is drawn as uppercase; unknown characters draw as a small box.

pub const GLYPH_W: usize = 5;
pub const GLYPH_H: usize = 5;
pub const ADVANCE: i32 = 6;
#[cfg_attr(not(test), allow(dead_code))]
pub const LINE: i32 = 8;

/// Each glyph is five rows of five bits, MSB = leftmost pixel.
pub fn glyph(ch: char) -> [u8; 5] {
    let ch = ch.to_ascii_uppercase();
    match ch {
        ' ' => [0, 0, 0, 0, 0],
        'A' => [0b01110, 0b10001, 0b11111, 0b10001, 0b10001],
        'B' => [0b11110, 0b10001, 0b11110, 0b10001, 0b11110],
        'C' => [0b01111, 0b10000, 0b10000, 0b10000, 0b01111],
        'D' => [0b11110, 0b10001, 0b10001, 0b10001, 0b11110],
        'E' => [0b11111, 0b10000, 0b11110, 0b10000, 0b11111],
        'F' => [0b11111, 0b10000, 0b11110, 0b10000, 0b10000],
        'G' => [0b01111, 0b10000, 0b10111, 0b10001, 0b01111],
        'H' => [0b10001, 0b10001, 0b11111, 0b10001, 0b10001],
        'I' => [0b11111, 0b00100, 0b00100, 0b00100, 0b11111],
        'J' => [0b00111, 0b00001, 0b00001, 0b10001, 0b01110],
        'K' => [0b10001, 0b10010, 0b11100, 0b10010, 0b10001],
        'L' => [0b10000, 0b10000, 0b10000, 0b10000, 0b11111],
        'M' => [0b10001, 0b11011, 0b10101, 0b10001, 0b10001],
        'N' => [0b10001, 0b11001, 0b10101, 0b10011, 0b10001],
        'O' => [0b01110, 0b10001, 0b10001, 0b10001, 0b01110],
        'P' => [0b11110, 0b10001, 0b11110, 0b10000, 0b10000],
        'Q' => [0b01110, 0b10001, 0b10101, 0b10010, 0b01101],
        'R' => [0b11110, 0b10001, 0b11110, 0b10010, 0b10001],
        'S' => [0b01111, 0b10000, 0b01110, 0b00001, 0b11110],
        'T' => [0b11111, 0b00100, 0b00100, 0b00100, 0b00100],
        'U' => [0b10001, 0b10001, 0b10001, 0b10001, 0b01110],
        'V' => [0b10001, 0b10001, 0b10001, 0b01010, 0b00100],
        'W' => [0b10001, 0b10001, 0b10101, 0b11011, 0b10001],
        'X' => [0b10001, 0b01010, 0b00100, 0b01010, 0b10001],
        'Y' => [0b10001, 0b01010, 0b00100, 0b00100, 0b00100],
        'Z' => [0b11111, 0b00010, 0b00100, 0b01000, 0b11111],
        '0' => [0b01110, 0b10011, 0b10101, 0b11001, 0b01110],
        '1' => [0b00100, 0b01100, 0b00100, 0b00100, 0b01110],
        '2' => [0b01110, 0b10001, 0b00110, 0b01000, 0b11111],
        '3' => [0b11110, 0b00001, 0b00110, 0b00001, 0b11110],
        '4' => [0b00110, 0b01010, 0b10010, 0b11111, 0b00010],
        '5' => [0b11111, 0b10000, 0b11110, 0b00001, 0b11110],
        '6' => [0b01110, 0b10000, 0b11110, 0b10001, 0b01110],
        '7' => [0b11111, 0b00001, 0b00010, 0b00100, 0b00100],
        '8' => [0b01110, 0b10001, 0b01110, 0b10001, 0b01110],
        '9' => [0b01110, 0b10001, 0b01111, 0b00001, 0b01110],
        ':' => [0b00000, 0b00100, 0b00000, 0b00100, 0b00000],
        '·' => [0b00000, 0b00000, 0b00100, 0b00000, 0b00000],
        '/' => [0b00001, 0b00010, 0b00100, 0b01000, 0b10000],
        '-' => [0b00000, 0b00000, 0b11111, 0b00000, 0b00000],
        '×' => [0b00000, 0b01010, 0b00100, 0b01010, 0b00000],
        '▸' => [0b01000, 0b01100, 0b01110, 0b01100, 0b01000],
        '◂' => [0b00010, 0b00110, 0b01110, 0b00110, 0b00010],
        '(' => [0b00010, 0b00100, 0b00100, 0b00100, 0b00010],
        ')' => [0b01000, 0b00100, 0b00100, 0b00100, 0b01000],
        '.' => [0b00000, 0b00000, 0b00000, 0b00000, 0b00100],
        ',' => [0b00000, 0b00000, 0b00000, 0b00100, 0b01000],
        '…' => [0b00000, 0b00000, 0b00000, 0b00000, 0b10101],
        '\'' => [0b00100, 0b00100, 0b00000, 0b00000, 0b00000],
        '?' => [0b01110, 0b10001, 0b00110, 0b00000, 0b00100],
        '!' => [0b00100, 0b00100, 0b00100, 0b00000, 0b00100],
        '_' => [0b00000, 0b00000, 0b00000, 0b00000, 0b11111],
        '+' => [0b00000, 0b00100, 0b01110, 0b00100, 0b00000],
        '%' => [0b11001, 0b11010, 0b00100, 0b01011, 0b10011],
        '&' => [0b01100, 0b10010, 0b01100, 0b10101, 0b01010],
        '[' => [0b00110, 0b00100, 0b00100, 0b00100, 0b00110],
        ']' => [0b01100, 0b00100, 0b00100, 0b00100, 0b01100],
        '\\' => [0b10000, 0b01000, 0b00100, 0b00010, 0b00001],
        '"' => [0b01010, 0b01010, 0b00000, 0b00000, 0b00000],
        '=' => [0b00000, 0b11111, 0b00000, 0b11111, 0b00000],
        '*' => [0b00000, 0b10101, 0b01110, 0b10101, 0b00000],
        '#' => [0b01010, 0b11111, 0b01010, 0b11111, 0b01010],
        _ => [0b11111, 0b10001, 0b10001, 0b10001, 0b11111],
    }
}

/// Width in pixels of a string on one line (no trailing advance gap).
pub fn width(text: &str) -> i32 {
    let n = text.chars().count() as i32;
    if n == 0 {
        0
    } else {
        n * ADVANCE - 1
    }
}

/// Maximum characters per overlay line (`DESIGN.md`: 38) and the truncation rule.
pub const MAX_LINE: usize = 38;

pub fn truncate(text: &str) -> String {
    let n = text.chars().count();
    if n <= MAX_LINE {
        return text.to_string();
    }
    let mut s: String = text.chars().take(MAX_LINE - 1).collect();
    s.push('…');
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    const REQUIRED: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789 :·/-×▸().,…";

    #[test]
    fn required_glyphs_exist_and_fit_five_columns() {
        let fallback = glyph('\u{1F600}');
        for ch in REQUIRED.chars() {
            let g = glyph(ch);
            assert!(g.iter().all(|row| *row < 32), "{ch:?} wider than 5");
            if ch != ' ' {
                assert_ne!(g, fallback, "{ch:?} missing");
                assert!(g.iter().any(|r| *r != 0), "{ch:?} empty");
            }
        }
        assert_eq!(glyph('a'), glyph('A'), "lowercase draws as uppercase");
    }

    #[test]
    fn metrics_and_truncation() {
        assert_eq!(width(""), 0);
        assert_eq!(width("A"), 5);
        assert_eq!(width("AB"), 11);
        assert_eq!(ADVANCE, 6);
        assert_eq!(LINE, 8);
        let long = "X".repeat(50);
        let t = truncate(&long);
        assert_eq!(t.chars().count(), MAX_LINE);
        assert!(t.ends_with('…'));
        assert_eq!(truncate("short"), "short");
    }
}
