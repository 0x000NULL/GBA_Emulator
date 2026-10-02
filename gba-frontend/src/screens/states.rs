//! Screen 03, Save states (`docs/gui/SCREENS.md`, mockup `03-save-states.png`): eight slots as
//! tilted film strips, a grease-pencil ring on the chosen frame, a loupe and the slot's details.
//!
//! The screen owns no files. `update` turns input into a [`StatesOutcome`] and the integrator
//! performs it (load the slot and switch to the game, save the running game over the slot,
//! delete the file and rescan). `StatesCtx` is rebuilt by the caller every frame from the
//! rescanned `SlotSet`.
//!
//! Mouse hit-testing uses the layout computed by the previous `render`, so an `update` before
//! any `render` ignores clicks.

use crate::slots::{SlotMeta, SlotSet, SLOTS};
use crate::ui::canvas::Clip;
use crate::ui::drawings::{self, Drawing};
use crate::ui::fmt::{fmt_frames, fmt_play, fmt_size, fmt_when};
use crate::ui::input::{Action, UiInput};
use crate::ui::text::Style;
use crate::ui::tokens::{component as c, font as f, layout as l, object, Palette, Rgb, PAPER};
use crate::ui::widgets::{button_primary, button_secondary, destructive_link, detail_row, foot, head};
use crate::ui::{Canvas, Fonts};
use gba_core::{SCREEN_H, SCREEN_W};
use minifb::Key;
use tiny_skia::{Color, Paint, PathBuilder, Pixmap, Point, Rect as SkRect, Transform};

const PER_STRIP: usize = 3;
const STRIP_W: f32 = c::FILM_STRIP_PAD_X * 2.0 + 3.0 * c::FILM_FRAME_W + 2.0 * c::FILM_FRAME_GAP;
const EDGE_H: f32 = 22.0;
const STRIP_H: f32 = 2.0 * c::FILM_SPROCKET_BAND + c::FILM_FRAME_H + EDGE_H;
const STRIP_GAP: f32 = 26.0;
const TOP_PAD: f32 = 28.0;
const BOTTOM_PAD: f32 = 28.0;
const SHADOW_PAD: f32 = 12.0;
const RING_CENTRE: (f32, f32) = (150.0, 118.0);
const EMPTY_FG: Rgb = [0x9C, 0x95, 0x8A];

/// What the screen asks the integrator to do.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StatesOutcome {
    None,
    /// Load this slot (1..=8) and switch to the game. Only returned for loadable slots.
    Load(u8),
    /// Save the running game over this slot. Only returned when a game is running.
    SaveOver(u8),
    /// The player confirmed the delete dialog for this slot.
    Delete(u8),
    Back,
}

/// Per-frame data the screen reads but does not own.
pub struct StatesCtx<'a> {
    pub slots: &'a SlotSet,
    /// Game title for the head.
    pub game: &'a str,
    /// A game is loaded, so Save over and F1 work.
    pub running: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Choice {
    Delete,
    Keep,
}

#[derive(Clone, Copy, Debug)]
struct Dialog {
    slot: u8,
    hl: Choice,
}

#[derive(Clone, Copy, Debug, Default)]
struct Rect {
    x: f32,
    y: f32,
    w: f32,
    h: f32,
}

impl Rect {
    fn new(x: f32, y: f32, w: f32, h: f32) -> Self {
        Rect { x, y, w, h }
    }
    fn contains(&self, p: (f32, f32)) -> bool {
        p.0 >= self.x && p.0 < self.x + self.w && p.1 >= self.y && p.1 < self.y + self.h
    }
    fn click(&self, i: &UiInput) -> bool {
        i.clicked && self.contains(i.mouse)
    }
    fn centre(&self) -> (f32, f32) {
        (self.x + self.w / 2.0, self.y + self.h / 2.0)
    }
}

/// Screen-space geometry from the last render, used for mouse hit-testing.
#[derive(Clone, Debug, Default)]
struct Layout {
    view: Rect,
    frames: [Rect; SLOTS],
    load: Option<Rect>,
    save: Option<Rect>,
    delete: Option<Rect>,
    dlg_delete: Rect,
    dlg_keep: Rect,
}

type Sig = Vec<Option<(u64, u64, u64, u64, bool)>>;

pub struct StatesScreen {
    selected: u8,
    scroll: f32,
    follow: bool,
    dialog: Option<Dialog>,
    layout: Layout,
    mouse: (f32, f32),
    ring: Drawing,
    strip_cv: Option<Canvas>,
    strips: Vec<Pixmap>,
    sig: Sig,
    shadow: Option<Pixmap>,
}

/// Slot number after an arrow key. Left/Right step by one and wrap; Up/Down step by a strip and
/// stop at the first/last row (Down from the strip above the last lands on the final slot at most).
pub fn step_selection(sel: u8, a: Action) -> u8 {
    let n = SLOTS as u8;
    let sel = sel.clamp(1, n);
    match a {
        Action::Left => {
            if sel == 1 {
                n
            } else {
                sel - 1
            }
        }
        Action::Right => {
            if sel == n {
                1
            } else {
                sel + 1
            }
        }
        Action::Up => {
            if sel > PER_STRIP as u8 {
                sel - PER_STRIP as u8
            } else {
                sel
            }
        }
        Action::Down => {
            if strip_row(sel) < strip_row(n) {
                (sel + PER_STRIP as u8).min(n)
            } else {
                sel
            }
        }
        _ => sel,
    }
}

/// Strip row (0-based) a slot lives on.
pub fn strip_row(slot: u8) -> usize {
    (slot as usize - 1) / PER_STRIP
}

fn slot_of<'a>(ctx: &'a StatesCtx, n: u8) -> Option<&'a SlotMeta> {
    ctx.slots.slots.get(n as usize - 1).and_then(|s| s.as_ref())
}

fn loadable(ctx: &StatesCtx, n: u8) -> bool {
    slot_of(ctx, n).is_some_and(|s| s.broken.is_none())
}

impl StatesScreen {
    pub fn new(selected: u8) -> Self {
        StatesScreen {
            selected: selected.clamp(1, SLOTS as u8),
            scroll: 0.0,
            follow: true,
            dialog: None,
            layout: Layout::default(),
            mouse: (-1.0, -1.0),
            ring: drawings::parse(drawings::PENCIL_RING),
            strip_cv: None,
            strips: Vec::new(),
            sig: Vec::new(),
            shadow: None,
        }
    }

    pub fn selected(&self) -> u8 {
        self.selected
    }

    /// True while the delete dialog is open.
    pub fn dialog_open(&self) -> bool {
        self.dialog.is_some()
    }

    fn select(&mut self, n: u8) {
        self.selected = n;
        self.follow = true;
    }

    fn open_dialog(&mut self, ctx: &StatesCtx) {
        if slot_of(ctx, self.selected).is_some() {
            self.dialog = Some(Dialog { slot: self.selected, hl: Choice::Delete });
        }
    }

    pub fn update(&mut self, input: &UiInput, ctx: &StatesCtx) -> StatesOutcome {
        self.mouse = input.mouse;
        if let Some(d) = self.dialog {
            return self.update_dialog(d, input);
        }
        for a in &input.actions {
            match a {
                Action::Left | Action::Right | Action::Up | Action::Down => {
                    let n = step_selection(self.selected, *a);
                    self.select(n);
                }
                Action::Confirm if loadable(ctx, self.selected) => return StatesOutcome::Load(self.selected),
                Action::Back => return StatesOutcome::Back,
                _ => {}
            }
        }
        if input.pressed(Key::F3) && loadable(ctx, self.selected) {
            return StatesOutcome::Load(self.selected);
        }
        if input.pressed(Key::F1) && ctx.running {
            return StatesOutcome::SaveOver(self.selected);
        }
        if input.pressed(Key::Delete) {
            self.open_dialog(ctx);
            return StatesOutcome::None;
        }
        if input.wheel != 0.0 && self.layout.view.contains(input.mouse) {
            self.scroll -= input.wheel * 60.0;
            self.follow = false;
        }
        if input.clicked {
            let lay = self.layout.clone();
            if lay.view.contains(input.mouse) && lay.frames.iter().any(|r| r.contains(input.mouse)) {
                if let Some(i) = lay.frames.iter().position(|r| r.contains(input.mouse)) {
                    self.select(i as u8 + 1);
                }
            } else if lay.load.is_some_and(|r| r.click(input)) {
                if loadable(ctx, self.selected) {
                    return StatesOutcome::Load(self.selected);
                }
            } else if lay.save.is_some_and(|r| r.click(input)) {
                if ctx.running {
                    return StatesOutcome::SaveOver(self.selected);
                }
            } else if lay.delete.is_some_and(|r| r.click(input)) {
                self.open_dialog(ctx);
            }
        }
        StatesOutcome::None
    }

    fn update_dialog(&mut self, d: Dialog, input: &UiInput) -> StatesOutcome {
        let mut hl = d.hl;
        if self.layout.dlg_delete.contains(input.mouse) {
            hl = Choice::Delete;
        } else if self.layout.dlg_keep.contains(input.mouse) {
            hl = Choice::Keep;
        }
        for a in &input.actions {
            match a {
                Action::Left | Action::Right | Action::Up | Action::Down => {
                    hl = if hl == Choice::Delete { Choice::Keep } else { Choice::Delete };
                }
                Action::Back => {
                    self.dialog = None;
                    return StatesOutcome::None;
                }
                Action::Confirm => return self.finish_dialog(d.slot, hl),
                _ => {}
            }
        }
        if input.pressed(Key::Escape) {
            self.dialog = None;
            return StatesOutcome::None;
        }
        if input.pressed(Key::Enter) {
            return self.finish_dialog(d.slot, hl);
        }
        if input.clicked {
            if self.layout.dlg_delete.click(input) {
                return self.finish_dialog(d.slot, Choice::Delete);
            }
            if self.layout.dlg_keep.click(input) {
                return self.finish_dialog(d.slot, Choice::Keep);
            }
        }
        self.dialog = Some(Dialog { hl, ..d });
        StatesOutcome::None
    }

    fn finish_dialog(&mut self, slot: u8, choice: Choice) -> StatesOutcome {
        self.dialog = None;
        match choice {
            Choice::Delete => StatesOutcome::Delete(slot),
            Choice::Keep => StatesOutcome::None,
        }
    }

    // --- rendering -------------------------------------------------------------------------

    pub fn render(&mut self, cv: &mut Canvas, ctx: &StatesCtx) {
        let pal = PAPER;
        let (w, h) = (cv.width(), cv.height());
        cv.set_clip(None);
        cv.clear(pal.bg);

        // Strips stay 1x unless the window is too narrow for strips and the column together.
        let mut col_w = 360.0;
        if w - 2.0 * l::MARGIN - l::GUTTER - col_w < STRIP_W {
            col_w = 300.0;
        }
        let s = ((w - 2.0 * l::MARGIN - l::GUTTER - col_w) / STRIP_W).clamp(0.3, 1.0);
        let col_x = w - l::MARGIN - col_w;

        let view = Rect::new(0.0, l::HEAD_H, w, (h - l::HEAD_H - l::FOOT_H).max(1.0));
        let rows = SLOTS.div_ceil(PER_STRIP);
        let pitch = (STRIP_H + STRIP_GAP) * s;
        let content_h = TOP_PAD + rows as f32 * STRIP_H * s + (rows - 1) as f32 * STRIP_GAP * s + BOTTOM_PAD;
        let max_scroll = (content_h - view.h).max(0.0);
        if self.follow {
            let top = TOP_PAD + strip_row(self.selected) as f32 * pitch;
            let bot = top + STRIP_H * s;
            let lo = bot + 24.0 - view.h;
            let hi = if strip_row(self.selected) == 0 { 0.0 } else { top - 24.0 };
            self.scroll = if lo <= hi { self.scroll.clamp(lo, hi) } else { hi };
        }
        self.scroll = self.scroll.clamp(0.0, max_scroll);

        self.rebuild_strips(ctx);
        let shadow = self.shadow.get_or_insert_with(build_shadow);

        cv.set_clip(Some(Clip { x: 0, y: view.y as i32, w: w as i32, h: view.h as i32 }));
        let strip_x = l::MARGIN;
        let mut layout = Layout { view, ..Layout::default() };
        for (row, pm) in self.strips.iter().enumerate() {
            let oy = view.y + TOP_PAD + row as f32 * pitch - self.scroll;
            let xf = strip_xf(c::FILM_TILT[row % 2], s, strip_x, oy);
            if oy < view.y + view.h + 12.0 && oy + STRIP_H * s > view.y - 12.0 {
                cv.draw_pixmap(shadow, 0.0, 0.0, xf.pre_translate(3.0 - SHADOW_PAD, 6.0 - SHADOW_PAD));
                cv.draw_pixmap(pm, 0.0, 0.0, xf);
            }
            for col in 0..PER_STRIP {
                let i = row * PER_STRIP + col;
                if i >= SLOTS {
                    break;
                }
                let (fx, fy) = frame_local(col);
                let mut p = [Point::from_xy(fx + c::FILM_FRAME_W / 2.0, fy + c::FILM_FRAME_H / 2.0)];
                xf.map_points(&mut p);
                let (fw, fh) = (c::FILM_FRAME_W * s, c::FILM_FRAME_H * s);
                layout.frames[i] = Rect::new(p[0].x - fw / 2.0, p[0].y - fh / 2.0, fw, fh);
            }
        }
        // Grease-pencil ring: the only selection mark.
        let (rcx, rcy) = layout.frames[self.selected as usize - 1].centre();
        if (s - 1.0).abs() < 1e-3 {
            cv.drawing(&self.ring, rcx - RING_CENTRE.0, rcy - RING_CENTRE.1);
        } else {
            let mut sub = Canvas::new(304, 240, Fonts::new());
            sub.pix.fill(Color::TRANSPARENT);
            sub.drawing(&self.ring, 0.0, 0.0);
            let xf = Transform::from_row(s, 0.0, 0.0, s, rcx - RING_CENTRE.0 * s, rcy - RING_CENTRE.1 * s);
            cv.draw_pixmap(&sub.pix, 0.0, 0.0, xf);
        }
        cv.set_clip(None);

        // Head and foot cover any overshoot at the edges of the scroll area.
        let right = format!("{} of {SLOTS} slots · {}", ctx.slots.used(), fmt_size(ctx.slots.total_bytes()));
        head(cv, "03", "Save states", Some(ctx.game), Some((&right, true)), &pal);
        let mut keys: Vec<(&[&str], &str)> = vec![(&["←", "→"], "Choose"), (&["F3"], "Load")];
        if ctx.running {
            keys.push((&["F1"], "Save over"));
        }
        keys.push((&["Del"], "Delete"));
        foot(cv, &keys, Some(("X", "Back")), &pal);

        self.draw_column(cv, ctx, &mut layout, col_x, col_w, view);
        self.layout = layout;
        if self.dialog.is_some() {
            self.draw_dialog(cv);
        }
    }

    fn rebuild_strips(&mut self, ctx: &StatesCtx) {
        let sig: Sig = (0..SLOTS)
            .map(|i| {
                ctx.slots.slots.get(i).and_then(|s| s.as_ref()).map(|m| {
                    let t = m.saved_at.duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
                    let px = m.frame_full.iter().step_by(97).fold(0u64, |a, &p| a.wrapping_mul(31).wrapping_add(p as u64));
                    (m.frame, m.play_secs, m.size ^ t.rotate_left(17), px, m.broken.is_some())
                })
            })
            .collect();
        if sig == self.sig && !self.strips.is_empty() {
            return;
        }
        let cv = self.strip_cv.get_or_insert_with(|| Canvas::new(STRIP_W as u32, STRIP_H as u32, Fonts::new()));
        self.strips.clear();
        for row in 0..SLOTS.div_ceil(PER_STRIP) {
            cv.pix.fill(Color::TRANSPARENT);
            draw_strip(cv, ctx, row);
            self.strips.push(cv.pix.clone());
        }
        self.sig = sig;
    }

    fn draw_column(&mut self, cv: &mut Canvas, ctx: &StatesCtx, lay: &mut Layout, x: f32, w: f32, view: Rect) {
        let pal = PAPER;
        let n = self.selected;
        let meta = slot_of(ctx, n);
        let broken = meta.and_then(|m| m.broken.as_deref());

        // Loupe: shrinks only on short windows.
        let full = c::LOUPE_DIAMETER + 2.0 * (c::LOUPE_RING + c::LOUPE_RIM);
        let below = 268.0 + if broken.is_some() { 54.0 } else { 0.0 };
        let outer = (view.h - 15.0 - below).clamp(180.0, full);
        let r_out = outer / 2.0;
        let r_glass = r_out - c::LOUPE_RING - c::LOUPE_RIM;
        let (lcx, lcy) = (x + w / 2.0, view.y + 15.0 + r_out);
        cv.circle(lcx, lcy, r_out, object::LOUPE_RIM);
        cv.circle(lcx, lcy, r_out - c::LOUPE_RIM, object::LOUPE_RING);
        cv.circle(lcx, lcy, r_glass, pal.bg2);
        if let Some(m) = meta {
            let fr = lay.frames[n as usize - 1];
            let (mut px, mut py) = (SCREEN_W as f32 / 2.0, SCREEN_H as f32 / 2.0);
            if fr.w > 0.0 && lay.view.contains(self.mouse) && fr.contains(self.mouse) {
                let k = SCREEN_W as f32 / fr.w;
                px = (self.mouse.0 - fr.x) * k;
                py = (self.mouse.1 - fr.y) * k;
            }
            let mag = c::LOUPE_MAGNIFY;
            let reach = r_glass / mag;
            px = px.clamp(reach.min(120.0), (SCREEN_W as f32 - reach).max(120.0));
            py = py.clamp(reach.min(80.0), (SCREEN_H as f32 - reach).max(80.0));
            let (ox, oy) = ((lcx - px * mag).round() as i32, (lcy - py * mag).round() as i32);
            let grey;
            let pic: &[u16] = if broken.is_some() {
                grey = desaturate(&m.frame_full[..]);
                &grey
            } else {
                &m.frame_full[..]
            };
            cv.blit_indexed_in_circle(pic, SCREEN_W, SCREEN_H, crate::pixels::bgr555_lut(), ox, oy, mag as usize, lcx, lcy, r_glass);
        }

        // Title line
        let mut y = lcy + r_out + 22.0;
        let title = Style::display(f::DISPLAY_TITLE).color(pal.fg);
        cv.text(x, y, &format!("SLOT {n}"), &title);
        if let Some(m) = meta.filter(|m| m.saved_at > std::time::UNIX_EPOCH) {
            let st = Style::data(f::DATA_SMALL).color(pal.fg2);
            cv.text_right(x + w, y + 18.0, &fmt_when(m.saved_at), &st);
        }
        y += 41.0;
        cv.hrule(x, x + w, y, 1.5, pal.fg);
        y += 12.0;

        match meta {
            Some(m) => {
                let healthy = m.broken.is_none();
                let col = 104.0;
                let dash = "–".to_string();
                let frame = if healthy || m.frame > 0 { fmt_frames(m.frame) } else { dash.clone() };
                let play = if healthy || m.play_secs > 0 { fmt_play(m.play_secs) } else { dash };
                detail_row(cv, x, y, col, "FRAME", &frame, true, &pal);
                detail_row(cv, x, y + 24.0, col, "PLAY TIME", &play, true, &pal);
                detail_row(cv, x, y + 48.0, col, "SIZE", &fmt_size(m.size), true, &pal);
                let name = m.path.file_name().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
                let name = cv.fonts.ellipsize_middle(&Style::data(f::DATA_BODY), &name, w - col);
                detail_row(cv, x, y + 72.0, col, "FILE", &name, true, &pal);
                y += 96.0;
                if let Some(reason) = &m.broken {
                    let st = Style::reading(f::READING_BODY).color(pal.spot);
                    let used = cv.paragraph(x, y + 2.0, w, reason, &st, f::READING_LINE_HEIGHT, 3);
                    y += used + 6.0;
                }
            }
            None => {
                let st = Style::reading(f::READING_BODY).color(pal.fg2);
                let msg = if ctx.slots.used() == 0 {
                    "No saved states for this game yet. Press F1 while you play to keep a moment here."
                } else if ctx.running {
                    "Nothing saved in this slot. Press F1 to save the game as it is now."
                } else {
                    "Nothing saved in this slot. Start the game, then press F1 to save here."
                };
                let used = cv.paragraph(x, y + 2.0, w, msg, &st, f::READING_LINE_HEIGHT, 4);
                y += used.max(72.0) + 6.0;
            }
        }
        y += 16.0;

        // Buttons
        let load_w = if ctx.running { ((w - 10.0) * 0.69).round() } else { w };
        let load_pal = if loadable(ctx, n) { pal } else { Palette { fg: pal.fg2, ..pal } };
        button_primary(cv, x, y, load_w, "Load", Some("F3"), &load_pal);
        lay.load = Some(Rect::new(x, y, load_w, c::BUTTON_H));
        if ctx.running {
            let sx = x + load_w + 10.0;
            let sw = w - load_w - 10.0;
            button_secondary(cv, sx, y, sw, "Save over", &pal);
            lay.save = Some(Rect::new(sx, y, sw, c::BUTTON_H));
        }
        y += c::BUTTON_H + 16.0;
        if meta.is_some() {
            let lw = destructive_link(cv, x, y, "Delete this state", &pal);
            lay.delete = Some(Rect::new(x, y - 4.0, lw, 24.0));
        }
    }

    fn draw_dialog(&mut self, cv: &mut Canvas) {
        let Some(d) = self.dialog else { return };
        let pal = PAPER;
        let (w, h) = (cv.width(), cv.height());
        fill_alpha(cv, 0.0, 0.0, w, h, pal.bg, 140);
        let (dw, dh) = (440.0, 172.0);
        let (dx, dy) = (((w - dw) / 2.0).round(), ((h - dh) / 2.0).round());
        for k in 0..6 {
            let g = k as f32 * 2.0;
            fill_alpha(cv, dx + 3.0 - g, dy + 6.0 - g, dw + 2.0 * g, dh + 2.0 * g, [0, 0, 0], 9);
        }
        cv.fill_rect(dx, dy, dw, dh, pal.fg);
        cv.fill_rect(dx + 1.5, dy + 1.5, dw - 3.0, dh - 3.0, pal.bg);
        let st = Style::reading(f::READING_INTRO).color(pal.fg);
        cv.paragraph(dx + 28.0, dy + 28.0, dw - 56.0, &format!("Delete slot {}? This cannot be undone.", d.slot), &st, f::READING_LINE_HEIGHT, 2);
        let by = dy + dh - 28.0 - c::BUTTON_H;
        let bw = 128.0;
        let del = Rect::new(dx + 28.0, by, bw, c::BUTTON_H);
        let keep = Rect::new(dx + 28.0 + bw + 14.0, by, bw, c::BUTTON_H);
        cv.fill_rect(del.x, del.y, del.w, del.h, pal.spot);
        let bt = Style::interface_bold(f::INTERFACE_BUTTON).color(pal.bg);
        let tw = cv.measure("Delete", &bt);
        cv.text(del.x + (del.w - tw) / 2.0, del.y + (del.h - bt.size * 1.2) / 2.0, "Delete", &bt);
        button_secondary(cv, keep.x, keep.y, keep.w, "Keep", &pal);
        let ring = if d.hl == Choice::Delete { del } else { keep };
        cv.stroke_rect(ring.x - 4.0, ring.y - 4.0, ring.w + 8.0, ring.h + 8.0, 1.5, pal.fg);
        self.layout.dlg_delete = del;
        self.layout.dlg_keep = keep;
    }
}

// --- helpers -------------------------------------------------------------------------------

/// Top-left of frame `col` (0..3) inside a strip, in strip pixels.
fn frame_local(col: usize) -> (f32, f32) {
    (c::FILM_STRIP_PAD_X + col as f32 * (c::FILM_FRAME_W + c::FILM_FRAME_GAP), c::FILM_SPROCKET_BAND)
}

/// Strip pixels to screen: rotate `deg` about the strip centre, scale by `s`, place at (ox, oy).
fn strip_xf(deg: f32, s: f32, ox: f32, oy: f32) -> Transform {
    let (cx, cy) = (STRIP_W / 2.0, STRIP_H / 2.0);
    let (sn, cs) = deg.to_radians().sin_cos();
    Transform::from_row(
        s * cs,
        s * sn,
        -s * sn,
        s * cs,
        s * (cx - cs * cx + sn * cy) + ox,
        s * (cy - sn * cx - cs * cy) + oy,
    )
}

/// Fill a rectangle with `c` at `alpha` (0..=255) straight on the pixmap, honouring no clip.
fn fill_alpha(cv: &mut Canvas, x: f32, y: f32, w: f32, h: f32, c: Rgb, alpha: u8) {
    let Some(r) = SkRect::from_xywh(x, y, w, h) else { return };
    let mut p = Paint::default();
    p.set_color(Color::from_rgba8(c[0], c[1], c[2], alpha));
    cv.pix.fill_rect(r, &p, Transform::identity(), None);
}

/// Soft drop shadow for one strip, drawn in the strip's own (pre-rotation) space.
fn build_shadow() -> Pixmap {
    let mut pm = Pixmap::new((STRIP_W + 2.0 * SHADOW_PAD) as u32, (STRIP_H + 2.0 * SHADOW_PAD) as u32).expect("shadow pixmap");
    let mut p = Paint::default();
    p.set_color(Color::from_rgba8(0, 0, 0, 6));
    for k in 0..6 {
        let g = (5 - k) as f32 * 2.0;
        let r = SkRect::from_xywh(SHADOW_PAD - g, SHADOW_PAD - g, STRIP_W + 2.0 * g, STRIP_H + 2.0 * g).expect("rect");
        pm.fill_rect(r, &p, Transform::identity(), None);
    }
    pm
}

/// Average the colour channels of a BGR555 picture (a slot that cannot be loaded).
fn desaturate(fb: &[u16]) -> Vec<u16> {
    fb.iter()
        .map(|&p| {
            let g = ((p & 31) + ((p >> 5) & 31) + ((p >> 10) & 31)) / 3;
            g | (g << 5) | (g << 10)
        })
        .collect()
}

/// One film strip (slots `3*row+1 ..= 3*row+3`) into `cv`, which is `STRIP_W` x `STRIP_H`.
fn draw_strip(cv: &mut Canvas, ctx: &StatesCtx, row: usize) {
    let pal = PAPER;
    cv.round_rect(0.0, 0.0, STRIP_W, STRIP_H, [2.0; 4], object::FILM);
    // Sprocket holes
    let band = c::FILM_SPROCKET_BAND;
    let mut x = 9.0;
    while x + c::FILM_HOLE_W <= STRIP_W - 6.0 {
        for y in [(band - c::FILM_HOLE_H) / 2.0, STRIP_H - band + (band - c::FILM_HOLE_H) / 2.0] {
            cv.round_rect(x, y, c::FILM_HOLE_W, c::FILM_HOLE_H, [1.5; 4], pal.bg);
        }
        x += c::FILM_HOLE_PITCH;
    }
    let edge = Style::data(f::DATA_SMALL).color(object::FILM_EDGE_TEXT);
    for col in 0..PER_STRIP {
        let i = row * PER_STRIP + col;
        if i >= SLOTS {
            break; // unexposed film
        }
        let n = i as u8 + 1;
        let (fx, fy) = frame_local(col);
        let meta = slot_of(ctx, n);
        let (mid, right): (String, String) = match meta {
            Some(m) => {
                cv.fill_rect(fx, fy, c::FILM_FRAME_W, c::FILM_FRAME_H, [0, 0, 0]);
                if m.broken.is_some() {
                    let grey = desaturate(&m.frame_full[..]);
                    cv.blit_indexed(&grey, SCREEN_W, SCREEN_H, crate::pixels::bgr555_lut(), fx as i32, fy as i32, 1);
                    ("CAN'T LOAD".into(), if m.play_secs > 0 { fmt_play(m.play_secs) } else { "–".into() })
                } else {
                    cv.blit_indexed(&m.frame_full[..], SCREEN_W, SCREEN_H, crate::pixels::bgr555_lut(), fx as i32, fy as i32, 1);
                    (format!("FR {}", fmt_frames(m.frame)), fmt_play(m.play_secs))
                }
            }
            None => {
                cv.fill_rect(fx, fy, c::FILM_FRAME_W, c::FILM_FRAME_H, object::FILM_EMPTY);
                let (cx, cy) = (fx + c::FILM_FRAME_W / 2.0, fy + c::FILM_FRAME_H / 2.0);
                cv.text_center(cx, cy - 22.0, "EMPTY", &Style::display(22.0).color(EMPTY_FG));
                cv.text_center(cx, cy + 6.0, "F1 saves here", &Style::data(f::DATA_SMALL).color(EMPTY_FG));
                ("FR –".into(), "–".into())
            }
        };
        let ty = fy + c::FILM_FRAME_H + 6.0;
        // The Data face has no "▸" glyph: draw the marker.
        let mut pb = PathBuilder::new();
        pb.move_to(fx + 1.0, ty + 2.5);
        pb.line_to(fx + 6.0, ty + 6.0);
        pb.line_to(fx + 1.0, ty + 9.5);
        pb.close();
        if let Some(tri) = pb.finish() {
            cv.fill_path(&tri, object::FILM_EDGE_TEXT, Transform::identity());
        }
        cv.text(fx + 11.0, ty, &n.to_string(), &edge);
        cv.text_center(fx + c::FILM_FRAME_W / 2.0, ty, &mid, &edge);
        cv.text_right(fx + c::FILM_FRAME_W, ty, &right, &edge);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::time::{Duration, UNIX_EPOCH};

    fn meta(n: u8, fb: Box<gba_core::FrameBuffer>, broken: Option<&str>) -> SlotMeta {
        SlotMeta {
            slot: n,
            path: PathBuf::from(format!("Kingdom Hearts - Chain of Memories (USA).s{n}.state")),
            frame: 1800 * n as u64 * 7,
            saved_at: UNIX_EPOCH + Duration::from_secs(1_790_000_000 + 3600 * n as u64),
            play_secs: 30 * n as u64 * 7,
            core_ver: 1,
            emulator: "test".into(),
            size: 429 * 1024,
            frame_full: fb.to_vec(),
            broken: broken.map(str::to_string),
        }
    }

    fn blank() -> Box<gba_core::FrameBuffer> {
        vec![0x2108u16; SCREEN_W * SCREEN_H].into_boxed_slice().try_into().unwrap()
    }

    /// Slots 1, 2 loadable; 3 broken; the rest empty.
    fn set() -> SlotSet {
        let mut slots: Vec<Option<SlotMeta>> = (0..SLOTS).map(|_| None).collect();
        slots[0] = Some(meta(1, blank(), None));
        slots[1] = Some(meta(2, blank(), None));
        slots[2] = Some(meta(3, blank(), Some("saved with state format 0")));
        SlotSet { slots }
    }

    fn act(a: Action) -> UiInput {
        UiInput { actions: vec![a], ..UiInput::default() }
    }

    fn key(k: Key) -> UiInput {
        UiInput { pressed: vec![k], repeated: vec![k], ..UiInput::default() }
    }

    fn ctx<'a>(slots: &'a SlotSet, running: bool) -> StatesCtx<'a> {
        StatesCtx { slots, game: "Test", running }
    }

    #[test]
    fn selection_wraps_left_right_and_stops_up_down() {
        assert_eq!(step_selection(1, Action::Left), 8);
        assert_eq!(step_selection(8, Action::Right), 1);
        assert_eq!(step_selection(3, Action::Right), 4);
        assert_eq!(step_selection(2, Action::Up), 2, "top strip: nowhere to go");
        assert_eq!(step_selection(5, Action::Up), 2);
        assert_eq!(step_selection(2, Action::Down), 5);
        assert_eq!(step_selection(6, Action::Down), 8, "third strip has two frames");
        assert_eq!(step_selection(7, Action::Down), 7, "already on the last strip");
        assert_eq!(strip_row(1), 0);
        assert_eq!(strip_row(3), 0);
        assert_eq!(strip_row(4), 1);
        assert_eq!(strip_row(8), 2);
    }

    #[test]
    fn arrows_move_the_selection() {
        let slots = set();
        let cx = ctx(&slots, true);
        let mut s = StatesScreen::new(1);
        assert_eq!(s.update(&act(Action::Right), &cx), StatesOutcome::None);
        assert_eq!(s.selected(), 2);
        s.update(&act(Action::Down), &cx);
        assert_eq!(s.selected(), 5);
        assert_eq!(StatesScreen::new(99).selected(), 8);
        assert_eq!(StatesScreen::new(0).selected(), 1);
    }

    #[test]
    fn load_refused_for_empty_and_broken_slots() {
        let slots = set();
        let cx = ctx(&slots, true);
        assert_eq!(StatesScreen::new(1).update(&act(Action::Confirm), &cx), StatesOutcome::Load(1));
        assert_eq!(StatesScreen::new(2).update(&key(Key::F3), &cx), StatesOutcome::Load(2));
        for n in [3, 4, 8] {
            assert_eq!(StatesScreen::new(n).update(&act(Action::Confirm), &cx), StatesOutcome::None, "slot {n}");
            assert_eq!(StatesScreen::new(n).update(&key(Key::F3), &cx), StatesOutcome::None, "slot {n}");
        }
    }

    #[test]
    fn save_over_needs_a_running_game() {
        let slots = set();
        assert_eq!(StatesScreen::new(5).update(&key(Key::F1), &ctx(&slots, true)), StatesOutcome::SaveOver(5));
        assert_eq!(StatesScreen::new(5).update(&key(Key::F1), &ctx(&slots, false)), StatesOutcome::None);
    }

    #[test]
    fn back_leaves_the_screen() {
        let slots = set();
        assert_eq!(StatesScreen::new(1).update(&act(Action::Back), &ctx(&slots, false)), StatesOutcome::Back);
    }

    #[test]
    fn delete_dialog_keep_closes_without_deleting() {
        let slots = set();
        let cx = ctx(&slots, true);
        let mut s = StatesScreen::new(2);
        assert_eq!(s.update(&key(Key::Delete), &cx), StatesOutcome::None);
        assert!(s.dialog_open());
        // arrows toggle the highlight, Confirm acts on it
        assert_eq!(s.update(&act(Action::Right), &cx), StatesOutcome::None);
        assert_eq!(s.update(&act(Action::Confirm), &cx), StatesOutcome::None);
        assert!(!s.dialog_open());
        // Back and Esc are Keep too
        s.update(&key(Key::Delete), &cx);
        assert_eq!(s.update(&act(Action::Back), &cx), StatesOutcome::None);
        assert!(!s.dialog_open());
        s.update(&key(Key::Delete), &cx);
        assert_eq!(s.update(&key(Key::Escape), &cx), StatesOutcome::None);
        assert!(!s.dialog_open());
    }

    #[test]
    fn delete_dialog_confirm_deletes() {
        let slots = set();
        let cx = ctx(&slots, false);
        let mut s = StatesScreen::new(3); // a broken slot can still be deleted
        s.update(&key(Key::Delete), &cx);
        assert_eq!(s.update(&act(Action::Confirm), &cx), StatesOutcome::Delete(3));
        assert!(!s.dialog_open());
        let mut s = StatesScreen::new(1);
        s.update(&key(Key::Delete), &cx);
        assert_eq!(s.update(&key(Key::Enter), &cx), StatesOutcome::Delete(1));
    }

    #[test]
    fn dialog_swallows_other_keys_and_empty_slots_have_nothing_to_delete() {
        let slots = set();
        let cx = ctx(&slots, true);
        let mut s = StatesScreen::new(6);
        s.update(&key(Key::Delete), &cx);
        assert!(!s.dialog_open(), "empty slot");
        let mut s = StatesScreen::new(1);
        s.update(&key(Key::Delete), &cx);
        assert_eq!(s.update(&key(Key::F1), &cx), StatesOutcome::None, "F1 is ignored under the dialog");
        assert_eq!(s.update(&act(Action::Back), &cx), StatesOutcome::None);
        assert_eq!(s.update(&act(Action::Back), &cx), StatesOutcome::Back);
    }

    #[test]
    fn mouse_selects_frames_and_presses_buttons() {
        let slots = set();
        let cx = ctx(&slots, true);
        let mut canvas = Canvas::new(1280, 800, Fonts::new());
        let mut s = StatesScreen::new(1);
        s.render(&mut canvas, &cx);
        let lay = s.layout.clone();
        let click_at = |p: (f32, f32)| UiInput { mouse: p, clicked: true, mouse_down: true, ..UiInput::default() };
        // frame 2
        assert_eq!(s.update(&click_at(lay.frames[1].centre()), &cx), StatesOutcome::None);
        assert_eq!(s.selected(), 2);
        s.render(&mut canvas, &cx);
        let lay = s.layout.clone();
        assert_eq!(s.update(&click_at(lay.load.unwrap().centre()), &cx), StatesOutcome::Load(2));
        assert_eq!(s.update(&click_at(lay.save.unwrap().centre()), &cx), StatesOutcome::SaveOver(2));
        // Delete this state opens the dialog; Keep closes it, Delete returns the slot
        s.update(&click_at(lay.delete.unwrap().centre()), &cx);
        assert!(s.dialog_open());
        s.render(&mut canvas, &cx);
        let lay = s.layout.clone();
        assert_eq!(s.update(&click_at(lay.dlg_keep.centre()), &cx), StatesOutcome::None);
        assert!(!s.dialog_open());
        s.update(&key(Key::Delete), &cx);
        s.render(&mut canvas, &cx);
        let lay = s.layout.clone();
        assert_eq!(s.update(&click_at(lay.dlg_delete.centre()), &cx), StatesOutcome::Delete(2));
        // Load on an empty slot does nothing
        s.select(6);
        s.render(&mut canvas, &cx);
        let lay = s.layout.clone();
        assert_eq!(s.update(&click_at(lay.load.unwrap().centre()), &cx), StatesOutcome::None);
        assert!(lay.delete.is_none());
    }

    #[test]
    fn save_over_button_hidden_when_not_running() {
        let slots = set();
        let mut canvas = Canvas::new(1280, 800, Fonts::new());
        let mut s = StatesScreen::new(1);
        s.render(&mut canvas, &ctx(&slots, false));
        assert!(s.layout.save.is_none());
        s.render(&mut canvas, &ctx(&slots, true));
        assert!(s.layout.save.is_some());
    }

    #[test]
    fn selection_scrolls_into_view_and_wheel_clamps() {
        let slots = set();
        let cx = ctx(&slots, true);
        let mut canvas = Canvas::new(1280, 800, Fonts::new());
        let mut s = StatesScreen::new(1);
        s.render(&mut canvas, &cx);
        assert_eq!(s.scroll, 0.0);
        s.select(5);
        s.render(&mut canvas, &cx);
        assert_eq!(s.scroll, 0.0, "two strips fit at 800 px");
        s.select(8);
        s.render(&mut canvas, &cx);
        assert!(s.scroll > 0.0, "third strip needs scrolling");
        let bottom = s.layout.frames[7].y + c::FILM_FRAME_H;
        assert!(bottom < 800.0 - l::FOOT_H);
        s.select(1);
        s.render(&mut canvas, &cx);
        assert_eq!(s.scroll, 0.0);
        let wheel = UiInput { wheel: -50.0, mouse: (100.0, 300.0), ..UiInput::default() };
        s.update(&wheel, &cx);
        s.render(&mut canvas, &cx);
        assert!(s.scroll > 0.0 && s.scroll < 200.0, "clamped to the content: {}", s.scroll);
    }

    #[test]
    fn narrow_window_still_lays_out() {
        let slots = set();
        let mut canvas = Canvas::new(960, 640, Fonts::new());
        let mut s = StatesScreen::new(4);
        s.render(&mut canvas, &ctx(&slots, true));
        let r = s.layout.frames[2];
        assert!(r.x + r.w < 960.0 - l::MARGIN - 300.0 + 1.0, "strips leave the column free");
    }

    #[test]
    fn desaturate_is_grey() {
        let g = desaturate(&[0x7FFF, 0x001F, 0x0000]);
        assert_eq!(g[0], 0x7FFF);
        assert_eq!(g[1] & 31, g[1] >> 5 & 31);
        assert_eq!(g[1] >> 5 & 31, g[1] >> 10);
        assert_eq!(g[2], 0);
    }

    #[test]
    fn pinned_states_renders() {
        crate::ui::fmt::test_clock::freeze(crate::pinned::NOW);
        let mut slots: Vec<Option<SlotMeta>> = (0..SLOTS).map(|_| None).collect();
        for (i, frames) in [(0usize, 300u32), (1, 900), (2, 1500), (3, 2400), (4, 3300)] {
            let broken = if i == 2 { Some("saved with state format 1, this build reads 2") } else { None };
            let mut m = meta(i as u8 + 1, crate::pinned::test_frame(frames), broken);
            m.frame = [1800, 12_000, 18_000, 27_400, 34_850][i];
            m.play_secs = [30, 200, 301, 458, 583][i];
            // today, yesterday, this year, last year, today
            let ago = [3600u64, 86_400 + 3600, 86_400 * 30, 86_400 * 400, 7200][i];
            m.saved_at = UNIX_EPOCH + Duration::from_secs(crate::pinned::NOW as u64 - ago);
            slots[i] = Some(m);
        }
        let set = SlotSet { slots };
        let cx = StatesCtx { slots: &set, game: "Kingdom Hearts: Chain of Memories", running: true };
        let mut canvas = Canvas::new(1280, 800, Fonts::new());
        let mut s = StatesScreen::new(4);
        s.render(&mut canvas, &cx);
        crate::pinned::check_canvas("states", &canvas);
        s.update(&key(Key::Delete), &cx);
        s.render(&mut canvas, &cx);
        crate::pinned::check_canvas("states-dialog", &canvas);
        let mut s = StatesScreen::new(8);
        let mut hover = UiInput::default();
        s.render(&mut canvas, &cx);
        let c = s.layout.frames[7].centre();
        hover.mouse = (c.0 - 40.0, c.1 + 10.0);
        s.update(&hover, &cx);
        s.render(&mut canvas, &cx);
        crate::pinned::check_canvas("states-scrolled", &canvas);
        let mut canvas = Canvas::new(960, 640, Fonts::new());
        let mut s = StatesScreen::new(1);
        s.render(&mut canvas, &cx);
        crate::pinned::check_canvas("states-960", &canvas);
    }

    /// Visual review: `GBA_ROM=<rom> cargo test -p gba-frontend --release -- --ignored render_states`.
    #[test]
    #[ignore]
    fn render_states() {
        let mut slots: Vec<Option<SlotMeta>> = (0..SLOTS).map(|_| None).collect();
        for (i, frames) in [(0usize, 300u32), (1, 900), (2, 1500), (3, 2400), (4, 3300)] {
            let broken = if i == 2 { Some("saved with state format 1, this build reads 2") } else { None };
            let mut m = meta(i as u8 + 1, crate::render_tests::sample_frame(frames), broken);
            m.frame = [1800, 12_000, 18_000, 27_400, 34_850][i];
            m.play_secs = [30, 200, 301, 458, 583][i];
            m.saved_at = std::time::SystemTime::now() - Duration::from_secs(3600 * i as u64);
            slots[i] = Some(m);
        }
        let set = SlotSet { slots };
        let cx = StatesCtx { slots: &set, game: "Kingdom Hearts: Chain of Memories", running: true };
        let mut canvas = Canvas::new(1280, 800, Fonts::new());
        let mut s = StatesScreen::new(4);
        s.render(&mut canvas, &cx);
        let dir = crate::render_tests::out_dir();
        canvas.save_png(&dir.join("03-save-states.png")).unwrap();
        s.update(&key(Key::Delete), &cx);
        s.render(&mut canvas, &cx);
        canvas.save_png(&dir.join("03-save-states-dialog.png")).unwrap();
        // scrolled to the third strip, pointer over the selected frame
        let mut s = StatesScreen::new(8);
        let mut hover = UiInput::default();
        s.render(&mut canvas, &cx);
        let c = s.layout.frames[7].centre();
        hover.mouse = (c.0 - 40.0, c.1 + 10.0);
        s.update(&hover, &cx);
        s.render(&mut canvas, &cx);
        canvas.save_png(&dir.join("03-save-states-scrolled.png")).unwrap();
    }
}
