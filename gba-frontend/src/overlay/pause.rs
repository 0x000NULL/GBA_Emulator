//! 02 Paused: the pause menu drawn at native resolution inside the frame (`SCREENS.md`).
//!
//! Main window at (8, 8) 84x112 with RESUME / SAVE STATE / LOAD STATE / SETTINGS / BENCH / QUIT
//! (plus a SUN row after LOAD STATE on solar-sensor carts, changed in place with LEFT/RIGHT);
//! submenu at (100, 8) 132x112 with a 2x2 grid of slot thumbnails (pages of four, L/R to turn);
//! bar at (8, 128) 224x24 with the selected slot's summary and the two actions.

use super::{font5x5, Overlay, Thumb, THUMB_H, THUMB_W};
use crate::ui::fmt::fmt_play;
use crate::ui::input::{Action, UiInput};
use crate::ui::tokens::pixel::*;
use std::time::{Duration, Instant};

/// Rows of the main window.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Item {
    Resume,
    SaveState,
    LoadState,
    /// The solar sensor's light (Boktai carts only).
    Sun,
    Settings,
    Bench,
    Quit,
}

impl Item {
    pub fn label(self) -> &'static str {
        match self {
            Item::Resume => "RESUME",
            Item::SaveState => "SAVE STATE",
            Item::LoadState => "LOAD STATE",
            Item::Sun => "SUN",
            Item::Settings => "SETTINGS",
            Item::Bench => "BENCH",
            Item::Quit => "QUIT",
        }
    }
}

pub const ITEMS: [Item; 6] = [Item::Resume, Item::SaveState, Item::LoadState, Item::Settings, Item::Bench, Item::Quit];
pub const MESSAGE_TIME: Duration = Duration::from_millis(1500);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Main,
    Save,
    Load,
}

/// What the pause menu knows about a slot.
#[derive(Clone, Debug, PartialEq)]
pub struct SlotInfo {
    pub thumb: Box<Thumb>,
    pub play_secs: u64,
    pub frame: u64,
    /// `Some(reason)` when the file exists but cannot be loaded.
    pub broken: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Outcome {
    None,
    Resume,
    SaveSlot(u8),
    LoadSlot(u8),
    Settings,
    Bench,
    Quit,
    /// The SUN row moved to this level.
    Sun(u8),
}

#[derive(Clone, Debug)]
pub struct PauseMenu {
    pub items: Vec<Item>,
    /// Sun level shown on the SUN row (`Some` exactly when the row exists).
    pub sun: Option<u8>,
    pub sel: usize,
    pub mode: Mode,
    pub page: usize,
    pub slot_sel: usize,
    pub message: Option<(String, Instant)>,
}

impl Default for PauseMenu {
    fn default() -> Self {
        Self::new(0)
    }
}

impl PauseMenu {
    /// `slot` is the current slot (1..=8): the submenu opens on it.
    pub fn new(slot: u8) -> Self {
        let s = (slot.max(1) - 1) as usize;
        PauseMenu { items: ITEMS.to_vec(), sun: None, sel: 0, mode: Mode::Main, page: s / 4, slot_sel: s % 4, message: None }
    }

    /// Add the SUN row (after LOAD STATE) when the cart has a solar sensor at `level`.
    pub fn with_sun(mut self, level: Option<u8>) -> Self {
        if let (Some(l), None) = (level, self.sun) {
            self.sun = Some(l);
            self.items.insert(3, Item::Sun);
        }
        self
    }

    /// Slot number (1..=8) under the cursor.
    pub fn slot(&self) -> u8 {
        (self.page * 4 + self.slot_sel) as u8 + 1
    }

    pub fn show_message(&mut self, text: impl Into<String>, now: Instant) {
        self.message = Some((font5x5::truncate(&text.into()), now));
    }

    fn open_slots(&mut self, mode: Mode) {
        self.mode = mode;
    }

    /// React to this frame's input. `slots[i]` describes slot i+1.
    pub fn handle(&mut self, input: &UiInput, slots: &[Option<SlotInfo>; 8], now: Instant) -> Outcome {
        if let Some((_, t)) = self.message {
            if now.duration_since(t) >= MESSAGE_TIME {
                self.message = None;
            }
        }
        match self.mode {
            Mode::Main => {
                let n = self.items.len();
                if input.action(Action::Up) {
                    self.sel = (self.sel + n - 1) % n;
                }
                if input.action(Action::Down) {
                    self.sel = (self.sel + 1) % n;
                }
                if input.action(Action::Back) {
                    return Outcome::Resume;
                }
                let item = self.items[self.sel];
                if let (Item::Sun, Some(level)) = (item, self.sun) {
                    let next = if input.action(Action::Left) {
                        level.saturating_sub(1)
                    } else if input.action(Action::Right) {
                        (level + 1).min(gba_core::SUN_MAX)
                    } else {
                        level
                    };
                    if next != level {
                        self.sun = Some(next);
                        return Outcome::Sun(next);
                    }
                }
                if input.action(Action::Confirm) || input.action(Action::Start) {
                    return match item {
                        Item::Resume => Outcome::Resume,
                        Item::SaveState => {
                            self.open_slots(Mode::Save);
                            Outcome::None
                        }
                        Item::LoadState => {
                            self.open_slots(Mode::Load);
                            Outcome::None
                        }
                        Item::Sun => Outcome::None,
                        Item::Settings => Outcome::Settings,
                        Item::Bench => Outcome::Bench,
                        Item::Quit => Outcome::Quit,
                    };
                }
                Outcome::None
            }
            Mode::Save | Mode::Load => {
                if input.action(Action::Back) {
                    self.mode = Mode::Main;
                    return Outcome::None;
                }
                if input.action(Action::PrevPage) {
                    self.page = (self.page + 1) % 2;
                }
                if input.action(Action::NextPage) {
                    self.page = (self.page + 1) % 2;
                }
                if input.action(Action::Left) || input.action(Action::Right) {
                    self.slot_sel ^= 1;
                }
                if input.action(Action::Up) || input.action(Action::Down) {
                    self.slot_sel ^= 2;
                }
                if input.action(Action::Confirm) {
                    let n = self.slot();
                    let info = &slots[n as usize - 1];
                    return match self.mode {
                        Mode::Save => Outcome::SaveSlot(n),
                        _ => match info {
                            Some(i) if i.broken.is_none() => Outcome::LoadSlot(n),
                            _ => Outcome::None,
                        },
                    };
                }
                Outcome::None
            }
        }
    }

    /// Draw the menu over an already dimmed frame.
    pub fn draw(&self, o: &mut Overlay, slots: &[Option<SlotInfo>; 8], now: Instant) {
        // main window
        let (mx, my) = o.window(8, 8, 84, 112);
        for (i, item) in self.items.iter().enumerate() {
            let y = my + 4 + i as i32 * MENU_PITCH;
            let selected = i == self.sel && self.mode == Mode::Main;
            let dimmed = self.mode != Mode::Main;
            let c = if selected { RED_555 } else if dimmed { DIM_555 } else { INK_555 };
            if selected {
                o.cursor(mx, y + 2, RED_555);
            }
            o.text(mx + 8, y, item.label(), c);
            if let (Item::Sun, Some(level)) = (item, self.sun) {
                let v = if selected { format!("◂ {level} ▸") } else { level.to_string() };
                o.text_right(mx + 84 - 14, y, &v, c);
            }
        }

        // slot submenu
        if self.mode != Mode::Main {
            let (sx, sy) = o.window(100, 8, 132, 112);
            let title = if self.mode == Mode::Save { "SAVE TO SLOT" } else { "LOAD SLOT" };
            o.text(sx, sy, title, INK_555);
            for k in 0..4 {
                let idx = self.page * 4 + k;
                let n = idx + 1;
                let col = (k % 2) as i32;
                let row = (k / 2) as i32;
                let tx = sx + 1 + col * (THUMB_W as i32 + 22);
                let ty = sy + 10 + row * (THUMB_H as i32 + 12);
                let selected = k == self.slot_sel;
                match &slots[idx] {
                    Some(info) => {
                        o.thumb(tx, ty, &info.thumb, selected);
                        let label_c = if selected { RED_555 } else { INK_555 };
                        o.text(tx, ty + THUMB_H as i32 + 3, &n.to_string(), label_c);
                        let t = if info.broken.is_some() { "CAN'T LOAD".to_string() } else { fmt_play(info.play_secs) };
                        o.text_right(tx + THUMB_W as i32, ty + THUMB_H as i32 + 3, &t, label_c);
                    }
                    None => {
                        o.thumb_empty(tx, ty, selected);
                        let label_c = if selected { RED_555 } else { INK_555 };
                        o.text(tx, ty + THUMB_H as i32 + 3, &n.to_string(), label_c);
                    }
                }
            }
            // page marker
            let pm = if self.page == 0 { "1-4 ▸" } else { "◂ 5-8" };
            o.text_right(sx + 132 - 14, sy, pm, DIM_555);
        }

        // bar
        let (bx, by) = o.window(8, 128, 224, 24);
        let ty = by + 1;
        if let Some((msg, t)) = &self.message {
            if now.duration_since(*t) < MESSAGE_TIME {
                o.text(bx, ty, msg, RED_555);
                return;
            }
        }
        match self.mode {
            Mode::Main => {
                o.text(bx, ty, "PAUSED", INK_555);
                o.text_right(bx + 224 - 14, ty, "A SELECT  B RESUME", INK_555);
            }
            Mode::Save | Mode::Load => {
                let n = self.slot();
                let summary = match &slots[n as usize - 1] {
                    Some(i) if i.broken.is_some() => format!("SLOT {n} · CAN'T LOAD"),
                    Some(i) => format!("SLOT {n} · {}", fmt_play(i.play_secs)),
                    None => format!("SLOT {n} · EMPTY"),
                };
                o.text(bx, ty, &summary, INK_555);
                let verb = match (self.mode, &slots[n as usize - 1]) {
                    (Mode::Save, Some(_)) => "OVERWRITE",
                    (Mode::Save, None) => "SAVE",
                    (Mode::Load, Some(i)) if i.broken.is_none() => "LOAD",
                    _ => "",
                };
                let right = if verb.is_empty() { "B BACK".to_string() } else { format!("A {verb}  B BACK") };
                let w = font5x5::width(&right);
                let x = bx + 224 - 14 - w;
                if !verb.is_empty() {
                    o.text(x, ty, &format!("A {verb}"), RED_555);
                }
                o.text_right(bx + 224 - 14, ty, "B BACK", INK_555);
            }
        }
    }
}

/// The pause menu's view of a game's slots.
pub fn slot_infos(set: &crate::slots::SlotSet) -> [Option<SlotInfo>; 8] {
    let mut out: [Option<SlotInfo>; 8] = Default::default();
    for (i, s) in set.slots.iter().enumerate().take(8) {
        out[i] = s.as_ref().map(|m| SlotInfo {
            thumb: Box::new(crate::overlay::thumb_from_frame(&m.frame_full[..])),
            play_secs: m.play_secs,
            frame: m.frame,
            broken: m.broken.clone(),
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input(acts: &[Action]) -> UiInput {
        UiInput { actions: acts.to_vec(), ..Default::default() }
    }

    fn slots() -> [Option<SlotInfo>; 8] {
        let mut s: [Option<SlotInfo>; 8] = Default::default();
        let info = |secs| SlotInfo { thumb: Box::new([0u16; THUMB_W * THUMB_H]), play_secs: secs, frame: 0, broken: None };
        s[0] = Some(info(30));
        s[1] = Some(info(200));
        s[4] = Some(SlotInfo { broken: Some("version".into()), ..info(5) });
        s
    }

    #[test]
    fn main_menu_navigation_wraps_and_selects() {
        let now = Instant::now();
        let s = slots();
        let mut m = PauseMenu::new(1);
        assert_eq!(m.handle(&input(&[Action::Up]), &s, now), Outcome::None);
        assert_eq!(m.sel, 5, "wraps to QUIT");
        assert_eq!(m.handle(&input(&[Action::Confirm]), &s, now), Outcome::Quit);
        m.sel = 0;
        assert_eq!(m.handle(&input(&[Action::Confirm]), &s, now), Outcome::Resume);
        assert_eq!(m.handle(&input(&[Action::Back]), &s, now), Outcome::Resume);
        m.sel = 3;
        assert_eq!(m.handle(&input(&[Action::Confirm]), &s, now), Outcome::Settings);
        m.sel = 4;
        assert_eq!(m.handle(&input(&[Action::Confirm]), &s, now), Outcome::Bench);
    }

    #[test]
    fn sun_row_changes_level_in_place() {
        let now = Instant::now();
        let s = slots();
        let plain = PauseMenu::new(1);
        assert!(!plain.items.contains(&Item::Sun) && plain.sun.is_none());
        let mut m = PauseMenu::new(1).with_sun(Some(9));
        assert_eq!(m.items[3], Item::Sun);
        assert_eq!(m.items.len(), 7);
        m.sel = 3;
        assert_eq!(m.handle(&input(&[Action::Right]), &s, now), Outcome::Sun(10));
        assert_eq!(m.handle(&input(&[Action::Right]), &s, now), Outcome::None, "clamped at the top");
        assert_eq!(m.handle(&input(&[Action::Left]), &s, now), Outcome::Sun(9));
        assert_eq!(m.handle(&input(&[Action::Confirm]), &s, now), Outcome::None);
        m.sel = 4;
        assert_eq!(m.handle(&input(&[Action::Left]), &s, now), Outcome::None, "LEFT elsewhere does nothing");
        assert_eq!(m.handle(&input(&[Action::Confirm]), &s, now), Outcome::Settings, "rows below SUN shift down");
        m.sel = 0;
        m.handle(&input(&[Action::Up]), &s, now);
        assert_eq!(m.items[m.sel], Item::Quit);
        let mut z = PauseMenu::new(1).with_sun(Some(0));
        z.sel = 3;
        assert_eq!(z.handle(&input(&[Action::Left]), &s, now), Outcome::None, "clamped at 0");
    }

    #[test]
    fn save_submenu_pages_and_confirms() {
        let now = Instant::now();
        let s = slots();
        let mut m = PauseMenu::new(2);
        assert_eq!((m.page, m.slot_sel, m.slot()), (0, 1, 2));
        m.sel = 1;
        m.handle(&input(&[Action::Confirm]), &s, now);
        assert_eq!(m.mode, Mode::Save);
        m.handle(&input(&[Action::Down]), &s, now);
        assert_eq!(m.slot(), 4);
        m.handle(&input(&[Action::Left]), &s, now);
        assert_eq!(m.slot(), 3);
        m.handle(&input(&[Action::NextPage]), &s, now);
        assert_eq!(m.slot(), 7);
        m.handle(&input(&[Action::PrevPage]), &s, now);
        assert_eq!(m.slot(), 3);
        assert_eq!(m.handle(&input(&[Action::Confirm]), &s, now), Outcome::SaveSlot(3));
        assert_eq!(m.handle(&input(&[Action::Back]), &s, now), Outcome::None);
        assert_eq!(m.mode, Mode::Main);
    }

    #[test]
    fn load_refuses_empty_and_broken_slots() {
        let now = Instant::now();
        let s = slots();
        let mut m = PauseMenu::new(1);
        m.sel = 2;
        m.handle(&input(&[Action::Confirm]), &s, now);
        assert_eq!(m.mode, Mode::Load);
        assert_eq!(m.handle(&input(&[Action::Confirm]), &s, now), Outcome::LoadSlot(1));
        m.handle(&input(&[Action::Down]), &s, now); // slot 3: empty
        assert_eq!(m.handle(&input(&[Action::Confirm]), &s, now), Outcome::None);
        m.handle(&input(&[Action::NextPage]), &s, now); // slot 7
        m.handle(&input(&[Action::Up]), &s, now); // slot 5: broken
        assert_eq!(m.slot(), 5);
        assert_eq!(m.handle(&input(&[Action::Confirm]), &s, now), Outcome::None);
    }

    #[test]
    fn message_expires_after_a_second_and_a_half() {
        let t0 = Instant::now();
        let s = slots();
        let mut m = PauseMenu::new(1);
        m.show_message("SAVED TO SLOT 2", t0);
        m.handle(&input(&[]), &s, t0 + Duration::from_millis(1000));
        assert!(m.message.is_some());
        m.handle(&input(&[]), &s, t0 + Duration::from_millis(1600));
        assert!(m.message.is_none());
    }

    #[test]
    fn draws_every_mode_without_panicking() {
        let now = Instant::now();
        let s = slots();
        let mut fb: Box<gba_core::FrameBuffer> = vec![0x7FFFu16; 240 * 160].into_boxed_slice().try_into().unwrap();
        for mode in [Mode::Main, Mode::Save, Mode::Load] {
            for page in 0..2 {
                let m = PauseMenu { sel: 1, mode, page, slot_sel: 0, message: None, ..PauseMenu::new(1).with_sun(Some(4)) };
                let mut o = Overlay::new(&mut fb);
                o.dim(DIM_EVY);
                m.draw(&mut o, &s, now);
            }
        }
        let mut m = PauseMenu::new(1);
        m.show_message("SAVED TO SLOT 1", now);
        let mut o = Overlay::new(&mut fb);
        m.draw(&mut o, &s, now);
        assert!(fb.iter().any(|p| *p == RED_555));
    }
}
