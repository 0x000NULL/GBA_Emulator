//! Opcode-fetch sequentiality and block-transfer timing (mGBA suite "Timing tests", GBATEK
//! "ARM CPU Instruction Cycle Times" / "GBA GamePak Prefetch" / WAITCNT notes).

use super::*;

/// The opcode fetch that follows a data access or an internal cycle is non-sequential; the one
/// that follows a plain ALU op, or an instruction whose condition failed, stays sequential.
#[test]
fn arm_fetch_after_data_access_or_internal_cycle_is_nonsequential() {
    let mut cpu = arm_cpu();
    let mut bus = TestBus::new();
    cpu.regs.r[1] = DATA;
    cpu.regs.r[2] = 1;
    let code = [
        0xE581_0000, // str r0, [r1]            -> next fetch N (data access, "2N")
        0xE1A0_0000, // mov r0, r0              -> next fetch S
        0xE591_0000, // ldr r0, [r1]            -> next fetch N (internal cycle)
        0xE1A0_0000, // mov r0, r0              -> S
        0xE1A0_0210, // mov r0, r0, lsl r2      -> N (shift by register: 1I)
        0xE1A0_0000, // mov r0, r0              -> S
        0xE000_0291, // mul r0, r1, r2          -> N (mI)
        0xE1A0_0000, // mov r0, r0              -> S
        0x0581_0000, // streq r0, [r1] (Z clear: not executed) -> S
        0xE1A0_0000, // mov r0, r0              -> S
        0xE1A0_0000,
    ];
    run_arm(&mut cpu, &mut bus, &code, code.len() - 1);
    let expect: Vec<(u32, Access)> = vec![
        (CODE, Access::N),
        (CODE + 4, Access::S),
        (CODE + 8, Access::S), // fetched by the str: the pipeline was just refilled
        (CODE + 12, Access::N),
        (CODE + 16, Access::S),
        (CODE + 20, Access::N),
        (CODE + 24, Access::S),
        (CODE + 28, Access::N),
        (CODE + 32, Access::S),
        (CODE + 36, Access::N),
        (CODE + 40, Access::S),
        (CODE + 44, Access::S),
    ];
    assert_eq!(bus.fetches, expect);
}

#[test]
fn thumb_fetch_after_data_access_or_internal_cycle_is_nonsequential() {
    let mut cpu = thumb_cpu();
    let mut bus = TestBus::new();
    cpu.regs.r[1] = DATA;
    let code = [
        0x6008, // str r0, [r1]     -> next fetch N
        0x46C0, // mov r8, r8 (nop) -> S
        0x6808, // ldr r0, [r1]     -> N
        0x46C0, // nop              -> S
        0x4088, // lsl r0, r1       -> N (shift by register)
        0x46C0, // nop              -> S
        0x4348, // mul r0, r1       -> N
        0x46C0, // nop              -> S
        0x46C0,
    ];
    run_thumb(&mut cpu, &mut bus, &code, code.len() - 1);
    let expect: Vec<(u32, Access)> = vec![
        (CODE, Access::N),
        (CODE + 2, Access::S),
        (CODE + 4, Access::S),
        (CODE + 6, Access::N),
        (CODE + 8, Access::S),
        (CODE + 10, Access::N),
        (CODE + 12, Access::S),
        (CODE + 14, Access::N),
        (CODE + 16, Access::S),
        (CODE + 18, Access::N),
    ];
    assert_eq!(bus.fetches, expect);
}

/// A branch clears the "non-sequential" state: the refill is N then S regardless of what the
/// branch was preceded by, and the fetch after the refill is sequential again.
#[test]
fn branch_after_store_refills_n_then_s() {
    let mut cpu = arm_cpu();
    let mut bus = TestBus::new();
    cpu.regs.r[1] = DATA;
    bus.load_arm(0x2000, &[0xE1A0_0000, 0xE1A0_0000, 0xE1A0_0000]);
    // str r0, [r1] ; b 0x2000
    let b = 0xEA00_0000 | (((0x2000u32.wrapping_sub(CODE + 4 + 8)) >> 2) & 0x00FF_FFFF);
    run_arm(&mut cpu, &mut bus, &[0xE581_0000, b], 3);
    let n = bus.fetches.len();
    assert_eq!(bus.fetches[n - 4], (CODE + 12, Access::N)); // fetched by the branch, after the str
    assert_eq!(bus.fetches[n - 3], (0x2000, Access::N));
    assert_eq!(bus.fetches[n - 2], (0x2004, Access::S));
    assert_eq!(bus.fetches[n - 1], (0x2008, Access::S));
}

/// LDM/STM: the first word is N, the rest S, except that a word starting a 128K game pak block
/// is N again (this is where a transfer running off the end of OAM at 07FFFFFCh enters the ROM).
#[test]
fn block_transfer_restarts_nonsequential_at_128k_block() {
    // ldmia r1, {r2-r5} with r1 = 0x1FFF8
    let (_, bus) = arm1(0xE891_003C, |c, _| c.regs.r[1] = 0x1_FFF8);
    assert_eq!(
        bus.accesses,
        vec![
            (0x1_FFF8, 4, false, Access::N),
            (0x1_FFFC, 4, false, Access::S),
            (0x2_0000, 4, false, Access::N),
            (0x2_0004, 4, false, Access::S),
        ]
    );
    // stmia r1, {r2-r5}
    let (_, bus) = arm1(0xE881_003C, |c, _| c.regs.r[1] = 0x1_FFF8);
    assert_eq!(
        bus.accesses,
        vec![
            (0x1_FFF8, 4, true, Access::N),
            (0x1_FFFC, 4, true, Access::S),
            (0x2_0000, 4, true, Access::N),
            (0x2_0004, 4, true, Access::S),
        ]
    );
    // Thumb ldmia r1!, {r2-r5}
    let (_, bus) = thumb1(0xC93C, |c, _| c.regs.r[1] = 0x1_FFF8);
    assert_eq!(bus.accesses[1], (0x1_FFFC, 4, false, Access::S));
    assert_eq!(bus.accesses[2], (0x2_0000, 4, false, Access::N));
    // A transfer that does not cross a block boundary stays N, S, S, S.
    let (_, bus) = arm1(0xE891_003C, |c, _| c.regs.r[1] = 0x1_FFF0);
    assert!(bus.accesses[1..].iter().all(|a| a.3 == Access::S));
}

/// Opcode fetches at the first halfword of a 128K game pak block are non-sequential, both in the
/// running pipeline and in the second word of a refill.
#[test]
fn fetch_at_128k_block_start_is_nonsequential() {
    let nops = [0xE1A0_0000u32; 4];
    let mut cpu = arm_cpu();
    let mut bus = TestBus::new();
    cpu.regs.r[15] = 0x1_FFF4;
    bus.load_arm(0x1_FFF4, &nops);
    cpu.step(&mut bus);
    cpu.step(&mut bus);
    assert_eq!(
        bus.fetches,
        vec![
            (0x1_FFF4, Access::N),
            (0x1_FFF8, Access::S),
            (0x1_FFFC, Access::S),
            (0x2_0000, Access::N),
        ]
    );
    // Refill whose second word starts the block.
    let mut cpu = arm_cpu();
    let mut bus = TestBus::new();
    cpu.regs.r[15] = 0x1_FFFC;
    bus.load_arm(0x1_FFFC, &nops);
    cpu.step(&mut bus);
    assert_eq!(
        bus.fetches,
        vec![
            (0x1_FFFC, Access::N),
            (0x2_0000, Access::N),
            (0x2_0004, Access::S),
        ]
    );
    // Thumb: same rule per halfword.
    let mut cpu = thumb_cpu();
    let mut bus = TestBus::new();
    cpu.regs.r[15] = 0x1_FFFC;
    bus.load_thumb(0x1_FFFC, &[0x46C0; 4]);
    cpu.step(&mut bus);
    cpu.step(&mut bus);
    assert_eq!(
        bus.fetches,
        vec![
            (0x1_FFFC, Access::N),
            (0x1_FFFE, Access::S),
            (0x2_0000, Access::N),
            (0x2_0002, Access::S),
        ]
    );
}
