//! Shared helpers for the BIOS HLE unit tests.

use crate::bios::Bios;
use crate::bus::{Access, Bus, MemoryBus};
use crate::cart::Cartridge;
use crate::cpu::regs::{Regs, MODE_SYS};

/// A bus with an empty 256-byte ROM and the HLE BIOS.
pub fn bus() -> Bus {
    let rom = vec![0u8; 0x100];
    Bus::new(Cartridge::new(rom, None).unwrap(), Bios::hle())
}

/// A bus whose ROM holds `rom` (padded to 256 bytes) and the HLE BIOS.
pub fn bus_with_rom(rom: &[u8]) -> Bus {
    let mut data = rom.to_vec();
    if data.len() < 0x100 {
        data.resize(0x100, 0);
    }
    Bus::new(Cartridge::new(data, None).unwrap(), Bios::hle())
}

pub fn poke(bus: &mut Bus, addr: u32, data: &[u8]) {
    for (i, &b) in data.iter().enumerate() {
        bus.write8(addr + i as u32, b, Access::N);
    }
}

pub fn peek(bus: &Bus, addr: u32, n: usize) -> Vec<u8> {
    (0..n).map(|i| bus.peek8(addr + i as u32)).collect()
}

/// Issue `swi num` from System mode at ROM address 0x08000000 with r0-r3 = `r`.
pub fn swi(bus: &mut Bus, num: u8, r: [u32; 4]) -> Regs {
    let mut regs = Regs::new();
    regs.set_cpsr(MODE_SYS);
    regs.r[..4].copy_from_slice(&r);
    regs.r[15] = 0x0800_0004;
    assert!(bus.hle_swi(num, &mut regs));
    regs
}
