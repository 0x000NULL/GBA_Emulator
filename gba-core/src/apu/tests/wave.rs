//! Channel 3: wave RAM banks, playback order, volume, length, DAC.

use super::*;
use crate::apu::psg::PSG_TICK_CYCLES;

/// Write 16 bytes to the CPU-visible (inactive) wave RAM bank.
fn fill_ram(a: &mut Apu, s: &mut Scheduler, bytes: &[u8; 16]) {
    for (i, &b) in bytes.iter().enumerate() {
        w8(a, s, 0x90 + i as u16, b);
    }
}

const RAMP: [u8; 16] = [0x01, 0x23, 0x45, 0x67, 0x89, 0xAB, 0xCD, 0xEF, 0xFE, 0xDC, 0xBA, 0x98, 0x76, 0x54, 0x32, 0x10];

/// Levels after each of `ticks` PSG ticks.
fn levels(a: &mut Apu, ticks: usize) -> Vec<u8> {
    (0..ticks)
        .map(|_| {
            a.wave.step(PSG_TICK_CYCLES);
            a.wave.output()
        })
        .collect()
}

#[test]
fn playback_order_msb_first_single_bank() {
    let (mut a, mut s) = apu_on();
    // Select bank 1 for playback so CPU writes land in bank 0, then play bank 0.
    w8(&mut a, &mut s, 0x70, 0x40);
    fill_ram(&mut a, &mut s, &RAMP);
    w8(&mut a, &mut s, 0x70, 0x80);
    w8(&mut a, &mut s, 0x73, 0x20); // 100%
    // n = 2040 => 2097152/8 = 262144 digits/s = one digit per PSG tick.
    w16(&mut a, &mut s, 0x74, 0x8000 | 2040);
    assert_eq!(a.wave.period(), 64);
    assert_eq!(status(&a), 0b0100);
    assert_eq!(a.wave.output(), 0, "digit 0 = MSBs of byte 0");
    let lv = levels(&mut a, 64);
    let mut want: Vec<u8> = (0..16).chain((0..16).rev()).collect();
    let second = want.clone();
    want.extend(second);
    assert_eq!(lv[..31], want[1..32], "digits 1..31 in order");
    assert_eq!(lv[31], 0, "wraps to digit 0 after 32 digits");
    assert_eq!(lv[32..63], want[1..32]);
}

#[test]
fn sample_rate_follows_frequency_value() {
    let (mut a, mut s) = apu_on();
    w8(&mut a, &mut s, 0x70, 0x40);
    fill_ram(&mut a, &mut s, &RAMP);
    w8(&mut a, &mut s, 0x70, 0x80);
    w8(&mut a, &mut s, 0x73, 0x20);
    // n = 2032 => 128 cycles per digit = 2 ticks.
    w16(&mut a, &mut s, 0x74, 0x8000 | 2032);
    assert_eq!(a.wave.period(), 128);
    let lv = levels(&mut a, 8);
    assert_eq!(lv, vec![0, 1, 1, 2, 2, 3, 3, 4]);
}

#[test]
fn two_bank_mode_plays_selected_bank_then_other() {
    let (mut a, mut s) = apu_on();
    // Bank 0 <- 0x11 (written while bank 1 is selected), bank 1 <- 0x22 (while bank 0 selected).
    w8(&mut a, &mut s, 0x70, 0x40);
    fill_ram(&mut a, &mut s, &[0x11; 16]);
    w8(&mut a, &mut s, 0x70, 0x00);
    fill_ram(&mut a, &mut s, &[0x22; 16]);
    // Two banks, bank 1 selected, playback on.
    w8(&mut a, &mut s, 0x70, 0x80 | 0x20 | 0x40);
    w8(&mut a, &mut s, 0x73, 0x20);
    w16(&mut a, &mut s, 0x74, 0x8000 | 2040);
    assert_eq!(a.wave.output(), 2);
    let lv = levels(&mut a, 128);
    assert!(lv[..31].iter().all(|&v| v == 2), "rest of the selected bank first");
    assert!(lv[31..63].iter().all(|&v| v == 1), "then the other bank");
    assert!(lv[63..95].iter().all(|&v| v == 2), "then back to the selected bank");
    assert!(lv[95..127].iter().all(|&v| v == 1));
}

#[test]
fn single_bank_mode_ignores_other_bank() {
    let (mut a, mut s) = apu_on();
    w8(&mut a, &mut s, 0x70, 0x40);
    fill_ram(&mut a, &mut s, &[0x11; 16]);
    w8(&mut a, &mut s, 0x70, 0x00);
    fill_ram(&mut a, &mut s, &[0x22; 16]);
    w8(&mut a, &mut s, 0x70, 0x80); // one bank, bank 0
    w8(&mut a, &mut s, 0x73, 0x20);
    w16(&mut a, &mut s, 0x74, 0x8000 | 2040);
    let lv = levels(&mut a, 128);
    assert!(lv.iter().all(|&v| v == 1));
}

#[test]
fn volume_settings_scale_digits() {
    for (vol_byte, want) in [(0x00u8, 0u8), (0x20, 15), (0x40, 7), (0x60, 3), (0x80, 11), (0xA0, 11)] {
        let (mut a, mut s) = apu_on();
        w8(&mut a, &mut s, 0x70, 0x40);
        fill_ram(&mut a, &mut s, &[0xFF; 16]);
        w8(&mut a, &mut s, 0x70, 0x80);
        w8(&mut a, &mut s, 0x73, vol_byte);
        w16(&mut a, &mut s, 0x74, 0x8000 | 2040);
        assert_eq!(a.wave.output(), want, "NR32 high byte {vol_byte:02X}");
        assert_eq!(a.read_io8(0x73), vol_byte & 0xE0);
    }
}

#[test]
fn wave_ram_reads_return_inactive_bank() {
    let (mut a, mut s) = apu_on();
    w8(&mut a, &mut s, 0x70, 0x00); // bank 0 plays, CPU sees bank 1
    fill_ram(&mut a, &mut s, &RAMP);
    for i in 0..16 {
        assert_eq!(a.read_io8(0x90 + i as u16), RAMP[i]);
    }
    assert_eq!(r16(&a, 0x90), 0x2301);
    w8(&mut a, &mut s, 0x70, 0x40); // bank 1 plays, CPU sees bank 0 (still zero)
    for i in 0..16 {
        assert_eq!(a.read_io8(0x90 + i as u16), 0);
    }
    fill_ram(&mut a, &mut s, &[0xAA; 16]);
    w8(&mut a, &mut s, 0x70, 0x00);
    assert_eq!(a.read_io8(0x95), RAMP[5], "bank 1 contents intact");
    assert_eq!(a.wave.ram[0], [0xAA; 16]);
}

#[test]
fn length_counter_256_units() {
    let (mut a, mut s) = apu_on();
    w8(&mut a, &mut s, 0x70, 0x80);
    w8(&mut a, &mut s, 0x72, 254); // 2 length ticks
    w8(&mut a, &mut s, 0x73, 0x20);
    w16(&mut a, &mut s, 0x74, 0xC000 | 1000);
    assert_eq!(status(&a) & 4, 4);
    frames(&mut a, 2);
    assert_eq!(status(&a) & 4, 4);
    frames(&mut a, 1);
    assert_eq!(status(&a) & 4, 0);
    // Length 0 => 256 ticks => expires on frame call 511 (starting from step 0).
    frames(&mut a, 5); // back to step 0
    w8(&mut a, &mut s, 0x72, 0);
    w8(&mut a, &mut s, 0x75, 0xC0 | (1000u16 >> 8) as u8);
    frames(&mut a, 510);
    assert_eq!(status(&a) & 4, 4);
    frames(&mut a, 1);
    assert_eq!(status(&a) & 4, 0);
}

#[test]
fn playback_bit_off_stops_channel() {
    let (mut a, mut s) = apu_on();
    w8(&mut a, &mut s, 0x70, 0x40);
    fill_ram(&mut a, &mut s, &[0xFF; 16]);
    w8(&mut a, &mut s, 0x70, 0x80);
    w8(&mut a, &mut s, 0x73, 0x20);
    w16(&mut a, &mut s, 0x74, 0x8000 | 2040);
    assert_eq!(status(&a) & 4, 4);
    assert_eq!(a.wave.output(), 15);
    w8(&mut a, &mut s, 0x70, 0x00);
    assert_eq!(status(&a) & 4, 0);
    assert_eq!(a.wave.output(), 0);
    // Restart with playback off does nothing.
    w8(&mut a, &mut s, 0x75, 0x87);
    assert_eq!(status(&a) & 4, 0);
}

#[test]
fn restart_rewinds_to_first_digit() {
    let (mut a, mut s) = apu_on();
    w8(&mut a, &mut s, 0x70, 0x40);
    fill_ram(&mut a, &mut s, &RAMP);
    w8(&mut a, &mut s, 0x70, 0x80);
    w8(&mut a, &mut s, 0x73, 0x20);
    w16(&mut a, &mut s, 0x74, 0x8000 | 2040);
    levels(&mut a, 10);
    assert_eq!(a.wave.output(), 10);
    w8(&mut a, &mut s, 0x75, 0x87);
    assert_eq!(a.wave.output(), 0);
    assert_eq!(levels(&mut a, 3), vec![1, 2, 3]);
}
