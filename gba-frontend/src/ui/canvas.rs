//! Software canvas: a tiny-skia pixmap for vector shapes, direct pixel writes for text and
//! framebuffer blits, and a conversion to the 0RGB buffer minifb wants.

use super::drawings::{Drawing, Geometry, Seg};
use super::text::{Fonts, Style};
use super::tokens::Rgb;
use tiny_skia::{
    Color, FillRule, LineCap, Mask, Paint, Path, PathBuilder, Pixmap, PixmapPaint, Rect, Stroke, StrokeDash, Transform,
};

/// Integer clip rectangle (x, y, w, h).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Clip {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

pub struct Canvas {
    pub pix: Pixmap,
    pub fonts: Fonts,
    clip: Option<Clip>,
    mask: Option<Mask>,
}

fn color(c: Rgb) -> Color {
    Color::from_rgba8(c[0], c[1], c[2], 255)
}

fn paint(c: Rgb) -> Paint<'static> {
    let mut p = Paint::default();
    p.set_color(color(c));
    p.anti_alias = true;
    p
}

/// Rounded rectangle with per-corner radii (top-left, top-right, bottom-right, bottom-left).
pub fn round_rect_path(x: f32, y: f32, w: f32, h: f32, r: [f32; 4]) -> Option<Path> {
    const K: f32 = 0.5523;
    let lim = (w / 2.0).min(h / 2.0);
    let r = r.map(|v| v.max(0.0).min(lim));
    let mut pb = PathBuilder::new();
    pb.move_to(x + r[0], y);
    pb.line_to(x + w - r[1], y);
    if r[1] > 0.0 {
        pb.cubic_to(x + w - r[1] + r[1] * K, y, x + w, y + r[1] - r[1] * K, x + w, y + r[1]);
    }
    pb.line_to(x + w, y + h - r[2]);
    if r[2] > 0.0 {
        pb.cubic_to(x + w, y + h - r[2] + r[2] * K, x + w - r[2] + r[2] * K, y + h, x + w - r[2], y + h);
    }
    pb.line_to(x + r[3], y + h);
    if r[3] > 0.0 {
        pb.cubic_to(x + r[3] - r[3] * K, y + h, x, y + h - r[3] + r[3] * K, x, y + h - r[3]);
    }
    pb.line_to(x, y + r[0]);
    if r[0] > 0.0 {
        pb.cubic_to(x, y + r[0] - r[0] * K, x + r[0] - r[0] * K, y, x + r[0], y);
    }
    pb.close();
    pb.finish()
}

pub fn segs_to_path(segs: &[Seg]) -> Option<Path> {
    let mut pb = PathBuilder::new();
    for s in segs {
        match *s {
            Seg::Move(x, y) => pb.move_to(x, y),
            Seg::Line(x, y) => pb.line_to(x, y),
            Seg::Cubic(a, b, c, d, e, f) => pb.cubic_to(a, b, c, d, e, f),
            Seg::Close => pb.close(),
        }
    }
    pb.finish()
}

impl Canvas {
    pub fn new(w: u32, h: u32, fonts: Fonts) -> Self {
        Canvas { pix: Pixmap::new(w.max(1), h.max(1)).expect("pixmap"), fonts, clip: None, mask: None }
    }

    pub fn resize(&mut self, w: u32, h: u32) {
        if self.pix.width() != w.max(1) || self.pix.height() != h.max(1) {
            self.pix = Pixmap::new(w.max(1), h.max(1)).expect("pixmap");
            self.clip = None;
            self.mask = None;
        }
    }

    pub fn width(&self) -> f32 {
        self.pix.width() as f32
    }

    pub fn height(&self) -> f32 {
        self.pix.height() as f32
    }

    // --- clipping --------------------------------------------------------------------------

    /// Restrict every following draw to `clip` (None lifts it).
    pub fn set_clip(&mut self, clip: Option<Clip>) {
        self.clip = clip;
        self.mask = clip.and_then(|c| {
            let mut m = Mask::new(self.pix.width(), self.pix.height())?;
            let r = Rect::from_xywh(c.x as f32, c.y as f32, c.w as f32, c.h as f32)?;
            m.fill_path(&PathBuilder::from_rect(r), FillRule::Winding, false, Transform::identity());
            Some(m)
        });
    }

    pub fn clip(&self) -> Option<Clip> {
        self.clip
    }

    fn clip_bounds(&self) -> (i32, i32, i32, i32) {
        match self.clip {
            Some(c) => (c.x.max(0), c.y.max(0), (c.x + c.w).min(self.pix.width() as i32), (c.y + c.h).min(self.pix.height() as i32)),
            None => (0, 0, self.pix.width() as i32, self.pix.height() as i32),
        }
    }

    // --- fills and strokes -----------------------------------------------------------------

    pub fn clear(&mut self, c: Rgb) {
        self.pix.fill(color(c));
    }

    /// Axis-aligned fill, clipped to the canvas first (tiny-skia's anti-aliased rect fill has
    /// a debug assertion that fires on rectangles starting off-canvas).
    pub fn fill_rect(&mut self, x: f32, y: f32, w: f32, h: f32, c: Rgb) {
        let mut x0 = x.max(0.0);
        let mut y0 = y.max(0.0);
        let mut x1 = (x + w).min(self.width());
        let mut y1 = (y + h).min(self.height());
        if x1 <= x0 || y1 <= y0 {
            return;
        }
        // A span whose two edges are both fractional with no whole pixel between them trips
        // the same assertion; snap such thin spans to whole pixels.
        if x0.ceil() >= x1.floor() && (x0.fract() != 0.0 || x1.fract() != 0.0) {
            x0 = x0.round();
            x1 = x1.round().max(x0 + 1.0);
        }
        if y0.ceil() >= y1.floor() && (y0.fract() != 0.0 || y1.fract() != 0.0) {
            y0 = y0.round();
            y1 = y1.round().max(y0 + 1.0);
        }
        if let Some(r) = Rect::from_ltrb(x0, y0, x1, y1) {
            self.pix.fill_rect(r, &paint(c), Transform::identity(), self.mask.as_ref());
        }
    }

    pub fn fill_path(&mut self, path: &Path, c: Rgb, t: Transform) {
        self.pix.fill_path(path, &paint(c), FillRule::Winding, t, self.mask.as_ref());
    }

    /// tiny-skia treats widths <= 1.0 as hairlines, whose rasteriser carries a debug assertion
    /// that fires on partially clipped geometry; every stroke here stays just above that.
    const MIN_STROKE: f32 = 1.05;

    pub fn stroke_path(&mut self, path: &Path, width: f32, c: Rgb, round: bool, t: Transform) {
        let mut st = Stroke { width: width.max(Self::MIN_STROKE), ..Stroke::default() };
        if round {
            st.line_cap = LineCap::Round;
        }
        self.pix.stroke_path(path, &paint(c), &st, t, self.mask.as_ref());
    }

    pub fn stroke_rect(&mut self, x: f32, y: f32, w: f32, h: f32, width: f32, c: Rgb) {
        if let Some(r) = Rect::from_xywh(x, y, w, h) {
            self.stroke_path(&PathBuilder::from_rect(r), width, c, false, Transform::identity());
        }
    }

    pub fn dashed_rect(&mut self, x: f32, y: f32, w: f32, h: f32, width: f32, dash: f32, c: Rgb) {
        let Some(r) = Rect::from_xywh(x, y, w, h) else { return };
        let st = Stroke { width: width.max(Self::MIN_STROKE), dash: StrokeDash::new(vec![dash, dash], 0.0), ..Stroke::default() };
        self.pix.stroke_path(&PathBuilder::from_rect(r), &paint(c), &st, Transform::identity(), self.mask.as_ref());
    }

    pub fn round_rect(&mut self, x: f32, y: f32, w: f32, h: f32, r: [f32; 4], c: Rgb) {
        if let Some(p) = round_rect_path(x, y, w, h, r) {
            self.fill_path(&p, c, Transform::identity());
        }
    }

    pub fn stroke_round_rect(&mut self, x: f32, y: f32, w: f32, h: f32, r: [f32; 4], width: f32, c: Rgb) {
        if let Some(p) = round_rect_path(x, y, w, h, r) {
            self.stroke_path(&p, width, c, false, Transform::identity());
        }
    }

    /// Horizontal rule `width` thick whose top edge is at `y`.
    pub fn hrule(&mut self, x0: f32, x1: f32, y: f32, width: f32, c: Rgb) {
        self.fill_rect(x0, y, x1 - x0, width, c);
    }

    pub fn vrule(&mut self, x: f32, y0: f32, y1: f32, width: f32, c: Rgb) {
        self.fill_rect(x, y0, width, y1 - y0, c);
    }

    pub fn line(&mut self, x0: f32, y0: f32, x1: f32, y1: f32, width: f32, c: Rgb) {
        let mut pb = PathBuilder::new();
        pb.move_to(x0, y0);
        pb.line_to(x1, y1);
        if let Some(p) = pb.finish() {
            self.stroke_path(&p, width, c, false, Transform::identity());
        }
    }

    /// Dotted leader (used by the settings "Emulator keys" list).
    pub fn dotted_line(&mut self, x0: f32, x1: f32, y: f32, c: Rgb) {
        let mut pb = PathBuilder::new();
        pb.move_to(x0, y);
        pb.line_to(x1, y);
        let Some(p) = pb.finish() else { return };
        let st = Stroke { width: Self::MIN_STROKE, dash: StrokeDash::new(vec![1.0, 3.0], 0.0), ..Stroke::default() };
        self.pix.stroke_path(&p, &paint(c), &st, Transform::identity(), self.mask.as_ref());
    }

    pub fn circle(&mut self, cx: f32, cy: f32, r: f32, c: Rgb) {
        if let Some(p) = PathBuilder::from_circle(cx, cy, r) {
            self.fill_path(&p, c, Transform::identity());
        }
    }

    pub fn stroke_circle(&mut self, cx: f32, cy: f32, r: f32, width: f32, c: Rgb) {
        if let Some(p) = PathBuilder::from_circle(cx, cy, r) {
            self.stroke_path(&p, width, c, false, Transform::identity());
        }
    }

    /// Draw a parsed SVG drawing with its origin at (x, y). `stroke_override` recolours every
    /// stroke that is the drawing's default ink (callout dots keep their own colour).
    pub fn drawing(&mut self, d: &Drawing, x: f32, y: f32) {
        let t = Transform::from_translate(x, y);
        for s in &d.shapes {
            let path = match &s.geom {
                Geometry::Rect { x, y, w, h, rx } => round_rect_path(*x, *y, *w, *h, [*rx; 4]),
                Geometry::Circle { cx, cy, r } => PathBuilder::from_circle(*cx, *cy, *r),
                Geometry::Path(segs) => segs_to_path(segs),
            };
            let Some(path) = path else { continue };
            if let Some(f) = s.fill {
                self.fill_path(&path, f, t);
            }
            if let Some(st) = s.stroke {
                self.stroke_path(&path, s.stroke_width, st, s.round_cap, t);
            }
        }
    }

    /// Draw another canvas' pixmap through `t` (rotation for film strips); bilinear when rotated.
    pub fn draw_pixmap(&mut self, other: &Pixmap, x: f32, y: f32, t: Transform) {
        let pp = PixmapPaint { quality: tiny_skia::FilterQuality::Bilinear, ..PixmapPaint::default() };
        self.pix.draw_pixmap(x as i32, y as i32, other.as_ref(), &pp, t, self.mask.as_ref());
    }

    // --- direct pixel writes ---------------------------------------------------------------

    #[inline]
    fn blend_px(data: &mut [u8], idx: usize, c: Rgb, cov: u8) {
        if cov == 0 {
            return;
        }
        let p = &mut data[idx * 4..idx * 4 + 4];
        if cov == 255 {
            p[0] = c[0];
            p[1] = c[1];
            p[2] = c[2];
            p[3] = 255;
            return;
        }
        let a = cov as u32;
        for i in 0..3 {
            p[i] = ((p[i] as u32 * (255 - a) + c[i] as u32 * a) / 255) as u8;
        }
        p[3] = 255;
    }

    /// Nearest-neighbour blit of a BGR555 picture at an integer scale; bit 15 set = skip (transparent).
    pub fn blit_bgr555(&mut self, fb: &[u16], fw: usize, fh: usize, x: i32, y: i32, scale: usize) {
        let (cx0, cy0, cx1, cy1) = self.clip_bounds();
        let stride = self.pix.width() as usize;
        let data = self.pix.data_mut();
        for sy in 0..fh {
            for r in 0..scale {
                let dy = y + (sy * scale + r) as i32;
                if dy < cy0 || dy >= cy1 {
                    continue;
                }
                let row = &fb[sy * fw..(sy + 1) * fw];
                for (sx, &p) in row.iter().enumerate() {
                    if p & 0x8000 != 0 {
                        continue;
                    }
                    let [cr, cg, cb] = gba_test::bgr555_to_rgb(p);
                    for s in 0..scale {
                        let dx = x + (sx * scale + s) as i32;
                        if dx < cx0 || dx >= cx1 {
                            continue;
                        }
                        let idx = (dy as usize * stride + dx as usize) * 4;
                        data[idx] = cr;
                        data[idx + 1] = cg;
                        data[idx + 2] = cb;
                        data[idx + 3] = 255;
                    }
                }
            }
        }
    }

    /// Blit packed RGB8 pixels 1:1 (decoded PNG art).
    pub fn blit_rgb(&mut self, rgb: &[u8], w: usize, h: usize, x: i32, y: i32) {
        let (cx0, cy0, cx1, cy1) = self.clip_bounds();
        let stride = self.pix.width() as usize;
        let data = self.pix.data_mut();
        for sy in 0..h {
            let dy = y + sy as i32;
            if dy < cy0 || dy >= cy1 {
                continue;
            }
            for sx in 0..w {
                let dx = x + sx as i32;
                if dx < cx0 || dx >= cx1 {
                    continue;
                }
                let s = (sy * w + sx) * 3;
                let idx = (dy as usize * stride + dx as usize) * 4;
                data[idx..idx + 3].copy_from_slice(&rgb[s..s + 3]);
                data[idx + 3] = 255;
            }
        }
    }

    /// Blit a BGR555 picture scaled, but only inside the circle (cx, cy, r): the loupe.
    pub fn blit_bgr555_in_circle(&mut self, fb: &[u16], fw: usize, fh: usize, x: i32, y: i32, scale: usize, cx: f32, cy: f32, r: f32) {
        let (cx0, cy0, cx1, cy1) = self.clip_bounds();
        let stride = self.pix.width() as usize;
        let data = self.pix.data_mut();
        let y0 = ((cy - r).floor() as i32).max(cy0);
        let y1 = ((cy + r).ceil() as i32).min(cy1);
        for dy in y0..y1 {
            let sy = (dy - y).div_euclid(scale as i32);
            if sy < 0 || sy >= fh as i32 {
                continue;
            }
            let x0 = ((cx - r).floor() as i32).max(cx0);
            let x1 = ((cx + r).ceil() as i32).min(cx1);
            for dx in x0..x1 {
                let ddx = dx as f32 + 0.5 - cx;
                let ddy = dy as f32 + 0.5 - cy;
                let d = (ddx * ddx + ddy * ddy).sqrt();
                if d > r {
                    continue;
                }
                let sx = (dx - x).div_euclid(scale as i32);
                if sx < 0 || sx >= fw as i32 {
                    continue;
                }
                let p = fb[sy as usize * fw + sx as usize];
                let [cr, cg, cb] = gba_test::bgr555_to_rgb(p);
                let cov = ((r - d).clamp(0.0, 1.0) * 255.0) as u8;
                Self::blend_px(data, dy as usize * stride + dx as usize, [cr, cg, cb], cov);
            }
        }
    }

    // --- text --------------------------------------------------------------------------------

    pub fn measure(&mut self, text: &str, style: &Style) -> f32 {
        self.fonts.measure(style, text)
    }

    /// Draw text with its top at `y` (the line box starts at y; baseline = y + ascent).
    /// Returns the advance width.
    pub fn text(&mut self, x: f32, y: f32, text: &str, style: &Style) -> f32 {
        let (ascent, _, _) = self.fonts.line_metrics(style.face, style.size);
        self.text_baseline(x, y + ascent, text, style)
    }

    /// Draw text on a baseline. Returns the advance width.
    pub fn text_baseline(&mut self, x: f32, baseline: f32, text: &str, style: &Style) -> f32 {
        let run = self.fonts.layout(style, text);
        let (cx0, cy0, cx1, cy1) = self.clip_bounds();
        let stride = self.pix.width() as usize;
        let base = baseline.round() as i32;
        let mut width = 0.0;
        let data = self.pix.data_mut();
        for (_, pen, g) in &run {
            width = pen + g.advance;
            if g.w == 0 || g.h == 0 {
                continue;
            }
            let gx = (x + pen).round() as i32 + g.xmin;
            let gy = base - g.ymin - g.h as i32;
            for row in 0..g.h {
                let dy = gy + row as i32;
                if dy < cy0 || dy >= cy1 {
                    continue;
                }
                for col in 0..g.w {
                    let dx = gx + col as i32;
                    if dx < cx0 || dx >= cx1 {
                        continue;
                    }
                    let cov = g.bitmap[row * g.w + col];
                    Self::blend_px(data, dy as usize * stride + dx as usize, style.color, cov);
                }
            }
        }
        width
    }

    pub fn text_right(&mut self, x_right: f32, y: f32, text: &str, style: &Style) -> f32 {
        let w = self.measure(text, style);
        self.text(x_right - w, y, text, style)
    }

    pub fn text_center(&mut self, cx: f32, y: f32, text: &str, style: &Style) -> f32 {
        let w = self.measure(text, style);
        self.text(cx - w / 2.0, y, text, style)
    }

    /// Wrapped paragraph; returns the height used.
    pub fn paragraph(&mut self, x: f32, y: f32, max_w: f32, text: &str, style: &Style, line_height: f32, max_lines: usize) -> f32 {
        let lines = self.fonts.wrap(style, text, max_w, max_lines);
        let lh = style.size * line_height;
        for (i, l) in lines.iter().enumerate() {
            self.text(x, y + i as f32 * lh, l, style);
        }
        lines.len() as f32 * lh
    }

    /// Underline for the destructive link style: 1 px, 2 px under the baseline.
    pub fn underline(&mut self, x: f32, baseline: f32, width: f32, c: Rgb) {
        self.fill_rect(x, baseline + 2.0, width, 1.0, c);
    }

    // --- output ------------------------------------------------------------------------------

    /// Convert the opaque RGBA pixmap to minifb's 0RGB.
    pub fn to_0rgb(&self, out: &mut Vec<u32>) {
        let data = self.pix.data();
        out.clear();
        out.reserve(data.len() / 4);
        for p in data.chunks_exact(4) {
            out.push((p[0] as u32) << 16 | (p[1] as u32) << 8 | p[2] as u32);
        }
    }

    /// Read one pixel back (tests and probes).
    pub fn pixel(&self, x: u32, y: u32) -> Rgb {
        let i = (y * self.pix.width() + x) as usize * 4;
        let d = self.pix.data();
        [d[i], d[i + 1], d[i + 2]]
    }

    /// Write the canvas as a PNG (render tests).
    pub fn save_png(&self, path: &std::path::Path) -> anyhow::Result<()> {
        self.pix.save_png(path).map_err(|e| anyhow::anyhow!("{e}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::tokens::{NIGHT, PAPER};

    #[test]
    fn fills_and_clip() {
        let mut c = Canvas::new(64, 64, Fonts::new());
        c.clear(PAPER.bg);
        assert_eq!(c.pixel(1, 1), PAPER.bg);
        c.fill_rect(10.0, 10.0, 20.0, 20.0, PAPER.fg);
        assert_eq!(c.pixel(15, 15), PAPER.fg);
        assert_eq!(c.pixel(9, 9), PAPER.bg);
        c.set_clip(Some(Clip { x: 0, y: 0, w: 32, h: 64 }));
        c.fill_rect(0.0, 40.0, 64.0, 10.0, NIGHT.spot);
        assert_eq!(c.pixel(10, 45), NIGHT.spot);
        assert_eq!(c.pixel(50, 45), PAPER.bg, "clipped");
        c.set_clip(None);
        let mut out = Vec::new();
        c.to_0rgb(&mut out);
        assert_eq!(out.len(), 64 * 64);
        assert_eq!(out[45 * 64 + 10], 0xE5483C);
    }

    #[test]
    fn thin_and_offscreen_fills_do_not_panic() {
        let mut c = Canvas::new(64, 64, Fonts::new());
        c.clear(PAPER.bg);
        c.fill_rect(10.25, 10.25, 1.5, 20.0, PAPER.fg); // two fractional edges, no inner pixel
        c.fill_rect(-5.0, -5.0, 10.0, 10.0, PAPER.fg); // starts off-canvas
        c.fill_rect(60.0, 60.0, 10.0, 10.0, PAPER.fg); // ends off-canvas
        c.fill_rect(70.0, 70.0, 10.0, 10.0, PAPER.fg); // fully off-canvas
        c.fill_rect(5.0, 30.5, 30.0, 1.0, PAPER.fg); // horizontal hairline at a half pixel
        assert_eq!(c.pixel(10, 15), PAPER.fg);
        assert_eq!(c.pixel(2, 2), PAPER.fg);
        assert_eq!(c.pixel(62, 62), PAPER.fg);
    }

    #[test]
    fn text_marks_pixels_and_respects_clip() {
        let mut c = Canvas::new(200, 60, Fonts::new());
        c.clear(PAPER.bg);
        let st = Style::display(40.0).color(PAPER.fg);
        let w = c.text(4.0, 4.0, "LIBRARY", &st);
        assert!(w > 60.0);
        let dark = (0..200).flat_map(|x| (0..60).map(move |y| (x, y))).filter(|&(x, y)| c.pixel(x, y) == PAPER.fg).count();
        assert!(dark > 200, "glyph cores are solid ink: {dark}");
        let mut c2 = Canvas::new(200, 60, Fonts::new());
        c2.clear(PAPER.bg);
        c2.set_clip(Some(Clip { x: 0, y: 0, w: 10, h: 60 }));
        c2.text(4.0, 4.0, "LIBRARY", &st);
        for x in 10..200 {
            for y in 0..60 {
                assert_eq!(c2.pixel(x, y), PAPER.bg);
            }
        }
    }

    #[test]
    fn bgr555_blit_scales_and_skips_transparent() {
        let mut c = Canvas::new(20, 20, Fonts::new());
        c.clear(PAPER.bg);
        let fb = [0x001Fu16, 0x8000, 0x7C00, 0x03E0];
        c.blit_bgr555(&fb, 2, 2, 2, 2, 3);
        assert_eq!(c.pixel(2, 2), [255, 0, 0]);
        assert_eq!(c.pixel(4, 4), [255, 0, 0]);
        assert_eq!(c.pixel(5, 2), PAPER.bg, "transparent texel skipped");
        assert_eq!(c.pixel(2, 5), [0, 0, 255]);
        assert_eq!(c.pixel(7, 7), [0, 255, 0]);
    }

    #[test]
    fn loupe_blit_stays_inside_the_circle() {
        let mut c = Canvas::new(40, 40, Fonts::new());
        c.clear(PAPER.bg);
        let fb = vec![0x001Fu16; 4];
        c.blit_bgr555_in_circle(&fb, 2, 2, 0, 0, 20, 20.0, 20.0, 10.0);
        assert_eq!(c.pixel(20, 20), [255, 0, 0]);
        assert_eq!(c.pixel(2, 2), PAPER.bg);
        assert_eq!(c.pixel(20, 2), PAPER.bg);
    }

    #[test]
    fn drawings_render_without_panicking() {
        let mut c = Canvas::new(520, 340, Fonts::new());
        c.clear(PAPER.bg);
        for svg in [super::super::drawings::HANDHELD_CONTROLS, super::super::drawings::HANDHELD_LIBRARY, super::super::drawings::PENCIL_RING] {
            let d = super::super::drawings::parse(svg);
            c.drawing(&d, 0.0, 0.0);
        }
        let ink = (0..520).flat_map(|x| (0..340).map(move |y| (x, y))).filter(|&(x, y)| c.pixel(x, y) != PAPER.bg).count();
        assert!(ink > 1000);
    }
}
