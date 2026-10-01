//! The PCM mixer of SoundDriverMain: one frame of samples for every active channel into the slot
//! of the ring buffer the FIFO DMA will fetch next.
//!
//! Behaviour from GBATEK "BIOS Sound Functions": a channel starts when `sf` = 80h is written
//! (its volume rises from zero by `at` every 1/60 s until 255, then is multiplied by `de`/256
//! per frame down to `su`, where it holds); OR-ing 40h releases it (multiplied by `re`/256 per
//! frame until zero, then the channel stops); `fr` is the playback rate in Hz as computed by
//! MidiKey2Freq; `WaveData` is signed 8-bit with an optional forward loop. The output scaling,
//! the reverb mix and the pseudo-echo phase are design choices of this implementation and are
//! documented as such in PROGRESS.md.

use super::area::{chan, info, status, wave};
use crate::bus::{Access, Bus, MemoryBus};

/// Largest `samples per V-Blank` of the rate table (42048 Hz).
const MAX_SPV: usize = 704;
/// Channel `type` bit: fixed-rate voice, played without resampling.
pub const TYPE_FIXED: u8 = 0x08;
/// Estimated cost of the real mixer per sample and active channel, and per output sample.
const MIX_CYCLES_PER_CHANNEL_SAMPLE: u32 = 6;
const MIX_CYCLES_PER_OUTPUT_SAMPLE: u32 = 3;

/// Index of the ring slot SoundDriverMain writes when the V-Blank counter reads `cnt` (1..=period
/// after a VSync): the DMA is fetching slot `period - cnt`, so the next one is written.
pub fn slot_for(cnt: u8, period: u8) -> u32 {
    let period = period.max(1) as u32;
    let cnt = (cnt as u32).clamp(1, period);
    (period - cnt + 1) % period
}

/// One envelope step (once per frame). Returns the new (status, level); a status of 0 means the
/// channel has stopped. `echo` is (pseudo-echo volume, pseudo-echo length).
pub fn envelope_step(st: u8, ev: u8, adsr: [u8; 4], echo: (u8, u8)) -> (u8, u8, u8) {
    let [at, de, su, re] = adsr;
    let (mut st, mut ev, mut echo_len) = (st, ev as u32, echo.1);
    if st & status::IEC != 0 {
        echo_len = echo_len.saturating_sub(1);
        if echo_len == 0 {
            st = 0;
        }
    } else if st & status::STOP != 0 {
        // Release: the STOP bit stays set so the channel still reads as in use.
        ev = (ev * re as u32) >> 8;
        if ev == 0 {
            if echo_len != 0 {
                st = (st & status::LOOP) | status::IEC;
                ev = echo.0 as u32;
            } else {
                st = 0;
            }
        }
    } else {
        match st & status::ENV_MASK {
            status::ENV_ATTACK => {
                ev += at as u32;
                if ev >= 255 {
                    ev = 255;
                    st = (st & !status::ENV_MASK) | status::ENV_DECAY;
                }
            }
            status::ENV_DECAY => {
                ev = (ev * de as u32) >> 8;
                if ev <= su as u32 {
                    ev = su as u32;
                    st = (st & !status::ENV_MASK) | status::ENV_SUSTAIN;
                    if ev == 0 {
                        st = 0;
                    }
                }
            }
            status::ENV_SUSTAIN => ev = su as u32,
            // Envelope stage 0 without STOP: not a state the mixer produces; treat as stopped.
            _ => st = 0,
        }
    }
    (st, ev as u8, echo_len)
}

/// Q16 sample-position step: the channel's rate over the output rate.
pub fn pitch_step_q16(frequency: u32, pcm_freq: u32) -> u32 {
    if pcm_freq == 0 {
        return 0;
    }
    (((frequency as u64) << 16) / pcm_freq as u64).min(u32::MAX as u64) as u32
}

impl Bus {
    /// Mix one frame into the ring slot the DMA will fetch next. `area` is the initialised (and
    /// locked) work area.
    pub(crate) fn sound_mix_frame(&mut self, area: u32) {
        let spv = self.read32(area + info::SAMPLES_PER_VBLANK, Access::N);
        let pcm_freq = self.read32(area + info::PCM_FREQ, Access::N);
        let period = self.read8(area + info::DMA_PERIOD, Access::N);
        if spv == 0 || spv as usize > MAX_SPV || spv % 4 != 0 || pcm_freq == 0 || period == 0 {
            return;
        }
        let spv = spv as usize;
        let cnt = self.read8(area + info::DMA_COUNTER, Access::N);
        let slot = slot_for(cnt, period);
        let max_chans = (self.read8(area + info::MAX_CHANS, Access::N) as u32).min(info::MAX_CHANNELS);
        let master = (self.read8(area + info::MASTER_VOLUME, Access::N) & 0xF) as i32;
        let reverb = (self.read8(area + info::REVERB, Access::N) & 0x7F) as i32;
        let right_base = area + info::PCM_BUFFER + slot * spv as u32;
        let left_base = area + info::PCM_BUFFER_LEFT + slot * spv as u32;

        let mut right = [0i32; MAX_SPV];
        let mut left = [0i32; MAX_SPV];
        for c in 0..max_chans {
            let cb = area + info::CHANS + c * info::CHAN_SIZE;
            let mut st = self.read8(cb + chan::STATUS, Access::N);
            if st & status::ON == 0 {
                continue;
            }
            let wav = self.read32(cb + chan::WAV, Access::N);
            let mut ev = self.read8(cb + chan::ENV_VOLUME, Access::N);
            let (mut count, mut cp, mut fw);
            if st & status::START != 0 {
                if wav < 0x0200_0000 {
                    self.write8(cb + chan::STATUS, 0, Access::S);
                    continue;
                }
                let wstatus = self.read16(wav + wave::STATUS, Access::N);
                count = self.read32(wav + wave::SIZE, Access::N);
                cp = wav + wave::DATA;
                fw = 0u32;
                ev = 0;
                st = status::ENV_ATTACK | if wstatus & wave::LOOP_FLAG != 0 { status::LOOP } else { 0 };
                if count == 0 {
                    self.write8(cb + chan::STATUS, 0, Access::S);
                    continue;
                }
            } else {
                count = self.read32(cb + chan::COUNT, Access::N);
                cp = self.read32(cb + chan::CURRENT, Access::N);
                fw = self.read32(cb + chan::FW, Access::N);
            }
            // Key-off (GBATEK "logical OR 40h"): enter the release stage. STOP stays set while the
            // level decays, which is what keeps the channel in the "in use" mask.
            if st & status::STOP != 0 {
                if st & status::IEC != 0 {
                    st &= !status::STOP;
                } else {
                    st = (st & (status::LOOP | status::STOP)) | status::ENV_RELEASE;
                }
            }
            let adsr = [
                self.read8(cb + chan::ATTACK, Access::N),
                self.read8(cb + chan::DECAY, Access::N),
                self.read8(cb + chan::SUSTAIN, Access::N),
                self.read8(cb + chan::RELEASE, Access::N),
            ];
            let echo = (self.read8(cb + chan::ECHO_VOLUME, Access::N), self.read8(cb + chan::ECHO_LENGTH, Access::N));
            let (new_st, new_ev, echo_len) = envelope_step(st, ev, adsr, echo);
            st = new_st;
            ev = new_ev;
            if echo_len != echo.1 {
                self.write8(cb + chan::ECHO_LENGTH, echo_len, Access::S);
            }
            if st == 0 {
                self.write8(cb + chan::STATUS, 0, Access::S);
                continue;
            }
            let rv = self.read8(cb + chan::RIGHT_VOLUME, Access::N) as i32;
            let lv = self.read8(cb + chan::LEFT_VOLUME, Access::N) as i32;
            let er = (ev as i32 * rv) >> 8;
            let el = (ev as i32 * lv) >> 8;
            self.write8(cb + chan::ENV_VOLUME, ev, Access::S);
            self.write8(cb + chan::ENV_RIGHT, er as u8, Access::S);
            self.write8(cb + chan::ENV_LEFT, el as u8, Access::S);

            // A fixed-rate voice (`type` bit 3) is copied one source sample per output sample:
            // its data was prepared at the driver's output rate (Pocket Professor pairs 22 kHz
            // samples with the 21024 Hz mode and never writes `fr`; Rampage writes an `fr` that
            // is meaningless as a rate). Anything else is resampled from `fr` Hz.
            let fixed = self.read8(cb + chan::TYPE, Access::N) & TYPE_FIXED != 0;
            let step = if fixed {
                1u64 << 16
            } else {
                let frequency = self.read32(cb + chan::FREQUENCY, Access::N);
                pitch_step_q16(frequency, pcm_freq) as u64
            };
            let (loop_start, size) = if st & status::LOOP != 0 {
                (self.read32(wav + wave::LOOP_START, Access::N), self.read32(wav + wave::SIZE, Access::N))
            } else {
                (0, 0)
            };
            for i in 0..spv {
                let s = self.read8(cp, Access::S) as i8 as i32;
                right[i] += s * er;
                left[i] += s * el;
                let pos = fw as u64 + step;
                let adv = (pos >> 16) as u32;
                fw = (pos & 0xFFFF) as u32;
                if adv != 0 {
                    if adv < count {
                        cp = cp.wrapping_add(adv);
                        count -= adv;
                    } else if st & status::LOOP != 0 {
                        let len = size.saturating_sub(loop_start);
                        if len == 0 {
                            st = 0;
                            break;
                        }
                        let over = (adv - count) % len;
                        cp = wav + wave::DATA + loop_start + over;
                        count = len - over;
                    } else {
                        st = 0;
                        break;
                    }
                }
                if i % 32 == 31 {
                    self.idle(32 * MIX_CYCLES_PER_CHANNEL_SAMPLE);
                    self.hle_service();
                }
            }
            self.write32(cb + chan::COUNT, count, Access::S);
            self.write32(cb + chan::CURRENT, cp, Access::S);
            self.write32(cb + chan::FW, fw, Access::S);
            self.write8(cb + chan::STATUS, st, Access::S);
        }

        // Output: master volume scaling, reverb from the samples this slot held `period` frames
        // ago, clamp to the signed 8-bit FIFO format, four samples per word.
        for i in (0..spv).step_by(4) {
            let mut rw = 0u32;
            let mut lw = 0u32;
            for k in 0..4 {
                let (r_old, l_old) = if reverb != 0 {
                    (
                        self.read8(right_base + (i + k) as u32, Access::S) as i8 as i32,
                        self.read8(left_base + (i + k) as u32, Access::S) as i8 as i32,
                    )
                } else {
                    (0, 0)
                };
                let echo = ((r_old + l_old) * reverb) >> 8;
                let r = (((right[i + k] * master) >> 12) + echo).clamp(-128, 127) as i8 as u8;
                let l = (((left[i + k] * master) >> 12) + echo).clamp(-128, 127) as i8 as u8;
                rw |= (r as u32) << (8 * k);
                lw |= (l as u32) << (8 * k);
            }
            self.write32(right_base + i as u32, rw, Access::S);
            self.write32(left_base + i as u32, lw, Access::S);
            if i % 32 == 28 {
                self.idle(32 * MIX_CYCLES_PER_OUTPUT_SAMPLE);
                self.hle_service();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::ID_NUMBER;
    use super::*;
    use crate::bios::swi;
    use crate::bios::test_util::{bus, poke, swi};

    const AREA: u32 = 0x0300_2810;
    const WAVE: u32 = 0x0200_1000;

    fn chan_base(n: u32) -> u32 {
        AREA + info::CHANS + n * info::CHAN_SIZE
    }

    /// Init at AREA; a WaveData at WAVE with `data`, `loop_start` (None = one-shot) and the
    /// given original rate.
    fn setup(b: &mut Bus, data: &[i8], loop_start: Option<u32>, hz: u32) {
        swi(b, swi::SOUND_DRIVER_INIT, [AREA, 0, 0, 0]);
        let mut w = Vec::new();
        w.extend_from_slice(&0u16.to_le_bytes());
        w.extend_from_slice(&if loop_start.is_some() { wave::LOOP_FLAG } else { 0 }.to_le_bytes());
        w.extend_from_slice(&hz.to_le_bytes());
        w.extend_from_slice(&loop_start.unwrap_or(0).to_le_bytes());
        w.extend_from_slice(&(data.len() as u32).to_le_bytes());
        w.extend(data.iter().map(|&s| s as u8));
        w.push(if let Some(l) = loop_start { data[l as usize] as u8 } else { 0 });
        poke(b, WAVE, &w);
    }

    /// Start channel `n` playing WAVE at `fr` with the given volumes and envelope.
    fn start(b: &mut Bus, n: u32, fr: u32, rv: u8, lv: u8, adsr: [u8; 4]) {
        let cb = chan_base(n);
        poke(b, cb + chan::RIGHT_VOLUME, &[rv, lv]);
        poke(b, cb + chan::ATTACK, &adsr);
        b.write32(cb + chan::FREQUENCY, fr, Access::N);
        b.write32(cb + chan::WAV, WAVE, Access::N);
        b.write8(cb + chan::STATUS, status::START, Access::N);
    }

    fn main(b: &mut Bus) {
        swi(b, swi::SOUND_DRIVER_MAIN, [0, 0, 0, 0]);
    }

    fn slot_bytes(b: &Bus, slot: u32, left: bool, n: usize) -> Vec<i8> {
        let base = AREA + if left { info::PCM_BUFFER_LEFT } else { info::PCM_BUFFER } + slot * 224;
        (0..n).map(|i| b.peek8(base + i as u32) as i8).collect()
    }

    #[test]
    fn slot_formula_follows_the_vsync_counter() {
        // After a restart the counter reads the period and the DMA fetches slot 0: write slot 1.
        assert_eq!(slot_for(9, 9), 1);
        assert_eq!(slot_for(8, 9), 2);
        assert_eq!(slot_for(2, 9), 8);
        // Counter 1: the DMA is on the last slot and the next VSync restarts into slot 0.
        assert_eq!(slot_for(1, 9), 0);
        assert_eq!(slot_for(7, 7), 1);
        assert_eq!(slot_for(0, 7), 0, "out-of-range counts clamp");
        assert_eq!(slot_for(200, 7), 1);
    }

    #[test]
    fn envelope_attack_decay_sustain_release_stages() {
        let adsr = [100, 128, 60, 128];
        let mut st = status::ENV_ATTACK;
        let mut ev = 0u8;
        let mut levels = Vec::new();
        for _ in 0..3 {
            let (s, e, _) = envelope_step(st, ev, adsr, (0, 0));
            st = s;
            ev = e;
            levels.push(ev);
        }
        assert_eq!(levels, vec![100, 200, 255]);
        assert_eq!(st & status::ENV_MASK, status::ENV_DECAY);
        let (s, e, _) = envelope_step(st, ev, adsr, (0, 0));
        assert_eq!((s & status::ENV_MASK, e), (status::ENV_DECAY, 127));
        let (s, e, _) = envelope_step(s, e, adsr, (0, 0));
        assert_eq!((s & status::ENV_MASK, e), (status::ENV_DECAY, 63));
        let (s, e, _) = envelope_step(s, e, adsr, (0, 0));
        assert_eq!((s & status::ENV_MASK, e), (status::ENV_SUSTAIN, 60), "clamped to the sustain level");
        let (s, e, _) = envelope_step(s, e, adsr, (0, 0));
        assert_eq!((s & status::ENV_MASK, e), (status::ENV_SUSTAIN, 60));
        // Key-off: release halves per frame until zero, then the channel stops.
        let mut s = (s & status::LOOP) | status::STOP;
        let mut e = e;
        let mut frames = 0;
        while s != 0 {
            let (ns, ne, _) = envelope_step(s, e, adsr, (0, 0));
            s = ns;
            e = ne;
            frames += 1;
            assert!(frames < 20);
        }
        assert_eq!(frames, 6);
        // Pseudo-echo: after the release reaches zero the channel keeps sounding at the echo
        // volume for the echo length, then stops.
        let (s, e, len) = envelope_step(status::STOP, 1, adsr, (40, 3));
        assert_eq!((s, e, len), (status::IEC, 40, 3));
        let (s, _, len) = envelope_step(s, e, adsr, (40, len));
        assert_eq!((s, len), (status::IEC, 2));
        let (s, _, len) = envelope_step(s, e, adsr, (40, len));
        let (s, _, _) = envelope_step(s, e, adsr, (40, len));
        assert_eq!(s, 0);
        // The loop flag survives the stages.
        let (s, _, _) = envelope_step(status::LOOP | status::ENV_SUSTAIN, 10, adsr, (0, 0));
        assert_eq!(s, status::LOOP | status::ENV_SUSTAIN);
    }

    #[test]
    fn pitch_step_is_the_rate_ratio_in_q16() {
        assert_eq!(pitch_step_q16(13379, 13379), 0x1_0000);
        assert_eq!(pitch_step_q16(26758, 13379), 0x2_0000);
        assert_eq!(pitch_step_q16(6689, 13379), 32765);
        assert_eq!(pitch_step_q16(100, 0), 0);
    }

    #[test]
    fn one_shot_wave_mixes_hand_computed_bytes_then_stops() {
        let mut b = bus();
        let data: Vec<i8> = (0..8).map(|i| (i * 16 - 60) as i8).collect(); // -60, -44, ..., 52
        setup(&mut b, &data, None, 13379);
        // Instant attack to 255, full right, half left, play at the original rate.
        start(&mut b, 0, 13379, 255, 128, [255, 0, 0, 0]);
        main(&mut b);
        let cb = chan_base(0);
        assert_eq!(b.peek8(cb + chan::STATUS), 0, "eight samples fit in one frame: the channel ended");
        assert_eq!(b.peek8(cb + chan::ENV_VOLUME), 255);
        assert_eq!(b.peek8(cb + chan::ENV_RIGHT), 254, "255 * 255 >> 8");
        assert_eq!(b.peek8(cb + chan::ENV_LEFT), 127);
        // After Init the counter is the period (7) and the DMA is on slot 0: Main writes slot 1.
        let right = slot_bytes(&b, 1, false, 12);
        let left = slot_bytes(&b, 1, true, 12);
        let expect_r: Vec<i8> = data.iter().map(|&s| ((s as i32 * 254 * 15) >> 12) as i8).collect();
        let expect_l: Vec<i8> = data.iter().map(|&s| ((s as i32 * 127 * 15) >> 12) as i8).collect();
        assert_eq!(&right[..8], &expect_r[..]);
        assert_eq!(&left[..8], &expect_l[..]);
        assert_eq!(&right[8..], &[0, 0, 0, 0], "silence after the end");
        assert_eq!(slot_bytes(&b, 0, false, 4), vec![0, 0, 0, 0], "slot 0 untouched");
        assert_eq!(slot_bytes(&b, 2, false, 4), vec![0, 0, 0, 0], "slot 2 untouched");
        // The next frame writes slot 2 (counter 6) and nothing is active.
        swi(&mut b, swi::SOUND_DRIVER_VSYNC, [0, 0, 0, 0]);
        b.write32(AREA + info::PCM_BUFFER + 2 * 224, 0x0101_0101, Access::N);
        main(&mut b);
        assert_eq!(slot_bytes(&b, 2, false, 4), vec![0, 0, 0, 0], "stale slot content is overwritten");
    }

    #[test]
    fn looped_wave_wraps_at_loop_start_and_keeps_playing() {
        let mut b = bus();
        let data: Vec<i8> = vec![10, 20, 30, 40, 50, 60];
        setup(&mut b, &data, Some(2), 13379); // loop over 30,40,50,60
        start(&mut b, 3, 13379, 255, 255, [255, 255, 255, 0]);
        main(&mut b);
        let cb = chan_base(3);
        assert_eq!(b.peek8(cb + chan::STATUS), status::LOOP | status::ENV_DECAY);
        let right = slot_bytes(&b, 1, false, 224);
        let scale = |s: i8| ((s as i32 * 254 * 15) >> 12) as i8;
        let mut expect: Vec<i8> = data.iter().map(|&s| scale(s)).collect();
        while expect.len() < 224 {
            let i = (expect.len() - 2) % 4 + 2;
            expect.push(scale(data[i]));
        }
        assert_eq!(right, expect);
        // Position bookkeeping: 224 samples from the start = 6 + 218 more, 218 % 4 = 2 -> data[4].
        assert_eq!(b.peek32(cb + chan::CURRENT), WAVE + wave::DATA + 4);
        assert_eq!(b.peek32(cb + chan::COUNT), 2);
        // Next frame continues seamlessly.
        swi(&mut b, swi::SOUND_DRIVER_VSYNC, [0, 0, 0, 0]);
        main(&mut b);
        let next = slot_bytes(&b, 2, false, 4);
        assert_eq!(next, vec![scale(50), scale(60), scale(30), scale(40)]);
    }

    #[test]
    fn pitch_step_half_rate_doubles_duration() {
        let mut b = bus();
        let data: Vec<i8> = (0..100).map(|i| (i as i8) - 50).collect();
        setup(&mut b, &data, None, 13379);
        start(&mut b, 0, 6690, 255, 255, [255, 0, 0, 0]);
        main(&mut b);
        let right = slot_bytes(&b, 1, false, 224);
        let scale = |s: i8| ((s as i32 * 254 * 15) >> 12) as i8;
        // Each source sample is held for two output samples (step just over 0.5).
        assert_eq!(&right[..6], &[scale(-50), scale(-50), scale(-49), scale(-49), scale(-48), scale(-48)]);
        assert_eq!(right[199], scale(49));
        assert_eq!(right[200], 0, "100 samples at half rate end after 200 outputs");
        assert_eq!(b.peek8(chan_base(0) + chan::STATUS), 0);
        // Double rate: every other sample.
        let mut b = bus();
        setup(&mut b, &data, None, 13379);
        start(&mut b, 0, 13379 * 2, 255, 255, [255, 0, 0, 0]);
        main(&mut b);
        let right = slot_bytes(&b, 1, false, 4);
        assert_eq!(right, vec![scale(-50), scale(-48), scale(-46), scale(-44)]);
    }

    /// A `type` 8 channel plays its data at the output rate whatever `fr` says (Pocket
    /// Professor leaves it 0, Rampage writes 946921).
    #[test]
    fn fixed_rate_voice_ignores_fr() {
        for fr in [0u32, 946_921, 26758] {
            let mut b = bus();
            let data: Vec<i8> = (0..100).map(|i| (i as i8) - 50).collect();
            setup(&mut b, &data, None, 0x0123_4567);
            start(&mut b, 0, fr, 255, 255, [255, 0, 0, 0]);
            b.write8(chan_base(0) + chan::TYPE, TYPE_FIXED, Access::N);
            main(&mut b);
            let right = slot_bytes(&b, 1, false, 4);
            let scale = |s: i8| ((s as i32 * 254 * 15) >> 12) as i8;
            assert_eq!(right, vec![scale(-50), scale(-49), scale(-48), scale(-47)], "fr={fr}");
            assert_eq!(b.peek8(chan_base(0) + chan::STATUS), 0, "100 samples end within the frame");
        }
    }

    #[test]
    fn key_off_enters_release_and_pause_stops_at_once() {
        let mut b = bus();
        let data: Vec<i8> = vec![100; 2000];
        setup(&mut b, &data, Some(0), 13379);
        start(&mut b, 1, 13379, 200, 200, [255, 255, 255, 128]);
        main(&mut b);
        let cb = chan_base(1);
        assert_eq!(b.peek8(cb + chan::STATUS) & status::ENV_MASK, status::ENV_DECAY);
        main(&mut b);
        assert_eq!(b.peek8(cb + chan::STATUS) & status::ENV_MASK, status::ENV_SUSTAIN, "decay 255/256 of 255 is below sustain 255 -> sustain");
        // Key-off: GBATEK "logical OR 40h".
        let st = b.peek8(cb + chan::STATUS);
        b.write8(cb + chan::STATUS, st | status::STOP, Access::N);
        main(&mut b);
        assert_eq!(b.peek8(cb + chan::STATUS), status::LOOP | status::STOP, "release entered, STOP kept");
        assert_eq!(b.peek8(cb + chan::ENV_VOLUME), 127);
        let mut frames = 0;
        while b.peek8(cb + chan::STATUS) != 0 {
            main(&mut b);
            frames += 1;
            assert!(frames < 20);
        }
        assert_eq!(frames, 7);
        // Pause: writing zero stops the channel without mixing it.
        start(&mut b, 2, 13379, 200, 200, [255, 255, 255, 128]);
        main(&mut b);
        b.write8(chan_base(2) + chan::STATUS, 0, Access::N);
        swi(&mut b, swi::SOUND_DRIVER_VSYNC, [0, 0, 0, 0]);
        main(&mut b);
        assert_eq!(b.peek8(chan_base(2) + chan::STATUS), 0);
        let slot = slot_for(b.peek8(AREA + info::DMA_COUNTER), 7);
        assert_eq!(slot_bytes(&b, slot, false, 4), vec![0, 0, 0, 0]);
    }

    #[test]
    fn channels_beyond_max_chans_and_free_channels_are_skipped() {
        let mut b = bus();
        let data: Vec<i8> = vec![100; 300];
        setup(&mut b, &data, None, 13379);
        swi(&mut b, swi::SOUND_DRIVER_MODE, [0x0000_0200, 0, 0, 0]); // 2 channels
        start(&mut b, 0, 13379, 255, 255, [255, 0, 0, 0]);
        start(&mut b, 5, 13379, 255, 255, [255, 0, 0, 0]);
        main(&mut b);
        assert_eq!(b.peek8(chan_base(5) + chan::STATUS), status::START, "channel 5 not serviced with maxChans 2");
        let scale = |s: i8| ((s as i32 * 254 * 15) >> 12) as i8;
        assert_eq!(slot_bytes(&b, 1, false, 2), vec![scale(100); 2], "one channel's worth, not two");
        // A channel whose wave pointer is null is dropped.
        b.write32(chan_base(1) + chan::WAV, 0, Access::N);
        b.write8(chan_base(1) + chan::STATUS, status::START, Access::N);
        main(&mut b);
        assert_eq!(b.peek8(chan_base(1) + chan::STATUS), 0);
    }

    #[test]
    fn master_volume_and_clamp() {
        let mut b = bus();
        let data: Vec<i8> = vec![127; 300];
        setup(&mut b, &data, None, 13379);
        // Three channels at full volume clip; master volume 5 brings them back in range.
        for n in 0..3 {
            start(&mut b, n, 13379, 255, 255, [255, 255, 255, 0]);
        }
        main(&mut b);
        assert_eq!(slot_bytes(&b, 1, false, 1), vec![127], "clamped");
        swi(&mut b, swi::SOUND_DRIVER_MODE, [0x0000_5000, 0, 0, 0]);
        swi(&mut b, swi::SOUND_DRIVER_VSYNC, [0, 0, 0, 0]);
        main(&mut b);
        let expect = ((3 * 127 * 254 * 5) >> 12).min(127) as i8;
        assert_eq!(slot_bytes(&b, 2, false, 1), vec![expect]);
    }

    #[test]
    fn reverb_feeds_back_the_old_slot_content() {
        let mut b = bus();
        setup(&mut b, &[0; 8], None, 13379);
        swi(&mut b, swi::SOUND_DRIVER_MODE, [0x0000_00FF, 0, 0, 0]); // reverb 127
        // Slot 1 holds 64 on both sides from `period` frames ago; nothing is playing now.
        for i in 0..4u32 {
            b.write8(AREA + info::PCM_BUFFER + 224 + i, 64, Access::N);
            b.write8(AREA + info::PCM_BUFFER_LEFT + 224 + i, 64, Access::N);
        }
        main(&mut b);
        let echo = ((64 + 64) * 127) >> 8;
        assert_eq!(slot_bytes(&b, 1, false, 4), vec![echo as i8; 4]);
        assert_eq!(slot_bytes(&b, 1, true, 4), vec![echo as i8; 4]);
        assert_eq!(slot_bytes(&b, 1, false, 8)[4..], [0, 0, 0, 0]);
    }

    #[test]
    fn fifo_is_fed_while_main_mixes() {
        let mut b = bus();
        let data: Vec<i8> = (0..2000).map(|i| ((i % 200) as i32 - 100) as i8).collect();
        setup(&mut b, &data, Some(0), 13379);
        for n in 0..8 {
            start(&mut b, n, 13379 + n * 100, 200, 200, [255, 0, 0, 0]);
        }
        // Let the primed FIFO drain a little, then mix: timer pops must be served meanwhile.
        b.idle(3000);
        b.hle_service();
        let t0 = b.sched.now;
        main(&mut b);
        let elapsed = b.sched.now - t0;
        assert!(elapsed > 5_000, "8 channels x 224 samples should cost more than {elapsed} cycles");
        assert!(elapsed < 60_000, "mixing took {elapsed} cycles, a quarter of the frame");
        assert_eq!(b.apu.fifo_a.underruns, 0);
        assert_eq!(b.apu.fifo_b.underruns, 0);
        assert!(b.apu.fifo_a.popped >= (elapsed / 1254) - 1);
        assert_eq!(b.peek32(AREA), ID_NUMBER);
    }
}
