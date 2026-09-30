//! Register read-back masks, SOUNDCNT_X master enable, SOUNDBIAS, scheduling.

use super::*;
use crate::sched::Event;
use crate::timing::CYCLES_PER_FRAME_SEQ;

#[test]
fn read_masks_hide_write_only_and_unused_bits() {
    let (mut a, mut s) = apu_on();
    for off in 0x60..0x84u16 {
        w8(&mut a, &mut s, off, 0xFF);
    }
    for off in 0x85..0xB0u16 {
        w8(&mut a, &mut s, off, 0xFF);
    }
    let want: [(u16, u8); 24] = [
        (0x60, 0x7F), (0x61, 0x00), // SOUND1CNT_L: sweep bits 0-6
        (0x62, 0xC0), (0x63, 0xFF), // SOUND1CNT_H: length write-only
        (0x64, 0x00), (0x65, 0x40), // SOUND1CNT_X: length flag only
        (0x68, 0xC0), (0x69, 0xFF),
        (0x6C, 0x00), (0x6D, 0x40),
        (0x70, 0xE0), (0x71, 0x00),
        (0x72, 0x00), (0x73, 0xE0),
        (0x74, 0x00), (0x75, 0x40),
        (0x78, 0x00), (0x79, 0xFF),
        (0x7C, 0xFF), (0x7D, 0x40),
        (0x80, 0x77), (0x81, 0xFF),
        (0x82, 0x0F), (0x83, 0x77),
    ];
    for (off, v) in want {
        assert_eq!(a.read_io8(off), v, "read {off:03X}");
    }
    for off in [0x66, 0x67, 0x6A, 0x6B, 0x6E, 0x6F, 0x76, 0x77, 0x7A, 0x7B, 0x7E, 0x7F, 0x85, 0x86, 0x87] {
        assert_eq!(a.read_io8(off), 0, "unused {off:03X}");
    }
    assert_eq!(r16(&a, 0x88), 0xC3FE);
    for off in 0x8A..0x90u16 {
        assert_eq!(a.read_io8(off), 0, "unused {off:03X}");
    }
    for off in 0x90..0xA0u16 {
        assert_eq!(a.read_io8(off), 0xFF, "wave RAM {off:03X}");
    }
    for off in 0xA0..0xB0u16 {
        assert_eq!(a.read_io8(off), 0, "FIFO / gap {off:03X}");
    }
    // All four channels were restarted with their DACs on.
    assert_eq!(a.read_io8(0x84), 0x8F);
}

#[test]
fn accesses_outside_the_range_are_ignored() {
    let (mut a, mut s) = apu_on();
    w8(&mut a, &mut s, 0x5F, 0xFF);
    w8(&mut a, &mut s, 0xB0, 0xFF);
    assert_eq!(a.read_io8(0x5F), 0);
    assert_eq!(a.read_io8(0xB0), 0);
    assert_eq!(a.read_io8(0x60), 0);
}

#[test]
fn master_off_resets_psg_registers_and_channels() {
    let (mut a, mut s) = apu_on();
    w8(&mut a, &mut s, 0x60, 0x11);
    w16(&mut a, &mut s, 0x62, 0xF080);
    w16(&mut a, &mut s, 0x64, 0x8000 | 1000);
    w16(&mut a, &mut s, 0x68, 0xF080);
    w16(&mut a, &mut s, 0x6C, 0x8000 | 1000);
    w8(&mut a, &mut s, 0x70, 0x80);
    w8(&mut a, &mut s, 0x73, 0x20);
    for i in 0..16 {
        w8(&mut a, &mut s, 0x90 + i, 0x5A);
    }
    w16(&mut a, &mut s, 0x74, 0x8000 | 1000);
    w8(&mut a, &mut s, 0x79, 0xF0);
    w8(&mut a, &mut s, 0x7D, 0x80);
    w16(&mut a, &mut s, 0x80, 0xFF77);
    w16(&mut a, &mut s, 0x82, 0x770F);
    w16(&mut a, &mut s, 0x88, 0x4200);
    assert_eq!(a.read_io8(0x84), 0x8F);

    w8(&mut a, &mut s, 0x84, 0x00);
    assert!(!a.master_enabled());
    for off in 0x60..0x82u16 {
        assert_eq!(a.read_io8(off), 0, "PSG register {off:03X} reset");
    }
    assert_eq!(a.read_io8(0x84), 0x00, "status bits cleared");
    assert_eq!(r16(&a, 0x82), 0x770F, "SOUNDCNT_H kept");
    assert_eq!(r16(&a, 0x88), 0x4200, "SOUNDBIAS kept");
    assert_eq!(a.read_io8(0x95), 0x5A, "wave RAM kept");
    assert_eq!(a.sq1.output() + a.sq2.output() + a.wave.output() + a.noise.output(), 0);
    assert_eq!(a.sq1.freq, 0);
    assert_eq!(a.sq1.envelope.volume, 0);

    // Writes to 0x60..0x81 are ignored while off; 0x82 and 0x88 still work.
    w16(&mut a, &mut s, 0x62, 0xF080);
    w16(&mut a, &mut s, 0x64, 0x8000 | 1000);
    w16(&mut a, &mut s, 0x80, 0xFF77);
    w16(&mut a, &mut s, 0x82, 0x0004);
    w16(&mut a, &mut s, 0x88, 0x0200);
    assert_eq!(r16(&a, 0x62), 0);
    assert_eq!(r16(&a, 0x80), 0);
    assert_eq!(a.read_io8(0x84), 0);
    assert_eq!(r16(&a, 0x82), 0x0004);
    assert_eq!(r16(&a, 0x88), 0x0200);
    frames(&mut a, 16);
    a.on_sample();
    assert_eq!(a.samples[0], [0, 0]);

    // Re-enable: registers stay zero until rewritten, then the channel works again.
    w8(&mut a, &mut s, 0x84, 0x80);
    assert_eq!(a.read_io8(0x84), 0x80);
    assert_eq!(r16(&a, 0x62), 0);
    w16(&mut a, &mut s, 0x62, 0xF080);
    w16(&mut a, &mut s, 0x64, 0x8000 | 1000);
    assert_eq!(a.read_io8(0x84), 0x81);
}

#[test]
fn master_off_from_boot_blocks_psg_writes() {
    let mut a = Apu::new();
    let mut s = Scheduler::new();
    assert!(!a.master_enabled());
    w16(&mut a, &mut s, 0x62, 0xF080);
    w16(&mut a, &mut s, 0x64, 0x8000 | 1000);
    assert_eq!(a.read_io8(0x84), 0);
    assert_eq!(r16(&a, 0x62), 0);
    w8(&mut a, &mut s, 0x84, 0x80);
    w16(&mut a, &mut s, 0x62, 0xF080);
    w16(&mut a, &mut s, 0x64, 0x8000 | 1000);
    assert_eq!(a.read_io8(0x84), 0x81);
    assert_eq!(r16(&a, 0x62), 0xF080);
}

#[test]
fn soundcnt_x_only_bit_7_is_writable() {
    let (mut a, mut s) = apu_on();
    w8(&mut a, &mut s, 0x84, 0xFF);
    assert_eq!(a.read_io8(0x84), 0x80, "status bits are read-only");
    assert!(a.master_enabled());
    w8(&mut a, &mut s, 0x84, 0x0F);
    assert_eq!(a.read_io8(0x84), 0x00);
    assert!(!a.master_enabled());
}

#[test]
fn status_bits_track_channel_enables() {
    let (mut a, mut s) = apu_on();
    assert_eq!(status(&a), 0);
    w16(&mut a, &mut s, 0x62, 0xF000);
    w16(&mut a, &mut s, 0x64, 0x8000);
    assert_eq!(status(&a), 0b0001);
    w16(&mut a, &mut s, 0x68, 0xF000);
    w16(&mut a, &mut s, 0x6C, 0x8000);
    assert_eq!(status(&a), 0b0011);
    w8(&mut a, &mut s, 0x70, 0x80);
    w16(&mut a, &mut s, 0x74, 0x8000);
    assert_eq!(status(&a), 0b0111);
    w8(&mut a, &mut s, 0x79, 0xF0);
    w8(&mut a, &mut s, 0x7D, 0x80);
    assert_eq!(status(&a), 0b1111);
    w8(&mut a, &mut s, 0x70, 0x00);
    assert_eq!(status(&a), 0b1011);
    w8(&mut a, &mut s, 0x69, 0x00);
    assert_eq!(status(&a), 0b1001);
}

#[test]
fn soundbias_default_and_cycles_per_sample() {
    let a = Apu::new();
    assert_eq!(a.reg16(0x88), 0x0200);
    assert_eq!(a.soundbias(), 0x0200);
    assert_eq!(a.cycles_per_sample(), 512);
    let mut a = a;
    let mut s = Scheduler::new();
    for (hi, want) in [(0x02u8, 512u64), (0x42, 256), (0x82, 128), (0xC2, 64)] {
        w8(&mut a, &mut s, 0x89, hi);
        assert_eq!(a.cycles_per_sample(), want);
        assert_eq!(a.read_io8(0x89), hi);
    }
    w8(&mut a, &mut s, 0x88, 0xFF);
    assert_eq!(a.read_io8(0x88), 0xFE, "bit 0 unused");
    w8(&mut a, &mut s, 0x89, 0x3F);
    assert_eq!(a.read_io8(0x89), 0x03, "bits 10-13 unused");
}

#[test]
fn reg16_composes_bytes_as_the_cpu_sees_them() {
    let (mut a, mut s) = apu_on();
    w16(&mut a, &mut s, 0x80, 0xABCD);
    assert_eq!(a.reg16(0x80), 0xABCD & 0xFF77);
    assert_eq!(a.reg16(0x81), 0xABCD & 0xFF77, "odd offset rounds down");
    w16(&mut a, &mut s, 0x62, 0xFFFF);
    assert_eq!(a.reg16(0x62), 0xFFC0);
}

#[test]
fn boot_schedule_arms_sample_and_frame_sequencer() {
    let mut a = Apu::new();
    let mut s = Scheduler::new();
    s.now = 1000;
    a.boot_schedule(&mut s);
    assert_eq!(s.time_of(Event::ApuSample), Some(1000 + 512));
    assert_eq!(s.time_of(Event::ApuFrameSeq), Some(1000 + CYCLES_PER_FRAME_SEQ));
    assert_eq!(CYCLES_PER_FRAME_SEQ, 32768);
}

#[test]
fn frame_sequencer_does_not_run_while_master_off() {
    let (mut a, mut s) = apu_on();
    w16(&mut a, &mut s, 0x62, 0xF100 | 62);
    w16(&mut a, &mut s, 0x64, 0xC000 | 1000);
    w8(&mut a, &mut s, 0x84, 0x00);
    frames(&mut a, 64);
    assert_eq!(a.frame_step, 0);
    w8(&mut a, &mut s, 0x84, 0x80);
    frames(&mut a, 3);
    assert_eq!(a.frame_step, 3);
}
