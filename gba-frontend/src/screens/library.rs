//! Screen 01 Library (`docs/gui/SCREENS.md`, mockup `01-library.png`, Paper palette).
//!
//! Left: the *case*, a dark foam block holding the cartridges in a grid (4 columns at 1280 px;
//! fewer as the window narrows) that scrolls vertically. The first cell is the "in the console"
//! slot: a dashed outline showing the code of the game last played. Right: the handheld line
//! drawing with the selected cartridge sticking out of its top and the game's picture on its
//! screen, then title, details and the buttons.
//!
//! Driven like every screen: `update(&input, &library)` then `render(&mut canvas, &library)`.
//! Layout is recomputed on every render and remembered (cell rectangles, columns, buttons) so
//! the next `update` can hit-test the mouse and move the selection grid-aware.
//!
//! Find: typing a character that is not one of the bound GBA keys focuses the Find field. `/` and
//! Tab focus it explicitly (needed to search for a title that starts with Z, X, A or S, which are
//! also Play, Eject and the page keys). While Find has focus all characters go to it, and only
//! Enter plays.
//!
//! Empty library (`docs/branding/boards/05-strawberry-empty-library.png`): the case shows three
//! rows of dashed placeholders (the console slot says STRAWBERRY, the next cell says where to put
//! games), the handheld holds a Strawberry cartridge with the mark on its screen, and the right
//! column is the product card with `Open <folder>` (Z) and `Settings`; without any folder the one
//! button is `Settings`.
#![allow(dead_code)]

use crate::library::{Art, Library, LibraryEntry};
use crate::overlay::Overlay;
use crate::brand;
use crate::ui::canvas::{Clip, Canvas};
use crate::ui::drawings::{self, Drawing};
use crate::ui::fmt;
use crate::ui::input::{Action, UiInput};
use crate::ui::text::Style;
use crate::ui::tokens::{component as comp, font as fnt, label_for, layout as lay, object, pixel, Palette, Rgb, PAPER};
use crate::ui::widgets::{self, mix};
use gba_core::{FrameBuffer, SCREEN_H, SCREEN_W};
use minifb::Key;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime};
use tiny_skia::{Color, Paint, Pixmap, Transform};

/// What the screen asks the app to do.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LibraryOutcome {
    None,
    Play(PathBuf),
    ResumeLatest(PathBuf),
    OpenStates(PathBuf),
    /// Show this library folder in the file manager (empty library).
    OpenFolder(PathBuf),
    /// Open the Settings manual (empty library).
    OpenSettings,
    Quit,
}

/// Why the library shows no cartridges.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Empty {
    /// No folder is configured and `./Games` does not exist.
    NoFolder,
    /// A scan is running and has found nothing yet.
    Scanning,
    /// The folders were scanned and hold no games.
    NoGames,
}

fn empty_state(lib: &Library) -> Option<Empty> {
    if !lib.entries().is_empty() {
        None
    } else if lib.folders().is_empty() {
        Some(Empty::NoFolder)
    } else if lib.is_scanning() {
        Some(Empty::Scanning)
    } else {
        Some(Empty::NoGames)
    }
}

/// Rows of placeholder cells the empty case shows (the board's 3 x 4 at the design size).
const EMPTY_ROWS: usize = 3;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl Rect {
    pub fn new(x: f32, y: f32, w: f32, h: f32) -> Rect {
        Rect { x, y, w, h }
    }
    pub fn contains(&self, p: (f32, f32)) -> bool {
        p.0 >= self.x && p.0 < self.x + self.w && p.1 >= self.y && p.1 < self.y + self.h
    }
    pub fn intersect(&self, o: &Rect) -> Option<Rect> {
        let x0 = self.x.max(o.x);
        let y0 = self.y.max(o.y);
        let x1 = (self.x + self.w).min(o.x + o.w);
        let y1 = (self.y + self.h).min(o.y + o.h);
        (x1 > x0 && y1 > y0).then(|| Rect::new(x0, y0, x1 - x0, y1 - y0))
    }
    pub fn inset(&self, d: f32) -> Rect {
        Rect::new(self.x + d, self.y + d, (self.w - 2.0 * d).max(0.0), (self.h - 2.0 * d).max(0.0))
    }
    fn clip(&self) -> Clip {
        Clip { x: self.x.floor() as i32, y: self.y.floor() as i32, w: self.w.ceil() as i32, h: self.h.ceil() as i32 }
    }
}

// --- layout -----------------------------------------------------------------------------------

const SIDE_W: f32 = 436.0;
const CASE_TOP_GAP: f32 = 26.0;
const CASE_PAD: f32 = 32.0;
const FOAM_INSET: f32 = 12.0;
const ROW_PITCH: f32 = 154.0;
const ROW_TOP: f32 = 53.0;
const GAP_MIN: f32 = 10.0;
const GAP_MAX: f32 = 24.0;
const STRIP_W: usize = comp::CART_ART_W as usize;
const SHADOW_PAD: f32 = 26.0;
const FIND_W: f32 = 226.0;

/// Where everything goes for a window size and a number of cells (slot + cartridges).
#[derive(Clone, Copy, Debug)]
pub struct Geometry {
    pub case: Rect,
    /// The foam interior; cartridges are clipped to it.
    pub viewport: Rect,
    pub cols: usize,
    pub gap: f32,
    /// x of the first column.
    pub grid_x: f32,
    /// y of the first row's top edge with no scrolling.
    pub row0_y: f32,
    pub rows: usize,
    pub side: Rect,
    pub max_scroll: f32,
}

pub fn geometry(w: f32, h: f32, n_cells: usize) -> Geometry {
    let case_x = lay::MARGIN;
    let case_y = lay::HEAD_H + CASE_TOP_GAP;
    let case_w = (w - 2.0 * lay::MARGIN - lay::GUTTER - SIDE_W).max(comp::CART_W + 2.0 * CASE_PAD);
    let inner = case_w - 2.0 * CASE_PAD;
    let cols = (((inner + GAP_MIN) / (comp::CART_W + GAP_MIN)).floor() as usize).max(1);
    let gap = if cols > 1 { ((inner - cols as f32 * comp::CART_W) / (cols - 1) as f32).clamp(GAP_MIN, GAP_MAX) } else { 0.0 };
    let grid_w = cols as f32 * comp::CART_W + cols.saturating_sub(1) as f32 * gap;
    let grid_x = case_x + CASE_PAD + (inner - grid_w) / 2.0;
    let rows = n_cells.max(1).div_ceil(cols);
    let full_h = rows as f32 * ROW_PITCH + 60.0;
    let avail = (h - lay::FOOT_H - CASE_TOP_GAP - case_y).max(ROW_PITCH + 60.0);
    let case_h = full_h.min(avail);
    let case = Rect::new(case_x, case_y, case_w, case_h);
    let side_y = if h < 720.0 { lay::HEAD_H + 12.0 } else { case_y };
    Geometry {
        case,
        viewport: case.inset(FOAM_INSET),
        cols,
        gap,
        grid_x,
        row0_y: case_y + ROW_TOP,
        rows,
        side: Rect::new((w - lay::MARGIN - SIDE_W).max(case_x + case_w + 16.0), side_y, SIDE_W, h - side_y - lay::FOOT_H),
        max_scroll: (full_h - case_h).max(0.0),
    }
}

impl Geometry {
    /// Rectangle of cell `i` (0 = the console slot) for a scroll offset.
    pub fn cell_rect(&self, i: usize, scroll: f32) -> Rect {
        let (r, c) = (i / self.cols, i % self.cols);
        Rect::new(self.grid_x + c as f32 * (comp::CART_W + self.gap), self.row0_y + r as f32 * ROW_PITCH - scroll, comp::CART_W, comp::CART_H)
    }
}

/// Grid-aware selection move. `dx`/`dy` are -1, 0 or 1. Left/right step through the cells and wrap
/// across rows; up/down move a row, and down from a shorter last row's neighbour lands on the last
/// cell. Never leaves `0..n`.
pub fn move_cell(cur: usize, n: usize, cols: usize, dx: i32, dy: i32) -> usize {
    if n == 0 {
        return 0;
    }
    let cols = cols.max(1);
    let cur = cur.min(n - 1);
    let mut t = cur;
    if dx < 0 {
        t = t.saturating_sub(1);
    } else if dx > 0 {
        t = (t + 1).min(n - 1);
    }
    if dy < 0 && cur >= cols {
        t = cur - cols;
    } else if dy > 0 {
        if cur + cols < n {
            t = cur + cols;
        } else if cur / cols < (n - 1) / cols {
            t = n - 1;
        }
    }
    t
}

// --- state ------------------------------------------------------------------------------------

/// The newest loadable save state of the selected game.
#[derive(Clone, Debug, PartialEq)]
struct LatestState {
    frame: u64,
    play_secs: u64,
    saved_at: SystemTime,
}

#[derive(Clone, Debug)]
struct SlotInfo {
    path: PathBuf,
    latest: Option<LatestState>,
}

/// Cells of the grid: the slot plus the cartridges that pass Find, as indices into the entries.
struct Grid {
    console: Option<usize>,
    carts: Vec<usize>,
}

impl Grid {
    fn len(&self) -> usize {
        1 + self.carts.len()
    }
    fn entry_of(&self, cell: usize) -> Option<usize> {
        if cell == 0 {
            self.console
        } else {
            self.carts.get(cell - 1).copied()
        }
    }
    fn cell_of(&self, entry: usize) -> Option<usize> {
        if Some(entry) == self.console {
            Some(0)
        } else {
            self.carts.iter().position(|&c| c == entry).map(|i| i + 1)
        }
    }
}

pub struct LibraryScreen {
    selected: Option<PathBuf>,
    initialised: bool,
    /// After Eject: the last-played game at that moment; the console stays empty until it changes.
    ejected: Option<Option<PathBuf>>,
    find: String,
    find_focus: bool,
    scroll: f32,
    scroll_target: f32,
    reveal: bool,
    max_scroll: f32,
    page_rows: usize,
    cols: usize,
    lifts: HashMap<PathBuf, f32>,
    last_tick: Instant,
    slot_info: Option<SlotInfo>,
    sel_changed: Instant,
    /// How long a selection must rest before its save states are read from disk.
    pub slot_delay: Duration,
    last_played_seen: Option<PathBuf>,
    // remembered by render for the next update
    hits: Vec<(Rect, PathBuf)>,
    play_btn: Option<Rect>,
    resume_btn: Option<Rect>,
    /// Empty library: `Open <folder>` and `Settings`.
    open_btn: Option<Rect>,
    settings_btn: Option<Rect>,
    find_rect: Rect,
    case_rect: Rect,
    // render caches
    handheld: Drawing,
    case_cache: Option<(u32, u32, Pixmap)>,
    strips: HashMap<PathBuf, (u64, Vec<u8>)>,
}

impl Default for LibraryScreen {
    fn default() -> Self {
        Self::new()
    }
}

fn printable(c: char) -> bool {
    !c.is_control()
}

fn matches_find(e: &LibraryEntry, q: &str) -> bool {
    q.is_empty() || e.title.to_lowercase().contains(q) || e.subtitle.to_lowercase().contains(q) || e.code.to_lowercase().contains(q)
}

impl LibraryScreen {
    pub fn new() -> Self {
        let now = Instant::now();
        LibraryScreen {
            selected: None,
            initialised: false,
            ejected: None,
            find: String::new(),
            find_focus: false,
            scroll: 0.0,
            scroll_target: 0.0,
            reveal: false,
            max_scroll: 0.0,
            page_rows: 2,
            cols: 4,
            lifts: HashMap::new(),
            last_tick: now,
            slot_info: None,
            sel_changed: now,
            slot_delay: Duration::from_millis(120),
            last_played_seen: None,
            hits: Vec::new(),
            play_btn: None,
            resume_btn: None,
            open_btn: None,
            settings_btn: None,
            find_rect: Rect::default(),
            case_rect: Rect::default(),
            handheld: drawings::parse(crate::brand::HANDHELD_LIBRARY),
            case_cache: None,
            strips: HashMap::new(),
        }
    }

    // --- accessors (also used by tests and the integrator) ---------------------------------

    pub fn selected(&self) -> Option<&Path> {
        self.selected.as_deref()
    }

    pub fn find_text(&self) -> &str {
        &self.find
    }

    pub fn columns(&self) -> usize {
        self.cols
    }

    /// Tests: pretend the last render had `n` columns.
    pub fn set_columns(&mut self, n: usize) {
        self.cols = n.max(1);
    }

    /// Select a game by path (e.g. the one given on the command line).
    pub fn select(&mut self, path: &Path) {
        self.select_path(path.to_path_buf());
        self.initialised = true;
    }

    /// Forget the cached save-state info, e.g. after returning from a game.
    pub fn refresh_slots(&mut self) {
        self.slot_info = None;
        self.sel_changed = Instant::now() - self.slot_delay;
    }

    fn select_path(&mut self, p: PathBuf) {
        if self.selected.as_deref() != Some(p.as_path()) {
            self.selected = Some(p);
            self.sel_changed = Instant::now();
            self.reveal = true;
        }
    }

    fn console_path(&self, lib: &Library) -> Option<PathBuf> {
        if self.ejected.is_some() {
            None
        } else {
            lib.last_played()
        }
    }

    fn grid(&self, lib: &Library) -> Grid {
        let console_path = self.console_path(lib);
        let entries = lib.entries();
        let console = console_path.and_then(|p| entries.iter().position(|e| e.path == p));
        let q = self.find.to_lowercase();
        let carts = entries.iter().enumerate().filter(|(i, e)| Some(*i) != console && matches_find(e, &q)).map(|(i, _)| i).collect();
        Grid { console, carts }
    }

    fn selected_entry<'a>(&self, lib: &'a Library) -> Option<&'a LibraryEntry> {
        self.selected.as_deref().and_then(|p| lib.entry(p))
    }

    fn sel_cell(&self, grid: &Grid, lib: &Library) -> Option<usize> {
        let p = self.selected.as_deref()?;
        let ei = lib.entries().iter().position(|e| e.path == p)?;
        grid.cell_of(ei)
    }

    fn select_cell(&mut self, grid: &Grid, lib: &Library, cell: usize) {
        if let Some(ei) = grid.entry_of(cell) {
            self.select_path(lib.entries()[ei].path.clone());
        }
    }

    fn load_slot_info(&mut self, path: &Path) {
        let set = crate::glue::GBA_SLOTS.scan(path);
        let latest = set.latest().map(|m| LatestState { frame: m.frame, play_secs: m.play_secs, saved_at: m.saved_at });
        self.slot_info = Some(SlotInfo { path: path.to_path_buf(), latest });
    }

    fn latest_for_selected(&self) -> Option<&LatestState> {
        let (p, info) = (self.selected.as_deref()?, self.slot_info.as_ref()?);
        (info.path == p).then_some(info.latest.as_ref()).flatten()
    }

    // --- update ----------------------------------------------------------------------------

    pub fn update(&mut self, input: &UiInput, lib: &Library) -> LibraryOutcome {
        let now = Instant::now();
        let dt = now.duration_since(self.last_tick).as_secs_f32().min(0.1);
        self.last_tick = now;

        // an ejected console fills again once a game has been played
        if let Some(snap) = &self.ejected {
            if lib.last_played() != *snap {
                self.ejected = None;
            }
        }
        let lp = lib.last_played();
        if lp != self.last_played_seen {
            self.last_played_seen = lp;
            self.slot_info = None;
        }
        if !self.initialised && !lib.entries().is_empty() {
            self.initialised = true;
            let first = lib.last_played().or_else(|| lib.entries().first().map(|e| e.path.clone()));
            if let Some(p) = first {
                self.select_path(p);
            }
        }

        // --- Find ---
        let letters_are_commands = [Action::Confirm, Action::Back, Action::PrevPage, Action::NextPage].iter().any(|a| input.action(*a));
        let chars: Vec<char> = input.chars.iter().copied().filter(|c| printable(*c)).collect();
        let before = self.find.clone();
        if input.pressed(Key::Tab) {
            self.find_focus = !self.find_focus;
        }
        if self.find_focus {
            self.find.extend(chars.iter());
        } else if !chars.is_empty() && !letters_are_commands {
            self.find_focus = true;
            if chars != ['/'] {
                self.find.extend(chars.iter());
            }
        }
        if self.find_focus && input.repeated(Key::Backspace) {
            self.find.pop();
        }
        if input.pressed(Key::Escape) {
            if self.find.is_empty() {
                return LibraryOutcome::Quit;
            }
            self.find.clear();
            self.find_focus = false;
        }

        let grid = self.grid(lib);
        if self.find != before {
            self.scroll_target = 0.0;
            // keep a selection that still matches; else take the first match
            if self.sel_cell(&grid, lib).is_none() && !self.find.is_empty() {
                let cell = if grid.carts.is_empty() { 0 } else { 1 };
                self.select_cell(&grid, lib, cell);
            }
        }

        // --- keyboard navigation ---
        let n = grid.len();
        let cur = self.sel_cell(&grid, lib);
        let mut moved: Option<(i32, i32, usize)> = None;
        for a in &input.actions {
            match a {
                Action::Left => moved = Some((-1, 0, 1)),
                Action::Right => moved = Some((1, 0, 1)),
                Action::Up => moved = Some((0, -1, 1)),
                Action::Down => moved = Some((0, 1, 1)),
                Action::PrevPage if !self.find_focus => moved = Some((0, -1, self.page_rows)),
                Action::NextPage if !self.find_focus => moved = Some((0, 1, self.page_rows)),
                _ => {}
            }
        }
        if let Some((dx, dy, times)) = moved {
            let first_selectable = if grid.console.is_some() { 0 } else { 1.min(n - 1) };
            match cur {
                None => self.select_cell(&grid, lib, first_selectable),
                Some(mut c) => {
                    for _ in 0..times {
                        let t = move_cell(c, n, self.cols, dx, dy);
                        if grid.entry_of(t).is_some() {
                            c = t;
                        }
                    }
                    self.select_cell(&grid, lib, c);
                }
            }
        }

        // --- scrolling and hover animation ---
        if input.wheel != 0.0 {
            self.scroll_target = (self.scroll_target - input.wheel.clamp(-3.0, 3.0) * 60.0).clamp(0.0, self.max_scroll);
        }
        let diff = self.scroll_target - self.scroll;
        self.scroll = if diff.abs() < 0.5 { self.scroll_target } else { self.scroll + diff * (dt / 0.09).min(1.0) };
        let hovered = self.hits.iter().find(|(r, _)| r.contains(input.mouse)).map(|(_, p)| p.clone());
        let step = dt / (comp::CART_HOVER_MS as f32 / 1000.0);
        if let Some(h) = &hovered {
            self.lifts.entry(h.clone()).or_insert(0.0);
        }
        self.lifts.retain(|p, t| {
            *t = if Some(p) == hovered.as_ref() { (*t + step).min(1.0) } else { (*t - step).max(0.0) };
            *t > 0.0 || Some(p) == hovered.as_ref()
        });

        // --- save-state info for the selection, once it has rested ---
        if let Some(p) = self.selected.clone() {
            let stale = self.slot_info.as_ref().is_none_or(|i| i.path != p);
            if stale && self.sel_changed.elapsed() >= self.slot_delay && lib.entry(&p).is_some() {
                self.load_slot_info(&p);
            }
        }

        // --- commands ---
        let sel_ok = self.selected_entry(lib).filter(|e| e.unreadable.is_none()).map(|e| e.path.clone());
        let confirm = (input.action(Action::Confirm) && !self.find_focus) || input.action(Action::Start) || input.pressed(Key::Enter);
        if confirm {
            if let Some(p) = sel_ok.clone() {
                return LibraryOutcome::Play(p);
            }
            if let Some(e) = empty_state(lib) {
                return Self::empty_primary(lib, e);
            }
        }
        if input.action(Action::Back) && !self.find_focus {
            self.selected = None;
            self.slot_info = None;
            self.ejected = Some(lib.last_played());
            return LibraryOutcome::None;
        }
        if input.pressed(Key::F3) {
            if let Some(p) = sel_ok.clone() {
                if self.slot_info.as_ref().is_none_or(|i| i.path != p) {
                    self.load_slot_info(&p);
                }
                if self.latest_for_selected().is_some() {
                    return LibraryOutcome::ResumeLatest(p);
                }
            }
        }
        if input.pressed(Key::F2) {
            if let Some(p) = self.selected.clone() {
                return LibraryOutcome::OpenStates(p);
            }
        }

        // --- mouse ---
        if input.clicked {
            if self.find_rect.contains(input.mouse) {
                self.find_focus = true;
            } else if self.play_btn.is_some_and(|r| r.contains(input.mouse)) {
                if let Some(p) = sel_ok {
                    return LibraryOutcome::Play(p);
                }
            } else if self.resume_btn.is_some_and(|r| r.contains(input.mouse)) {
                if let Some(p) = sel_ok {
                    return LibraryOutcome::ResumeLatest(p);
                }
            } else if self.open_btn.is_some_and(|r| r.contains(input.mouse)) {
                if let Some(f) = lib.folders().first() {
                    return LibraryOutcome::OpenFolder(f.clone());
                }
            } else if self.settings_btn.is_some_and(|r| r.contains(input.mouse)) {
                return LibraryOutcome::OpenSettings;
            } else if let Some(h) = hovered {
                if self.selected.as_deref() == Some(h.as_path()) {
                    if lib.entry(&h).is_some_and(|e| e.unreadable.is_none()) {
                        return LibraryOutcome::Play(h);
                    }
                } else {
                    self.select_path(h);
                }
            }
        }
        LibraryOutcome::None
    }

    /// What Z does with no cartridges: show the first folder, or open Settings to add one.
    fn empty_primary(lib: &Library, empty: Empty) -> LibraryOutcome {
        match (empty, lib.folders().first()) {
            (Empty::NoFolder, _) | (_, None) => LibraryOutcome::OpenSettings,
            (_, Some(f)) => LibraryOutcome::OpenFolder(f.clone()),
        }
    }

    // --- render ----------------------------------------------------------------------------

    pub fn render(&mut self, cv: &mut Canvas, lib: &Library) {
        let pal = &PAPER;
        let (w, h) = (cv.width(), cv.height());
        let grid = self.grid(lib);
        // an empty case keeps the board's three rows of placeholders (columns follow the width)
        let n_cells = if empty_state(lib).is_some() { geometry(w, h, 1).cols * EMPTY_ROWS } else { grid.len() };
        let g = geometry(w, h, n_cells);
        self.cols = g.cols;
        self.max_scroll = g.max_scroll;
        self.page_rows = (((g.viewport.h) / ROW_PITCH).floor() as usize).max(1);
        self.scroll = self.scroll.clamp(0.0, g.max_scroll);
        self.scroll_target = self.scroll_target.clamp(0.0, g.max_scroll);
        if self.reveal {
            self.reveal = false;
            if let Some(c) = self.sel_cell(&grid, lib) {
                let top = ROW_TOP + (c / g.cols) as f32 * ROW_PITCH;
                let (lo, hi) = (top + comp::CART_H + 28.0 - g.case.h, top - 22.0);
                self.scroll_target = self.scroll_target.clamp(lo.max(0.0), hi.max(lo.max(0.0))).clamp(0.0, g.max_scroll);
            }
        }

        cv.clear(pal.bg);
        self.hits.clear();
        self.play_btn = None;
        self.resume_btn = None;
        self.open_btn = None;
        self.settings_btn = None;

        self.draw_head(cv, lib, pal);
        self.draw_case(cv, lib, &grid, &g, pal);
        self.draw_side(cv, lib, &g, pal);
        let back = if self.find.is_empty() { ("Esc", "Quit") } else { ("Esc", "Clear find") };
        match empty_state(lib) {
            Some(Empty::NoFolder) => widgets::foot(cv, &[(&["Z"], "Settings")], Some(back), pal),
            Some(_) => {
                let open = format!("Open {}", Self::folder_label(lib));
                widgets::foot(cv, &[(&["Z"], open.as_str())], Some(back), pal);
            }
            None => widgets::foot(
                cv,
                &[(&["↑", "↓", "←", "→"], "Choose"), (&["Z"], "Play"), (&["X"], "Eject"), (&["F3"], "Resume last state"), (&["F2"], "Save states")],
                Some(back),
                pal,
            ),
        }
    }

    fn folder_label(lib: &Library) -> String {
        let name = |p: &Path| p.file_name().map(|n| format!("{}/", n.to_string_lossy())).unwrap_or_else(|| p.display().to_string());
        match lib.folders() {
            [] => String::new(),
            [one] => name(one),
            [first, rest @ ..] => format!("{} +{}", name(first), rest.len()),
        }
    }

    fn draw_head(&mut self, cv: &mut Canvas, lib: &Library, pal: &Palette) {
        let n = lib.entries().len();
        let mut ctx = format!("{n} cartridge{}", if n == 1 { "" } else { "s" });
        let folder = Self::folder_label(lib);
        if !folder.is_empty() {
            ctx.push_str(" · ");
            ctx.push_str(&folder);
        }
        if lib.is_scanning() {
            ctx.push_str(" · scanning");
        }
        widgets::head(cv, "01", "Library", Some(&ctx), None, pal);
        let w = cv.width();
        let fx = w - lay::MARGIN - FIND_W;
        widgets::underline_field(cv, fx, 20.0, FIND_W, "FIND", &self.find, "Title or code", self.find_focus, pal);
        self.find_rect = Rect::new(fx, 10.0, FIND_W, 36.0);
    }

    fn ensure_case(&mut self, w: u32, h: u32, pad: u32) {
        if self.case_cache.as_ref().is_some_and(|(cw, ch, _)| *cw == w && *ch == h) {
            return;
        }
        self.case_cache = Some((w, h, build_case(w as f32, h as f32, pad as f32)));
    }

    fn draw_case(&mut self, cv: &mut Canvas, lib: &Library, grid: &Grid, g: &Geometry, pal: &Palette) {
        self.case_rect = g.case;
        let (cw, ch) = (g.case.w.round() as u32, g.case.h.round() as u32);
        self.ensure_case(cw, ch, SHADOW_PAD as u32);
        if let Some((_, _, pix)) = &self.case_cache {
            cv.draw_pixmap(pix, g.case.x - SHADOW_PAD, g.case.y - SHADOW_PAD, Transform::identity());
        }

        // art strips for the cells in view
        let entries = lib.entries();
        let strip_h = (comp::CART_H - comp::CART_LABEL_INSET[0] - comp::CART_LABEL_INSET[2]) as usize;
        let empty = empty_state(lib);
        let n = if empty.is_some() { g.rows * g.cols } else { grid.len() };
        let mut visible: Vec<usize> = Vec::new();
        for i in 0..n {
            let r = g.cell_rect(i, self.scroll);
            if r.y + r.h + 8.0 >= g.viewport.y && r.y - 8.0 <= g.viewport.y + g.viewport.h {
                visible.push(i);
            }
        }
        for &i in &visible {
            let Some(ei) = grid.entry_of(i) else { continue };
            let path = &entries[ei].path;
            if let Some(art) = lib.art_for(path) {
                if self.strips.get(path).is_none_or(|(id, _)| *id != art.id) {
                    self.strips.insert(path.clone(), (art.id, art.cover(STRIP_W, strip_h)));
                }
            }
        }

        let sel_cell = self.sel_cell(grid, lib);
        cv.set_clip(Some(g.viewport.clip()));
        let mut hits = Vec::new();
        for &i in &visible {
            let r = g.cell_rect(i, self.scroll);
            let selected = sel_cell == Some(i);
            let ei = grid.entry_of(i);
            if i == 0 {
                let code = match (empty, ei) {
                    (Some(_), _) => Some(brand::WORDMARK),
                    (None, Some(e)) => Some(entries[e].code.as_str()),
                    (None, None) => None,
                };
                draw_slot(cv, r, code, selected, pal);
                if let (Some(e), Some(vis)) = (ei, r.intersect(&g.viewport)) {
                    hits.push((vis, entries[e].path.clone()));
                }
                continue;
            }
            let Some(ei) = ei else {
                if let Some(e) = empty {
                    let hint = (i == 1).then(|| empty_hint(e, &Self::folder_label(lib)));
                    draw_empty_cell(cv, r, hint.as_deref(), pal);
                }
                continue;
            };
            let e = &entries[ei];
            let lift = self.lifts.get(&e.path).copied().unwrap_or(0.0);
            let ease = 1.0 - (1.0 - lift) * (1.0 - lift);
            let off = comp::CART_HOVER_LIFT * ease;
            let strip = self.strips.get(&e.path).map(|(_, s)| s.as_slice());
            draw_cart(cv, Rect::new(r.x, r.y - off, r.w, r.h), e, strip, selected, off, pal);
            if let Some(vis) = r.intersect(&g.viewport) {
                hits.push((vis, e.path.clone()));
            }
        }
        cv.set_clip(None);
        self.hits = hits;

        // a thin scroll thumb on the foam's right edge when the grid overflows
        if g.max_scroll > 0.0 {
            let track = g.viewport.h - 16.0;
            let thumb = (track * g.viewport.h / (g.viewport.h + g.max_scroll)).max(24.0);
            let ty = g.viewport.y + 8.0 + (track - thumb) * (self.scroll / g.max_scroll);
            cv.round_rect(g.viewport.x + g.viewport.w - 7.0, ty, 3.0, thumb, [1.5; 4], mix(object::FOAM, object::CART_SHELL, 0.35));
        }
    }

    fn draw_side(&mut self, cv: &mut Canvas, lib: &Library, g: &Geometry, pal: &Palette) {
        let (sx, sy, sw) = (g.side.x, g.side.y, g.side.w);
        let bottom = g.side.y + g.side.h;
        let tight = bottom - sy < 548.0;
        if let Some(e) = empty_state(lib) {
            self.draw_side_brand(cv, lib, e, g, pal);
            return;
        }
        let sel = self.selected_entry(lib).cloned();
        let art = sel.as_ref().and_then(|e| lib.art_for(&e.path));

        if let Some(e) = &sel {
            draw_inserted_cart(cv, Rect::new(sx + 143.0, sy, 150.0, 84.0), e, pal);
        }
        cv.drawing(&self.handheld, sx, sy);
        if let Some(e) = &sel {
            draw_picture(cv, sx + 98.0, sy + 92.0, e, art, pal);
        }
        widgets::keycap(cv, sx + 34.0, sy + 60.0, "A", pal, false);
        widgets::keycap(cv, sx + 380.0, sy + 60.0, "S", pal, false);
        widgets::keycap(cv, sx + 387.0, sy + 181.0, "Z", pal, true);
        widgets::keycap(cv, sx + 355.0, sy + 215.0, "X", pal, false);

        let mut y = sy + 320.0 + if tight { 12.0 } else { 20.0 };
        let Some(e) = sel else {
            self.draw_side_message(cv, lib, sx, y, sw, pal);
            return;
        };

        // title and subtitle
        let title_style = Style::display(fnt::DISPLAY_TITLE).color(pal.fg);
        let (size, lines) = cv.fonts.fit_display(title_style, &e.title, sw, 2, &[34.0, 30.0, 26.0]);
        let lh = size * fnt::DISPLAY_LINE_HEIGHT;
        for (i, l) in lines.iter().enumerate() {
            cv.text(sx, y + i as f32 * lh, l, &title_style.size(size));
        }
        y += lines.len() as f32 * lh + if tight { 4.0 } else { 8.0 };
        if !e.subtitle.is_empty() {
            let st = Style::reading(fnt::READING_INTRO).color(pal.fg2);
            let sub = cv.fonts.ellipsize(&st, &e.subtitle, sw);
            cv.text(sx, y, &sub, &st);
        }
        y += if tight { 24.0 } else { 33.0 };

        // details
        let pitch = if tight { 21.0 } else { 23.0 };
        let col = 108.0;
        if let Some(why) = &e.unreadable {
            let file = cv.fonts.ellipsize_middle(&Style::data(fnt::DATA_BODY), &e.file_name(), sw - col);
            widgets::detail_row(cv, sx, y, col, "FILE", &file, true, pal);
            y += pitch;
            let st = Style::reading(fnt::READING_BODY);
            let msg = cv.fonts.wrap(&st, why, sw - col, 3);
            widgets::label(cv, sx, y + 2.0, "PROBLEM", pal);
            for (i, l) in msg.iter().enumerate() {
                cv.text(sx + col, y + i as f32 * 19.0, l, &st.color(pal.fg));
            }
            return;
        }
        let cart = format!("{} · {} · {}", e.code, fmt::fmt_size(e.rom_len), crate::gba_copy::save_name(crate::gba_copy::save_type(&e.save)));
        widgets::detail_row(cv, sx, y, col, "CARTRIDGE", &cart, true, pal);
        y += pitch;
        let last = match (&self.slot_info, self.latest_for_selected()) {
            (Some(i), Some(s)) if i.path == e.path => format!("FR {} · {} · {}", fmt::fmt_frames(s.frame), fmt::fmt_play(s.play_secs), fmt::fmt_when(s.saved_at)),
            (Some(i), None) if i.path == e.path => "none".to_string(),
            _ => "…".to_string(),
        };
        widgets::detail_row(cv, sx, y, col, "LAST STATE", &last, true, pal);
        y += pitch;
        let played = lib.played_secs(&e.path);
        let played = if played == 0 { "not yet".to_string() } else { fmt::fmt_played(played) };
        widgets::detail_row(cv, sx, y, col, "PLAYED", &played, true, pal);
        y += pitch + if tight { 6.0 } else { 15.0 };

        // buttons
        let has_state = self.latest_for_selected().is_some();
        let play_w = 290.0f32.min(sw);
        widgets::button_primary(cv, sx, y, play_w, "Play", Some("Z"), pal);
        self.play_btn = Some(Rect::new(sx, y, play_w, comp::BUTTON_H));
        if has_state {
            let rx = sx + play_w + 12.0;
            widgets::button_secondary(cv, rx, y, sx + sw - rx, "Resume state", pal);
            self.resume_btn = Some(Rect::new(rx, y, sx + sw - rx, comp::BUTTON_H));
        }
    }

    fn draw_side_message(&self, cv: &mut Canvas, _lib: &Library, sx: f32, y: f32, sw: f32, pal: &Palette) {
        let st = Style::reading(fnt::READING_INTRO).color(pal.fg2);
        cv.paragraph(sx, y, sw, "No cartridge in the console. Choose one with the arrow keys.", &st, fnt::READING_LINE_HEIGHT, 3);
    }

    /// The right column with no cartridges (board 05): the Strawberry cart in the handheld, the
    /// mark on its screen, the product card and the `Open <folder>` / `Settings` buttons.
    fn draw_side_brand(&mut self, cv: &mut Canvas, lib: &Library, empty: Empty, g: &Geometry, pal: &Palette) {
        let (sx, sy, sw) = (g.side.x, g.side.y, g.side.w);
        let tight = g.side.h < 548.0;

        draw_brand_cart(cv, Rect::new(sx + 143.0, sy, 150.0, 84.0), pal);
        cv.drawing(&self.handheld, sx, sy);
        draw_brand_screen(cv, sx + 98.0, sy + 92.0, empty, pal);
        widgets::keycap(cv, sx + 34.0, sy + 60.0, "A", pal, false);
        widgets::keycap(cv, sx + 380.0, sy + 60.0, "S", pal, false);
        widgets::keycap(cv, sx + 387.0, sy + 181.0, "Z", pal, true);
        widgets::keycap(cv, sx + 355.0, sy + 215.0, "X", pal, false);

        // the card
        let mut y = sy + 320.0 + if tight { 12.0 } else { 20.0 };
        let title = Style::display(fnt::DISPLAY_TITLE).color(pal.fg);
        cv.text(sx, y, brand::WORDMARK, &title);
        y += fnt::DISPLAY_TITLE * fnt::DISPLAY_LINE_HEIGHT + if tight { 4.0 } else { 8.0 };
        cv.text(sx, y, brand::SERIES, &Style::reading(fnt::READING_INTRO).color(pal.fg2));
        y += if tight { 24.0 } else { 33.0 };

        let pitch = if tight { 21.0 } else { 23.0 };
        let col = 150.0;
        let folder = Self::folder_label(lib);
        let games = match empty {
            Empty::NoFolder => "no folder yet".to_string(),
            Empty::Scanning => format!("{folder} · scanning…"),
            Empty::NoGames => format!("{folder} · empty"),
        };
        widgets::detail_row(cv, sx, y, col, "GAMES", &games, true, pal);
        y += pitch;
        widgets::detail_row(cv, sx, y, col, "SYSTEM PROGRAM", "Built in. No BIOS dump needed.", false, pal);
        y += pitch;
        widgets::detail_row(cv, sx, y, col, "SAVES", "Written beside each game as .sav.", false, pal);
        y += pitch + if tight { 6.0 } else { 15.0 };

        // the buttons
        let play_w = 290.0f32.min(sw);
        match Self::empty_primary(lib, empty) {
            LibraryOutcome::OpenFolder(_) => {
                widgets::button_primary(cv, sx, y, play_w, &format!("Open {folder}"), Some("Z"), pal);
                self.open_btn = Some(Rect::new(sx, y, play_w, comp::BUTTON_H));
                let rx = sx + play_w + 12.0;
                widgets::button_secondary(cv, rx, y, sx + sw - rx, "Settings", pal);
                self.settings_btn = Some(Rect::new(rx, y, sx + sw - rx, comp::BUTTON_H));
            }
            _ => {
                widgets::button_primary(cv, sx, y, play_w, "Settings", Some("Z"), pal);
                self.settings_btn = Some(Rect::new(sx, y, play_w, comp::BUTTON_H));
            }
        }
    }
}

/// Cell 1's two-line hint with no cartridges.
fn empty_hint(empty: Empty, folder: &str) -> String {
    match empty {
        Empty::NoFolder => "ADD A FOLDER IN SETTINGS".to_string(),
        Empty::Scanning => "LOOKING FOR GAMES…".to_string(),
        Empty::NoGames => format!("PUT .GBA FILES IN {folder}"),
    }
}

/// The handheld's screen with no cartridges: ink, the night mark at 3x, the wordmark at 2x and
/// a one-line state, composed at native 240x160 and shown at 1x like a game's picture.
fn draw_brand_screen(cv: &mut Canvas, x: f32, y: f32, empty: Empty, pal: &Palette) {
    let mut fb: Box<FrameBuffer> = vec![pixel::INK_555; SCREEN_W * SCREEN_H].into_boxed_slice().try_into().expect("240x160");
    let mut o = Overlay::new(&mut fb);
    o.sprite(96, 24, &brand::mark().night_555(), brand::SPRITE, brand::SPRITE, 3);
    o.text_center_scaled(120, 96, brand::WORDMARK, pixel::CREAM_555, 2);
    let line = match empty {
        Empty::NoFolder => "ADD A FOLDER IN SETTINGS",
        Empty::Scanning => "LOOKING FOR GAMES…",
        Empty::NoGames => "NO GAMES YET",
    };
    o.text_center(120, 124, line, pixel::DIM_555);
    cv.blit_indexed(&fb[..], SCREEN_W, SCREEN_H, crate::pixels::bgr555_lut(), x as i32, y as i32, 1);
    cv.stroke_rect(x - 0.75, y - 0.75, SCREEN_W as f32 + 1.5, SCREEN_H as f32 + 1.5, 1.5, pal.fg);
}

/// The Strawberry cartridge standing in the handheld: a grey shell with the mark and the
/// wordmark on a light label.
fn draw_brand_cart(cv: &mut Canvas, r: Rect, pal: &Palette) {
    cv.round_rect(r.x, r.y, r.w, r.h, comp::CART_RADIUS, object::CART_SHELL);
    cv.fill_rect(r.x + 6.0, r.y, r.w - 12.0, 1.5, object::CART_SHELL_HIGHLIGHT);
    draw_grip(cv, r);
    let lab = Rect::new(r.x + 9.0, r.y + 21.0, r.w - 18.0, 40.0);
    cv.round_rect(lab.x, lab.y, lab.w, lab.h, [comp::CART_LABEL_RADIUS; 4], object::CART_SHELL_HIGHLIGHT);
    let st = Style::display(14.0).color(pal.fg);
    let tw = cv.fonts.measure(&st, brand::WORDMARK);
    let sprite = brand::SPRITE as f32;
    let x0 = lab.x + (lab.w - (sprite + 6.0 + tw)) / 2.0;
    cv.blit_indexed(&brand::mark().paper_555(), brand::SPRITE, brand::SPRITE, crate::pixels::bgr555_lut(), x0.round() as i32, (lab.y + 12.0) as i32, 1);
    cv.text(x0 + sprite + 6.0, lab.y + 11.0, brand::WORDMARK, &st);
}

/// A dashed placeholder cell of the empty case; cell 1 carries "EMPTY" and the hint.
fn draw_empty_cell(cv: &mut Canvas, cell: Rect, hint: Option<&str>, pal: &Palette) {
    let _ = pal;
    let r = Rect::new(cell.x + 7.0, cell.y - 7.0, cell.w - 14.0, cell.h + 4.0);
    cv.dashed_rect(r.x, r.y, r.w, r.h, 1.5, 4.0, SLOT_DASH);
    if let Some(hint) = hint {
        let cx = r.x + r.w / 2.0;
        let cy = r.y + r.h / 2.0;
        let label = Style::label().color(SLOT_TEXT);
        cv.text_center(cx, cy - 24.0, "EMPTY", &label);
        let lines = cv.fonts.wrap(&label, hint, r.w - 24.0, 2);
        for (i, l) in lines.iter().enumerate() {
            cv.text_center(cx, cy - 4.0 + i as f32 * 15.0, l, &label);
        }
    }
}

// --- drawing helpers --------------------------------------------------------------------------

const GREY_LABEL: (Rgb, Rgb) = ([0x6E, 0x6E, 0x74], [0xE8, 0xE8, 0xE8]);
const SLOT_DASH: Rgb = [0x4A, 0x46, 0x40];
const SLOT_TEXT: Rgb = [0x9C, 0x95, 0x8A];
const SLOT_CODE: Rgb = [0x6B, 0x66, 0x5C];

fn rgba(c: Rgb, a: u8) -> Paint<'static> {
    let mut p = Paint::default();
    p.set_color(Color::from_rgba8(c[0], c[1], c[2], a));
    p.anti_alias = true;
    p
}

/// The case block with its soft shadow, the foam and the foam's dot grid, on the paper colour.
/// `pad` is the margin around the block that the shadow may use.
fn build_case(w: f32, h: f32, pad: f32) -> Pixmap {
    let mut pix = Pixmap::new((w + 2.0 * pad) as u32, (h + 2.0 * pad) as u32).expect("case pixmap");
    pix.fill(Color::from_rgba8(PAPER.bg[0], PAPER.bg[1], PAPER.bg[2], 255));
    let fill = |pix: &mut Pixmap, x: f32, y: f32, w: f32, h: f32, r: f32, p: &Paint| {
        if let Some(path) = crate::ui::canvas::round_rect_path(x, y, w, h, [r; 4]) {
            pix.fill_path(&path, p, tiny_skia::FillRule::Winding, Transform::identity(), None);
        }
    };
    // soft shadow: stacked translucent blocks growing outwards, nudged down
    for k in (1..=12).rev() {
        let e = k as f32;
        fill(&mut pix, pad - e + 0.0, pad - e + 10.0, w + 2.0 * e, h + 2.0 * e, 26.0 + e, &rgba([0, 0, 0], 3));
    }
    fill(&mut pix, pad, pad, w, h, 26.0, &rgba(object::CASE_OUTER, 255));
    fill(&mut pix, pad + FOAM_INSET, pad + FOAM_INSET, w - 2.0 * FOAM_INSET, h - 2.0 * FOAM_INSET, 16.0, &rgba(object::FOAM, 255));
    // foam dot grid: 4 px pitch (the dots are close to the outer colour, so the corners need no mask)
    let mut dot = rgba(object::FOAM_DOT, 255);
    dot.anti_alias = false;
    let (x0, y0) = (pad + FOAM_INSET + 4.0, pad + FOAM_INSET + 4.0);
    let (x1, y1) = (pad + w - FOAM_INSET - 4.0, pad + h - FOAM_INSET - 4.0);
    let mut y = y0;
    while y < y1 {
        let mut x = x0;
        while x < x1 {
            if let Some(r) = tiny_skia::Rect::from_xywh(x, y, 2.0, 2.0) {
                pix.fill_rect(r, &dot, Transform::identity(), None);
            }
            x += 4.0;
        }
        y += 4.0;
    }
    pix
}

/// The "in the console" slot; `code` is the game's code, or the product name when the library
/// is empty (the system program is what is in the console).
fn draw_slot(cv: &mut Canvas, cell: Rect, code: Option<&str>, selected: bool, pal: &Palette) {
    let r = Rect::new(cell.x + 7.0, cell.y - 7.0, cell.w - 14.0, cell.h + 4.0);
    cv.fill_rect(r.x, r.y, r.w, r.h, mix(object::FOAM, [0, 0, 0], 0.22));
    cv.dashed_rect(r.x, r.y, r.w, r.h, 1.5, 4.0, if selected { pal.spot } else { SLOT_DASH });
    let cx = r.x + r.w / 2.0;
    let label = Style::label().color(SLOT_TEXT);
    let cy = r.y + r.h / 2.0;
    cv.text_center(cx, cy - 14.0, "IN THE CONSOLE", &label);
    if let Some(code) = code {
        cv.text_center(cx, cy + 4.0, code, &Style::data(fnt::DATA_SMALL).color(SLOT_CODE));
    }
}

/// One cartridge at `r` (already lifted).
fn draw_cart(cv: &mut Canvas, r: Rect, e: &LibraryEntry, strip: Option<&[u8]>, selected: bool, lifted: f32, pal: &Palette) {
    let shadow = mix(object::FOAM, [0, 0, 0], 0.55);
    cv.round_rect(r.x, r.y + 3.0 + lifted, r.w, r.h, comp::CART_RADIUS, shadow);
    cv.round_rect(r.x, r.y, r.w, r.h, comp::CART_RADIUS, object::CART_SHELL);
    cv.fill_rect(r.x + 6.0, r.y, r.w - 12.0, 1.5, object::CART_SHELL_HIGHLIGHT);
    cv.fill_rect(r.x + 3.0, r.y + r.h - 2.0, r.w - 6.0, 2.0, object::CART_SHELL_SHADOW);
    draw_grip(cv, r);
    if selected {
        cv.stroke_round_rect(r.x - 1.0, r.y - 1.0, r.w + 2.0, r.h + 2.0, comp::CART_RADIUS.map(|v| v + 1.0), 2.0, pal.spot);
    }

    let ins = comp::CART_LABEL_INSET;
    let lab = Rect::new(r.x + ins[3], r.y + ins[0], r.w - ins[1] - ins[3], r.h - ins[0] - ins[2]);
    let (bg, ink) = if e.unreadable.is_some() { GREY_LABEL } else { label_for(&e.code) };
    cv.round_rect(lab.x, lab.y, lab.w, lab.h, [comp::CART_LABEL_RADIUS; 4], bg);

    if e.unreadable.is_some() {
        let st = Style::display(12.0).color(ink);
        let (size, lines) = cv.fonts.fit_display(st, &e.file_name(), lab.w - 12.0, 3, &[12.0, 11.0, 10.0]);
        for (i, l) in lines.iter().enumerate() {
            cv.text(lab.x + 6.0, lab.y + 6.0 + i as f32 * size * 0.95, l, &st.size(size));
        }
        cv.text(lab.x + 6.0, lab.y + lab.h - 17.0, "UNREADABLE", &Style::data(fnt::DATA_SMALL - 1.0).color(ink));
        return;
    }

    // art strip at the left; a tint of the label when the game has no picture yet
    match strip {
        Some(s) => cv.blit_rgb(s, STRIP_W, lab.h as usize, lab.x as i32, lab.y as i32),
        None => cv.round_rect(lab.x, lab.y, STRIP_W as f32, lab.h, [comp::CART_LABEL_RADIUS, 0.0, 0.0, comp::CART_LABEL_RADIUS], mix(bg, ink, 0.12)),
    }
    let tx = lab.x + STRIP_W as f32 + 6.0;
    let tw = lab.x + lab.w - 3.0 - tx;
    let title = Style::display(fnt::DISPLAY_CART).color(ink);
    let (size, lines) = cv.fonts.fit_display(title, &e.title, tw, 2, &[13.0, 12.0, 11.0]);
    let lh = size * fnt::DISPLAY_LINE_HEIGHT;
    let mut y = lab.y + 5.0;
    for l in &lines {
        cv.text(tx, y, l, &title.size(size));
        y += lh;
    }
    y += 3.0;
    if !e.subtitle.is_empty() {
        let st = Style::interface_bold(8.5).color(ink);
        let sub = cv.fonts.ellipsize(&st, &e.subtitle, tw);
        cv.text(tx, y, &sub, &st);
    }
    let code = Style::data(8.4).color(ink);
    let line = cv.fonts.ellipsize(&code, &format!("{} · {}", e.code, crate::gba_copy::abbrev_save(crate::gba_copy::save_type(&e.save))), tw);
    cv.text(tx, lab.y + lab.h - 14.0, &line, &code);
}

/// Alternating 2 px stripes in the grip recess near the top edge.
fn draw_grip(cv: &mut Canvas, r: Rect) {
    let gx = r.x + comp::CART_GRIP_INSET;
    let gw = r.w - 2.0 * comp::CART_GRIP_INSET;
    let gy = r.y + comp::CART_GRIP_TOP;
    let stripes = (comp::CART_GRIP_H / 2.0).ceil() as usize;
    for i in 0..stripes {
        let h = (comp::CART_GRIP_H - 2.0 * i as f32).min(2.0);
        cv.fill_rect(gx, gy + 2.0 * i as f32, gw, h, if i % 2 == 0 { object::CART_GRIP_A } else { object::CART_GRIP_B });
    }
}

/// The cartridge standing in the handheld: only its top shows above the drawing.
fn draw_inserted_cart(cv: &mut Canvas, r: Rect, e: &LibraryEntry, pal: &Palette) {
    let _ = pal;
    cv.round_rect(r.x, r.y, r.w, r.h, comp::CART_RADIUS, object::CART_SHELL);
    cv.fill_rect(r.x + 6.0, r.y, r.w - 12.0, 1.5, object::CART_SHELL_HIGHLIGHT);
    draw_grip(cv, r);
    let (bg, ink) = if e.unreadable.is_some() { GREY_LABEL } else { label_for(&e.code) };
    let lab = Rect::new(r.x + 9.0, r.y + 21.0, r.w - 18.0, 40.0);
    cv.round_rect(lab.x, lab.y, lab.w, lab.h, [comp::CART_LABEL_RADIUS; 4], bg);
    let text = if e.unreadable.is_some() { e.file_name() } else { e.title.clone() };
    let st = Style::display(16.0).color(ink);
    let (size, lines) = cv.fonts.fit_display(st, &text, lab.w - 12.0, 1, &[16.0, 14.0, 12.0, 11.0]);
    if let Some(l) = lines.first() {
        cv.text_center(lab.x + lab.w / 2.0, lab.y + 8.0, l, &st.size(size));
    }
}

/// The game's picture on the handheld's screen at 1x with a 1.5 px ink outline, or a plain
/// rectangle with the code when there is no art yet.
fn draw_picture(cv: &mut Canvas, x: f32, y: f32, e: &LibraryEntry, art: Option<&Art>, pal: &Palette) {
    let (pw, ph) = (gba_core::SCREEN_W, gba_core::SCREEN_H);
    match art {
        Some(a) if e.unreadable.is_none() => {
            if (a.w, a.h) == (pw, ph) {
                cv.blit_rgb(&a.rgb, pw, ph, x as i32, y as i32);
            } else {
                cv.blit_rgb(&a.cover(pw, ph), pw, ph, x as i32, y as i32);
            }
        }
        _ => {
            cv.fill_rect(x, y, pw as f32, ph as f32, pal.bg2);
            let text = if e.unreadable.is_some() { "UNREADABLE" } else { e.code.as_str() };
            let st = Style::label().color(pal.fg2);
            cv.text_center(x + pw as f32 / 2.0, y + ph as f32 / 2.0 - 7.0, text, &st);
        }
    }
    cv.stroke_rect(x - 0.75, y - 0.75, pw as f32 + 1.5, ph as f32 + 1.5, 1.5, pal.fg);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::input::UiInput;
    use crate::ui::text::Fonts;

    fn entry(dir: &Path, title: &str, code: &str) -> LibraryEntry {
        LibraryEntry {
            path: dir.join(format!("{title}.gba")),
            size: 1,
            mtime: 1,
            title: title.into(),
            subtitle: "USA".into(),
            code: code.into(),
            version: 0,
            save: "sram32k".into(),
            rom_len: 32 * 1024 * 1024,
            unreadable: None,
        }
    }

    const NAMES: [(&str, &str); 7] =
        [("Alpha", "AAAA"), ("Bravo", "BBBB"), ("Charlie", "CCCC"), ("Delta", "DDDD"), ("Echo", "EEEE"), ("Foxtrot Zelda", "FFFF"), ("Golf", "GGGG")];

    fn temp(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("gba-libscreen-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    fn lib_with(dir: &Path, extra: impl FnOnce(&mut Vec<LibraryEntry>)) -> Library {
        let mut v: Vec<LibraryEntry> = NAMES.iter().map(|(t, c)| entry(dir, t, c)).collect();
        extra(&mut v);
        Library::from_entries(vec![dir.to_path_buf()], v)
    }

    fn screen() -> LibraryScreen {
        let mut s = LibraryScreen::new();
        s.slot_delay = Duration::ZERO;
        s
    }

    fn act(a: &[Action]) -> UiInput {
        UiInput { actions: a.to_vec(), ..UiInput::default() }
    }

    fn typed(s: &str) -> UiInput {
        UiInput { chars: s.chars().collect(), ..UiInput::default() }
    }

    fn key(k: Key) -> UiInput {
        UiInput { pressed: vec![k], repeated: vec![k], ..UiInput::default() }
    }

    fn sel_title(s: &LibraryScreen, lib: &Library) -> String {
        s.selected_entry(lib).map(|e| e.title.clone()).unwrap_or_default()
    }

    #[test]
    fn grid_moves_with_four_columns() {
        // 10 cells: rows of 4, 4, 2
        assert_eq!(move_cell(0, 10, 4, 1, 0), 1);
        assert_eq!(move_cell(0, 10, 4, -1, 0), 0, "no wrap before the first cell");
        assert_eq!(move_cell(3, 10, 4, 1, 0), 4, "right wraps to the next row");
        assert_eq!(move_cell(0, 10, 4, 0, 1), 4);
        assert_eq!(move_cell(5, 10, 4, 0, 1), 9);
        assert_eq!(move_cell(6, 10, 4, 0, 1), 9, "below is missing: land on the last cell");
        assert_eq!(move_cell(9, 10, 4, 0, 1), 9, "bottom row stays");
        assert_eq!(move_cell(9, 10, 4, 1, 0), 9);
        assert_eq!(move_cell(5, 10, 4, 0, -1), 1);
        assert_eq!(move_cell(2, 10, 4, 0, -1), 2, "top row stays");
    }

    #[test]
    fn grid_moves_with_three_columns() {
        // 10 cells: rows of 3, 3, 3, 1
        assert_eq!(move_cell(0, 10, 3, 0, 1), 3);
        assert_eq!(move_cell(2, 10, 3, 1, 0), 3);
        assert_eq!(move_cell(7, 10, 3, 0, 1), 9, "7+3 = 10 is past the end: last cell");
        assert_eq!(move_cell(8, 10, 3, 0, 1), 9);
        assert_eq!(move_cell(9, 10, 3, 0, -1), 6);
        assert_eq!(move_cell(9, 10, 3, 0, 1), 9);
        assert_eq!(move_cell(4, 10, 3, 0, -1), 1);
        assert_eq!(move_cell(0, 1, 3, 0, 1), 0);
        assert_eq!(move_cell(0, 0, 3, 1, 1), 0);
    }

    #[test]
    fn geometry_columns_follow_the_width() {
        assert_eq!(geometry(1280.0, 800.0, 12).cols, 4);
        assert_eq!(geometry(1920.0, 1080.0, 12).cols, 7);
        assert_eq!(geometry(1114.0, 800.0, 12).cols, 3);
        let narrow = geometry(960.0, 640.0, 12);
        assert_eq!(narrow.cols, 2, "the 436 px handheld column leaves room for two at 960");
        // 12 cells on 4 columns = 3 rows: the case is exactly as tall as in the mockup
        let g = geometry(1280.0, 800.0, 12);
        assert_eq!((g.case.x, g.case.y, g.case.w, g.case.h), (40.0, 82.0, 720.0, 522.0));
        assert_eq!(g.max_scroll, 0.0);
        assert_eq!(g.cell_rect(1, 0.0).x.round(), 239.0);
        assert!(geometry(960.0, 640.0, 40).max_scroll > 0.0);
    }

    #[test]
    fn selection_walks_the_grid_and_skips_an_empty_slot() {
        let dir = temp("nav");
        let lib = lib_with(&dir, |_| {});
        let mut s = screen();
        s.set_columns(3);
        // nothing played: the slot is empty, cells are the 7 games; the first is selected
        s.update(&UiInput::default(), &lib);
        assert_eq!(sel_title(&s, &lib), "Alpha");
        s.update(&act(&[Action::Right]), &lib);
        assert_eq!(sel_title(&s, &lib), "Bravo");
        s.update(&act(&[Action::Down]), &lib);
        // cells: 0 slot, 1 Alpha, 2 Bravo, 3 Charlie, 4 Delta, 5 Echo, 6 Foxtrot, 7 Golf; Bravo=2, 3 columns -> 5
        assert_eq!(sel_title(&s, &lib), "Echo");
        s.update(&act(&[Action::Down]), &lib);
        assert_eq!(sel_title(&s, &lib), "Golf", "5+3 = 8 is past the end, last cell");
        s.update(&act(&[Action::Up]), &lib);
        assert_eq!(sel_title(&s, &lib), "Delta");
        s.update(&act(&[Action::Left]), &lib);
        assert_eq!(sel_title(&s, &lib), "Charlie");
        for _ in 0..3 {
            s.update(&act(&[Action::Left]), &lib);
        }
        assert_eq!(sel_title(&s, &lib), "Alpha", "the empty slot is not selectable");
        // four columns: one row down is four cells on
        s.set_columns(4);
        s.update(&act(&[Action::Down]), &lib);
        assert_eq!(sel_title(&s, &lib), "Echo");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn last_played_sits_in_the_console_slot_and_starts_selected() {
        let dir = temp("console");
        let mut lib = lib_with(&dir, |_| {});
        let echo = dir.join("Echo.gba");
        lib.mark_played_at(&echo, 100);
        let mut s = screen();
        s.update(&UiInput::default(), &lib);
        assert_eq!(sel_title(&s, &lib), "Echo");
        let grid = s.grid(&lib);
        assert_eq!(grid.len(), 7, "slot + the six others: Echo is not in the grid twice");
        assert_eq!(s.sel_cell(&grid, &lib), Some(0));
        // Right from the slot goes to the first cartridge
        s.update(&act(&[Action::Right]), &lib);
        assert_eq!(sel_title(&s, &lib), "Alpha");
        // Back to the slot with Left
        s.update(&act(&[Action::Left]), &lib);
        assert_eq!(sel_title(&s, &lib), "Echo");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn find_filters_by_title_and_code_case_insensitively() {
        let dir = temp("find");
        let lib = lib_with(&dir, |_| {});
        let mut s = screen();
        s.update(&UiInput::default(), &lib);
        s.update(&typed("CHAR"), &lib);
        assert_eq!(s.find_text(), "CHAR");
        assert_eq!(s.grid(&lib).carts.len(), 1);
        assert_eq!(sel_title(&s, &lib), "Charlie", "the selection moved to the match");
        s.update(&key(Key::Escape), &lib);
        assert_eq!(s.find_text(), "");
        assert_eq!(s.grid(&lib).carts.len(), 7);
        s.update(&typed("ee"), &lib);
        assert_eq!(s.grid(&lib).carts.len(), 1, "code EEEE");
        assert_eq!(sel_title(&s, &lib), "Echo");
        s.update(&key(Key::Backspace), &lib);
        assert_eq!(s.find_text(), "e");
        assert_eq!(s.grid(&lib).carts.len(), 4, "Charlie, Delta, Echo and Foxtrot Zelda contain an e");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn bound_letters_play_instead_of_typing_until_find_has_focus() {
        let dir = temp("focus");
        let lib = lib_with(&dir, |_| {});
        let mut s = screen();
        s.update(&UiInput::default(), &lib);
        // 'z' arrives as a char and as the Confirm action: it plays
        let z = UiInput { chars: vec!['z'], actions: vec![Action::Confirm], ..UiInput::default() };
        assert_eq!(s.update(&z, &lib), LibraryOutcome::Play(dir.join("Alpha.gba")));
        assert_eq!(s.find_text(), "");
        // '/' focuses without typing; then z goes into Find and does not play
        s.update(&typed("/"), &lib);
        assert_eq!(s.find_text(), "");
        assert_eq!(s.update(&z, &lib), LibraryOutcome::None);
        assert_eq!(s.find_text(), "z");
        assert_eq!(s.grid(&lib).carts.len(), 1, "Foxtrot Zelda");
        // Enter still plays
        assert_eq!(s.update(&key(Key::Enter), &lib), LibraryOutcome::Play(dir.join("Foxtrot Zelda.gba")));
        // X while focused types too, it does not eject
        let x = UiInput { chars: vec!['x'], actions: vec![Action::Back], ..UiInput::default() };
        s.update(&x, &lib);
        assert_eq!(s.find_text(), "zx");
        assert!(s.selected().is_some() || s.grid(&lib).carts.is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn escape_clears_find_then_quits() {
        let dir = temp("esc");
        let lib = lib_with(&dir, |_| {});
        let mut s = screen();
        s.update(&typed("gol"), &lib);
        assert_eq!(s.update(&key(Key::Escape), &lib), LibraryOutcome::None);
        assert_eq!(s.find_text(), "");
        assert!(!s.find_focus);
        assert_eq!(s.update(&key(Key::Escape), &lib), LibraryOutcome::Quit);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn eject_empties_the_console_and_confirm_then_does_nothing() {
        let dir = temp("eject");
        let mut lib = lib_with(&dir, |_| {});
        lib.mark_played_at(&dir.join("Bravo.gba"), 50);
        let mut s = screen();
        s.update(&UiInput::default(), &lib);
        assert_eq!(sel_title(&s, &lib), "Bravo");
        assert_eq!(s.update(&act(&[Action::Back]), &lib), LibraryOutcome::None);
        assert_eq!(s.selected(), None);
        assert_eq!(s.grid(&lib).console, None, "the slot is empty");
        assert_eq!(s.grid(&lib).carts.len(), 7, "Bravo is back among the cartridges");
        assert_eq!(s.update(&act(&[Action::Confirm]), &lib), LibraryOutcome::None);
        assert_eq!(s.update(&key(Key::F2), &lib), LibraryOutcome::None);
        // an arrow selects the first cartridge
        s.update(&act(&[Action::Right]), &lib);
        assert_eq!(sel_title(&s, &lib), "Alpha");
        // playing something refills the console
        lib.mark_played_at(&dir.join("Golf.gba"), 90);
        s.update(&UiInput::default(), &lib);
        assert_eq!(s.grid(&lib).console, lib.entries().iter().position(|e| e.title == "Golf"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn f2_f3_and_unreadable_roms() {
        let dir = temp("keys");
        let lib = lib_with(&dir, |v| {
            let mut bad = entry(&dir, "Aaa Broken", "");
            bad.unreadable = Some("no .gba entry in Aaa Broken.gba".into());
            bad.code.clear();
            v.push(bad);
        });
        let mut s = screen();
        s.update(&UiInput::default(), &lib);
        assert_eq!(sel_title(&s, &lib), "Aaa Broken", "sorted first");
        assert_eq!(s.update(&act(&[Action::Confirm]), &lib), LibraryOutcome::None, "unreadable cannot be played");
        assert_eq!(s.update(&key(Key::F3), &lib), LibraryOutcome::None);
        assert_eq!(s.update(&key(Key::F2), &lib), LibraryOutcome::OpenStates(dir.join("Aaa Broken.gba")));
        // a game with a save state resumes
        s.select(&dir.join("Delta.gba"));
        assert_eq!(s.update(&key(Key::F3), &lib), LibraryOutcome::None, "no state yet");
        let fb = vec![0x001Fu16; gba_core::SCREEN_W * gba_core::SCREEN_H];
        std::fs::write(crate::slots::slot_path(&dir.join("Delta.gba"), 2), crate::glue::GBA_SLOTS.encode(27_400, 1_790_000_000, 458, &fb, b"body")).unwrap();
        s.refresh_slots();
        assert_eq!(s.update(&key(Key::F3), &lib), LibraryOutcome::ResumeLatest(dir.join("Delta.gba")));
        assert_eq!(s.latest_for_selected().unwrap().frame, 27_400);
        assert_eq!(s.update(&act(&[Action::Start]), &lib), LibraryOutcome::Play(dir.join("Delta.gba")));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn render_lays_out_and_the_mouse_selects_then_plays() {
        let dir = temp("mouse");
        let mut lib = lib_with(&dir, |_| {});
        lib.mark_played_at(&dir.join("Echo.gba"), 10);
        let mut cv = Canvas::new(1280, 800, Fonts::new());
        let mut s = screen();
        s.update(&UiInput::default(), &lib);
        s.render(&mut cv, &lib);
        assert_eq!(s.columns(), 4);
        assert_eq!(cv.pixel(5, 5), PAPER.bg);
        // the Play button is solid ink; there is no Resume button without a state
        let play = s.play_btn.expect("play button");
        assert_eq!(cv.pixel((play.x + 6.0) as u32, (play.y + 6.0) as u32), PAPER.fg);
        assert!(s.resume_btn.is_none());
        // click the first cartridge: selects; click again: plays
        let (rect, path) = s.hits.iter().find(|(_, p)| p.ends_with("Alpha.gba")).cloned().expect("Alpha is in view");
        let click = UiInput { mouse: (rect.x + 20.0, rect.y + 60.0), clicked: true, ..UiInput::default() };
        assert_eq!(s.update(&click, &lib), LibraryOutcome::None);
        assert_eq!(s.selected(), Some(path.as_path()));
        assert_eq!(s.update(&click, &lib), LibraryOutcome::Play(path.clone()));
        // hovering lifts: after some updates the lift for that cart is above zero
        let hover = UiInput { mouse: (rect.x + 20.0, rect.y + 60.0), ..UiInput::default() };
        std::thread::sleep(Duration::from_millis(30));
        s.update(&hover, &lib);
        assert!(s.lifts.get(&path).copied().unwrap_or(0.0) > 0.0);
        // narrow window: two columns
        let mut small = Canvas::new(960, 640, Fonts::new());
        s.render(&mut small, &lib);
        assert_eq!(s.columns(), 2);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn empty_library_shows_the_brand_and_offers_the_folder_and_settings() {
        let dir = temp("empty");
        let lib = Library::from_entries(vec![dir.clone()], Vec::new());
        let mut cv = Canvas::new(1280, 800, Fonts::new());
        let mut s = screen();
        // Z opens the folder even before the first render
        assert_eq!(s.update(&act(&[Action::Right, Action::Confirm]), &lib), LibraryOutcome::OpenFolder(dir.clone()));
        s.render(&mut cv, &lib);
        assert!(s.hits.is_empty());
        assert!(s.play_btn.is_none());
        let (open, settings) = (s.open_btn.expect("Open button"), s.settings_btn.expect("Settings button"));
        let mut click = UiInput { mouse: (settings.x + 6.0, settings.y + 6.0), clicked: true, ..UiInput::default() };
        assert_eq!(s.update(&click, &lib), LibraryOutcome::OpenSettings);
        click.mouse = (open.x + 6.0, open.y + 6.0);
        assert_eq!(s.update(&click, &lib), LibraryOutcome::OpenFolder(dir.clone()));
        // three rows of placeholders keep the mockup's case height
        assert_eq!((s.case_rect.x, s.case_rect.y, s.case_rect.w, s.case_rect.h), (40.0, 82.0, 720.0, 522.0));
        // the handheld's screen is ink with the night mark at 3x: its stem is cream
        let g = geometry(1280.0, 800.0, 12);
        let (px, py) = (g.side.x as u32 + 98, g.side.y as u32 + 92);
        assert_eq!(cv.pixel(px + 1, py + 1), gba_test::bgr555_to_rgb(pixel::INK_555));
        assert_eq!(cv.pixel(px + 96 + 7 * 3 + 1, py + 24 + 1), gba_test::bgr555_to_rgb(pixel::CREAM_555));
        assert_eq!(cv.pixel(px + 96 + 3 * 3 + 1, py + 24 + 7 * 3 + 1), gba_test::bgr555_to_rgb(pixel::RED_555));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn empty_library_without_a_folder_offers_settings_only() {
        let lib = Library::from_entries(Vec::new(), Vec::new());
        let mut cv = Canvas::new(1280, 800, Fonts::new());
        let mut s = screen();
        assert_eq!(s.update(&act(&[Action::Confirm]), &lib), LibraryOutcome::OpenSettings);
        s.render(&mut cv, &lib);
        assert!(s.open_btn.is_none());
        assert!(s.settings_btn.is_some());
        assert_eq!(s.update(&key(Key::Enter), &lib), LibraryOutcome::OpenSettings);
        // and the narrow window still lays out
        let mut small = Canvas::new(960, 640, Fonts::new());
        s.render(&mut small, &lib);
        assert_eq!(s.columns(), 2);
    }

    /// Review render: `cargo test -p gba-frontend --release -- --ignored render_library`.
    /// Uses `GBA_GAMES_DIR` or `<repo>/Games` when present (waits for the scan and for the title
    /// captures, cached under `target/ui-renders/library-cache`), else synthetic cartridges.
    #[test]
    #[ignore]
    fn render_library() {
        let games = std::env::var_os("GBA_GAMES_DIR").map(PathBuf::from).unwrap_or_else(|| gba_test::repo_root().join("Games"));
        let mut lib;
        if games.is_dir() {
            lib = Library::new(vec![games.clone()], Some(crate::render_tests::out_dir().join("library-cache")), std::sync::Arc::new(crate::glue::GbaPlatform));
            let start = Instant::now();
            while lib.is_scanning() && start.elapsed() < Duration::from_secs(10) {
                lib.poll();
                std::thread::sleep(Duration::from_millis(20));
            }
            // give the art thread time to find slot frames and make missing captures
            let patience = std::env::var("GBA_LIB_WAIT").ok().and_then(|v| v.parse().ok()).unwrap_or(150u64);
            while start.elapsed() < Duration::from_secs(patience) {
                lib.poll();
                let missing = lib.entries().iter().filter(|e| e.unreadable.is_none() && lib.art_for(&e.path).is_none()).count();
                if missing == 0 {
                    break;
                }
                std::thread::sleep(Duration::from_millis(100));
            }
            println!("{} entries from {}", lib.entries().len(), games.display());
        } else {
            let dir = PathBuf::from("Games");
            let list = [
                ("Kingdom Hearts", "Chain of Memories · USA", "B8CE"),
                ("The Legend of Zelda", "The Minish Cap · USA", "BZME"),
                ("The Legend of Zelda", "A Link to the Past & Four Swords · USA", "AZLE"),
                ("Initial D", "Another Stage · Japan", "AINJ"),
                ("Mario Kart", "Super Circuit · USA", "AMKE"),
                ("Final Fantasy VI Advance", "USA", "BZ6E"),
                ("Crash Bandicoot", "The Huge Adventure · USA", "ACQE"),
                ("Need for Speed", "Underground 2 · USA, Europe", "BNFE"),
            ];
            let entries = list
                .iter()
                .map(|(t, s, c)| {
                    let mut e = entry(&dir, t, c);
                    e.subtitle = s.to_string();
                    e.path = dir.join(format!("{t} {c}.zip"));
                    e
                })
                .collect();
            lib = Library::from_entries(vec![dir], entries);
        }
        // KH was played last, as in the mockup
        if let Some(kh) = lib.entries().iter().find(|e| e.code == "B8CE").map(|e| e.path.clone()) {
            lib.mark_played_at(&kh, 1_790_000_000);
        }
        let mut s = screen();
        for (name, w, h) in [("01-library.png", 1280, 800), ("01-library-960.png", 960, 640)] {
            let mut cv = Canvas::new(w, h, Fonts::new());
            s.update(&UiInput::default(), &lib);
            s.render(&mut cv, &lib);
            let path = crate::render_tests::out_dir().join(name);
            cv.save_png(&path).unwrap();
            println!("wrote {}", path.display());
        }
    }

    #[test]
    fn pinned_library_renders() {
        let dir = PathBuf::from("Games");
        let list = [
            ("Kingdom Hearts", "Chain of Memories · USA", "B8CE"),
            ("The Legend of Zelda", "The Minish Cap · USA", "BZME"),
            ("The Legend of Zelda", "A Link to the Past & Four Swords · USA", "AZLE"),
            ("Initial D", "Another Stage · Japan", "AINJ"),
            ("Mario Kart", "Super Circuit · USA", "AMKE"),
            ("Final Fantasy VI Advance", "USA", "BZ6E"),
            ("Crash Bandicoot", "The Huge Adventure · USA", "ACQE"),
            ("Need for Speed Underground 2 Special Collectors Edition", "A very long subtitle that cannot possibly fit · USA, Europe", "BNFE"),
        ];
        let mut entries: Vec<LibraryEntry> = list
            .iter()
            .map(|(t, s, c)| {
                let mut e = entry(&dir, t, c);
                e.subtitle = s.to_string();
                e.path = dir.join(format!("{t} {c}.zip"));
                e
            })
            .collect();
        entries[7].save = "flash128k".into();
        let mut bad = entry(&dir, "Broken Download (USA)", "");
        bad.unreadable = Some("no .gba entry in Broken Download (USA).zip".into());
        entries.push(bad);
        let mut lib = Library::from_entries(vec![dir.clone()], entries);
        let mut rgb = Vec::new();
        for y in 0..160usize {
            for x in 0..240usize {
                rgb.extend_from_slice(&[x as u8, y as u8, 128]);
            }
        }
        lib.set_art(&dir.join("Mario Kart AMKE.zip"), Art::new(240, 160, rgb.clone()));
        lib.set_art(&dir.join("Initial D AINJ.zip"), Art::new(240, 160, rgb));
        lib.mark_played_at(&dir.join("Kingdom Hearts B8CE.zip"), 1_790_000_000);
        let mut s = screen();
        for (name, w, h) in [("library", 1280, 800), ("library-960", 960, 640)] {
            let mut cv = Canvas::new(w, h, Fonts::new());
            s.update(&UiInput::default(), &lib);
            s.render(&mut cv, &lib);
            crate::pinned::check_canvas(name, &cv);
        }
        let mut cv = Canvas::new(1280, 800, Fonts::new());
        s.select(&dir.join("Broken Download (USA).gba"));
        s.update(&UiInput::default(), &lib);
        s.render(&mut cv, &lib);
        crate::pinned::check_canvas("library-unreadable", &cv);
        let empty = Library::from_entries(vec![PathBuf::from("Games")], Vec::new());
        let mut s2 = screen();
        s2.update(&UiInput::default(), &empty);
        s2.render(&mut cv, &empty);
        crate::pinned::check_canvas("library-empty", &cv);
        let no_folder = Library::from_entries(Vec::new(), Vec::new());
        s2.update(&UiInput::default(), &no_folder);
        s2.render(&mut cv, &no_folder);
        crate::pinned::check_canvas("library-empty-nofolder", &cv);
    }

    /// Review render of the unusual states: unreadable and art-less carts, long titles, a selected
    /// game with a save state, an empty library. Writes `01-library-states.png` / `-empty.png`.
    #[test]
    #[ignore]
    fn render_library_states() {
        let dir = temp("states");
        let mut lib = lib_with(&dir, |v| {
            let mut bad = entry(&dir, "Broken Download (USA)", "");
            bad.unreadable = Some("no .gba entry in Broken Download (USA).zip".into());
            v.push(bad);
            let mut long = entry(&dir, "Need for Speed Underground 2 Special Collectors Edition", "BNFE");
            long.subtitle = "A very long subtitle that cannot possibly fit · USA, Europe".into();
            long.save = "flash128k".into();
            v.push(long);
        });
        // art for two of them: a gradient standing in for a screenshot
        let mut rgb = Vec::new();
        for y in 0..160usize {
            for x in 0..240usize {
                rgb.extend_from_slice(&[(x as u8).wrapping_mul(1), (y as u8).wrapping_mul(1), 128]);
            }
        }
        lib.set_art(&dir.join("Alpha.gba"), Art::new(240, 160, rgb.clone()));
        lib.set_art(&dir.join("Echo.gba"), Art::new(240, 160, rgb));
        lib.mark_played_at(&dir.join("Echo.gba"), 100);
        // Delta has a save state
        let fb: Vec<u16> = (0..gba_core::SCREEN_W * gba_core::SCREEN_H).map(|i| ((i / 240) as u16 * 3) & 0x7FFF).collect();
        std::fs::write(crate::slots::slot_path(&dir.join("Delta.gba"), 3), crate::glue::GBA_SLOTS.encode(27_400, 1_790_000_000, 458, &fb, b"body")).unwrap();
        let mut s = screen();
        s.update(&UiInput::default(), &lib);
        s.select(&dir.join("Delta.gba"));
        s.update(&UiInput::default(), &lib);
        let mut cv = Canvas::new(1280, 800, Fonts::new());
        s.render(&mut cv, &lib);
        cv.save_png(&crate::render_tests::out_dir().join("01-library-states.png")).unwrap();
        s.select(&dir.join("Broken Download (USA).gba"));
        s.update(&UiInput::default(), &lib);
        s.render(&mut cv, &lib);
        cv.save_png(&crate::render_tests::out_dir().join("01-library-unreadable.png")).unwrap();
        let empty = Library::from_entries(vec![PathBuf::from("Games")], Vec::new());
        let mut s2 = screen();
        s2.update(&UiInput::default(), &empty);
        s2.render(&mut cv, &empty);
        cv.save_png(&crate::render_tests::out_dir().join("01-library-empty.png")).unwrap();
        let no_folder = Library::from_entries(Vec::new(), Vec::new());
        s2.update(&UiInput::default(), &no_folder);
        s2.render(&mut cv, &no_folder);
        cv.save_png(&crate::render_tests::out_dir().join("01-library-empty-nofolder.png")).unwrap();
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn scrolling_keeps_the_selection_in_view_and_wheel_moves() {
        let dir = temp("scroll");
        let lib = lib_with(&dir, |v| {
            for i in 0..30 {
                v.push(entry(&dir, &format!("Game {i:02}"), &format!("G{i:03}")));
            }
        });
        let mut cv = Canvas::new(960, 640, Fonts::new());
        let mut s = screen();
        s.update(&UiInput::default(), &lib);
        s.render(&mut cv, &lib);
        assert!(s.max_scroll > 0.0);
        // walk down; the selection's cell must always be inside the viewport after a render
        for _ in 0..20 {
            s.update(&act(&[Action::Down]), &lib);
            s.render(&mut cv, &lib);
            s.scroll = s.scroll_target; // skip the easing
            s.render(&mut cv, &lib);
            let sel = s.selected().unwrap().to_path_buf();
            assert!(s.hits.iter().any(|(_, p)| *p == sel), "selected cart visible");
        }
        assert!(s.scroll > 0.0);
        let before = s.scroll_target;
        s.update(&UiInput { wheel: 1.0, ..UiInput::default() }, &lib);
        assert!(s.scroll_target < before, "wheel up scrolls up");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
