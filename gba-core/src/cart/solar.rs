//! Boktai solar sensor: a digital-ramp A/D converter on the GPIO port.
//! Source: GBATEK "GBA Cart Solar Sensor".
//!
//! RST (GPIO bit1) high clears a binary counter, each CLK (bit0) rising edge counts it up (12 bits,
//! only the low 8 reach the D/A ladder), and FLG (bit3) goes high once the ramp voltage exceeds
//! the sensor's. GBATEK's sampling loop therefore exits with the sensor value: E8h in total
//! darkness, about 50h where Boktai's gauge is full, lower for brighter light.

/// Light levels the host can choose (0 = dark .. 10 = full gauge).
pub const SUN_MAX: u8 = 10;
const ADC_DARK: u32 = 0xE8;
const ADC_FULL: u32 = 0x50;

#[cfg_attr(feature = "savestate", derive(serde::Serialize, serde::Deserialize))]
pub struct Solar {
    counter: u16,
    clk: bool,
    level: u8,
}

impl Solar {
    pub fn new() -> Self {
        Solar { counter: 0, clk: false, level: 0 }
    }

    pub fn level(&self) -> u8 {
        self.level
    }

    pub fn set_level(&mut self, level: u8) {
        self.level = level.min(SUN_MAX);
    }

    /// The sensor value the sampling loop reads for the current level.
    pub fn adc(&self) -> u8 {
        (ADC_DARK - (ADC_DARK - ADC_FULL) * self.level as u32 / SUN_MAX as u32) as u8
    }

    /// Comparator output: ramp (counter low 8 bits) above the sensor value.
    pub fn flag(&self) -> bool {
        (self.counter & 0xFF) as u8 > self.adc()
    }

    pub fn write_pins(&mut self, pins: u8) {
        let clk = pins & 1 != 0;
        if pins & 2 != 0 {
            self.counter = 0;
        } else if clk && !self.clk {
            self.counter = (self.counter + 1) & 0xFFF;
        }
        self.clk = clk;
    }
}

impl Default for Solar {
    fn default() -> Self {
        Self::new()
    }
}
