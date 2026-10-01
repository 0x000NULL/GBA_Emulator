//! DirectSound FIFOs A and B. Source: GBATEK "GBA Sound Channel A and B - DMA Sound".
//!
//! "Internally, the capacity of the FIFO is 8 x 32bit (32 bytes)". On every overflow of the
//! selected timer one byte moves to the sound circuit; when 16 bytes or fewer remain the sound
//! controller requests a 16-byte DMA refill.

/// Bytes at or below which a pop requests a DMA refill ("If FIFO contains only 4 x 32bits").
pub const REFILL_THRESHOLD: usize = 16;

/// 32-byte DirectSound FIFO.
#[cfg_attr(feature = "savestate", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, Default)]
pub struct Fifo {
    buf: [u8; 32],
    head: usize,
    len: usize,
    /// Current output sample (latched on timer overflow, signed 8-bit).
    pub sample: i8,
    /// Audible starvation events: the first empty pop after bytes were fed (see `pop_ex`). A FIFO
    /// that was never fed, or that stays empty after its source stopped, is silence by the game's
    /// choice and does not count.
    pub underruns: u64,
    /// Every pop of an empty FIFO regardless of cause (raw diagnostic; the last sample is held).
    #[cfg_attr(feature = "savestate", serde(skip))]
    pub empty_pops: u64,
    /// Bytes were pushed since the last empty pop / reset.
    #[cfg_attr(feature = "savestate", serde(skip))]
    live: bool,
    /// Diagnostics: bytes accepted by `push` and samples consumed by `pop`.
    pub pushed: u64,
    pub popped: u64,
}

impl Fifo {
    /// Push bytes oldest-first ("Data 0 being located in least significant byte which is
    /// replayed first"). Bytes that do not fit are dropped.
    pub fn push(&mut self, bytes: &[u8]) {
        for &b in bytes {
            if self.len == 32 {
                return;
            }
            let i = (self.head + self.len) % 32;
            self.buf[i] = b;
            self.live = true;
            self.len += 1;
            self.pushed += 1;
        }
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// SOUNDCNT_H bit 11/15: reset FIFO.
    pub fn reset(&mut self) {
        self.head = 0;
        self.len = 0;
        self.sample = 0;
        self.live = false;
    }

    /// Pop one sample into `sample`. Returns true if the FIFO is at or below the refill threshold.
    pub fn pop(&mut self) -> bool {
        self.pop_ex(true)
    }

    /// `pop`, counting an empty FIFO as an underrun only when `count_underrun` is set and the FIFO
    /// is live. The sound controller clocks a FIFO on every overflow of its timer, even when the
    /// channel is routed to neither speaker or the game never feeds it; an empty FIFO then just
    /// holds its last sample (silence nobody hears). Rule: an underrun is the first empty pop after
    /// bytes were pushed (the stream ran dry). Further empty pops while nothing is fed are not new
    /// starvation (`empty_pops` keeps the raw count).
    pub fn pop_ex(&mut self, count_underrun: bool) -> bool {
        if self.len == 0 {
            self.empty_pops += 1;
            if count_underrun && self.live {
                self.underruns += 1;
            }
            self.live = false;
        } else {
            self.sample = self.buf[self.head] as i8;
            self.head = (self.head + 1) % 32;
            self.len -= 1;
            self.popped += 1;
        }
        self.len <= REFILL_THRESHOLD
    }
}

/// Which FIFOs want a DMA refill after a timer tick.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FifoRequest {
    pub a: bool,
    pub b: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn never_fed_fifo_does_not_underrun() {
        let mut f = Fifo::default();
        for _ in 0..1000 {
            f.pop_ex(true);
        }
        assert_eq!(f.underruns, 0);
        assert_eq!(f.empty_pops, 1000);
    }

    #[test]
    fn running_dry_counts_once_until_fed_again() {
        let mut f = Fifo::default();
        f.push(&[1, 2, 3, 4]);
        for _ in 0..4 {
            f.pop_ex(true);
        }
        assert_eq!(f.underruns, 0, "all fed bytes were consumed in time");
        for _ in 0..100 {
            f.pop_ex(true);
        }
        assert_eq!(f.underruns, 1, "the stream ran dry once; the idle pops after it are silence");
        f.push(&[9]);
        f.pop_ex(true);
        f.pop_ex(true);
        assert_eq!(f.underruns, 2, "fed again, ran dry again");
        assert_eq!(f.empty_pops, 101);
    }

    #[test]
    fn unrouted_or_reset_fifo_does_not_count() {
        let mut f = Fifo::default();
        f.push(&[1]);
        f.pop_ex(false);
        f.pop_ex(false);
        assert_eq!(f.underruns, 0);
        f.push(&[1]);
        f.reset();
        f.pop_ex(true);
        assert_eq!(f.underruns, 0);
    }
}
