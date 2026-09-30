//! Mixer: PSG/FIFO scaling, stereo enables, bias, clipping, resolution, end-to-end `on_sample`.

use super::*;
use crate::apu::mixer::{mix, MixInput};

/// Expected 16-bit output for a 10-bit mixed value at 9-bit resolution.
fn out9(v10: i32) -> i16 {
    (((v10.clamp(0, 0x3FF) >> 1) << 7) - 0x8000) as i16
}

fn base() -> MixInput {
    MixInput { psg_sum: [0; 4], ticks: 8, cnt_l: 0xFF77, cnt_h: 0x0002, fifo_a: 0, fifo_b: 0, bias: 0x200, master: true }
}

#[test]
fn silence_sits_at_zero_with_default_bias() {
    assert_eq!(mix(&base()), [0, 0]);
    assert_eq!(out9(0x200), 0);
}

#[test]
fn psg_full_volume_spans_a_quarter_of_the_range() {
    // One channel at level 15 for all 8 ticks, master volume 7, 100% ratio => 15 * 8 * 2 = 240.
    let mut m = base();
    m.psg_sum[0] = 15 * 8;
    assert_eq!(mix(&m), [out9(0x200 + 240); 2]);
    assert_eq!(mix(&m), [15360, 15360]);
    // 50% and 25% ratios.
    m.cnt_h = 0x0001;
    assert_eq!(mix(&m), [out9(0x200 + 120); 2]);
    m.cnt_h = 0x0000;
    assert_eq!(mix(&m), [out9(0x200 + 60); 2]);
    // Prohibited value 3 behaves as 100%.
    m.cnt_h = 0x0003;
    assert_eq!(mix(&m), [out9(0x200 + 240); 2]);
}

#[test]
fn psg_master_volume_scales_linearly() {
    let mut m = base();
    m.psg_sum[1] = 15 * 8;
    for vol in 0..8u16 {
        m.cnt_l = 0xFF00 | vol << 4 | vol;
        let want = 15 * (vol as i32 + 1) * 2;
        assert_eq!(mix(&m), [out9(0x200 + want); 2], "volume {vol}");
    }
}

#[test]
fn psg_channels_sum_and_respect_left_right_enables() {
    let mut m = base();
    m.psg_sum = [15 * 8, 15 * 8, 15 * 8, 15 * 8];
    assert_eq!(mix(&m), [out9(0x200 + 960); 2]);
    // Left: channel 1 only (bit 12); right: channels 2 and 4 (bits 9, 11).
    m.cnt_l = 0x1A77;
    assert_eq!(mix(&m), [out9(0x200 + 240), out9(0x200 + 480)]);
    // Nothing enabled on either side.
    m.cnt_l = 0x0077;
    assert_eq!(mix(&m), [0, 0]);
}

#[test]
fn psg_levels_are_averaged_over_ticks() {
    let mut m = base();
    // 50% duty seen as 4 high ticks out of 8 => 7.5 average => 7.5 * 16 = 120.
    m.psg_sum[0] = 15 * 4;
    assert_eq!(mix(&m), [out9(0x200 + 120); 2]);
    m.ticks = 1;
    m.psg_sum[0] = 15;
    assert_eq!(mix(&m), [out9(0x200 + 240); 2]);
}

#[test]
fn fifo_full_volume_spans_full_range() {
    let mut m = base();
    m.cnt_h = 0x0304; // A L+R, 100%
    m.fifo_a = 127;
    assert_eq!(mix(&m), [out9(0x200 + 508); 2]);
    assert_eq!(mix(&m), [32512, 32512]);
    m.fifo_a = -128;
    assert_eq!(mix(&m), [out9(0x200 - 512); 2]);
    assert_eq!(mix(&m), [-32768, -32768]);
    // 50%.
    m.cnt_h = 0x0300;
    m.fifo_a = 127;
    assert_eq!(mix(&m), [out9(0x200 + 254); 2]);
    m.fifo_a = -128;
    assert_eq!(mix(&m), [out9(0x200 - 256); 2]);
}

#[test]
fn fifo_b_and_stereo_enables() {
    let mut m = base();
    // B left only (bit 13), 100% (bit 3); A right only (bit 8), 50%.
    m.cnt_h = 0x2100 | 0x0008;
    m.fifo_a = 100;
    m.fifo_b = -50;
    assert_eq!(mix(&m), [out9(0x200 - 200), out9(0x200 + 200)]);
}

#[test]
fn sum_clips_to_10_bits() {
    let mut m = base();
    m.cnt_h = 0x330E; // A and B, L+R, 100%; PSG ratio 100%
    m.fifo_a = 127;
    m.fifo_b = 127;
    assert_eq!(mix(&m), [out9(0x3FF); 2]);
    assert_eq!(mix(&m), [32640, 32640]);
    m.fifo_a = -128;
    m.fifo_b = -128;
    m.psg_sum = [15 * 8; 4];
    // -1024 + 960 + 512 = 448 => no clip.
    assert_eq!(mix(&m), [out9(448); 2]);
    m.psg_sum = [0; 4];
    assert_eq!(mix(&m), [-32768, -32768]);
}

#[test]
fn bias_level_shifts_output() {
    let mut m = base();
    m.bias = 0x000;
    assert_eq!(mix(&m), [-32768, -32768], "zero bias: PWM low bits only");
    m.bias = 0x3FE;
    assert_eq!(mix(&m), [out9(0x3FE); 2]);
    m.bias = 0x100;
    m.cnt_h = 0x0304;
    m.fifo_a = 64;
    assert_eq!(mix(&m), [out9(0x100 + 256); 2]);
    // Bit 0 of SOUNDBIAS is unused.
    m.bias = 0x201;
    m.fifo_a = 0;
    assert_eq!(mix(&m), [0, 0]);
}

#[test]
fn amplitude_resolution_drops_low_bits() {
    let mut m = base();
    m.cnt_h = 0x0300; // A 50%: sample s => s*2
    // 9-bit: value 0x202 keeps bit 1 => 128 steps of 16-bit; 0x201 impossible (even inputs).
    m.fifo_a = 1;
    assert_eq!(mix(&m), [128, 128]);
    // 8-bit / 65.536 kHz: 0x202 -> (0x202 >> 2) << 8 = 0x8000 => 0; 0x204 -> 256.
    m.bias = 0x200 | 1 << 14;
    assert_eq!(mix(&m), [0, 0]);
    m.fifo_a = 2;
    assert_eq!(mix(&m), [256, 256]);
    // 6-bit / 262.144 kHz: 0x20E -> 0; 0x210 -> 1024.
    m.bias = 0x200 | 3 << 14;
    m.fifo_a = 7;
    assert_eq!(mix(&m), [0, 0]);
    m.fifo_a = 8;
    assert_eq!(mix(&m), [1024, 1024]);
}

#[test]
fn master_off_outputs_bias_only() {
    let mut m = base();
    m.master = false;
    m.psg_sum = [15 * 8; 4];
    m.cnt_h = 0x330E;
    m.fifo_a = 127;
    m.fifo_b = 127;
    assert_eq!(mix(&m), [0, 0]);
}

// ---------------------------------------------------------------- end to end

#[test]
fn on_sample_mixes_constant_wave_channel() {
    let (mut a, mut s) = apu_on();
    w16(&mut a, &mut s, 0x80, 0xFF77);
    w16(&mut a, &mut s, 0x82, 0x0002);
    w8(&mut a, &mut s, 0x70, 0x40);
    for i in 0..16 {
        w8(&mut a, &mut s, 0x90 + i, 0xFF);
    }
    w8(&mut a, &mut s, 0x70, 0x80);
    w8(&mut a, &mut s, 0x73, 0x20);
    w16(&mut a, &mut s, 0x74, 0x8000 | 2040);
    a.on_sample();
    a.on_sample();
    assert_eq!(a.samples, vec![[15360, 15360]; 2]);
    assert_eq!(a.sample_count, 2);
    // Channel 3 right only.
    w16(&mut a, &mut s, 0x80, 0x0477);
    a.on_sample();
    assert_eq!(a.samples[2], [0, 15360]);
}

#[test]
fn on_sample_averages_fast_square_wave() {
    let (mut a, mut s) = apu_on();
    w16(&mut a, &mut s, 0x80, 0xFF77);
    w16(&mut a, &mut s, 0x82, 0x0002);
    // n = 2047: 16 cycles per duty step, so each 64-cycle tick lands 4 steps apart and a 50%
    // duty alternates high/low tick by tick: 4 of 8 ticks high => 120.
    w16(&mut a, &mut s, 0x62, 0xF080);
    w16(&mut a, &mut s, 0x64, 0x8000 | 2047);
    a.on_sample();
    assert_eq!(a.samples[0], [out9(0x200 + 120); 2]);
}

#[test]
fn on_sample_uses_latched_fifo_samples() {
    let (mut a, mut s) = apu_on();
    w16(&mut a, &mut s, 0x82, 0x0304);
    a.fifo_write(0, &[100u8, 0x9C, 0, 0]); // 100, -100
    a.on_sample();
    assert_eq!(a.samples[0], [0, 0], "nothing latched yet");
    a.on_timer_overflow(0);
    a.on_sample();
    assert_eq!(a.samples[1], [out9(0x200 + 400); 2]);
    a.on_timer_overflow(0);
    a.on_sample();
    assert_eq!(a.samples[2], [out9(0x200 - 400); 2]);
}

#[test]
fn soundbias_resolution_sets_ticks_per_sample() {
    let (mut a, mut s) = apu_on();
    w16(&mut a, &mut s, 0x80, 0xFF77);
    w16(&mut a, &mut s, 0x82, 0x0002);
    w16(&mut a, &mut s, 0x62, 0xF080);
    w16(&mut a, &mut s, 0x64, 0x8000 | 2016); // 8 ticks per duty step
    // 262.144 kHz output: one PSG tick per sample, so the 50% duty shows as 32 high, 32 low.
    w16(&mut a, &mut s, 0x88, 0xC200);
    assert_eq!(a.cycles_per_sample(), 64);
    for _ in 0..64 {
        a.on_sample();
    }
    let high = a.samples.iter().filter(|s| s[0] == out9(0x200 + 240)).count();
    let low = a.samples.iter().filter(|s| s[0] == out9(0x200)).count();
    assert_eq!((high, low), (32, 32));
    // Output is 6-bit at this rate: 0x2F0 -> (0x2F0 >> 4) << 10 = 0xBC00 => 15360 still exact.
    assert_eq!(out9(0x200 + 240), (((0x2F0i32 >> 4) << 10) - 0x8000) as i16);
}
