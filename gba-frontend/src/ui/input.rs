//! Per-frame input snapshot for the screens: edge-triggered keys, typed characters, mouse,
//! and the navigation actions derived from the GBA key bindings and the gamepad.
//!
//! Menus navigate with the keys bound to the GBA buttons (confirm = A, back = B, pages = L/R,
//! arrows = D-pad), so a rebinding in Settings moves the menus too.

// Toolkit surface: the screens use a subset of these helpers.
#![allow(dead_code)]

use gba_core::Keys;
use gilrs::Button;
use minifb::Key;
use std::time::{Duration, Instant};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Action {
    Up,
    Down,
    Left,
    Right,
    Confirm,
    Back,
    PrevPage,
    NextPage,
    Start,
}

/// Keyboard binding for the ten GBA buttons.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeyMap {
    /// Ordered as `Keys` bits: A, B, SELECT, START, RIGHT, LEFT, UP, DOWN, R, L.
    pub keys: [Key; 10],
}

pub const BUTTON_BITS: [u16; 10] = [Keys::A, Keys::B, Keys::SELECT, Keys::START, Keys::RIGHT, Keys::LEFT, Keys::UP, Keys::DOWN, Keys::R, Keys::L];
pub const BUTTON_NAMES: [&str; 10] = ["A", "B", "SELECT", "START", "RIGHT", "LEFT", "UP", "DOWN", "R", "L"];

impl Default for KeyMap {
    fn default() -> Self {
        let mut keys = [Key::Unknown; 10];
        for (k, bit) in crate::input::KEYBOARD_MAP {
            keys[bit.trailing_zeros() as usize] = *k;
        }
        KeyMap { keys }
    }
}

impl KeyMap {
    pub fn key_for(&self, bit: u16) -> Key {
        self.keys[bit.trailing_zeros() as usize]
    }

    /// Key mask from an "is down" predicate.
    pub fn mask(&self, down: impl Fn(Key) -> bool) -> u16 {
        self.keys.iter().zip(BUTTON_BITS).filter(|(k, _)| down(**k)).fold(0, |m, (_, bit)| m | bit)
    }

    /// Bind `bit` to `key`; a key already used by another button is swapped. Returns the other
    /// button's bit when a swap happened.
    pub fn rebind(&mut self, bit: u16, key: Key) -> Option<u16> {
        let i = bit.trailing_zeros() as usize;
        let old = self.keys[i];
        let other = self.keys.iter().position(|k| *k == key && *k != old).filter(|&j| j != i);
        if let Some(j) = other {
            self.keys[j] = old;
        }
        self.keys[i] = key;
        other.map(|j| BUTTON_BITS[j])
    }
}

/// Gamepad binding for the ten GBA buttons: one map shared by every connected pad (their
/// states are OR-ed). The left stick always acts as the D-pad in addition.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PadMap {
    /// Ordered as `Keys` bits, like [`KeyMap::keys`].
    pub buttons: [Button; 10],
}

impl Default for PadMap {
    fn default() -> Self {
        let mut buttons = [Button::Unknown; 10];
        for (b, bit) in crate::input::GAMEPAD_MAP {
            buttons[bit.trailing_zeros() as usize] = *b;
        }
        PadMap { buttons }
    }
}

impl PadMap {
    pub fn button_for(&self, bit: u16) -> Button {
        self.buttons[bit.trailing_zeros() as usize]
    }

    /// Key mask from an "is pressed" predicate.
    pub fn mask(&self, pressed: impl Fn(Button) -> bool) -> u16 {
        self.buttons.iter().zip(BUTTON_BITS).filter(|(b, _)| pressed(**b)).fold(0, |m, (_, bit)| m | bit)
    }

    /// Bind `bit` to `button`; a button already used by another GBA button is swapped. Returns
    /// the other button's bit when a swap happened.
    pub fn rebind(&mut self, bit: u16, button: Button) -> Option<u16> {
        let i = bit.trailing_zeros() as usize;
        let old = self.buttons[i];
        let other = self.buttons.iter().position(|b| *b == button && *b != old).filter(|&j| j != i);
        if let Some(j) = other {
            self.buttons[j] = old;
        }
        self.buttons[i] = button;
        other.map(|j| BUTTON_BITS[j])
    }
}

/// Every gilrs button a binding may land on (all but `Unknown`).
pub const ALL_PAD_BUTTONS: [Button; 19] = [
    Button::South,
    Button::East,
    Button::West,
    Button::North,
    Button::C,
    Button::Z,
    Button::LeftTrigger,
    Button::LeftTrigger2,
    Button::RightTrigger,
    Button::RightTrigger2,
    Button::Select,
    Button::Start,
    Button::Mode,
    Button::LeftThumb,
    Button::RightThumb,
    Button::DPadUp,
    Button::DPadDown,
    Button::DPadLeft,
    Button::DPadRight,
];

/// Human name for a pad button, as the keycaps print it and the settings file stores it
/// (gilrs's `LeftTrigger` is the bumper, `LeftTrigger2` the analog trigger).
pub fn pad_button_name(b: Button) -> &'static str {
    match b {
        Button::South => "South",
        Button::East => "East",
        Button::West => "West",
        Button::North => "North",
        Button::C => "C",
        Button::Z => "Z",
        Button::LeftTrigger => "LB",
        Button::LeftTrigger2 => "LT",
        Button::RightTrigger => "RB",
        Button::RightTrigger2 => "RT",
        Button::Select => "Select",
        Button::Start => "Start",
        Button::Mode => "Mode",
        Button::LeftThumb => "L Stick",
        Button::RightThumb => "R Stick",
        Button::DPadUp => "D↑",
        Button::DPadDown => "D↓",
        Button::DPadLeft => "D←",
        Button::DPadRight => "D→",
        Button::Unknown => "?",
    }
}

pub fn pad_button_from_name(name: &str) -> Option<Button> {
    ALL_PAD_BUTTONS.iter().copied().find(|b| pad_button_name(*b) == name)
}

/// Everything a screen may react to this frame.
#[derive(Clone, Debug, Default)]
pub struct UiInput {
    /// Keys that went down this frame (no auto-repeat).
    pub pressed: Vec<Key>,
    /// Keys that went down this frame or auto-repeated (navigation).
    pub repeated: Vec<Key>,
    pub down: Vec<Key>,
    pub chars: Vec<char>,
    pub mouse: (f32, f32),
    pub mouse_down: bool,
    pub clicked: bool,
    pub wheel: f32,
    pub actions: Vec<Action>,
    /// Pad buttons that went down this frame (for rebinding; the mask drives everything else).
    pub pad_buttons: Vec<Button>,
    /// Combined keyboard + gamepad GBA mask (for the running game).
    pub game_mask: u16,
}

impl UiInput {
    pub fn pressed(&self, k: Key) -> bool {
        self.pressed.contains(&k)
    }
    pub fn repeated(&self, k: Key) -> bool {
        self.repeated.contains(&k)
    }
    pub fn is_down(&self, k: Key) -> bool {
        self.down.contains(&k)
    }
    pub fn action(&self, a: Action) -> bool {
        self.actions.contains(&a)
    }
    pub fn hover(&self, x: f32, y: f32, w: f32, h: f32) -> bool {
        self.mouse.0 >= x && self.mouse.0 < x + w && self.mouse.1 >= y && self.mouse.1 < y + h
    }
    pub fn click_in(&self, x: f32, y: f32, w: f32, h: f32) -> bool {
        self.clicked && self.hover(x, y, w, h)
    }
}

/// Edge detection and auto-repeat for the gamepad mask, mirroring the keyboard's repeat.
pub struct PadRepeat {
    prev: u16,
    held_since: [Option<Instant>; 10],
    last_fire: [Option<Instant>; 10],
}

pub const PAD_REPEAT_DELAY: Duration = Duration::from_millis(350);
pub const PAD_REPEAT_RATE: Duration = Duration::from_millis(60);

impl Default for PadRepeat {
    fn default() -> Self {
        Self::new()
    }
}

impl PadRepeat {
    pub fn new() -> Self {
        PadRepeat { prev: 0, held_since: [None; 10], last_fire: [None; 10] }
    }

    /// Returns `(pressed_mask, repeated_mask)` for this frame.
    pub fn update(&mut self, mask: u16, now: Instant) -> (u16, u16) {
        let pressed = mask & !self.prev;
        let mut repeated = pressed;
        for (i, bit) in BUTTON_BITS.iter().enumerate() {
            if mask & bit == 0 {
                self.held_since[i] = None;
                self.last_fire[i] = None;
                continue;
            }
            if pressed & bit != 0 {
                self.held_since[i] = Some(now);
                self.last_fire[i] = Some(now);
                continue;
            }
            if let (Some(since), Some(last)) = (self.held_since[i], self.last_fire[i]) {
                if now.duration_since(since) >= PAD_REPEAT_DELAY && now.duration_since(last) >= PAD_REPEAT_RATE {
                    repeated |= bit;
                    self.last_fire[i] = Some(now);
                }
            }
        }
        self.prev = mask;
        (pressed, repeated)
    }
}

/// Derive navigation actions from this frame's key events and the pad edge masks.
pub fn actions(map: &KeyMap, pressed: &[Key], repeated: &[Key], pad_pressed: u16, pad_repeated: u16) -> Vec<Action> {
    let mut out = Vec::new();
    let key_rep = |bit: u16| repeated.contains(&map.key_for(bit)) || pad_repeated & bit != 0;
    let key_edge = |bit: u16| pressed.contains(&map.key_for(bit)) || pad_pressed & bit != 0;
    if key_rep(Keys::UP) {
        out.push(Action::Up);
    }
    if key_rep(Keys::DOWN) {
        out.push(Action::Down);
    }
    if key_rep(Keys::LEFT) {
        out.push(Action::Left);
    }
    if key_rep(Keys::RIGHT) {
        out.push(Action::Right);
    }
    if key_edge(Keys::A) {
        out.push(Action::Confirm);
    }
    if key_edge(Keys::B) {
        out.push(Action::Back);
    }
    if key_edge(Keys::L) {
        out.push(Action::PrevPage);
    }
    if key_edge(Keys::R) {
        out.push(Action::NextPage);
    }
    if key_edge(Keys::START) {
        out.push(Action::Start);
    }
    out
}

/// Human name for a key, as the keycaps print it.
pub fn key_name(k: Key) -> String {
    match k {
        Key::Up => "↑".into(),
        Key::Down => "↓".into(),
        Key::Left => "←".into(),
        Key::Right => "→".into(),
        Key::RightShift => "R Shift".into(),
        Key::LeftShift => "L Shift".into(),
        Key::RightCtrl => "R Ctrl".into(),
        Key::LeftCtrl => "L Ctrl".into(),
        Key::RightAlt => "R Alt".into(),
        Key::LeftAlt => "L Alt".into(),
        Key::Space => "Space".into(),
        Key::Backspace => "Bksp".into(),
        Key::Delete => "Del".into(),
        Key::Escape => "Esc".into(),
        Key::Enter => "Enter".into(),
        Key::Tab => "Tab".into(),
        Key::Key0 => "0".into(),
        Key::Key1 => "1".into(),
        Key::Key2 => "2".into(),
        Key::Key3 => "3".into(),
        Key::Key4 => "4".into(),
        Key::Key5 => "5".into(),
        Key::Key6 => "6".into(),
        Key::Key7 => "7".into(),
        Key::Key8 => "8".into(),
        Key::Key9 => "9".into(),
        Key::Unknown => "?".into(),
        other => format!("{other:?}"),
    }
}

/// Keys a rebinding may land on (not the mouse buttons / unknown).
pub fn bindable(k: Key) -> bool {
    !matches!(k, Key::Unknown)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_map_matches_keyboard_map() {
        let m = KeyMap::default();
        assert_eq!(m.key_for(Keys::A), Key::Z);
        assert_eq!(m.key_for(Keys::B), Key::X);
        assert_eq!(m.key_for(Keys::L), Key::A);
        assert_eq!(m.key_for(Keys::R), Key::S);
        assert_eq!(m.key_for(Keys::START), Key::Enter);
        assert_eq!(m.key_for(Keys::SELECT), Key::RightShift);
        assert_eq!(m.mask(|k| matches!(k, Key::Z | Key::Left)), Keys::A | Keys::LEFT);
    }

    #[test]
    fn rebind_swaps_duplicates() {
        let mut m = KeyMap::default();
        assert_eq!(m.rebind(Keys::A, Key::X), Some(Keys::B), "X was B: B takes A's old key");
        assert_eq!(m.key_for(Keys::A), Key::X);
        assert_eq!(m.key_for(Keys::B), Key::Z);
        assert_eq!(m.rebind(Keys::L, Key::Q), None);
        assert_eq!(m.key_for(Keys::L), Key::Q);
        assert_eq!(m.rebind(Keys::L, Key::Q), None, "same key again is a no-op");
    }

    #[test]
    fn actions_from_keys_and_pad() {
        let m = KeyMap::default();
        let a = actions(&m, &[Key::Z], &[Key::Down, Key::Z], 0, 0);
        assert_eq!(a, vec![Action::Down, Action::Confirm]);
        let a = actions(&m, &[], &[], Keys::B | Keys::R, Keys::B | Keys::R | Keys::UP);
        assert_eq!(a, vec![Action::Up, Action::Back, Action::NextPage]);
    }

    #[test]
    fn pad_repeat_timing() {
        let mut p = PadRepeat::new();
        let t0 = Instant::now();
        let (pr, rp) = p.update(Keys::DOWN, t0);
        assert_eq!((pr, rp), (Keys::DOWN, Keys::DOWN));
        let (pr, rp) = p.update(Keys::DOWN, t0 + Duration::from_millis(100));
        assert_eq!((pr, rp), (0, 0));
        let (pr, rp) = p.update(Keys::DOWN, t0 + Duration::from_millis(400));
        assert_eq!((pr, rp), (0, Keys::DOWN));
        let (_, rp) = p.update(Keys::DOWN, t0 + Duration::from_millis(420));
        assert_eq!(rp, 0, "rate limited");
        let (_, rp) = p.update(Keys::DOWN, t0 + Duration::from_millis(470));
        assert_eq!(rp, Keys::DOWN);
        let (pr, rp) = p.update(0, t0 + Duration::from_millis(500));
        assert_eq!((pr, rp), (0, 0));
    }

    #[test]
    fn key_names() {
        assert_eq!(key_name(Key::RightShift), "R Shift");
        assert_eq!(key_name(Key::F12), "F12");
        assert_eq!(key_name(Key::Key3), "3");
        assert_eq!(key_name(Key::Up), "↑");
    }

    #[test]
    fn pad_map_default_matches_gamepad_map() {
        let m = PadMap::default();
        assert_eq!(m.button_for(Keys::A), Button::South);
        assert_eq!(m.button_for(Keys::B), Button::East);
        assert_eq!(m.button_for(Keys::L), Button::LeftTrigger);
        assert_eq!(m.button_for(Keys::R), Button::RightTrigger);
        assert_eq!(m.button_for(Keys::START), Button::Start);
        assert_eq!(m.button_for(Keys::SELECT), Button::Select);
        assert_eq!(m.button_for(Keys::UP), Button::DPadUp);
        assert_eq!(m.button_for(Keys::RIGHT), Button::DPadRight);
        assert_eq!(m.mask(|b| matches!(b, Button::South | Button::DPadLeft)), Keys::A | Keys::LEFT);
        assert_eq!(m.mask(|_| false), 0);
    }

    #[test]
    fn pad_rebind_swaps_duplicates() {
        let mut m = PadMap::default();
        assert_eq!(m.rebind(Keys::A, Button::East), Some(Keys::B), "East was B: B takes A's old button");
        assert_eq!(m.button_for(Keys::A), Button::East);
        assert_eq!(m.button_for(Keys::B), Button::South);
        assert_eq!(m.rebind(Keys::L, Button::LeftTrigger2), None);
        assert_eq!(m.button_for(Keys::L), Button::LeftTrigger2);
        assert_eq!(m.rebind(Keys::L, Button::LeftTrigger2), None, "same button again is a no-op");
    }

    #[test]
    fn pad_button_names_invert() {
        for b in ALL_PAD_BUTTONS {
            assert_eq!(pad_button_from_name(pad_button_name(b)), Some(b), "{b:?}");
        }
        let mut names: Vec<&str> = ALL_PAD_BUTTONS.iter().map(|b| pad_button_name(*b)).collect();
        names.sort();
        names.dedup();
        assert_eq!(names.len(), ALL_PAD_BUTTONS.len(), "names are unique");
        assert_eq!(pad_button_name(Button::Unknown), "?");
        assert_eq!(pad_button_from_name("?"), None);
        assert_eq!(pad_button_from_name("Nonsense"), None);
    }
}
