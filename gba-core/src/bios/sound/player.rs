//! Layout of the music player's structures (m4a `MusicPlayerInfo`, `MusicPlayerTrack`,
//! `ToneData`) as the game's sequencer and the jump-list handlers share them.
//!
//! The field order follows the published interface descriptions of the library; the offsets were
//! pinned black-box on Saibara Rieko no Dendou Mahjong (2026-10-01) from the game's own accesses
//! (its MPlayMain reads `cmdPtr` at +0x40, `flags` at +0, `wait` at +1, `runningStatus` at +7, the
//! LFO fields at +0x16..+0x1C, `chan` at +0x20, the tone at +0x24) and from dumps of its players
//! (`trackCount` 12 at +8, `tempoD`/`tempoU`/`tempoI` 150/256/150 at +0x1C/+0x1E/+0x20, `tracks`
//! at +0x2C, `tone` at +0x30, `ident` at +0x34).

/// `MusicPlayerInfo` (0x40 bytes).
pub mod ply {
    pub const SONG_HEADER: u32 = 0x00;
    /// Bit 31 = paused; the low bits are the active-track mask.
    pub const STATUS: u32 = 0x04;
    pub const STATUS_PAUSE: u32 = 0x8000_0000;
    pub const TRACK_COUNT: u32 = 0x08;
    pub const PRIORITY: u32 = 0x09;
    /// The command byte minus 0xB1, stored by the dispatcher before the jump.
    pub const CMD: u32 = 0x0A;
    pub const CLOCK: u32 = 0x0C;
    pub const MEM_ACC_AREA: u32 = 0x18;
    pub const TEMPO_D: u32 = 0x1C;
    pub const TEMPO_U: u32 = 0x1E;
    pub const TEMPO_I: u32 = 0x20;
    pub const TEMPO_C: u32 = 0x22;
    pub const FADE_OI: u32 = 0x24;
    pub const FADE_OC: u32 = 0x26;
    pub const FADE_OV: u32 = 0x28;
    pub const TRACKS: u32 = 0x2C;
    /// `ToneData*` voice table of the song.
    pub const TONE: u32 = 0x30;
    pub const IDENT: u32 = 0x34;
    pub const FUNC: u32 = 0x38;
    pub const INTP: u32 = 0x3C;
}

/// `MusicPlayerTrack` (0x50 bytes).
pub mod trk {
    pub const FLAGS: u32 = 0x00;
    pub const WAIT: u32 = 0x01;
    pub const PATTERN_LEVEL: u32 = 0x02;
    pub const REP_N: u32 = 0x03;
    pub const GATE_TIME: u32 = 0x04;
    pub const KEY: u32 = 0x05;
    pub const VELOCITY: u32 = 0x06;
    pub const RUNNING_STATUS: u32 = 0x07;
    pub const KEY_M: u32 = 0x08;
    pub const PIT_M: u32 = 0x09;
    pub const KEY_SHIFT: u32 = 0x0A;
    pub const KEY_SHIFT_X: u32 = 0x0B;
    pub const TUNE: u32 = 0x0C;
    pub const PIT_X: u32 = 0x0D;
    pub const BEND: u32 = 0x0E;
    pub const BEND_RANGE: u32 = 0x0F;
    pub const VOL_MR: u32 = 0x10;
    pub const VOL_ML: u32 = 0x11;
    pub const VOL: u32 = 0x12;
    pub const VOL_X: u32 = 0x13;
    pub const PAN: u32 = 0x14;
    pub const PAN_X: u32 = 0x15;
    pub const MOD_M: u32 = 0x16;
    pub const MOD: u32 = 0x17;
    pub const MOD_T: u32 = 0x18;
    pub const LFO_SPEED: u32 = 0x19;
    pub const LFO_SPEED_C: u32 = 0x1A;
    pub const LFO_DELAY: u32 = 0x1B;
    pub const LFO_DELAY_C: u32 = 0x1C;
    pub const PRIORITY: u32 = 0x1D;
    pub const ECHO_VOLUME: u32 = 0x1E;
    pub const ECHO_LENGTH: u32 = 0x1F;
    /// Head of the track's channel chain (`SoundChannel*` / `CgbChannel*`).
    pub const CHAN: u32 = 0x20;
    /// The current voice, a `ToneData` copy.
    pub const TONE: u32 = 0x24;
    pub const TONE_SIZE: u32 = 12;
    pub const CMD_PTR: u32 = 0x40;
    pub const PATTERN_STACK: u32 = 0x44;
    pub const PATTERN_DEPTH: u32 = 3;
    pub const SIZE: u32 = 0x50;

    /// `flags` bits.
    pub const F_VOL_SET: u8 = 0x01;
    pub const F_VOL_CHG: u8 = 0x03;
    pub const F_PIT_SET: u8 = 0x04;
    pub const F_PIT_CHG: u8 = 0x0C;
    pub const F_START: u8 = 0x40;
    pub const F_EXIST: u8 = 0x80;
}

/// Track command bytes (the jump list covers 0xB1..=0xCE; waits are 0x80..=0xB0, TIE is 0xCF,
/// notes 0xD0..=0xFF).
pub const CMD_FIRST: u8 = 0xB1;
pub const CMD_LAST: u8 = 0xCE;
/// "Center" value of the signed one-byte arguments (pan, bend, tune).
pub const C_V: i32 = 0x40;
