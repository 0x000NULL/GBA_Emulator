//! The three line drawings in `docs/gui/drawings/*.svg`, parsed from the SVG subset they use
//! (`rect` with `rx`, `circle`, `path` with `M L C h v z`, `g` with fill/stroke/stroke-width)
//! so the SVG files stay the source of truth.

use super::tokens::Rgb;

pub const HANDHELD_CONTROLS: &str = include_str!("../../../docs/gui/drawings/handheld-controls.svg");
pub const HANDHELD_LIBRARY: &str = include_str!("../../../docs/gui/drawings/handheld-library.svg");
pub const PENCIL_RING: &str = include_str!("../../../docs/gui/drawings/pencil-ring.svg");

#[derive(Clone, Debug, PartialEq)]
pub enum Geometry {
    Rect { x: f32, y: f32, w: f32, h: f32, rx: f32 },
    Circle { cx: f32, cy: f32, r: f32 },
    /// Path segments, already absolute.
    Path(Vec<Seg>),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Seg {
    Move(f32, f32),
    Line(f32, f32),
    Cubic(f32, f32, f32, f32, f32, f32),
    Close,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Shape {
    pub geom: Geometry,
    pub fill: Option<Rgb>,
    pub stroke: Option<Rgb>,
    pub stroke_width: f32,
    pub round_cap: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Drawing {
    pub width: f32,
    pub height: f32,
    pub shapes: Vec<Shape>,
}

#[derive(Clone, Copy)]
struct Style {
    fill: Option<Rgb>,
    stroke: Option<Rgb>,
    stroke_width: f32,
    round_cap: bool,
}

fn parse_color(s: &str) -> Option<Rgb> {
    let s = s.trim();
    if s == "none" {
        return None;
    }
    let v = u32::from_str_radix(s.trim_start_matches('#'), 16).ok()?;
    Some(super::tokens::hex(v))
}

/// Attribute lookup inside one tag's text.
fn attr<'a>(tag: &'a str, name: &str) -> Option<&'a str> {
    let mut rest = tag;
    while let Some(i) = rest.find(name) {
        let after = &rest[i + name.len()..];
        let before_ok = i == 0 || !rest.as_bytes()[i - 1].is_ascii_alphanumeric() && rest.as_bytes()[i - 1] != b'-';
        if before_ok && after.starts_with('=') {
            let q = after.as_bytes()[1];
            let body = &after[2..];
            let end = body.find(q as char)?;
            return Some(&body[..end]);
        }
        rest = after;
    }
    None
}

fn num(tag: &str, name: &str) -> f32 {
    attr(tag, name).and_then(|v| v.trim().parse().ok()).unwrap_or(0.0)
}

fn apply_style(tag: &str, base: Style) -> Style {
    let mut s = base;
    if let Some(v) = attr(tag, "fill") {
        s.fill = parse_color(v);
    }
    if let Some(v) = attr(tag, "stroke") {
        s.stroke = parse_color(v);
    }
    if let Some(v) = attr(tag, "stroke-width") {
        s.stroke_width = v.trim().parse().unwrap_or(s.stroke_width);
    }
    if attr(tag, "stroke-linecap") == Some("round") {
        s.round_cap = true;
    }
    s
}

/// Parse `d`: absolute `M L C Z`, relative `h v`.
pub fn parse_path(d: &str) -> Vec<Seg> {
    let mut segs = Vec::new();
    let mut nums: Vec<f32> = Vec::new();
    let mut cmd = ' ';
    let (mut x, mut y) = (0.0f32, 0.0f32);
    let flush = |cmd: char, nums: &mut Vec<f32>, segs: &mut Vec<Seg>, x: &mut f32, y: &mut f32| {
        match cmd {
            'M' => {
                for p in nums.chunks(2) {
                    *x = p[0];
                    *y = p[1];
                    segs.push(Seg::Move(*x, *y));
                }
            }
            'L' => {
                for p in nums.chunks(2) {
                    *x = p[0];
                    *y = p[1];
                    segs.push(Seg::Line(*x, *y));
                }
            }
            'C' => {
                for p in nums.chunks(6) {
                    segs.push(Seg::Cubic(p[0], p[1], p[2], p[3], p[4], p[5]));
                    *x = p[4];
                    *y = p[5];
                }
            }
            'h' => {
                for v in nums.iter() {
                    *x += v;
                    segs.push(Seg::Line(*x, *y));
                }
            }
            'v' => {
                for v in nums.iter() {
                    *y += v;
                    segs.push(Seg::Line(*x, *y));
                }
            }
            'z' | 'Z' => segs.push(Seg::Close),
            _ => {}
        }
        nums.clear();
    };
    let mut tok = String::new();
    let push_tok = |tok: &mut String, nums: &mut Vec<f32>| {
        if !tok.is_empty() {
            if let Ok(v) = tok.parse::<f32>() {
                nums.push(v);
            }
            tok.clear();
        }
    };
    for c in d.chars() {
        if c.is_ascii_alphabetic() {
            push_tok(&mut tok, &mut nums);
            flush(cmd, &mut nums, &mut segs, &mut x, &mut y);
            cmd = c;
            if cmd == 'z' || cmd == 'Z' {
                flush(cmd, &mut nums, &mut segs, &mut x, &mut y);
                cmd = ' ';
            }
        } else if c == '-' && !tok.is_empty() && !tok.ends_with('e') {
            push_tok(&mut tok, &mut nums);
            tok.push(c);
        } else if c.is_ascii_digit() || c == '.' || c == '-' || c == 'e' {
            tok.push(c);
        } else {
            push_tok(&mut tok, &mut nums);
        }
    }
    push_tok(&mut tok, &mut nums);
    flush(cmd, &mut nums, &mut segs, &mut x, &mut y);
    segs
}

/// Parse one of the bundled drawings. Comments are skipped; unknown elements are ignored.
pub fn parse(svg: &str) -> Drawing {
    let mut shapes = Vec::new();
    let mut stack: Vec<Style> = vec![Style { fill: None, stroke: None, stroke_width: 1.0, round_cap: false }];
    let (mut width, mut height) = (0.0, 0.0);
    let mut rest = svg;
    while let Some(start) = rest.find('<') {
        let after = &rest[start + 1..];
        if after.starts_with("!--") {
            let end = after.find("-->").map(|i| i + 3).unwrap_or(after.len());
            rest = &after[end..];
            continue;
        }
        let end = after.find('>').unwrap_or(after.len());
        let tag = &after[..end];
        rest = &after[end + 1..];
        let self_closing = tag.ends_with('/');
        let closing = tag.starts_with('/');
        let name = tag.trim_start_matches('/').split(|c: char| c.is_whitespace() || c == '/').next().unwrap_or("");
        let base = *stack.last().unwrap();
        match (closing, name) {
            (false, "svg") => {
                width = num(tag, "width");
                height = num(tag, "height");
                let s = apply_style(tag, base);
                *stack.last_mut().unwrap() = s;
            }
            (false, "g") => {
                let s = apply_style(tag, base);
                if self_closing {
                    continue;
                }
                stack.push(s);
            }
            (true, "g") => {
                if stack.len() > 1 {
                    stack.pop();
                }
            }
            (false, "rect" | "circle" | "path") => {
                let s = apply_style(tag, base);
                let geom = match name {
                    "rect" => Geometry::Rect {
                        x: num(tag, "x"),
                        y: num(tag, "y"),
                        w: num(tag, "width"),
                        h: num(tag, "height"),
                        rx: num(tag, "rx"),
                    },
                    "circle" => Geometry::Circle { cx: num(tag, "cx"), cy: num(tag, "cy"), r: num(tag, "r") },
                    _ => Geometry::Path(parse_path(attr(tag, "d").unwrap_or(""))),
                };
                shapes.push(Shape { geom, fill: s.fill, stroke: s.stroke, stroke_width: s.stroke_width, round_cap: s.round_cap });
            }
            _ => {}
        }
    }
    Drawing { width, height, shapes }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::tokens::{hex, PAPER};

    #[test]
    fn controls_drawing_parses() {
        let d = parse(HANDHELD_CONTROLS);
        assert_eq!((d.width, d.height), (504.0, 280.0));
        // 4 rects + 11 circles + 8 paths (1 pad + 7 leaders)
        assert_eq!(d.shapes.len(), 23);
        let rects = d.shapes.iter().filter(|s| matches!(s.geom, Geometry::Rect { .. })).count();
        let circles = d.shapes.iter().filter(|s| matches!(s.geom, Geometry::Circle { .. })).count();
        assert_eq!((rects, circles), (4, 11));
        // root defaults: no fill, ink stroke 1.5
        assert_eq!(d.shapes[0].stroke, Some(PAPER.fg));
        assert_eq!(d.shapes[0].stroke_width, 1.5);
        assert_eq!(d.shapes[0].fill, None);
        // the body is filled paper
        assert_eq!(d.shapes[2].fill, Some(PAPER.bg));
        // leader lines inherit the group's stroke-width 1
        let leaders: Vec<_> = d.shapes.iter().filter(|s| s.stroke_width == 1.0).collect();
        assert_eq!(leaders.len(), 7);
        // red callout dots: filled spot, no stroke
        let dots: Vec<_> = d.shapes.iter().filter(|s| s.fill == Some(hex(0xC8322A))).collect();
        assert_eq!(dots.len(), 7);
        assert!(dots.iter().all(|s| s.stroke.is_none()));
    }

    #[test]
    fn library_drawing_parses() {
        let d = parse(HANDHELD_LIBRARY);
        assert_eq!((d.width, d.height), (436.0, 320.0));
        assert_eq!(d.shapes.len(), 13);
        let ring = d.shapes.iter().find(|s| matches!(s.geom, Geometry::Circle { r, .. } if r == 15.0)).unwrap();
        assert_eq!(ring.stroke, Some(hex(0xC8322A)));
        assert_eq!(ring.stroke_width, 3.0);
        let speaker: Vec<_> = d.shapes.iter().filter(|s| matches!(s.geom, Geometry::Circle { r, .. } if r == 1.3)).collect();
        assert_eq!(speaker.len(), 4);
        assert_eq!(speaker[0].fill, Some(PAPER.fg));
    }

    #[test]
    fn pencil_ring_parses() {
        let d = parse(PENCIL_RING);
        assert_eq!(d.shapes.len(), 1);
        let s = &d.shapes[0];
        assert_eq!(s.stroke, Some(hex(0xC8322A)));
        assert_eq!(s.stroke_width, 3.5);
        assert!(s.round_cap);
        match &s.geom {
            Geometry::Path(segs) => {
                assert_eq!(segs[0], Seg::Move(150.0, 14.0));
                assert_eq!(segs.len(), 6);
                assert!(matches!(segs[5], Seg::Cubic(..)));
            }
            g => panic!("{g:?}"),
        }
    }

    #[test]
    fn path_parser_handles_relative_h_v_and_close() {
        let segs = parse_path("M165 134h14v10h10v14h-10v10h-14v-10h-10v-14h10z");
        assert_eq!(segs[0], Seg::Move(165.0, 134.0));
        assert_eq!(segs[1], Seg::Line(179.0, 134.0));
        assert_eq!(segs[2], Seg::Line(179.0, 144.0));
        assert_eq!(*segs.last().unwrap(), Seg::Close);
        assert_eq!(segs.len(), 13);
        let segs = parse_path("M172 76 L126 34");
        assert_eq!(segs, vec![Seg::Move(172.0, 76.0), Seg::Line(126.0, 34.0)]);
    }
}
