//! The APU's output (32768 Hz, or the rate SOUNDBIAS selects) resampled to 48 kHz stereo into a
//! ring the app drains. Linear interpolation, stateful across frames; the same scheme as
//! `basket_app::audio::Resampler`, which this crate must not depend on.

use std::collections::VecDeque;

/// What `audio_read` produces.
pub const OUT_RATE: u32 = 48_000;
/// Unread frames kept: 200 ms. Older frames are dropped first.
pub const RING_CAP: usize = OUT_RATE as usize / 5;

pub struct Audio {
    /// Source frames advanced per output frame.
    step: f64,
    /// Position on the timeline where 0.0 is `prev` and 1.0 is the first frame of the next input.
    pos: f64,
    prev: [i16; 2],
    ring: VecDeque<[i16; 2]>,
}

impl Audio {
    pub fn new() -> Self {
        Audio { step: 32768.0 / OUT_RATE as f64, pos: 1.0, prev: [0, 0], ring: VecDeque::with_capacity(RING_CAP) }
    }

    /// Resample `input` (at `src_rate` Hz) onto the ring, dropping the oldest frames past the cap.
    pub fn push(&mut self, input: &[[i16; 2]], src_rate: f64) {
        if input.is_empty() {
            return;
        }
        self.step = src_rate / OUT_RATE as f64;
        let n = input.len() as f64;
        while self.pos < n {
            let i = self.pos as usize; // floor, pos >= 0
            let frac = self.pos - i as f64;
            let a = if i == 0 { self.prev } else { input[i - 1] };
            let b = input[i];
            if self.ring.len() == RING_CAP {
                self.ring.pop_front();
            }
            self.ring.push_back([lerp(a[0], b[0], frac), lerp(a[1], b[1], frac)]);
            self.pos += self.step;
        }
        self.pos -= n;
        self.prev = input[input.len() - 1];
    }

    /// Move up to `out.len() / 2` frames into `out` as interleaved L,R. Returns frames written.
    pub fn read(&mut self, out: &mut [i16]) -> usize {
        let n = (out.len() / 2).min(self.ring.len());
        for (dst, s) in out.as_chunks_mut::<2>().0.iter_mut().zip(self.ring.drain(..n)) {
            dst.copy_from_slice(&s);
        }
        n
    }

    pub fn len(&self) -> usize {
        self.ring.len()
    }
}

#[inline]
fn lerp(a: i16, b: i16, t: f64) -> i16 {
    (a as f64 + (b as f64 - a as f64) * t).round().clamp(i16::MIN as f64, i16::MAX as f64) as i16
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_second_at_32768_gives_48000() {
        let mut a = Audio::new();
        let chunk = vec![[100i16, -100]; 547]; // about one frame's worth
        let mut total = 0;
        let mut out = vec![0i16; 4096];
        let mut fed = 0;
        while fed + chunk.len() <= 32768 * 4 {
            a.push(&chunk, 32768.0);
            fed += chunk.len();
            total += a.read(&mut out);
        }
        let expect = fed as f64 * 48000.0 / 32768.0;
        assert!((total as f64 - expect).abs() <= 2.0, "{total} vs {expect}");
        assert_eq!(&out[..2], &[100, -100]);
    }

    #[test]
    fn rate_follows_soundbias() {
        let mut a = Audio::new();
        a.push(&vec![[0, 0]; 65536], 65536.0);
        assert!(a.len().abs_diff(RING_CAP) == 0, "one second at 64 kHz overflows the 200 ms ring");
        let mut out = vec![0i16; RING_CAP * 2];
        assert_eq!(a.read(&mut out), RING_CAP);
        a.push(&vec![[0, 0]; 6554], 65536.0); // 100 ms
        assert!(a.len().abs_diff(4800) <= 1, "{}", a.len());
    }

    #[test]
    fn ring_drops_oldest() {
        let mut a = Audio::new();
        for i in 0..20_000i32 {
            a.push(&[[i as i16, 0]], OUT_RATE as f64);
        }
        assert_eq!(a.len(), RING_CAP);
        let mut out = [0i16; 2];
        a.read(&mut out);
        assert_eq!(out[0] as i32, 20_000 - RING_CAP as i32 - 1, "the oldest kept frame (output lags input by one)");
    }
}
