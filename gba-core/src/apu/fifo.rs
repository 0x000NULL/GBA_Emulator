//! DirectSound FIFOs A and B. Source: GBATEK "GBA Sound Channel A and B - DMA Sound".
//!
//! "Internally, the capacity of the FIFO is 8 x 32bit (32 bytes)". On every overflow of the
//! selected timer one byte moves to the sound circuit; when 16 bytes or fewer remain the sound
//! controller requests a 16-byte DMA refill.

/// Bytes at or below which a pop requests a DMA refill ("If FIFO contains only 4 x 32bits").
pub const REFILL_THRESHOLD: usize = 16;

/// 32-byte DirectSound FIFO.
#[derive(Clone, Debug, Default)]
pub struct Fifo {
    buf: [u8; 32],
    head: usize,
    len: usize,
    /// Current output sample (latched on timer overflow, signed 8-bit).
    pub sample: i8,
    /// Pops that found the FIFO empty (the previous sample is repeated).
    pub underruns: u64,
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
    }

    /// Pop one sample into `sample`. Returns true if the FIFO is at or below the refill threshold.
    pub fn pop(&mut self) -> bool {
        self.pop_ex(true)
    }

    /// `pop`, counting an empty FIFO as an underrun only when `count_underrun` is set. The sound
    /// controller clocks a FIFO whenever its timer overflows, even when the channel is routed to
    /// neither speaker; those silent pops of an unused FIFO are not diagnostics-worthy.
    pub fn pop_ex(&mut self, count_underrun: bool) -> bool {
        if self.len == 0 {
            if count_underrun {
                self.underruns += 1;
            }
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
