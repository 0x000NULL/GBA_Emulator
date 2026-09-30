//! APU. Owned by the APU agent (M7). This file fixes the public surface the bus depends on:
//! IO byte access for 0x060-0x0AF, FIFO writes, the timer-overflow -> FIFO -> DMA request path,
//! and the sample sink.
//!
//! M0 status: STUB. Registers are stored raw, FIFOs work (so DMA1/2 sound transfers behave),
//! output samples are silence.

use crate::sched::{Event, Scheduler};
use crate::timing::{CYCLES_PER_AUDIO_SAMPLE, CYCLES_PER_FRAME_SEQ};

/// 32-byte DirectSound FIFO.
#[derive(Clone, Debug, Default)]
pub struct Fifo {
    buf: [u8; 32],
    head: usize,
    len: usize,
    /// Current output sample (latched on timer overflow).
    pub sample: i8,
    pub underruns: u64,
}

impl Fifo {
    pub fn push(&mut self, bytes: &[u8]) {
        for &b in bytes {
            if self.len == 32 {
                // Hardware behaviour on overflow is to drop; keep the newest 32 bytes is also common.
                return;
            }
            let i = (self.head + self.len) % 32;
            self.buf[i] = b;
            self.len += 1;
        }
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn reset(&mut self) {
        self.head = 0;
        self.len = 0;
        self.sample = 0;
    }

    /// Pop one sample into `sample`. Returns true if the FIFO is at or below the refill threshold.
    pub fn pop(&mut self) -> bool {
        if self.len == 0 {
            self.underruns += 1;
        } else {
            self.sample = self.buf[self.head] as i8;
            self.head = (self.head + 1) % 32;
            self.len -= 1;
        }
        self.len <= 16
    }
}

/// Which FIFOs want a DMA refill after a timer tick.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FifoRequest {
    pub a: bool,
    pub b: bool,
}

pub struct Apu {
    /// Raw sound registers 0x060-0x0AF as written (agent replaces with typed state).
    pub io: [u8; 0x50],
    pub fifo_a: Fifo,
    pub fifo_b: Fifo,
    /// Stereo output samples at `cycles_per_sample()`; drained by the frontend/harness.
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
        let mut a = Apu { io: [0; 0x50], fifo_a: Fifo::default(), fifo_b: Fifo::default(), samples: Vec::new(), sample_count: 0 };
        // SOUNDBIAS = 0x200 after boot.
        a.io[0x28] = 0x00;
        a.io[0x29] = 0x02;
        a
    }

    #[inline]
    pub fn reg16(&self, off: usize) -> u16 {
        u16::from_le_bytes([self.io[off - 0x60], self.io[off - 0x5F]])
    }

    /// SOUNDCNT_X bit 7: master enable.
    pub fn master_enabled(&self) -> bool {
        self.io[0x84 - 0x60] & 0x80 != 0
    }

    /// Output cycles per sample from SOUNDBIAS bits 14-15 (0: 32.768 kHz ... 3: 262.144 kHz).
    pub fn cycles_per_sample(&self) -> u64 {
        let amp = (self.io[0x89 - 0x60] >> 6) & 3;
        CYCLES_PER_AUDIO_SAMPLE >> amp
    }

    pub fn boot_schedule(&mut self, sched: &mut Scheduler) {
        sched.schedule_in(Event::ApuSample, self.cycles_per_sample());
        sched.schedule_in(Event::ApuFrameSeq, CYCLES_PER_FRAME_SEQ);
    }

    pub fn read_io8(&self, off: u16) -> u8 {
        let i = off as usize;
        if !(0x60..0xB0).contains(&i) {
            return 0;
        }
        match i {
            // FIFOs are write-only; unused gaps read 0.
            0xA0..=0xAF => 0,
            _ => self.io[i - 0x60],
        }
    }

    pub fn write_io8(&mut self, off: u16, v: u8, _sched: &mut Scheduler) {
        let i = off as usize;
        if !(0x60..0xB0).contains(&i) {
            return;
        }
        match i {
            0xA0..=0xA3 => self.fifo_a.push(&[v]),
            0xA4..=0xA7 => self.fifo_b.push(&[v]),
            0x83 => {
                // SOUNDCNT_H high byte: bits 3/7 reset FIFO A/B.
                if v & 0x08 != 0 {
                    self.fifo_a.reset();
                }
                if v & 0x80 != 0 {
                    self.fifo_b.reset();
                }
                self.io[i - 0x60] = v & 0x77;
            }
            _ => self.io[i - 0x60] = v,
        }
    }

    /// 16/32-bit FIFO writes push all bytes in order (Classic NES Series writes 16 bits).
    pub fn fifo_write(&mut self, which: usize, bytes: &[u8]) {
        if which == 0 {
            self.fifo_a.push(bytes);
        } else {
            self.fifo_b.push(bytes);
        }
    }

    /// Timer `t` overflowed. Each FIFO clocked by `t` (SOUNDCNT_H bit 10 / bit 14) pops a sample;
    /// returns which FIFOs now need a DMA refill (<= 16 bytes left).
    pub fn on_timer_overflow(&mut self, t: u8) -> FifoRequest {
        let cnt_h = self.reg16(0x82);
        let mut req = FifoRequest::default();
        if t < 2 {
            if (cnt_h >> 10) & 1 == t as u16 {
                req.a = self.fifo_a.pop();
            }
            if (cnt_h >> 14) & 1 == t as u16 {
                req.b = self.fifo_b.pop();
            }
        }
        req
    }

    /// Produce one output sample. STUB: silence. The APU agent mixes PSG + FIFO A/B here.
    pub fn on_sample(&mut self) {
        self.samples.push([0, 0]);
        self.sample_count += 1;
    }

    /// 512 Hz frame sequencer tick. STUB.
    pub fn on_frame_seq(&mut self) {}
}
