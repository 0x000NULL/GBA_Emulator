//! KEYINPUT / KEYCNT. Source: GBATEK "GBA Keypad Input".

use crate::irq::{Irq, IrqController};

/// Active-high key bitmask in GBA bit order (KEYINPUT is the inverse).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Keys(pub u16);

impl Keys {
    pub const A: u16 = 1 << 0;
    pub const B: u16 = 1 << 1;
    pub const SELECT: u16 = 1 << 2;
    pub const START: u16 = 1 << 3;
    pub const RIGHT: u16 = 1 << 4;
    pub const LEFT: u16 = 1 << 5;
    pub const UP: u16 = 1 << 6;
    pub const DOWN: u16 = 1 << 7;
    pub const R: u16 = 1 << 8;
    pub const L: u16 = 1 << 9;
    pub const ALL: u16 = 0x03FF;

    pub const NONE: Keys = Keys(0);

    /// Parse a key name as used in input scripts ("A", "B", "SELECT", "START", "RIGHT", "LEFT", "UP", "DOWN", "R", "L").
    pub fn from_name(name: &str) -> Option<u16> {
        Some(match name.to_ascii_uppercase().as_str() {
            "A" => Keys::A,
            "B" => Keys::B,
            "SELECT" => Keys::SELECT,
            "START" => Keys::START,
            "RIGHT" => Keys::RIGHT,
            "LEFT" => Keys::LEFT,
            "UP" => Keys::UP,
            "DOWN" => Keys::DOWN,
            "R" => Keys::R,
            "L" => Keys::L,
            _ => return None,
        })
    }
}

#[cfg_attr(feature = "savestate", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Default, Clone)]
pub struct Keypad {
    /// Active-high pressed mask.
    pub pressed: u16,
    /// 4000132h KEYCNT
    pub keycnt: u16,
}

impl Keypad {
    pub fn new() -> Self {
        Self::default()
    }

    /// 4000130h KEYINPUT: 0 = pressed.
    pub fn keyinput(&self) -> u16 {
        !self.pressed & Keys::ALL
    }

    pub fn set_keys(&mut self, keys: Keys, irq: &mut IrqController) {
        self.pressed = keys.0 & Keys::ALL;
        self.check_irq(irq);
    }

    /// KEYCNT bit 14 enables the IRQ; bit 15 selects AND (all selected keys) vs OR (any).
    pub fn check_irq(&self, irq: &mut IrqController) {
        if self.keycnt & 0x4000 == 0 {
            return;
        }
        let sel = self.keycnt & Keys::ALL;
        let hit = if self.keycnt & 0x8000 != 0 {
            sel != 0 && (self.pressed & sel) == sel
        } else {
            (self.pressed & sel) != 0
        };
        if hit {
            irq.raise(Irq::Keypad);
            irq.stopped = false;
        }
    }

    pub fn read8(&self, off: u16) -> u8 {
        match off {
            0x130 => self.keyinput() as u8,
            0x131 => (self.keyinput() >> 8) as u8,
            0x132 => self.keycnt as u8,
            0x133 => (self.keycnt >> 8) as u8,
            _ => 0,
        }
    }

    pub fn write8(&mut self, off: u16, v: u8, irq: &mut IrqController) {
        match off {
            0x132 => self.keycnt = (self.keycnt & 0xFF00) | v as u16,
            0x133 => self.keycnt = (self.keycnt & 0x00FF) | ((v as u16) << 8),
            _ => return,
        }
        self.keycnt &= 0xC3FF;
        self.check_irq(irq);
    }
}
