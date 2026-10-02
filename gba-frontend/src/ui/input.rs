//! Per-frame input snapshot for the screens: edge-triggered keys, typed characters, mouse,
//! and the navigation actions derived from the console's key bindings and the gamepad.
//!
//! The app describes its console's buttons with a [`ButtonSet`]: names, bits in the button mask,
//! and which buttons drive the menus ([`MenuRoles`]; on the GBA confirm = A, back = B, pages = L/R,
//! arrows = D-pad), so a rebinding in Settings moves the menus too.

// Toolkit surface: the screens use a subset of these helpers.
#![allow(dead_code)]

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

/// Which buttons drive the menus: each a bit of the console's button mask, 0 = none.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MenuRoles {
    pub up: u16,
    pub down: u16,
    pub left: u16,
    pub right: u16,
    pub confirm: u16,
    pub back: u16,
    pub prev_page: u16,
    pub next_page: u16,
    pub start: u16,
}

/// A console's buttons, in the order the settings file and the keycaps list them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ButtonSet {
    /// Settings-file names, e.g. `"A"`, `"SELECT"`.
    pub names: &'static [&'static str],
    /// Each button's bit in the button mask, parallel to `names`.
    pub bits: &'static [u16],
    pub roles: MenuRoles,
}

impl ButtonSet {
    /// Position of `bit` in the set. Panics on a bit the set does not have.
    pub fn index(&self, bit: u16) -> usize {
        self.bits.iter().position(|b| *b == bit).unwrap_or_else(|| panic!("button bit {bit:#x} is not in the set"))
    }
}

/// Keyboard binding for one port's buttons.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeyMap {
    set: ButtonSet,
    /// Parallel to `set.bits`.
    pub keys: Vec<Key>,
}

impl KeyMap {
    /// A map for `set` with `defaults` (key, bit); buttons without one get `Key::Unknown`.
    pub fn new(set: ButtonSet, defaults: &[(Key, u16)]) -> Self {
        let mut keys = vec![Key::Unknown; set.bits.len()];
        for (k, bit) in defaults {
            keys[set.index(*bit)] = *k;
        }
        KeyMap { set, keys }
    }

    pub fn set(&self) -> &ButtonSet {
        &self.set
    }

    pub fn key_for(&self, bit: u16) -> Key {
        self.keys[self.set.index(bit)]
    }

    /// Key mask from an "is down" predicate.
    pub fn mask(&self, down: impl Fn(Key) -> bool) -> u16 {
        self.keys.iter().zip(self.set.bits).filter(|(k, _)| down(**k)).fold(0, |m, (_, bit)| m | bit)
    }

    /// Bind `bit` to `key`; a key already used by another button is swapped. Returns the other
    /// button's bit when a swap happened.
    pub fn rebind(&mut self, bit: u16, key: Key) -> Option<u16> {
        let i = self.set.index(bit);
        let old = self.keys[i];
        let other = self.keys.iter().position(|k| *k == key && *k != old).filter(|&j| j != i);
        if let Some(j) = other {
            self.keys[j] = old;
        }
        self.keys[i] = key;
        other.map(|j| self.set.bits[j])
    }
}

/// Gamepad binding for one port's buttons. Strawberry shares one map between every connected pad
/// (their states are OR-ed); the left stick always acts as the D-pad in addition.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PadMap {
    set: ButtonSet,
    /// Parallel to `set.bits`, like [`KeyMap::keys`].
    pub buttons: Vec<Button>,
}

impl PadMap {
    /// A map for `set` with `defaults` (button, bit); the rest get `Button::Unknown`.
    pub fn new(set: ButtonSet, defaults: &[(Button, u16)]) -> Self {
        let mut buttons = vec![Button::Unknown; set.bits.len()];
        for (b, bit) in defaults {
            buttons[set.index(*bit)] = *b;
        }
        PadMap { set, buttons }
    }

    pub fn set(&self) -> &ButtonSet {
        &self.set
    }

    pub fn button_for(&self, bit: u16) -> Button {
        self.buttons[self.set.index(bit)]
    }

    /// Key mask from an "is pressed" predicate.
    pub fn mask(&self, pressed: impl Fn(Button) -> bool) -> u16 {
        self.buttons.iter().zip(self.set.bits).filter(|(b, _)| pressed(**b)).fold(0, |m, (_, bit)| m | bit)
    }

    /// Bind `bit` to `button`; a button already used by another console button is swapped.
    /// Returns the other button's bit when a swap happened.
    pub fn rebind(&mut self, bit: u16, button: Button) -> Option<u16> {
        let i = self.set.index(bit);
        let old = self.buttons[i];
        let other = self.buttons.iter().position(|b| *b == button && *b != old).filter(|&j| j != i);
        if let Some(j) = other {
            self.buttons[j] = old;
        }
        self.buttons[i] = button;
        other.map(|j| self.set.bits[j])
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
    /// Combined keyboard + gamepad button mask (for the running game).
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
    bits: &'static [u16],
    prev: u16,
    held_since: Vec<Option<Instant>>,
    last_fire: Vec<Option<Instant>>,
}

pub const PAD_REPEAT_DELAY: Duration = Duration::from_millis(350);
pub const PAD_REPEAT_RATE: Duration = Duration::from_millis(60);

impl PadRepeat {
    pub fn new(set: &ButtonSet) -> Self {
        let n = set.bits.len();
        PadRepeat { bits: set.bits, prev: 0, held_since: vec![None; n], last_fire: vec![None; n] }
    }

    /// Returns `(pressed_mask, repeated_mask)` for this frame.
    pub fn update(&mut self, mask: u16, now: Instant) -> (u16, u16) {
        let pressed = mask & !self.prev;
        let mut repeated = pressed;
        for (i, bit) in self.bits.iter().enumerate() {
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

/// Derive navigation actions from this frame's key events and the pad edge masks, through the
/// map's [`MenuRoles`]. Arrows auto-repeat; the rest fire once per press.
pub fn actions(map: &KeyMap, pressed: &[Key], repeated: &[Key], pad_pressed: u16, pad_repeated: u16) -> Vec<Action> {
    let r = map.set().roles;
    let key_rep = |bit: u16| bit != 0 && (repeated.contains(&map.key_for(bit)) || pad_repeated & bit != 0);
    let key_edge = |bit: u16| bit != 0 && (pressed.contains(&map.key_for(bit)) || pad_pressed & bit != 0);
    let table = [
        (r.up, true, Action::Up),
        (r.down, true, Action::Down),
        (r.left, true, Action::Left),
        (r.right, true, Action::Right),
        (r.confirm, false, Action::Confirm),
        (r.back, false, Action::Back),
        (r.prev_page, false, Action::PrevPage),
        (r.next_page, false, Action::NextPage),
        (r.start, false, Action::Start),
    ];
    table.iter().filter(|(bit, rep, _)| if *rep { key_rep(*bit) } else { key_edge(*bit) }).map(|(_, _, a)| *a).collect()
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

/// Every `minifb::Key` variant except `Count`.
pub const ALL_KEYS: [Key; 107] = [
    Key::Key0,
    Key::Key1,
    Key::Key2,
    Key::Key3,
    Key::Key4,
    Key::Key5,
    Key::Key6,
    Key::Key7,
    Key::Key8,
    Key::Key9,
    Key::A,
    Key::B,
    Key::C,
    Key::D,
    Key::E,
    Key::F,
    Key::G,
    Key::H,
    Key::I,
    Key::J,
    Key::K,
    Key::L,
    Key::M,
    Key::N,
    Key::O,
    Key::P,
    Key::Q,
    Key::R,
    Key::S,
    Key::T,
    Key::U,
    Key::V,
    Key::W,
    Key::X,
    Key::Y,
    Key::Z,
    Key::F1,
    Key::F2,
    Key::F3,
    Key::F4,
    Key::F5,
    Key::F6,
    Key::F7,
    Key::F8,
    Key::F9,
    Key::F10,
    Key::F11,
    Key::F12,
    Key::F13,
    Key::F14,
    Key::F15,
    Key::Down,
    Key::Left,
    Key::Right,
    Key::Up,
    Key::Apostrophe,
    Key::Backquote,
    Key::Backslash,
    Key::Comma,
    Key::Equal,
    Key::LeftBracket,
    Key::Minus,
    Key::Period,
    Key::RightBracket,
    Key::Semicolon,
    Key::Slash,
    Key::Backspace,
    Key::Delete,
    Key::End,
    Key::Enter,
    Key::Escape,
    Key::Home,
    Key::Insert,
    Key::Menu,
    Key::PageDown,
    Key::PageUp,
    Key::Pause,
    Key::Space,
    Key::Tab,
    Key::NumLock,
    Key::CapsLock,
    Key::ScrollLock,
    Key::LeftShift,
    Key::RightShift,
    Key::LeftCtrl,
    Key::RightCtrl,
    Key::NumPad0,
    Key::NumPad1,
    Key::NumPad2,
    Key::NumPad3,
    Key::NumPad4,
    Key::NumPad5,
    Key::NumPad6,
    Key::NumPad7,
    Key::NumPad8,
    Key::NumPad9,
    Key::NumPadDot,
    Key::NumPadSlash,
    Key::NumPadAsterisk,
    Key::NumPadMinus,
    Key::NumPadPlus,
    Key::NumPadEnter,
    Key::LeftAlt,
    Key::RightAlt,
    Key::LeftSuper,
    Key::RightSuper,
    Key::Unknown,
];

/// The key whose [`key_name`] is `name` (the inverse of `key_name`).
pub fn key_from_name(name: &str) -> Option<Key> {
    ALL_KEYS.iter().copied().find(|k| key_name(*k) == name)
}

/// Keys a rebinding may land on (not the mouse buttons / unknown).
pub fn bindable(k: Key) -> bool {
    !matches!(k, Key::Unknown)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A ten-button set shaped like the GBA's, with literal bits.
    mod k {
        pub const A: u16 = 1;
        pub const B: u16 = 2;
        pub const SELECT: u16 = 4;
        pub const START: u16 = 8;
        pub const RIGHT: u16 = 16;
        pub const LEFT: u16 = 32;
        pub const UP: u16 = 64;
        pub const DOWN: u16 = 128;
        pub const R: u16 = 256;
        pub const L: u16 = 512;
    }

    const SET: ButtonSet = ButtonSet {
        names: &["A", "B", "SELECT", "START", "RIGHT", "LEFT", "UP", "DOWN", "R", "L"],
        bits: &[k::A, k::B, k::SELECT, k::START, k::RIGHT, k::LEFT, k::UP, k::DOWN, k::R, k::L],
        roles: MenuRoles { up: k::UP, down: k::DOWN, left: k::LEFT, right: k::RIGHT, confirm: k::A, back: k::B, prev_page: k::L, next_page: k::R, start: k::START },
    };

    fn km() -> KeyMap {
        KeyMap::new(SET, &[(Key::Z, k::A), (Key::X, k::B), (Key::A, k::L), (Key::S, k::R), (Key::Enter, k::START), (Key::Up, k::UP), (Key::Down, k::DOWN)])
    }

    fn pm() -> PadMap {
        PadMap::new(SET, &[(Button::South, k::A), (Button::East, k::B), (Button::LeftTrigger, k::L), (Button::DPadLeft, k::LEFT)])
    }

    #[test]
    fn maps_follow_the_set() {
        let m = km();
        assert_eq!((m.key_for(k::A), m.key_for(k::SELECT)), (Key::Z, Key::Unknown), "no default: unknown");
        assert_eq!(m.mask(|k| matches!(k, Key::Z | Key::Up)), k::A | k::UP);
        let p = pm();
        assert_eq!(p.button_for(k::L), Button::LeftTrigger);
        assert_eq!(p.mask(|b| matches!(b, Button::South | Button::DPadLeft)), k::A | k::LEFT);
        assert_eq!(p.mask(|_| false), 0);
        // a two-button set with sparse bits
        const TWO: ButtonSet = ButtonSet {
            names: &["FIRE", "JUMP"],
            bits: &[0x10, 0x80],
            roles: MenuRoles { up: 0, down: 0, left: 0, right: 0, confirm: 0x10, back: 0x80, prev_page: 0, next_page: 0, start: 0 },
        };
        let mut two = KeyMap::new(TWO, &[(Key::Space, 0x10)]);
        assert_eq!(two.mask(|k| k == Key::Space), 0x10);
        assert_eq!(two.rebind(0x80, Key::Space), Some(0x10));
        assert_eq!(actions(&two, &[Key::Unknown], &[], 0x80, 0), vec![Action::Confirm, Action::Back], "Unknown is FIRE's key now");
    }

    #[test]
    fn rebind_swaps_duplicates() {
        let mut m = km();
        assert_eq!(m.rebind(k::A, Key::X), Some(k::B), "X was B: B takes A's old key");
        assert_eq!(m.key_for(k::A), Key::X);
        assert_eq!(m.key_for(k::B), Key::Z);
        assert_eq!(m.rebind(k::L, Key::Q), None);
        assert_eq!(m.key_for(k::L), Key::Q);
        assert_eq!(m.rebind(k::L, Key::Q), None, "same key again is a no-op");
    }

    #[test]
    fn actions_from_keys_and_pad() {
        let m = km();
        let a = actions(&m, &[Key::Z], &[Key::Down, Key::Z], 0, 0);
        assert_eq!(a, vec![Action::Down, Action::Confirm]);
        let a = actions(&m, &[], &[], k::B | k::R, k::B | k::R | k::UP);
        assert_eq!(a, vec![Action::Up, Action::Back, Action::NextPage]);
    }

    #[test]
    fn pad_repeat_timing() {
        let mut p = PadRepeat::new(&SET);
        let t0 = Instant::now();
        let (pr, rp) = p.update(k::DOWN, t0);
        assert_eq!((pr, rp), (k::DOWN, k::DOWN));
        let (pr, rp) = p.update(k::DOWN, t0 + Duration::from_millis(100));
        assert_eq!((pr, rp), (0, 0));
        let (pr, rp) = p.update(k::DOWN, t0 + Duration::from_millis(400));
        assert_eq!((pr, rp), (0, k::DOWN));
        let (_, rp) = p.update(k::DOWN, t0 + Duration::from_millis(420));
        assert_eq!(rp, 0, "rate limited");
        let (_, rp) = p.update(k::DOWN, t0 + Duration::from_millis(470));
        assert_eq!(rp, k::DOWN);
        let (pr, rp) = p.update(0, t0 + Duration::from_millis(500));
        assert_eq!((pr, rp), (0, 0));
    }

    #[test]
    fn every_key_name_inverts() {
        assert_eq!(ALL_KEYS.len(), Key::Count as usize, "ALL_KEYS lists every variant");
        for k in ALL_KEYS {
            assert_eq!(key_from_name(&key_name(k)), Some(k), "{k:?}");
        }
        let mut names: Vec<String> = ALL_KEYS.iter().map(|k| key_name(*k)).collect();
        names.sort();
        names.dedup();
        assert_eq!(names.len(), ALL_KEYS.len(), "names are unique");
        assert_eq!(key_from_name("Nonsense"), None);
    }

    #[test]
    fn key_names() {
        assert_eq!(key_name(Key::RightShift), "R Shift");
        assert_eq!(key_name(Key::F12), "F12");
        assert_eq!(key_name(Key::Key3), "3");
        assert_eq!(key_name(Key::Up), "↑");
    }

    #[test]
    fn pad_rebind_swaps_duplicates() {
        let mut m = pm();
        assert_eq!(m.rebind(k::A, Button::East), Some(k::B), "East was B: B takes A's old button");
        assert_eq!(m.button_for(k::A), Button::East);
        assert_eq!(m.button_for(k::B), Button::South);
        assert_eq!(m.rebind(k::L, Button::LeftTrigger2), None);
        assert_eq!(m.button_for(k::L), Button::LeftTrigger2);
        assert_eq!(m.rebind(k::L, Button::LeftTrigger2), None, "same button again is a no-op");
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
