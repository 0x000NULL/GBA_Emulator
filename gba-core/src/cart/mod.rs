//! Cartridge: ROM, mirroring, save-type detection, backup media. Source: GBATEK "GBA Cartridges".

pub mod eeprom;
pub mod flash;
pub mod header;
#[cfg(test)]
mod tests;

use crate::CoreError;
use eeprom::Eeprom;
use flash::Flash;
pub use header::Header;

pub const ROM_MAX: usize = 32 * 1024 * 1024;
pub const SRAM_SIZE: usize = 0x8000;

#[cfg_attr(feature = "savestate", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SaveType {
    None,
    Sram32K,
    Flash64K,
    Flash128K,
    /// Size (512 B or 8 KB) resolved lazily from the first DMA access.
    Eeprom,
}

#[cfg_attr(feature = "savestate", derive(serde::Serialize, serde::Deserialize))]
pub enum SaveBackend {
    None,
    Sram(#[cfg_attr(feature = "savestate", serde(with = "crate::state::bytes_box"))] Box<[u8; SRAM_SIZE]>),
    Flash(Flash),
    Eeprom(Eeprom),
}

#[cfg_attr(feature = "savestate", derive(serde::Serialize, serde::Deserialize))]
pub struct Cartridge {
    #[cfg_attr(feature = "savestate", serde(skip))]
    rom: Vec<u8>,
    /// `addr & rom_mask` indexes the ROM when `mirror` is set; otherwise out-of-range reads are open bus.
    rom_mask: u32,
    /// ROM mirrored across the whole 0x08000000-0x09FFFFFF space (Classic NES Series, game code F***).
    pub mirror: bool,
    pub header: Header,
    pub save_type: SaveType,
    pub save: SaveBackend,
    /// Set when save memory changed since the last `clear_dirty`.
    pub dirty: bool,
}

impl Cartridge {
    pub fn new(rom: Vec<u8>, save_override: Option<SaveType>) -> Result<Self, CoreError> {
        if rom.len() > ROM_MAX {
            return Err(CoreError::RomTooLarge(rom.len()));
        }
        if rom.len() < 0xC0 {
            return Err(CoreError::RomTooSmall(rom.len()));
        }
        let header = Header::parse(&rom);
        let mirror = header.game_code[0] == b'F';
        let save_type = save_override.unwrap_or_else(|| Self::detect_save_type(&rom, &header));
        let rom_mask = rom.len().next_power_of_two() as u32 - 1;
        let save = match save_type {
            SaveType::None => SaveBackend::None,
            SaveType::Sram32K => SaveBackend::Sram(vec![0xFFu8; SRAM_SIZE].into_boxed_slice().try_into().unwrap()),
            SaveType::Flash64K => SaveBackend::Flash(Flash::new(false)),
            SaveType::Flash128K => SaveBackend::Flash(Flash::new(true)),
            SaveType::Eeprom => SaveBackend::Eeprom(Eeprom::new(rom.len() > 16 * 1024 * 1024)),
        };
        Ok(Cartridge { rom, rom_mask, mirror, header, save_type, save, dirty: false })
    }

    pub fn rom(&self) -> &[u8] {
        &self.rom
    }

    /// Detach the ROM (savestate reattachment).
    pub fn take_rom(&mut self) -> Vec<u8> {
        std::mem::take(&mut self.rom)
    }

    pub fn set_rom(&mut self, rom: Vec<u8>) {
        self.rom = rom;
    }

    /// Scan for the SDK backup ID string. First match wins; game code F*** forces EEPROM.
    /// Recognised: EEPROM_V, SRAM_V, SRAM_F_V, FLASH_V, FLASH512_V, FLASH1M_V.
    pub fn detect_save_type(rom: &[u8], header: &Header) -> SaveType {
        if header.game_code[0] == b'F' {
            return SaveType::Eeprom;
        }
        let pats: [(&[u8], SaveType); 6] = [
            (b"EEPROM_V", SaveType::Eeprom),
            (b"SRAM_V", SaveType::Sram32K),
            (b"SRAM_F_V", SaveType::Sram32K),
            (b"FLASH_V", SaveType::Flash64K),
            (b"FLASH512_V", SaveType::Flash64K),
            (b"FLASH1M_V", SaveType::Flash128K),
        ];
        let mut best: Option<(usize, SaveType)> = None;
        // ID strings are word-aligned in the SDK libraries.
        for (pat, ty) in pats {
            if let Some(pos) = find_aligned(rom, pat) {
                if best.map_or(true, |(p, _)| pos < p) {
                    best = Some((pos, ty));
                }
            }
        }
        best.map(|(_, t)| t).unwrap_or(SaveType::None)
    }

    // ------------------------------------------------------------ ROM region (0x08-0x0D)

    #[inline]
    fn rom_index(&self, addr: u32) -> Option<usize> {
        let off = addr & 0x01FF_FFFF;
        if self.mirror {
            Some((off & self.rom_mask) as usize % self.rom.len())
        } else if (off as usize) < self.rom.len() {
            Some(off as usize)
        } else {
            None
        }
    }

    /// Out-of-range ROM reads return the address bits (GBATEK: "the ROM address/2 is returned").
    #[inline]
    fn rom_open_bus16(addr: u32) -> u16 {
        ((addr & 0x01FF_FFFF) >> 1) as u16
    }

    pub fn peek8(&self, addr: u32) -> u8 {
        match self.rom_index(addr) {
            Some(i) => self.rom[i],
            None => {
                let h = Self::rom_open_bus16(addr);
                if addr & 1 != 0 {
                    (h >> 8) as u8
                } else {
                    h as u8
                }
            }
        }
    }

    pub fn peek16(&self, addr: u32) -> u16 {
        let a = addr & !1;
        match self.rom_index(a) {
            Some(i) if i + 1 < self.rom.len() => u16::from_le_bytes([self.rom[i], self.rom[i + 1]]),
            _ => Self::rom_open_bus16(a),
        }
    }

    #[inline]
    fn is_eeprom_addr(&self, addr: u32) -> bool {
        if !matches!(self.save, SaveBackend::Eeprom(_)) {
            return false;
        }
        let region = addr >> 24;
        if region != 0x0D {
            return false;
        }
        // Carts >16 MB only map EEPROM at 0x0DFFFF00-0x0DFFFFFF.
        self.rom.len() <= 16 * 1024 * 1024 || (addr & 0x01FF_FFFF) >= 0x01FF_FF00
    }

    pub fn read8(&mut self, addr: u32) -> u8 {
        if self.is_eeprom_addr(addr) {
            return self.eeprom_read() as u8;
        }
        self.peek8(addr)
    }

    pub fn read16(&mut self, addr: u32) -> u16 {
        if self.is_eeprom_addr(addr) {
            return self.eeprom_read();
        }
        self.peek16(addr)
    }

    pub fn read32(&mut self, addr: u32) -> u32 {
        let a = addr & !3;
        if self.is_eeprom_addr(a) {
            let lo = self.eeprom_read() as u32;
            let hi = self.eeprom_read() as u32;
            return lo | hi << 16;
        }
        self.peek16(a) as u32 | (self.peek16(a + 2) as u32) << 16
    }

    pub fn write8(&mut self, addr: u32, v: u8) {
        self.write16(addr, v as u16);
    }

    pub fn write16(&mut self, addr: u32, v: u16) {
        if self.is_eeprom_addr(addr) {
            if let SaveBackend::Eeprom(e) = &mut self.save {
                if e.write_bit(v & 1) {
                    self.dirty = true;
                }
            }
        }
        // ROM writes are otherwise ignored (GPIO/RTC not emulated: none of the target games use it).
    }

    /// Called by the DMA engine for transfers into the EEPROM region (halfword count of the request).
    pub fn eeprom_dma_hint(&mut self, halfwords: u32) {
        if let SaveBackend::Eeprom(e) = &mut self.save {
            e.hint_transfer_len(halfwords);
        }
    }

    fn eeprom_read(&mut self) -> u16 {
        match &mut self.save {
            SaveBackend::Eeprom(e) => e.read_bit() as u16,
            _ => 1,
        }
    }

    // ------------------------------------------------------------ SRAM/Flash region (0x0E-0x0F)

    pub fn save_peek8(&self, addr: u32) -> u8 {
        match &self.save {
            SaveBackend::Sram(s) => s[(addr & 0x7FFF) as usize],
            SaveBackend::Flash(f) => f.peek8(addr as u16),
            // No SRAM/flash chip: the pulled-up 8-bit bus reads 0xFF (jsmolka save/none.gba).
            _ => 0xFF,
        }
    }

    pub fn save_read8(&mut self, addr: u32) -> u8 {
        match &mut self.save {
            SaveBackend::Sram(s) => s[(addr & 0x7FFF) as usize],
            SaveBackend::Flash(f) => f.read8(addr as u16),
            _ => 0xFF,
        }
    }

    pub fn save_write8(&mut self, addr: u32, v: u8) {
        match &mut self.save {
            SaveBackend::Sram(s) => {
                s[(addr & 0x7FFF) as usize] = v;
                self.dirty = true;
            }
            SaveBackend::Flash(f) => {
                if f.write8(addr as u16, v) {
                    self.dirty = true;
                }
            }
            _ => {}
        }
    }

    // ------------------------------------------------------------ persistence

    /// Raw save contents for `<rom>.sav`, or None if the cart has no backup media.
    pub fn save_data(&self) -> Option<Vec<u8>> {
        match &self.save {
            SaveBackend::None => None,
            SaveBackend::Sram(s) => Some(s.to_vec()),
            SaveBackend::Flash(f) => Some(f.data().to_vec()),
            SaveBackend::Eeprom(e) => Some(e.data().to_vec()),
        }
    }

    pub fn load_save(&mut self, data: &[u8]) {
        match &mut self.save {
            SaveBackend::None => {}
            SaveBackend::Sram(s) => {
                let n = data.len().min(SRAM_SIZE);
                s[..n].copy_from_slice(&data[..n]);
            }
            SaveBackend::Flash(f) => f.load(data),
            SaveBackend::Eeprom(e) => e.load(data),
        }
        self.dirty = false;
    }
}

fn find_aligned(hay: &[u8], pat: &[u8]) -> Option<usize> {
    (0..hay.len().saturating_sub(pat.len())).step_by(4).find(|&i| &hay[i..i + pat.len()] == pat)
}
