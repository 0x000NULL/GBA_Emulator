//! Cartridge unit tests: save-type detection, SRAM, Flash command set, EEPROM protocol.

use super::*;

fn rom_with(id: &[u8], code: &[u8; 4], size: usize) -> Vec<u8> {
    let mut rom = vec![0u8; size];
    rom[0xAC..0xB0].copy_from_slice(code);
    rom[0x1000..0x1000 + id.len()].copy_from_slice(id);
    rom
}

#[test]
fn detects_every_sdk_save_string() {
    let cases: [(&[u8], SaveType); 6] = [
        (b"EEPROM_V124", SaveType::Eeprom),
        (b"SRAM_V112", SaveType::Sram32K),
        (b"SRAM_F_V103", SaveType::Sram32K),
        (b"FLASH_V126", SaveType::Flash64K),
        (b"FLASH512_V131", SaveType::Flash64K),
        (b"FLASH1M_V103", SaveType::Flash128K),
    ];
    for (id, want) in cases {
        let c = Cartridge::new(rom_with(id, b"AXXE", 0x4000), None).unwrap();
        assert_eq!(c.save_type, want, "{}", String::from_utf8_lossy(id));
        assert!(!c.mirror);
    }
    let c = Cartridge::new(rom_with(b"", b"AXXE", 0x4000), None).unwrap();
    assert_eq!(c.save_type, SaveType::None);
    // Game code F*** forces EEPROM and ROM mirroring even without an ID string.
    let c = Cartridge::new(rom_with(b"SRAM_V112", b"FZLE", 0x4000), None).unwrap();
    assert_eq!(c.save_type, SaveType::Eeprom);
    assert!(c.mirror);
    // Unaligned occurrences are ignored (SDK strings are word aligned).
    let mut rom = vec![0u8; 0x4000];
    rom[0xAC..0xB0].copy_from_slice(b"AXXE");
    rom[0x1001..0x1001 + 9].copy_from_slice(b"SRAM_V112");
    assert_eq!(Cartridge::new(rom, None).unwrap().save_type, SaveType::None);
}

#[test]
fn rom_mirroring_for_classic_nes() {
    let mut rom = rom_with(b"", b"FZLE", 0x10_0000);
    rom[0x1234] = 0xAA;
    let c = Cartridge::new(rom, None).unwrap();
    assert_eq!(c.peek8(0x0800_1234), 0xAA);
    assert_eq!(c.peek8(0x0810_1234), 0xAA, "1 MB ROM mirrored");
    assert_eq!(c.peek8(0x09F0_1234), 0xAA);
    let c = Cartridge::new(rom_with(b"", b"AXXE", 0x10_0000), None).unwrap();
    assert_eq!(c.peek16(0x0810_1234), (0x0010_1234u32 >> 1) as u16, "no mirroring: open bus");
}

#[test]
fn sram_reads_ff_when_blank_and_persists() {
    let mut c = Cartridge::new(rom_with(b"SRAM_V112", b"AXXE", 0x4000), None).unwrap();
    assert_eq!(c.save_read8(0x0E00_0000), 0xFF);
    c.save_write8(0x0E00_1234, 0x42);
    assert!(c.dirty);
    assert_eq!(c.save_read8(0x0E00_1234), 0x42);
    assert_eq!(c.save_read8(0x0E00_9234), 0x42, "32K mirrored across the 64K window");
    assert_eq!(c.save_read8(0x0F00_1234), 0x42, "and across 0x0F");
    let data = c.save_data().unwrap();
    assert_eq!(data.len(), SRAM_SIZE);
    assert_eq!(data[0x1234], 0x42);
    let mut c2 = Cartridge::new(rom_with(b"SRAM_V112", b"AXXE", 0x4000), None).unwrap();
    c2.load_save(&data);
    assert_eq!(c2.save_read8(0x0E00_1234), 0x42);
    assert!(!c2.dirty);
}

fn flash_cmd(c: &mut Cartridge, cmd: u8) {
    c.save_write8(0x0E00_5555, 0xAA);
    c.save_write8(0x0E00_2AAA, 0x55);
    c.save_write8(0x0E00_5555, cmd);
}

#[test]
fn flash64_id_program_erase() {
    let mut c = Cartridge::new(rom_with(b"FLASH_V126", b"AXXE", 0x4000), None).unwrap();
    flash_cmd(&mut c, 0x90);
    assert_eq!(c.save_read8(0x0E00_0000), 0xBF, "SST manufacturer");
    assert_eq!(c.save_read8(0x0E00_0001), 0xD4);
    flash_cmd(&mut c, 0xF0);
    assert_eq!(c.save_read8(0x0E00_0000), 0xFF, "blank after leaving ID mode");
    // Program a byte.
    flash_cmd(&mut c, 0xA0);
    c.save_write8(0x0E00_0100, 0x5A);
    assert_eq!(c.save_read8(0x0E00_0100), 0x5A);
    // Programming can only clear bits.
    flash_cmd(&mut c, 0xA0);
    c.save_write8(0x0E00_0100, 0xA5);
    assert_eq!(c.save_read8(0x0E00_0100), 0x00);
    // Sector erase 0x0000-0x0FFF restores 0xFF.
    flash_cmd(&mut c, 0x80);
    c.save_write8(0x0E00_5555, 0xAA);
    c.save_write8(0x0E00_2AAA, 0x55);
    c.save_write8(0x0E00_0000, 0x30);
    assert_eq!(c.save_read8(0x0E00_0100), 0xFF);
    // Chip erase.
    flash_cmd(&mut c, 0xA0);
    c.save_write8(0x0E00_8000, 0x00);
    flash_cmd(&mut c, 0x80);
    flash_cmd(&mut c, 0x10);
    assert_eq!(c.save_read8(0x0E00_8000), 0xFF);
    assert_eq!(c.save_data().unwrap().len(), 0x1_0000);
}

#[test]
fn flash128_banks() {
    let mut c = Cartridge::new(rom_with(b"FLASH1M_V103", b"AXXE", 0x4000), None).unwrap();
    flash_cmd(&mut c, 0x90);
    assert_eq!(c.save_read8(0x0E00_0000), 0x62, "Sanyo");
    assert_eq!(c.save_read8(0x0E00_0001), 0x13);
    flash_cmd(&mut c, 0xF0);
    flash_cmd(&mut c, 0xA0);
    c.save_write8(0x0E00_0010, 0x11);
    // Switch to bank 1 and program the same offset.
    flash_cmd(&mut c, 0xB0);
    c.save_write8(0x0E00_0000, 1);
    assert_eq!(c.save_read8(0x0E00_0010), 0xFF);
    flash_cmd(&mut c, 0xA0);
    c.save_write8(0x0E00_0010, 0x22);
    assert_eq!(c.save_read8(0x0E00_0010), 0x22);
    flash_cmd(&mut c, 0xB0);
    c.save_write8(0x0E00_0000, 0);
    assert_eq!(c.save_read8(0x0E00_0010), 0x11);
    let d = c.save_data().unwrap();
    assert_eq!(d.len(), 0x2_0000);
    assert_eq!(d[0x10], 0x11);
    assert_eq!(d[0x1_0010], 0x22);
}

/// Feed a request bit string (MSB first) through 16-bit writes.
fn eeprom_send(c: &mut Cartridge, bits: &[u8]) {
    for &b in bits {
        c.write16(0x0D00_0000, b as u16);
    }
}

fn eeprom_read_block(c: &mut Cartridge) -> u64 {
    for _ in 0..4 {
        c.read16(0x0D00_0000);
    }
    let mut v = 0u64;
    for _ in 0..64 {
        v = (v << 1) | (c.read16(0x0D00_0000) & 1) as u64;
    }
    v
}

fn addr_bits(addr: u16, n: usize) -> Vec<u8> {
    (0..n).rev().map(|i| ((addr >> i) & 1) as u8).collect()
}

#[test]
fn eeprom_8k_write_then_read() {
    let mut c = Cartridge::new(rom_with(b"EEPROM_V124", b"AXXE", 0x4000), None).unwrap();
    if let SaveBackend::Eeprom(e) = &mut c.save {
        e.hint_transfer_len(17);
    }
    let value: u64 = 0x0123_4567_89AB_CDEF;
    let mut req = vec![1u8, 0];
    req.extend(addr_bits(0x123, 14));
    req.extend((0..64).rev().map(|i| ((value >> i) & 1) as u8));
    req.push(0);
    eeprom_send(&mut c, &req);
    assert!(c.dirty);
    let mut rd = vec![1u8, 1];
    rd.extend(addr_bits(0x123, 14));
    rd.push(0);
    eeprom_send(&mut c, &rd);
    assert_eq!(eeprom_read_block(&mut c), value);
    // Untouched block reads all ones.
    let mut rd = vec![1u8, 1];
    rd.extend(addr_bits(0x124, 14));
    rd.push(0);
    eeprom_send(&mut c, &rd);
    assert_eq!(eeprom_read_block(&mut c), u64::MAX);
    assert_eq!(c.save_data().unwrap().len(), 0x2000);
}

#[test]
fn eeprom_512_uses_6_bit_addresses() {
    let mut c = Cartridge::new(rom_with(b"EEPROM_V122", b"AXXE", 0x4000), None).unwrap();
    if let SaveBackend::Eeprom(e) = &mut c.save {
        e.hint_transfer_len(9);
    }
    let value: u64 = 0xFEDC_BA98_7654_3210;
    let mut req = vec![1u8, 0];
    req.extend(addr_bits(0x3F, 6));
    req.extend((0..64).rev().map(|i| ((value >> i) & 1) as u8));
    req.push(0);
    eeprom_send(&mut c, &req);
    let mut rd = vec![1u8, 1];
    rd.extend(addr_bits(0x3F, 6));
    rd.push(0);
    eeprom_send(&mut c, &rd);
    assert_eq!(eeprom_read_block(&mut c), value);
    assert_eq!(c.save_data().unwrap().len(), 0x200);
}

#[test]
fn eeprom_window_on_32mb_cart() {
    let mut rom = rom_with(b"EEPROM_V124", b"AXXE", 32 * 1024 * 1024);
    rom[0x0100_0000] = 0x77;
    let mut c = Cartridge::new(rom, None).unwrap();
    assert_eq!(c.read8(0x0D00_0000), 0x77, "below the window this is ROM on a 32 MB cart");
    assert_eq!(c.read16(0x0DFF_FF00), 1, "EEPROM idle reads 1 inside the window");
}

// ---------------------------------------------------------------- GPIO: RTC and solar sensor

use crate::timing::CLOCK_HZ;
use rtc::days_from_civil;

const GPIO_DATA: u32 = 0x0800_00C4;
const GPIO_DIR: u32 = 0x0800_00C6;
const GPIO_CTRL: u32 = 0x0800_00C8;

/// A cart with the SDK RTC string (and a save string and game code).
fn rtc_cart(code: &[u8; 4], save_id: &[u8]) -> Cartridge {
    let mut rom = rom_with(save_id, code, 0x4000);
    rom[0x2000..0x2008].copy_from_slice(b"SIIRTC_V");
    Cartridge::new(rom, None).unwrap()
}

/// 2026-10-03 13:45:30 (a Saturday).
const SEED: i64 = 20729 * 86400 + 13 * 3600 + 45 * 60 + 30;

fn seed(c: &mut Cartridge, secs: i64) {
    let now = c.now;
    c.gpio.as_mut().unwrap().rtc.as_mut().unwrap().set_clock(secs, now);
}

/// One RTC transfer the way GBATEK's flowchart does it: CS low with SCK high, CS high, the
/// command MSB first, parameters LSB first, read bits sampled with SCK high, CS low.
fn rtc_xfer(c: &mut Cartridge, cmd: u8, write: &[u8], read: usize) -> Vec<u8> {
    c.write16(GPIO_CTRL, 1);
    c.write16(GPIO_DIR, 7);
    c.write16(GPIO_DATA, 1);
    c.write16(GPIO_DATA, 5);
    let send = |c: &mut Cartridge, b: u16| {
        c.write16(GPIO_DATA, 4 | b << 1);
        c.write16(GPIO_DATA, 5 | b << 1);
    };
    for i in (0..8).rev() {
        send(c, (cmd >> i & 1) as u16);
    }
    for &byte in write {
        for i in 0..8 {
            send(c, (byte >> i & 1) as u16);
        }
    }
    let mut out = Vec::new();
    if read > 0 {
        c.write16(GPIO_DIR, 5);
        for _ in 0..read {
            let mut byte = 0u8;
            for i in 0..8 {
                c.write16(GPIO_DATA, 4);
                c.write16(GPIO_DATA, 5);
                byte |= ((c.read16(GPIO_DATA) >> 1 & 1) as u8) << i;
            }
            out.push(byte);
        }
        c.write16(GPIO_DIR, 7);
    }
    c.write16(GPIO_DATA, 1);
    out
}

#[test]
fn detects_gpio_devices() {
    let c = rtc_cart(b"BPEE", b"FLASH1M_V103");
    assert_eq!(c.gpio.as_ref().unwrap().devices(), GpioDevices { rtc: true, solar: false });
    assert_eq!(c.save_type, SaveType::Flash128K);
    let c = Cartridge::new(rom_with(b"EEPROM_V124", b"U3IE", 0x4000), None).unwrap();
    assert_eq!(c.gpio.as_ref().unwrap().devices(), GpioDevices { rtc: true, solar: true });
    let c = Cartridge::new(rom_with(b"FLASH1M_V103", b"BPRE", 0x4000), None).unwrap();
    assert!(c.gpio.is_none(), "no RTC string, no GPIO");
}

#[test]
fn gpio_reads_rom_until_read_write_mode() {
    let mut rom = rom_with(b"", b"AXVE", 0x4000);
    rom[0x2000..0x2008].copy_from_slice(b"SIIRTC_V");
    rom[0xC4..0xCA].copy_from_slice(&[0xFF; 6]);
    let mut c = Cartridge::new(rom, None).unwrap();
    c.write16(GPIO_DIR, 0xB);
    assert_eq!(c.read16(GPIO_DATA), 0xFFFF, "write-only: the ROM bytes show through");
    assert_eq!(c.read16(GPIO_DIR), 0xFFFF);
    c.write16(GPIO_CTRL, 1);
    assert_eq!(c.read16(GPIO_DIR), 0xB);
    assert_eq!(c.read16(GPIO_CTRL), 1);
    assert_eq!(c.read8(GPIO_DIR), 0xB);
    assert_eq!(c.read32(GPIO_DIR), 0x000B_0000, "word-aligned: data, then direction");
    c.write16(GPIO_DATA, 0xFFFF);
    assert_eq!(c.read16(GPIO_DATA), 0xB, "output pins read back, 4 bits only; input SIO idles low");
    // The rest of the ROM is untouched.
    assert_eq!(c.read16(0x0800_2000), u16::from_le_bytes(*b"SI"));
}

#[test]
fn rtc_reads_seeded_datetime_and_control() {
    let mut c = rtc_cart(b"BPEE", b"FLASH1M_V103");
    seed(&mut c, SEED);
    assert_eq!(rtc_xfer(&mut c, 0x63, &[], 1), [0x40], "control: 24h mode, no power failure");
    assert_eq!(rtc_xfer(&mut c, 0x65, &[], 7), [0x26, 0x10, 0x03, 5, 0x93, 0x45, 0x30]);
    assert_eq!(rtc_xfer(&mut c, 0x67, &[], 3), [0x93, 0x45, 0x30]);
    // Emulated time moves the clock.
    c.now += 90 * CLOCK_HZ;
    assert_eq!(rtc_xfer(&mut c, 0x67, &[], 3), [0x93, 0x47, 0x00]);
    // Unused registers read FFh; a bad fixed code is ignored (SIO stays low).
    assert_eq!(rtc_xfer(&mut c, 0x6F, &[], 1), [0xFF]);
    assert_eq!(rtc_xfer(&mut c, 0x25, &[], 1), [0x00]);
}

#[test]
fn rtc_write_datetime_keeps_running_and_persists() {
    let mut c = rtc_cart(b"AXVE", b"FLASH1M_V103");
    seed(&mut c, SEED);
    c.dirty = false;
    rtc_xfer(&mut c, 0x64, &[0x30, 0x12, 0x31, 2, 0x23, 0x59, 0x50], 0);
    assert!(c.dirty, "a new time is saved with the game");
    c.now += 10 * CLOCK_HZ;
    assert_eq!(rtc_xfer(&mut c, 0x65, &[], 7), [0x31, 0x01, 0x01, 3, 0x00, 0x00, 0x00]);

    // The offset rides in the .sav trailer: a fresh cart seeded with a host time an hour later
    // reads the game's clock an hour on.
    let sav = c.save_data().unwrap();
    assert_eq!(sav.len(), 0x20000 + 16);
    assert_eq!(&sav[0x20000..0x20006], b"SBRTC\x01");
    let mut d = rtc_cart(b"AXVE", b"FLASH1M_V103");
    d.load_save(&sav);
    seed(&mut d, SEED + 3600 + 10);
    assert_eq!(rtc_xfer(&mut d, 0x65, &[], 7), [0x31, 0x01, 0x01, 3, 0x01, 0x00, 0x00]);

    // Writing only the time keeps the date.
    rtc_xfer(&mut d, 0x66, &[0x08, 0x15, 0x00], 0);
    assert_eq!(rtc_xfer(&mut d, 0x65, &[], 7), [0x31, 0x01, 0x01, 3, 0x08, 0x15, 0x00]);
}

#[test]
fn rtc_12_hour_mode_and_force_reset() {
    let mut c = rtc_cart(b"BR4J", b"FLASH_V126");
    seed(&mut c, SEED);
    rtc_xfer(&mut c, 0x62, &[0x00], 0);
    assert_eq!(rtc_xfer(&mut c, 0x63, &[], 1), [0x00]);
    assert_eq!(rtc_xfer(&mut c, 0x67, &[], 3), [0x81, 0x45, 0x30], "1 PM: hour 1, AM/PM in bit 7");
    // 12h write: 11 PM, then back to 24h.
    rtc_xfer(&mut c, 0x66, &[0x91, 0x00, 0x00], 0);
    rtc_xfer(&mut c, 0x62, &[0x40], 0);
    assert_eq!(rtc_xfer(&mut c, 0x67, &[], 3), [0xA3, 0x00, 0x00]);
    // Force reset: 2000-01-01 00:00:00, control 00h.
    rtc_xfer(&mut c, 0x60, &[], 0);
    assert_eq!(rtc_xfer(&mut c, 0x63, &[], 1), [0x00]);
    assert_eq!(rtc_xfer(&mut c, 0x65, &[], 7), [0x00, 0x01, 0x01, 0, 0x00, 0x00, 0x00]);
}

#[test]
fn rtc_trailer_is_split_off_before_eeprom_sizing() {
    let mut c = rtc_cart(b"U3IE", b"EEPROM_V124");
    let mut sav = vec![0x5Au8; 512];
    sav.extend_from_slice(b"SBRTC\x01");
    sav.extend_from_slice(&3600i64.to_le_bytes());
    sav.extend_from_slice(&[0, 0x40]);
    c.load_save(&sav);
    assert_eq!(c.save_data().unwrap(), sav, "512 B EEPROM kept, trailer restored as written");
    // A plain .sav without a trailer loads as before (and gains one on the next write).
    let mut d = rtc_cart(b"U3IE", b"EEPROM_V124");
    d.load_save(&[0x5A; 512]);
    assert_eq!(d.save_data().unwrap().len(), 512 + 16);
    // A cart without a clock strips a trailer and ignores it.
    let mut e = Cartridge::new(rom_with(b"EEPROM_V124", b"AXXE", 0x4000), None).unwrap();
    e.load_save(&sav);
    assert_eq!(e.save_data().unwrap(), vec![0x5A; 512]);
    assert_eq!(days_from_civil(2000, 1, 1), 10957);
    assert_eq!(rtc::civil_from_days(20729), (2026, 10, 3));
    assert_eq!(days_from_civil(2026, 10, 3), 20729);
}

/// GBATEK's solar sampling loop: reset the counter, clock it until FLG goes high.
fn solar_sample(c: &mut Cartridge) -> u32 {
    c.write16(GPIO_CTRL, 1);
    c.write16(GPIO_DIR, 7);
    c.write16(GPIO_DATA, 2);
    c.write16(GPIO_DATA, 0);
    let mut r0 = 0;
    loop {
        c.write16(GPIO_DATA, 1);
        c.write16(GPIO_DATA, 0);
        if c.read16(GPIO_DATA) & 8 != 0 || r0 == 0x100 {
            return r0;
        }
        r0 += 1;
    }
}

#[test]
fn solar_sensor_ramp() {
    let mut c = Cartridge::new(rom_with(b"EEPROM_V124", b"U32E", 0x4000), None).unwrap();
    for (level, adc) in [(0, 0xE8), (5, 0x9C), (10, 0x50)] {
        c.gpio.as_mut().unwrap().solar.as_mut().unwrap().set_level(level);
        assert_eq!(solar_sample(&mut c), adc, "level {level}");
    }
    c.gpio.as_mut().unwrap().solar.as_mut().unwrap().set_level(200);
    assert_eq!(solar_sample(&mut c), 0x50, "levels clamp at the top");
}
