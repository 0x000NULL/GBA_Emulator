//! Sound register offsets (IO offset, 0x060..0x0AF) and read-back masks.
//! Source: GBATEK register bit tables in "GBA Sound Channel 1..4", "GBA Sound Control Registers".
//! Write-only bits (sound length, frequency, initial/restart, FIFO reset) read back as 0;
//! "Not used" bits read 0.

pub const SOUND1CNT_L: usize = 0x60;
pub const SOUND1CNT_H: usize = 0x62;
pub const SOUND1CNT_X: usize = 0x64;
pub const SOUND2CNT_L: usize = 0x68;
pub const SOUND2CNT_H: usize = 0x6C;
pub const SOUND3CNT_L: usize = 0x70;
pub const SOUND3CNT_H: usize = 0x72;
pub const SOUND3CNT_X: usize = 0x74;
pub const SOUND4CNT_L: usize = 0x78;
pub const SOUND4CNT_H: usize = 0x7C;
pub const SOUNDCNT_L: usize = 0x80;
pub const SOUNDCNT_H: usize = 0x82;
pub const SOUNDCNT_X: usize = 0x84;
pub const SOUNDBIAS: usize = 0x88;
pub const WAVE_RAM: usize = 0x90;
pub const FIFO_A: usize = 0xA0;
pub const FIFO_B: usize = 0xA4;

/// First IO offset owned by the APU.
pub const BASE: usize = 0x60;
/// One past the last IO offset owned by the APU.
pub const END: usize = 0xB0;

/// Read mask for each 16-bit register, indexed by `(off - BASE) / 2`.
pub const READ_MASK16: [u16; 0x28] = [
    0x007F, // 0x60 SOUND1CNT_L: sweep shift/dir/time
    0xFFC0, // 0x62 SOUND1CNT_H: duty, envelope (length write-only)
    0x4000, // 0x64 SOUND1CNT_X: length flag only
    0x0000, // 0x66
    0xFFC0, // 0x68 SOUND2CNT_L
    0x0000, // 0x6A
    0x4000, // 0x6C SOUND2CNT_H
    0x0000, // 0x6E
    0x00E0, // 0x70 SOUND3CNT_L: dimension, bank, playback
    0xE000, // 0x72 SOUND3CNT_H: volume, force volume (length write-only)
    0x4000, // 0x74 SOUND3CNT_X
    0x0000, // 0x76
    0xFF00, // 0x78 SOUND4CNT_L: envelope (length write-only)
    0x0000, // 0x7A
    0x40FF, // 0x7C SOUND4CNT_H: ratio, width, shift, length flag
    0x0000, // 0x7E
    0xFF77, // 0x80 SOUNDCNT_L
    0x770F, // 0x82 SOUNDCNT_H (FIFO reset bits write-only)
    0x008F, // 0x84 SOUNDCNT_X (bits 0-3 status, 7 master)
    0x0000, // 0x86
    0xC3FE, // 0x88 SOUNDBIAS
    0x0000, // 0x8A
    0x0000, // 0x8C
    0x0000, // 0x8E
    0xFFFF, 0xFFFF, 0xFFFF, 0xFFFF, 0xFFFF, 0xFFFF, 0xFFFF, 0xFFFF, // 0x90-0x9F wave RAM
    0x0000, 0x0000, 0x0000, 0x0000, // 0xA0-0xA7 FIFOs (write-only)
    0x0000, 0x0000, 0x0000, 0x0000, // 0xA8-0xAF
];

/// Read mask for a single byte at IO offset `off`.
#[inline]
pub fn read_mask8(off: usize) -> u8 {
    let m = READ_MASK16[(off - BASE) >> 1];
    if off & 1 == 0 {
        m as u8
    } else {
        (m >> 8) as u8
    }
}
