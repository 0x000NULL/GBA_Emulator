//! Text: the seven embedded faces, a glyph cache, measurement and line fitting.
//!
//! Faces follow `docs/gui/DESIGN.md` "Type": Display = Archivo ExtraCondensed Black (uppercase),
//! Interface = Archivo SemiBold/Bold, Label = Archivo Bold 10.5 tracked, Reading = Source Serif 4,
//! Data = IBM Plex Mono. All OFL; licences sit beside the files in `assets/fonts/`.

use super::tokens::{font as sizes, Rgb};
use fontdue::{Font, FontSettings};
use std::collections::HashMap;
use std::rc::Rc;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Face {
    Display,
    Interface,
    InterfaceBold,
    Reading,
    Data,
    DataMedium,
    DataSemi,
}

const FACE_DATA: [&[u8]; 7] = [
    include_bytes!("../../assets/fonts/ArchivoExtraCondensed-Black.ttf"),
    include_bytes!("../../assets/fonts/Archivo-SemiBold.ttf"),
    include_bytes!("../../assets/fonts/Archivo-Bold.ttf"),
    include_bytes!("../../assets/fonts/SourceSerif4-Variable.ttf"),
    include_bytes!("../../assets/fonts/IBMPlexMono-Regular.ttf"),
    include_bytes!("../../assets/fonts/IBMPlexMono-Medium.ttf"),
    include_bytes!("../../assets/fonts/IBMPlexMono-SemiBold.ttf"),
];

/// A rasterised glyph: coverage bitmap plus placement relative to the pen position / baseline.
pub struct Glyph {
    pub xmin: i32,
    pub ymin: i32,
    pub w: usize,
    pub h: usize,
    pub advance: f32,
    pub bitmap: Vec<u8>,
}

/// How to draw a run of text.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Style {
    pub face: Face,
    pub size: f32,
    pub tracking: f32,
    pub upper: bool,
    pub color: Rgb,
}

impl Style {
    pub const fn new(face: Face, size: f32) -> Self {
        Style { face, size, tracking: 0.0, upper: false, color: [0, 0, 0] }
    }
    /// Display: condensed black caps, tracking 0.3.
    pub const fn display(size: f32) -> Self {
        Style { face: Face::Display, size, tracking: sizes::DISPLAY_TRACKING, upper: true, color: [0, 0, 0] }
    }
    pub const fn interface(size: f32) -> Self {
        Style::new(Face::Interface, size)
    }
    pub const fn interface_bold(size: f32) -> Self {
        Style::new(Face::InterfaceBold, size)
    }
    /// Label: Archivo Bold 10.5, uppercase, tracking 1.8 (colour fg2 is the caller's job).
    pub const fn label() -> Self {
        Style { face: Face::InterfaceBold, size: sizes::LABEL, tracking: sizes::LABEL_TRACKING, upper: true, color: [0, 0, 0] }
    }
    pub const fn reading(size: f32) -> Self {
        Style::new(Face::Reading, size)
    }
    pub const fn data(size: f32) -> Self {
        Style::new(Face::Data, size)
    }
    pub const fn data_medium(size: f32) -> Self {
        Style::new(Face::DataMedium, size)
    }
    pub const fn data_semi(size: f32) -> Self {
        Style::new(Face::DataSemi, size)
    }
    pub const fn color(mut self, c: Rgb) -> Self {
        self.color = c;
        self
    }
    pub const fn tracking(mut self, t: f32) -> Self {
        self.tracking = t;
        self
    }
    pub const fn upper(mut self) -> Self {
        self.upper = true;
        self
    }
    pub const fn size(mut self, s: f32) -> Self {
        self.size = s;
        self
    }
}

pub struct Fonts {
    faces: Vec<Font>,
    cache: HashMap<(Face, u32, char), Rc<Glyph>>,
}

impl Default for Fonts {
    fn default() -> Self {
        Self::new()
    }
}

impl Fonts {
    pub fn new() -> Self {
        let faces = FACE_DATA
            .iter()
            .map(|d| Font::from_bytes(*d, FontSettings { scale: 40.0, ..FontSettings::default() }).expect("bundled font parses"))
            .collect();
        Fonts { faces, cache: HashMap::new() }
    }

    fn font(&self, face: Face) -> &Font {
        &self.faces[face as usize]
    }

    pub fn glyph(&mut self, face: Face, size: f32, ch: char) -> Rc<Glyph> {
        let key = (face, (size * 10.0).round() as u32, ch);
        if let Some(g) = self.cache.get(&key) {
            return g.clone();
        }
        let (m, bitmap) = self.font(face).rasterize(ch, size);
        let g = Rc::new(Glyph { xmin: m.xmin, ymin: m.ymin, w: m.width, h: m.height, advance: m.advance_width, bitmap });
        self.cache.insert(key, g.clone());
        g
    }

    pub fn kern(&self, face: Face, size: f32, a: char, b: char) -> f32 {
        self.font(face).horizontal_kern(a, b, size).unwrap_or(0.0)
    }

    /// `(ascent, descent, natural line height)` in px; descent is positive.
    pub fn line_metrics(&self, face: Face, size: f32) -> (f32, f32, f32) {
        match self.font(face).horizontal_line_metrics(size) {
            Some(m) => (m.ascent, -m.descent, m.new_line_size),
            None => (size * 0.8, size * 0.2, size * 1.2),
        }
    }

    /// Apply the style's case rule.
    pub fn cased(style: &Style, text: &str) -> String {
        if style.upper {
            text.to_uppercase()
        } else {
            text.to_string()
        }
    }

    /// Advance width of a run (tracking included between glyphs, not after the last).
    pub fn measure(&mut self, style: &Style, text: &str) -> f32 {
        let text = Self::cased(style, text);
        let mut w = 0.0;
        let mut prev: Option<char> = None;
        for ch in text.chars() {
            if let Some(p) = prev {
                w += self.kern(style.face, style.size, p, ch) + style.tracking;
            }
            w += self.glyph(style.face, style.size, ch).advance;
            prev = Some(ch);
        }
        w
    }

    /// Positions of each glyph (pen x) for drawing; mirrors `measure`.
    pub fn layout(&mut self, style: &Style, text: &str) -> Vec<(char, f32, Rc<Glyph>)> {
        let text = Self::cased(style, text);
        let mut out = Vec::with_capacity(text.len());
        let mut x = 0.0;
        let mut prev: Option<char> = None;
        for ch in text.chars() {
            if let Some(p) = prev {
                x += self.kern(style.face, style.size, p, ch) + style.tracking;
            }
            let g = self.glyph(style.face, style.size, ch);
            out.push((ch, x, g.clone()));
            x += g.advance;
            prev = Some(ch);
        }
        out
    }

    /// Truncate with `…` at the end so the run fits `max_w`.
    pub fn ellipsize(&mut self, style: &Style, text: &str, max_w: f32) -> String {
        if self.measure(style, text) <= max_w {
            return text.to_string();
        }
        let chars: Vec<char> = text.chars().collect();
        for n in (0..chars.len()).rev() {
            let s: String = chars[..n].iter().collect::<String>().trim_end().to_string() + "…";
            if self.measure(style, &s) <= max_w {
                return s;
            }
        }
        "…".into()
    }

    /// Truncate in the middle (`Kingdom He…(USA).zip`) so the run fits `max_w`.
    pub fn ellipsize_middle(&mut self, style: &Style, text: &str, max_w: f32) -> String {
        if self.measure(style, text) <= max_w {
            return text.to_string();
        }
        let chars: Vec<char> = text.chars().collect();
        let n = chars.len();
        for keep in (1..n).rev() {
            let head = keep.div_ceil(2);
            let tail = keep / 2;
            let s: String = chars[..head].iter().chain(std::iter::once(&'…')).chain(chars[n - tail..].iter()).collect();
            if self.measure(style, &s) <= max_w {
                return s;
            }
        }
        "…".into()
    }

    /// Greedy word wrap; a word longer than the width is split by character. At most `max_lines`
    /// lines; the last line is ellipsized if text remains.
    pub fn wrap(&mut self, style: &Style, text: &str, max_w: f32, max_lines: usize) -> Vec<String> {
        let max_lines = max_lines.max(1);
        let mut lines: Vec<String> = Vec::new();
        let mut cur = String::new();
        let mut words: std::collections::VecDeque<String> = text.split_whitespace().map(str::to_string).collect();
        while let Some(w) = words.pop_front() {
            let candidate = if cur.is_empty() { w.clone() } else { format!("{cur} {w}") };
            if self.measure(style, &candidate) <= max_w {
                cur = candidate;
                continue;
            }
            if cur.is_empty() {
                // a word longer than the line: take as many characters as fit, requeue the rest
                let mut piece = String::new();
                for ch in w.chars() {
                    let t = format!("{piece}{ch}");
                    if self.measure(style, &t) > max_w && !piece.is_empty() {
                        break;
                    }
                    piece = t;
                }
                let rest = w[piece.len()..].to_string();
                cur = piece;
                if !rest.is_empty() {
                    words.push_front(rest);
                }
            } else {
                words.push_front(w);
            }
            // the current line is full
            if lines.len() + 1 == max_lines {
                let remaining: Vec<String> = words.drain(..).collect();
                let joined = if remaining.is_empty() { cur.clone() } else { format!("{cur} {}", remaining.join(" ")) };
                lines.push(self.ellipsize(style, &joined, max_w));
                return lines;
            }
            lines.push(std::mem::take(&mut cur));
        }
        if !cur.is_empty() || lines.is_empty() {
            lines.push(cur);
        }
        lines
    }

    /// Cart titles: try each size in turn with `max_lines`; the first that fits without an
    /// ellipsis wins, else the smallest size ellipsized.
    pub fn fit_display(&mut self, base: Style, text: &str, max_w: f32, max_lines: usize, sizes: &[f32]) -> (f32, Vec<String>) {
        for &s in sizes {
            let st = base.size(s);
            let lines = self.wrap(&st, text, max_w, max_lines);
            if !lines.iter().any(|l| l.ends_with('…')) {
                return (s, lines);
            }
        }
        let s = *sizes.last().unwrap_or(&base.size);
        (s, self.wrap(&base.size(s), text, max_w, max_lines))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn faces_load_and_measure() {
        let mut f = Fonts::new();
        let st = Style::data(12.5);
        let w1 = f.measure(&st, "FR 027 400");
        let w2 = f.measure(&st, "FR 027 4000");
        assert!(w1 > 0.0 && w2 > w1);
        // mono: every glyph the same advance
        let a = f.glyph(Face::Data, 12.5, 'i').advance;
        let b = f.glyph(Face::Data, 12.5, 'W').advance;
        assert!((a - b).abs() < 0.01);
        let (asc, desc, lh) = f.line_metrics(Face::Display, 40.0);
        assert!(asc > 20.0 && desc > 0.0 && lh > asc);
    }

    #[test]
    fn display_is_uppercased_and_tracked() {
        let mut f = Fonts::new();
        let st = Style::display(34.0);
        assert_eq!(Fonts::cased(&st, "Kingdom Hearts"), "KINGDOM HEARTS");
        let plain = f.measure(&Style::new(Face::Display, 34.0).upper(), "KINGDOM");
        let tracked = f.measure(&st, "KINGDOM");
        assert!((tracked - plain - 6.0 * sizes::DISPLAY_TRACKING).abs() < 0.01);
    }

    #[test]
    fn ellipsis_fits() {
        let mut f = Fonts::new();
        let st = Style::interface(13.0);
        let full = "Kingdom Hearts - Chain of Memories (USA).zip";
        let w = f.measure(&st, full);
        let e = f.ellipsize(&st, full, w * 0.5);
        assert!(e.ends_with('…') && f.measure(&st, &e) <= w * 0.5);
        let m = f.ellipsize_middle(&st, full, w * 0.5);
        assert!(m.contains('…') && !m.ends_with('…') && m.starts_with("Kingdom") && m.ends_with(".zip"));
        assert!(f.measure(&st, &m) <= w * 0.5);
        assert_eq!(f.ellipsize(&st, "short", 1000.0), "short");
    }

    #[test]
    fn wrap_and_fit() {
        let mut f = Fonts::new();
        let st = Style::display(13.0);
        let lines = f.wrap(&st, "The Legend of Zelda", 60.0, 2);
        assert_eq!(lines.len(), 2);
        assert!(lines[1].ends_with('…') || f.measure(&st, &lines[1]) <= 60.0);
        let (size, lines) = f.fit_display(st, "A Link to the Past", 70.0, 2, &[13.0, 12.0, 11.0]);
        assert!(size <= 13.0 && lines.len() <= 2);
        let (size, lines) = f.fit_display(st, "Crash Bandicoot", 40.0, 2, &[13.0, 12.0, 11.0]);
        assert_eq!(size, 11.0);
        assert!(lines.iter().any(|l| l.ends_with('…')));
        let (_, lines) = f.fit_display(st, "FF Tactics", 200.0, 2, &[13.0]);
        assert_eq!(lines, vec!["FF Tactics"]);
    }
}
