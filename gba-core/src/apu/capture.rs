//! Bench capture for the APU: per-channel peak levels over a frame and per-channel mutes.
//!
//! The APU owns an `ApuCapture` behind `Apu::debug` (`None` unless the bench is attached via
//! `Bus::set_bench`). `on_sample` records the level of each channel and, for muted channels,
//! clears their routing bits in the SOUNDCNT copies handed to the mixer. The registers, FIFO
//! state and underrun accounting are never touched, so a muted channel behaves exactly like an
//! audible one to the guest. Nothing here is serialised.

pub const CH_SQ1: usize = 0;
pub const CH_SQ2: usize = 1;
pub const CH_WAVE: usize = 2;
pub const CH_NOISE: usize = 3;
pub const CH_FIFO_A: usize = 4;
pub const CH_FIFO_B: usize = 5;

/// Capture state owned by the APU while the bench is attached.
#[derive(Clone, Debug, Default)]
pub struct ApuCapture {
    /// Highest level seen since the last `take_peaks`: PSG 0..=15, FIFO 0..=128.
    pub peak: [u8; 6],
    /// Muted channels: bit `CH_*`.
    pub mute: u8,
    /// SOUNDCNT_L bits cleared before mixing (left/right enables of muted PSG channels).
    pub(super) mask_l: u16,
    /// SOUNDCNT_H bits cleared before mixing (left/right enables of muted FIFOs).
    pub(super) mask_h: u16,
}

impl ApuCapture {
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the muted channels (bit `CH_*`) and recompute the routing masks.
    pub fn set_mute(&mut self, mask: u8) {
        self.mute = mask & 0x3F;
        self.mask_l = 0;
        for ch in 0..4 {
            if self.mute & (1 << ch) != 0 {
                self.mask_l |= (1 << (8 + ch)) | (1 << (12 + ch));
            }
        }
        self.mask_h = 0;
        if self.mute & (1 << CH_FIFO_A) != 0 {
            self.mask_h |= (1 << 8) | (1 << 9);
        }
        if self.mute & (1 << CH_FIFO_B) != 0 {
            self.mask_h |= (1 << 12) | (1 << 13);
        }
    }

    /// Return the peaks recorded since the last call and reset them.
    pub(crate) fn take_peaks(&mut self) -> [u8; 6] {
        std::mem::take(&mut self.peak)
    }

    #[inline]
    pub(super) fn note(&mut self, ch: usize, level: u8) {
        if level > self.peak[ch] {
            self.peak[ch] = level;
        }
    }
}
