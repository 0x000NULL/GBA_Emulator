//! The 36 SoundGetJumpList entries: the music player's track-command handlers and a few helpers
//! the game's sequencer calls through the table it copied with SWI 2Ah.
//!
//! Calling contract (traced on Saibara Rieko no Dendou Mahjong, 2026-10-01): the game's
//! `MPlayMain` reads the command byte, advances `cmdPtr` past it, stores `cmd - 0xB1` in the
//! player, and calls slot `cmd - 0xB1` with r0 = player, r1 = track, in ARM state, `lr` holding
//! the Thumb return address. Handlers read their arguments at `cmdPtr`, advance it, and return
//! with `bx lr`; afterwards the dispatcher treats `flags == 0` as "track ended". Slots 30-35 are
//! helpers with their own arguments (r0 = frequency, player or channel). Argument formats and
//! semantics come from the published MusicPlayer2000 format descriptions; where a slot's
//! behaviour is not documented the handler only advances `cmdPtr` by the documented argument
//! count and logs the call (`--log-swi`).

use super::super::call::{Pending, SavedFrame};
use super::super::{HLE_JUMPLIST_BASE, HLE_JUMPLIST_SLOTS};
use super::area::{chan, info, status, RateParams, ID_NUMBER, SOUND_INFO_PTR};
use super::player::{ply, trk, C_V};
use crate::bus::{Access, Bus, MemoryBus};
use crate::cpu::regs::{Regs, FLAG_T};

pub const SLOT_NAMES: [&str; 36] = [
    "ply_fine", "ply_goto", "ply_patt", "ply_pend", "ply_rept", "ply_dummy", "ply_dummy", "ply_dummy", "ply_memacc", "ply_prio",
    "ply_tempo", "ply_keysh", "ply_voice", "ply_vol", "ply_pan", "ply_bend", "ply_bendr", "ply_lfos", "ply_lfodl", "ply_mod",
    "ply_modt", "ply_dummy", "ply_dummy", "ply_tune", "ply_dummy", "ply_dummy", "ply_dummy", "ply_port", "ply_xcmd", "ply_endtie",
    "SampleFreqSet", "TrackStop", "FadeOutBody", "TrkVolPitSet", "RealClearChain", "Clear64",
];

/// Slot of a jump-list trap address, if `pc` is one.
pub fn slot_of_trap(pc: u32) -> Option<u8> {
    if pc & 3 == 0 && (HLE_JUMPLIST_BASE..HLE_JUMPLIST_BASE + 4 * HLE_JUMPLIST_SLOTS).contains(&pc) {
        Some(((pc - HLE_JUMPLIST_BASE) / 4) as u8)
    } else {
        None
    }
}

/// `bx lr`: return to the caller in the state bit 0 of `lr` selects.
pub(crate) fn ret_to_lr(regs: &mut Regs) {
    let lr = regs.r[14];
    if lr & 1 != 0 {
        regs.set_cpsr(regs.cpsr | FLAG_T);
        regs.r[15] = lr & !1;
    } else {
        regs.set_cpsr(regs.cpsr & !FLAG_T);
        regs.r[15] = lr & !3;
    }
}

/// A channel chain never has more than the 12 PCM plus 4 CGB channels.
const CHAIN_LIMIT: usize = 16;
const RAM_START: u32 = 0x0200_0000;

fn in_ram(a: u32) -> bool {
    (RAM_START..0x0800_0000).contains(&a)
}

impl Bus {
    #[inline]
    fn g8(&mut self, a: u32) -> u8 {
        self.read8(a, Access::S)
    }
    #[inline]
    fn g16(&mut self, a: u32) -> u16 {
        self.read16(a, Access::S)
    }
    #[inline]
    fn g32(&mut self, a: u32) -> u32 {
        self.read32(a, Access::S)
    }
    /// Little-endian word at an arbitrary (unaligned) byte address in the command stream.
    fn g32_unaligned(&mut self, a: u32) -> u32 {
        let b = [self.g8(a), self.g8(a + 1), self.g8(a + 2), self.g8(a + 3)];
        u32::from_le_bytes(b)
    }
    #[inline]
    fn p8(&mut self, a: u32, v: u8) {
        self.write8(a, v, Access::S);
    }
    #[inline]
    fn p16(&mut self, a: u32, v: u16) {
        self.write16(a, v, Access::S);
    }
    #[inline]
    fn p32(&mut self, a: u32, v: u32) {
        self.write32(a, v, Access::S);
    }

    /// The sound work area whatever its lock state (ident ID..ID+10), or None.
    fn sound_area_any(&mut self) -> Option<u32> {
        let area = self.read32(SOUND_INFO_PTR, Access::N) & !3;
        if !in_ram(area) {
            return None;
        }
        let ident = self.read32(area + info::IDENT, Access::N);
        (ID_NUMBER..=ID_NUMBER + 10).contains(&ident).then_some(area)
    }

    /// Execution reached jump-list slot `slot`. Returns true (the trap is always handled).
    pub(crate) fn jumplist_call(&mut self, slot: u8, regs: &mut Regs) -> bool {
        let r0 = regs.r[0];
        let r1 = regs.r[1];
        if self.debug.log_swi {
            let cmd_ptr = if slot < 30 && in_ram(r1) { self.g32(r1 + trk::CMD_PTR) } else { 0 };
            let line = format!("jl slot={slot} {} r0={r0:08X} r1={r1:08X} cmdPtr={cmd_ptr:08X} lr={:08X}", SLOT_NAMES[slot as usize], regs.r[14]);
            self.debug.log.push(line);
        }
        let done = if slot < 30 && !in_ram(r1) {
            true
        } else {
            match slot {
                0 | 5 | 6 | 7 | 21 | 22 | 24 | 25 | 26 => self.ply_fine(regs, r0, r1),
                1 => {
                    let p = self.g32(r1 + trk::CMD_PTR);
                    let dest = self.g32_unaligned(p);
                    self.p32(r1 + trk::CMD_PTR, dest);
                    true
                }
                2 => {
                    self.ply_patt(r1);
                    true
                }
                3 => {
                    self.ply_pend(r1);
                    true
                }
                4 => {
                    self.ply_rept(r1);
                    true
                }
                8 => {
                    self.ply_memacc(r0, r1);
                    true
                }
                9 => {
                    self.ply_byte(r1, trk::PRIORITY, 0, 0);
                    true
                }
                10 => {
                    self.ply_tempo(r0, r1);
                    true
                }
                11 => {
                    self.ply_byte(r1, trk::KEY_SHIFT, 0, trk::F_PIT_CHG);
                    true
                }
                12 => {
                    self.ply_voice(r0, r1);
                    true
                }
                13 => {
                    self.ply_byte(r1, trk::VOL, 0, trk::F_VOL_CHG);
                    true
                }
                14 => {
                    self.ply_byte(r1, trk::PAN, C_V, trk::F_VOL_CHG);
                    true
                }
                15 => {
                    self.ply_byte(r1, trk::BEND, C_V, trk::F_PIT_CHG);
                    true
                }
                16 => {
                    self.ply_byte(r1, trk::BEND_RANGE, 0, trk::F_PIT_CHG);
                    true
                }
                17 => {
                    self.ply_lfo(r1, trk::LFO_SPEED);
                    true
                }
                18 => {
                    self.ply_byte(r1, trk::LFO_DELAY, 0, 0);
                    true
                }
                19 => {
                    self.ply_lfo(r1, trk::MOD);
                    true
                }
                20 => {
                    self.ply_modt(r1);
                    true
                }
                23 => {
                    self.ply_byte(r1, trk::TUNE, C_V, trk::F_PIT_CHG);
                    true
                }
                27 => {
                    // ply_port: two argument bytes, semantics undocumented.
                    let p = self.g32(r1 + trk::CMD_PTR);
                    self.p32(r1 + trk::CMD_PTR, p + 2);
                    true
                }
                28 => {
                    self.ply_xcmd(r1);
                    true
                }
                29 => {
                    self.ply_endtie(r1);
                    true
                }
                30 => {
                    self.sample_freq_set(r0);
                    true
                }
                31 => self.track_stop(regs, r1, false),
                32 => {
                    self.fade_out_body(r0);
                    true
                }
                33 => {
                    self.trk_vol_pit_set(r1);
                    true
                }
                34 => {
                    self.real_clear_chain(r0);
                    true
                }
                35 => {
                    self.clear64(r0);
                    true
                }
                _ => true,
            }
        };
        if done {
            ret_to_lr(regs);
        }
        true
    }

    /// A guest routine called by a jump-list handler returned.
    pub(crate) fn jumplist_resume(&mut self, regs: &mut Regs, saved: SavedFrame, slot: u8, stage: u8, data: [u32; 4]) {
        match (slot, stage) {
            // ply_fine delegated to the library's TrackStop: now end the track.
            (0, 1) => {
                self.p8(data[0] + trk::FLAGS, 0);
                self.hle_call_finish(regs, &saved, true);
            }
            // TrackStop: CgbOscOff returned for channel data[1]; finish it and go on.
            (31, 1) => {
                let c = data[1];
                self.p8(c + chan::STATUS, 0);
                self.p32(c + chan::TRACK, 0);
                let next = self.g32(c + chan::NEXT);
                if self.track_stop_chain(regs, Some(saved), data[0], next, data[2] != 0) {
                    self.hle_call_finish(regs, &saved, true);
                }
            }
            _ => self.hle_call_finish(regs, &saved, true),
        }
    }

    // ------------------------------------------------------------------ track commands

    /// One-byte argument into a track field (minus `bias`), raising `flags`.
    fn ply_byte(&mut self, track: u32, field: u32, bias: i32, flags: u8) {
        let p = self.g32(track + trk::CMD_PTR);
        let v = self.g8(p) as i32 - bias;
        self.p8(track + field, v as u8);
        if flags != 0 {
            let f = self.g8(track + trk::FLAGS);
            self.p8(track + trk::FLAGS, f | flags);
        }
        self.p32(track + trk::CMD_PTR, p + 1);
    }

    /// PATT: push the return address and jump to the pattern (three levels of nesting).
    fn ply_patt(&mut self, track: u32) {
        let p = self.g32(track + trk::CMD_PTR);
        let level = self.g8(track + trk::PATTERN_LEVEL) as u32;
        if level < trk::PATTERN_DEPTH {
            self.p32(track + trk::PATTERN_STACK + level * 4, p + 4);
            self.p8(track + trk::PATTERN_LEVEL, level as u8 + 1);
            let dest = self.g32_unaligned(p);
            self.p32(track + trk::CMD_PTR, dest);
        } else {
            self.p32(track + trk::CMD_PTR, p + 4);
        }
    }

    /// PEND: return from the pattern.
    fn ply_pend(&mut self, track: u32) {
        let level = self.g8(track + trk::PATTERN_LEVEL);
        if level > 0 {
            let level = level - 1;
            self.p8(track + trk::PATTERN_LEVEL, level);
            let ret = self.g32(track + trk::PATTERN_STACK + level as u32 * 4);
            self.p32(track + trk::CMD_PTR, ret);
        }
    }

    /// REPT n, dest: jump back to `dest` until the section has played `n` times.
    fn ply_rept(&mut self, track: u32) {
        let p = self.g32(track + trk::CMD_PTR);
        let n = self.g8(p);
        let dest = self.g32_unaligned(p + 1);
        let count = self.g8(track + trk::REP_N) + 1;
        if count < n {
            self.p8(track + trk::REP_N, count);
            self.p32(track + trk::CMD_PTR, dest);
        } else {
            self.p8(track + trk::REP_N, 0);
            self.p32(track + trk::CMD_PTR, p + 5);
        }
    }

    /// MEMACC op, adr, dat [, dest]: byte arithmetic on the player's memory-access area and
    /// conditional jumps on it.
    fn ply_memacc(&mut self, player: u32, track: u32) {
        let p = self.g32(track + trk::CMD_PTR);
        let op = self.g8(p);
        let adr = self.g8(p + 1) as u32;
        let dat = self.g8(p + 2);
        let mem = self.g32(player + ply::MEM_ACC_AREA);
        if !in_ram(mem) {
            self.p32(track + trk::CMD_PTR, p + if (6..18).contains(&op) { 7 } else { 3 });
            return;
        }
        let cur = self.g8(mem + adr);
        let operand = if (3..6).contains(&op) || (12..18).contains(&op) { self.g8(mem + dat as u32) } else { dat };
        match op {
            0 | 3 => self.p8(mem + adr, operand),
            1 | 4 => self.p8(mem + adr, cur.wrapping_add(operand)),
            2 | 5 => self.p8(mem + adr, cur.wrapping_sub(operand)),
            6..=17 => {
                let taken = match (op - 6) % 6 {
                    0 => cur == operand,
                    1 => cur != operand,
                    2 => cur > operand,
                    3 => cur >= operand,
                    4 => cur <= operand,
                    _ => cur < operand,
                };
                let next = if taken { self.g32_unaligned(p + 3) } else { p + 7 };
                self.p32(track + trk::CMD_PTR, next);
                return;
            }
            _ => {}
        }
        self.p32(track + trk::CMD_PTR, p + 3);
    }

    /// TEMPO: the argument is half the BPM; `tempoI` is the per-frame tick increment scaled by
    /// the player's tempo multiplier (`tempoU`, 256 = 1.0), matching the observed defaults
    /// 150 / 256 / 150.
    fn ply_tempo(&mut self, player: u32, track: u32) {
        let p = self.g32(track + trk::CMD_PTR);
        let tempo_d = (self.g8(p) as u16) * 2;
        let tempo_u = self.g16(player + ply::TEMPO_U) as u32;
        let tempo_i = ((tempo_d as u32 * tempo_u) >> 8) as u16;
        self.p16(player + ply::TEMPO_D, tempo_d);
        self.p16(player + ply::TEMPO_I, tempo_i);
        self.p32(track + trk::CMD_PTR, p + 1);
    }

    /// VOICE n: copy the voice's `ToneData` into the track.
    fn ply_voice(&mut self, player: u32, track: u32) {
        let p = self.g32(track + trk::CMD_PTR);
        let n = self.g8(p) as u32;
        let table = self.g32(player + ply::TONE);
        if table >= RAM_START {
            let src = table + n * trk::TONE_SIZE;
            for i in 0..3 {
                let w = self.g32_unaligned(src + i * 4);
                self.p32(track + trk::TONE + i * 4, w);
            }
        }
        self.p32(track + trk::CMD_PTR, p + 1);
    }

    /// LFOS / MOD: a zero argument switches the modulation off and schedules a recompute of
    /// whichever quantity it was modulating.
    fn ply_lfo(&mut self, track: u32, field: u32) {
        let p = self.g32(track + trk::CMD_PTR);
        let v = self.g8(p);
        self.p8(track + field, v);
        if v == 0 {
            self.p8(track + trk::MOD_M, 0);
            let mod_t = self.g8(track + trk::MOD_T);
            let f = self.g8(track + trk::FLAGS);
            self.p8(track + trk::FLAGS, f | if mod_t == 0 { trk::F_PIT_CHG } else { trk::F_VOL_CHG });
        }
        self.p32(track + trk::CMD_PTR, p + 1);
    }

    /// MODT: modulation target (0 pitch, 1 volume, 2 pan); a change recomputes both.
    fn ply_modt(&mut self, track: u32) {
        let p = self.g32(track + trk::CMD_PTR);
        let v = self.g8(p);
        if v != self.g8(track + trk::MOD_T) {
            self.p8(track + trk::MOD_T, v);
            let f = self.g8(track + trk::FLAGS);
            self.p8(track + trk::FLAGS, f | trk::F_VOL_CHG | trk::F_PIT_CHG);
        }
        self.p32(track + trk::CMD_PTR, p + 1);
    }

    /// XCMD sub, value: pseudo-echo volume (8) and length (9).
    fn ply_xcmd(&mut self, track: u32) {
        let p = self.g32(track + trk::CMD_PTR);
        let sub = self.g8(p);
        let v = self.g8(p + 1);
        match sub {
            8 => self.p8(track + trk::ECHO_VOLUME, v),
            9 => self.p8(track + trk::ECHO_LENGTH, v),
            _ => {}
        }
        self.p32(track + trk::CMD_PTR, p + 2);
    }

    /// EOT [key]: key-off the track's channels playing `key` (the last key when omitted).
    fn ply_endtie(&mut self, track: u32) {
        let p = self.g32(track + trk::CMD_PTR);
        let arg = self.g8(p);
        let key = if arg < 0x80 {
            self.p32(track + trk::CMD_PTR, p + 1);
            arg
        } else {
            self.g8(track + trk::KEY)
        };
        let mut c = self.g32(track + trk::CHAN);
        let mut guard = 0;
        while in_ram(c) && guard < CHAIN_LIMIT {
            guard += 1;
            let st = self.g8(c + chan::STATUS);
            if st & status::ON != 0 && st & status::STOP == 0 && self.g8(c + chan::MIDI_KEY) == key {
                self.p8(c + chan::STATUS, st | status::STOP);
            }
            c = self.g32(c + chan::NEXT);
        }
    }

    /// FINE (and the undocumented dummy slots): stop the track's channels and end the track.
    /// Stopping is delegated to whatever TrackStop the table holds (the library installs its
    /// own, which also silences CGB channels) when it is guest code; otherwise ours runs.
    fn ply_fine(&mut self, regs: &mut Regs, player: u32, track: u32) -> bool {
        let track_stop = self.sound_area_any().map(|area| self.g32(area + info::JUMP_TABLE)).filter(|&jt| in_ram(jt)).map(|jt| self.g32(jt + 31 * 4));
        if let Some(ts) = track_stop.filter(|&ts| ts >= RAM_START) {
            let saved = SavedFrame::from_bx_lr(regs);
            self.hle_call_guest(regs, saved, ts, &[player, track], Pending::JumpList { slot: 0, stage: 1, data: [track, 0, 0, 0] });
            return false;
        }
        self.track_stop(regs, track, true)
    }

    // ------------------------------------------------------------------ helpers (slots 30-35)

    /// SampleFreqSet: the playback rate as SoundDriverMode bits 16-19 (in place or shifted down).
    fn sample_freq_set(&mut self, freq: u32) {
        let index = if freq > 0xF { (freq >> 16) & 0xF } else { freq & 0xF };
        let Some(rate) = RateParams::for_index(index as u8) else { return };
        if let Some(area) = self.sound_area_any() {
            self.sound_set_rate(area, &rate);
        }
    }

    /// TrackStop: silence and release every channel of the track. CGB channels go through the
    /// game's `CgbOscOff` (a guest call), so the walk may suspend; `fine` also ends the track.
    fn track_stop(&mut self, regs: &mut Regs, track: u32, fine: bool) -> bool {
        if self.g8(track + trk::FLAGS) & trk::F_EXIST == 0 {
            if fine {
                self.p8(track + trk::FLAGS, 0);
            }
            return true;
        }
        let head = self.g32(track + trk::CHAN);
        self.track_stop_chain(regs, None, track, head, fine)
    }

    fn track_stop_chain(&mut self, regs: &mut Regs, saved: Option<SavedFrame>, track: u32, mut c: u32, fine: bool) -> bool {
        let osc_off = self.sound_area_any().map(|area| self.g32(area + info::CGB_OSC_OFF)).unwrap_or(0);
        let mut guard = 0;
        while in_ram(c) && guard < CHAIN_LIMIT {
            guard += 1;
            let st = self.g8(c + chan::STATUS);
            if st != 0 {
                let ty = self.g8(c + chan::TYPE);
                if ty & 7 != 0 && osc_off >= RAM_START {
                    let saved = saved.unwrap_or_else(|| SavedFrame::from_bx_lr(regs));
                    let pending = Pending::JumpList { slot: 31, stage: 1, data: [track, c, fine as u32, 0] };
                    self.hle_call_guest(regs, saved, osc_off, &[(ty & 7) as u32], pending);
                    return false;
                }
                self.p8(c + chan::STATUS, 0);
            }
            self.p32(c + chan::TRACK, 0);
            c = self.g32(c + chan::NEXT);
        }
        self.p32(track + trk::CHAN, 0);
        if fine {
            self.p8(track + trk::FLAGS, 0);
        }
        true
    }

    /// FadeOutBody: every `fadeOI` frames lower the fade level by one step, apply it as the
    /// tracks' volume multiplier, and pause the player when it reaches zero. Only the fade-out
    /// direction is documented; it is all the fallback implements.
    fn fade_out_body(&mut self, player: u32) {
        let interval = self.g16(player + ply::FADE_OI);
        if interval == 0 {
            return;
        }
        let counter = self.g16(player + ply::FADE_OC).wrapping_sub(1);
        self.p16(player + ply::FADE_OC, counter);
        if counter != 0 {
            return;
        }
        self.p16(player + ply::FADE_OC, interval);
        let level = self.g16(player + ply::FADE_OV).saturating_sub(16);
        self.p16(player + ply::FADE_OV, level);
        let tracks = self.g32(player + ply::TRACKS);
        let n = self.g8(player + ply::TRACK_COUNT) as u32;
        if !in_ram(tracks) {
            return;
        }
        for i in 0..n.min(16) {
            let t = tracks + i * trk::SIZE;
            let f = self.g8(t + trk::FLAGS);
            if f & trk::F_EXIST == 0 {
                continue;
            }
            if level == 0 {
                let head = self.g32(t + trk::CHAN);
                let mut c = head;
                let mut guard = 0;
                while in_ram(c) && guard < CHAIN_LIMIT {
                    guard += 1;
                    self.p8(c + chan::STATUS, 0);
                    self.p32(c + chan::TRACK, 0);
                    c = self.g32(c + chan::NEXT);
                }
                self.p32(t + trk::CHAN, 0);
            } else {
                self.p8(t + trk::VOL_X, (level >> 2) as u8);
                self.p8(t + trk::FLAGS, f | trk::F_VOL_CHG);
            }
        }
        if level == 0 {
            let st = self.g32(player + ply::STATUS);
            self.p32(player + ply::STATUS, st | ply::STATUS_PAUSE);
            self.p16(player + ply::FADE_OI, 0);
        }
    }

    /// TrkVolPitSet: fold the track's volume, expression, pan and the pitch controls into the
    /// per-track multipliers the note code consumes. The exact scaling is a design choice (the
    /// library installs its own copy in every title seen).
    fn trk_vol_pit_set(&mut self, track: u32) {
        let flags = self.g8(track + trk::FLAGS);
        if flags & trk::F_VOL_CHG != 0 {
            let vol = self.g8(track + trk::VOL) as i32;
            let vol_x = self.g8(track + trk::VOL_X) as i32;
            let mut x = (vol * vol_x) >> 5;
            let mod_t = self.g8(track + trk::MOD_T);
            let mod_m = self.g8(track + trk::MOD_M) as i8 as i32;
            if mod_t == 1 {
                x = (x + ((x * mod_m) >> 7)).clamp(0, 255);
            }
            let mut pan = self.g8(track + trk::PAN) as i8 as i32 + self.g8(track + trk::PAN_X) as i8 as i32;
            if mod_t == 2 {
                pan += mod_m;
            }
            let pan = pan.clamp(-64, 63) + 64;
            self.p8(track + trk::VOL_MR, ((x * pan) >> 7).clamp(0, 255) as u8);
            self.p8(track + trk::VOL_ML, ((x * (127 - pan)) >> 7).clamp(0, 255) as u8);
        }
        if flags & trk::F_PIT_CHG != 0 {
            let bend = self.g8(track + trk::BEND) as i8 as i32 * self.g8(track + trk::BEND_RANGE) as i32;
            let tune = self.g8(track + trk::TUNE) as i8 as i32;
            let mut fine = (bend + tune) * 4 + self.g8(track + trk::PIT_X) as i8 as i32;
            if self.g8(track + trk::MOD_T) == 0 {
                fine += (self.g8(track + trk::MOD_M) as i8 as i32) * 4;
            }
            let key = self.g8(track + trk::KEY_SHIFT) as i8 as i32 + self.g8(track + trk::KEY_SHIFT_X) as i8 as i32 + (fine >> 8);
            self.p8(track + trk::KEY_M, key as u8);
            self.p8(track + trk::PIT_M, (fine & 0xFF) as u8);
        }
    }

    /// RealClearChain: unlink a finished channel from its track's chain. The predecessor is
    /// found by walking from the head rather than trusting `prev`, so either linking discipline
    /// of the game's note code works; `prev` of the successor is kept consistent anyway.
    fn real_clear_chain(&mut self, c: u32) {
        if !in_ram(c) {
            return;
        }
        let track = self.g32(c + chan::TRACK);
        if !in_ram(track) {
            return;
        }
        let next = self.g32(c + chan::NEXT);
        let prev = self.g32(c + chan::PREV);
        let head = self.g32(track + trk::CHAN);
        if head == c {
            self.p32(track + trk::CHAN, next);
        } else {
            let mut p = head;
            let mut guard = 0;
            while in_ram(p) && guard < CHAIN_LIMIT {
                guard += 1;
                let n = self.g32(p + chan::NEXT);
                if n == c {
                    self.p32(p + chan::NEXT, next);
                    break;
                }
                p = n;
            }
        }
        if in_ram(next) {
            self.p32(next + chan::PREV, if head == c { 0 } else { prev });
        }
        self.p32(c + chan::TRACK, 0);
        self.p32(c + chan::NEXT, 0);
        self.p32(c + chan::PREV, 0);
    }

    /// Zero a 64-byte structure (a player at open, a track at start; its `cmdPtr` and pattern
    /// stack past the first 0x40 bytes are untouched).
    fn clear64(&mut self, p: u32) {
        if !in_ram(p) {
            return;
        }
        for i in 0..16 {
            self.p32(p + i * 4, 0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bios::swi;
    use crate::bios::test_util::{bus, peek, poke, swi};
    use crate::bios::{HLE_CALL_RETURN_ADDR, HLE_JUMPLIST_BASE};
    use crate::cpu::regs::MODE_SYS;

    const INFO: u32 = 0x0300_6DD0;
    const TRACK: u32 = 0x0300_0FA8;
    const CMDS: u32 = 0x0200_0100;
    const TONES: u32 = 0x0200_0200;
    const AREA: u32 = 0x0300_5C90;
    const LR: u32 = 0x0803_B5AB;

    /// Call slot `slot` as the game's dispatcher does (ARM, r0 = player, r1 = track, Thumb lr).
    fn call(b: &mut Bus, slot: u32, r0: u32, r1: u32) -> Regs {
        let mut regs = Regs::new();
        regs.set_cpsr(MODE_SYS);
        regs.r[0] = r0;
        regs.r[1] = r1;
        regs.r[4] = 0x44;
        regs.r[14] = LR;
        regs.r[15] = HLE_JUMPLIST_BASE + slot * 4;
        assert!(b.hle_vector(&mut regs));
        regs
    }

    fn returned(regs: &Regs) {
        assert_eq!(regs.r[15], LR & !1);
        assert!(regs.thumb());
        assert_eq!(regs.r[4], 0x44);
    }

    fn track_with(b: &mut Bus, cmds: &[u8]) {
        poke(b, CMDS, cmds);
        b.write8(TRACK + trk::FLAGS, trk::F_EXIST, Access::N);
        b.write32(TRACK + trk::CMD_PTR, CMDS + 1, Access::N); // past the command byte
    }

    #[test]
    fn slot_addresses_round_trip() {
        assert_eq!(slot_of_trap(HLE_JUMPLIST_BASE), Some(0));
        assert_eq!(slot_of_trap(HLE_JUMPLIST_BASE + 35 * 4), Some(35));
        assert_eq!(slot_of_trap(HLE_JUMPLIST_BASE + 36 * 4), None);
        assert_eq!(slot_of_trap(HLE_JUMPLIST_BASE + 2), None);
        assert_eq!(slot_of_trap(0x150), None);
    }

    #[test]
    fn goto_patt_pend_and_rept_move_the_command_pointer() {
        let mut b = bus();
        // GOTO 0x08001234
        track_with(&mut b, &[0xB2, 0x34, 0x12, 0x00, 0x08]);
        let regs = call(&mut b, 1, INFO, TRACK);
        returned(&regs);
        assert_eq!(b.peek32(TRACK + trk::CMD_PTR), 0x0800_1234);
        // PATT three deep, then a fourth is skipped.
        for level in 0..3u32 {
            poke(&mut b, CMDS, &[0xB3, 0x00, 0x20 + level as u8, 0x00, 0x08]);
            b.write32(TRACK + trk::CMD_PTR, CMDS + 1, Access::N);
            call(&mut b, 2, INFO, TRACK);
            assert_eq!(b.peek8(TRACK + trk::PATTERN_LEVEL), level as u8 + 1);
            assert_eq!(b.peek32(TRACK + trk::CMD_PTR), 0x0800_2000 + (level << 8));
            assert_eq!(b.peek32(TRACK + trk::PATTERN_STACK + level * 4), CMDS + 5);
        }
        b.write32(TRACK + trk::CMD_PTR, CMDS + 1, Access::N);
        call(&mut b, 2, INFO, TRACK);
        assert_eq!(b.peek8(TRACK + trk::PATTERN_LEVEL), 3);
        assert_eq!(b.peek32(TRACK + trk::CMD_PTR), CMDS + 5, "overflowing PATT is skipped");
        // PEND pops in order; underflow is a no-op.
        for level in (0..3u8).rev() {
            b.write32(TRACK + trk::CMD_PTR, 0, Access::N);
            call(&mut b, 3, INFO, TRACK);
            assert_eq!(b.peek8(TRACK + trk::PATTERN_LEVEL), level);
            assert_eq!(b.peek32(TRACK + trk::CMD_PTR), CMDS + 5);
        }
        b.write32(TRACK + trk::CMD_PTR, 0x0800_0010, Access::N);
        call(&mut b, 3, INFO, TRACK);
        assert_eq!(b.peek32(TRACK + trk::CMD_PTR), 0x0800_0010);
        // REPT 3, dest: jumps back twice, then falls through with the count reset.
        track_with(&mut b, &[0xB5, 0x03, 0x00, 0x30, 0x00, 0x08]);
        for n in 1..3u8 {
            b.write32(TRACK + trk::CMD_PTR, CMDS + 1, Access::N);
            call(&mut b, 4, INFO, TRACK);
            assert_eq!(b.peek8(TRACK + trk::REP_N), n);
            assert_eq!(b.peek32(TRACK + trk::CMD_PTR), 0x0800_3000);
        }
        b.write32(TRACK + trk::CMD_PTR, CMDS + 1, Access::N);
        call(&mut b, 4, INFO, TRACK);
        assert_eq!(b.peek8(TRACK + trk::REP_N), 0);
        assert_eq!(b.peek32(TRACK + trk::CMD_PTR), CMDS + 6);
    }

    #[test]
    fn one_byte_commands_set_fields_and_change_flags() {
        let mut b = bus();
        let cases: [(u32, u8, u32, u8, u8); 8] = [
            // slot, arg, field, stored, flags raised
            (9, 0x28, trk::PRIORITY, 0x28, 0),
            (11, 0xF4, trk::KEY_SHIFT, 0xF4, trk::F_PIT_CHG),
            (13, 0x50, trk::VOL, 0x50, trk::F_VOL_CHG),
            (14, 0x40 + 10, trk::PAN, 10, trk::F_VOL_CHG),
            (15, 0x40 - 3, trk::BEND, 0xFD, trk::F_PIT_CHG),
            (16, 12, trk::BEND_RANGE, 12, trk::F_PIT_CHG),
            (18, 7, trk::LFO_DELAY, 7, 0),
            (23, 0x40 + 1, trk::TUNE, 1, trk::F_PIT_CHG),
        ];
        for (slot, arg, field, stored, flags) in cases {
            track_with(&mut b, &[0xB1 + slot as u8, arg]);
            let before = peek(&b, TRACK, trk::SIZE as usize);
            let regs = call(&mut b, slot, INFO, TRACK);
            returned(&regs);
            let after = peek(&b, TRACK, trk::SIZE as usize);
            assert_eq!(after[field as usize], stored, "slot {slot}");
            assert_eq!(after[0], trk::F_EXIST | flags, "slot {slot} flags");
            assert_eq!(b.peek32(TRACK + trk::CMD_PTR), CMDS + 2, "slot {slot} advanced one byte");
            for i in 1..0x40 {
                if i != field as usize {
                    assert_eq!(after[i], before[i], "slot {slot} touched byte {i:#x}");
                }
            }
        }
    }

    #[test]
    fn tempo_voice_lfo_modt_xcmd() {
        let mut b = bus();
        b.write16(INFO + ply::TEMPO_U, 0x100, Access::N);
        track_with(&mut b, &[0xBB, 75]);
        call(&mut b, 10, INFO, TRACK);
        assert_eq!(b.peek16(INFO + ply::TEMPO_D), 150);
        assert_eq!(b.peek16(INFO + ply::TEMPO_I), 150);
        b.write16(INFO + ply::TEMPO_U, 0x80, Access::N);
        b.write32(TRACK + trk::CMD_PTR, CMDS + 1, Access::N);
        call(&mut b, 10, INFO, TRACK);
        assert_eq!(b.peek16(INFO + ply::TEMPO_I), 75, "half-speed multiplier");
        // VOICE 2 copies 12 bytes of the tone table entry.
        let tone: Vec<u8> = (0..36).collect();
        poke(&mut b, TONES, &tone);
        b.write32(INFO + ply::TONE, TONES, Access::N);
        track_with(&mut b, &[0xBD, 2]);
        call(&mut b, 12, INFO, TRACK);
        assert_eq!(peek(&b, TRACK + trk::TONE, 12), &tone[24..36]);
        assert_eq!(b.peek8(TRACK + trk::FLAGS), trk::F_EXIST, "voice raises no flag");
        // LFOS 0 with pitch modulation pending schedules a pitch recompute.
        b.write8(TRACK + trk::MOD_M, 5, Access::N);
        track_with(&mut b, &[0xC2, 0]);
        call(&mut b, 17, INFO, TRACK);
        assert_eq!(b.peek8(TRACK + trk::LFO_SPEED), 0);
        assert_eq!(b.peek8(TRACK + trk::MOD_M), 0);
        assert_eq!(b.peek8(TRACK + trk::FLAGS), trk::F_EXIST | trk::F_PIT_CHG);
        // MOD 20 just stores.
        track_with(&mut b, &[0xC4, 20]);
        call(&mut b, 19, INFO, TRACK);
        assert_eq!(b.peek8(TRACK + trk::MOD), 20);
        assert_eq!(b.peek8(TRACK + trk::FLAGS), trk::F_EXIST);
        // MODT 1 (volume): both change flags; repeating it changes nothing.
        track_with(&mut b, &[0xC5, 1]);
        call(&mut b, 20, INFO, TRACK);
        assert_eq!(b.peek8(TRACK + trk::MOD_T), 1);
        assert_eq!(b.peek8(TRACK + trk::FLAGS), trk::F_EXIST | trk::F_VOL_CHG | trk::F_PIT_CHG);
        track_with(&mut b, &[0xC5, 1]);
        call(&mut b, 20, INFO, TRACK);
        assert_eq!(b.peek8(TRACK + trk::FLAGS), trk::F_EXIST);
        // XCMD 8/9: pseudo-echo volume and length; two bytes consumed.
        track_with(&mut b, &[0xCD, 8, 33]);
        call(&mut b, 28, INFO, TRACK);
        assert_eq!(b.peek8(TRACK + trk::ECHO_VOLUME), 33);
        assert_eq!(b.peek32(TRACK + trk::CMD_PTR), CMDS + 3);
        track_with(&mut b, &[0xCD, 9, 4]);
        call(&mut b, 28, INFO, TRACK);
        assert_eq!(b.peek8(TRACK + trk::ECHO_LENGTH), 4);
        // PORT: undocumented, two bytes skipped.
        track_with(&mut b, &[0xCC, 1, 2]);
        call(&mut b, 27, INFO, TRACK);
        assert_eq!(b.peek32(TRACK + trk::CMD_PTR), CMDS + 3);
    }

    #[test]
    fn memacc_arithmetic_and_branches() {
        let mut b = bus();
        const MEM: u32 = 0x0300_6E90;
        b.write32(INFO + ply::MEM_ACC_AREA, MEM, Access::N);
        // op 0: mem[2] = 5
        track_with(&mut b, &[0xB9, 0, 2, 5]);
        call(&mut b, 8, INFO, TRACK);
        assert_eq!(b.peek8(MEM + 2), 5);
        assert_eq!(b.peek32(TRACK + trk::CMD_PTR), CMDS + 4);
        // op 1: mem[2] += 3
        track_with(&mut b, &[0xB9, 1, 2, 3]);
        call(&mut b, 8, INFO, TRACK);
        assert_eq!(b.peek8(MEM + 2), 8);
        // op 4: mem[2] += mem[0] (mem[0] = 10)
        b.write8(MEM, 10, Access::N);
        track_with(&mut b, &[0xB9, 4, 2, 0]);
        call(&mut b, 8, INFO, TRACK);
        assert_eq!(b.peek8(MEM + 2), 18);
        // op 6 (==): taken -> dest; op 7 (!=): not taken -> skip 7 bytes.
        track_with(&mut b, &[0xB9, 6, 2, 18, 0x00, 0x40, 0x00, 0x08]);
        call(&mut b, 8, INFO, TRACK);
        assert_eq!(b.peek32(TRACK + trk::CMD_PTR), 0x0800_4000);
        track_with(&mut b, &[0xB9, 7, 2, 18, 0x00, 0x40, 0x00, 0x08]);
        call(&mut b, 8, INFO, TRACK);
        assert_eq!(b.peek32(TRACK + trk::CMD_PTR), CMDS + 8);
        // op 14 (>, memory operand): mem[2]=18 > mem[0]=10 -> taken.
        track_with(&mut b, &[0xB9, 14, 2, 0, 0x00, 0x50, 0x00, 0x08]);
        call(&mut b, 8, INFO, TRACK);
        assert_eq!(b.peek32(TRACK + trk::CMD_PTR), 0x0800_5000);
    }

    /// Three channels chained from the track at CH0 -> CH1 -> CH2.
    fn chain(b: &mut Bus) -> [u32; 3] {
        let ch = [AREA + info::CHANS, AREA + info::CHANS + 0x40, AREA + info::CHANS + 0x80];
        b.write32(TRACK + trk::CHAN, ch[0], Access::N);
        for (i, &c) in ch.iter().enumerate() {
            b.write8(c + chan::STATUS, status::ENV_SUSTAIN, Access::N);
            b.write8(c + chan::MIDI_KEY, 60 + i as u8, Access::N);
            b.write32(c + chan::TRACK, TRACK, Access::N);
            b.write32(c + chan::PREV, if i > 0 { ch[i - 1] } else { 0 }, Access::N);
            b.write32(c + chan::NEXT, if i < 2 { ch[i + 1] } else { 0 }, Access::N);
        }
        ch
    }

    #[test]
    fn endtie_keys_off_the_matching_channels() {
        let mut b = bus();
        let ch = chain(&mut b);
        b.write8(ch[2] + chan::MIDI_KEY, 60, Access::N);
        b.write8(ch[2] + chan::STATUS, status::ENV_SUSTAIN | status::STOP, Access::N); // already released
        b.write8(TRACK + trk::KEY, 60, Access::N);
        // Explicit key argument.
        track_with(&mut b, &[0xCE, 61]);
        call(&mut b, 29, INFO, TRACK);
        assert_eq!(b.peek8(ch[1] + chan::STATUS), status::ENV_SUSTAIN | status::STOP);
        assert_eq!(b.peek8(ch[0] + chan::STATUS), status::ENV_SUSTAIN);
        assert_eq!(b.peek32(TRACK + trk::CMD_PTR), CMDS + 2);
        // No argument (next byte >= 0x80): the track's last key.
        track_with(&mut b, &[0xCE, 0x90]);
        call(&mut b, 29, INFO, TRACK);
        assert_eq!(b.peek8(ch[0] + chan::STATUS), status::ENV_SUSTAIN | status::STOP);
        assert_eq!(b.peek8(ch[2] + chan::STATUS), status::ENV_SUSTAIN | status::STOP, "unchanged");
        assert_eq!(b.peek32(TRACK + trk::CMD_PTR), CMDS + 1, "nothing consumed");
    }

    #[test]
    fn real_clear_chain_unlinks_head_middle_and_tail() {
        let mut b = bus();
        let ch = chain(&mut b);
        // Middle.
        let regs = call(&mut b, 34, ch[1], 0);
        returned(&regs);
        assert_eq!(b.peek32(ch[0] + chan::NEXT), ch[2]);
        assert_eq!(b.peek32(ch[2] + chan::PREV), ch[0]);
        assert_eq!(b.peek32(ch[1] + chan::TRACK), 0);
        assert_eq!(b.peek32(TRACK + trk::CHAN), ch[0]);
        // Head.
        call(&mut b, 34, ch[0], 0);
        assert_eq!(b.peek32(TRACK + trk::CHAN), ch[2]);
        assert_eq!(b.peek32(ch[2] + chan::PREV), 0);
        // Tail (now the only one).
        call(&mut b, 34, ch[2], 0);
        assert_eq!(b.peek32(TRACK + trk::CHAN), 0);
        // A channel with no track is ignored.
        call(&mut b, 34, ch[2], 0);
        // Singly-linked discipline (no prev maintained) also works: rebuild without prev.
        let ch = chain(&mut b);
        for &c in &ch {
            b.write32(c + chan::PREV, 0, Access::N);
        }
        call(&mut b, 34, ch[1], 0);
        assert_eq!(b.peek32(ch[0] + chan::NEXT), ch[2]);
        assert_eq!(b.peek32(TRACK + trk::CHAN), ch[0]);
    }

    #[test]
    fn clear64_zeroes_the_struct_but_not_the_command_pointer() {
        let mut b = bus();
        poke(&mut b, TRACK, &[0xAA; 0x50]);
        let regs = call(&mut b, 35, TRACK, 0);
        returned(&regs);
        assert_eq!(peek(&b, TRACK, 0x40), vec![0; 0x40]);
        assert_eq!(peek(&b, TRACK + 0x40, 0x10), vec![0xAA; 0x10]);
    }

    #[test]
    fn fine_with_bios_track_stop_silences_the_chain_and_ends_the_track() {
        let mut b = bus();
        // Work area with our own jump table (slot 31 is a trap address).
        b.write32(SOUND_INFO_PTR, AREA, Access::N);
        b.write32(AREA, ID_NUMBER, Access::N);
        b.write32(AREA + info::JUMP_TABLE, 0x0300_6C40, Access::N);
        swi(&mut b, swi::SOUND_GET_JUMP_LIST, [0x0300_6C40, 0, 0, 0]);
        let ch = chain(&mut b);
        track_with(&mut b, &[0xB1]);
        let regs = call(&mut b, 0, INFO, TRACK);
        returned(&regs);
        assert!(b.bios.calls.is_empty());
        assert_eq!(b.peek8(TRACK + trk::FLAGS), 0, "track ended");
        assert_eq!(b.peek32(TRACK + trk::CHAN), 0);
        for &c in &ch {
            assert_eq!(b.peek8(c + chan::STATUS), 0);
            assert_eq!(b.peek32(c + chan::TRACK), 0);
        }
        assert_eq!(b.peek32(TRACK + trk::CMD_PTR), CMDS + 1, "cmdPtr untouched");
    }

    #[test]
    fn fine_delegates_to_the_librarys_track_stop_then_ends_the_track() {
        let mut b = bus();
        b.write32(SOUND_INFO_PTR, AREA, Access::N);
        b.write32(AREA, ID_NUMBER, Access::N);
        b.write32(AREA + info::JUMP_TABLE, 0x0300_6C40, Access::N);
        swi(&mut b, swi::SOUND_GET_JUMP_LIST, [0x0300_6C40, 0, 0, 0]);
        b.write32(0x0300_6C40 + 31 * 4, 0x0803_B711, Access::N); // the game's TrackStop
        let ch = chain(&mut b);
        track_with(&mut b, &[0xB1]);
        let mut regs = call(&mut b, 0, INFO, TRACK);
        assert_eq!(regs.r[15], 0x0803_B710, "into the game's TrackStop");
        assert!(regs.thumb());
        assert_eq!((regs.r[0], regs.r[1]), (INFO, TRACK));
        assert_eq!(regs.r[14], HLE_CALL_RETURN_ADDR);
        assert_eq!(b.bios.calls.len(), 1);
        assert_eq!(b.peek8(TRACK + trk::FLAGS), trk::F_EXIST, "not ended yet");
        // The game's routine returns.
        regs.r[0] = 0xDEAD;
        regs.r[15] = HLE_CALL_RETURN_ADDR;
        regs.set_cpsr(regs.cpsr & !FLAG_T);
        assert!(b.hle_vector(&mut regs));
        assert!(b.bios.calls.is_empty());
        assert_eq!(b.peek8(TRACK + trk::FLAGS), 0);
        assert_eq!(regs.r[15], LR & !1);
        assert!(regs.thumb());
        assert_eq!(regs.r[0], INFO, "dispatcher's registers restored");
        assert_eq!(b.peek8(ch[0] + chan::STATUS), status::ENV_SUSTAIN, "channels were the library's business");
    }

    #[test]
    fn track_stop_calls_cgb_osc_off_for_psg_channels_and_resumes() {
        let mut b = bus();
        b.write32(SOUND_INFO_PTR, AREA, Access::N);
        b.write32(AREA, ID_NUMBER, Access::N);
        b.write32(AREA + info::CGB_OSC_OFF, 0x0803_C4DD, Access::N);
        let ch = chain(&mut b);
        b.write8(ch[1] + chan::TYPE, 2, Access::N); // CGB channel 2
        b.write8(TRACK + trk::FLAGS, trk::F_EXIST, Access::N);
        let mut regs = call(&mut b, 31, INFO, TRACK);
        assert_eq!(regs.r[15], 0x0803_C4DC);
        assert_eq!(regs.r[0], 2, "oscillator number");
        assert_eq!(b.peek8(ch[0] + chan::STATUS), 0, "PCM channel before it already stopped");
        assert_eq!(b.peek8(ch[1] + chan::STATUS), status::ENV_SUSTAIN, "waiting for CgbOscOff");
        regs.r[15] = HLE_CALL_RETURN_ADDR;
        assert!(b.hle_vector(&mut regs));
        assert!(b.bios.calls.is_empty());
        assert_eq!(b.peek8(ch[1] + chan::STATUS), 0);
        assert_eq!(b.peek8(ch[2] + chan::STATUS), 0);
        assert_eq!(b.peek32(TRACK + trk::CHAN), 0);
        assert_eq!(b.peek8(TRACK + trk::FLAGS), trk::F_EXIST, "TrackStop alone keeps the track");
        returned(&regs);
    }

    #[test]
    fn sample_freq_set_reprograms_the_rate() {
        let mut b = bus();
        swi(&mut b, swi::SOUND_DRIVER_INIT, [AREA, 0, 0, 0]);
        call(&mut b, 30, 0x0003_0000, 0);
        assert_eq!(b.peek32(AREA + info::PCM_FREQ), 10512);
        assert_eq!(b.peek8(AREA + info::DMA_PERIOD), 9);
        assert_eq!(b.timers.t[0].reload, 0xF9C4);
        call(&mut b, 30, 7, 0);
        assert_eq!(b.peek32(AREA + info::PCM_FREQ), 21024);
    }

    #[test]
    fn fade_out_lowers_expression_then_pauses() {
        let mut b = bus();
        b.write32(INFO + ply::TRACKS, TRACK, Access::N);
        b.write8(INFO + ply::TRACK_COUNT, 1, Access::N);
        b.write8(TRACK + trk::FLAGS, trk::F_EXIST, Access::N);
        b.write16(INFO + ply::FADE_OI, 2, Access::N);
        b.write16(INFO + ply::FADE_OC, 2, Access::N);
        b.write16(INFO + ply::FADE_OV, 32, Access::N);
        call(&mut b, 32, INFO, 0);
        assert_eq!(b.peek16(INFO + ply::FADE_OC), 1);
        assert_eq!(b.peek16(INFO + ply::FADE_OV), 32, "not yet");
        call(&mut b, 32, INFO, 0);
        assert_eq!(b.peek16(INFO + ply::FADE_OV), 16);
        assert_eq!(b.peek8(TRACK + trk::VOL_X), 4);
        assert_eq!(b.peek8(TRACK + trk::FLAGS), trk::F_EXIST | trk::F_VOL_CHG);
        call(&mut b, 32, INFO, 0);
        call(&mut b, 32, INFO, 0);
        assert_eq!(b.peek16(INFO + ply::FADE_OV), 0);
        assert_eq!(b.peek32(INFO + ply::STATUS) & ply::STATUS_PAUSE, ply::STATUS_PAUSE);
        assert_eq!(b.peek16(INFO + ply::FADE_OI), 0);
    }

    #[test]
    fn trk_vol_pit_set_folds_volume_pan_and_pitch() {
        let mut b = bus();
        b.write8(TRACK + trk::FLAGS, trk::F_EXIST | trk::F_VOL_CHG | trk::F_PIT_CHG, Access::N);
        b.write8(TRACK + trk::VOL, 100, Access::N);
        b.write8(TRACK + trk::VOL_X, 0x40, Access::N);
        b.write8(TRACK + trk::PAN, 0, Access::N);
        b.write8(TRACK + trk::KEY_SHIFT, 2, Access::N);
        b.write8(TRACK + trk::BEND, 32, Access::N);
        b.write8(TRACK + trk::BEND_RANGE, 2, Access::N);
        b.write8(TRACK + trk::TUNE, 0, Access::N);
        let regs = call(&mut b, 33, INFO, TRACK);
        returned(&regs);
        let x = (100 * 0x40) >> 5; // 200
        assert_eq!(b.peek8(TRACK + trk::VOL_MR), ((x * 64) >> 7) as u8);
        assert_eq!(b.peek8(TRACK + trk::VOL_ML), ((x * 63) >> 7) as u8);
        // bend 32/64 of a 2-semitone range = +1 semitone: 256 fine units.
        assert_eq!(b.peek8(TRACK + trk::KEY_M), 3);
        assert_eq!(b.peek8(TRACK + trk::PIT_M), 0);
        assert_eq!(b.peek8(TRACK + trk::FLAGS), trk::F_EXIST | trk::F_VOL_CHG | trk::F_PIT_CHG, "flags are the dispatcher's to clear");
        // Hard-right pan.
        b.write8(TRACK + trk::PAN, 63, Access::N);
        call(&mut b, 33, INFO, TRACK);
        assert_eq!(b.peek8(TRACK + trk::VOL_MR), ((x * 127) >> 7) as u8);
        assert_eq!(b.peek8(TRACK + trk::VOL_ML), 0);
    }

    #[test]
    fn calls_with_a_bad_track_pointer_just_return() {
        let mut b = bus();
        let regs = call(&mut b, 13, INFO, 0);
        returned(&regs);
        let regs = call(&mut b, 34, 0, 0);
        returned(&regs);
    }
}
