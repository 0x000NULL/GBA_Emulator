//! Channel 4: LFSR sequences against a reference, clock divider, envelope and length.

use super::*;
use crate::apu::psg::{NOISE_DIVISOR, PSG_TICK_CYCLES};

/// Reference polynomial counter, transcribed from GBATEK:
/// `X=X SHR 1, IF carry THEN Out=HIGH, X=X XOR 6000h (60h for 7bit) ELSE Out=LOW`.
fn reference(x: &mut u16, width7: bool) -> bool {
    let carry = *x & 1 != 0;
    *x >>= 1;
    if carry {
        *x ^= if width7 { 0x60 } else { 0x6000 };
    }
    carry
}

fn start_noise(a: &mut Apu, s: &mut Scheduler, env: u8, poly: u8, ctrl_hi: u8) {
    w8(a, s, 0x78, 0);
    w8(a, s, 0x79, env);
    w8(a, s, 0x7C, poly);
    w8(a, s, 0x7D, ctrl_hi);
}

#[test]
fn lfsr_15bit_matches_reference_and_repeats_after_7fff_steps() {
    let (mut a, mut s) = apu_on();
    start_noise(&mut a, &mut s, 0xF0, 0x00, 0x80);
    assert_eq!(a.noise.lfsr, 0x4000, "initial value X=4000h");
    let mut x = 0x4000u16;
    let mut first_repeat = None;
    for i in 1..=0x1_0000u32 {
        let want = reference(&mut x, false);
        a.noise.clock_lfsr();
        assert_eq!(a.noise.lfsr, x, "state after step {i}");
        assert_eq!(a.noise.output() != 0, want, "output after step {i}");
        if a.noise.lfsr == 0x4000 && first_repeat.is_none() {
            first_repeat = Some(i);
        }
    }
    assert_eq!(first_repeat, Some(0x7FFF), "data stream repeats after 7FFFh steps");
}

#[test]
fn lfsr_7bit_matches_reference_and_repeats_after_7f_steps() {
    let (mut a, mut s) = apu_on();
    start_noise(&mut a, &mut s, 0xF0, 0x08, 0x80);
    assert_eq!(a.noise.lfsr, 0x40, "initial value X=40h");
    let mut x = 0x40u16;
    let mut first_repeat = None;
    for i in 1..=1024u32 {
        let want = reference(&mut x, true);
        a.noise.clock_lfsr();
        assert_eq!(a.noise.lfsr, x, "state after step {i}");
        assert_eq!(a.noise.output() != 0, want);
        if a.noise.lfsr == 0x40 && first_repeat.is_none() {
            first_repeat = Some(i);
        }
    }
    assert_eq!(first_repeat, Some(0x7F));
}

#[test]
fn lfsr_output_is_high_or_envelope_volume() {
    let (mut a, mut s) = apu_on();
    start_noise(&mut a, &mut s, 0xB0, 0x00, 0x80);
    let mut seen_high = false;
    for _ in 0..64 {
        a.noise.clock_lfsr();
        let o = a.noise.output();
        assert!(o == 0 || o == 11);
        seen_high |= o == 11;
    }
    assert!(seen_high);
}

#[test]
fn clock_period_from_ratio_and_shift() {
    let (mut a, mut s) = apu_on();
    // Frequency = 524288 / r / 2^(s+1); r=0 counts as 0.5.
    for r in 0..8u8 {
        for sh in 0..16u8 {
            w8(&mut a, &mut s, 0x7C, r | (sh << 4));
            let hz = if r == 0 { 524288.0 * 2.0 } else { 524288.0 / r as f64 } / (1u64 << (sh + 1)) as f64;
            let want = (16_777_216.0 / hz).round() as u32;
            assert_eq!(a.noise.period(), want, "r={r} s={sh}");
            assert_eq!(a.noise.period(), NOISE_DIVISOR[r as usize] << (sh + 2));
        }
    }
}

#[test]
fn lfsr_advances_once_per_tick_at_262khz() {
    let (mut a, mut s) = apu_on();
    // r=1, s=0 => 262144 Hz => one shift per PSG tick.
    start_noise(&mut a, &mut s, 0xF0, 0x01, 0x80);
    let mut x = 0x4000u16;
    for _ in 0..500 {
        reference(&mut x, false);
        a.noise.step(PSG_TICK_CYCLES);
        assert_eq!(a.noise.lfsr, x);
    }
    // r=0, s=0 => 524288 Hz => two shifts per tick.
    w8(&mut a, &mut s, 0x7C, 0x00);
    w8(&mut a, &mut s, 0x7D, 0x80);
    let mut x = 0x4000u16;
    for _ in 0..500 {
        reference(&mut x, false);
        reference(&mut x, false);
        a.noise.step(PSG_TICK_CYCLES);
        assert_eq!(a.noise.lfsr, x);
    }
    // r=2, s=1 => 65536 Hz => one shift every 4 ticks.
    w8(&mut a, &mut s, 0x7C, 0x12);
    w8(&mut a, &mut s, 0x7D, 0x80);
    let mut x = 0x4000u16;
    for _ in 0..100 {
        for _ in 0..3 {
            a.noise.step(PSG_TICK_CYCLES);
            assert_eq!(a.noise.lfsr, x);
        }
        reference(&mut x, false);
        a.noise.step(PSG_TICK_CYCLES);
        assert_eq!(a.noise.lfsr, x);
    }
}

#[test]
fn restart_reloads_lfsr() {
    let (mut a, mut s) = apu_on();
    start_noise(&mut a, &mut s, 0xF0, 0x00, 0x80);
    for _ in 0..37 {
        a.noise.clock_lfsr();
    }
    assert_ne!(a.noise.lfsr, 0x4000);
    w8(&mut a, &mut s, 0x7D, 0x80);
    assert_eq!(a.noise.lfsr, 0x4000);
    w8(&mut a, &mut s, 0x7C, 0x08);
    w8(&mut a, &mut s, 0x7D, 0x80);
    assert_eq!(a.noise.lfsr, 0x40);
}

#[test]
fn envelope_and_length_apply_to_noise() {
    let (mut a, mut s) = apu_on();
    w8(&mut a, &mut s, 0x78, 62);
    w8(&mut a, &mut s, 0x79, 0xF1);
    w8(&mut a, &mut s, 0x7C, 0x00);
    w8(&mut a, &mut s, 0x7D, 0xC0);
    assert_eq!(status(&a), 0b1000);
    frames(&mut a, 2);
    assert_eq!(status(&a), 0b1000);
    frames(&mut a, 1);
    assert_eq!(status(&a), 0, "length expiry stops channel 4");
    // Without length: envelope decays.
    w8(&mut a, &mut s, 0x7D, 0x80);
    frames(&mut a, 5);
    assert_eq!(a.noise.envelope.volume, 14);
    assert_eq!(status(&a), 0b1000);
}

#[test]
fn dac_off_silences_noise() {
    let (mut a, mut s) = apu_on();
    start_noise(&mut a, &mut s, 0x00, 0x00, 0x80);
    assert_eq!(status(&a), 0);
    start_noise(&mut a, &mut s, 0xF0, 0x00, 0x80);
    assert_eq!(status(&a), 0b1000);
    w8(&mut a, &mut s, 0x79, 0x00);
    assert_eq!(status(&a), 0);
}
