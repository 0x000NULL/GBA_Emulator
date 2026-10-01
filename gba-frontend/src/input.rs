//! Keyboard (minifb) and gamepad (gilrs) -> GBA key mask.

use gba_core::Keys;
use gilrs::{Axis, Button, Gilrs};
use minifb::Key;

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

/// Gamepad button layout.
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

/// Left-stick deflection needed to register as a D-pad press.
pub const STICK_DEADZONE: f32 = 0.5;

/// Build the key mask from an "is this key down" predicate (the default layout; the live
/// layout is `ui::input::KeyMap`).
#[cfg_attr(not(test), allow(dead_code))]
pub fn keyboard_keys(down: impl Fn(Key) -> bool) -> u16 {
    KEYBOARD_MAP.iter().filter(|(k, _)| down(*k)).fold(0, |m, (_, bit)| m | bit)
}

/// Map an analog stick (gilrs convention: +x right, +y up) to D-pad bits.
pub fn stick_to_dpad(x: f32, y: f32, deadzone: f32) -> u16 {
    let mut m = 0;
    if x >= deadzone {
        m |= Keys::RIGHT;
    } else if x <= -deadzone {
        m |= Keys::LEFT;
    }
    if y >= deadzone {
        m |= Keys::UP;
    } else if y <= -deadzone {
        m |= Keys::DOWN;
    }
    m
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

pub struct Gamepads {
    gilrs: Option<Gilrs>,
}

impl Gamepads {
    pub fn new() -> Self {
        let gilrs = match Gilrs::new() {
            Ok(g) => {
                for (_, pad) in g.gamepads() {
                    println!("gamepad: {}", pad.name());
                }
                Some(g)
            }
            Err(e) => {
                eprintln!("warning: gamepad support unavailable ({e})");
                None
            }
        };
        Gamepads { gilrs }
    }

    /// Drain pending events and return the OR of every connected pad's key mask.
    pub fn poll(&mut self) -> u16 {
        let Some(g) = self.gilrs.as_mut() else { return 0 };
        while g.next_event().is_some() {}
        let mut mask = 0;
        for (_, pad) in g.gamepads() {
            for (btn, bit) in GAMEPAD_MAP {
                if pad.is_pressed(*btn) {
                    mask |= bit;
                }
            }
            let x = pad.value(Axis::LeftStickX);
            let y = pad.value(Axis::LeftStickY);
            mask |= stick_to_dpad(x, y, STICK_DEADZONE);
        }
        mask
    }
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

    #[test]
    fn stick_mapping() {
        assert_eq!(stick_to_dpad(0.0, 0.0, 0.5), 0);
        assert_eq!(stick_to_dpad(0.3, -0.3, 0.5), 0);
        assert_eq!(stick_to_dpad(1.0, 0.0, 0.5), Keys::RIGHT);
        assert_eq!(stick_to_dpad(-0.6, 0.9, 0.5), Keys::LEFT | Keys::UP);
        assert_eq!(stick_to_dpad(0.0, -1.0, 0.5), Keys::DOWN);
    }

    #[test]
    fn sanitize_opposites() {
        assert_eq!(sanitize(Keys::LEFT | Keys::RIGHT | Keys::A), Keys::A);
        assert_eq!(sanitize(Keys::UP | Keys::DOWN | Keys::LEFT), Keys::LEFT);
        assert_eq!(sanitize(0xFFFF), Keys::ALL & !(Keys::LEFT | Keys::RIGHT | Keys::UP | Keys::DOWN));
    }
}
