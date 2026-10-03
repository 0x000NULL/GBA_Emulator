//! Keyboard (minifb) and gamepad (gilrs) -> GBA key mask; the gamepads themselves are
//! `basket_app::pads`.
//!
//! [`GBA_BUTTONS`] describes the ten buttons to the generic input code. The live layouts are
//! `ui::input::KeyMap` and `ui::input::PadMap` (rebindable in Settings); the tables here are their
//! defaults.

use crate::ui::input::{ButtonSet, KeyMap, MenuRoles, PadMap};
#[cfg(test)]
use basket_app::pads::stick_to_dpad;
use gba_core::Keys;
use gilrs::Button;
use minifb::Key;
use std::time::{Duration, Instant};

/// The GBA's buttons, ordered as `Keys` bits, and the ones the menus use.
pub const GBA_BUTTONS: ButtonSet = ButtonSet {
    names: &["A", "B", "SELECT", "START", "RIGHT", "LEFT", "UP", "DOWN", "R", "L"],
    bits: &[Keys::A, Keys::B, Keys::SELECT, Keys::START, Keys::RIGHT, Keys::LEFT, Keys::UP, Keys::DOWN, Keys::R, Keys::L],
    roles: MenuRoles {
        up: Keys::UP,
        down: Keys::DOWN,
        left: Keys::LEFT,
        right: Keys::RIGHT,
        confirm: Keys::A,
        back: Keys::B,
        prev_page: Keys::L,
        next_page: Keys::R,
        start: Keys::START,
    },
};

pub fn default_keymap() -> KeyMap {
    KeyMap::new(GBA_BUTTONS, KEYBOARD_MAP)
}

pub fn default_padmap() -> PadMap {
    PadMap::new(GBA_BUTTONS, GAMEPAD_MAP)
}

/// Default keyboard layout.
pub const KEYBOARD_MAP: &[(Key, u16)] = &[
    (Key::Z, Keys::A),
    (Key::X, Keys::B),
    (Key::A, Keys::L),
    (Key::S, Keys::R),
    (Key::Enter, Keys::START),
    (Key::RightShift, Keys::SELECT),
    (Key::Up, Keys::UP),
    (Key::Down, Keys::DOWN),
    (Key::Left, Keys::LEFT),
    (Key::Right, Keys::RIGHT),
];

/// Default gamepad button layout (gilrs `LeftTrigger`/`RightTrigger` are the bumpers).
pub const GAMEPAD_MAP: &[(Button, u16)] = &[
    (Button::South, Keys::A),
    (Button::East, Keys::B),
    (Button::LeftTrigger, Keys::L),
    (Button::RightTrigger, Keys::R),
    (Button::Start, Keys::START),
    (Button::Select, Keys::SELECT),
    (Button::DPadUp, Keys::UP),
    (Button::DPadDown, Keys::DOWN),
    (Button::DPadLeft, Keys::LEFT),
    (Button::DPadRight, Keys::RIGHT),
];

/// Build the key mask from an "is this key down" predicate (the default layout; the live
/// layout is `ui::input::KeyMap`).
#[cfg_attr(not(test), allow(dead_code))]
pub fn keyboard_keys(down: impl Fn(Key) -> bool) -> u16 {
    KEYBOARD_MAP.iter().filter(|(k, _)| down(*k)).fold(0, |m, (_, bit)| m | bit)
}

/// How long SELECT+START must be held to open the pause menu from a pad.
pub const PAD_PAUSE_HOLD: Duration = Duration::from_secs(1);

/// The controller route to the pause menu (the Fruit Basket couch convention, shared with ps2emu
/// and Crabapple): the Guide button, or SELECT+START held for [`PAD_PAUSE_HOLD`]. The hold fires
/// once; both buttons must be released before it can fire again. The game still sees the buttons.
#[derive(Debug, Default)]
pub struct PadPause {
    held_since: Option<Instant>,
    fired: bool,
}

impl PadPause {
    /// `pad_pressed`: pad buttons that went down this frame; `mask`: the GBA buttons held now.
    /// True when the pause menu should toggle.
    pub fn update(&mut self, pad_pressed: &[Button], mask: u16, now: Instant) -> bool {
        let combo = Keys::SELECT | Keys::START;
        let mut toggle = pad_pressed.contains(&Button::Mode);
        if mask & combo == combo {
            let since = *self.held_since.get_or_insert(now);
            if !self.fired && now.duration_since(since) >= PAD_PAUSE_HOLD {
                self.fired = true;
                toggle = true;
            }
        } else {
            self.held_since = None;
            self.fired = false;
        }
        toggle
    }
}

/// Drop simultaneous opposite directions (possible when keyboard and pad disagree); real hardware
/// cannot report them and some games misbehave.
pub fn sanitize(mask: u16) -> u16 {
    let mut m = mask & Keys::ALL;
    if m & (Keys::LEFT | Keys::RIGHT) == (Keys::LEFT | Keys::RIGHT) {
        m &= !(Keys::LEFT | Keys::RIGHT);
    }
    if m & (Keys::UP | Keys::DOWN) == (Keys::UP | Keys::DOWN) {
        m &= !(Keys::UP | Keys::DOWN);
    }
    m
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keyboard_mapping() {
        let down = |k: Key| matches!(k, Key::Z | Key::Enter | Key::Left);
        assert_eq!(keyboard_keys(down), Keys::A | Keys::START | Keys::LEFT);
        assert_eq!(keyboard_keys(|_| false), 0);
        assert_eq!(keyboard_keys(|_| true), Keys::ALL);
    }

    const R: &MenuRoles = &GBA_BUTTONS.roles;

    #[test]
    fn pad_pause_is_guide_or_a_one_second_select_start_hold() {
        let t0 = Instant::now();
        let at = |ms: u64| t0 + Duration::from_millis(ms);
        let combo = Keys::SELECT | Keys::START;
        let mut p = PadPause::default();
        // Guide toggles on the frame it goes down
        assert!(p.update(&[Button::Mode], 0, at(0)));
        assert!(!p.update(&[], 0, at(16)));
        // SELECT+START: nothing before the second, once at it, not again while held
        assert!(!p.update(&[Button::Select, Button::Start], combo, at(100)));
        assert!(!p.update(&[], combo | Keys::A, at(1099)));
        assert!(p.update(&[], combo, at(1100)));
        assert!(!p.update(&[], combo, at(3000)));
        // releasing either button re-arms it; a short hold does nothing
        assert!(!p.update(&[], Keys::START, at(3016)));
        assert!(!p.update(&[], combo, at(3032)));
        assert!(!p.update(&[], 0, at(3500)));
        assert!(!p.update(&[], combo, at(3600)));
        assert!(p.update(&[], combo, at(4600)));
        // either button alone never fires
        let mut p = PadPause::default();
        assert!(!p.update(&[], Keys::START, at(0)));
        assert!(!p.update(&[], Keys::START, at(5000)));
    }

    #[test]
    fn default_maps_match_the_tables() {
        let m = default_keymap();
        assert_eq!(m.key_for(Keys::A), Key::Z);
        assert_eq!(m.key_for(Keys::B), Key::X);
        assert_eq!(m.key_for(Keys::L), Key::A);
        assert_eq!(m.key_for(Keys::R), Key::S);
        assert_eq!(m.key_for(Keys::START), Key::Enter);
        assert_eq!(m.key_for(Keys::SELECT), Key::RightShift);
        assert_eq!(m.mask(|k| matches!(k, Key::Z | Key::Left)), Keys::A | Keys::LEFT);
        let p = default_padmap();
        assert_eq!(p.button_for(Keys::A), Button::South);
        assert_eq!(p.button_for(Keys::B), Button::East);
        assert_eq!(p.button_for(Keys::L), Button::LeftTrigger);
        assert_eq!(p.button_for(Keys::R), Button::RightTrigger);
        assert_eq!(p.button_for(Keys::START), Button::Start);
        assert_eq!(p.button_for(Keys::SELECT), Button::Select);
        assert_eq!(p.button_for(Keys::UP), Button::DPadUp);
        assert_eq!(p.button_for(Keys::RIGHT), Button::DPadRight);
        assert_eq!(p.mask(|b| matches!(b, Button::South | Button::DPadLeft)), Keys::A | Keys::LEFT);
        // GBA bits are 1 << position, as the generic maps index them
        assert!(GBA_BUTTONS.bits.iter().enumerate().all(|(i, b)| *b == 1 << i));
        // the menus: A confirms, B backs out, L/R page, Start starts
        let acts = crate::ui::input::actions(&m, &[Key::Z, Key::X, Key::A, Key::S, Key::Enter], &[Key::Up], 0, 0);
        use crate::ui::input::Action::*;
        assert_eq!(acts, vec![Up, Confirm, Back, PrevPage, NextPage, Start]);
    }

    #[test]
    fn stick_mapping() {
        assert_eq!(stick_to_dpad(0.0, 0.0, 0.5, R), 0);
        assert_eq!(stick_to_dpad(0.3, -0.3, 0.5, R), 0);
        assert_eq!(stick_to_dpad(1.0, 0.0, 0.5, R), Keys::RIGHT);
        assert_eq!(stick_to_dpad(-0.6, 0.9, 0.5, R), Keys::LEFT | Keys::UP);
        assert_eq!(stick_to_dpad(0.0, -1.0, 0.5, R), Keys::DOWN);
    }

    #[test]
    fn sanitize_opposites() {
        assert_eq!(sanitize(Keys::LEFT | Keys::RIGHT | Keys::A), Keys::A);
        assert_eq!(sanitize(Keys::UP | Keys::DOWN | Keys::LEFT), Keys::LEFT);
        assert_eq!(sanitize(0xFFFF), Keys::ALL & !(Keys::LEFT | Keys::RIGHT | Keys::UP | Keys::DOWN));
    }
}
