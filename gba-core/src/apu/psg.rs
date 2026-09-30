//! PSG channels 1-4. Source: GBATEK "GBA Sound Channel 1 - Tone & Sweep", "Channel 2 - Tone",
//! "Channel 3 - Wave Output", "Channel 4 - Noise".
//!
//! Every channel keeps a down-counting cycle timer; `step(cycles)` advances it by whole system
//! cycles (16.78 MHz) so the generator frequencies are exact. The mixer samples the channel
//! outputs every [`PSG_TICK_CYCLES`] cycles (262.144 kHz, the rate GBATEK gives for the internal
//! PSG generation) and averages them down to the SOUNDBIAS output rate.

/// PSG channels are "internally generated at 262.144kHz" (GBATEK): one tick every 64 cycles.
pub const PSG_TICK_CYCLES: u32 = 64;

// ------------------------------------------------------------------ length counter

/// Sound length counter, clocked at 256 Hz by the frame sequencer.
#[derive(Clone, Debug)]
pub struct Length {
    pub counter: u16,
    /// 64 for channels 1/2/4, 256 for channel 3.
    max: u16,
}

impl Length {
    pub fn new(max: u16) -> Self {
        Length { counter: 0, max }
    }

    /// Write of the length field `n`: the channel plays for `(max - n)` length ticks.
    pub fn load(&mut self, n: u16) {
        self.counter = self.max - n;
    }

    /// Restart: an expired counter is reloaded to the full length.
    pub fn trigger(&mut self) {
        if self.counter == 0 {
            self.counter = self.max;
        }
    }

    /// 256 Hz tick. Returns true when the counter just expired (channel must stop).
    pub fn clock(&mut self, length_enabled: bool) -> bool {
        if length_enabled && self.counter > 0 {
            self.counter -= 1;
            return self.counter == 0;
        }
        false
    }

    pub fn reset(&mut self) {
        self.counter = 0;
    }
}

// ------------------------------------------------------------------ envelope

/// Volume envelope (NRx2), clocked at 64 Hz.
#[derive(Clone, Debug, Default)]
pub struct Envelope {
    /// Bits 12-15: initial volume.
    pub initial: u8,
    /// Bit 11: 1 = increase.
    pub increase: bool,
    /// Bits 8-10: step time in 1/64 s units; 0 = no envelope.
    pub period: u8,
    /// Current volume 0..15.
    pub volume: u8,
    timer: u8,
}

impl Envelope {
    /// Write the NRx2 byte (bits 0-2 step time, 3 direction, 4-7 initial volume).
    pub fn write(&mut self, v: u8) {
        self.period = v & 7;
        self.increase = v & 0x08 != 0;
        self.initial = v >> 4;
    }

    /// The channel's DAC is powered when the envelope can produce a non-zero level
    /// ("Initial Volume of envelope (1-15, 0=No Sound)").
    pub fn dac_on(&self) -> bool {
        self.initial != 0 || self.increase
    }

    pub fn trigger(&mut self) {
        self.volume = self.initial;
        self.timer = self.period;
    }

    /// 64 Hz tick.
    pub fn clock(&mut self) {
        if self.period == 0 {
            return;
        }
        if self.timer > 0 {
            self.timer -= 1;
        }
        if self.timer == 0 {
            self.timer = self.period;
            if self.increase {
                if self.volume < 15 {
                    self.volume += 1;
                }
            } else if self.volume > 0 {
                self.volume -= 1;
            }
        }
    }

    pub fn reset(&mut self) {
        *self = Envelope::default();
    }
}

// ------------------------------------------------------------------ sweep

/// Frequency sweep (NR10), clocked at 128 Hz. `X(t) = X(t-1) +/- X(t-1)/2^n`.
#[derive(Clone, Debug, Default)]
pub struct Sweep {
    pub shift: u8,
    pub decrease: bool,
    pub period: u8,
    timer: u8,
    enabled: bool,
    shadow: u16,
}

impl Sweep {
    pub fn write(&mut self, v: u8) {
        self.shift = v & 7;
        self.decrease = v & 0x08 != 0;
        self.period = (v >> 4) & 7;
    }

    fn reload_timer(&mut self) {
        // A zero sweep time behaves like 8 for the internal timer (sweep stays disabled anyway).
        self.timer = if self.period == 0 { 8 } else { self.period };
    }

    /// Next frequency, or `None` on overflow (> 2047), which must stop the channel.
    fn calc(&self) -> Option<u16> {
        let delta = self.shadow >> self.shift;
        let next = if self.decrease { self.shadow.wrapping_sub(delta) } else { self.shadow + delta };
        if next > 2047 {
            None
        } else {
            Some(next)
        }
    }

    /// Restart. Returns false when the initial overflow check already disables the channel.
    pub fn trigger(&mut self, freq: u16) -> bool {
        self.shadow = freq;
        self.reload_timer();
        self.enabled = self.period != 0 || self.shift != 0;
        if self.shift != 0 {
            return self.calc().is_some();
        }
        true
    }

    /// 128 Hz tick. Returns `Ok(Some(f))` when the frequency changed to `f`, `Ok(None)` when
    /// nothing happened, `Err(())` when the sweep overflowed and the channel must stop.
    pub fn clock(&mut self) -> Result<Option<u16>, ()> {
        if self.timer > 0 {
            self.timer -= 1;
        }
        if self.timer != 0 {
            return Ok(None);
        }
        self.reload_timer();
        if !self.enabled || self.period == 0 {
            return Ok(None);
        }
        let next = self.calc().ok_or(())?;
        if self.shift == 0 {
            return Ok(None);
        }
        self.shadow = next;
        // Second overflow check with the new frequency (does not write it back).
        self.calc().ok_or(())?;
        Ok(Some(next))
    }

    pub fn reset(&mut self) {
        *self = Sweep::default();
    }
}

// ------------------------------------------------------------------ square (channels 1 and 2)

/// Wave duty patterns from GBATEK: 12.5% `-_______`, 25% `--______`, 50% `----____`, 75% `------__`.
/// Bit `i` set = output high during duty step `i`.
pub const DUTY_TABLE: [u8; 4] = [0b0000_0001, 0b0000_0011, 0b0000_1111, 0b0011_1111];

#[derive(Clone, Debug)]
pub struct Square {
    pub enabled: bool,
    pub duty: u8,
    /// 11-bit frequency value; tone = 131072/(2048-n) Hz.
    pub freq: u16,
    pub length_enabled: bool,
    pub length: Length,
    pub envelope: Envelope,
    /// Channel 1 only.
    pub sweep: Option<Sweep>,
    /// Cycles until the next duty step.
    timer: u32,
    /// Duty step 0..7.
    pos: u8,
}

impl Square {
    pub fn new(with_sweep: bool) -> Self {
        Square {
            enabled: false,
            duty: 0,
            freq: 0,
            length_enabled: false,
            length: Length::new(64),
            envelope: Envelope::default(),
            sweep: if with_sweep { Some(Sweep::default()) } else { None },
            timer: 2048 * 16,
            pos: 0,
        }
    }

    /// Cycles per duty step: the tone has 8 steps at 131072/(2048-n) Hz, so a step lasts
    /// 16.78M / (8 * 131072/(2048-n)) = (2048-n) * 16 cycles.
    #[inline]
    pub fn period(&self) -> u32 {
        (2048 - self.freq as u32) * 16
    }

    pub fn dac_on(&self) -> bool {
        self.envelope.dac_on()
    }

    /// Restart (bit 15 of the frequency register).
    pub fn trigger(&mut self) {
        self.enabled = self.dac_on();
        self.timer = self.period();
        self.length.trigger();
        self.envelope.trigger();
        if let Some(sw) = &mut self.sweep {
            if !sw.trigger(self.freq) {
                self.enabled = false;
            }
        }
    }

    /// Advance the generator by `cycles` system cycles.
    pub fn step(&mut self, mut cycles: u32) {
        while cycles >= self.timer {
            cycles -= self.timer;
            self.timer = self.period();
            self.pos = (self.pos + 1) & 7;
        }
        self.timer -= cycles;
    }

    /// Current 4-bit level (0 when the channel is stopped).
    #[inline]
    pub fn output(&self) -> u8 {
        if self.enabled && self.dac_on() && (DUTY_TABLE[self.duty as usize] >> self.pos) & 1 != 0 {
            self.envelope.volume
        } else {
            0
        }
    }

    pub fn clock_length(&mut self) {
        if self.length.clock(self.length_enabled) {
            self.enabled = false;
        }
    }

    pub fn clock_envelope(&mut self) {
        self.envelope.clock();
    }

    pub fn clock_sweep(&mut self) {
        if let Some(sw) = &mut self.sweep {
            match sw.clock() {
                Ok(Some(f)) => self.freq = f,
                Ok(None) => {}
                Err(()) => self.enabled = false,
            }
        }
    }

    pub fn reset(&mut self) {
        let with_sweep = self.sweep.is_some();
        *self = Square::new(with_sweep);
    }
}

// ------------------------------------------------------------------ wave (channel 3)

#[derive(Clone, Debug)]
pub struct Wave {
    pub enabled: bool,
    /// NR30 bit 7: "Sound Channel 3 Off (0=Stop, 1=Playback)" — the channel's DAC.
    pub dac_on: bool,
    /// NR30 bit 5: 1 = two banks / 64 digits.
    pub two_banks: bool,
    /// NR30 bit 6: bank being played; CPU accesses address the other one.
    pub bank: u8,
    /// NR32 bits 13-14: 0 mute, 1 100%, 2 50%, 3 25%.
    pub volume: u8,
    /// NR32 bit 15: force 75%.
    pub force75: bool,
    pub freq: u16,
    pub length_enabled: bool,
    pub length: Length,
    pub ram: [[u8; 16]; 2],
    timer: u32,
    /// Digit index 0..31 (0..63 in two-bank mode).
    pos: u8,
    /// Digit currently being output.
    sample: u8,
}

impl Wave {
    pub fn new() -> Self {
        Wave {
            enabled: false,
            dac_on: false,
            two_banks: false,
            bank: 0,
            volume: 0,
            force75: false,
            freq: 0,
            length_enabled: false,
            length: Length::new(256),
            ram: [[0; 16]; 2],
            timer: 2048 * 8,
            pos: 0,
            sample: 0,
        }
    }

    /// Cycles per digit: sample rate is 2097152/(2048-n) Hz => (2048-n) * 8 cycles.
    #[inline]
    pub fn period(&self) -> u32 {
        (2048 - self.freq as u32) * 8
    }

    /// Write NR30 (0x70).
    pub fn write_ctrl(&mut self, v: u8) {
        self.two_banks = v & 0x20 != 0;
        self.bank = (v >> 6) & 1;
        self.dac_on = v & 0x80 != 0;
        if !self.dac_on {
            self.enabled = false;
        }
    }

    /// Write NR32 high byte (0x73).
    pub fn write_volume(&mut self, v: u8) {
        self.volume = (v >> 5) & 3;
        self.force75 = v & 0x80 != 0;
    }

    /// CPU access to wave RAM goes to the bank that is not selected for playback.
    pub fn ram_read(&self, i: usize) -> u8 {
        self.ram[(self.bank ^ 1) as usize][i & 15]
    }

    pub fn ram_write(&mut self, i: usize, v: u8) {
        self.ram[(self.bank ^ 1) as usize][i & 15] = v;
    }

    /// Digit `i` of the playback sequence: MSBs of byte 0, LSBs of byte 0, MSBs of byte 1, ...
    /// In two-bank mode the selected bank plays first, then the other one.
    fn digit(&self, i: u8) -> u8 {
        let bank = if self.two_banks { (self.bank + (i >> 5)) & 1 } else { self.bank };
        let byte = self.ram[bank as usize][((i & 31) >> 1) as usize];
        if i & 1 == 0 {
            byte >> 4
        } else {
            byte & 0x0F
        }
    }

    fn digits_per_loop(&self) -> u8 {
        if self.two_banks {
            64
        } else {
            32
        }
    }

    pub fn trigger(&mut self) {
        self.enabled = self.dac_on;
        self.timer = self.period();
        self.length.trigger();
        self.pos = 0;
        self.sample = self.digit(0);
    }

    pub fn step(&mut self, mut cycles: u32) {
        while cycles >= self.timer {
            cycles -= self.timer;
            self.timer = self.period();
            self.pos = (self.pos + 1) % self.digits_per_loop();
            self.sample = self.digit(self.pos);
        }
        self.timer -= cycles;
    }

    /// Current level after the volume setting: 100%/50%/25%, or forced 75%.
    #[inline]
    pub fn output(&self) -> u8 {
        if !(self.enabled && self.dac_on) {
            return 0;
        }
        let s = self.sample;
        if self.force75 {
            (s * 3) >> 2
        } else {
            match self.volume {
                0 => 0,
                1 => s,
                2 => s >> 1,
                _ => s >> 2,
            }
        }
    }

    pub fn clock_length(&mut self) {
        if self.length.clock(self.length_enabled) {
            self.enabled = false;
        }
    }

    /// Master-off reset: wave RAM contents survive (0x90-0x9F is outside the reset range).
    pub fn reset(&mut self) {
        let ram = self.ram;
        *self = Wave::new();
        self.ram = ram;
    }
}

impl Default for Wave {
    fn default() -> Self {
        Self::new()
    }
}

// ------------------------------------------------------------------ noise (channel 4)

/// Dividing ratio `r` -> base divisor in units of 4 cycles: "For r=0 assume r=0.5 instead".
/// Frequency = 524288 / r / 2^(s+1) Hz  =>  period = r * 2^(s+6) cycles = (DIVISOR[r] << s) * 4.
pub const NOISE_DIVISOR: [u32; 8] = [8, 16, 32, 48, 64, 80, 96, 112];

#[derive(Clone, Debug)]
pub struct Noise {
    pub enabled: bool,
    pub length_enabled: bool,
    pub length: Length,
    pub envelope: Envelope,
    /// Bits 0-2 of NR43.
    pub ratio: u8,
    /// Bit 3: 1 = 7-bit counter.
    pub width7: bool,
    /// Bits 4-7: shift clock frequency.
    pub shift: u8,
    pub lfsr: u16,
    timer: u32,
    high: bool,
}

impl Noise {
    pub fn new() -> Self {
        Noise {
            enabled: false,
            length_enabled: false,
            length: Length::new(64),
            envelope: Envelope::default(),
            ratio: 0,
            width7: false,
            shift: 0,
            lfsr: 0x4000,
            timer: NOISE_DIVISOR[0] << 2,
            high: false,
        }
    }

    #[inline]
    pub fn period(&self) -> u32 {
        NOISE_DIVISOR[self.ratio as usize] << (self.shift as u32 + 2)
    }

    /// Write NR43 (0x7C).
    pub fn write_poly(&mut self, v: u8) {
        self.ratio = v & 7;
        self.width7 = v & 0x08 != 0;
        self.shift = v >> 4;
    }

    pub fn dac_on(&self) -> bool {
        self.envelope.dac_on()
    }

    pub fn trigger(&mut self) {
        self.enabled = self.dac_on();
        self.timer = self.period();
        self.length.trigger();
        self.envelope.trigger();
        // "The initial value when (re-)starting the sound is X=40h (7bit) or X=4000h (15bit)."
        self.lfsr = if self.width7 { 0x40 } else { 0x4000 };
        self.high = false;
    }

    /// One shift of the polynomial counter:
    /// `X=X SHR 1, IF carry THEN Out=HIGH, X=X XOR 6000h (60h) ELSE Out=LOW`.
    #[inline]
    pub fn clock_lfsr(&mut self) {
        let carry = self.lfsr & 1 != 0;
        self.lfsr >>= 1;
        if carry {
            self.lfsr ^= if self.width7 { 0x60 } else { 0x6000 };
        }
        self.high = carry;
    }

    pub fn step(&mut self, mut cycles: u32) {
        while cycles >= self.timer {
            cycles -= self.timer;
            self.timer = self.period();
            self.clock_lfsr();
        }
        self.timer -= cycles;
    }

    #[inline]
    pub fn output(&self) -> u8 {
        if self.enabled && self.dac_on() && self.high {
            self.envelope.volume
        } else {
            0
        }
    }

    pub fn clock_length(&mut self) {
        if self.length.clock(self.length_enabled) {
            self.enabled = false;
        }
    }

    pub fn clock_envelope(&mut self) {
        self.envelope.clock();
    }

    pub fn reset(&mut self) {
        *self = Noise::new();
    }
}

impl Default for Noise {
    fn default() -> Self {
        Self::new()
    }
}
