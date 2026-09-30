//! APU. Source: GBATEK "GBA Sound Controller" and its subsections.
//!
//! Public surface the bus depends on: IO byte access for 0x060-0x0AF, FIFO writes, the timer
//! overflow -> FIFO -> DMA request path, the 512 Hz frame sequencer and the sample sink.
//!
//! Structure: `psg` holds the four tone/noise generators driven in system cycles, `fifo` the two
//! DirectSound FIFOs, `mixer` the SOUNDCNT_L/H volume mix, SOUNDBIAS and the 10-bit -> 16-bit
//! conversion. Registers with R/W bits keep a raw byte mirror (`raw`) that is masked on read.

pub mod fifo;
pub mod mixer;
pub mod psg;
pub mod regs;
#[cfg(test)]
mod tests;

pub use fifo::{Fifo, FifoRequest};

use crate::sched::{Event, Scheduler};
use crate::timing::{CYCLES_PER_AUDIO_SAMPLE, CYCLES_PER_FRAME_SEQ};
use mixer::MixInput;
use psg::{Noise, Square, Wave, PSG_TICK_CYCLES};
use regs::*;

#[cfg_attr(feature = "savestate", derive(serde::Serialize, serde::Deserialize))]
pub struct Apu {
    /// Raw mirror of 0x60..0x8F as written (masked on read). Wave RAM and FIFOs live elsewhere.
    #[cfg_attr(feature = "savestate", serde(with = "crate::state::bytes_arr"))]
    raw: [u8; 0x30],
    pub sq1: Square,
    pub sq2: Square,
    pub wave: Wave,
    pub noise: Noise,
    /// Frame sequencer step 0..7 (512 Hz).
    pub frame_step: u8,
    pub fifo_a: Fifo,
    pub fifo_b: Fifo,
    /// Stereo output samples at `cycles_per_sample()`; drained by the frontend/harness.
    #[cfg_attr(feature = "savestate", serde(skip))]
    pub samples: Vec<[i16; 2]>,
    pub sample_count: u64,
}

impl Default for Apu {
    fn default() -> Self {
        Self::new()
    }
}

impl Apu {
    pub fn new() -> Self {
        let mut a = Apu {
            raw: [0; 0x30],
            sq1: Square::new(true),
            sq2: Square::new(false),
            wave: Wave::new(),
            noise: Noise::new(),
            frame_step: 0,
            fifo_a: Fifo::default(),
            fifo_b: Fifo::default(),
            samples: Vec::new(),
            sample_count: 0,
        };
        // SOUNDBIAS = 0x200 after boot.
        a.raw[SOUNDBIAS - BASE] = 0x00;
        a.raw[SOUNDBIAS + 1 - BASE] = 0x02;
        a
    }

    /// 16-bit register value as readable by the CPU (IO offset 0x60..0xAE).
    #[inline]
    pub fn reg16(&self, off: usize) -> u16 {
        let off = off & !1;
        u16::from_le_bytes([self.read_io8(off as u16), self.read_io8(off as u16 + 1)])
    }

    #[inline]
    fn raw16(&self, off: usize) -> u16 {
        u16::from_le_bytes([self.raw[off - BASE], self.raw[off + 1 - BASE]])
    }

    /// SOUNDCNT_X bit 7: master enable.
    #[inline]
    pub fn master_enabled(&self) -> bool {
        self.raw[SOUNDCNT_X - BASE] & 0x80 != 0
    }

    /// SOUNDBIAS as written (bits 1-9 level, 14-15 resolution).
    pub fn soundbias(&self) -> u16 {
        self.raw16(SOUNDBIAS)
    }

    /// Output cycles per sample from SOUNDBIAS bits 14-15 (0: 32.768 kHz ... 3: 262.144 kHz).
    pub fn cycles_per_sample(&self) -> u64 {
        let amp = (self.raw[SOUNDBIAS + 1 - BASE] >> 6) & 3;
        CYCLES_PER_AUDIO_SAMPLE >> amp
    }

    pub fn boot_schedule(&mut self, sched: &mut Scheduler) {
        sched.schedule_in(Event::ApuSample, self.cycles_per_sample());
        sched.schedule_in(Event::ApuFrameSeq, CYCLES_PER_FRAME_SEQ);
    }

    /// SOUNDCNT_X bits 0-3: channel ON flags (read-only).
    fn status_bits(&self) -> u8 {
        (self.sq1.enabled as u8) | (self.sq2.enabled as u8) << 1 | (self.wave.enabled as u8) << 2 | (self.noise.enabled as u8) << 3
    }

    // ---------------------------------------------------------------- IO

    pub fn read_io8(&self, off: u16) -> u8 {
        let i = off as usize;
        if !(BASE..END).contains(&i) {
            return 0;
        }
        match i {
            SOUNDCNT_X => (self.raw[i - BASE] & 0x80) | self.status_bits(),
            WAVE_RAM..=0x9F => self.wave.ram_read(i - WAVE_RAM),
            0xA0..=0xAF => 0,
            _ => self.raw[i - BASE] & read_mask8(i),
        }
    }

    pub fn write_io8(&mut self, off: u16, v: u8, _sched: &mut Scheduler) {
        let i = off as usize;
        if !(BASE..END).contains(&i) {
            return;
        }
        match i {
            // While the master enable is clear, PSG registers 0x60..0x81 are held at zero.
            0x60..=0x81 if !self.master_enabled() => {}
            0x60 => {
                self.raw[i - BASE] = v & 0x7F;
                if let Some(sw) = &mut self.sq1.sweep {
                    sw.write(v);
                }
            }
            0x62 => {
                self.raw[i - BASE] = v;
                self.sq1.duty = v >> 6;
                self.sq1.length.load((v & 0x3F) as u16);
            }
            0x63 => {
                self.raw[i - BASE] = v;
                self.sq1.envelope.write(v);
                if !self.sq1.dac_on() {
                    self.sq1.enabled = false;
                }
            }
            0x64 => {
                self.raw[i - BASE] = v;
                self.sq1.freq = (self.sq1.freq & 0x700) | v as u16;
            }
            0x65 => {
                self.raw[i - BASE] = v & 0x47;
                self.sq1.freq = (self.sq1.freq & 0xFF) | ((v as u16 & 7) << 8);
                self.sq1.length_enabled = v & 0x40 != 0;
                if v & 0x80 != 0 {
                    self.sq1.trigger();
                }
            }
            0x68 => {
                self.raw[i - BASE] = v;
                self.sq2.duty = v >> 6;
                self.sq2.length.load((v & 0x3F) as u16);
            }
            0x69 => {
                self.raw[i - BASE] = v;
                self.sq2.envelope.write(v);
                if !self.sq2.dac_on() {
                    self.sq2.enabled = false;
                }
            }
            0x6C => {
                self.raw[i - BASE] = v;
                self.sq2.freq = (self.sq2.freq & 0x700) | v as u16;
            }
            0x6D => {
                self.raw[i - BASE] = v & 0x47;
                self.sq2.freq = (self.sq2.freq & 0xFF) | ((v as u16 & 7) << 8);
                self.sq2.length_enabled = v & 0x40 != 0;
                if v & 0x80 != 0 {
                    self.sq2.trigger();
                }
            }
            0x70 => {
                self.raw[i - BASE] = v & 0xE0;
                self.wave.write_ctrl(v);
            }
            0x72 => {
                self.raw[i - BASE] = v;
                self.wave.length.load(v as u16);
            }
            0x73 => {
                self.raw[i - BASE] = v & 0xE0;
                self.wave.write_volume(v);
            }
            0x74 => {
                self.raw[i - BASE] = v;
                self.wave.freq = (self.wave.freq & 0x700) | v as u16;
            }
            0x75 => {
                self.raw[i - BASE] = v & 0x47;
                self.wave.freq = (self.wave.freq & 0xFF) | ((v as u16 & 7) << 8);
                self.wave.length_enabled = v & 0x40 != 0;
                if v & 0x80 != 0 {
                    self.wave.trigger();
                }
            }
            0x78 => {
                self.raw[i - BASE] = v & 0x3F;
                self.noise.length.load((v & 0x3F) as u16);
            }
            0x79 => {
                self.raw[i - BASE] = v;
                self.noise.envelope.write(v);
                if !self.noise.dac_on() {
                    self.noise.enabled = false;
                }
            }
            0x7C => {
                self.raw[i - BASE] = v;
                self.noise.write_poly(v);
            }
            0x7D => {
                self.raw[i - BASE] = v & 0x40;
                self.noise.length_enabled = v & 0x40 != 0;
                if v & 0x80 != 0 {
                    self.noise.trigger();
                }
            }
            0x80 => self.raw[i - BASE] = v & 0x77,
            0x81 => self.raw[i - BASE] = v,
            // SOUNDCNT_H is writable regardless of the master enable.
            0x82 => self.raw[i - BASE] = v & 0x0F,
            0x83 => {
                // Bits 3/7 of the high byte (register bits 11/15) reset FIFO A/B.
                if v & 0x08 != 0 {
                    self.fifo_a.reset();
                }
                if v & 0x80 != 0 {
                    self.fifo_b.reset();
                }
                self.raw[i - BASE] = v & 0x77;
            }
            SOUNDCNT_X => {
                let was_on = self.master_enabled();
                let on = v & 0x80 != 0;
                self.raw[i - BASE] = v & 0x80;
                if was_on && !on {
                    self.master_off();
                } else if !was_on && on {
                    self.frame_step = 0;
                }
            }
            0x88 => self.raw[i - BASE] = v & 0xFE,
            0x89 => self.raw[i - BASE] = v & 0xC3,
            WAVE_RAM..=0x9F => self.wave.ram_write(i - WAVE_RAM, v),
            0xA0..=0xA3 => self.fifo_a.push(&[v]),
            0xA4..=0xA7 => self.fifo_b.push(&[v]),
            _ => {}
        }
    }

    /// SOUNDCNT_X bit 7 cleared: "all PSG registers at 4000060h..4000081h are reset to zero".
    fn master_off(&mut self) {
        for b in &mut self.raw[0..(0x82 - BASE)] {
            *b = 0;
        }
        self.sq1.reset();
        self.sq2.reset();
        self.wave.reset();
        self.noise.reset();
        self.frame_step = 0;
    }

    /// 16/32-bit FIFO writes push all bytes in order (Classic NES Series writes 16 bits).
    pub fn fifo_write(&mut self, which: usize, bytes: &[u8]) {
        if which == 0 {
            self.fifo_a.push(bytes);
        } else {
            self.fifo_b.push(bytes);
        }
    }

    // ---------------------------------------------------------------- events

    /// Timer `t` overflowed. Each FIFO clocked by `t` (SOUNDCNT_H bit 10 / bit 14) pops a sample;
    /// returns which FIFOs now need a DMA refill (<= 16 bytes left). Nothing happens while the
    /// master enable is clear ("both PSG and FIFO sounds are disabled").
    pub fn on_timer_overflow(&mut self, t: u8) -> FifoRequest {
        let mut req = FifoRequest::default();
        if t >= 2 || !self.master_enabled() {
            return req;
        }
        let cnt_h = self.raw16(SOUNDCNT_H);
        if (cnt_h >> 10) & 1 == t as u16 {
            // Underruns only count for a FIFO that is routed to at least one speaker (bits 8/9).
            req.a = self.fifo_a.pop_ex(cnt_h & 0x0300 != 0);
        }
        if (cnt_h >> 14) & 1 == t as u16 {
            req.b = self.fifo_b.pop_ex(cnt_h & 0x3000 != 0);
        }
        req
    }

    /// Produce one output sample: advance the PSG generators by `cycles_per_sample()` cycles in
    /// 262.144 kHz ticks, average their levels, then mix with the FIFO samples.
    pub fn on_sample(&mut self) {
        let ticks = (self.cycles_per_sample() as u32 / PSG_TICK_CYCLES).max(1);
        let mut sum = [0u32; 4];
        for _ in 0..ticks {
            self.sq1.step(PSG_TICK_CYCLES);
            self.sq2.step(PSG_TICK_CYCLES);
            self.wave.step(PSG_TICK_CYCLES);
            self.noise.step(PSG_TICK_CYCLES);
            sum[0] += self.sq1.output() as u32;
            sum[1] += self.sq2.output() as u32;
            sum[2] += self.wave.output() as u32;
            sum[3] += self.noise.output() as u32;
        }
        let inp = MixInput {
            psg_sum: sum,
            ticks,
            cnt_l: self.raw16(SOUNDCNT_L),
            cnt_h: self.raw16(SOUNDCNT_H),
            fifo_a: self.fifo_a.sample,
            fifo_b: self.fifo_b.sample,
            bias: self.raw16(SOUNDBIAS),
            master: self.master_enabled(),
        };
        self.samples.push(mixer::mix(&inp));
        self.sample_count += 1;
    }

    /// 512 Hz frame sequencer tick: length at 256 Hz (steps 0,2,4,6), sweep at 128 Hz
    /// (steps 2,6), envelope at 64 Hz (step 7).
    pub fn on_frame_seq(&mut self) {
        if !self.master_enabled() {
            return;
        }
        match self.frame_step {
            0 | 4 => self.clock_lengths(),
            2 | 6 => {
                self.clock_lengths();
                self.sq1.clock_sweep();
            }
            7 => {
                self.sq1.clock_envelope();
                self.sq2.clock_envelope();
                self.noise.clock_envelope();
            }
            _ => {}
        }
        self.frame_step = (self.frame_step + 1) & 7;
    }

    fn clock_lengths(&mut self) {
        self.sq1.clock_length();
        self.sq2.clock_length();
        self.wave.clock_length();
        self.noise.clock_length();
    }
}
