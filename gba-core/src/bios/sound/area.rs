//! Layout of the sound driver's work area and its derived rate parameters.
//!
//! GBATEK "BIOS Sound Functions" documents the work area (`SoundArea`: ident, DmaCount, reverb,
//! func, intp, the channel array, the PCM buffer), the channel (`sf rv lv at de su re fr wp`) and
//! `WaveData`. Its struct listing abbreviates the reserved fields; the offsets below are the ones
//! the games actually use, confirmed black-box on 2026-10-01 by watching the five SDK titles
//! write their channels at `area+0x50+0x40*n` (status +0, rv/lv +2/+3, at/de/su/re +4..+7,
//! `fr` +0x20, `wp` +0x24) and by the m4a work area Saibara Rieko no Dendou Mahjong hands to
//! SWI 1Ch (func +0x20, intp +0x24, CgbSound +0x28, jump table +0x34). The field names follow the
//! published m4a `SoundInfo` / `SoundChannel` interface descriptions; no BIOS code was consulted.

/// `ident` of an initialised work area ("Smsh" as a little-endian word); `+1` while
/// SoundDriverMain owns it.
pub const ID_NUMBER: u32 = 0x6873_6D53;
/// GBATEK "BIOS RAM Usage": 3007FF0h holds the pointer to the sound work area.
pub const SOUND_INFO_PTR: u32 = 0x0300_7FF0;

/// Work area (m4a `SoundInfo`) field offsets.
pub mod info {
    pub const IDENT: u32 = 0x00;
    /// Frames until the DMA is restarted at the buffer start (GBATEK `DmaCount`).
    pub const DMA_COUNTER: u32 = 0x04;
    pub const REVERB: u32 = 0x05;
    pub const MAX_CHANS: u32 = 0x06;
    pub const MASTER_VOLUME: u32 = 0x07;
    /// Playback-rate index 1..=12 (SoundDriverMode bits 16-19).
    pub const FREQ: u32 = 0x08;
    pub const MODE: u32 = 0x09;
    /// 15-frame countdown kept by SoundDriverMain.
    pub const C15: u32 = 0x0A;
    /// DMA restart period in frames (`PCM_BUF_SIZE / samples per frame`).
    pub const DMA_PERIOD: u32 = 0x0B;
    pub const MAX_LINES: u32 = 0x0C;
    pub const SAMPLES_PER_VBLANK: u32 = 0x10;
    pub const PCM_FREQ: u32 = 0x14;
    pub const DIV_FREQ: u32 = 0x18;
    pub const CGB_CHANS: u32 = 0x1C;
    /// Music player entry called by SoundDriverMain with `intp` (GBATEK `func`).
    pub const FUNC: u32 = 0x20;
    pub const INTP: u32 = 0x24;
    /// Game-side CGB (PSG) channel routine, called after `func`.
    pub const CGB_SOUND: u32 = 0x28;
    pub const CGB_OSC_OFF: u32 = 0x2C;
    pub const MIDI_KEY_TO_CGB_FREQ: u32 = 0x30;
    pub const JUMP_TABLE: u32 = 0x34;
    pub const PLYNOTE: u32 = 0x38;
    pub const EXT_VOL_PIT: u32 = 0x3C;
    pub const CHANS: u32 = 0x50;
    pub const CHAN_SIZE: u32 = 0x40;
    pub const MAX_CHANNELS: u32 = 12;
    /// Two PCM halves: the first feeds FIFO A (right speaker), the second FIFO B (left).
    pub const PCM_BUFFER: u32 = 0x350;
    pub const PCM_BUF_SIZE: u32 = 0x630;
    pub const PCM_BUFFER_LEFT: u32 = PCM_BUFFER + PCM_BUF_SIZE;
    pub const SIZE: u32 = PCM_BUFFER + 2 * PCM_BUF_SIZE;
}

/// Channel (m4a `SoundChannel`, GBATEK `SoundChannel`) field offsets.
pub mod chan {
    /// GBATEK `sf`.
    pub const STATUS: u32 = 0x00;
    pub const TYPE: u32 = 0x01;
    /// GBATEK `rv` / `lv`.
    pub const RIGHT_VOLUME: u32 = 0x02;
    pub const LEFT_VOLUME: u32 = 0x03;
    /// GBATEK `at de su re`.
    pub const ATTACK: u32 = 0x04;
    pub const DECAY: u32 = 0x05;
    pub const SUSTAIN: u32 = 0x06;
    pub const RELEASE: u32 = 0x07;
    pub const KEY: u32 = 0x08;
    /// Envelope level, and its right/left products with the pan volumes (mixer-owned).
    pub const ENV_VOLUME: u32 = 0x09;
    pub const ENV_RIGHT: u32 = 0x0A;
    pub const ENV_LEFT: u32 = 0x0B;
    pub const ECHO_VOLUME: u32 = 0x0C;
    pub const ECHO_LENGTH: u32 = 0x0D;
    pub const GATE_TIME: u32 = 0x10;
    pub const MIDI_KEY: u32 = 0x11;
    pub const VELOCITY: u32 = 0x12;
    pub const PRIORITY: u32 = 0x13;
    pub const RHYTHM_PAN: u32 = 0x14;
    /// Samples left before the end (or the loop point) is reached (mixer-owned).
    pub const COUNT: u32 = 0x18;
    /// Fractional sample position, Q16 (mixer-owned).
    pub const FW: u32 = 0x1C;
    /// GBATEK `fr`: playback rate in Hz (from MidiKey2Freq).
    pub const FREQUENCY: u32 = 0x20;
    /// GBATEK `wp`: `WaveData*`.
    pub const WAV: u32 = 0x24;
    /// Byte pointer to the next sample (mixer-owned).
    pub const CURRENT: u32 = 0x28;
    pub const TRACK: u32 = 0x2C;
    pub const PREV: u32 = 0x30;
    pub const NEXT: u32 = 0x34;
}

/// Channel status bits (GBATEK: write 80h to start, OR 40h for key-off, 0 for pause; the
/// envelope stage lives in the low bits).
pub mod status {
    pub const START: u8 = 0x80;
    pub const STOP: u8 = 0x40;
    pub const LOOP: u8 = 0x10;
    /// Pseudo-echo phase after the release has decayed to zero.
    pub const IEC: u8 = 0x04;
    pub const ENV_MASK: u8 = 0x03;
    pub const ENV_ATTACK: u8 = 3;
    pub const ENV_DECAY: u8 = 2;
    pub const ENV_SUSTAIN: u8 = 1;
    pub const ENV_RELEASE: u8 = 0;
    /// A channel with none of these bits is free.
    pub const ON: u8 = START | STOP | IEC | ENV_MASK;
}

/// `WaveData` field offsets (GBATEK).
pub mod wave {
    pub const TYPE: u32 = 0x00;
    pub const STATUS: u32 = 0x02;
    /// `status` bit: forward loop.
    pub const LOOP_FLAG: u16 = 0x4000;
    pub const FREQ: u32 = 0x04;
    pub const LOOP_START: u32 = 0x08;
    pub const SIZE: u32 = 0x0C;
    pub const DATA: u32 = 0x10;
}

/// Cycles per frame (228 lines of 1232 cycles).
const CYCLES_PER_FRAME: u32 = 280_896;

/// Playback-rate parameters for SoundDriverMode bits 16-19. Everything derives from GBATEK's
/// twelve frequencies: the timer-0 period is `16777216 / Hz` rounded, which divides the frame
/// length exactly, so a whole number of samples plays per frame and the PCM ring advances one
/// slot per V-Blank.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RateParams {
    pub index: u8,
    pub hz: u32,
    /// Timer-0 period in cycles (the reload is `0x10000 - period`).
    pub period: u32,
    pub samples_per_vblank: u32,
    /// Frames per DMA restart: `PCM_BUF_SIZE / samples_per_vblank`.
    pub dma_period: u8,
}

const RATE_HZ: [u32; 12] = [5734, 7884, 10512, 13379, 15768, 18157, 21024, 26758, 31536, 36314, 40137, 42048];

impl RateParams {
    /// `index` 1..=12; None otherwise.
    pub fn for_index(index: u8) -> Option<RateParams> {
        if !(1..=12).contains(&index) {
            return None;
        }
        let hz = RATE_HZ[index as usize - 1];
        let period = (16_777_216 + hz / 2) / hz;
        let samples_per_vblank = CYCLES_PER_FRAME / period;
        let dma_period = (info::PCM_BUF_SIZE / samples_per_vblank) as u8;
        Some(RateParams { index, hz, period, samples_per_vblank, dma_period })
    }

    pub const DEFAULT_INDEX: u8 = 4;

    pub fn timer_reload(&self) -> u16 {
        (0x1_0000 - self.period) as u16
    }

    /// `divFreq` field: half the timer period (the library's value for rate 3 is 0x31E = 1596/2).
    pub fn div_freq(&self) -> u32 {
        self.period / 2
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rate_table_divides_the_frame_exactly() {
        for i in 1..=12u8 {
            let r = RateParams::for_index(i).unwrap();
            assert_eq!(CYCLES_PER_FRAME % r.period, 0, "index {i}: period {} does not divide the frame", r.period);
            assert_eq!(r.samples_per_vblank % 4, 0, "index {i}: whole words per slot");
            assert!(r.dma_period as u32 * r.samples_per_vblank <= info::PCM_BUF_SIZE);
        }
        assert!(RateParams::for_index(0).is_none());
        assert!(RateParams::for_index(13).is_none());
    }

    /// Saibara Rieko no Dendou Mahjong's own library fills its work area for rate 3 with
    /// samplesPerVBlank 0xB0, pcmFreq 0x2910, pcmDmaPeriod 9 and divFreq 0x31E (dumped
    /// 2026-10-01); the default rate reproduces the 0xFB1A timer reload the driver programs.
    #[test]
    fn derived_tables_match_the_library() {
        let r = RateParams::for_index(3).unwrap();
        assert_eq!((r.hz, r.period, r.samples_per_vblank, r.dma_period, r.div_freq()), (10512, 1596, 176, 9, 0x31E));
        assert_eq!(r.timer_reload(), 0xF9C4);
        let d = RateParams::for_index(RateParams::DEFAULT_INDEX).unwrap();
        assert_eq!((d.hz, d.samples_per_vblank, d.dma_period), (13379, 224, 7));
        assert_eq!(d.timer_reload(), 0xFB1A);
        assert_eq!(RateParams::for_index(12).unwrap().samples_per_vblank, 704);
    }

    #[test]
    fn layout_constants() {
        assert_eq!(info::CHANS + info::MAX_CHANNELS * info::CHAN_SIZE, info::PCM_BUFFER);
        assert_eq!(info::SIZE, 0xFB0);
        assert_eq!(info::PCM_BUFFER_LEFT, 0x980);
        assert_eq!(status::ON, 0xC7);
    }
}
