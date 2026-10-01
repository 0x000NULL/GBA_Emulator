//! Bench capture: peak levels and channel mutes. Muting only edits the SOUNDCNT copies handed to
//! the mixer; registers, FIFOs and underrun accounting must stay exactly as without a mute.

use super::*;
use crate::apu::capture::*;

/// SOUNDCNT_H routing both FIFOs to both speakers at 100%, PSG ratio 100%.
const CNT_H_ALL: u16 = 0x330E;

fn attach(a: &mut Apu) {
    a.debug = Some(Box::new(ApuCapture::new()));
}

fn cap(a: &mut Apu) -> &mut ApuCapture {
    a.debug.as_deref_mut().unwrap()
}

/// Route everything to both sides at full volume.
fn route_all(a: &mut Apu, s: &mut Scheduler) {
    w16(a, s, 0x80, 0xFF77);
    w16(a, s, 0x82, CNT_H_ALL);
}

fn trigger_sq1(a: &mut Apu, s: &mut Scheduler, vol: u8) {
    w8(a, s, 0x63, vol << 4);
    w8(a, s, 0x65, 0x80);
}

fn trigger_sq2(a: &mut Apu, s: &mut Scheduler, vol: u8) {
    w8(a, s, 0x69, vol << 4);
    w8(a, s, 0x6D, 0x80);
}

fn trigger_noise(a: &mut Apu, s: &mut Scheduler, vol: u8) {
    w8(a, s, 0x79, vol << 4);
    w8(a, s, 0x7D, 0x80);
}

/// Wave channel playing a bank of 0xFF digits (level 15) at 100%.
fn trigger_wave(a: &mut Apu, s: &mut Scheduler) {
    w8(a, s, 0x70, 0x40);
    for i in 0..16 {
        w8(a, s, 0x90 + i, 0xFF);
    }
    w8(a, s, 0x70, 0x80);
    w8(a, s, 0x73, 0x20);
    w16(a, s, 0x74, 0x8000 | 2040);
}

fn run(a: &mut Apu, n: usize) -> Vec<[i16; 2]> {
    a.samples.clear();
    for _ in 0..n {
        a.on_sample();
    }
    std::mem::take(&mut a.samples)
}

#[test]
fn peaks_track_channel_levels() {
    let (mut a, mut s) = apu_on();
    attach(&mut a);
    route_all(&mut a, &mut s);
    trigger_sq1(&mut a, &mut s, 10);
    trigger_sq2(&mut a, &mut s, 7);
    trigger_wave(&mut a, &mut s);
    trigger_noise(&mut a, &mut s, 12);
    a.fifo_a.push(&[(-100i8) as u8]);
    a.fifo_a.pop();
    a.fifo_b.push(&[0x80]);
    a.fifo_b.pop();
    run(&mut a, 8);
    let peaks = cap(&mut a).take_peaks();
    assert_eq!(peaks[CH_SQ1], 10);
    assert_eq!(peaks[CH_SQ2], 7);
    assert_eq!(peaks[CH_WAVE], 15);
    assert_eq!(peaks[CH_NOISE], 12);
    assert_eq!(peaks[CH_FIFO_A], 100);
    assert_eq!(peaks[CH_FIFO_B], 128, "i8::MIN has magnitude 128");
    assert_eq!(cap(&mut a).take_peaks(), [0; 6], "taking the peaks resets them");
}

#[test]
fn peaks_are_zero_for_silent_channels() {
    let (mut a, mut s) = apu_on();
    attach(&mut a);
    route_all(&mut a, &mut s);
    // Envelope volume set but never triggered: the channel is off.
    w8(&mut a, &mut s, 0x63, 0xA0);
    run(&mut a, 4);
    assert_eq!(cap(&mut a).take_peaks(), [0; 6]);
}

#[test]
fn channel_mute_silences_only_that_channel() {
    // Reference: only SQ2 sounds. Test: SQ1 and SQ2 sound but SQ1 is muted.
    let (mut reference, mut rs) = apu_on();
    route_all(&mut reference, &mut rs);
    trigger_sq2(&mut reference, &mut rs, 9);
    let want = run(&mut reference, 64);

    let (mut a, mut s) = apu_on();
    attach(&mut a);
    route_all(&mut a, &mut s);
    trigger_sq1(&mut a, &mut s, 12);
    trigger_sq2(&mut a, &mut s, 9);
    cap(&mut a).set_mute(1 << CH_SQ1);
    assert_eq!(run(&mut a, 64), want, "muted SQ1 leaves exactly the SQ2 mix");
    assert_eq!(cap(&mut a).take_peaks()[CH_SQ1], 12, "a muted channel is still captured");

    // Unmuting brings it back.
    cap(&mut a).set_mute(0);
    let (mut both, mut bs) = apu_on();
    route_all(&mut both, &mut bs);
    trigger_sq1(&mut both, &mut bs, 12);
    trigger_sq2(&mut both, &mut bs, 9);
    run(&mut both, 64);
    assert_eq!(run(&mut a, 64), run(&mut both, 64));
}

#[test]
fn fifo_mute_removes_only_the_fifo() {
    let (mut quiet, mut qs) = apu_on();
    route_all(&mut quiet, &mut qs);
    trigger_sq1(&mut quiet, &mut qs, 8);
    // Three consecutive 32-sample windows of the SQ1-only mix.
    let reference = run(&mut quiet, 96);
    let window = |i: usize| reference[i * 32..(i + 1) * 32].to_vec();

    let (mut a, mut s) = apu_on();
    attach(&mut a);
    route_all(&mut a, &mut s);
    trigger_sq1(&mut a, &mut s, 8);
    a.fifo_a.push(&[100]);
    a.fifo_a.pop();
    a.fifo_b.push(&[(-90i8) as u8]);
    a.fifo_b.pop();
    let loud = run(&mut a, 32);
    assert_ne!(loud, window(0), "the FIFOs are audible without a mute");
    cap(&mut a).set_mute((1 << CH_FIFO_A) | (1 << CH_FIFO_B));
    assert_eq!(run(&mut a, 32), window(1));
    // One FIFO at a time.
    cap(&mut a).set_mute(1 << CH_FIFO_A);
    let only_b = run(&mut a, 32);
    assert_ne!(only_b, window(2), "FIFO B still sounds");
}

#[test]
fn mute_leaves_registers_fifo_and_underruns_alone() {
    fn drive(a: &mut Apu, s: &mut Scheduler) {
        route_all(a, s);
        w16(a, s, 0x82, CNT_H_ALL | 1 << 14);
        trigger_sq1(a, s, 11);
        trigger_noise(a, s, 5);
        a.fifo_a.push(&[1, 2, 3]);
        a.fifo_b.push(&[4]);
        for _ in 0..6 {
            // Timer 0 clocks FIFO A, timer 1 FIFO B (SOUNDCNT_H bit 14 set below): both underrun.
            a.on_timer_overflow(0);
            a.on_timer_overflow(1);
            a.on_sample();
        }
    }
    let (mut plain, mut ps) = apu_on();
    let (mut muted, mut ms) = apu_on();
    attach(&mut muted);
    cap(&mut muted).set_mute(0x3F);
    drive(&mut plain, &mut ps);
    drive(&mut muted, &mut ms);
    for off in (0x60..0x88).step_by(2) {
        assert_eq!(r16(&plain, off), r16(&muted, off), "register {off:#x}");
    }
    assert_eq!(status(&plain), status(&muted));
    for (p, m) in [(&plain.fifo_a, &muted.fifo_a), (&plain.fifo_b, &muted.fifo_b)] {
        assert_eq!((p.len(), p.underruns, p.pushed, p.popped, p.sample), (m.len(), m.underruns, m.pushed, m.popped, m.sample));
    }
    assert!(muted.fifo_a.underruns > 0 && muted.fifo_b.underruns > 0, "the underruns really happened");
    assert!(muted.samples.iter().all(|&s| s == [0, 0]), "everything muted: silence");
    assert_ne!(plain.samples, muted.samples);
    assert_eq!(plain.sample_count, muted.sample_count);
}

#[test]
fn set_mute_builds_the_routing_masks() {
    let mut c = ApuCapture::new();
    c.set_mute(1 << CH_SQ2);
    assert_eq!((c.mask_l, c.mask_h), ((1 << 9) | (1 << 13), 0));
    c.set_mute(1 << CH_FIFO_B);
    assert_eq!((c.mask_l, c.mask_h), (0, (1 << 12) | (1 << 13)));
    c.set_mute(0x3F);
    assert_eq!((c.mask_l, c.mask_h), (0xFF00, 0x3300));
    c.set_mute(0);
    assert_eq!((c.mask_l, c.mask_h), (0, 0));
}

#[test]
fn capture_off_mix_is_identical() {
    fn drive(a: &mut Apu, s: &mut Scheduler) -> Vec<[i16; 2]> {
        route_all(a, s);
        trigger_sq1(a, s, 10);
        trigger_sq2(a, s, 6);
        trigger_wave(a, s);
        trigger_noise(a, s, 9);
        a.fifo_a.push(&[40, 41]);
        a.fifo_a.pop();
        run(a, 200)
    }
    let (mut off, mut os) = apu_on();
    let (mut on, mut ns) = apu_on();
    attach(&mut on);
    assert_eq!(drive(&mut off, &mut os), drive(&mut on, &mut ns));
    assert!(Apu::new().debug.is_none(), "capture is off by default");
}
