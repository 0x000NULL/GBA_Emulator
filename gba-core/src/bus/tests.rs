//! Bus unit tests: mirroring, write quirks, waitstates, IO, DMA execution, timers through IO.

use super::*;
use crate::bios::Bios;
use crate::cart::Cartridge;
use crate::irq::Irq;

fn bus_with_rom(rom: Vec<u8>) -> Bus {
    Bus::new(Cartridge::new(rom, None).unwrap(), Bios::hle())
}

fn bus() -> Bus {
    bus_with_rom(vec![0u8; 0x100])
}

#[test]
fn ewram_and_iwram_mirror() {
    let mut b = bus();
    b.write32(0x0200_0000, 0xDEAD_BEEF, Access::N);
    assert_eq!(b.read32(0x0204_0000, Access::N), 0xDEAD_BEEF);
    assert_eq!(b.read32(0x02FC_0000, Access::N), 0xDEAD_BEEF);
    b.write16(0x0300_7FFE, 0x1234, Access::N);
    assert_eq!(b.read16(0x0300_FFFE, Access::N), 0x1234);
    assert_eq!(b.read16(0x03FF_FFFE, Access::N), 0x1234);
}

#[test]
fn vram_mirrors_96k_into_128k() {
    let mut b = bus();
    b.write16(0x0601_0000, 0xABCD, Access::N);
    assert_eq!(b.read16(0x0601_8000, Access::N), 0xABCD, "0x18000-0x1FFFF mirrors 0x10000-0x17FFF");
    b.write16(0x0600_0002, 0x5555, Access::N);
    assert_eq!(b.read16(0x0602_0002, Access::N), 0x5555, "128K block repeats");
}

#[test]
fn byte_writes_duplicate_or_are_ignored() {
    let mut b = bus();
    b.write8(0x0500_0001, 0x7F, Access::N);
    assert_eq!(b.read16(0x0500_0000, Access::N), 0x7F7F, "palette byte write fills the halfword");
    b.write8(0x0600_0000, 0x42, Access::N);
    assert_eq!(b.read16(0x0600_0000, Access::N), 0x4242, "BG VRAM byte write fills the halfword");
    b.write8(0x0601_0000, 0x42, Access::N);
    assert_eq!(b.read16(0x0601_0000, Access::N), 0, "OBJ VRAM byte write ignored in mode 0");
    // In bitmap modes the OBJ region starts at 0x14000, so 0x10000 accepts byte writes.
    b.write16(0x0400_0000, 3, Access::N);
    b.write8(0x0601_0000, 0x42, Access::N);
    assert_eq!(b.read16(0x0601_0000, Access::N), 0x4242);
    b.write8(0x0601_4000, 0x42, Access::N);
    assert_eq!(b.read16(0x0601_4000, Access::N), 0);
    b.write8(0x0700_0000, 0x99, Access::N);
    assert_eq!(b.read16(0x0700_0000, Access::N), 0, "OAM byte write ignored");
}

#[test]
fn waitstates_default_waitcnt() {
    let mut b = bus();
    let t0 = b.sched.now;
    b.read32(0x0300_0000, Access::N);
    assert_eq!(b.sched.now - t0, 1, "IWRAM 32-bit = 1 cycle");
    let t0 = b.sched.now;
    b.read32(0x0200_0000, Access::N);
    assert_eq!(b.sched.now - t0, 6, "EWRAM 32-bit = 2x3 cycles");
    let t0 = b.sched.now;
    b.read16(0x0200_0000, Access::N);
    assert_eq!(b.sched.now - t0, 3);
    let t0 = b.sched.now;
    b.read16(0x0800_0000, Access::N);
    assert_eq!(b.sched.now - t0, 5, "ROM WS0 N = 4 waits + 1");
    let t0 = b.sched.now;
    b.read16(0x0800_0002, Access::S);
    assert_eq!(b.sched.now - t0, 3, "ROM WS0 S = 2 waits + 1");
    let t0 = b.sched.now;
    b.read32(0x0800_0000, Access::N);
    assert_eq!(b.sched.now - t0, 8, "ROM 32-bit N = N + S");
    let t0 = b.sched.now;
    b.read8(0x0E00_0000, Access::N);
    assert_eq!(b.sched.now - t0, 5, "SRAM default 4 waits + 1");
    // WAITCNT = 0x4317 (common game setting): SRAM 8, WS0 N=3 S=1, WS1 N=4 S=4... check WS0.
    b.write16(0x0400_0204, 0x4317, Access::N);
    let t0 = b.sched.now;
    b.read16(0x0800_0000, Access::N);
    assert_eq!(b.sched.now - t0, 4);
    let t0 = b.sched.now;
    b.read16(0x0800_0002, Access::S);
    assert_eq!(b.sched.now - t0, 2);
    let t0 = b.sched.now;
    b.read8(0x0E00_0000, Access::N);
    assert_eq!(b.sched.now - t0, 9);
}

#[test]
fn rom_out_of_range_reads_address_bits() {
    let mut rom = vec![0u8; 0x200];
    rom[0x100] = 0x34;
    rom[0x101] = 0x12;
    let mut b = bus_with_rom(rom);
    assert_eq!(b.read16(0x0800_0100, Access::N), 0x1234);
    assert_eq!(b.read16(0x0800_0300, Access::N), (0x300 >> 1) as u16);
    assert_eq!(b.read32(0x0900_0000, Access::N), ((0x0100_0000u32 >> 1) & 0xFFFF) | ((((0x0100_0002u32) >> 1) & 0xFFFF) << 16));
    assert_eq!(b.read16(0x0A00_0300, Access::N), (0x300 >> 1) as u16, "same in every ROM mirror");
}

#[test]
fn irq_registers_and_acknowledge() {
    let mut b = bus();
    b.write16(0x0400_0200, 0x0009, Access::N);
    assert_eq!(b.irq.ie, 0x0009);
    b.write16(0x0400_0208, 1, Access::N);
    assert!(b.irq.ime);
    b.irq.raise(Irq::VBlank);
    b.irq.raise(Irq::Timer0);
    assert_eq!(b.read16(0x0400_0202, Access::N), 0x0009);
    assert!(b.irq_pending());
    b.write16(0x0400_0202, 0x0001, Access::N);
    assert_eq!(b.irq.if_, 0x0008, "writing 1 acknowledges only that bit");
    b.write16(0x0400_0200, 0x0001, Access::N);
    assert!(!b.irq_pending());
    assert!(!b.irq.wake_condition());
}

#[test]
fn immediate_dma_copies_and_raises_irq() {
    let mut b = bus();
    for i in 0..16u32 {
        b.write32(0x0200_0000 + i * 4, 0x1000 + i, Access::N);
    }
    b.write32(0x0400_00D4, 0x0200_0000, Access::N); // DMA3SAD
    b.write32(0x0400_00D8, 0x0300_0000, Access::N); // DMA3DAD
    b.write16(0x0400_00DC, 16, Access::N); // count
    b.write16(0x0400_00DE, 0x8000 | 0x4000 | 0x0400, Access::N); // enable, irq, 32-bit, immediate
    assert_eq!(b.dma.pending, 0, "immediate DMA waits 2 cycles");
    b.sched.now += 2;
    b.process_events();
    assert_eq!(b.dma.pending, 1 << 3);
    b.dma_run();
    for i in 0..16u32 {
        assert_eq!(b.read32(0x0300_0000 + i * 4, Access::N), 0x1000 + i);
    }
    assert_eq!(b.irq.if_ & Irq::Dma3 as u16, Irq::Dma3 as u16);
    assert_eq!(b.read16(0x0400_00DE, Access::N) & 0x8000, 0, "non-repeat DMA disables itself");
}

#[test]
fn dma_fixed_destination_fill_and_decrement_source() {
    let mut b = bus();
    for i in 0..4u32 {
        b.write16(0x0200_0000 + i * 2, 0xA000 + i as u16, Access::N);
    }
    // src decrement from the last halfword, dst fixed.
    b.write32(0x0400_00B0, 0x0200_0006, Access::N);
    b.write32(0x0400_00B4, 0x0300_0100, Access::N);
    b.write16(0x0400_00B8, 4, Access::N);
    b.write16(0x0400_00BA, 0x8000 | (2 << 5) | (1 << 7), Access::N);
    b.sched.now += 2;
    b.process_events();
    b.dma_run();
    assert_eq!(b.read16(0x0300_0100, Access::N), 0xA000, "last written = first source element (decrementing)");
}

#[test]
fn hblank_dma_repeats_and_reloads_destination() {
    let mut b = bus();
    b.write32(0x0400_00C8, 0x0200_0000, Access::N); // DMA2
    b.write32(0x0400_00CC, 0x0500_0000, Access::N);
    b.write16(0x0400_00D0, 2, Access::N);
    // enable, repeat, hblank timing, dst increment+reload (3), 16-bit
    b.write16(0x0400_00D2, 0x8000 | 0x0200 | (2 << 12) | (3 << 5), Access::N);
    assert_eq!(b.dma.pending, 0, "not immediate");
    b.write16(0x0200_0000, 0x1111, Access::N);
    b.write16(0x0200_0002, 0x2222, Access::N);
    b.dma.trigger(DmaTrigger::HBlank);
    b.dma_run();
    assert_eq!(b.read16(0x0500_0000, Access::N), 0x1111);
    assert_eq!(b.read16(0x0500_0002, Access::N), 0x2222);
    assert!(b.dma.ch[2].enabled(), "repeat keeps it enabled");
    assert_eq!(b.dma.ch[2].dst, 0x0500_0000, "destination reloaded");
    assert_eq!(b.dma.ch[2].src, 0x0200_0004, "source continues");
}

#[test]
fn fifo_dma_only_serves_its_fifo() {
    let mut b = bus();
    b.write32(0x0400_00BC, 0x0200_0000, Access::N); // DMA1 -> FIFO A
    b.write32(0x0400_00C0, 0x0400_00A0, Access::N);
    b.write16(0x0400_00C6, 0x8000 | 0x0200 | (3 << 12) | (2 << 5) | 0x0400, Access::N);
    b.dma.trigger(DmaTrigger::FifoB);
    assert_eq!(b.dma.pending, 0);
    b.dma.trigger(DmaTrigger::FifoA);
    assert_eq!(b.dma.pending, 1 << 1);
    b.dma_run();
    assert_eq!(b.apu.fifo_a.len(), 16, "4 words pushed");
    assert!(b.dma.ch[1].enabled());
}

#[test]
fn timer_through_io_counts_and_interrupts() {
    let mut b = bus();
    b.write16(0x0400_0100, 0xFF00, Access::N); // reload
    b.write16(0x0400_0102, 0x00C1, Access::N); // enable, irq, prescale 64
    let start = b.sched.now;
    // 0x100 ticks * 64 cycles = 0x4000 cycles until overflow (+1 start latency).
    b.sched.now = start + 0x2000;
    assert_eq!(b.read16(0x0400_0100, Access::N), 0xFF00 + 0x2000 / 64 - 0, "counter derived from time");
    b.sched.now = start + 0x4000 + 1;
    b.process_events();
    assert_eq!(b.irq.if_ & Irq::Timer0 as u16, Irq::Timer0 as u16);
    assert_eq!(b.read16(0x0400_0100, Access::N), 0xFF00, "reloaded");
}

#[test]
fn keypad_reads_inverted_and_raises_irq() {
    let mut b = bus();
    assert_eq!(b.read16(0x0400_0130, Access::N), 0x03FF);
    b.write16(0x0400_0132, 0x4000 | crate::keypad::Keys::START, Access::N);
    let Bus { keypad, irq, .. } = &mut b;
    keypad.set_keys(crate::keypad::Keys(crate::keypad::Keys::START | crate::keypad::Keys::A), irq);
    assert_eq!(b.read16(0x0400_0130, Access::N), 0x03FF & !(crate::keypad::Keys::START | crate::keypad::Keys::A));
    assert_eq!(b.irq.if_ & Irq::Keypad as u16, Irq::Keypad as u16);
}

#[test]
fn mgba_debug_registers_print() {
    let mut b = bus();
    b.write16(0x04FF_F780, 0xC0DE, Access::N);
    assert_eq!(b.read16(0x04FF_F780, Access::N), 0x1DEA);
    for (i, &c) in b"END: 3/3".iter().enumerate() {
        b.write8(0x04FF_F600 + i as u32, c, Access::N);
    }
    b.write16(0x04FF_F700, 0x0104, Access::N);
    assert_eq!(b.debug.log, vec!["[4] END: 3/3".to_string()]);
}

#[test]
fn bios_region_reads_open_bus_from_outside() {
    let mut b = bus();
    b.last_fetch = 0x0800_0000;
    assert_eq!(b.read32(0x0000_0000, Access::N), crate::bios::BIOS_OPENBUS_BOOT);
    assert_eq!(b.read32(0x0000_1234, Access::N), crate::bios::BIOS_OPENBUS_BOOT);
    b.bios.open_bus = crate::bios::BIOS_OPENBUS_AFTER_SWI;
    assert_eq!(b.read16(0x0000_0002, Access::N), (crate::bios::BIOS_OPENBUS_AFTER_SWI >> 16) as u16);
}

/// GBATEK "Unpredictable Things", unused/write-only I/O: open bus when the whole 32-bit word has no
/// readable half, zero otherwise. The io-read suite arranges 0xDEADDEAD as the prefetched opcode.
#[test]
fn io_write_only_and_unused_reads() {
    let mut b = bus();
    b.open_bus = 0xDEAD_BEEF;
    // Whole word write-only: BGxHOFS/VOFS, affine, WINxH/V, MOSAIC+4E, BLDY+56, 58-5E, FIFO, DMA SAD/DAD.
    for off in [0x10u32, 0x14, 0x20, 0x28, 0x3C, 0x40, 0x44, 0x4C, 0x54, 0x58, 0x5C, 0x8C, 0xA0, 0xA4, 0xA8, 0xAC, 0xB0, 0xB4, 0xD4, 0xD8, 0xE0, 0xFC, 0x110, 0x12C, 0x138, 0x144, 0x15C, 0x20C, 0x304] {
        b.write16(0x0400_0000 + off, 0xFFFF, Access::N);
        b.write16(0x0400_0000 + off + 2, 0xFFFF, Access::N);
        assert_eq!(b.read16(0x0400_0000 + off, Access::N), 0xBEEF, "{off:03X}: low half of the open-bus word");
        assert_eq!(b.read16(0x0400_0000 + off + 2, Access::N), 0xDEAD, "{off:03X}: high half of the open-bus word");
        assert_eq!(b.read32(0x0400_0000 + off, Access::N), 0xDEAD_BEEF);
        assert_eq!(b.read8(0x0400_0000 + off + 1, Access::N), 0xBE, "{off:03X}: byte lane");
        assert_eq!(b.read8(0x0400_0000 + off + 3, Access::N), 0xDE);
    }
    // Word with a readable half: the unreadable half is zero (66h, 6Ah, 86h, 8Ah, DMAxCNT_L, 136h,
    // 142h, 15Ah, 206h, 20Ah, 302h).
    for off in [0x66u32, 0x6A, 0x6E, 0x76, 0x7A, 0x7E, 0x86, 0x8A, 0xB8, 0xC4, 0xD0, 0xDC, 0x136, 0x142, 0x15A, 0x206, 0x20A, 0x302] {
        b.write16(0x0400_0000 + off, 0xFFFF, Access::N);
        assert_eq!(b.read16(0x0400_0000 + off, Access::N), 0, "{off:03X}");
        assert_eq!(b.read8(0x0400_0000 + off, Access::N), 0, "{off:03X}");
    }
    // Beyond the register block nothing answers, except the 4000800h port.
    assert_eq!(b.read16(0x0400_100C, Access::N), 0xBEEF);
    assert_eq!(b.read32(0x0400_0800, Access::N), b.imc);
    // Readable registers still read.
    b.write16(0x0400_0008, 0x1234, Access::N);
    assert_eq!(b.read16(0x0400_0008, Access::N), 0x1234);
}

/// Unused bits of readable LCD registers have no storage (io-read suite masks).
#[test]
fn io_lcd_register_masks() {
    let mut b = bus();
    for (off, mask) in [(0x08u32, 0xDFFFu16), (0x0A, 0xDFFF), (0x0C, 0xFFFF), (0x0E, 0xFFFF), (0x48, 0x3F3F), (0x4A, 0x3F3F), (0x50, 0x3FFF), (0x52, 0x1F1F)] {
        b.write16(0x0400_0000 + off, 0xFFFF, Access::N);
        assert_eq!(b.read16(0x0400_0000 + off, Access::N), mask, "{off:02X}");
    }
}

/// A normal-mode serial transfer through the bus: scheduled from the SIOCNT write, completes via
/// `process_events`, raises the serial IRQ.
#[test]
fn sio_normal_transfer_through_bus() {
    let mut b = bus();
    b.sio.reset();
    b.write16(0x0400_0134, 0, Access::N);
    b.write32(0x0400_0128, 0xAA55_5003, Access::N);
    assert_eq!(b.read16(0x0400_012A, Access::N), 0xAA55);
    assert!(!b.sio.busy);
    b.write16(0x0400_0128, 0x5083, Access::N);
    let t0 = b.sched.now;
    assert!(b.sio.busy, "16-bit write starts the transfer with the 32-bit length in view");
    assert_eq!(b.sched.time_of(Event::SioComplete), Some(t0 + 32 * 8));
    b.sched.now = t0 + 32 * 8 - 1;
    b.process_events();
    assert_eq!(b.read16(0x0400_0128, Access::N) & 0x80, 0x80);
    b.sched.now = t0 + 32 * 8;
    b.process_events();
    assert_eq!(b.read16(0x0400_0128, Access::N), 0x5007);
    assert_eq!(b.read32(0x0400_0120, Access::N), 0xFFFF_FFFF);
    assert_eq!(b.irq.if_ & Irq::Serial as u16, Irq::Serial as u16);
    // Byte writes reach the same model.
    b.write8(0x0400_0128, 0x81, Access::N);
    assert!(b.sio.busy);
    assert_eq!(b.sched.time_of(Event::SioComplete), Some(b.sched.now + 32 * 64));
/// Program `ch` for an immediate transfer of `count` units and enable it (the start is due 2
/// cycles later).
fn start_dma(b: &mut Bus, ch: u32, src: u32, dst: u32, count: u16, cnt_h: u16) {
    let base = 0x0400_00B0 + ch * 12;
    b.write32(base, src, Access::N);
    b.write32(base + 4, dst, Access::N);
    b.write16(base + 8, count, Access::N);
    b.write16(base + 10, 0x8000 | cnt_h, Access::N);
}

/// Run every scheduled immediate start and the resulting transfers.
fn finish_dma(b: &mut Bus) {
    b.sched.now += 2;
    b.process_events();
    b.dma_run();
}

#[test]
fn due_immediate_dma_takes_the_bus_before_a_cpu_data_access() {
    let mut b = bus();
    b.write32(0x0200_0000, 0xA5B6_C7D8, Access::N);
    b.write32(0x0300_0000, 0xAA55_BB66, Access::N);
    start_dma(&mut b, 3, 0x0200_0000, 0x0300_0000, 1, 0x0400);
    let t = b.sched.now;
    // One cycle later the start latency has not elapsed: the CPU still sees the old word.
    assert_eq!(b.read32(0x0300_0000, Access::N), 0xAA55_BB66);
    assert_eq!(b.sched.now, t + 1);
    // An access on the cycle the latency ends (t+2) still belongs to the CPU.
    assert_eq!(b.read32(0x0300_0000, Access::N), 0xAA55_BB66);
    assert_eq!(b.dma.pending, 0);
    // From t+3 on the DMA owns the bus first and the read returns the copied word.
    assert_eq!(b.read32(0x0300_0000, Access::N), 0xA5B6_C7D8);
    assert_eq!(b.dma.pending, 0);
    assert_eq!(b.dma.next_start, u64::MAX, "start bookkeeping cleared");
    assert_eq!(b.sched.time_of(crate::sched::Event::DmaStart(3)), None, "scheduled start cancelled");
    // The normal event path still works when no CPU access intervenes.
    b.write32(0x0200_0000, 0x1234_5678, Access::N);
    start_dma(&mut b, 3, 0x0200_0000, 0x0300_0004, 1, 0x0400);
    finish_dma(&mut b);
    assert_eq!(b.peek32(0x0300_0004), 0x1234_5678);
    assert_eq!(b.dma.next_start, u64::MAX);
}

#[test]
fn dma_reads_of_bios_and_unused_areas_return_the_channel_latch() {
    let mut b = bus();
    b.write32(0x0200_0000, 0xFEED_FACE, Access::N);
    b.write16(0x0200_0010, 0xBABE, Access::N);
    // A 32-bit copy from EWRAM loads the latch of DMA3.
    start_dma(&mut b, 3, 0x0200_0000, 0x0300_0000, 1, 0x0400);
    finish_dma(&mut b);
    assert_eq!(b.dma.ch[3].latch, 0xFEED_FACE);
    // Reading the BIOS region (not the CPU's BIOS open-bus value) and the unused area re-use it.
    start_dma(&mut b, 3, 0x0000_0010, 0x0300_0010, 1, 0x0400);
    finish_dma(&mut b);
    assert_eq!(b.peek32(0x0300_0010), 0xFEED_FACE);
    start_dma(&mut b, 3, 0x0100_0000, 0x0300_0014, 1, 0);
    finish_dma(&mut b);
    assert_eq!(b.peek16(0x0300_0014), 0xFACE, "16-bit reads take the low half");
    assert_eq!(b.dma.ch[3].latch, 0xFEED_FACE, "unmapped reads leave the latch alone");
    // A 16-bit read stores the halfword replicated, so a later 32-bit unused read yields it twice.
    start_dma(&mut b, 3, 0x0200_0010, 0x0300_0020, 1, 0);
    finish_dma(&mut b);
    assert_eq!(b.dma.ch[3].latch, 0xBABE_BABE);
    start_dma(&mut b, 3, 0x0000_0000, 0x0300_0024, 1, 0x0400);
    finish_dma(&mut b);
    assert_eq!(b.peek32(0x0300_0024), 0xBABE_BABE);
    // The latch is per channel: DMA0 has not transferred anything yet.
    start_dma(&mut b, 0, 0x0000_0000, 0x0300_0028, 1, 0x0400);
    finish_dma(&mut b);
    assert_eq!(b.peek32(0x0300_0028), 0);
}

#[test]
fn dma0_source_is_27_bits_so_rom_reads_hit_the_unused_area() {
    let mut rom = vec![0u8; 0x200];
    rom[0x100..0x104].copy_from_slice(&0xDEAD_BEEFu32.to_le_bytes());
    let mut b = bus_with_rom(rom);
    b.write32(0x0200_0000, 0xFEED_FACE, Access::N);
    start_dma(&mut b, 0, 0x0200_0000, 0x0300_0000, 1, 0x0400);
    finish_dma(&mut b);
    start_dma(&mut b, 0, 0x0800_0100, 0x0300_0004, 1, 0x0400);
    finish_dma(&mut b);
    assert_eq!(b.peek32(0x0300_0004), 0xFEED_FACE, "DMA0 cannot read the game pak");
    start_dma(&mut b, 1, 0x0800_0100, 0x0300_0008, 1, 0x0400);
    finish_dma(&mut b);
    assert_eq!(b.peek32(0x0300_0008), 0xDEAD_BEEF, "DMA1 can");
}

#[test]
fn dma_rom_source_always_advances_whatever_the_source_control() {
    let mut rom = vec![0u8; 0x200];
    for i in 0..8u32 {
        rom[0x100 + i as usize * 2..0x102 + i as usize * 2].copy_from_slice(&(0xB000 + i as u16).to_le_bytes());
    }
    let mut b = bus_with_rom(rom);
    // Fixed source (src ctrl 2), fixed destination: the last unit written is the 4th halfword.
    start_dma(&mut b, 3, 0x0800_0100, 0x0300_0000, 4, (2 << 7) | (2 << 5));
    finish_dma(&mut b);
    assert_eq!(b.peek16(0x0300_0000), 0xB003);
    // Decrementing source: same, the cart bus only counts up.
    start_dma(&mut b, 3, 0x0800_0100, 0x0300_0002, 4, (1 << 7) | (2 << 5));
    finish_dma(&mut b);
    assert_eq!(b.peek16(0x0300_0002), 0xB003);
    // RAM sources honour the control: fixed re-reads the same halfword.
    b.write16(0x0200_0000, 0xCAFE, Access::N);
    b.write16(0x0200_0002, 0x1111, Access::N);
    start_dma(&mut b, 3, 0x0200_0000, 0x0300_0004, 4, (2 << 7) | (2 << 5));
    finish_dma(&mut b);
    assert_eq!(b.peek16(0x0300_0004), 0xCAFE);
}

#[test]
fn sram_reads_use_the_address_to_pick_the_byte_and_replicate_it() {
    let rom = vec![0u8; 0x100];
    let mut b = Bus::new(Cartridge::new(rom, Some(crate::cart::SaveType::Sram32K)).unwrap(), Bios::hle());
    for (i, &c) in b"Game".iter().enumerate() {
        b.write8(0x0E00_0000 + i as u32, c, Access::N);
    }
    assert_eq!(b.read16(0x0E00_0000, Access::N), 0x4747);
    assert_eq!(b.read16(0x0E00_0001, Access::N), 0x6161, "odd address selects byte 1 ('a')");
    assert_eq!(b.read32(0x0E00_0000, Access::N), 0x4747_4747);
    assert_eq!(b.read32(0x0E00_0002, Access::N), 0x6D6D_6D6D, "address+2 selects 'm'");
    assert_eq!(b.read32(0x0E00_0003, Access::N), 0x6565_6565);
    assert_eq!(b.read32(0x0F00_0003, Access::N), 0x6565_6565, "0x0F mirror");
    // Wider writes store the lane the address selects.
    b.write32(0x0E00_0001, 0xA5B6_C7D8, Access::N);
    assert_eq!(b.read8(0x0E00_0000, Access::N), b'G');
    assert_eq!(b.read8(0x0E00_0001, Access::N), 0xC7);
}
