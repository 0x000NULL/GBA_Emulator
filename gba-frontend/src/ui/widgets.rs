//! Shared components from `docs/gui/DESIGN.md`: keycap, tick box, buttons, the head and foot
//! frame of every window-resolution screen, tabs and the numbered settings item.

use super::canvas::Canvas;
use super::text::Style;
use super::tokens::{component as c, font as f, layout as l, Palette, Rgb};

/// Keycap drawn with its top-left at (x, y). Returns its width.
pub fn keycap(cv: &mut Canvas, x: f32, y: f32, label: &str, pal: &Palette, active: bool) -> f32 {
    let st = Style::data_semi(f::DATA_KEY).color(if active { pal.spot } else { pal.fg });
    let tw = cv.measure(label, &st);
    let w = (tw + 2.0 * c::KEY_PAD_X).max(c::KEY_MIN_W).round();
    let border = if active { pal.spot } else { pal.fg };
    // the cap: border 1.5 all round, 3 at the bottom
    cv.round_rect(x, y, w, c::KEY_H, [c::KEY_RADIUS; 4], border);
    cv.round_rect(
        x + c::KEY_BORDER,
        y + c::KEY_BORDER,
        w - 2.0 * c::KEY_BORDER,
        c::KEY_H - c::KEY_BORDER - c::KEY_BORDER_BOTTOM,
        [c::KEY_RADIUS - 1.0; 4],
        pal.bg,
    );
    let (asc, desc, _) = cv.fonts.line_metrics(st.face, st.size);
    let inner_h = c::KEY_H - c::KEY_BORDER - c::KEY_BORDER_BOTTOM;
    let baseline = y + c::KEY_BORDER + (inner_h + asc - desc) / 2.0;
    cv.text_baseline(x + (w - tw) / 2.0, baseline, label, &st);
    w
}

/// Width a keycap would take (for right-aligned layouts).
pub fn keycap_width(cv: &mut Canvas, label: &str) -> f32 {
    let st = Style::data_semi(f::DATA_KEY);
    (cv.measure(label, &st) + 2.0 * c::KEY_PAD_X).max(c::KEY_MIN_W).round()
}

/// Keycap followed by its verb; returns the total width.
pub fn key_verb(cv: &mut Canvas, x: f32, y: f32, keys: &[&str], verb: &str, pal: &Palette) -> f32 {
    let mut cx = x;
    for (i, k) in keys.iter().enumerate() {
        if i > 0 {
            cx += 4.0;
        }
        cx += keycap(cv, cx, y, k, pal, false);
    }
    let st = Style::interface(f::INTERFACE_BODY).color(pal.fg);
    let (asc, desc, _) = cv.fonts.line_metrics(st.face, st.size);
    let baseline = y + (c::KEY_H + asc - desc) / 2.0;
    cx += 8.0;
    cx += cv.text_baseline(cx, baseline, verb, &st);
    cx - x
}

/// 14 px tick box; checked = fg fill with a 2 px bg inset. Returns the box size.
pub fn tick(cv: &mut Canvas, x: f32, y: f32, checked: bool, pal: &Palette) -> f32 {
    cv.fill_rect(x, y, c::TICK_SIZE, c::TICK_SIZE, pal.fg);
    cv.fill_rect(
        x + c::TICK_BORDER,
        y + c::TICK_BORDER,
        c::TICK_SIZE - 2.0 * c::TICK_BORDER,
        c::TICK_SIZE - 2.0 * c::TICK_BORDER,
        pal.bg,
    );
    if checked {
        let i = c::TICK_BORDER + c::TICK_INSET;
        cv.fill_rect(x + i, y + i, c::TICK_SIZE - 2.0 * i, c::TICK_SIZE - 2.0 * i, pal.fg);
    }
    c::TICK_SIZE
}

/// Tick box with a label to its right; returns the total width.
pub fn tick_option(cv: &mut Canvas, x: f32, y: f32, label: &str, checked: bool, pal: &Palette, highlight: bool) -> f32 {
    tick(cv, x, y + 1.0, checked, pal);
    let st = Style::interface(f::INTERFACE_BODY).color(if highlight { pal.spot } else { pal.fg });
    let (asc, desc, _) = cv.fonts.line_metrics(st.face, st.size);
    let baseline = y + 1.0 + (c::TICK_SIZE + asc - desc) / 2.0;
    let w = cv.text_baseline(x + c::TICK_SIZE + 8.0, baseline, label, &st);
    c::TICK_SIZE + 8.0 + w
}

/// Primary button: fg fill, bg text, 44 px high, key in Data at 65 % opacity. Returns width.
pub fn button_primary(cv: &mut Canvas, x: f32, y: f32, w: f32, label: &str, key: Option<&str>, pal: &Palette) -> f32 {
    cv.fill_rect(x, y, w, c::BUTTON_H, pal.fg);
    let st = Style::interface_bold(f::INTERFACE_BUTTON).color(pal.bg);
    let lw = cv.measure(label, &st);
    let ks = Style::data(f::DATA_BODY).color(mix(pal.bg, pal.fg, 0.65));
    let kw = key.map(|k| cv.measure(k, &ks) + 10.0).unwrap_or(0.0);
    let (asc, desc, _) = cv.fonts.line_metrics(st.face, st.size);
    let baseline = y + (c::BUTTON_H + asc - desc) / 2.0;
    let x0 = x + (w - lw - kw) / 2.0;
    cv.text_baseline(x0, baseline, label, &st);
    if let Some(k) = key {
        cv.text_baseline(x0 + lw + 10.0, baseline, k, &ks);
    }
    w
}

/// Secondary button: 1.5 px fg outline. Returns width.
pub fn button_secondary(cv: &mut Canvas, x: f32, y: f32, w: f32, label: &str, pal: &Palette) -> f32 {
    cv.fill_rect(x, y, w, c::BUTTON_H, pal.fg);
    cv.fill_rect(x + c::BUTTON_OUTLINE, y + c::BUTTON_OUTLINE, w - 2.0 * c::BUTTON_OUTLINE, c::BUTTON_H - 2.0 * c::BUTTON_OUTLINE, pal.bg);
    let st = Style::interface_bold(f::INTERFACE_BUTTON).color(pal.fg);
    let lw = cv.measure(label, &st);
    let (asc, desc, _) = cv.fonts.line_metrics(st.face, st.size);
    let baseline = y + (c::BUTTON_H + asc - desc) / 2.0;
    cv.text_baseline(x + (w - lw) / 2.0, baseline, label, &st);
    w
}

/// Natural width of a button for its label (+ key).
pub fn button_width(cv: &mut Canvas, label: &str, key: Option<&str>) -> f32 {
    let st = Style::interface_bold(f::INTERFACE_BUTTON);
    let ks = Style::data(f::DATA_BODY);
    let kw = key.map(|k| cv.measure(k, &ks) + 10.0).unwrap_or(0.0);
    (cv.measure(label, &st) + kw + 2.0 * c::BUTTON_PAD_X).round()
}

/// Destructive action: spot text, underlined, no box. Returns width.
pub fn destructive_link(cv: &mut Canvas, x: f32, y: f32, label: &str, pal: &Palette) -> f32 {
    let st = Style::interface_bold(f::INTERFACE_BODY).color(pal.spot);
    let (asc, _, _) = cv.fonts.line_metrics(st.face, st.size);
    let w = cv.text(x, y, label, &st);
    cv.underline(x, y + asc, w, pal.spot);
    w
}

/// Mix `a` towards `b` by `t` (0 = a, 1 = b).
pub fn mix(a: Rgb, b: Rgb, t: f32) -> Rgb {
    let f = |i: usize| (a[i] as f32 + (b[i] as f32 - a[i] as f32) * t).round() as u8;
    [f(0), f(1), f(2)]
}

/// Head: section number in spot, title, optional context (Label style), right-aligned context.
/// Returns the x where free space begins after the title/context.
pub fn head(cv: &mut Canvas, number: &str, title: &str, context: Option<&str>, right: Option<(&str, bool)>, pal: &Palette) -> f32 {
    let w = cv.width();
    cv.fill_rect(0.0, 0.0, w, l::HEAD_H, pal.bg);
    cv.hrule(l::MARGIN, w - l::MARGIN, l::HEAD_H - l::HEAD_RULE, l::HEAD_RULE, pal.fg);
    let num = Style::display(f::DISPLAY_HEAD_NUMBER).color(pal.spot);
    let ttl = Style::display(f::DISPLAY_HEAD_TITLE).color(pal.fg);
    let (asc_n, _, _) = cv.fonts.line_metrics(num.face, num.size);
    let baseline = (l::HEAD_H - l::HEAD_RULE) / 2.0 + asc_n * 0.36;
    let mut x = l::MARGIN;
    x += cv.text_baseline(x, baseline, number, &num) + l::HEAD_GAP;
    x += cv.text_baseline(x, baseline, title, &ttl) + l::HEAD_GAP + 4.0;
    if let Some(ctx) = context {
        let st = Style::label().color(pal.fg2);
        let max_w = w - l::MARGIN - x - 260.0;
        let ctx = cv.fonts.ellipsize_middle(&st, ctx, max_w.max(80.0));
        x += cv.text_baseline(x, baseline - 1.0, &ctx, &st) + l::HEAD_GAP;
    }
    if let Some((txt, data)) = right {
        let st = if data { Style::data(f::DATA_SMALL + 0.5).color(pal.fg2) } else { Style::label().color(pal.fg2) };
        cv.text_right(w - l::MARGIN, (l::HEAD_H - l::HEAD_RULE - st.size * 1.2) / 2.0 + 1.0, txt, &st);
    }
    x
}

/// Foot: keycaps + verb for every key that works here; Back/Quit alone at the right.
pub fn foot(cv: &mut Canvas, keys: &[(&[&str], &str)], back: Option<(&str, &str)>, pal: &Palette) {
    let w = cv.width();
    let h = cv.height();
    let y0 = h - l::FOOT_H;
    cv.fill_rect(0.0, y0, w, l::FOOT_H, pal.bg);
    cv.hrule(l::MARGIN, w - l::MARGIN, y0, l::FOOT_RULE, pal.line);
    let ky = y0 + (l::FOOT_H - c::KEY_H) / 2.0 + 1.0;
    let mut x = l::MARGIN;
    for (ks, verb) in keys {
        x += key_verb(cv, x, ky, ks, verb, pal) + l::FOOT_GAP;
    }
    if let Some((k, verb)) = back {
        let st = Style::interface(f::INTERFACE_BODY).color(pal.fg);
        let vw = cv.measure(verb, &st);
        let kw = keycap_width(cv, k);
        let x = w - l::MARGIN - vw - 8.0 - kw;
        key_verb(cv, x, ky, &[k], verb, pal);
    }
}

/// Head tabs (Settings): the active one underlined in spot. Returns the left x of the first tab.
pub fn tabs(cv: &mut Canvas, labels: &[&str], active: usize, pal: &Palette) -> f32 {
    let w = cv.width();
    let st_on = Style::interface_bold(f::INTERFACE_BODY).color(pal.fg);
    let st_off = Style::interface_bold(f::INTERFACE_BODY).color(pal.fg2);
    let gap = 24.0;
    let widths: Vec<f32> = labels.iter().map(|t| cv.measure(t, &st_on)).collect();
    let total: f32 = widths.iter().sum::<f32>() + gap * (labels.len().saturating_sub(1)) as f32;
    let mut x = w - l::MARGIN - total;
    let x0 = x;
    let y = (l::HEAD_H - l::HEAD_RULE - st_on.size * 1.2) / 2.0 + 1.0;
    for (i, t) in labels.iter().enumerate() {
        let st = if i == active { st_on } else { st_off };
        cv.text(x, y, t, &st);
        if i == active {
            cv.fill_rect(x, l::HEAD_H - l::HEAD_RULE, widths[i], 2.5, pal.spot);
        }
        x += widths[i] + gap;
    }
    x0
}

/// Field label in Label style (fg2). Returns width.
pub fn label(cv: &mut Canvas, x: f32, y: f32, text: &str, pal: &Palette) -> f32 {
    cv.text(x, y, text, &Style::label().color(pal.fg2))
}

/// A details row: label column at `x`, value at `x + col`, Data or Reading style.
pub fn detail_row(cv: &mut Canvas, x: f32, y: f32, col: f32, name: &str, value: &str, mono: bool, pal: &Palette) {
    label(cv, x, y + 2.0, name, pal);
    let st = if mono { Style::data(f::DATA_BODY).color(pal.fg) } else { Style::reading(f::READING_BODY).color(pal.fg) };
    cv.text(x + col, y, value, &st);
}

/// Numbered settings item: red numeral, title, one-line description. Returns the y below it.
pub fn numbered_item(cv: &mut Canvas, x: f32, y: f32, w: f32, n: usize, title: &str, desc: &str, pal: &Palette) -> f32 {
    let num = Style::interface_bold(f::INTERFACE_EMPHASIS).color(pal.spot);
    cv.text(x, y, &n.to_string(), &num);
    let tx = x + 34.0;
    let ts = Style::interface_bold(f::INTERFACE_BODY).color(pal.fg).upper().tracking(0.4);
    cv.text(tx, y + 1.0, title, &ts);
    let ds = Style::reading(f::READING_BODY).color(pal.fg2);
    let used = cv.paragraph(tx, y + 20.0, w - 34.0, desc, &ds, f::READING_LINE_HEIGHT, 2);
    y + 20.0 + used + 6.0
}

/// Ruler 0..10 with a red marker at `value`. Returns height.
pub fn ruler(cv: &mut Canvas, x: f32, y: f32, w: f32, value: u8, pal: &Palette) -> f32 {
    cv.hrule(x, x + w, y + 8.0, 1.5, pal.fg);
    let st = Style::data(f::DATA_SMALL - 2.0).color(pal.fg2);
    for i in 0..=10 {
        let tx = x + w * i as f32 / 10.0;
        let th = if i % 5 == 0 { 8.0 } else { 5.0 };
        cv.vrule(tx - 0.5, y + 8.0 - th, y + 8.0, 1.0, pal.fg);
        if i % 5 == 0 {
            cv.text_center(tx, y + 12.0, &i.to_string(), &st);
        }
    }
    let mx = x + w * value.min(10) as f32 / 10.0;
    let mut pb = tiny_skia::PathBuilder::new();
    pb.move_to(mx - 5.0, y - 3.0);
    pb.line_to(mx + 5.0, y - 3.0);
    pb.line_to(mx, y + 6.0);
    pb.close();
    if let Some(p) = pb.finish() {
        cv.fill_path(&p, pal.spot, tiny_skia::Transform::identity());
    }
    26.0
}

/// Simple underline text field (Find, folder paths). Returns the field's width.
pub fn underline_field(cv: &mut Canvas, x: f32, y: f32, w: f32, label_text: &str, value: &str, placeholder: &str, active: bool, pal: &Palette) -> f32 {
    let lw = label(cv, x, y + 3.0, label_text, pal) + 12.0;
    let st = Style::interface(f::INTERFACE_BODY + 1.0).color(if value.is_empty() { pal.fg2 } else { pal.fg });
    let shown = if value.is_empty() { placeholder.to_string() } else { value.to_string() };
    let max_w = w - lw;
    let shown = cv.fonts.ellipsize_middle(&st, &shown, max_w);
    let tw = cv.text(x + lw, y, &shown, &st);
    if active && !value.is_empty() {
        cv.fill_rect(x + lw + tw + 1.0, y + 1.0, 1.5, st.size + 2.0, pal.spot);
    }
    cv.hrule(x + lw, x + w, y + st.size + 6.0, 1.0, if active { pal.fg } else { pal.line });
    w
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::text::Fonts;
    use crate::ui::tokens::{NIGHT, PAPER};

    #[test]
    fn keycap_size_and_colours() {
        let mut cv = Canvas::new(120, 40, Fonts::new());
        cv.clear(PAPER.bg);
        let w = keycap(&mut cv, 4.0, 4.0, "Z", &PAPER, false);
        assert!((c::KEY_MIN_W..=30.0).contains(&w), "{w}");
        // bottom border is 3 px of ink, the top 1.5
        assert_eq!(cv.pixel(10, 4 + 22 - 2), PAPER.fg);
        assert_eq!(cv.pixel(10, 4 + 3), PAPER.bg);
        let w2 = keycap(&mut cv, 40.0, 4.0, "R Shift", &PAPER, true);
        assert!(w2 > w);
        assert_eq!(cv.pixel(60, 4 + 22 - 2), PAPER.spot, "active cap uses spot");
    }

    #[test]
    fn ticks_and_buttons() {
        let mut cv = Canvas::new(300, 120, Fonts::new());
        cv.clear(PAPER.bg);
        tick(&mut cv, 10.0, 10.0, true, &PAPER);
        assert_eq!(cv.pixel(17, 17), PAPER.fg);
        assert_eq!(cv.pixel(12, 12), PAPER.bg, "bg inset ring");
        tick(&mut cv, 40.0, 10.0, false, &PAPER);
        assert_eq!(cv.pixel(47, 17), PAPER.bg);
        button_primary(&mut cv, 10.0, 40.0, 120.0, "Load", Some("F3"), &PAPER);
        assert_eq!(cv.pixel(15, 45), PAPER.fg);
        button_secondary(&mut cv, 150.0, 40.0, 120.0, "Save over", &PAPER);
        assert_eq!(cv.pixel(150, 45), PAPER.fg);
        assert_eq!(cv.pixel(155, 45), PAPER.bg);
        let w = destructive_link(&mut cv, 10.0, 95.0, "Delete this state", &PAPER);
        assert!(w > 50.0);
    }

    #[test]
    fn head_and_foot_frame() {
        let mut cv = Canvas::new(1280, 800, Fonts::new());
        cv.clear(PAPER.bg);
        head(&mut cv, "01", "Library", Some("12 cartridges · Games/"), Some(("B8CE", true)), &PAPER);
        foot(&mut cv, &[(&["↑", "↓", "←", "→"], "Choose"), (&["Z"], "Play")], Some(("Esc", "Quit")), &PAPER);
        // head rule: 1.5 px ink under the head
        assert_eq!(cv.pixel(640, 55), PAPER.fg);
        assert_eq!(cv.pixel(20, 55), PAPER.bg, "rule starts at the margin");
        // foot rule in line colour
        assert_eq!(cv.pixel(640, 800 - 44), PAPER.line);
        // section number is spot-coloured somewhere in the head's left corner
        let spot = (40..80).flat_map(|x| (10..46).map(move |y| (x, y))).filter(|&(x, y)| cv.pixel(x, y) == PAPER.spot).count();
        assert!(spot > 20, "{spot}");
        let mut night = Canvas::new(1280, 800, Fonts::new());
        night.clear(NIGHT.bg);
        head(&mut night, "D", "Bench", None, Some(("■ HALTED", false)), &NIGHT);
        assert_eq!(night.pixel(640, 55), NIGHT.fg);
    }

    #[test]
    fn mix_endpoints() {
        assert_eq!(mix([0, 0, 0], [255, 255, 255], 0.0), [0, 0, 0]);
        assert_eq!(mix([0, 0, 0], [255, 255, 255], 1.0), [255, 255, 255]);
        assert_eq!(mix([0, 0, 0], [200, 100, 0], 0.5), [100, 50, 0]);
    }
}
