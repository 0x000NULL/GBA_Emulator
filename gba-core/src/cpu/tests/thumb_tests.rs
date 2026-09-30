//! THUMB formats 1-19: classification and behaviour.

use super::*;
use crate::cpu::decode::{decode_thumb, ThumbOp};

#[test]
fn decoder_classifies_every_format() {
    let cases: &[(u16, ThumbOp)] = &[
        (0x0000, ThumbOp::ShiftImm),   // lsl r0, r0, #0
        (0x1000, ThumbOp::ShiftImm),   // asr
        (0x1800, ThumbOp::AddSub),     // add r0, r0, r0
        (0x1E40, ThumbOp::AddSub),     // sub r0, r0, #1
        (0x2000, ThumbOp::Imm8),       // mov r0, #0
        (0x3FFF, ThumbOp::Imm8),       // sub r7, #255
        (0x4000, ThumbOp::Alu),        // and r0, r0
        (0x43C0, ThumbOp::Alu),        // mvn r0, r0
        (0x4400, ThumbOp::HiReg),      // add r0, r0
        (0x4770, ThumbOp::HiReg),      // bx lr
        (0x4800, ThumbOp::LdrPc),      // ldr r0, [pc]
        (0x5000, ThumbOp::LdStReg),    // str r0, [r0, r0]
        (0x5C00, ThumbOp::LdStReg),    // ldrb
        (0x5200, ThumbOp::LdStSigned), // strh r0, [r0, r0]
        (0x5E00, ThumbOp::LdStSigned), // ldsh
        (0x6000, ThumbOp::LdStImm),    // str r0, [r0]
        (0x7800, ThumbOp::LdStImm),    // ldrb
        (0x8000, ThumbOp::LdStHalf),   // strh
        (0x8800, ThumbOp::LdStHalf),   // ldrh
        (0x9000, ThumbOp::LdStSp),     // str r0, [sp]
        (0x9800, ThumbOp::LdStSp),     // ldr r0, [sp]
        (0xA000, ThumbOp::AddrPcSp),   // add r0, pc, #0
        (0xA800, ThumbOp::AddrPcSp),   // add r0, sp, #0
        (0xB000, ThumbOp::AddSp),      // add sp, #0
        (0xB080, ThumbOp::AddSp),      // sub sp, #0
        (0xB400, ThumbOp::PushPop),    // push {}
        (0xB500, ThumbOp::PushPop),    // push {lr}
        (0xBC00, ThumbOp::PushPop),    // pop {}
        (0xBD00, ThumbOp::PushPop),    // pop {pc}
        (0xB100, ThumbOp::Undefined),
        (0xBE00, ThumbOp::Undefined), // bkpt (ARMv5)
        (0xC000, ThumbOp::LdmStm),    // stmia r0!, {}
        (0xC800, ThumbOp::LdmStm),    // ldmia r0!, {}
        (0xD000, ThumbOp::BCond),     // beq
        (0xDD00, ThumbOp::BCond),     // ble
        (0xDE00, ThumbOp::Undefined), // cond 0xE
        (0xDF00, ThumbOp::Swi),       // swi 0
        (0xE000, ThumbOp::B),         // b
        (0xE800, ThumbOp::Undefined), // blx (ARMv5)
        (0xF000, ThumbOp::BlHi),
        (0xF800, ThumbOp::BlLo),
    ];
    for &(op, expect) in cases {
        assert_eq!(decode_thumb(op), expect, "opcode {op:04X}");
    }
}

// ------------------------------------------------------------------ builders

fn f1(op: u16, amount: u16, rs: u16, rd: u16) -> u16 {
    op << 11 | amount << 6 | rs << 3 | rd
}
fn f2(imm: bool, sub: bool, rn: u16, rs: u16, rd: u16) -> u16 {
    0x1800 | (imm as u16) << 10 | (sub as u16) << 9 | rn << 6 | rs << 3 | rd
}
fn f3(op: u16, rd: u16, imm: u16) -> u16 {
    0x2000 | op << 11 | rd << 8 | imm
}
fn f4(op: u16, rs: u16, rd: u16) -> u16 {
    0x4000 | op << 6 | rs << 3 | rd
}
fn f5(op: u16, rd: u16, rs: u16) -> u16 {
    0x4400 | op << 8 | ((rd >> 3) & 1) << 7 | ((rs >> 3) & 1) << 6 | (rs & 7) << 3 | (rd & 7)
}

#[test]
fn format1_shifts() {
    let (cpu, _) = thumb1(f1(0, 4, 1, 0), |c, _| c.regs.r[1] = 0xF000_0001);
    assert_eq!(cpu.regs.r[0], 0x0000_0010);
    assert_eq!(flags(&cpu), (false, false, true, false));
    // lsl #0 leaves carry alone.
    let (cpu, _) = thumb1(f1(0, 0, 1, 0), |c, _| {
        c.regs.r[1] = 0x8000_0000;
        c.regs.cpsr |= FLAG_C;
    });
    assert_eq!(flags(&cpu), (true, false, true, false));
    // lsr #0 = lsr #32.
    let (cpu, _) = thumb1(f1(1, 0, 1, 0), |c, _| c.regs.r[1] = 0x8000_0000);
    assert_eq!(cpu.regs.r[0], 0);
    assert_eq!(flags(&cpu), (false, true, true, false));
    // asr #0 = asr #32.
    let (cpu, _) = thumb1(f1(2, 0, 1, 0), |c, _| c.regs.r[1] = 0x8000_0000);
    assert_eq!(cpu.regs.r[0], 0xFFFF_FFFF);
    assert_eq!(flags(&cpu), (true, false, true, false));
}

#[test]
fn format2_add_sub() {
    let (cpu, _) = thumb1(f2(false, false, 2, 1, 0), |c, _| {
        c.regs.r[1] = 0x7FFF_FFFF;
        c.regs.r[2] = 1;
    });
    assert_eq!(cpu.regs.r[0], 0x8000_0000);
    assert_eq!(flags(&cpu), (true, false, false, true));
    let (cpu, _) = thumb1(f2(true, true, 3, 1, 0), |c, _| c.regs.r[1] = 3);
    assert_eq!(cpu.regs.r[0], 0);
    assert_eq!(flags(&cpu), (false, true, true, false));
    let (cpu, _) = thumb1(f2(true, false, 0, 1, 0), |c, _| c.regs.r[1] = 0xFFFF_FFFF);
    assert_eq!(cpu.regs.r[0], 0xFFFF_FFFF);
    assert_eq!(flags(&cpu), (true, false, false, false));
}

#[test]
fn format3_immediates() {
    let (cpu, _) = thumb1(f3(0, 0, 0), |c, _| c.regs.cpsr |= FLAG_C | FLAG_V);
    assert_eq!(flags(&cpu), (false, true, true, true)); // MOV touches only N,Z
    let (cpu, _) = thumb1(f3(1, 0, 5), |c, _| c.regs.r[0] = 5);
    assert_eq!(flags(&cpu), (false, true, true, false));
    let (cpu, _) = thumb1(f3(2, 0, 0xFF), |c, _| c.regs.r[0] = 1);
    assert_eq!(cpu.regs.r[0], 0x100);
    let (cpu, _) = thumb1(f3(3, 0, 1), |c, _| c.regs.r[0] = 0);
    assert_eq!(cpu.regs.r[0], 0xFFFF_FFFF);
    assert_eq!(flags(&cpu), (true, false, false, false));
}

#[test]
fn format4_alu() {
    // (op, rd, rs, carry_in, expected, expected flags NZCV)
    let cases: &[(u16, u32, u32, bool, u32, (bool, bool, bool, bool))] = &[
        (0x0, 0xF0, 0x3C, false, 0x30, (false, false, false, false)),
        (0x1, 0xF0, 0x3C, true, 0xCC, (false, false, true, false)),
        (0x2, 1, 32, false, 0, (false, true, true, false)), // lsl by 32 -> C = bit 0
        (0x2, 1, 33, true, 0, (false, true, false, false)),
        (0x3, 0x8000_0000, 31, false, 1, (false, false, false, false)),
        (0x3, 0x8000_0000, 32, false, 0, (false, true, true, false)),
        (
            0x4,
            0x8000_0000,
            40,
            false,
            0xFFFF_FFFF,
            (true, false, true, false),
        ),
        (0x5, 1, 1, true, 3, (false, false, false, false)),
        (0x6, 1, 1, false, 0xFFFF_FFFF, (true, false, false, false)),
        (
            0x7,
            0x8000_0001,
            1,
            false,
            0xC000_0000,
            (true, false, true, false),
        ),
        (
            0x7,
            0x8000_0001,
            0,
            false,
            0x8000_0001,
            (true, false, false, false),
        ), // ror 0: C unchanged
        (
            0x7,
            0x8000_0001,
            32,
            false,
            0x8000_0001,
            (true, false, true, false),
        ),
        (
            0x9,
            0xDEAD,
            1,
            false,
            0xFFFF_FFFF,
            (true, false, false, false),
        ), // neg
        (
            0x9,
            0xDEAD,
            0x8000_0000,
            false,
            0x8000_0000,
            (true, false, false, true),
        ),
        (0xC, 0xF0, 0x0F, false, 0xFF, (false, false, false, false)),
        (
            0xD,
            3,
            0xFFFF_FFFF,
            true,
            0xFFFF_FFFD,
            (true, false, true, false),
        ), // mul keeps C
        (0xE, 0xFF, 0x0F, false, 0xF0, (false, false, false, false)),
        (
            0xF,
            0xDEAD,
            0,
            false,
            0xFFFF_FFFF,
            (true, false, false, false),
        ),
    ];
    for &(op, rd, rs, c_in, expect, fl) in cases {
        let (cpu, _) = thumb1(f4(op, 1, 0), |c, _| {
            c.regs.r[0] = rd;
            c.regs.r[1] = rs;
            c.regs.set_flag(FLAG_C, c_in);
        });
        assert_eq!(cpu.regs.r[0], expect, "op {op:X}");
        assert_eq!(flags(&cpu), fl, "op {op:X} flags");
    }
    // tst / cmp / cmn do not write rd.
    let (cpu, _) = thumb1(f4(0x8, 1, 0), |c, _| {
        c.regs.r[0] = 1;
        c.regs.r[1] = 2;
    });
    assert_eq!(cpu.regs.r[0], 1);
    assert!(cpu.regs.flag_z());
    let (cpu, _) = thumb1(f4(0xA, 1, 0), |c, _| {
        c.regs.r[0] = 1;
        c.regs.r[1] = 2;
    });
    assert_eq!(cpu.regs.r[0], 1);
    assert_eq!(flags(&cpu), (true, false, false, false));
    let (cpu, _) = thumb1(f4(0xB, 1, 0), |c, _| {
        c.regs.r[0] = 1;
        c.regs.r[1] = 0xFFFF_FFFF;
    });
    assert_eq!(flags(&cpu), (false, true, true, false));
    // Register shifts cost an internal cycle.
    let (_, bus) = thumb1(f4(0x2, 1, 0), |_, _| {});
    assert_eq!(bus.idles, 1);
}

#[test]
fn format5_hi_register_ops_and_bx() {
    let (cpu, _) = thumb1(f5(0, 8, 1), |c, _| {
        c.regs.r[8] = 5;
        c.regs.r[1] = 7;
    });
    assert_eq!(cpu.regs.r[8], 12);
    assert_eq!(cpu.regs.cpsr & 0xF000_0000, 0); // no flags
    let (cpu, _) = thumb1(f5(1, 8, 1), |c, _| {
        c.regs.r[8] = 5;
        c.regs.r[1] = 7;
    });
    assert_eq!(flags(&cpu), (true, false, false, false));
    // mov r0, pc -> PC+4
    let (cpu, _) = thumb1(f5(2, 0, 15), |_, _| {});
    assert_eq!(cpu.regs.r[0], CODE + 4);
    // add pc, r0 branches (bit 0 ignored, stays THUMB)
    let (cpu, _) = thumb1(f5(0, 15, 0), |c, _| c.regs.r[0] = 0x101);
    assert_eq!(cpu.pc(), CODE + 4 + 0x100);
    assert!(cpu.regs.thumb());
    // mov pc, r0
    let (cpu, _) = thumb1(f5(2, 15, 0), |c, _| c.regs.r[0] = 0x3001);
    assert_eq!(cpu.pc(), 0x3000);
    assert!(cpu.regs.thumb());
    // bx r0 to ARM
    let (cpu, _) = thumb1(f5(3, 0, 0), |c, _| c.regs.r[0] = 0x3000);
    assert_eq!(cpu.pc(), 0x3000);
    assert!(!cpu.regs.thumb());
    // bx pc from an address that is 2 mod 4 -> ARM at (PC+4) & !2
    let mut cpu = thumb_cpu();
    let mut bus = TestBus::new();
    cpu.regs.r[15] = CODE + 2;
    bus.load_thumb(CODE + 2, &[f5(3, 0, 15)]);
    cpu.step(&mut bus);
    assert_eq!(cpu.pc(), CODE + 4);
    assert!(!cpu.regs.thumb());
}

#[test]
fn format6_ldr_pc_relative_aligns_pc() {
    // At CODE+2: ldr r0, [pc, #4] -> address ((CODE+2+4) & !2) + 4 = CODE+8
    let mut cpu = thumb_cpu();
    let mut bus = TestBus::new();
    cpu.regs.r[15] = CODE + 2;
    bus.load_thumb(CODE + 2, &[0x4801]);
    bus.poke32(CODE + 8, 0xCAFE_BABE);
    cpu.step(&mut bus);
    assert_eq!(cpu.regs.r[0], 0xCAFE_BABE);
    assert_eq!(bus.idles, 1);
}

#[test]
fn formats7_to_11_loads_and_stores() {
    // str r0, [r1, r2]
    let (_, bus) = thumb1(0x5088, |c, _| {
        c.regs.r[0] = 0x1234_5678;
        c.regs.r[1] = DATA;
        c.regs.r[2] = 4;
    });
    assert_eq!(bus.peek32(DATA + 4), 0x1234_5678);
    // ldr r0, [r1, r2] misaligned rotates
    let (cpu, _) = thumb1(0x5888, |c, b| {
        c.regs.r[1] = DATA;
        c.regs.r[2] = 1;
        b.poke32(DATA, 0xFF);
    });
    assert_eq!(cpu.regs.r[0], 0xFFu32.rotate_right(8));
    // strb / ldrb
    let (_, bus) = thumb1(0x5488, |c, _| {
        c.regs.r[0] = 0xABCD;
        c.regs.r[1] = DATA;
        c.regs.r[2] = 0;
    });
    assert_eq!(bus.peek32(DATA), 0xCD);
    let (cpu, _) = thumb1(0x5C88, |c, b| {
        c.regs.r[1] = DATA;
        c.regs.r[2] = 0;
        b.poke32(DATA, 0xFFFF_FF80);
    });
    assert_eq!(cpu.regs.r[0], 0x80);
    // strh / ldsb / ldrh / ldsh (format 8)
    let (_, bus) = thumb1(0x5288, |c, _| {
        c.regs.r[0] = 0xABCD_1234;
        c.regs.r[1] = DATA;
        c.regs.r[2] = 0;
    });
    assert_eq!(bus.peek32(DATA), 0x1234);
    let (cpu, _) = thumb1(0x5688, |c, b| {
        c.regs.r[1] = DATA;
        c.regs.r[2] = 0;
        b.poke32(DATA, 0xFF);
    });
    assert_eq!(cpu.regs.r[0], 0xFFFF_FFFF);
    let (cpu, _) = thumb1(0x5A88, |c, b| {
        c.regs.r[1] = DATA;
        c.regs.r[2] = 1;
        b.poke32(DATA, 0xFF);
    });
    assert_eq!(cpu.regs.r[0], 0xFFu32.rotate_right(8));
    let (cpu, _) = thumb1(0x5E88, |c, b| {
        c.regs.r[1] = DATA;
        c.regs.r[2] = 1;
        b.poke32(DATA, 0xFF00);
    });
    assert_eq!(cpu.regs.r[0], 0xFFFF_FFFF);
    let (cpu, _) = thumb1(0x5E88, |c, b| {
        c.regs.r[1] = DATA;
        c.regs.r[2] = 0;
        b.poke32(DATA, 0x8000);
    });
    assert_eq!(cpu.regs.r[0], 0xFFFF_8000);
    // format 9: str r0, [r1, #8]; ldrb r0, [r1, #3]
    let (_, bus) = thumb1(0x6088, |c, _| {
        c.regs.r[0] = 0x55;
        c.regs.r[1] = DATA;
    });
    assert_eq!(bus.peek32(DATA + 8), 0x55);
    let (cpu, _) = thumb1(0x78C8, |c, b| {
        c.regs.r[1] = DATA;
        b.poke32(DATA, 0x7700_0000);
    });
    assert_eq!(cpu.regs.r[0], 0x77);
    // format 10: strh r0, [r1, #2]; ldrh r0, [r1, #2]
    let (_, bus) = thumb1(0x8048, |c, _| {
        c.regs.r[0] = 0x9999_1234;
        c.regs.r[1] = DATA;
    });
    assert_eq!(bus.peek32(DATA), 0x1234_0000);
    let (cpu, _) = thumb1(0x8848, |c, b| {
        c.regs.r[1] = DATA;
        b.poke32(DATA, 0xBEEF_0000);
    });
    assert_eq!(cpu.regs.r[0], 0xBEEF);
    // format 11: str r0, [sp, #4]; ldr r0, [sp, #4]
    let (_, bus) = thumb1(0x9001, |c, _| {
        c.regs.r[0] = 0x66;
        c.regs.r[13] = DATA;
    });
    assert_eq!(bus.peek32(DATA + 4), 0x66);
    let (cpu, _) = thumb1(0x9801, |c, b| {
        c.regs.r[13] = DATA + 1;
        b.poke32(DATA + 4, 0xFF);
    });
    assert_eq!(cpu.regs.r[0], 0xFFu32.rotate_right(8));
}

#[test]
fn formats12_13_addresses() {
    // add r0, pc, #8 at CODE+2 -> ((CODE+6) & !2) + 8
    let mut cpu = thumb_cpu();
    let mut bus = TestBus::new();
    cpu.regs.r[15] = CODE + 2;
    bus.load_thumb(CODE + 2, &[0xA002]);
    cpu.step(&mut bus);
    assert_eq!(cpu.regs.r[0], CODE + 4 + 8);
    let (cpu, _) = thumb1(0xA802, |c, _| c.regs.r[13] = 0x100);
    assert_eq!(cpu.regs.r[0], 0x108);
    let (cpu, _) = thumb1(0xB002, |c, _| c.regs.r[13] = 0x100);
    assert_eq!(cpu.regs.r[13], 0x108);
    let (cpu, _) = thumb1(0xB082, |c, _| c.regs.r[13] = 0x100);
    assert_eq!(cpu.regs.r[13], 0xF8);
}

#[test]
fn format14_push_pop() {
    let mut cpu = thumb_cpu();
    let mut bus = TestBus::new();
    cpu.regs.r[13] = DATA;
    cpu.regs.r[0] = 1;
    cpu.regs.r[1] = 2;
    cpu.regs.r[14] = 0x3001;
    // push {r0, r1, lr}; pop {r2, r3, pc}
    run_thumb(&mut cpu, &mut bus, &[0xB503, 0xBD0C], 2);
    assert_eq!(bus.peek32(DATA - 12), 1);
    assert_eq!(bus.peek32(DATA - 8), 2);
    assert_eq!(bus.peek32(DATA - 4), 0x3001);
    assert_eq!(cpu.regs.r[2], 1);
    assert_eq!(cpu.regs.r[3], 2);
    assert_eq!(cpu.regs.r[13], DATA);
    assert_eq!(cpu.pc(), 0x3000);
    assert!(cpu.regs.thumb(), "POP {{PC}} keeps THUMB state on ARMv4");
    // pop {pc} with an even address also stays THUMB.
    let (cpu, _) = thumb1(0xBD00, |c, b| {
        c.regs.r[13] = DATA;
        b.poke32(DATA, 0x4000);
    });
    assert!(cpu.regs.thumb());
    assert_eq!(cpu.pc(), 0x4000);
    // push does not align the base.
    let (cpu, _) = thumb1(0xB40C, |c, _| c.regs.r[13] = DATA + 1);
    assert_eq!(cpu.regs.r[13], DATA + 1 - 8);
}

#[test]
fn format15_ldmia_stmia_edge_cases() {
    // stmia r3!, {r0, r1}
    let (cpu, bus) = thumb1(0xC303, |c, _| {
        c.regs.r[0] = 1;
        c.regs.r[1] = 2;
        c.regs.r[3] = DATA;
    });
    assert_eq!(bus.peek32(DATA), 1);
    assert_eq!(bus.peek32(DATA + 4), 2);
    assert_eq!(cpu.regs.r[3], DATA + 8);
    // ldmia r3!, {r2, r4}
    let (cpu, _) = thumb1(0xCB14, |c, b| {
        c.regs.r[3] = DATA;
        b.poke32(DATA, 7);
        b.poke32(DATA + 4, 9);
    });
    assert_eq!(
        (cpu.regs.r[2], cpu.regs.r[4], cpu.regs.r[3]),
        (7, 9, DATA + 8)
    );
    // Empty rlist: stm stores PC+6, ldm loads PC, base += 0x40.
    let (cpu, bus) = thumb1(0xC000, |c, _| c.regs.r[0] = DATA);
    assert_eq!(bus.peek32(DATA), CODE + 6);
    assert_eq!(cpu.regs.r[0], DATA + 0x40);
    let (cpu, _) = thumb1(0xC800, |c, b| {
        c.regs.r[0] = DATA;
        b.poke32(DATA, 0x3000);
    });
    assert_eq!(cpu.pc(), 0x3000);
    assert_eq!(cpu.regs.r[0], DATA + 0x40);
    // Base in list, not first: stores new base. stm r1!, {r0-r3}
    let (_, bus) = thumb1(0xC10F, |c, _| c.regs.r[1] = DATA);
    assert_eq!(bus.peek32(DATA + 4), DATA + 16);
    // Base first: stores old base. stm r1!, {r1-r4}
    let (_, bus) = thumb1(0xC11E, |c, _| c.regs.r[1] = DATA);
    assert_eq!(bus.peek32(DATA), DATA);
    // ldm with base in list: loaded value wins.
    let (cpu, _) = thumb1(0xC902, |c, b| {
        c.regs.r[1] = DATA;
        b.poke32(DATA, 0x1234);
    });
    assert_eq!(cpu.regs.r[1], 0x1234);
}

#[test]
fn format16_18_branches() {
    // beq +4 with Z set: PC = CODE+4+8
    let (cpu, _) = thumb1(0xD004, |c, _| c.regs.cpsr |= FLAG_Z);
    assert_eq!(cpu.pc(), CODE + 4 + 8);
    // bne not taken
    let (cpu, _) = thumb1(0xD104, |c, _| c.regs.cpsr |= FLAG_Z);
    assert_eq!(cpu.pc(), CODE + 2);
    // negative offset
    let (cpu, _) = thumb1(0xD0FE, |c, _| c.regs.cpsr |= FLAG_Z);
    assert_eq!(cpu.pc(), CODE);
    // b -2 (to self)
    let (cpu, _) = thumb1(0xE7FE, |_, _| {});
    assert_eq!(cpu.pc(), CODE);
    // b +0x400
    let (cpu, _) = thumb1(0xE200, |_, _| {});
    assert_eq!(cpu.pc(), CODE + 4 + 0x400);
}

#[test]
fn format19_bl_two_halves_and_split_by_interrupt() {
    // bl to CODE+4+0x1000+0x10 = target; encoded hi = 1, lo = 8
    let mut cpu = thumb_cpu();
    let mut bus = TestBus::new();
    run_thumb(&mut cpu, &mut bus, &[0xF001, 0xF808], 2);
    assert_eq!(cpu.pc(), CODE + 4 + 0x1000 + 0x10);
    assert_eq!(cpu.regs.r[14], (CODE + 4) | 1);
    // Negative offset: hi = 0x7FF (-1 << 12), lo = 0x7FE -> CODE+4 - 4
    let mut cpu = thumb_cpu();
    let mut bus = TestBus::new();
    run_thumb(&mut cpu, &mut bus, &[0xF7FF, 0xFFFE], 2);
    assert_eq!(cpu.pc(), CODE);
    // Interrupt between the halves: LR survives in the interrupted mode and the second half
    // completes after the return.
    let mut cpu = thumb_cpu();
    let mut bus = TestBus::new();
    bus.load_thumb(CODE, &[0xF001, 0xF808]);
    // IRQ handler at 0x18: subs pc, lr, #4
    bus.load_arm(0x18, &[dp_imm(SUB, true, 15, 14, 4, 0)]);
    cpu.step(&mut bus); // first half
    assert_eq!(cpu.regs.r[14], CODE + 4 + 0x1000);
    bus.irq = true;
    cpu.step(&mut bus); // takes the IRQ
    assert_eq!(cpu.regs.mode(), MODE_IRQ);
    assert_eq!(cpu.pc(), 0x18);
    bus.irq = false;
    cpu.step(&mut bus); // return
    assert_eq!(cpu.pc(), CODE + 2);
    assert!(cpu.regs.thumb());
    cpu.step(&mut bus); // second half
    assert_eq!(cpu.pc(), CODE + 4 + 0x1000 + 0x10);
    assert_eq!(cpu.regs.r[14], (CODE + 4) | 1);
    // Second half alone: BL LR+imm (Mario Golf).
    let (cpu, _) = thumb1(0xF800, |c, _| c.regs.r[14] = 0x5000);
    assert_eq!(cpu.pc(), 0x5000);
    assert_eq!(cpu.regs.r[14], (CODE + 2) | 1);
}

#[test]
fn thumb_pipeline_fetches_halfwords() {
    let mut cpu = thumb_cpu();
    let mut bus = TestBus::new();
    run_thumb(&mut cpu, &mut bus, &[f3(0, 0, 1), f3(0, 1, 2)], 2);
    assert_eq!(cpu.regs.r[0], 1);
    assert_eq!(cpu.regs.r[1], 2);
    assert_eq!(
        bus.fetches,
        vec![
            (CODE, Access::N),
            (CODE + 2, Access::S),
            (CODE + 4, Access::S),
            (CODE + 6, Access::S)
        ]
    );
    assert!(cpu
        .trace_line()
        .contains(&format!("{:08X} cpsr:", CODE + 4 + 4)));
}
