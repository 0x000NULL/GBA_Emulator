//! Game pak prefetch buffer: concrete cycle counts. WAITCNT = 4000h keeps the default ROM
//! waitstates (WS0 N = 4, S = 2, so a halfword costs 5 non-sequential and 3 sequential) and turns
//! the buffer on.

use super::*;
use crate::bios::Bios;
use crate::cart::Cartridge;

const ROM: u32 = 0x0800_0000;

fn bus(waitcnt: u16) -> Bus {
    let mut b = Bus::new(Cartridge::new(vec![0u8; 0x4_0000], None).unwrap(), Bios::hle());
    b.write16(0x0400_0204, waitcnt, Access::N);
    b
}

/// Cycles taken by `f`.
fn cost(b: &mut Bus, f: impl FnOnce(&mut Bus)) -> u64 {
    let t0 = b.now();
    f(b);
    b.now() - t0
}

fn f16(b: &mut Bus, addr: u32, a: Access) -> u64 {
    cost(b, |b| {
        b.fetch16(addr, a);
    })
}

fn f32c(b: &mut Bus, addr: u32, a: Access) -> u64 {
    cost(b, |b| {
        b.fetch32(addr, a);
    })
}

#[test]
fn disabled_buffer_charges_the_plain_waitstates() {
    let mut b = bus(0);
    assert_eq!(f16(&mut b, ROM, Access::N), 5);
    b.idle(20);
    assert_eq!(f16(&mut b, ROM + 2, Access::S), 3, "idle time buys nothing without the buffer");
    assert_eq!(f32c(&mut b, ROM + 4, Access::S), 6);
}

#[test]
fn back_to_back_sequential_fetches_run_at_bus_speed() {
    let mut b = bus(0x4000);
    assert_eq!(f16(&mut b, ROM, Access::N), 5, "miss: N access");
    // The prefetcher started the next halfword the moment the CPU's access ended, so the CPU
    // waits out its full S time: same as no buffer.
    assert_eq!(f16(&mut b, ROM + 2, Access::S), 3);
    assert_eq!(f16(&mut b, ROM + 4, Access::S), 3);
    assert_eq!(f32c(&mut b, ROM + 6, Access::S), 6, "ARM opcode = two halfwords");
}

#[test]
fn internal_cycles_fill_the_buffer_and_hits_cost_one_cycle() {
    let mut b = bus(0x4000);
    assert_eq!(f16(&mut b, ROM, Access::N), 5);
    // 6 idle cycles: halfwords land at +3 and +6; a third is in flight until +9.
    b.idle(6);
    assert_eq!(f16(&mut b, ROM + 2, Access::S), 1);
    assert_eq!(f16(&mut b, ROM + 4, Access::S), 1);
    // Third halfword lands 1 cycle from now (in flight), then the next costs a full S time.
    assert_eq!(f16(&mut b, ROM + 6, Access::S), 1);
    assert_eq!(f16(&mut b, ROM + 8, Access::S), 3);
}

#[test]
fn arm_opcode_comes_out_of_the_buffer_in_one_cycle() {
    let mut b = bus(0x4000);
    assert_eq!(f32c(&mut b, ROM, Access::N), 8, "N + S");
    // 10 idle cycles: three halfwords buffered, the fourth lands 2 cycles later.
    b.idle(10);
    assert_eq!(f32c(&mut b, ROM + 4, Access::S), 1, "both halves buffered: one cycle");
    // One halfword buffered, the other lands next cycle: the pair completes when it does.
    assert_eq!(f32c(&mut b, ROM + 8, Access::S), 1);
    // Empty buffer: two halfwords back to back.
    assert_eq!(f32c(&mut b, ROM + 12, Access::S), 6);
}

#[test]
fn branch_flushes_the_buffer() {
    let mut b = bus(0x4000);
    assert_eq!(f16(&mut b, ROM, Access::N), 5);
    b.idle(30);
    assert_eq!(f16(&mut b, ROM + 0x100, Access::N), 5, "target is not the buffer head");
    b.idle(6);
    assert_eq!(f16(&mut b, ROM + 0x102, Access::S), 1, "the buffer restarts behind the target");
}

#[test]
fn buffer_holds_eight_halfwords() {
    let mut b = bus(0x4000);
    assert_eq!(f16(&mut b, ROM, Access::N), 5);
    b.idle(200);
    let total: u64 = (1..=8).map(|i| f16(&mut b, ROM + 2 * i, Access::S)).sum();
    assert_eq!(total, 8, "eight buffered halfwords, one cycle each");
}

#[test]
fn prefetch_does_not_cross_a_128k_block() {
    let mut b = bus(0x4000);
    assert_eq!(f16(&mut b, ROM + 0x1_FFFC, Access::N), 5);
    b.idle(50);
    assert_eq!(f16(&mut b, ROM + 0x1_FFFE, Access::S), 1, "last halfword of the block is buffered");
    assert_eq!(
        f16(&mut b, ROM + 0x2_0000, Access::N),
        5,
        "the first halfword of the next block is a fresh non-sequential access"
    );
}

#[test]
fn data_access_to_rom_drops_the_buffer() {
    let mut b = bus(0x4000);
    assert_eq!(f16(&mut b, ROM, Access::N), 5);
    b.idle(12);
    // Data load from the pak: N cost, and the buffered opcodes are gone.
    assert_eq!(cost(&mut b, |b| {
        b.read16(ROM + 0x1000, Access::N);
    }), 5);
    assert_eq!(f16(&mut b, ROM + 2, Access::N), 5, "buffer was invalidated: plain miss");
}

#[test]
fn data_access_waits_only_for_a_transfer_in_its_last_cycle() {
    // The halfword after the fetch is in flight for 3 cycles.
    let mut b = bus(0x4000);
    f16(&mut b, ROM, Access::N);
    b.idle(1); // 2 cycles to go: cancelled for free
    assert_eq!(cost(&mut b, |b| {
        b.read16(ROM + 0x1000, Access::N);
    }), 5);

    let mut b = bus(0x4000);
    f16(&mut b, ROM, Access::N);
    b.idle(2); // 1 cycle to go: the CPU waits for it
    assert_eq!(cost(&mut b, |b| {
        b.read16(ROM + 0x1000, Access::N);
    }), 6);

    // An access on the cycle the prefetcher would start a halfword simply wins the bus.
    let mut b = bus(0x4000);
    f16(&mut b, ROM, Access::N);
    assert_eq!(cost(&mut b, |b| {
        b.read16(ROM + 0x1000, Access::N);
    }), 5);
}

#[test]
fn accesses_to_other_regions_do_not_disturb_the_buffer() {
    let mut b = bus(0x4000);
    assert_eq!(f16(&mut b, ROM, Access::N), 5);
    b.read32(0x0200_0000, Access::N); // EWRAM: 6 cycles, the pak bus keeps prefetching
    b.write16(0x0300_0000, 0, Access::N); // IWRAM
    assert_eq!(f16(&mut b, ROM + 2, Access::S), 1);
}

#[test]
fn fetch_from_ram_drops_the_buffer() {
    let mut b = bus(0x4000);
    assert_eq!(f16(&mut b, ROM, Access::N), 5);
    b.idle(10);
    assert_eq!(f16(&mut b, 0x0300_0000, Access::N), 1);
    assert_eq!(f16(&mut b, ROM + 2, Access::S), 3, "sequential to the old head, but the buffer is gone");
}

#[test]
fn clearing_waitcnt_bit_14_drops_the_buffer() {
    let mut b = bus(0x4000);
    assert_eq!(f16(&mut b, ROM, Access::N), 5);
    b.idle(10);
    b.write16(0x0400_0204, 0, Access::N);
    assert_eq!(f16(&mut b, ROM + 2, Access::S), 3);
}

#[test]
fn faster_waitstates_shorten_the_prefetch_interval() {
    // WS0 N = 3 (4 cycles), S = 1 (2 cycles).
    let mut b = bus(0x4000 | (1 << 4) | (1 << 2));
    assert_eq!(f16(&mut b, ROM, Access::N), 4);
    b.idle(2); // one halfword buffered, the next lands at +4
    assert_eq!(f16(&mut b, ROM + 2, Access::S), 1);
    assert_eq!(f16(&mut b, ROM + 4, Access::S), 1, "lands on the next cycle");
    assert_eq!(f16(&mut b, ROM + 6, Access::S), 2, "empty buffer: the S time of a fresh halfword");
}

#[test]
fn dma_waits_a_cycle_for_a_halfword_under_way() {
    let mut b = bus(0x4000);
    f16(&mut b, ROM, Access::N);
    b.idle(1);
    b.in_dma = true;
    assert_eq!(cost(&mut b, |b| {
        b.write16(ROM + 0x1000, 0, Access::N);
    }), 6);
    b.in_dma = false;
}

#[cfg(feature = "savestate")]
#[test]
fn prefetch_state_survives_a_savestate_round_trip() {
    let mut b = bus(0x4000);
    f16(&mut b, ROM, Access::N);
    b.idle(6);
    let bytes = bincode::serialize(&b.pf).unwrap();
    let pf: Prefetch = bincode::deserialize(&bytes).unwrap();
    b.pf = pf;
    assert_eq!(f16(&mut b, ROM + 2, Access::S), 1);
}
