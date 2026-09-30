//! EEPROM 512 B / 8 KB serial backup. Source: GBATEK "GBA Cart Backup EEPROM".
//!
//! Protocol (bits arrive via 16-bit DMA writes, LSB of each halfword):
//!   read request : `11` + address (6 or 14 bits) + `0`
//!   write request: `10` + address (6 or 14 bits) + 64 data bits + `0`
//! Read data: 4 dummy bits then 64 data bits, MSB first, via 16-bit reads.
//! Size is inferred from the first request's length (9 bits => 512 B, 17 bits => 8 KB).

#[cfg_attr(feature = "savestate", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Mode {
    Idle,
    /// Collecting request bits.
    Request,
    /// Read request accepted: streaming out data.
    ReadOut,
}

#[cfg_attr(feature = "savestate", derive(serde::Serialize, serde::Deserialize))]
pub struct Eeprom {
    data: Vec<u8>,
    /// None until the first request fixes the size.
    size_known: bool,
    mode: Mode,
    /// Request bits accumulated MSB-first.
    bits: u128,
    nbits: u32,
    /// Address (in 64-bit blocks) for the current read.
    read_addr: usize,
    /// Bits remaining in the read stream (68 total: 4 dummy + 64).
    read_bits_left: u32,
    /// Expected request length for reads / writes once the size is known.
    addr_bits: u32,
}

impl Eeprom {
    pub fn new(_large_rom: bool) -> Self {
        Eeprom {
            data: vec![0xFF; 0x2000],
            size_known: false,
            mode: Mode::Idle,
            bits: 0,
            nbits: 0,
            read_addr: 0,
            read_bits_left: 0,
            addr_bits: 14,
        }
    }

    /// 512 B or 8 KB depending on what was detected (8 KB buffer until known).
    pub fn data(&self) -> &[u8] {
        if self.size_known && self.addr_bits == 6 {
            &self.data[..0x200]
        } else {
            &self.data
        }
    }

    pub fn load(&mut self, d: &[u8]) {
        let n = d.len().min(self.data.len());
        self.data[..n].copy_from_slice(&d[..n]);
        if d.len() <= 0x200 {
            self.size_known = true;
            self.addr_bits = 6;
        } else {
            self.size_known = true;
            self.addr_bits = 14;
        }
    }

    /// The DMA transfer length is the strongest size hint: 9 or 73 halfwords => 512 B,
    /// 17 or 81 => 8 KB. Called by the bus/DMA when a DMA targets the EEPROM region.
    pub fn hint_transfer_len(&mut self, halfwords: u32) {
        if self.size_known {
            return;
        }
        match halfwords {
            9 | 73 => {
                self.addr_bits = 6;
                self.size_known = true;
            }
            17 | 81 => {
                self.addr_bits = 14;
                self.size_known = true;
            }
            _ => {}
        }
    }

    /// Feed one request bit. Returns true if data was modified.
    pub fn write_bit(&mut self, bit: u16) -> bool {
        if self.mode == Mode::ReadOut {
            // A write while streaming out terminates the read.
            self.mode = Mode::Idle;
            self.nbits = 0;
            self.bits = 0;
        }
        self.mode = Mode::Request;
        self.bits = (self.bits << 1) | (bit & 1) as u128;
        self.nbits += 1;
        if self.nbits < 2 {
            return false;
        }
        let kind = (self.bits >> (self.nbits - 2)) & 3;
        let addr_bits = self.addr_bits;
        match kind {
            0b11 => {
                // Read: 2 + addr + 1 bits.
                if self.nbits == 2 + addr_bits + 1 {
                    let addr = ((self.bits >> 1) & ((1u128 << addr_bits) - 1)) as usize;
                    self.read_addr = addr & 0x3FF;
                    self.read_bits_left = 68;
                    self.mode = Mode::ReadOut;
                    self.nbits = 0;
                    self.bits = 0;
                }
                false
            }
            0b10 => {
                // Write: 2 + addr + 64 + 1 bits.
                if self.nbits == 2 + addr_bits + 64 + 1 {
                    let addr = ((self.bits >> 65) & ((1u128 << addr_bits) - 1)) as usize & 0x3FF;
                    let data = ((self.bits >> 1) & (u64::MAX as u128)) as u64;
                    let base = addr * 8;
                    if base + 8 <= self.data.len() {
                        self.data[base..base + 8].copy_from_slice(&data.to_be_bytes());
                    }
                    self.mode = Mode::Idle;
                    self.nbits = 0;
                    self.bits = 0;
                    return true;
                }
                false
            }
            _ => {
                // Invalid prefix: drop.
                self.mode = Mode::Idle;
                self.nbits = 0;
                self.bits = 0;
                false
            }
        }
    }

    /// Read one bit of the response stream. Idle reads return 1 ("ready").
    pub fn read_bit(&mut self) -> u16 {
        if self.mode != Mode::ReadOut || self.read_bits_left == 0 {
            return 1;
        }
        self.read_bits_left -= 1;
        if self.read_bits_left >= 64 {
            return 0; // 4 dummy bits
        }
        let base = self.read_addr * 8;
        let bit_index = 63 - self.read_bits_left; // 0 = MSB of the 64-bit block
        let byte = self.data.get(base + (bit_index / 8) as usize).copied().unwrap_or(0xFF);
        let v = (byte >> (7 - (bit_index % 8))) & 1;
        if self.read_bits_left == 0 {
            self.mode = Mode::Idle;
        }
        v as u16
    }
}
