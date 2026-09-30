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
