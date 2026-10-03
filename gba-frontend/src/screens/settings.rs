//! Screen 04, Settings: an open instruction manual, two paper pages per spread
//! (`docs/gui/SCREENS.md`, mockup `04-settings.png`).
//!
//! Spread 0 holds Controls (left) and Screen & sound (right), spread 1 System, spread 2 Saves.
//! The screen edits the [`Settings`] it is given (and the live keyboard, pad and hotkey maps)
//! and reports
//! [`SettingsOutcome::Changed`] after every edit so the app can persist and re-apply them.
//!
//! The app builds a [`SettingsCtx`] for every `update` and `render` call. While
//! `ctx.rebinding` is true after `update` (waiting for a key, or typing a path) the app must not
//! treat key presses as hotkeys.

// Not wired into `app` yet: the integrator connects `SettingsScreen` and `Settings`.
#![allow(dead_code)]

use crate::debug_ui::{Hotkey, HotkeyMap};
use crate::settings::{AudioMode, Bios, FfLimit, Filter, Pacing, PictureSize, Settings};
use crate::ui::canvas::{round_rect_path, segs_to_path, Canvas, Clip};
use crate::ui::drawings::{self, Drawing, Geometry};
use crate::ui::input::{bindable, key_name, pad_button_name, Action, KeyMap, PadMap, UiInput};
use gilrs::Button;
use crate::ui::text::Style;
use crate::ui::tokens::{component as c, font as f, layout as l, Palette, PAPER};
use crate::ui::widgets::{self, mix};
use gba_core::Keys;
use minifb::Key;
use std::path::PathBuf;
use std::time::{Duration, Instant};
use tiny_skia::Transform;

/// What the screen needs from the app for one `update` / `render` call.
pub struct SettingsCtx<'a> {
    pub settings: &'a mut Settings,
    pub keymap: &'a mut KeyMap,
    pub padmap: &'a mut PadMap,
    pub hotkeys: &'a mut HotkeyMap,
    /// Output device names for the System spread (the app lists them with cpal).
    pub audio_devices: &'a [String],
    /// Out: true while the screen waits for a key (rebinding) or a typed path, so the app does
    /// not treat key presses as hotkeys.
    pub rebinding: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SettingsOutcome {
    None,
    /// Something changed: the app persists the settings and re-applies them.
    Changed,
    Back,
    /// Show this folder in the file manager (`explorer`).
    OpenFolder(PathBuf),
}

const SPREADS: usize = 3;
const TABS: [&str; SPREADS] = ["Controls · Screen & sound", "System", "Saves"];
/// How long a swapped keycap flashes.
const FLASH: Duration = Duration::from_millis(250);

/// What a keycap rebinds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Target {
    /// The keyboard key of a GBA button (`Keys` bit).
    Button(u16),
    /// The gamepad button of a GBA button.
    Pad(u16),
    Hot(Hotkey),
    FastForward,
}

/// A focusable thing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Item {
    Key(Target),
    Picture,
    Filter,
    FfLimit,
    Volume,
    Bios,
    BiosPath,
    Folder(usize),
    AddFolder,
    Device,
    Audio,
    Pacing,
    Autosave,
    OpenFolder,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum HitKind {
    /// Keycap, button or text field.
    Activate,
    Option(usize),
    Ruler,
    Remove,
}

#[derive(Clone, Copy, Debug, Default)]
struct R {
    x: f32,
    y: f32,
    w: f32,
    h: f32,
}

impl R {
    fn contains(&self, p: (f32, f32)) -> bool {
        p.0 >= self.x && p.0 < self.x + self.w && p.1 >= self.y && p.1 < self.y + self.h
    }
}

#[derive(Clone, Copy, Debug)]
struct Hit {
    r: R,
    item: Item,
    kind: HitKind,
    page: usize,
}

struct TextEdit {
    item: Item,
    buf: String,
}

/// Hotkeys shown under "Emulator keys", in focus order (left column, then right).
const HOT_LEFT: [(Hotkey, &str); 5] = [
    (Hotkey::Pause, "Pause"),
    (Hotkey::FrameStep, "Advance one frame"),
    (Hotkey::Screenshot, "Screenshot"),
    (Hotkey::SaveStates, "Save states"),
    (Hotkey::SunUp, "Sun brighter (Boktai)"),
];
const HOT_RIGHT: [(Hotkey, &str); 5] = [
    (Hotkey::SaveState, "Save state"),
    (Hotkey::LoadState, "Load state"),
    (Hotkey::SaveNow, "Write .sav now"),
    (Hotkey::Bench, "Bench"),
    (Hotkey::SunDown, "Sun dimmer (Boktai)"),
];

pub struct SettingsScreen {
    spread: usize,
    focus: [usize; SPREADS],
    waiting: Option<Target>,
    flash: Vec<(Target, Instant)>,
    text: Option<TextEdit>,
    /// The path typed last, restored when "From file…" is chosen again.
    bios_memory: PathBuf,
    /// Folder of the game being played, for "Open folder".
    game_dir: Option<PathBuf>,
    figure: Drawing,
    // layout of the last render (hit testing, scrolling)
    hits: Vec<Hit>,
    tab_rects: [R; SPREADS],
    page_rects: [R; 2],
    scroll: [f32; 2],
    content_h: [f32; 2],
    cur_focus: Option<Item>,
    focus_rect: Option<(usize, R)>,
    ensure_visible: bool,
    last_mouse: (f32, f32),
}

impl Default for SettingsScreen {
    fn default() -> Self {
        Self::new()
    }
}

fn hot_key(hk: &HotkeyMap, h: Hotkey) -> Option<Key> {
    // Esc also pauses; the keycap shows the rebindable key
    hk.keys.iter().find(|(k, x)| *x == h && *k != Key::Escape).map(|(k, _)| *k).or_else(|| hk.key_for(h))
}

/// What the keycap of `t` prints.
fn target_label(t: Target, ctx: &SettingsCtx) -> String {
    match t {
        Target::Button(bit) => key_name(ctx.keymap.key_for(bit)),
        Target::Pad(bit) => pad_button_name(ctx.padmap.button_for(bit)).to_string(),
        Target::Hot(h) => hot_key(ctx.hotkeys, h).map(key_name).unwrap_or_else(|| "?".into()),
        Target::FastForward => key_name(ctx.hotkeys.fast_forward),
    }
}

/// Bind a hotkey; a key used by another hotkey (or fast-forward) is swapped. Returns whether
/// anything changed and the partner of a swap.
fn bind_hot(hk: &mut HotkeyMap, h: Hotkey, key: Key) -> (bool, Option<Target>) {
    let Some(i) = hk.keys.iter().position(|(k, x)| *x == h && *k != Key::Escape) else {
        hk.keys.push((key, h));
        return (true, None);
    };
    let old = hk.keys[i].0;
    if old == key {
        return (false, None);
    }
    let mut partner = None;
    if let Some(j) = hk.keys.iter().position(|(k, x)| *k == key && *x != h) {
        partner = Some(Target::Hot(hk.keys[j].1));
        hk.keys[j].0 = old;
    } else if hk.fast_forward == key {
        partner = Some(Target::FastForward);
        hk.fast_forward = old;
    }
    hk.keys[i].0 = key;
    (true, partner)
}

fn bind_fast_forward(hk: &mut HotkeyMap, key: Key) -> (bool, Option<Target>) {
    let old = hk.fast_forward;
    if old == key {
        return (false, None);
    }
    let mut partner = None;
    if let Some(j) = hk.keys.iter().position(|(k, _)| *k == key) {
        partner = Some(Target::Hot(hk.keys[j].1));
        hk.keys[j].0 = old;
    }
    hk.fast_forward = key;
    (true, partner)
}

fn option_labels(item: Item, ctx: &SettingsCtx) -> Vec<String> {
    let s = |v: &[&str]| v.iter().map(|x| x.to_string()).collect::<Vec<_>>();
    match item {
        Item::Picture => s(&["1×", "2×", "3×", "4×", "Largest that fits"]),
        Item::Filter => s(&["None", "LCD grid", "Frame blend"]),
        Item::FfLimit => s(&["2×", "4×", "No limit"]),
        Item::Bios => s(&["Built-in", "From file…"]),
        Item::Audio => s(&["On", "Muted", "Off"]),
        Item::Pacing => s(&["Audio-clocked", "Wall clock"]),
        Item::Autosave => s(&["Every second", "Every 5 s", "Every 10 s"]),
        Item::Device => {
            let mut v = vec!["Default".to_string()];
            v.extend(ctx.audio_devices.iter().cloned());
            if let Some(d) = &ctx.settings.audio_device {
                if !ctx.audio_devices.contains(d) {
                    v.push(format!("{d} (not found)"));
                }
            }
            v
        }
        _ => Vec::new(),
    }
}

fn option_selected(item: Item, ctx: &SettingsCtx) -> Option<usize> {
    let st = &*ctx.settings;
    match item {
        Item::Picture => match st.picture {
            PictureSize::Fixed(n) if (1..=4).contains(&n) => Some(n as usize - 1),
            PictureSize::Fixed(_) => None,
            PictureSize::Largest => Some(4),
        },
        Item::Filter => Some(st.filter as usize),
        Item::FfLimit => Some(st.ff_limit as usize),
        Item::Bios => Some(matches!(st.bios, Bios::File(_)) as usize),
        Item::Audio => Some(st.audio as usize),
        Item::Pacing => Some(st.pacing as usize),
        Item::Autosave => match st.autosave_frames {
            60 => Some(0),
            300 => Some(1),
            600 => Some(2),
            _ => None,
        },
        Item::Device => match &st.audio_device {
            None => Some(0),
            Some(d) => Some(ctx.audio_devices.iter().position(|x| x == d).unwrap_or(ctx.audio_devices.len()) + 1),
        },
        _ => None,
    }
}

impl SettingsScreen {
    pub fn new() -> Self {
        SettingsScreen {
            spread: 0,
            focus: [0; SPREADS],
            waiting: None,
            flash: Vec::new(),
            text: None,
            bios_memory: PathBuf::new(),
            game_dir: None,
            figure: drawings::parse(crate::brand::HANDHELD_CONTROLS),
            hits: Vec::new(),
            tab_rects: [R::default(); SPREADS],
            page_rects: [R::default(); 2],
            scroll: [0.0; 2],
            content_h: [0.0; 2],
            cur_focus: None,
            focus_rect: None,
            ensure_visible: false,
            last_mouse: (-1.0, -1.0),
        }
    }

    /// Folder of the game being played (target of "Open folder"); `None` without a game.
    pub fn set_game_dir(&mut self, dir: Option<PathBuf>) {
        self.game_dir = dir;
    }

    /// Current spread (0 = Controls and Screen & sound, 1 = System, 2 = Saves).
    pub fn spread(&self) -> usize {
        self.spread
    }

    /// Forget any half-finished rebinding or typing (the app leaves the screen).
    pub fn reset(&mut self) {
        self.waiting = None;
        self.text = None;
        self.flash.clear();
    }

    // --- focus ---------------------------------------------------------------------------------

    fn items(&self, ctx: &SettingsCtx) -> Vec<Item> {
        let st = &*ctx.settings;
        match self.spread {
            0 => {
                // each GBA button: its keyboard keycap, then its pad keycap
                let mut v: Vec<Item> = [Keys::L, Keys::R, Keys::A, Keys::B, Keys::UP, Keys::DOWN, Keys::LEFT, Keys::RIGHT, Keys::START, Keys::SELECT].iter().flat_map(|b| [Item::Key(Target::Button(*b)), Item::Key(Target::Pad(*b))]).collect();
                v.push(Item::Key(Target::FastForward));
                v.extend(HOT_LEFT.iter().map(|(h, _)| Item::Key(Target::Hot(*h))));
                v.extend(HOT_RIGHT.iter().map(|(h, _)| Item::Key(Target::Hot(*h))));
                v.extend([Item::Picture, Item::Filter, Item::FfLimit, Item::Volume, Item::Bios]);
                if matches!(st.bios, Bios::File(_)) {
                    v.push(Item::BiosPath);
                }
                v
            }
            1 => {
                let mut v: Vec<Item> = (0..st.library_folders.len()).map(Item::Folder).collect();
                v.extend([Item::AddFolder, Item::Device, Item::Audio, Item::Pacing]);
                v
            }
            _ => vec![Item::Autosave, Item::OpenFolder],
        }
    }

    fn current_item(&self, ctx: &SettingsCtx) -> Option<Item> {
        let items = self.items(ctx);
        items.get(self.focus[self.spread].min(items.len().saturating_sub(1))).copied()
    }

    fn focus_on(&mut self, ctx: &SettingsCtx, item: Item) {
        if let Some(i) = self.items(ctx).iter().position(|x| *x == item) {
            if self.focus[self.spread] != i {
                self.focus[self.spread] = i;
                self.ensure_visible = true;
            }
        }
    }

    /// Open on spread `n` (0 Controls, 1 System, 2 Saves).
    pub fn show_spread(&mut self, n: usize) {
        self.set_spread(n);
    }

    fn set_spread(&mut self, n: usize) {
        let n = n.min(SPREADS - 1);
        if n != self.spread {
            self.spread = n;
            self.waiting = None;
            self.text = None;
            self.scroll = [0.0; 2];
            self.hits.clear();
        }
    }

    // --- editing -------------------------------------------------------------------------------

    fn flash_targets(&mut self, a: Target, b: Option<Target>) {
        let now = Instant::now();
        self.flash.push((a, now));
        if let Some(b) = b {
            self.flash.push((b, now));
        }
    }

    /// Bind `key` to `t`. Returns whether anything changed.
    fn bind(&mut self, t: Target, key: Key, ctx: &mut SettingsCtx) -> bool {
        match t {
            Target::Button(bit) => {
                if ctx.keymap.key_for(bit) == key {
                    return false;
                }
                let swapped = ctx.keymap.rebind(bit, key);
                if let Some(other) = swapped {
                    self.flash_targets(t, Some(Target::Button(other)));
                }
                ctx.settings.from_keymap(ctx.keymap);
                true
            }
            Target::Pad(_) => false,
            Target::Hot(h) => {
                let (changed, partner) = bind_hot(ctx.hotkeys, h, key);
                if partner.is_some() {
                    self.flash_targets(t, partner);
                }
                if changed {
                    ctx.settings.from_hotkeys(ctx.hotkeys);
                }
                changed
            }
            Target::FastForward => {
                let (changed, partner) = bind_fast_forward(ctx.hotkeys, key);
                if partner.is_some() {
                    self.flash_targets(t, partner);
                }
                if changed {
                    ctx.settings.from_hotkeys(ctx.hotkeys);
                }
                changed
            }
        }
    }

    /// Bind pad `button` to the GBA button of `t` (a `Target::Pad`). Returns whether anything
    /// changed.
    fn bind_pad(&mut self, t: Target, button: Button, ctx: &mut SettingsCtx) -> bool {
        let Target::Pad(bit) = t else { return false };
        if ctx.padmap.button_for(bit) == button {
            return false;
        }
        if let Some(other) = ctx.padmap.rebind(bit, button) {
            self.flash_targets(t, Some(Target::Pad(other)));
        }
        ctx.settings.from_padmap(ctx.padmap);
        true
    }

    /// Select option `i` of an option row. Returns whether the setting changed.
    fn set_option(&mut self, item: Item, i: usize, ctx: &mut SettingsCtx) -> bool {
        let before = ctx.settings.clone();
        let st = &mut *ctx.settings;
        match item {
            Item::Picture => st.picture = if i < 4 { PictureSize::Fixed(i as u32 + 1) } else { PictureSize::Largest },
            Item::Filter => st.filter = [Filter::None, Filter::LcdGrid, Filter::FrameBlend][i.min(2)],
            Item::FfLimit => st.ff_limit = [FfLimit::X2, FfLimit::X4, FfLimit::Unlimited][i.min(2)],
            Item::Bios => {
                if i == 0 {
                    if let Bios::File(p) = &st.bios {
                        self.bios_memory = p.clone();
                    }
                    st.bios = Bios::BuiltIn;
                } else if !matches!(st.bios, Bios::File(_)) {
                    st.bios = Bios::File(self.bios_memory.clone());
                }
            }
            Item::Audio => st.audio = [AudioMode::On, AudioMode::Muted, AudioMode::Off][i.min(2)],
            Item::Pacing => st.pacing = [Pacing::AudioClocked, Pacing::WallClock][i.min(1)],
            Item::Autosave => st.autosave_frames = [60, 300, 600][i.min(2)],
            Item::Device => {
                if i == 0 {
                    st.audio_device = None;
                } else if let Some(d) = ctx.audio_devices.get(i - 1) {
                    st.audio_device = Some(d.clone());
                }
            }
            _ => {}
        }
        *ctx.settings != before
    }

    /// Left/Right on an option row (`dir` = -1 / +1, clamped) or Confirm (`cycle`, wraps).
    fn step_option(&mut self, item: Item, dir: i32, cycle: bool, ctx: &mut SettingsCtx) -> bool {
        let n = option_labels(item, ctx).len();
        if n == 0 {
            return false;
        }
        let next = match option_selected(item, ctx) {
            Some(cur) if cycle => (cur + 1) % n,
            Some(cur) => (cur as i32 + dir).clamp(0, n as i32 - 1) as usize,
            None => {
                if dir < 0 {
                    n - 1
                } else {
                    0
                }
            }
        };
        self.set_option(item, next, ctx)
    }

    fn set_volume(&mut self, v: i32, ctx: &mut SettingsCtx) -> bool {
        let v = v.clamp(0, 10) as u8;
        let changed = ctx.settings.volume != v;
        ctx.settings.volume = v;
        changed
    }

    fn remove_folder(&mut self, i: usize, ctx: &mut SettingsCtx) -> bool {
        if i >= ctx.settings.library_folders.len() {
            return false;
        }
        ctx.settings.library_folders.remove(i);
        let n = self.items(ctx).len();
        self.focus[self.spread] = self.focus[self.spread].min(n.saturating_sub(1));
        true
    }

    fn start_text(&mut self, item: Item, ctx: &SettingsCtx) {
        let buf = match (item, &ctx.settings.bios) {
            (Item::BiosPath, Bios::File(p)) => p.to_string_lossy().into_owned(),
            _ => String::new(),
        };
        self.text = Some(TextEdit { item, buf });
    }

    /// Finish typing. Returns whether a setting changed.
    fn commit_text(&mut self, ctx: &mut SettingsCtx) -> bool {
        let Some(te) = self.text.take() else { return false };
        let path = te.buf.trim().trim_matches('"').trim().to_string();
        match te.item {
            Item::BiosPath => {
                let new = Bios::File(PathBuf::from(&path));
                self.bios_memory = PathBuf::from(&path);
                let changed = ctx.settings.bios != new;
                ctx.settings.bios = new;
                changed
            }
            Item::AddFolder => {
                if path.is_empty() {
                    return false;
                }
                let p = PathBuf::from(path);
                if ctx.settings.library_folders.contains(&p) {
                    return false;
                }
                ctx.settings.library_folders.push(p);
                // stay on the field, which moved down one row
                if let Some(i) = self.items(ctx).iter().position(|x| *x == Item::AddFolder) {
                    self.focus[self.spread] = i;
                }
                true
            }
            _ => false,
        }
    }

    fn open_folder_target(&self, ctx: &SettingsCtx) -> Option<PathBuf> {
        self.game_dir.clone().or_else(|| ctx.settings.library_folders.first().cloned())
    }

    /// Do what Confirm (or a click) does on `item`.
    fn activate(&mut self, item: Item, ctx: &mut SettingsCtx) -> SettingsOutcome {
        match item {
            Item::Key(t) => {
                self.waiting = Some(t);
                SettingsOutcome::None
            }
            Item::BiosPath | Item::AddFolder => {
                self.start_text(item, ctx);
                SettingsOutcome::None
            }
            Item::OpenFolder => match self.open_folder_target(ctx) {
                Some(p) => SettingsOutcome::OpenFolder(p),
                None => SettingsOutcome::None,
            },
            Item::Volume | Item::Folder(_) => SettingsOutcome::None,
            _ => changed(self.step_option(item, 1, true, ctx)),
        }
    }

    // --- input ---------------------------------------------------------------------------------

    /// React to this frame's input.
    pub fn update(&mut self, input: &UiInput, ctx: &mut SettingsCtx) -> SettingsOutcome {
        let now = Instant::now();
        self.flash.retain(|(_, at)| now.duration_since(*at) < FLASH);
        let out = self.update_inner(input, ctx);
        ctx.rebinding = self.waiting.is_some() || self.text.is_some();
        out
    }

    fn update_inner(&mut self, input: &UiInput, ctx: &mut SettingsCtx) -> SettingsOutcome {
        // waiting for the new key
        if let Some(t) = self.waiting {
            if input.clicked {
                self.waiting = None; // the click is handled below like any other
            } else {
                if input.pressed.contains(&Key::Escape) {
                    self.waiting = None;
                    return SettingsOutcome::None;
                }
                if let Target::Pad(_) = t {
                    // a pad keycap takes the next pad button; keyboard keys are not candidates
                    if let Some(&b) = input.pad_buttons.first() {
                        self.waiting = None;
                        return changed(self.bind_pad(t, b, ctx));
                    }
                } else if let Some(&k) = input.pressed.iter().find(|k| bindable(**k)) {
                    self.waiting = None;
                    return changed(self.bind(t, k, ctx));
                }
                return SettingsOutcome::None;
            }
        }
        // typing a path
        if self.text.is_some() {
            if input.clicked {
                let c = self.commit_text(ctx);
                let o = self.handle_click(input, ctx);
                return if o == SettingsOutcome::None { changed(c) } else { o };
            }
            if input.pressed(Key::Escape) {
                self.text = None;
                return SettingsOutcome::None;
            }
            if input.pressed(Key::Enter) || input.pressed(Key::NumPadEnter) {
                return changed(self.commit_text(ctx));
            }
            if let Some(te) = &mut self.text {
                if input.repeated(Key::Backspace) {
                    te.buf.pop();
                }
                te.buf.extend(input.chars.iter().filter(|c| !c.is_control()));
            }
            return SettingsOutcome::None;
        }

        // pointer: hover moves the focus, the wheel scrolls the page under it
        if input.mouse != self.last_mouse {
            self.last_mouse = input.mouse;
            let hovered = self.hits.iter().rev().find(|h| h.r.contains(input.mouse) && self.page_rects[h.page].contains(input.mouse)).map(|h| h.item);
            if let Some(item) = hovered {
                self.focus_on(ctx, item);
            }
        }
        if input.wheel != 0.0 {
            if let Some(p) = (0..2).find(|&p| self.page_rects[p].contains(input.mouse)) {
                let max = (self.content_h[p] - (self.page_rects[p].h - 34.0)).max(0.0);
                self.scroll[p] = (self.scroll[p] - input.wheel * 30.0).clamp(0.0, max);
            }
        }

        if input.clicked {
            let o = self.handle_click(input, ctx);
            if o != SettingsOutcome::None {
                return o;
            }
        }
        if input.pressed(Key::Escape) {
            return SettingsOutcome::Back;
        }

        let mut did_change = false;
        for a in &input.actions {
            let item = self.current_item(ctx);
            match a {
                Action::Back => return SettingsOutcome::Back,
                Action::NextPage => self.set_spread(self.spread + 1),
                Action::PrevPage => self.set_spread(self.spread.saturating_sub(1)),
                Action::Up | Action::Down => {
                    let n = self.items(ctx).len();
                    let f = &mut self.focus[self.spread];
                    let before = *f;
                    *f = if *a == Action::Up { f.saturating_sub(1) } else { (*f + 1).min(n.saturating_sub(1)) };
                    self.ensure_visible |= *f != before;
                }
                Action::Left | Action::Right => {
                    let dir = if *a == Action::Left { -1 } else { 1 };
                    match item {
                        Some(Item::Volume) => did_change |= self.set_volume(ctx.settings.volume as i32 + dir, ctx),
                        Some(it) if !option_labels(it, ctx).is_empty() => did_change |= self.step_option(it, dir, false, ctx),
                        _ => {}
                    }
                }
                Action::Confirm => {
                    if let Some(it) = item {
                        match self.activate(it, ctx) {
                            SettingsOutcome::None => {}
                            SettingsOutcome::Changed => did_change = true,
                            o => return o,
                        }
                        if self.waiting.is_some() || self.text.is_some() {
                            return changed(did_change);
                        }
                    }
                }
                Action::Start => {}
            }
        }
        if input.pressed(Key::Delete) {
            if let Some(Item::Folder(i)) = self.current_item(ctx) {
                did_change |= self.remove_folder(i, ctx);
            }
        }
        changed(did_change)
    }

    fn handle_click(&mut self, input: &UiInput, ctx: &mut SettingsCtx) -> SettingsOutcome {
        let m = input.mouse;
        if let Some(i) = self.tab_rects.iter().position(|r| r.contains(m)) {
            self.set_spread(i);
            return SettingsOutcome::None;
        }
        let Some(hit) = self.hits.iter().rev().find(|h| h.r.contains(m) && self.page_rects[h.page].contains(m)).copied() else {
            return SettingsOutcome::None;
        };
        self.focus_on(ctx, hit.item);
        match hit.kind {
            HitKind::Activate => self.activate(hit.item, ctx),
            HitKind::Option(i) => changed(self.set_option(hit.item, i, ctx)),
            HitKind::Ruler => {
                let v = ((m.0 - hit.r.x) / hit.r.w * 10.0).round() as i32;
                changed(self.set_volume(v, ctx))
            }
            HitKind::Remove => match hit.item {
                Item::Folder(i) => changed(self.remove_folder(i, ctx)),
                _ => SettingsOutcome::None,
            },
        }
    }
}

fn changed(b: bool) -> SettingsOutcome {
    if b {
        SettingsOutcome::Changed
    } else {
        SettingsOutcome::None
    }
}

// --- drawing -----------------------------------------------------------------------------------

/// The handheld line art, scaled by `s` about (x, y). At 1x this is `Canvas::drawing`.
fn draw_figure(cv: &mut Canvas, d: &Drawing, x: f32, y: f32, s: f32) {
    if (s - 1.0).abs() < 1e-3 {
        cv.drawing(d, x, y);
        return;
    }
    let t = Transform::from_row(s, 0.0, 0.0, s, x, y);
    for sh in &d.shapes {
        let path = match &sh.geom {
            Geometry::Rect { x, y, w, h, rx } => round_rect_path(*x, *y, *w, *h, [*rx; 4]),
            Geometry::Circle { cx, cy, r } => tiny_skia::PathBuilder::from_circle(*cx, *cy, *r),
            Geometry::Path(segs) => segs_to_path(segs),
        };
        let Some(path) = path else { continue };
        if let Some(fill) = sh.fill {
            cv.fill_path(&path, fill, t);
        }
        if let Some(st) = sh.stroke {
            cv.stroke_path(&path, sh.stroke_width, st, sh.round_cap, t);
        }
    }
}

/// `widgets::ruler` with whole-pixel ticks. The shared one puts 1 px ticks on half-pixel
/// positions, which trips a tiny-skia hairline assertion under a clip mask (the pages clip).
fn ruler(cv: &mut Canvas, x: f32, y: f32, w: f32, value: u8, pal: &Palette) -> f32 {
    cv.hrule(x, x + w, y + 8.0, 1.5, pal.fg);
    let st = Style::data(f::DATA_SMALL - 2.0).color(pal.fg2);
    for i in 0..=10 {
        let tx = x + w * i as f32 / 10.0;
        let th = if i % 5 == 0 { 8.0 } else { 5.0 };
        cv.vrule(tx.round(), y + 8.0 - th, y + 8.0, 1.0, pal.fg);
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
        cv.fill_path(&p, pal.spot, Transform::identity());
    }
    26.0
}

/// Page title in Display. Returns the height it takes including the gap below.
fn page_title(cv: &mut Canvas, x: f32, y: f32, text: &str) -> f32 {
    cv.text(x, y, text, &Style::display(f::DISPLAY_SCREEN).color(PAPER.fg));
    46.0
}

/// Text vertically centred on a keycap row of height 22 starting at `y`. Returns the width.
fn centred_text(cv: &mut Canvas, x: f32, y: f32, text: &str, st: &Style) -> f32 {
    let (asc, desc, _) = cv.fonts.line_metrics(st.face, st.size);
    cv.text_baseline(x, y + (c::KEY_H + asc - desc) / 2.0, text, st)
}

fn shade(pal: &Palette, a: f32) -> [u8; 3] {
    mix(pal.bg, [0, 0, 0], a)
}

impl SettingsScreen {
    /// Draw the whole screen (head, tabs, manual, foot) onto `cv` at its current size.
    pub fn render(&mut self, cv: &mut Canvas, ctx: &SettingsCtx) {
        let pal = &PAPER;
        let now = Instant::now();
        let (w, h) = (cv.width(), cv.height());
        cv.clear(pal.bg);
        self.hits.clear();
        self.focus_rect = None;
        self.cur_focus = self.current_item(ctx);

        // the desk
        let by0 = l::HEAD_H;
        let bh = h - l::FOOT_H - by0;
        cv.fill_rect(0.0, by0, w, bh, pal.bg2);
        let top = by0 + 18.0;
        let ph = if bh - 98.0 >= 560.0 { (bh - 98.0).min(720.0) } else { (bh - 34.0).max(200.0) };
        let (outer, gutter) = (60.0, 20.0);
        let pw = ((w - 2.0 * outer - gutter) / 2.0).floor().max(200.0);
        let rects = [R { x: outer, y: top, w: pw, h: ph }, R { x: outer + pw + gutter, y: top, w: pw, h: ph }];
        self.page_rects = rects;

        // shadows, pages, the spine
        for r in rects {
            for i in (1..=14).rev() {
                let a = 0.11 * (1.0 - i as f32 / 15.0).powi(2);
                let e = i as f32;
                cv.fill_rect(r.x - e, r.y - e + 5.0, r.w + 2.0 * e, r.h + 2.0 * e, mix(pal.bg2, [0, 0, 0], a));
            }
        }
        cv.fill_rect(rects[0].x + pw, top, gutter, ph, mix(pal.bg2, [0, 0, 0], 0.07));
        for r in rects {
            cv.fill_rect(r.x, r.y, r.w, r.h, pal.bg);
        }
        for d in 0..18 {
            let a = 0.10 * (1.0 - d as f32 / 18.0).powi(2);
            cv.fill_rect(rects[0].x + pw - 1.0 - d as f32, top, 1.0, ph, shade(pal, a));
            cv.fill_rect(rects[1].x + d as f32, top, 1.0, ph, shade(pal, a));
        }

        // pages
        for (p, r) in rects.into_iter().enumerate() {
            let view_h = r.h - 30.0;
            cv.set_clip(Some(Clip { x: r.x as i32, y: r.y as i32, w: r.w as i32, h: view_h as i32 }));
            let end = match (self.spread, p) {
                (0, 0) => self.page_controls(cv, ctx, r, now),
                (0, _) => self.page_screen_sound(cv, ctx, r),
                (1, 0) => self.page_library(cv, ctx, r),
                (1, _) => self.page_sound_timing(cv, ctx, r),
                (_, 0) => self.page_saves(cv, ctx, r),
                _ => self.page_autosave(cv, ctx, r),
            };
            cv.set_clip(None);
            self.content_h[p] = end + self.scroll[p] - r.y;
            let max = (self.content_h[p] - view_h).max(0.0);
            self.scroll[p] = self.scroll[p].clamp(0.0, max);
            if max > 0.0 {
                let th = (view_h * view_h / self.content_h[p]).max(24.0);
                let ty = r.y + 6.0 + (view_h - 12.0 - th) * self.scroll[p] / max;
                cv.fill_rect(r.x + r.w - 8.0, ty, 3.0, th, pal.line);
            }
            let num = format!("— {} —", self.spread * 2 + 4 + p);
            cv.text_center(r.x + r.w / 2.0, r.y + r.h - 28.0, &num, &Style::data(f::DATA_SMALL + 0.5).color(pal.fg2));
        }
        // keep the focused item on its page
        if self.ensure_visible {
            if let Some((p, fr)) = self.focus_rect {
                let r = rects[p];
                let max = (self.content_h[p] - (r.h - 30.0)).max(0.0);
                if fr.y < r.y + 8.0 {
                    self.scroll[p] = (self.scroll[p] - (r.y + 8.0 - fr.y)).clamp(0.0, max);
                } else if fr.y + fr.h > r.y + r.h - 40.0 {
                    self.scroll[p] = (self.scroll[p] + (fr.y + fr.h - (r.y + r.h - 40.0))).clamp(0.0, max);
                }
            }
            self.ensure_visible = false;
        }

        // head and tabs
        widgets::head(cv, "04", "Settings", None, None, pal);
        widgets::tabs(cv, &TABS, self.spread, pal);
        let st = Style::interface_bold(f::INTERFACE_BODY);
        let widths: Vec<f32> = TABS.iter().map(|t| cv.measure(t, &st)).collect();
        let mut tx = w - l::MARGIN - widths.iter().sum::<f32>() - 24.0 * (TABS.len() - 1) as f32;
        for (i, tw) in widths.iter().enumerate() {
            self.tab_rects[i] = R { x: tx - 6.0, y: 0.0, w: tw + 12.0, h: l::HEAD_H };
            tx += tw + 24.0;
        }

        // foot
        let kn = |bit: u16| key_name(ctx.keymap.key_for(bit));
        let (up, down, a, lk, rk, b) = (kn(Keys::UP), kn(Keys::DOWN), kn(Keys::A), kn(Keys::L), kn(Keys::R), kn(Keys::B));
        if self.waiting.is_some() || self.text.is_some() {
            widgets::foot(cv, &[], None, pal);
            let msg = match self.waiting {
                Some(Target::Pad(_)) => "Press a pad button · Esc cancels",
                Some(_) => "Press the new key · Esc cancels",
                None => "Type the path · Enter confirms · Esc cancels",
            };
            let st = Style::interface(f::INTERFACE_BODY).color(pal.fg);
            cv.text(l::MARGIN, h - l::FOOT_H + (l::FOOT_H - st.size * 1.2) / 2.0 + 1.0, msg, &st);
        } else {
            let (ud, pg) = ([up.as_str(), down.as_str()], [lk.as_str(), rk.as_str()]);
            let ak = [a.as_str()];
            widgets::foot(cv, &[(&ud[..], "Choose"), (&ak[..], "Change"), (&pg[..], "Turn the page")], Some((b.as_str(), "Back")), pal);
        }
    }

    // --- building blocks -----------------------------------------------------------------------

    fn is_focus(&self, item: Item) -> bool {
        self.cur_focus == Some(item)
    }

    /// The keycap text of `t` ("…" while it waits for its new binding).
    fn cap_label(&self, t: Target, ctx: &SettingsCtx) -> String {
        if self.waiting == Some(t) {
            "…".to_string()
        } else {
            target_label(t, ctx)
        }
    }

    /// Width of the keycap pair of GBA button `bit` (see [`Self::pair`]).
    fn pair_width(&self, cv: &mut Canvas, ctx: &SettingsCtx, bit: u16, stacked: bool) -> f32 {
        let wk = widgets::keycap_width(cv, &self.cap_label(Target::Button(bit), ctx));
        let wp = widgets::keycap_width(cv, &self.cap_label(Target::Pad(bit), ctx));
        if stacked {
            wk.max(wp)
        } else {
            wk + 4.0 + wp
        }
    }

    /// The keyboard keycap and the pad keycap of GBA button `bit`: side by side 4 px apart, or
    /// with the pad keycap 26 px under the keyboard one when `stacked`. `x` is the pair's left
    /// edge, or its right edge with `right`. Returns the pair's width.
    #[allow(clippy::too_many_arguments)]
    fn pair(&mut self, cv: &mut Canvas, ctx: &SettingsCtx, bit: u16, x: f32, y: f32, right: bool, stacked: bool, page: usize, now: Instant) -> f32 {
        let (kb, pad) = (Target::Button(bit), Target::Pad(bit));
        let wk = widgets::keycap_width(cv, &self.cap_label(kb, ctx));
        let wp = widgets::keycap_width(cv, &self.cap_label(pad, ctx));
        let w = self.pair_width(cv, ctx, bit, stacked);
        let x = if right { x - w } else { x };
        if stacked {
            // right-aligned caps end together, left-aligned ones start together
            let (xk, xp) = if right { (x + w - wk, x + w - wp) } else { (x, x) };
            self.cap(cv, ctx, kb, xk, y, false, page, now);
            self.cap(cv, ctx, pad, xp, y + 26.0, false, page, now);
        } else {
            self.cap(cv, ctx, kb, x, y, false, page, now);
            self.cap(cv, ctx, pad, x + wk + 4.0, y, false, page, now);
        }
        w
    }

    /// A keycap that rebinds `t`; `x` is its left edge, or its right edge with `right`.
    #[allow(clippy::too_many_arguments)]
    fn cap(&mut self, cv: &mut Canvas, ctx: &SettingsCtx, t: Target, x: f32, y: f32, right: bool, page: usize, now: Instant) -> f32 {
        let pal = &PAPER;
        let waiting = self.waiting == Some(t);
        let label = self.cap_label(t, ctx);
        let w = widgets::keycap_width(cv, &label);
        let x = if right { x - w } else { x };
        if self.flash.iter().any(|(ft, at)| *ft == t && now.duration_since(*at) < FLASH) {
            cv.round_rect(x - 3.0, y - 3.0, w + 6.0, c::KEY_H + 6.0, [6.0; 4], mix(pal.bg, pal.spot, 0.45));
        }
        let focused = self.is_focus(Item::Key(t));
        widgets::keycap(cv, x, y, &label, pal, waiting || focused);
        let r = R { x, y, w, h: c::KEY_H };
        self.hits.push(Hit { r, item: Item::Key(t), kind: HitKind::Activate, page });
        if focused {
            self.focus_rect = Some((page, r));
        }
        w
    }

    /// Start of a numbered item: rule, red numeral, title, description. Returns (top, y below).
    #[allow(clippy::too_many_arguments)]
    fn item_begin(&mut self, cv: &mut Canvas, x: f32, y: f32, w: f32, n: usize, title: &str, desc: &str) -> (f32, f32) {
        let pal = &PAPER;
        cv.hrule(x, x + w, y, 1.0, pal.line);
        // as `widgets::numbered_item`, with the mockup's tighter 1.34 line height
        cv.text(x, y + 8.0, &n.to_string(), &Style::interface_bold(f::INTERFACE_EMPHASIS).color(pal.spot));
        cv.text(x + 34.0, y + 9.0, title, &Style::interface_bold(f::INTERFACE_BODY).color(pal.fg).upper().tracking(0.4));
        let y1 = if desc.is_empty() {
            y + 8.0 + 22.0
        } else {
            let ds = Style::reading(f::READING_BODY).color(pal.fg2);
            let used = cv.paragraph(x + 34.0, y + 8.0 + 19.0, w - 34.0, desc, &ds, 1.34, 2);
            y + 8.0 + 19.0 + used + 5.0
        };
        (y, y1)
    }

    /// End of an item that `item` focuses: the red bar in the margin. Returns `end`.
    #[allow(clippy::too_many_arguments)]
    fn item_end(&mut self, cv: &mut Canvas, item: Item, x: f32, w: f32, top: f32, end: f32, page: usize) -> f32 {
        if self.is_focus(item) {
            cv.fill_rect(x - 14.0, top + 10.0, 3.0, end - top - 16.0, PAPER.spot);
            self.focus_rect = Some((page, R { x, y: top, w, h: end - top }));
        }
        end
    }

    /// Tick boxes for the options of `item`, wrapping at `w`. Returns the height used.
    #[allow(clippy::too_many_arguments)]
    fn options_row(&mut self, cv: &mut Canvas, ctx: &SettingsCtx, item: Item, x: f32, y: f32, w: f32, page: usize) -> f32 {
        let pal = &PAPER;
        let labels = option_labels(item, ctx);
        let sel = option_selected(item, ctx);
        let focused = self.is_focus(item);
        let st = Style::interface(f::INTERFACE_BODY);
        let (mut cx, mut cy) = (x, y);
        for (i, lab) in labels.iter().enumerate() {
            let need = c::TICK_SIZE + 8.0 + cv.measure(lab, &st);
            if cx > x && cx + need > x + w {
                cx = x;
                cy += 22.0;
            }
            let used = widgets::tick_option(cv, cx, cy, lab, sel == Some(i), pal, focused && sel == Some(i));
            self.hits.push(Hit { r: R { x: cx, y: cy, w: used, h: 18.0 }, item, kind: HitKind::Option(i), page });
            cx += used + 22.0;
        }
        cy + 18.0 - y
    }

    /// A numbered item whose body is a row of tick boxes.
    #[allow(clippy::too_many_arguments)]
    fn option_item(&mut self, cv: &mut Canvas, ctx: &SettingsCtx, item: Item, n: usize, title: &str, desc: &str, r: R, y: f32, page: usize) -> f32 {
        let (x, w) = Self::column(r, page);
        let (top, y1) = self.item_begin(cv, x, y, w, n, title, desc);
        let hh = self.options_row(cv, ctx, item, x + 34.0, y1, w - 34.0, page);
        self.item_end(cv, item, x, w, top, y1 + hh + 8.0, page)
    }

    /// Left edge and width of the content column of page `p`.
    fn column(r: R, page: usize) -> (f32, f32) {
        if page == 0 {
            (r.x + 38.0, r.w - 66.0)
        } else {
            (r.x + 28.0, r.w - 66.0)
        }
    }

    // --- spread 0 ------------------------------------------------------------------------------

    fn page_controls(&mut self, cv: &mut Canvas, ctx: &SettingsCtx, r: R, now: Instant) -> f32 {
        let pal = &PAPER;
        let (x0, cw) = Self::column(r, 0);
        let mut y = r.y + 24.0 - self.scroll[0];
        y += page_title(cv, x0, y, "Controls");
        let reading = Style::reading(f::READING_BODY).color(pal.fg2);
        y += cv.paragraph(x0, y, cw.min(420.0), "Click a keycap, then press the new key or pad button. The second keycap of every GBA button is the gamepad's; the left stick is always a D-pad.", &reading, f::READING_LINE_HEIGHT, 3) + 2.0;

        // the handheld with its callouts; the line art shrinks until the right-hand callouts
        // (keycap pair, label) fit the column, and narrow pages stack each pad keycap under its
        // keyboard keycap
        const RIGHT: [(u16, &str); 5] = [(Keys::R, "R Button"), (Keys::A, "A Button"), (Keys::B, "B Button"), (Keys::START, "START"), (Keys::SELECT, "SELECT")];
        let label = Style::interface_bold(f::INTERFACE_BODY).color(pal.fg);
        let budget = RIGHT.iter().map(|(bit, name)| self.pair_width(cv, ctx, *bit, false) + 8.0 + cv.measure(name, &label)).fold(0.0, f32::max);
        let s = ((cw - budget) / 378.0).clamp(0.5, 1.0);
        let stacked = s < 0.75;
        let dy = y - 6.0 + if stacked { 20.0 } else { 0.0 };
        draw_figure(cv, &self.figure, x0, dy, s);
        let ry = |v: f32| dy + v * s;
        let pitch = if stacked { 50.0 } else { 24.0 };
        let mut right_y = [20.0, 100.0, 142.0, 184.0, 222.0].map(ry);
        for i in 1..right_y.len() {
            right_y[i] = right_y[i].max(right_y[i - 1] + pitch);
        }
        let lx = x0 + 126.0 * s;
        let rx = x0 + 378.0 * s;
        // left: the L button's keycaps end at the leader, the label before them (above them when
        // stacked)
        let wl = self.pair(cv, ctx, Keys::L, lx, ry(20.0), true, stacked, 0, now);
        let tw = cv.measure("L Button", &label);
        if stacked {
            centred_text(cv, lx - tw, ry(20.0) - 24.0, "L Button", &label);
        } else {
            centred_text(cv, lx - wl - 8.0 - tw, ry(20.0), "L Button", &label);
        }
        // control pad: label above, the four keyboard arrows in a row ending at the leader, the
        // pad's four directions in a second row under them (two keycaps per row when stacked)
        let ky = ry(146.0).max(ry(20.0) + if stacked { 80.0 } else { 70.0 });
        let arrows = [Keys::UP, Keys::DOWN, Keys::LEFT, Keys::RIGHT];
        let per_row = if stacked { 2 } else { 4 };
        let mut row_y = ky;
        for pad in [false, true] {
            for chunk in arrows.chunks(per_row) {
                let targets: Vec<Target> = chunk.iter().map(|b| if pad { Target::Pad(*b) } else { Target::Button(*b) }).collect();
                let widths: Vec<f32> = targets.iter().map(|t| widgets::keycap_width(cv, &self.cap_label(*t, ctx))).collect();
                let mut ax = lx - widths.iter().sum::<f32>() - 4.0 * (widths.len() - 1) as f32;
                for (t, wd) in targets.iter().zip(&widths) {
                    self.cap(cv, ctx, *t, ax, row_y, false, 0, now);
                    ax += wd + 4.0;
                }
                row_y += 26.0;
            }
        }
        let arrows_bottom = row_y - 26.0 + c::KEY_H;
        let tw = cv.measure("+Control Pad", &label);
        centred_text(cv, lx - tw, ky - 24.0, "+Control Pad", &label);
        // right: the keycap pair then the label
        for (i, (bit, name)) in RIGHT.into_iter().enumerate() {
            let cwid = self.pair(cv, ctx, bit, rx, right_y[i], false, stacked, 0, now);
            centred_text(cv, rx + cwid + 8.0, right_y[i], name, &label);
        }
        let pair_h = if stacked { 26.0 + c::KEY_H } else { c::KEY_H };
        y = (right_y[4] + pair_h).max(arrows_bottom) + 18.0;

        // emulator keys
        widgets::label(cv, x0, y, "Emulator keys", pal);
        y += 24.0;
        let col_w = ((cw - 32.0) / 2.0).floor();
        let left: Vec<(Target, &str)> = std::iter::once((Target::FastForward, "Fast-forward (hold)")).chain(HOT_LEFT.iter().map(|(h, n)| (Target::Hot(*h), *n))).collect();
        let right: Vec<(Target, &str)> = HOT_RIGHT.iter().map(|(h, n)| (Target::Hot(*h), *n)).collect();
        for (col, rows) in [left, right].iter().enumerate() {
            let cx = x0 + col as f32 * (col_w + 32.0);
            for (i, (t, name)) in rows.iter().enumerate() {
                let row_y = y + i as f32 * 26.0;
                let kw = self.cap(cv, ctx, *t, cx + col_w, row_y, true, 0, now);
                let tw = centred_text(cv, cx, row_y, name, &Style::reading(f::READING_BODY).color(pal.fg));
                cv.dotted_line(cx + tw + 8.0, cx + col_w - kw - 8.0, row_y + 15.0, pal.fg2);
            }
        }
        y += (1 + HOT_LEFT.len()).max(HOT_RIGHT.len()) as f32 * 26.0 + 4.0;
        y + 6.0
    }

    fn page_screen_sound(&mut self, cv: &mut Canvas, ctx: &SettingsCtx, r: R) -> f32 {
        let pal = &PAPER;
        let (x, w) = Self::column(r, 1);
        let mut y = r.y + 24.0 - self.scroll[1];
        y += page_title(cv, x, y, "Screen & sound") - 4.0;
        y = self.option_item(cv, ctx, Item::Picture, 1, "Picture size", "The screen is 240 × 160 dots. Whole-number sizes keep every dot square.", r, y, 1);
        y = self.option_item(cv, ctx, Item::Filter, 2, "Picture filter", "LCD grid draws the gaps between dots; frame blend imitates the slow original screen.", r, y, 1);
        let ff_desc = format!("Normally the sound sets the pace. Holding {} runs up to this speed.", key_name(ctx.hotkeys.fast_forward));
        y = self.option_item(cv, ctx, Item::FfLimit, 3, "Fast-forward limit", &ff_desc, r, y, 1);

        // volume: a ruler
        let (top, y1) = self.item_begin(cv, x, y, w, 4, "Volume", "");
        let rw = (w - 34.0 - 12.0).min(360.0);
        let rh = ruler(cv, x + 34.0, y1 + 4.0, rw, ctx.settings.volume, pal);
        self.hits.push(Hit { r: R { x: x + 34.0 - 6.0, y: y1, w: rw + 12.0, h: rh + 8.0 }, item: Item::Volume, kind: HitKind::Ruler, page: 1 });
        y = self.item_end(cv, Item::Volume, x, w, top, y1 + 4.0 + rh + 12.0, 1);

        // BIOS, with a path field when a file is chosen
        let (top, y1) = self.item_begin(cv, x, y, w, 5, "System program (BIOS)", "The built-in one runs every game in the library. To use a dump, set GBA_BIOS.");
        let hh = self.options_row(cv, ctx, Item::Bios, x + 34.0, y1, w - 34.0, 1);
        let mut end = y1 + hh + 8.0;
        if let Bios::File(p) = &ctx.settings.bios {
            let editing = self.text.as_ref().filter(|t| t.item == Item::BiosPath);
            let value = editing.map(|t| t.buf.clone()).unwrap_or_else(|| p.to_string_lossy().into_owned());
            let fy = end + 2.0;
            widgets::underline_field(cv, x + 34.0, fy, w - 34.0, "File", &value, "Path to a 16 KiB BIOS image", editing.is_some() || self.is_focus(Item::BiosPath), pal);
            self.hits.push(Hit { r: R { x: x + 34.0, y: fy - 2.0, w: w - 34.0, h: 26.0 }, item: Item::BiosPath, kind: HitKind::Activate, page: 1 });
            end = fy + 32.0;
        }
        if self.is_focus(Item::Bios) || self.is_focus(Item::BiosPath) {
            cv.fill_rect(x - 14.0, top + 10.0, 3.0, end - top - 16.0, pal.spot);
            self.focus_rect = Some((1, R { x, y: top, w, h: end - top }));
        }
        y = end;
        cv.hrule(x, x + w, y, 1.0, pal.line);
        y += 14.0;
        y + self.note_box(cv, ctx, x, y, w) + 12.0
    }

    /// The NOTE box about `.sav` versus save states, with the F5 keycap inline.
    fn note_box(&mut self, cv: &mut Canvas, ctx: &SettingsCtx, x: f32, y: f32, w: f32) -> f32 {
        let pal = &PAPER;
        let st = Style::reading(f::READING_BODY).color(pal.fg);
        let key = hot_key(ctx.hotkeys, Hotkey::SaveNow).map(key_name).unwrap_or_else(|| "F5".into());
        let kw = widgets::keycap_width(cv, &key);
        let lh = f::READING_BODY * f::READING_LINE_HEIGHT;
        let label_w = 60.0;
        let (ix, iw) = (x + label_w + 14.0, w - label_w - 28.0);
        // words and the keycap, flowed to the width
        let mut toks: Vec<(Option<String>, bool)> = Vec::new();
        for wd in "Game saves (.sav) are written beside the game when you quit or press".split_whitespace() {
            toks.push((Some(wd.to_string()), false));
        }
        toks.push((None, false));
        for (i, wd) in ". Save states belong to this emulator only.".split_whitespace().enumerate() {
            toks.push((Some(wd.to_string()), i == 0));
        }
        let space = cv.measure(" ", &st);
        let (mut pen, mut line) = (0.0f32, 0usize);
        let mut placed = Vec::new();
        for (tok, glue) in &toks {
            let tw = match tok {
                Some(t) => cv.measure(t, &st),
                None => kw,
            };
            let gap = if pen == 0.0 || *glue { 0.0 } else { space };
            if pen > 0.0 && pen + gap + tw > iw {
                pen = 0.0;
                line += 1;
            }
            let px = if pen == 0.0 { 0.0 } else { pen + gap };
            placed.push((px, line, tok.clone()));
            pen = px + tw;
        }
        let h = ((line + 1) as f32 * lh + 26.0).max(58.0);
        cv.fill_rect(x, y, label_w, h, pal.fg);
        let ls = Style::label().color(pal.bg);
        let lw = cv.measure("Note", &ls);
        cv.text(x + (label_w - lw) / 2.0, y + (h - ls.size * 1.2) / 2.0, "Note", &ls);
        cv.fill_rect(x + label_w, y, w - label_w, 1.5, pal.fg);
        cv.fill_rect(x + label_w, y + h - 1.5, w - label_w, 1.5, pal.fg);
        cv.fill_rect(x + w - 1.5, y, 1.5, h, pal.fg);
        for (px, ln, tok) in placed {
            let ty = y + 13.0 + ln as f32 * lh;
            match tok {
                Some(t) => {
                    cv.text(ix + px, ty, &t, &st);
                }
                None => {
                    widgets::keycap(cv, ix + px, ty - (c::KEY_H - lh) / 2.0, &key, pal, false);
                }
            }
        }
        h
    }

    // --- spread 1 ------------------------------------------------------------------------------

    fn page_library(&mut self, cv: &mut Canvas, ctx: &SettingsCtx, r: R) -> f32 {
        let pal = &PAPER;
        let (x, w) = Self::column(r, 0);
        let mut y = r.y + 24.0 - self.scroll[0];
        y += page_title(cv, x, y, "Library") - 4.0;
        let (_, mut y1) = self.item_begin(cv, x, y, w, 1, "Library folders", "Games in these folders appear in the library. The folder of a game opened from the command line is always included.");
        let fx = x + 34.0;
        let fw = w - 34.0;
        if ctx.settings.library_folders.is_empty() {
            cv.text(fx, y1, "No folders added yet.", &Style::reading(f::READING_BODY).color(pal.fg2));
            y1 += 26.0;
        }
        for (i, p) in ctx.settings.library_folders.iter().enumerate() {
            let focused = self.is_focus(Item::Folder(i));
            let st = Style::data(f::DATA_BODY).color(if focused { pal.spot } else { pal.fg });
            let cap_w = widgets::keycap_width(cv, "Del");
            let shown = cv.fonts.ellipsize_middle(&st, &p.to_string_lossy(), fw - cap_w - 16.0);
            centred_text(cv, fx, y1, &shown, &st);
            widgets::keycap(cv, fx + fw - cap_w, y1, "Del", pal, focused);
            self.hits.push(Hit { r: R { x: fx, y: y1, w: fw - cap_w - 8.0, h: c::KEY_H }, item: Item::Folder(i), kind: HitKind::Activate, page: 0 });
            self.hits.push(Hit { r: R { x: fx + fw - cap_w, y: y1, w: cap_w, h: c::KEY_H }, item: Item::Folder(i), kind: HitKind::Remove, page: 0 });
            if focused {
                self.focus_rect = Some((0, R { x: fx, y: y1, w: fw, h: c::KEY_H }));
            }
            cv.hrule(fx, fx + fw, y1 + c::KEY_H + 3.0, 1.0, pal.line);
            y1 += 30.0;
        }
        y1 += 6.0;
        let editing = self.text.as_ref().filter(|t| t.item == Item::AddFolder);
        let value = editing.map(|t| t.buf.as_str()).unwrap_or("");
        widgets::underline_field(cv, fx, y1, fw, "Add folder", value, "C:\\Games", editing.is_some() || self.is_focus(Item::AddFolder), pal);
        self.hits.push(Hit { r: R { x: fx, y: y1 - 2.0, w: fw, h: 26.0 }, item: Item::AddFolder, kind: HitKind::Activate, page: 0 });
        if self.is_focus(Item::AddFolder) {
            self.focus_rect = Some((0, R { x: fx, y: y1 - 2.0, w: fw, h: 26.0 }));
        }
        y1 + 46.0
    }

    fn page_sound_timing(&mut self, cv: &mut Canvas, ctx: &SettingsCtx, r: R) -> f32 {
        let (x, _) = Self::column(r, 1);
        let mut y = r.y + 24.0 - self.scroll[1];
        y += page_title(cv, x, y, "Sound & timing") - 4.0;
        y = self.option_item(cv, ctx, Item::Device, 2, "Audio device", "Where the sound goes. Default follows the system's choice.", r, y, 1);
        y = self.option_item(cv, ctx, Item::Audio, 3, "Audio", "On plays sound. Muted keeps the device open but plays silence. Off opens no device.", r, y, 1);
        y = self.option_item(cv, ctx, Item::Pacing, 4, "Pacing", "Audio-clocked keeps sound and picture in step; wall clock ignores the sound buffer.", r, y, 1);
        y + 12.0
    }

    // --- spread 2 ------------------------------------------------------------------------------

    fn page_saves(&mut self, cv: &mut Canvas, ctx: &SettingsCtx, r: R) -> f32 {
        let pal = &PAPER;
        let (x, w) = Self::column(r, 0);
        let mut y = r.y + 24.0 - self.scroll[0];
        y += page_title(cv, x, y, "Saves") - 4.0;
        let (_, y1) = self.item_begin(cv, x, y, w, 1, "Where things go", "Game saves (.sav) and save states (.s1.state … .s8.state) are written beside the ROM.");
        let here = match (&self.game_dir, ctx.settings.library_folders.first()) {
            (Some(d), _) => d.to_string_lossy().into_owned(),
            (None, Some(d)) => d.to_string_lossy().into_owned(),
            (None, None) => "Beside each game file".to_string(),
        };
        widgets::label(cv, x + 34.0, y1 + 3.0, if self.game_dir.is_some() { "This game" } else { "Library" }, pal);
        let st = Style::data(f::DATA_BODY).color(pal.fg);
        let shown = cv.fonts.ellipsize_middle(&st, &here, w - 34.0 - 100.0);
        cv.text(x + 34.0 + 100.0, y1, &shown, &st);
        y1 + 36.0
    }

    fn page_autosave(&mut self, cv: &mut Canvas, ctx: &SettingsCtx, r: R) -> f32 {
        let pal = &PAPER;
        let (x, w) = Self::column(r, 1);
        let mut y = r.y + 24.0 - self.scroll[1];
        y += page_title(cv, x, y, "Autosave") - 4.0;
        y = self.option_item(cv, ctx, Item::Autosave, 2, "Autosave", "The .sav is written this often while it changes, and always on quit.", r, y, 1);
        let (top, y1) = self.item_begin(cv, x, y, w, 3, "Look at the files", "Show the folder of the current game, or the first library folder.");
        let bw = widgets::button_width(cv, "Open folder", None).max(140.0);
        widgets::button_secondary(cv, x + 34.0, y1 + 2.0, bw, "Open folder", pal);
        self.hits.push(Hit { r: R { x: x + 34.0, y: y1 + 2.0, w: bw, h: c::BUTTON_H }, item: Item::OpenFolder, kind: HitKind::Activate, page: 1 });
        let end = y1 + c::BUTTON_H + 14.0;
        self.item_end(cv, Item::OpenFolder, x, w, top, end, 1) + 12.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::text::Fonts;

    struct Fix {
        s: Settings,
        km: KeyMap,
        pm: PadMap,
        hk: HotkeyMap,
        devices: Vec<String>,
    }

    impl Fix {
        fn new() -> Self {
            Fix { s: Settings::default(), km: crate::input::default_keymap(), pm: crate::input::default_padmap(), hk: HotkeyMap::default(), devices: vec!["Speakers".into(), "Headphones".into()] }
        }
        fn ctx(&mut self) -> SettingsCtx<'_> {
            SettingsCtx { settings: &mut self.s, keymap: &mut self.km, padmap: &mut self.pm, hotkeys: &mut self.hk, audio_devices: &self.devices, rebinding: false }
        }
    }

    fn act(a: Action) -> UiInput {
        UiInput { actions: vec![a], ..UiInput::default() }
    }

    fn press(k: Key) -> UiInput {
        UiInput { pressed: vec![k], repeated: vec![k], ..UiInput::default() }
    }

    fn pad(b: Button) -> UiInput {
        UiInput { pad_buttons: vec![b], ..UiInput::default() }
    }

    fn typed(s: &str) -> UiInput {
        UiInput { chars: s.chars().collect(), ..UiInput::default() }
    }

    fn goto(scr: &mut SettingsScreen, fx: &mut Fix, item: Item) {
        let ctx = fx.ctx();
        let i = scr.items(&ctx).iter().position(|x| *x == item).expect("item on this spread");
        scr.focus[scr.spread] = i;
    }

    #[test]
    fn focus_order_and_page_turning() {
        let mut fx = Fix::new();
        let mut scr = SettingsScreen::new();
        let mut ctx = fx.ctx();
        assert_eq!(scr.current_item(&ctx), Some(Item::Key(Target::Button(Keys::L))));
        scr.update(&act(Action::Down), &mut ctx);
        assert_eq!(scr.current_item(&ctx), Some(Item::Key(Target::Pad(Keys::L))), "the pad keycap follows the keyboard one");
        scr.update(&act(Action::Down), &mut ctx);
        assert_eq!(scr.current_item(&ctx), Some(Item::Key(Target::Button(Keys::R))));
        for _ in 0..3 {
            scr.update(&act(Action::Up), &mut ctx);
        }
        assert_eq!(scr.focus[0], 0, "clamped at the top");
        // left page first (10 buttons x keyboard + pad, fast-forward, 10 emulator keys), then the
        // right page
        let items = scr.items(&ctx);
        assert_eq!(items.len(), 20 + 1 + 10 + 5);
        assert_eq!(items[31], Item::Picture);
        assert_eq!(*items.last().unwrap(), Item::Bios);
        for _ in 0..31 {
            scr.update(&act(Action::Down), &mut ctx);
        }
        assert_eq!(scr.current_item(&ctx), Some(Item::Picture));
        scr.update(&act(Action::Down), &mut ctx);
        assert_eq!(scr.current_item(&ctx), Some(Item::Filter));

        // pages
        assert_eq!(scr.spread(), 0);
        scr.update(&act(Action::NextPage), &mut ctx);
        assert_eq!(scr.spread(), 1);
        assert_eq!(scr.current_item(&ctx), Some(Item::AddFolder), "no folders yet");
        scr.update(&act(Action::NextPage), &mut ctx);
        assert_eq!(scr.spread(), 2);
        scr.update(&act(Action::NextPage), &mut ctx);
        assert_eq!(scr.spread(), 2, "the last spread stays");
        assert_eq!(scr.items(&ctx), vec![Item::Autosave, Item::OpenFolder]);
        scr.update(&act(Action::PrevPage), &mut ctx);
        scr.update(&act(Action::PrevPage), &mut ctx);
        assert_eq!(scr.spread(), 0);
        assert_eq!(scr.current_item(&ctx), Some(Item::Filter), "each spread keeps its focus");
        assert_eq!(scr.update(&act(Action::Back), &mut ctx), SettingsOutcome::Back);
        assert_eq!(scr.update(&press(Key::Escape), &mut ctx), SettingsOutcome::Back);
    }

    #[test]
    fn rebinding_flow_with_swap() {
        let mut fx = Fix::new();
        let mut scr = SettingsScreen::new();
        let mut ctx = fx.ctx();
        // L: Confirm, then the next key
        assert_eq!(scr.update(&act(Action::Confirm), &mut ctx), SettingsOutcome::None);
        assert!(ctx.rebinding, "the app must not treat keys as hotkeys now");
        assert_eq!(scr.waiting, Some(Target::Button(Keys::L)));
        assert_eq!(scr.update(&press(Key::Q), &mut ctx), SettingsOutcome::Changed);
        assert!(!ctx.rebinding);
        assert_eq!(ctx.keymap.key_for(Keys::L), Key::Q);
        assert_eq!(ctx.settings.keymap.0["L"], "Q", "the settings follow the live map");
        assert!(scr.flash.is_empty(), "no swap, no flash");

        // Escape cancels and is not "Back"
        scr.update(&act(Action::Confirm), &mut ctx);
        assert_eq!(scr.update(&press(Key::Escape), &mut ctx), SettingsOutcome::None);
        assert!(!ctx.rebinding);
        assert_eq!(ctx.keymap.key_for(Keys::L), Key::Q);
        // confirm keys typed while waiting are not actions
        scr.update(&act(Action::Confirm), &mut ctx);
        assert_eq!(scr.update(&UiInput { actions: vec![Action::Back, Action::Down], ..UiInput::default() }, &mut ctx), SettingsOutcome::None);
        assert!(ctx.rebinding);
        scr.update(&press(Key::Escape), &mut ctx);

        // A button: X is B's key, so they swap and both keycaps flash
        for _ in 0..4 {
            scr.update(&act(Action::Down), &mut ctx);
        }
        assert_eq!(scr.current_item(&ctx), Some(Item::Key(Target::Button(Keys::A))));
        scr.update(&act(Action::Confirm), &mut ctx);
        assert_eq!(scr.update(&press(Key::X), &mut ctx), SettingsOutcome::Changed);
        assert_eq!(ctx.keymap.key_for(Keys::A), Key::X);
        assert_eq!(ctx.keymap.key_for(Keys::B), Key::Z);
        assert_eq!(ctx.settings.keymap.0["A"], "X");
        assert_eq!(ctx.settings.keymap.0["B"], "Z");
        let flashed: Vec<Target> = scr.flash.iter().map(|(t, _)| *t).collect();
        assert!(flashed.contains(&Target::Button(Keys::A)) && flashed.contains(&Target::Button(Keys::B)));
        // binding the key it already has changes nothing
        scr.update(&act(Action::Confirm), &mut ctx);
        assert_eq!(scr.update(&press(Key::X), &mut ctx), SettingsOutcome::None);
    }

    #[test]
    fn pad_rebinding_flow_with_swap() {
        let mut fx = Fix::new();
        let mut scr = SettingsScreen::new();
        let mut ctx = fx.ctx();
        // the pad keycap of L: Confirm, then a pad button
        scr.update(&act(Action::Down), &mut ctx);
        assert_eq!(scr.current_item(&ctx), Some(Item::Key(Target::Pad(Keys::L))));
        scr.update(&act(Action::Confirm), &mut ctx);
        assert!(ctx.rebinding);
        assert_eq!(scr.waiting, Some(Target::Pad(Keys::L)));
        // keyboard keys are not pad buttons
        assert_eq!(scr.update(&press(Key::Q), &mut ctx), SettingsOutcome::None);
        assert!(ctx.rebinding, "still waiting");
        assert_eq!(ctx.keymap.key_for(Keys::L), Key::A, "the keyboard map is untouched");
        assert_eq!(scr.update(&pad(Button::LeftTrigger2), &mut ctx), SettingsOutcome::Changed);
        assert!(!ctx.rebinding);
        assert_eq!(ctx.padmap.button_for(Keys::L), Button::LeftTrigger2);
        assert_eq!(ctx.settings.gamepad.0["L"], "LT", "the settings follow the live map");
        assert!(scr.flash.is_empty(), "no swap, no flash");
        // the button it already has changes nothing
        scr.update(&act(Action::Confirm), &mut ctx);
        assert_eq!(scr.update(&pad(Button::LeftTrigger2), &mut ctx), SettingsOutcome::None);
        // Esc cancels
        scr.update(&act(Action::Confirm), &mut ctx);
        assert_eq!(scr.update(&press(Key::Escape), &mut ctx), SettingsOutcome::None);
        assert!(!ctx.rebinding);
        assert_eq!(ctx.padmap.button_for(Keys::L), Button::LeftTrigger2);
        // East is B's button: binding it to L swaps, and both pad keycaps flash
        scr.update(&act(Action::Confirm), &mut ctx);
        assert_eq!(scr.update(&pad(Button::East), &mut ctx), SettingsOutcome::Changed);
        assert_eq!(ctx.padmap.button_for(Keys::L), Button::East);
        assert_eq!(ctx.padmap.button_for(Keys::B), Button::LeftTrigger2);
        assert_eq!(ctx.settings.gamepad.0["B"], "LT");
        let flashed: Vec<Target> = scr.flash.iter().map(|(t, _)| *t).collect();
        assert!(flashed.contains(&Target::Pad(Keys::L)) && flashed.contains(&Target::Pad(Keys::B)));
        // a keyboard rebind ignores pad buttons
        scr.update(&act(Action::Up), &mut ctx);
        assert_eq!(scr.current_item(&ctx), Some(Item::Key(Target::Button(Keys::L))));
        scr.update(&act(Action::Confirm), &mut ctx);
        assert_eq!(scr.update(&pad(Button::North), &mut ctx), SettingsOutcome::None);
        assert!(ctx.rebinding);
        assert_eq!(ctx.padmap.button_for(Keys::L), Button::East);
        assert_eq!(scr.update(&press(Key::Q), &mut ctx), SettingsOutcome::Changed);
        assert_eq!(ctx.keymap.key_for(Keys::L), Key::Q);
    }

    #[test]
    fn hotkey_rebinding_and_swap() {
        let mut fx = Fix::new();
        let mut scr = SettingsScreen::new();
        let mut ctx = fx.ctx();
        let i = scr.items(&ctx).iter().position(|x| *x == Item::Key(Target::FastForward)).unwrap();
        scr.focus[0] = i;
        scr.update(&act(Action::Confirm), &mut ctx);
        assert_eq!(scr.update(&press(Key::Q), &mut ctx), SettingsOutcome::Changed);
        assert_eq!(ctx.hotkeys.fast_forward, Key::Q);
        assert_eq!(ctx.settings.hotkeys.fast_forward, "Q");

        // Pause is P (Esc also pauses); give it Tab's... a key used by fast-forward swaps
        let i = scr.items(&ctx).iter().position(|x| *x == Item::Key(Target::Hot(Hotkey::Pause))).unwrap();
        scr.focus[0] = i;
        scr.update(&act(Action::Confirm), &mut ctx);
        assert_eq!(scr.update(&press(Key::Q), &mut ctx), SettingsOutcome::Changed);
        assert_eq!(ctx.hotkeys.fast_forward, Key::P, "fast-forward takes pause's old key");
        assert_eq!(hot_key(ctx.hotkeys, Hotkey::Pause), Some(Key::Q));
        assert!(ctx.hotkeys.keys.contains(&(Key::Escape, Hotkey::Pause)), "Esc still pauses");
        assert_eq!(ctx.settings.hotkeys.keys["pause"], "Q");

        // another hotkey's key swaps with it
        let i = scr.items(&ctx).iter().position(|x| *x == Item::Key(Target::Hot(Hotkey::Screenshot))).unwrap();
        scr.focus[0] = i;
        scr.update(&act(Action::Confirm), &mut ctx);
        assert_eq!(scr.update(&press(Key::F12), &mut ctx), SettingsOutcome::Changed);
        assert_eq!(hot_key(ctx.hotkeys, Hotkey::Screenshot), Some(Key::F12));
        assert_eq!(hot_key(ctx.hotkeys, Hotkey::Bench), Some(Key::F11));
        assert_eq!(ctx.hotkeys.key_for(Hotkey::Bench), Some(Key::F11));
        assert!(!scr.flash.is_empty());
    }

    #[test]
    fn option_cycling_writes_into_settings() {
        let mut fx = Fix::new();
        let mut scr = SettingsScreen::new();
        goto(&mut scr, &mut fx, Item::Picture);
        let mut ctx = fx.ctx();
        // default Largest: Confirm wraps to 1x
        assert_eq!(scr.update(&act(Action::Confirm), &mut ctx), SettingsOutcome::Changed);
        assert_eq!(ctx.settings.picture, PictureSize::Fixed(1));
        assert_eq!(scr.update(&act(Action::Right), &mut ctx), SettingsOutcome::Changed);
        assert_eq!(ctx.settings.picture, PictureSize::Fixed(2));
        scr.update(&act(Action::Left), &mut ctx);
        assert_eq!(scr.update(&act(Action::Left), &mut ctx), SettingsOutcome::None, "clamped at the first option");
        assert_eq!(ctx.settings.picture, PictureSize::Fixed(1));
        for _ in 0..9 {
            scr.update(&act(Action::Right), &mut ctx);
        }
        assert_eq!(ctx.settings.picture, PictureSize::Largest);

        let i = scr.items(&ctx).iter().position(|x| *x == Item::Filter).unwrap();
        scr.focus[0] = i;
        scr.update(&act(Action::Right), &mut ctx);
        assert_eq!(ctx.settings.filter, Filter::LcdGrid);
        scr.update(&act(Action::Confirm), &mut ctx);
        assert_eq!(ctx.settings.filter, Filter::FrameBlend);
        scr.update(&act(Action::Confirm), &mut ctx);
        assert_eq!(ctx.settings.filter, Filter::None, "Confirm wraps");

        let i = scr.items(&ctx).iter().position(|x| *x == Item::FfLimit).unwrap();
        scr.focus[0] = i;
        scr.update(&act(Action::Right), &mut ctx);
        assert_eq!(ctx.settings.ff_limit, FfLimit::Unlimited);
        scr.update(&act(Action::Left), &mut ctx);
        scr.update(&act(Action::Left), &mut ctx);
        assert_eq!(ctx.settings.ff_limit, FfLimit::X2);

        // spread 1 and 2
        scr.update(&act(Action::NextPage), &mut ctx);
        let i = scr.items(&ctx).iter().position(|x| *x == Item::Device).unwrap();
        scr.focus[1] = i;
        scr.update(&act(Action::Right), &mut ctx);
        assert_eq!(ctx.settings.audio_device.as_deref(), Some("Speakers"));
        scr.update(&act(Action::Right), &mut ctx);
        assert_eq!(ctx.settings.audio_device.as_deref(), Some("Headphones"));
        scr.update(&act(Action::Right), &mut ctx);
        assert_eq!(ctx.settings.audio_device.as_deref(), Some("Headphones"), "last option");
        scr.update(&act(Action::Confirm), &mut ctx);
        assert_eq!(ctx.settings.audio_device, None, "Confirm wraps to Default");
        let i = scr.items(&ctx).iter().position(|x| *x == Item::Audio).unwrap();
        scr.focus[1] = i;
        scr.update(&act(Action::Right), &mut ctx);
        assert_eq!(ctx.settings.audio, AudioMode::Muted);
        scr.update(&act(Action::Right), &mut ctx);
        assert_eq!(ctx.settings.audio, AudioMode::Off);
        let i = scr.items(&ctx).iter().position(|x| *x == Item::Pacing).unwrap();
        scr.focus[1] = i;
        scr.update(&act(Action::Confirm), &mut ctx);
        assert_eq!(ctx.settings.pacing, Pacing::WallClock);

        scr.update(&act(Action::NextPage), &mut ctx);
        assert_eq!(scr.current_item(&ctx), Some(Item::Autosave));
        assert_eq!(ctx.settings.autosave_frames, 60);
        scr.update(&act(Action::Right), &mut ctx);
        assert_eq!(ctx.settings.autosave_frames, 300);
        scr.update(&act(Action::Right), &mut ctx);
        assert_eq!(ctx.settings.autosave_frames, 600);
        scr.update(&act(Action::Confirm), &mut ctx);
        assert_eq!(ctx.settings.autosave_frames, 60);
    }

    #[test]
    fn missing_audio_device_stays_visible() {
        let mut fx = Fix::new();
        fx.s.audio_device = Some("Unplugged".into());
        let ctx = fx.ctx();
        let labels = option_labels(Item::Device, &ctx);
        assert_eq!(labels, vec!["Default", "Speakers", "Headphones", "Unplugged (not found)"]);
        assert_eq!(option_selected(Item::Device, &ctx), Some(3));
    }

    #[test]
    fn volume_ruler_bounds() {
        let mut fx = Fix::new();
        let mut scr = SettingsScreen::new();
        goto(&mut scr, &mut fx, Item::Volume);
        let mut ctx = fx.ctx();
        assert_eq!(ctx.settings.volume, 10);
        assert_eq!(scr.update(&act(Action::Right), &mut ctx), SettingsOutcome::None, "already at the top");
        assert_eq!(ctx.settings.volume, 10);
        assert_eq!(scr.update(&act(Action::Left), &mut ctx), SettingsOutcome::Changed);
        assert_eq!(ctx.settings.volume, 9);
        for _ in 0..9 {
            scr.update(&act(Action::Left), &mut ctx);
        }
        assert_eq!(ctx.settings.volume, 0);
        assert_eq!(scr.update(&act(Action::Left), &mut ctx), SettingsOutcome::None, "floor");
        assert_eq!(ctx.settings.volume, 0);
        assert_eq!(scr.update(&act(Action::Confirm), &mut ctx), SettingsOutcome::None, "Confirm does not wrap the volume");
    }

    #[test]
    fn library_folders_add_and_remove() {
        let mut fx = Fix::new();
        let mut scr = SettingsScreen::new();
        let mut ctx = fx.ctx();
        scr.update(&act(Action::NextPage), &mut ctx);
        // type a path, Enter adds it
        assert_eq!(scr.update(&act(Action::Confirm), &mut ctx), SettingsOutcome::None);
        assert!(ctx.rebinding, "typing also keeps the app's hotkeys quiet");
        assert_eq!(scr.update(&typed("\"C:\\Games\""), &mut ctx), SettingsOutcome::None);
        assert_eq!(scr.update(&press(Key::Enter), &mut ctx), SettingsOutcome::Changed);
        assert!(!ctx.rebinding);
        assert_eq!(ctx.settings.library_folders, vec![PathBuf::from("C:\\Games")], "quotes from Copy as path are stripped");
        // Escape drops what was typed; an empty or repeated path adds nothing
        scr.update(&act(Action::Up), &mut ctx);
        assert_eq!(scr.current_item(&ctx), Some(Item::Folder(0)), "the field stays last; Up reaches the folder");
        scr.update(&act(Action::Down), &mut ctx);
        assert_eq!(scr.current_item(&ctx), Some(Item::AddFolder));
        scr.update(&act(Action::Confirm), &mut ctx);
        scr.update(&typed("zzz"), &mut ctx);
        scr.update(&press(Key::Escape), &mut ctx);
        assert_eq!(ctx.settings.library_folders.len(), 1);
        scr.update(&act(Action::Confirm), &mut ctx);
        scr.update(&typed("C:\\Games"), &mut ctx);
        assert_eq!(scr.update(&press(Key::Enter), &mut ctx), SettingsOutcome::None, "already listed");
        scr.update(&act(Action::Confirm), &mut ctx);
        scr.update(&typed("D:\\Roms"), &mut ctx);
        scr.update(&UiInput { pressed: vec![Key::Backspace], repeated: vec![Key::Backspace], ..UiInput::default() }, &mut ctx);
        scr.update(&press(Key::Enter), &mut ctx);
        assert_eq!(ctx.settings.library_folders, vec![PathBuf::from("C:\\Games"), PathBuf::from("D:\\Rom")]);

        // Del removes the focused folder
        scr.update(&act(Action::Up), &mut ctx);
        assert_eq!(scr.current_item(&ctx), Some(Item::Folder(1)));
        assert_eq!(scr.update(&press(Key::Delete), &mut ctx), SettingsOutcome::Changed);
        assert_eq!(ctx.settings.library_folders, vec![PathBuf::from("C:\\Games")]);
        assert_eq!(scr.current_item(&ctx), Some(Item::AddFolder), "focus stays in range");
        scr.update(&act(Action::Up), &mut ctx);
        scr.update(&press(Key::Delete), &mut ctx);
        assert!(ctx.settings.library_folders.is_empty());
        // Delete elsewhere does nothing
        assert_eq!(scr.update(&press(Key::Delete), &mut ctx), SettingsOutcome::None);
    }

    #[test]
    fn bios_from_file_with_a_typed_path() {
        let mut fx = Fix::new();
        let mut scr = SettingsScreen::new();
        goto(&mut scr, &mut fx, Item::Bios);
        let mut ctx = fx.ctx();
        assert!(!scr.items(&ctx).contains(&Item::BiosPath));
        assert_eq!(scr.update(&act(Action::Right), &mut ctx), SettingsOutcome::Changed);
        assert_eq!(ctx.settings.bios, Bios::File(PathBuf::new()));
        assert_eq!(scr.items(&ctx).last(), Some(&Item::BiosPath));
        scr.update(&act(Action::Down), &mut ctx);
        assert_eq!(scr.current_item(&ctx), Some(Item::BiosPath));
        scr.update(&act(Action::Confirm), &mut ctx);
        scr.update(&typed("C:\\bios\\gba.bin"), &mut ctx);
        assert_eq!(scr.update(&press(Key::Enter), &mut ctx), SettingsOutcome::Changed);
        assert_eq!(ctx.settings.bios, Bios::File(PathBuf::from("C:\\bios\\gba.bin")));
        // back to built-in and again: the path comes back
        scr.update(&act(Action::Up), &mut ctx);
        scr.update(&act(Action::Left), &mut ctx);
        assert_eq!(ctx.settings.bios, Bios::BuiltIn);
        scr.update(&act(Action::Right), &mut ctx);
        assert_eq!(ctx.settings.bios, Bios::File(PathBuf::from("C:\\bios\\gba.bin")));
    }

    #[test]
    fn open_folder_outcome() {
        let mut fx = Fix::new();
        let mut scr = SettingsScreen::new();
        let mut ctx = fx.ctx();
        scr.update(&act(Action::NextPage), &mut ctx);
        scr.update(&act(Action::NextPage), &mut ctx);
        scr.update(&act(Action::Down), &mut ctx);
        assert_eq!(scr.current_item(&ctx), Some(Item::OpenFolder));
        assert_eq!(scr.update(&act(Action::Confirm), &mut ctx), SettingsOutcome::None, "nothing to open yet");
        ctx.settings.library_folders.push(PathBuf::from("C:\\Games"));
        assert_eq!(scr.update(&act(Action::Confirm), &mut ctx), SettingsOutcome::OpenFolder(PathBuf::from("C:\\Games")));
        scr.set_game_dir(Some(PathBuf::from("D:\\Roms\\Kingdom")));
        assert_eq!(scr.update(&act(Action::Confirm), &mut ctx), SettingsOutcome::OpenFolder(PathBuf::from("D:\\Roms\\Kingdom")), "the current game wins");
    }

    #[test]
    fn mouse_clicks_after_a_render() {
        let mut fx = Fix::new();
        let mut scr = SettingsScreen::new();
        let mut cv = Canvas::new(1280, 800, Fonts::new());
        scr.render(&mut cv, &fx.ctx());
        let click = |x: f32, y: f32| UiInput { mouse: (x, y), clicked: true, mouse_down: true, ..UiInput::default() };

        // a keycap: focus and wait for the key
        let hit = scr.hits.iter().find(|h| h.item == Item::Key(Target::Button(Keys::A))).copied().unwrap();
        let mut ctx = fx.ctx();
        scr.update(&click(hit.r.x + 3.0, hit.r.y + 3.0), &mut ctx);
        assert_eq!(scr.waiting, Some(Target::Button(Keys::A)));
        assert!(ctx.rebinding);
        scr.update(&press(Key::Escape), &mut ctx);

        // an option: picks it
        let hit = scr.hits.iter().find(|h| h.item == Item::Picture && h.kind == HitKind::Option(2)).copied().unwrap();
        assert_eq!(scr.update(&click(hit.r.x + 3.0, hit.r.y + 3.0), &mut ctx), SettingsOutcome::Changed);
        assert_eq!(ctx.settings.picture, PictureSize::Fixed(3));
        assert_eq!(scr.current_item(&ctx), Some(Item::Picture));

        // the ruler
        let hit = scr.hits.iter().find(|h| h.kind == HitKind::Ruler).copied().unwrap();
        assert_eq!(scr.update(&click(hit.r.x + 6.0 + (hit.r.w - 12.0) * 0.3, hit.r.y + 5.0), &mut ctx), SettingsOutcome::Changed);
        assert_eq!(ctx.settings.volume, 3);

        // a tab
        let t = scr.tab_rects[2];
        scr.update(&click(t.x + 3.0, t.y + 3.0), &mut ctx);
        assert_eq!(scr.spread(), 2);
        // the wheel scrolls only when the page overflows: not at the design size
        scr.render(&mut cv, &ctx);
        scr.update(&UiInput { mouse: (300.0, 300.0), wheel: -3.0, ..UiInput::default() }, &mut ctx);
        assert_eq!(scr.scroll, [0.0, 0.0]);
        // but at the minimum size the pages do overflow, and the wheel moves them
        let mut small = Canvas::new(960, 640, Fonts::new());
        scr.set_spread(0);
        scr.render(&mut small, &ctx);
        scr.render(&mut small, &ctx);
        assert!(scr.content_h[1] > scr.page_rects[1].h - 30.0, "{:?} {:?}", scr.content_h, scr.page_rects);
        scr.update(&UiInput { mouse: (scr.page_rects[1].x + 50.0, 300.0), wheel: -3.0, ..UiInput::default() }, &mut ctx);
        assert!(scr.scroll[1] > 0.0);
        scr.render(&mut small, &ctx);
        assert!(scr.scroll[1] <= scr.content_h[1], "clamped");
    }

    #[test]
    fn every_spread_renders_at_both_sizes() {
        for (w, h) in [(1280, 800), (960, 640)] {
            let mut fx = Fix::new();
            fx.s.library_folders = vec![PathBuf::from("C:\\Games"), PathBuf::from("D:\\A\\very\\long\\path\\to\\the\\roms\\folder")];
            fx.s.bios = Bios::File(PathBuf::from("C:\\bios\\gba.bin"));
            let mut scr = SettingsScreen::new();
            let mut cv = Canvas::new(w, h, Fonts::new());
            for spread in 0..3 {
                scr.set_spread(spread);
                scr.render(&mut cv, &fx.ctx());
                assert!(!scr.hits.is_empty(), "spread {spread} registers hit targets");
                // the manual: page background is paper, desk is bg2
                assert_eq!(cv.pixel(70, 100), PAPER.bg, "left page");
                assert_eq!(cv.pixel(20, 100), PAPER.bg2, "desk");
            }
        }
    }

    fn sample(fx: &mut Fix) {
        fx.s.picture = PictureSize::Fixed(3);
        fx.s.volume = 8;
        fx.s.library_folders = vec![PathBuf::from("C:\\Users\\ethan\\Games"), PathBuf::from("D:\\Emulation\\GBA")];
        fx.s.audio_device = Some("Speakers (Realtek Audio)".into());
        fx.devices = vec!["Speakers (Realtek Audio)".into(), "Headphones (USB)".into(), "Monitor (HDMI)".into()];
    }

    #[test]
    fn pinned_settings_renders() {
        let mut fx = Fix::new();
        sample(&mut fx);
        for (name, w, h, spread, rebinding) in [
            ("settings-controls", 1280, 800, 0, false),
            ("settings-system", 1280, 800, 1, false),
            ("settings-saves", 1280, 800, 2, false),
            ("settings-960-controls", 960, 640, 0, false),
            ("settings-960-system", 960, 640, 1, false),
            ("settings-rebind", 1280, 800, 0, true),
        ] {
            let mut scr = SettingsScreen::new();
            let mut cv = Canvas::new(w, h, Fonts::new());
            scr.set_spread(spread);
            let items = scr.items(&fx.ctx());
            if rebinding {
                scr.focus[0] = items.iter().position(|i| *i == Item::Key(Target::Pad(Keys::A))).unwrap();
                scr.waiting = Some(Target::Pad(Keys::A));
            } else if spread == 0 {
                scr.focus[0] = items.iter().position(|i| *i == Item::Picture).unwrap();
            } else if spread == 1 {
                scr.focus[1] = 1;
            }
            scr.set_game_dir(Some(PathBuf::from("D:\\Emulation\\GBA\\Kingdom Hearts")));
            scr.render(&mut cv, &fx.ctx());
            scr.render(&mut cv, &fx.ctx());
            crate::pinned::check_canvas(name, &cv);
        }
    }

    #[test]
    #[ignore]
    fn render_settings() {
        let out = crate::render_tests::out_dir();
        let mut fx = Fix::new();
        sample(&mut fx);
        for (name, w, h, spread, rebinding) in [
            ("04-settings.png", 1280, 800, 0, false),
            ("04-settings-system.png", 1280, 800, 1, false),
            ("04-settings-saves.png", 1280, 800, 2, false),
            ("04-settings-960.png", 960, 640, 0, false),
            ("04-settings-960-system.png", 960, 640, 1, false),
            ("04-settings-rebind.png", 1280, 800, 0, true),
        ] {
            let mut scr = SettingsScreen::new();
            let mut cv = Canvas::new(w, h, Fonts::new());
            scr.set_spread(spread);
            let items = scr.items(&fx.ctx());
            if rebinding {
                scr.focus[0] = items.iter().position(|i| *i == Item::Key(Target::Pad(Keys::A))).unwrap();
                scr.waiting = Some(Target::Pad(Keys::A));
            } else if spread == 0 {
                scr.focus[0] = items.iter().position(|i| *i == Item::Picture).unwrap();
            } else if spread == 1 {
                scr.focus[1] = 1;
            }
            scr.set_game_dir(Some(PathBuf::from("D:\\Emulation\\GBA\\Kingdom Hearts")));
            // two passes: the second sees the scroll limits of the first
            scr.render(&mut cv, &fx.ctx());
            scr.render(&mut cv, &fx.ctx());
            let p = out.join(name);
            cv.save_png(&p).unwrap();
            println!("wrote {}", p.display());
        }
    }
}
