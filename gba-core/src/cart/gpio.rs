//! 4-bit GPIO port in the ROM chip at 0x080000C4-0x080000C9 and the devices wired to it.
//! Source: GBATEK "GBA Cart I/O Port (GPIO)".
//!
//!   C4h data (bits 0-3), C6h direction (0=in, 1=out), C8h control (bit0: 0=write-only, 1=R/W).
//!   In write-only mode reads see the ROM bytes underneath (zero-filled in most images).
//!
//! Pin map (GBATEK's connection table, Boktai): RTC SCK=0, SIO=1, CS=2; solar sensor CLK=0,
//! RST=1, FLG=3. A device sees an output pin's level `data & dir`; an input pin reads what the
//! device drives.

use super::rtc::Rtc;
use super::solar::Solar;

pub const GPIO_DATA: u32 = 0xC4;
pub const GPIO_DIR: u32 = 0xC6;
pub const GPIO_CTRL: u32 = 0xC8;

/// Which devices sit behind the port.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct GpioDevices {
    pub rtc: bool,
    pub solar: bool,
}

#[cfg_attr(feature = "savestate", derive(serde::Serialize, serde::Deserialize))]
pub struct Gpio {
    data: u8,
    dir: u8,
    readable: bool,
    pub rtc: Option<Rtc>,
    pub solar: Option<Solar>,
}

impl Gpio {
    pub fn new(dev: GpioDevices) -> Self {
        Gpio {
            data: 0,
            dir: 0,
            readable: false,
            rtc: dev.rtc.then(Rtc::new),
            solar: dev.solar.then(Solar::new),
        }
    }

    pub fn devices(&self) -> GpioDevices {
        GpioDevices { rtc: self.rtc.is_some(), solar: self.solar.is_some() }
    }

    /// `off` is the address within the ROM space (`addr & 0x01FF_FFFF`).
    #[inline]
    pub fn is_reg(off: u32) -> bool {
        (GPIO_DATA..GPIO_CTRL + 2).contains(&off)
    }

    /// True when reads of the registers return them instead of the ROM bytes.
    #[inline]
    pub fn readable(&self) -> bool {
        self.readable
    }

    /// Levels the devices drive onto the port (only input pins pass them through).
    fn device_out(&self) -> u8 {
        let mut v = 0;
        if let Some(rtc) = &self.rtc {
            v |= (rtc.sio_out() as u8) << 1;
        }
        if let Some(s) = &self.solar {
            v |= (s.flag() as u8) << 3;
        }
        v
    }

    /// Halfword read of a register (caller checked `is_reg` and `readable`).
    pub fn read16(&self, off: u32) -> u16 {
        match off & !1 {
            GPIO_DATA => ((self.data & self.dir) | (self.device_out() & !self.dir)) as u16 & 0xF,
            GPIO_DIR => self.dir as u16,
            _ => self.readable as u16,
        }
    }

    /// Halfword write (ROM-bus writes are 16-bit; 8-bit ones arrive here zero-extended).
    /// `now` is the scheduler cycle, for the clock.
    pub fn write16(&mut self, off: u32, v: u16, now: u64) {
        match off & !1 {
            GPIO_DATA => {
                self.data = v as u8 & 0xF;
                let pins = self.data & self.dir;
                if let Some(rtc) = &mut self.rtc {
                    rtc.write_pins(pins, self.dir, now);
                }
                if let Some(s) = &mut self.solar {
                    s.write_pins(pins);
                }
            }
            GPIO_DIR => self.dir = v as u8 & 0xF,
            _ => self.readable = v & 1 != 0,
        }
    }
}
