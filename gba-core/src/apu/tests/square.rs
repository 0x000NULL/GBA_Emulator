//! Channels 1/2: tone period, duty, envelope, sweep, length, DAC.

use super::*;
use crate::apu::psg::PSG_TICK_CYCLES;

/// Set up channel 1: duty `duty`, envelope byte `env`, frequency `n`, restart with `ctrl_hi` bits.
fn start_ch1(a: &mut Apu, s: &mut Scheduler, duty: u8, env: u8, n: u16, ctrl_hi: u8) {
    w8(a, s, 0x62, duty << 6);
    w8(a, s, 0x63, env);
    w8(a, s, 0x64, n as u8);
    w8(a, s, 0x65, ctrl_hi | ((n >> 8) as u8 & 7));
}

/// Step channel 1 through `ticks` PSG ticks, returning its level after each tick.
fn levels_ch1(a: &mut Apu, ticks: usize) -> Vec<u8> {
    (0..ticks)
        .map(|_| {
            a.sq1.step(PSG_TICK_CYCLES);
            a.sq1.output()
        })
        .collect()
}

#[test]
fn square_period_matches_frequency_value() {
    // n = 2016: tone 131072/32 = 4096 Hz = 4096 cycles per wave = 64 PSG ticks; 8 ticks per duty step.
    let (mut a, mut s) = apu_on();
    start_ch1(&mut a, &mut s, 2, 0xF0, 2016, 0x80);
    assert_eq!(a.sq1.period(), 512);
    let lv = levels_ch1(&mut a, 64 * 4);
    let rising: Vec<usize> = (1..lv.len()).filter(|&i| lv[i - 1] == 0 && lv[i] != 0).collect();
    assert!(rising.len() >= 3, "expected several rising edges, got {rising:?}");
    for w in rising.windows(2) {
        assert_eq!(w[1] - w[0], 64, "wave period in PSG ticks");
    }
    // 50% duty: each high run is 32 ticks.
    let run = lv[rising[0]..].iter().take_while(|&&v| v != 0).count();
    assert_eq!(run, 32);
    assert!(lv.iter().all(|&v| v == 0 || v == 15));
}

#[test]
fn channel2_period_matches_channel1() {
    let (mut a, mut s) = apu_on();
    w16(&mut a, &mut s, 0x68, 0xF080);
    w16(&mut a, &mut s, 0x6C, 0x8000 | 1024);
    assert_eq!(a.sq2.period(), 1024 * 16);
    assert_eq!(status(&a), 0b0010);
    // 131072/1024 = 128 Hz => 131072 cycles per wave = 2048 ticks; a high run lasts 1024 ticks.
    let mut lv = Vec::new();
    for _ in 0..4096 {
        a.sq2.step(PSG_TICK_CYCLES);
        lv.push(a.sq2.output());
    }
    let first_low = lv.iter().position(|&v| v == 0).unwrap();
    let run = lv[first_low..].iter().take_while(|&&v| v == 0).count();
    assert_eq!(run, 1024);
}

#[test]
fn duty_cycles_high_fraction() {
    // One wave = 64 ticks at n = 2016; the fresh channel starts at duty step 0.
    for (duty, want_high) in [(0u8, 8usize), (1, 16), (2, 32), (3, 48)] {
        let (mut a, mut s) = apu_on();
        start_ch1(&mut a, &mut s, duty, 0xF0, 2016, 0x80);
        let lv = levels_ch1(&mut a, 64);
        let high = lv.iter().filter(|&&v| v != 0).count();
        assert_eq!(high, want_high, "duty {duty}");
    }
}

#[test]
fn envelope_decreases_one_step_per_period() {
    let (mut a, mut s) = apu_on();
    // Initial 15, decrease, step time 1 (1/64 s = every 8 frame-sequencer ticks).
    start_ch1(&mut a, &mut s, 2, 0xF1, 1000, 0x80);
    assert_eq!(a.sq1.envelope.volume, 15);
    frames(&mut a, 7);
    assert_eq!(a.sq1.envelope.volume, 15, "envelope clocks on step 7 only");
    frames(&mut a, 1);
    assert_eq!(a.sq1.envelope.volume, 14);
    frames(&mut a, 8);
    assert_eq!(a.sq1.envelope.volume, 13);
    frames(&mut a, 8 * 13);
    assert_eq!(a.sq1.envelope.volume, 0);
    frames(&mut a, 64);
    assert_eq!(a.sq1.envelope.volume, 0, "stops at zero");
    // "The bits are NOT reset when a volume envelope ends."
    assert_eq!(status(&a) & 1, 1);
    assert_eq!(a.sq1.output(), 0);
}

#[test]
fn envelope_increase_clamps_at_15_and_period_scales() {
    let (mut a, mut s) = apu_on();
    // Initial 0, increase, step time 3 => one step every 24 frame ticks.
    start_ch1(&mut a, &mut s, 2, 0x0B, 1000, 0x80);
    assert_eq!(status(&a) & 1, 1, "DAC on because direction is increase");
    assert_eq!(a.sq1.envelope.volume, 0);
    frames(&mut a, 23);
    assert_eq!(a.sq1.envelope.volume, 0);
    frames(&mut a, 1);
    assert_eq!(a.sq1.envelope.volume, 1);
    frames(&mut a, 24 * 14);
    assert_eq!(a.sq1.envelope.volume, 15);
    frames(&mut a, 24 * 4);
    assert_eq!(a.sq1.envelope.volume, 15);
}

#[test]
fn envelope_step_time_zero_holds_volume() {
    let (mut a, mut s) = apu_on();
    start_ch1(&mut a, &mut s, 2, 0xA0, 1000, 0x80);
    frames(&mut a, 512);
    assert_eq!(a.sq1.envelope.volume, 10);
}

#[test]
fn sweep_increase_updates_frequency_then_overflow_disables() {
    let (mut a, mut s) = apu_on();
    // Sweep: shift 1, increase, time 1 (one 128 Hz tick).
    w8(&mut a, &mut s, 0x60, 0x11);
    start_ch1(&mut a, &mut s, 2, 0xF0, 0x200, 0x80);
    assert_eq!(a.sq1.freq, 0x200);
    assert_eq!(status(&a) & 1, 1);
    // Sweep clocks on frame steps 2 and 6.
    frames(&mut a, 2);
    assert_eq!(a.sq1.freq, 0x200);
    frames(&mut a, 1);
    assert_eq!(a.sq1.freq, 0x300, "X(1) = X(0) + X(0)/2");
    frames(&mut a, 4);
    assert_eq!(a.sq1.freq, 0x480, "X(2) = X(1) + X(1)/2");
    assert_eq!(status(&a) & 1, 1);
    frames(&mut a, 4);
    // 0x480 + 0x240 = 0x6C0 fits, but the follow-up check 0x6C0 + 0x360 > 2047 stops the channel.
    assert_eq!(status(&a) & 1, 0, "sweep overflow disables channel 1");
    assert_eq!(a.sq1.output(), 0);
}

#[test]
fn sweep_decrease_updates_frequency() {
    let (mut a, mut s) = apu_on();
    // shift 2, decrease, time 1.
    w8(&mut a, &mut s, 0x60, 0x1A);
    start_ch1(&mut a, &mut s, 2, 0xF0, 0x400, 0x80);
    frames(&mut a, 3);
    assert_eq!(a.sq1.freq, 0x300);
    frames(&mut a, 4);
    assert_eq!(a.sq1.freq, 0x240);
    assert_eq!(status(&a) & 1, 1);
}

#[test]
fn sweep_time_scales_with_period() {
    let (mut a, mut s) = apu_on();
    // shift 1, increase, time 3 => first update on the third 128 Hz tick (frame call 11).
    w8(&mut a, &mut s, 0x60, 0x31);
    start_ch1(&mut a, &mut s, 2, 0xF0, 0x100, 0x80);
    frames(&mut a, 10);
    assert_eq!(a.sq1.freq, 0x100);
    frames(&mut a, 1);
    assert_eq!(a.sq1.freq, 0x180);
}

#[test]
fn sweep_overflow_on_restart_disables_immediately() {
    let (mut a, mut s) = apu_on();
    w8(&mut a, &mut s, 0x60, 0x11);
    start_ch1(&mut a, &mut s, 2, 0xF0, 0x7FF, 0x80);
    assert_eq!(status(&a) & 1, 0);
}

#[test]
fn sweep_time_zero_never_changes_frequency() {
    let (mut a, mut s) = apu_on();
    // time 0, decrease (as recommended), shift 1.
    w8(&mut a, &mut s, 0x60, 0x09);
    start_ch1(&mut a, &mut s, 2, 0xF0, 0x400, 0x80);
    frames(&mut a, 64);
    assert_eq!(a.sq1.freq, 0x400);
    assert_eq!(status(&a) & 1, 1);
}

#[test]
fn length_counter_expires_when_enabled() {
    let (mut a, mut s) = apu_on();
    // Length 62 => plays for (64-62) = 2 length ticks (256 Hz: frame steps 0,2,4,6).
    w8(&mut a, &mut s, 0x62, 0x80 | 62);
    w8(&mut a, &mut s, 0x63, 0xF0);
    w16(&mut a, &mut s, 0x64, 0xC000 | 1000);
    assert_eq!(status(&a) & 1, 1);
    frames(&mut a, 2);
    assert_eq!(status(&a) & 1, 1);
    frames(&mut a, 1);
    assert_eq!(status(&a) & 1, 0, "channel stops when length expires");
    assert_eq!(a.sq1.output(), 0);
}

#[test]
fn length_flag_clear_plays_forever() {
    let (mut a, mut s) = apu_on();
    w8(&mut a, &mut s, 0x62, 0x80 | 62);
    w8(&mut a, &mut s, 0x63, 0xF0);
    w16(&mut a, &mut s, 0x64, 0x8000 | 1000);
    frames(&mut a, 1000);
    assert_eq!(status(&a) & 1, 1);
}

#[test]
fn length_zero_means_full_64_ticks() {
    let (mut a, mut s) = apu_on();
    w8(&mut a, &mut s, 0x62, 0x80);
    w8(&mut a, &mut s, 0x63, 0xF0);
    w16(&mut a, &mut s, 0x64, 0xC000 | 1000);
    // 64 length ticks happen on frame calls 1,3,5,...,127.
    frames(&mut a, 126);
    assert_eq!(status(&a) & 1, 1);
    frames(&mut a, 1);
    assert_eq!(status(&a) & 1, 0);
}

#[test]
fn restart_reloads_length_and_envelope() {
    let (mut a, mut s) = apu_on();
    w8(&mut a, &mut s, 0x62, 0x80 | 62);
    w8(&mut a, &mut s, 0x63, 0xF1);
    w16(&mut a, &mut s, 0x64, 0xC000 | 1000);
    frames(&mut a, 3);
    assert_eq!(status(&a) & 1, 0);
    // Restart: an expired length counter reloads to 64, envelope back to 15.
    frames(&mut a, 5); // reach step 0 again (8 calls total)
    assert_eq!(a.sq1.envelope.volume, 14);
    w8(&mut a, &mut s, 0x65, 0xC0 | (1000u16 >> 8) as u8);
    assert_eq!(status(&a) & 1, 1);
    assert_eq!(a.sq1.envelope.volume, 15);
    assert_eq!(a.sq1.length.counter, 64);
}

#[test]
fn dac_off_stops_channel_and_blocks_restart() {
    let (mut a, mut s) = apu_on();
    start_ch1(&mut a, &mut s, 2, 0xF0, 1000, 0x80);
    assert_eq!(status(&a) & 1, 1);
    // Initial volume 0 + decrease = DAC off.
    w8(&mut a, &mut s, 0x63, 0x00);
    assert_eq!(status(&a) & 1, 0);
    w8(&mut a, &mut s, 0x65, 0x80 | (1000u16 >> 8) as u8);
    assert_eq!(status(&a) & 1, 0, "restart with DAC off does not enable the channel");
    // Volume 0 but increasing: DAC on, silent until the envelope rises.
    w8(&mut a, &mut s, 0x63, 0x08);
    w8(&mut a, &mut s, 0x65, 0x80 | (1000u16 >> 8) as u8);
    assert_eq!(status(&a) & 1, 1);
    assert_eq!(a.sq1.output(), 0);
}

#[test]
fn frequency_change_takes_effect_on_next_reload() {
    let (mut a, mut s) = apu_on();
    start_ch1(&mut a, &mut s, 2, 0xF0, 2016, 0x80);
    // Halve the period (n = 2032 => 256 cycles per step) without restarting.
    w8(&mut a, &mut s, 0x64, 2032u16 as u8);
    w8(&mut a, &mut s, 0x65, (2032u16 >> 8) as u8);
    let lv = levels_ch1(&mut a, 8 + 32 * 4);
    let lv = &lv[8..];
    let rising: Vec<usize> = (1..lv.len()).filter(|&i| lv[i - 1] == 0 && lv[i] != 0).collect();
    for w in rising.windows(2) {
        assert_eq!(w[1] - w[0], 32);
    }
}
