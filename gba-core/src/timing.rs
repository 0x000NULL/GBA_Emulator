//! Timing constants. Source: GBATEK "LCD Dimensions and Timings" and "LCD I/O Interrupts and Status".
//! mGBA uses an HDRAW of 1008; keep `HDRAW_CYCLES` a constant so it can be flipped when diffing traces.

/// System clock, Hz.
pub const CLOCK_HZ: u64 = 16_777_216;
/// Cycles per scanline (308 dots x 4).
pub const CYCLES_PER_LINE: u64 = 1232;
/// Cycles from line start until the H-Blank flag is set ("the H-Blank flag is 0 for a total of 1006 cycles").
pub const HDRAW_CYCLES: u64 = 1006;
/// Total scanlines per frame including V-Blank.
pub const LINES_PER_FRAME: u32 = 228;
/// Visible scanlines.
pub const VISIBLE_LINES: u32 = 160;
/// Cycles per frame (1232 x 228).
pub const CYCLES_PER_FRAME: u64 = CYCLES_PER_LINE * LINES_PER_FRAME as u64;
/// V-Blank flag is set in lines 160..=226 (not 227).
pub const VBLANK_FLAG_LAST_LINE: u32 = 226;
/// Video-capture DMA (DMA3 special) fires at H-Blank of lines 2..=162.
pub const VIDEO_CAPTURE_FIRST_LINE: u32 = 2;
pub const VIDEO_CAPTURE_LAST_LINE: u32 = 162;
/// Default APU output rate (SOUNDBIAS bits 14-15 = 0 => 32768 Hz), in cycles per sample.
pub const CYCLES_PER_AUDIO_SAMPLE: u64 = 512;
/// PSG frame sequencer: 512 Hz.
pub const CYCLES_PER_FRAME_SEQ: u64 = CLOCK_HZ / 512;
