//! ARM encoding groups: decoder classification, data processing, PSR transfer, branches, single
//! and halfword transfers, block transfers, swap, multiply, pipeline behaviour.

use super::*;
use crate::cpu::alu;
use crate::cpu::decode::{decode_arm, ArmOp};

#[test]
fn decoder_classifies_every_group() {
    let cases: &[(u32, ArmOp)] = &[
        (0xE3A0_0001, ArmOp::DataProcImm),      // mov r0, #1
        (0xE1A0_0001, ArmOp::DataProcRegImm),   // mov r0, r1
        (0xE1A0_0211, ArmOp::DataProcRegReg),   // mov r0, r1, lsl r2
        (0xE10F_0000, ArmOp::Mrs),              // mrs r0, cpsr
        (0xE129_F000, ArmOp::MsrReg),           // msr cpsr_fc, r0
        (0xE328_F201, ArmOp::MsrImm),           // msr cpsr_f, #0x10000000
        (0xE12F_FF10, ArmOp::Bx),               // bx r0
        (0xE000_0291, ArmOp::Multiply),         // mul r0, r1, r2
        (0xE020_3291, ArmOp::Multiply),         // mla r0, r1, r2, r3
        (0xE083_2190, ArmOp::MultiplyLong),     // umull r2, r3, r0, r1
        (0xE0E3_2190, ArmOp::MultiplyLong),     // smlal
        (0xE100_0091, ArmOp::Swp),              // swp r0, r1, [r0]
        (0xE140_0091, ArmOp::Swp),              // swpb
        (0xE1C0_00B0, ArmOp::HalfwordTransfer), // strh r0, [r0]
        (0xE1D0_00F0, ArmOp::HalfwordTransfer), // ldrsh r0, [r0]
        (0xE190_00D2, ArmOp::HalfwordTransfer), // ldrsb r0, [r0, r2]
        (0xE590_0000, ArmOp::SingleTransfer),   // ldr r0, [r0]
        (0xE7D0_0001, ArmOp::SingleTransfer),   // ldrb r0, [r0, r1]
        (0xE7D0_0011, ArmOp::Undefined),        // bit 4 set in register-offset LDR
        (0xE8BD_8000, ArmOp::BlockTransfer),    // ldmia sp!, {pc}
        (0xEA00_0000, ArmOp::Branch),           // b
        (0xEB00_0000, ArmOp::Branch),           // bl
        (0xEF00_0004, ArmOp::Swi),              // swi 4
        (0xEE00_0010, ArmOp::Undefined),        // mcr (coprocessor)
        (0xED00_0000, ArmOp::Undefined),        // stc (coprocessor)
        (0xE600_0010, ArmOp::Undefined),        // explicit undefined space
    ];
    for &(op, expect) in cases {
        assert_eq!(decode_arm(op), expect, "opcode {op:08X}");
    }
}

#[test]
fn all_sixteen_alu_ops() {
    // (opcode, rn, op2, expected result)
    let cases: &[(u32, u32, u32, u32)] = &[
        (AND, 0xF0F0, 0xFF, 0xF0),
        (EOR, 0xF0F0, 0xFF, 0xF00F),
        (SUB, 10, 3, 7),
        (RSB, 3, 10, 7),
        (ADD, 10, 3, 13),
        (ADC, 10, 3, 13),
        (SBC, 10, 3, 6),
        (RSC, 3, 10, 6),
        (ORR, 0xF000, 0x0F, 0xF00F),
        (MOV, 0, 0x42, 0x42),
        (BIC, 0xFF, 0x0F, 0xF0),
        (MVN, 0, 0, 0xFFFF_FFFF),
    ];
    for &(opcode, rn, op2, expect) in cases {
        let (cpu, _) = arm1(dp_reg(opcode, false, 0, 1, 2, alu::LSL, 0), |c, _| {
            c.regs.r[1] = rn;
            c.regs.r[2] = op2;
        });
        assert_eq!(cpu.regs.r[0], expect, "opcode {opcode}");
        // Without S the flags are untouched.
        assert_eq!(cpu.regs.cpsr & 0xF000_0000, 0);
    }
}

#[test]
fn pc_as_operand_reads_pc_plus_8_or_12() {
    // mov r0, pc -> PC+8
    let (cpu, _) = arm1(dp_reg(MOV, false, 0, 0, 15, alu::LSL, 0), |_, _| {});
    assert_eq!(cpu.regs.r[0], CODE + 8);
    // add r0, pc, #0 -> PC+8 as Rn
    let (cpu, _) = arm1(dp_imm(ADD, false, 0, 15, 0, 0), |_, _| {});
    assert_eq!(cpu.regs.r[0], CODE + 8);
    // mov r0, pc, lsl r1 (r1 = 0) -> PC+12
    let (cpu, _) = arm1(dp_regreg(MOV, false, 0, 0, 15, alu::LSL, 1), |c, _| {
        c.regs.r[1] = 0
    });
    assert_eq!(cpu.regs.r[0], CODE + 12);
    // add r0, pc, r1, lsl r2 -> Rn = PC+12
    let (cpu, _) = arm1(dp_regreg(ADD, false, 0, 15, 1, alu::LSL, 2), |c, _| {
        c.regs.r[1] = 0;
        c.regs.r[2] = 0;
    });
    assert_eq!(cpu.regs.r[0], CODE + 12);
}

#[test]
fn alu_write_to_pc_branches_and_aligns() {
    let mut cpu = arm_cpu();
    let mut bus = TestBus::new();
    // mov pc, r0 with r0 = 0x2003 -> PC = 0x2000
    cpu.regs.r[0] = 0x2003;
    bus.load_arm(0x2000, &[mov_imm(1, 7), B_SELF]);
    run_arm(
        &mut cpu,
        &mut bus,
        &[dp_reg(MOV, false, 15, 0, 0, alu::LSL, 0)],
        1,
    );
    assert_eq!(cpu.pc(), 0x2000);
    // The branch step ends with the pipeline refill (N at the target, S after it).
    assert!(cpu.next_fetch_seq);
    cpu.step(&mut bus);
    assert_eq!(cpu.regs.r[1], 7);
    assert_eq!(cpu.pc(), 0x2004);
    // The refill fetched 0x2000 with N and 0x2004 with S.
    let n = bus.fetches.len();
    assert_eq!(bus.fetches[n - 3], (0x2000, Access::N));
    assert_eq!(bus.fetches[n - 2], (0x2004, Access::S));
    assert_eq!(bus.fetches[n - 1], (0x2008, Access::S));
}

#[test]
fn movs_pc_restores_spsr() {
    // In SVC mode with SPSR = SYS | T: movs pc, lr switches to System mode, THUMB, at lr & !1.
    let (cpu, _) = arm1(dp_reg(MOV, true, 15, 0, 14, alu::LSL, 0), |c, _| {
        c.regs.set_cpsr(MODE_SVC | FLAG_I);
        c.regs.set_spsr(MODE_SYS | FLAG_T | FLAG_Z);
        c.regs.r[14] = 0x3001;
    });
    assert_eq!(cpu.regs.mode(), MODE_SYS);
    assert!(cpu.regs.thumb());
    assert!(cpu.regs.flag_z());
    assert!(!cpu.regs.irq_disabled());
    assert_eq!(cpu.pc(), 0x3000);
}

#[test]
fn subs_pc_lr_4_returns_from_irq() {
    let (cpu, _) = arm1(dp_imm(SUB, true, 15, 14, 4, 0), |c, _| {
        c.regs.set_cpsr(MODE_IRQ | FLAG_I);
        c.regs.set_spsr(MODE_USR | FLAG_C);
        c.regs.r[14] = 0x4004;
    });
    assert_eq!(cpu.regs.mode(), MODE_USR);
    assert_eq!(cpu.pc(), 0x4000);
    assert!(cpu.regs.flag_c());
}

#[test]
fn mrs_msr_fields_and_user_restrictions() {
    // MSR cpsr_f only touches flags.
    let (cpu, _) = arm1(msr_imm(false, 8, 0xF, 1), |_, _| {}); // #0xF ror 2 = 0xC0000003 -> flags N,Z
    assert_eq!(cpu.regs.cpsr & 0xF000_0000, FLAG_N | FLAG_Z);
    assert_eq!(cpu.regs.mode(), MODE_SYS);

    // MSR cpsr_c switches mode and banks.
    let (cpu, _) = arm1(msr_reg(false, 1, 0), |c, _| {
        c.regs.r[13] = 0x1111;
        c.regs.r[0] = MODE_FIQ | FLAG_I;
    });
    assert_eq!(cpu.regs.mode(), MODE_FIQ);
    assert!(cpu.regs.irq_disabled());
    assert_eq!(cpu.regs.sp_of(MODE_SYS), 0x1111);

    // In User mode the control field is ignored.
    let (cpu, _) = arm1(msr_reg(false, 9, 0), |c, _| {
        c.regs.set_cpsr(MODE_USR);
        c.regs.r[0] = MODE_SVC | FLAG_I | FLAG_N;
    });
    assert_eq!(cpu.regs.mode(), MODE_USR);
    assert!(!cpu.regs.irq_disabled());
    assert!(cpu.regs.flag_n());

    // T bit cannot be set through MSR.
    let (cpu, _) = arm1(msr_reg(false, 1, 0), |c, _| c.regs.r[0] = MODE_SYS | FLAG_T);
    assert!(!cpu.regs.thumb());

    // MRS/MSR SPSR round trip in a privileged mode.
    let mut cpu = arm_cpu();
    let mut bus = TestBus::new();
    cpu.regs.set_cpsr(MODE_IRQ);
    cpu.regs.r[0] = 0xF000_0010;
    run_arm(
        &mut cpu,
        &mut bus,
        &[msr_reg(true, 0xF, 0), mrs(1, true), mrs(2, false)],
        3,
    );
    assert_eq!(cpu.regs.r[1], 0xF000_0010);
    assert_eq!(cpu.regs.r[2], MODE_IRQ);
}

#[test]
fn branch_bl_bx() {
    // b +2 words: target = PC+8+8
    let (cpu, _) = arm1(branch(false, 2), |_, _| {});
    assert_eq!(cpu.pc(), CODE + 16);
    // b -2 words
    let (cpu, _) = arm1(branch(false, -2), |_, _| {});
    assert_eq!(cpu.pc(), CODE);
    // bl sets LR = PC+4
    let (cpu, _) = arm1(branch(true, 0x10), |_, _| {});
    assert_eq!(cpu.regs.r[14], CODE + 4);
    assert_eq!(cpu.pc(), CODE + 8 + 0x40);
    // bx r0 with bit 0 -> THUMB
    let (cpu, _) = arm1(bx(0), |c, _| c.regs.r[0] = 0x2001);
    assert!(cpu.regs.thumb());
    assert_eq!(cpu.pc(), 0x2000);
    // bx r0 without bit 0 stays ARM and aligns
    let (cpu, _) = arm1(bx(0), |c, _| c.regs.r[0] = 0x2002);
    assert!(!cpu.regs.thumb());
    assert_eq!(cpu.pc(), 0x2000);
    // bx pc = PC+8
    let (cpu, _) = arm1(bx(15), |_, _| {});
    assert_eq!(cpu.pc(), CODE + 8);
}

#[test]
fn condition_false_costs_one_fetch_and_skips() {
    let mut cpu = arm_cpu();
    let mut bus = TestBus::new();
    // moveq r0, #1 (Z clear) then mov r1, #2
    let op = (dp_imm(MOV, false, 0, 0, 1, 0) & 0x0FFF_FFFF) | 0x0 << 28;
    run_arm(&mut cpu, &mut bus, &[op, mov_imm(1, 2)], 2);
    assert_eq!(cpu.regs.r[0], 0);
    assert_eq!(cpu.regs.r[1], 2);
    // NV condition never executes.
    let mut cpu = arm_cpu();
    let mut bus = TestBus::new();
    let op = (dp_imm(MOV, false, 0, 0, 1, 0) & 0x0FFF_FFFF) | 0xF << 28;
    run_arm(&mut cpu, &mut bus, &[op], 1);
    assert_eq!(cpu.regs.r[0], 0);
    assert_eq!(cpu.pc(), CODE + 4);
}

#[test]
fn ldr_str_word_and_byte_with_indexing() {
    // str r0, [r1], #4 (post, writeback) then ldr r2, [r1, #-4]! (pre, writeback)
    let mut cpu = arm_cpu();
    let mut bus = TestBus::new();
    cpu.regs.r[0] = 0xDEAD_BEEF;
    cpu.regs.r[1] = DATA;
    run_arm(
        &mut cpu,
        &mut bus,
        &[
            ldst_imm(false, false, false, true, false, 0, 1, 4),
            ldst_imm(true, false, true, false, true, 2, 1, 4),
        ],
        2,
    );
    assert_eq!(bus.peek32(DATA), 0xDEAD_BEEF);
    assert_eq!(cpu.regs.r[1], DATA);
    assert_eq!(cpu.regs.r[2], 0xDEAD_BEEF);
    // strb stores the low byte; ldrb zero-extends.
    let (cpu, bus) = arm1(ldst_imm(false, true, true, true, false, 0, 1, 1), |c, _| {
        c.regs.r[0] = 0x1234_56AB;
        c.regs.r[1] = DATA;
    });
    assert_eq!(bus.peek32(DATA), 0x0000_AB00);
    let (cpu2, _) = arm1(ldst_imm(true, true, true, true, false, 2, 1, 1), |c, b| {
        c.regs.r[1] = DATA;
        b.poke32(DATA, 0xFFFF_FFFF);
    });
    assert_eq!(cpu2.regs.r[2], 0xFF);
    assert_eq!(cpu.regs.r[1], DATA);
}

#[test]
fn ldr_unaligned_rotates_and_str_aligns() {
    let (cpu, _) = arm1(ldst_imm(true, false, true, true, false, 0, 1, 3), |c, b| {
        c.regs.r[1] = DATA;
        b.poke32(DATA, 0x1122_3344);
    });
    assert_eq!(cpu.regs.r[0], 0x1122_3344u32.rotate_right(24));
    let (_, bus) = arm1(
        ldst_imm(false, false, true, true, false, 0, 1, 2),
        |c, _| {
            c.regs.r[1] = DATA;
            c.regs.r[0] = 0xAABB_CCDD;
        },
    );
    assert_eq!(bus.peek32(DATA), 0xAABB_CCDD);
}

#[test]
fn str_pc_stores_pc_plus_12_and_ldr_pc_branches() {
    let (_, bus) = arm1(
        ldst_imm(false, false, true, true, false, 15, 1, 0),
        |c, _| c.regs.r[1] = DATA,
    );
    assert_eq!(bus.peek32(DATA), CODE + 12);
    let (cpu, _) = arm1(
        ldst_imm(true, false, true, true, false, 15, 1, 0),
        |c, b| {
            c.regs.r[1] = DATA;
            b.poke32(DATA, 0x3002);
        },
    );
    assert_eq!(cpu.pc(), 0x3000);
    assert!(!cpu.regs.thumb());
    // ldr r0, [pc, #-8] loads the instruction itself.
    let op = ldst_imm(true, false, true, false, false, 0, 15, 8);
    let (cpu, _) = arm1(op, |_, _| {});
    assert_eq!(cpu.regs.r[0], op);
}

#[test]
fn ldr_str_writeback_with_rd_equal_rn() {
    // str r0, [r0, #4]! stores the old r0 and writes back.
    let (cpu, bus) = arm1(0xE5A0_0004, |c, _| c.regs.r[0] = DATA);
    assert_eq!(cpu.regs.r[0], DATA + 4);
    assert_eq!(bus.peek32(DATA + 4), DATA);
    // ldr r0, [r0], #4: loaded value wins over writeback.
    let (cpu, _) = arm1(0xE490_0004, |c, b| {
        c.regs.r[0] = DATA;
        b.poke32(DATA, 32);
    });
    assert_eq!(cpu.regs.r[0], 32);
}

#[test]
fn ldr_register_offset_with_rrx_does_not_touch_flags() {
    // ldr r2, [r1, r0, rrx]! with C set and r0 = r1 = 0 -> r1 = 0x80000000, C still set.
    let (cpu, _) = arm1(0xE7B1_2060, |c, _| c.regs.cpsr |= FLAG_C);
    assert_eq!(cpu.regs.r[1], 0x8000_0000);
    assert!(cpu.regs.flag_c());
}

#[test]
fn ldr_costs_one_internal_cycle_and_n_data_access() {
    let (_, bus) = arm1(ldst_imm(true, false, true, true, false, 0, 1, 0), |c, _| {
        c.regs.r[1] = DATA
    });
    assert_eq!(bus.idles, 1);
    assert_eq!(bus.accesses, vec![(DATA, 4, false, Access::N)]);
}

#[test]
fn halfword_and_signed_transfers() {
    // strh stores low halfword.
    let (_, bus) = arm1(ldsth_imm(false, 1, true, true, false, 0, 1, 0), |c, _| {
        c.regs.r[0] = 0x1234_ABCD;
        c.regs.r[1] = DATA;
    });
    assert_eq!(bus.peek32(DATA), 0xABCD);
    // ldrh zero-extends.
    let (cpu, _) = arm1(ldsth_imm(true, 1, true, true, false, 0, 1, 0), |c, b| {
        c.regs.r[1] = DATA;
        b.poke32(DATA, 0xFFFF_8001);
    });
    assert_eq!(cpu.regs.r[0], 0x8001);
    // ldrsh sign-extends.
    let (cpu, _) = arm1(ldsth_imm(true, 3, true, true, false, 0, 1, 0), |c, b| {
        c.regs.r[1] = DATA;
        b.poke32(DATA, 0x0000_8001);
    });
    assert_eq!(cpu.regs.r[0], 0xFFFF_8001);
    // ldrsb sign-extends a byte.
    let (cpu, _) = arm1(ldsth_imm(true, 2, true, true, false, 0, 1, 0), |c, b| {
        c.regs.r[1] = DATA;
        b.poke32(DATA, 0x0000_0080);
    });
    assert_eq!(cpu.regs.r[0], 0xFFFF_FF80);
    // Unaligned ldrh rotates by 8.
    let (cpu, _) = arm1(ldsth_imm(true, 1, true, true, false, 0, 1, 1), |c, b| {
        c.regs.r[1] = DATA;
        b.poke32(DATA, 0x0000_0020);
    });
    assert_eq!(cpu.regs.r[0], 0x20u32.rotate_right(8));
    // Unaligned ldrsh = sign-extended byte at the odd address.
    let (cpu, _) = arm1(ldsth_imm(true, 3, true, true, false, 0, 1, 1), |c, b| {
        c.regs.r[1] = DATA;
        b.poke32(DATA, 0x0000_FF00);
    });
    assert_eq!(cpu.regs.r[0], 0xFFFF_FFFF);
    // Unaligned strh is force-aligned.
    let (_, bus) = arm1(ldsth_imm(false, 1, true, true, false, 0, 1, 1), |c, _| {
        c.regs.r[0] = 32;
        c.regs.r[1] = DATA;
    });
    assert_eq!(bus.peek16(DATA), 32);
    // Register offset, post-indexed: strh r0, [r2], r1 ; base moves by r1.
    let (cpu, bus) = arm1(0xE082_00B1, |c, _| {
        c.regs.r[0] = 7;
        c.regs.r[1] = 4;
        c.regs.r[2] = DATA;
    });
    assert_eq!(bus.peek16(DATA), 7);
    assert_eq!(cpu.regs.r[2], DATA + 4);
}

#[test]
fn swp_and_swpb() {
    let (cpu, bus) = arm1(swp(false, 1, 0, 2), |c, b| {
        c.regs.r[0] = 0x1111_1111;
        c.regs.r[2] = DATA;
        b.poke32(DATA, 0x2222_2222);
    });
    assert_eq!(cpu.regs.r[1], 0x2222_2222);
    assert_eq!(bus.peek32(DATA), 0x1111_1111);
    assert_eq!(bus.idles, 1);
    // Byte swap zero-extends.
    let (cpu, bus) = arm1(swp(true, 1, 0, 2), |c, b| {
        c.regs.r[0] = 0xFFFF_FF11;
        c.regs.r[2] = DATA;
        b.poke32(DATA, 0xFFFF_FFFF);
    });
    assert_eq!(cpu.regs.r[1], 0xFF);
    assert_eq!(bus.peek32(DATA), 0xFFFF_FF11);
    // Misaligned: read rotated, write unrotated. Same Rd and Rm works.
    let (cpu, bus) = arm1(swp(false, 0, 0, 2), |c, b| {
        c.regs.r[0] = 32;
        c.regs.r[2] = DATA + 1;
        b.poke32(DATA, 64);
    });
    assert_eq!(cpu.regs.r[0], 64u32.rotate_right(8));
    assert_eq!(bus.peek32(DATA), 32);
}

#[test]
fn multiply_family() {
    let (cpu, bus) = arm1(mul(true, 0, 1, 2), |c, _| {
        c.regs.r[1] = -4i32 as u32;
        c.regs.r[2] = 8;
        c.regs.cpsr |= FLAG_C | FLAG_V;
    });
    assert_eq!(cpu.regs.r[0], -32i32 as u32);
    // C is rewritten by the multiplier array model (GBATEK: "destroyed"); V is untouched.
    let c = alu::multiply_carry(alu::MulKind::Short, -4i32 as u32, 8, 0);
    assert_eq!(flags(&cpu), (true, false, c, true));
    assert_eq!(bus.idles, 1);

    let (cpu, bus) = arm1(mla(false, 0, 1, 2, 3), |c, _| {
        c.regs.r[1] = 4;
        c.regs.r[2] = 0x1_0000;
        c.regs.r[3] = 8;
    });
    assert_eq!(cpu.regs.r[0], 0x4_0008);
    assert_eq!(bus.idles, 4); // m=3 for Rs=0x10000, +1 for accumulate

    let (cpu, _) = arm1(mull(0, false, 2, 3, 0, 1), |c, _| {
        c.regs.r[0] = 0xFFFF_FFFF;
        c.regs.r[1] = 0xFFFF_FFFF;
    });
    assert_eq!((cpu.regs.r[2], cpu.regs.r[3]), (1, 0xFFFF_FFFE));

    let (cpu, _) = arm1(mull(1, false, 2, 3, 0, 1), |c, _| {
        c.regs.r[0] = 0xFFFF_FFFF;
        c.regs.r[1] = 0xFFFF_FFFF;
        c.regs.r[2] = 0xFFFF_FFFE;
        c.regs.r[3] = 1;
    });
    assert_eq!((cpu.regs.r[2], cpu.regs.r[3]), (0xFFFF_FFFF, 0xFFFF_FFFF));

    let (cpu, _) = arm1(mull(2, true, 2, 3, 0, 1), |c, _| {
        c.regs.r[0] = 4;
        c.regs.r[1] = -8i32 as u32;
    });
    assert_eq!((cpu.regs.r[2], cpu.regs.r[3]), (-32i32 as u32, 0xFFFF_FFFF));
    assert!(cpu.regs.flag_n());
    assert!(!cpu.regs.flag_z());

    let (cpu, _) = arm1(mull(3, true, 2, 3, 0, 1), |c, _| {
        c.regs.r[0] = 4;
        c.regs.r[1] = -8i32 as u32;
        c.regs.r[2] = 32;
        c.regs.r[3] = 0;
    });
    assert_eq!((cpu.regs.r[2], cpu.regs.r[3]), (0, 0));
    assert!(cpu.regs.flag_z());
}

// ------------------------------------------------------------------ block transfers

#[test]
fn ldm_stm_addressing_modes() {
    // (pre, up, expected offsets of r0 and r1 relative to base, writeback)
    let cases: &[(bool, bool, i32, i32, i32)] = &[
        (false, true, 0, 4, 8),    // IA
        (true, true, 4, 8, 8),     // IB
        (false, false, -4, 0, -8), // DA
        (true, false, -8, -4, -8), // DB
    ];
    for &(pre, up, o0, o1, wb) in cases {
        let (cpu, bus) = arm1(ldstm(false, pre, up, false, true, 5, 0b11), |c, _| {
            c.regs.r[0] = 0xA0;
            c.regs.r[1] = 0xA1;
            c.regs.r[5] = DATA;
        });
        assert_eq!(
            bus.peek32((DATA as i32 + o0) as u32),
            0xA0,
            "pre={pre} up={up}"
        );
        assert_eq!(
            bus.peek32((DATA as i32 + o1) as u32),
            0xA1,
            "pre={pre} up={up}"
        );
        assert_eq!(cpu.regs.r[5], (DATA as i32 + wb) as u32);
        // Accesses are ascending, first N then S.
        assert_eq!(bus.accesses[0].3, Access::N);
        assert_eq!(bus.accesses[1].3, Access::S);
        assert!(bus.accesses[0].0 < bus.accesses[1].0);

        let (cpu, bus) = arm1(ldstm(true, pre, up, false, true, 5, 0b1100), |c, b| {
            c.regs.r[5] = DATA;
            b.poke32((DATA as i32 + o0) as u32, 0xB0);
            b.poke32((DATA as i32 + o1) as u32, 0xB1);
        });
        assert_eq!(cpu.regs.r[2], 0xB0);
        assert_eq!(cpu.regs.r[3], 0xB1);
        assert_eq!(cpu.regs.r[5], (DATA as i32 + wb) as u32);
        assert_eq!(bus.idles, 1);
    }
}

#[test]
fn ldm_stm_ignore_base_alignment() {
    let (cpu, bus) = arm1(ldstm(false, true, false, false, true, 2, 0b11), |c, _| {
        c.regs.r[0] = 32;
        c.regs.r[1] = 64;
        c.regs.r[2] = DATA + 3;
    });
    assert_eq!(cpu.regs.r[2], DATA - 5);
    assert_eq!(bus.peek32(DATA - 8), 32);
    assert_eq!(bus.peek32(DATA - 4), 64);
}

#[test]
fn stm_pc_stores_pc_plus_12_and_ldm_pc_branches() {
    let (_, bus) = arm1(
        ldstm(false, false, true, false, false, 1, 1 << 15),
        |c, _| c.regs.r[1] = DATA,
    );
    assert_eq!(bus.peek32(DATA), CODE + 12);
    let (cpu, _) = arm1(
        ldstm(true, false, true, false, true, 1, 1 << 15 | 1),
        |c, b| {
            c.regs.r[1] = DATA;
            b.poke32(DATA, 0x77);
            b.poke32(DATA + 4, 0x3001);
        },
    );
    assert_eq!(cpu.regs.r[0], 0x77);
    assert_eq!(cpu.pc(), 0x3000);
    assert_eq!(cpu.regs.r[1], DATA + 8);
}

#[test]
fn empty_rlist_transfers_pc_and_moves_base_by_0x40() {
    // stmia r0!, {}
    let (cpu, bus) = arm1(0xE8A0_0000, |c, _| c.regs.r[0] = DATA);
    assert_eq!(bus.peek32(DATA), CODE + 12);
    assert_eq!(cpu.regs.r[0], DATA + 0x40);
    // stmda r0!, {} -> stored at base - 0x3C
    let (cpu, bus) = arm1(0xE820_0000, |c, _| c.regs.r[0] = DATA);
    assert_eq!(bus.peek32(DATA - 0x3C), CODE + 12);
    assert_eq!(cpu.regs.r[0], DATA - 0x40);
    // stmdb r0!, {} -> stored at base - 0x40
    let (cpu, bus) = arm1(0xE920_0000, |c, _| c.regs.r[0] = DATA);
    assert_eq!(bus.peek32(DATA - 0x40), CODE + 12);
    assert_eq!(cpu.regs.r[0], DATA - 0x40);
    // stmib r0!, {} -> stored at base + 4
    let (cpu, bus) = arm1(0xE9A0_0000, |c, _| c.regs.r[0] = DATA);
    assert_eq!(bus.peek32(DATA + 4), CODE + 12);
    assert_eq!(cpu.regs.r[0], DATA + 0x40);
    // ldmia r0!, {} loads PC.
    let (cpu, _) = arm1(0xE8B0_0000, |c, b| {
        c.regs.r[0] = DATA;
        b.poke32(DATA, 0x5000);
    });
    assert_eq!(cpu.pc(), 0x5000);
    assert_eq!(cpu.regs.r[0], DATA + 0x40);
}

#[test]
fn stm_writeback_with_base_in_list() {
    // Base first in list: old base stored.
    let (cpu, bus) = arm1(ldstm(false, true, false, false, true, 0, 0b11), |c, _| {
        c.regs.r[0] = DATA
    });
    assert_eq!(bus.peek32(DATA - 8), DATA);
    assert_eq!(cpu.regs.r[0], DATA - 8);
    // Base not first: new base stored. stmfd r1!, {r0-r3}
    let (_, bus) = arm1(0xE921_000F, |c, _| c.regs.r[1] = DATA);
    assert_eq!(bus.peek32(DATA - 12), DATA - 16);
    // stmfa r2!, {r0-r3} (IB): new base.
    let (_, bus) = arm1(0xE9A2_000F, |c, _| c.regs.r[2] = DATA);
    assert_eq!(bus.peek32(DATA + 12), DATA + 16);
}

#[test]
fn ldm_writeback_with_base_in_list_loads_value() {
    // ldmia r1!, {r1, r2}
    let (cpu, _) = arm1(0xE8B1_0006, |c, b| {
        c.regs.r[1] = DATA;
        b.poke32(DATA, 0xA);
        b.poke32(DATA + 4, 0xB);
    });
    assert_eq!(cpu.regs.r[1], 0xA);
    assert_eq!(cpu.regs.r[2], 0xB);
    // ldmia r1!, {r0, r1}
    let (cpu, _) = arm1(0xE8B1_0003, |c, b| {
        c.regs.r[1] = DATA;
        b.poke32(DATA, 0xA);
        b.poke32(DATA + 4, 0xB);
    });
    assert_eq!(cpu.regs.r[0], 0xA);
    assert_eq!(cpu.regs.r[1], 0xB);
}

#[test]
fn ldm_stm_user_bank_and_spsr_restore() {
    // In FIQ mode, stm r0, {r8, r9}^ stores the User-bank r8.
    let (_, bus) = arm1(
        ldstm(false, false, true, true, false, 0, 0b11 << 8),
        |c, _| {
            c.regs.set_cpsr(MODE_SYS);
            c.regs.r[8] = 32;
            c.regs.switch_mode(MODE_FIQ);
            c.regs.r[8] = 64;
            c.regs.r[0] = DATA;
        },
    );
    assert_eq!(bus.peek32(DATA), 32);
    // ldm r0, {r8, r9}^ in FIQ loads the User-bank r8 and leaves r8_fiq alone.
    let (mut cpu, _) = arm1(
        ldstm(true, false, true, true, false, 0, 0b11 << 8),
        |c, b| {
            c.regs.switch_mode(MODE_FIQ);
            c.regs.r[8] = 0xB;
            c.regs.r[0] = DATA;
            b.poke32(DATA, 0xA);
        },
    );
    assert_eq!(cpu.regs.r[8], 0xB);
    cpu.regs.switch_mode(MODE_SYS);
    assert_eq!(cpu.regs.r[8], 0xA);
    // ldmfd sp!, {r0, pc}^ in IRQ mode restores CPSR from SPSR (THUMB target).
    let (cpu, _) = arm1(
        ldstm(true, false, true, true, true, 13, 1 << 15 | 1),
        |c, b| {
            c.regs.set_cpsr(MODE_IRQ | FLAG_I);
            c.regs.set_spsr(MODE_USR | FLAG_T | FLAG_N);
            c.regs.r[13] = DATA;
            b.poke32(DATA, 0x99);
            b.poke32(DATA + 4, 0x6001);
        },
    );
    assert_eq!(cpu.regs.r[0], 0x99);
    assert_eq!(cpu.regs.mode(), MODE_USR);
    assert!(cpu.regs.thumb());
    assert!(cpu.regs.flag_n());
    assert_eq!(cpu.pc(), 0x6000);
    // The IRQ-mode SP was written back before the mode switch.
    assert_eq!(cpu.regs.sp_of(MODE_IRQ), DATA + 8);
}

// ------------------------------------------------------------------ pipeline

#[test]
fn pipeline_executes_already_fetched_word_after_self_modification() {
    let mut cpu = arm_cpu();
    let mut bus = TestBus::new();
    // 0: mov r0, #1
    // 4: str r2, [pc, #0]   -> writes to address 12 (PC+8), which is already fetched
    // 8: mov r1, #5
    // 12: mov r1, #7        <- overwritten with mov r1, #9 while in the pipeline
    // 16: mov r3, #7        <- not yet fetched when the store happens; sees new value
    cpu.regs.r[2] = mov_imm(1, 9);
    bus.load_arm(
        CODE,
        &[
            mov_imm(0, 1),
            ldst_imm(false, false, true, true, false, 2, 15, 0),
            mov_imm(1, 5),
            mov_imm(1, 7),
            B_SELF,
        ],
    );
    for _ in 0..4 {
        cpu.step(&mut bus);
    }
    assert_eq!(bus.peek32(CODE + 12), mov_imm(1, 9));
    assert_eq!(
        cpu.regs.r[1], 7,
        "the word at PC+8 was already in the pipeline"
    );

    // Now the same but writing PC+12, which is fetched only after the store.
    let mut cpu = arm_cpu();
    let mut bus = TestBus::new();
    cpu.regs.r[2] = mov_imm(3, 9);
    bus.load_arm(
        CODE,
        &[
            mov_imm(0, 1),
            ldst_imm(false, false, true, true, false, 2, 15, 4),
            mov_imm(1, 5),
            mov_imm(1, 7),
            mov_imm(3, 7),
            B_SELF,
        ],
    );
    for _ in 0..5 {
        cpu.step(&mut bus);
    }
    assert_eq!(
        cpu.regs.r[3], 9,
        "the word at PC+12 is fetched after the store"
    );
}

#[test]
fn sequential_execution_fetches_one_word_per_step() {
    let mut cpu = arm_cpu();
    let mut bus = TestBus::new();
    run_arm(
        &mut cpu,
        &mut bus,
        &[mov_imm(0, 1), mov_imm(1, 2), mov_imm(2, 3)],
        3,
    );
    assert_eq!(cpu.pc(), CODE + 12);
    assert_eq!(
        bus.fetches,
        vec![
            (CODE, Access::N),
            (CODE + 4, Access::S),
            (CODE + 8, Access::S),
            (CODE + 12, Access::S),
            (CODE + 16, Access::S)
        ]
    );
    assert_eq!(cpu.instr_count, 3);
    assert_eq!(cpu.last_opcode, mov_imm(2, 3));
}

#[test]
fn trace_line_and_state_hash() {
    let cpu = arm_cpu();
    let line = cpu.trace_line();
    assert!(line.ends_with(&format!("{:08X} cpsr: {:08X}", CODE + 8, MODE_SYS)));
    assert_eq!(line.split(' ').count(), 18);
    let mut other = arm_cpu();
    assert_eq!(cpu.state_hash(), other.state_hash());
    other.regs.r[3] = 1;
    assert_ne!(cpu.state_hash(), other.state_hash());
}

#[test]
fn disassembler_smoke() {
    use crate::cpu::disasm;
    assert_eq!(disasm::arm(0xE3A0_0001, 0), "mov r0,#0x1");
    assert_eq!(disasm::arm(0xE8BD_8003, 0), "ldmia sp!,{r0,r1,pc}");
    assert_eq!(disasm::arm(0xEAFF_FFFE, 0x1000), "b 0x00001000");
    assert_eq!(disasm::arm(0xE5A0_0004, 0), "str r0,[r0,#0x4]!");
    assert_eq!(disasm::thumb(0x4770, 0), "bx lr");
    assert_eq!(disasm::thumb(0xB500, 0), "push {lr}");
    assert_eq!(disasm::thumb(0xE7FE, 0x1000), "b 0x00001000");
}
