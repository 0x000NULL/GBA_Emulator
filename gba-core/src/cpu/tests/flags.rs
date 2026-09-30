//! Barrel shifter carry-out rules and arithmetic flag behaviour (ADC/SBC/RSC, CMN, overflow).

use super::*;
use crate::cpu::alu;

#[test]
fn shift_imm_special_cases() {
    // LSL#0: unchanged, carry preserved.
    assert_eq!(
        alu::shift_imm(0x8000_0001, alu::LSL, 0, true),
        (0x8000_0001, true)
    );
    assert_eq!(
        alu::shift_imm(0x8000_0001, alu::LSL, 0, false),
        (0x8000_0001, false)
    );
    // LSL#1 carries bit 31.
    assert_eq!(alu::shift_imm(0x8000_0001, alu::LSL, 1, false), (2, true));
    // LSR#0 = LSR#32.
    assert_eq!(alu::shift_imm(0x8000_0000, alu::LSR, 0, false), (0, true));
    assert_eq!(alu::shift_imm(0x7FFF_FFFF, alu::LSR, 0, true), (0, false));
    // ASR#0 = ASR#32.
    assert_eq!(
        alu::shift_imm(0x8000_0000, alu::ASR, 0, false),
        (0xFFFF_FFFF, true)
    );
    assert_eq!(alu::shift_imm(0x4000_0000, alu::ASR, 0, true), (0, false));
    // ROR#0 = RRX.
    assert_eq!(
        alu::shift_imm(0x0000_0001, alu::ROR, 0, true),
        (0x8000_0000, true)
    );
    assert_eq!(alu::shift_imm(0x0000_0002, alu::ROR, 0, false), (1, false));
    // Ordinary shifts.
    assert_eq!(alu::shift_imm(0xF0, alu::LSR, 4, false), (0xF, false));
    assert_eq!(alu::shift_imm(0xF8, alu::LSR, 4, false), (0xF, true));
    assert_eq!(
        alu::shift_imm(0x8000_0000, alu::ASR, 31, false),
        (0xFFFF_FFFF, false)
    );
    assert_eq!(
        alu::shift_imm(0x0000_00F1, alu::ROR, 4, false),
        (0x1000_000F, false)
    );
}

#[test]
fn shift_reg_large_amounts() {
    // Amount 0 leaves everything alone regardless of type.
    for ty in [alu::LSL, alu::LSR, alu::ASR, alu::ROR] {
        assert_eq!(
            alu::shift_reg(0x8000_0000, ty, 0, true),
            (0x8000_0000, true)
        );
    }
    assert_eq!(alu::shift_reg(1, alu::LSL, 32, false), (0, true));
    assert_eq!(alu::shift_reg(1, alu::LSL, 33, true), (0, false));
    assert_eq!(alu::shift_reg(0x8000_0000, alu::LSR, 32, false), (0, true));
    assert_eq!(alu::shift_reg(0x8000_0000, alu::LSR, 40, true), (0, false));
    assert_eq!(
        alu::shift_reg(0x8000_0000, alu::ASR, 32, false),
        (0xFFFF_FFFF, true)
    );
    assert_eq!(
        alu::shift_reg(0x8000_0000, alu::ASR, 200, false),
        (0xFFFF_FFFF, true)
    );
    assert_eq!(alu::shift_reg(0x1000_0000, alu::ASR, 99, true), (0, false));
    // ROR by 32 (and 64): value unchanged, carry = bit 31.
    assert_eq!(
        alu::shift_reg(0x8000_0001, alu::ROR, 32, false),
        (0x8000_0001, true)
    );
    assert_eq!(
        alu::shift_reg(0x0000_0001, alu::ROR, 64, true),
        (0x0000_0001, false)
    );
    assert_eq!(
        alu::shift_reg(0x0000_0003, alu::ROR, 33, false),
        (0x8000_0001, true)
    );
}

#[test]
fn immediate_operand_carry() {
    assert_eq!(alu::imm_operand(0xFF, 0, true), (0xFF, true));
    assert_eq!(alu::imm_operand(0xFF, 0, false), (0xFF, false));
    // #0xFF ror 4 = 0xF000000F: carry = bit 31.
    assert_eq!(alu::imm_operand(0xFF, 2, false), (0xF000_000F, true));
    assert_eq!(alu::imm_operand(0x01, 1, true), (0x4000_0000, false));
}

#[test]
fn add_sub_flags() {
    let r = alu::add_with_carry(0x7FFF_FFFF, 1, false);
    assert_eq!((r.value, r.carry, r.overflow), (0x8000_0000, false, true));
    let r = alu::add_with_carry(0xFFFF_FFFF, 1, false);
    assert_eq!((r.value, r.carry, r.overflow), (0, true, false));
    let r = alu::add_with_carry(0xFFFF_FFFF, 0, true);
    assert_eq!((r.value, r.carry, r.overflow), (0, true, false));
    let r = alu::sub(0, 1);
    assert_eq!((r.value, r.carry, r.overflow), (0xFFFF_FFFF, false, false));
    let r = alu::sub(0x8000_0000, 1);
    assert_eq!((r.value, r.carry, r.overflow), (0x7FFF_FFFF, true, true));
    let r = alu::sub(5, 5);
    assert_eq!((r.value, r.carry, r.overflow), (0, true, false));
    // SBC with carry clear subtracts one more.
    let r = alu::sub_with_carry(5, 3, false);
    assert_eq!((r.value, r.carry, r.overflow), (1, true, false));
    let r = alu::sub_with_carry(3, 3, false);
    assert_eq!((r.value, r.carry, r.overflow), (0xFFFF_FFFF, false, false));
    let r = alu::sub_with_carry(3, 3, true);
    assert_eq!((r.value, r.carry, r.overflow), (0, true, false));
}

#[test]
fn logical_ops_keep_v_and_take_shifter_carry() {
    // ANDS r0, r1, r2, LSL #1 with r2 = 0x80000000 -> carry out 1, result 0 -> Z.
    let (cpu, _) = arm1(dp_reg(AND, true, 0, 1, 2, alu::LSL, 1), |c, _| {
        c.regs.r[1] = 0xFFFF_FFFF;
        c.regs.r[2] = 0x8000_0000;
        c.regs.cpsr |= FLAG_V;
    });
    assert_eq!(cpu.regs.r[0], 0);
    assert_eq!(flags(&cpu), (false, true, true, true));
    // MOVS with LSL#0 leaves carry untouched.
    let (cpu, _) = arm1(dp_reg(MOV, true, 0, 0, 1, alu::LSL, 0), |c, _| {
        c.regs.r[1] = 0x8000_0000;
        c.regs.cpsr |= FLAG_C;
    });
    assert_eq!(flags(&cpu), (true, false, true, false));
    let (cpu, _) = arm1(dp_reg(MOV, true, 0, 0, 1, alu::LSL, 0), |c, _| {
        c.regs.r[1] = 0;
    });
    assert_eq!(flags(&cpu), (false, true, false, false));
}

#[test]
fn adc_sbc_rsc_use_carry_in() {
    let (cpu, _) = arm1(dp_imm(ADC, true, 0, 1, 1, 0), |c, _| {
        c.regs.r[1] = 1;
        c.regs.cpsr |= FLAG_C;
    });
    assert_eq!(cpu.regs.r[0], 3);
    assert_eq!(flags(&cpu), (false, false, false, false));

    let (cpu, _) = arm1(dp_imm(SBC, true, 0, 1, 1, 0), |c, _| {
        c.regs.r[1] = 1;
    });
    assert_eq!(cpu.regs.r[0], 0xFFFF_FFFF);
    assert_eq!(flags(&cpu), (true, false, false, false));

    let (cpu, _) = arm1(dp_imm(RSC, true, 0, 1, 10, 0), |c, _| {
        c.regs.r[1] = 3;
        c.regs.cpsr |= FLAG_C;
    });
    assert_eq!(cpu.regs.r[0], 7);
    assert_eq!(flags(&cpu), (false, false, true, false));

    let (cpu, _) = arm1(dp_imm(RSB, true, 0, 1, 0, 0), |c, _| {
        c.regs.r[1] = 0x8000_0000;
    });
    assert_eq!(cpu.regs.r[0], 0x8000_0000);
    assert_eq!(flags(&cpu), (true, false, false, true));
}

#[test]
fn cmp_cmn_tst_teq_do_not_write_rd() {
    let (cpu, _) = arm1(dp_imm(CMP, true, 0, 1, 5, 0), |c, _| {
        c.regs.r[0] = 0xDEAD;
        c.regs.r[1] = 5;
    });
    assert_eq!(cpu.regs.r[0], 0xDEAD);
    assert_eq!(flags(&cpu), (false, true, true, false));

    let (cpu, _) = arm1(dp_imm(CMN, true, 0, 1, 1, 0), |c, _| {
        c.regs.r[0] = 0xDEAD;
        c.regs.r[1] = 0xFFFF_FFFF;
    });
    assert_eq!(cpu.regs.r[0], 0xDEAD);
    assert_eq!(flags(&cpu), (false, true, true, false));

    let (cpu, _) = arm1(dp_imm(TST, true, 0, 1, 0x80, 8), |c, _| {
        // #0x80 ror 16 = 0x00800000; the rotated immediate's bit 31 (0) becomes C.
        c.regs.r[1] = 0x0080_0000;
        c.regs.cpsr |= FLAG_C;
    });
    assert_eq!(flags(&cpu), (false, false, false, false));

    let (cpu, _) = arm1(dp_imm(TEQ, true, 0, 1, 0xFF, 0), |c, _| {
        c.regs.r[1] = 0xFF;
    });
    assert_eq!(flags(&cpu), (false, true, false, false));
}

#[test]
fn register_shift_uses_low_byte_and_adds_internal_cycle() {
    let (cpu, bus) = arm1(dp_regreg(MOV, true, 0, 0, 1, alu::LSR, 2), |c, _| {
        c.regs.r[1] = 0x8000_0000;
        c.regs.r[2] = 0x0000_011F; // low byte 0x1F
    });
    assert_eq!(cpu.regs.r[0], 1);
    assert!(!cpu.regs.flag_c());
    assert_eq!(bus.idles, 1);
}

#[test]
fn condition_codes() {
    use alu::condition_passed as cp;
    let n = FLAG_N;
    let z = FLAG_Z;
    let c = FLAG_C;
    let v = FLAG_V;
    assert!(cp(0x0, z) && !cp(0x0, 0));
    assert!(cp(0x1, 0) && !cp(0x1, z));
    assert!(cp(0x2, c) && !cp(0x2, 0));
    assert!(cp(0x3, 0) && !cp(0x3, c));
    assert!(cp(0x4, n) && !cp(0x4, 0));
    assert!(cp(0x5, 0) && !cp(0x5, n));
    assert!(cp(0x6, v) && !cp(0x6, 0));
    assert!(cp(0x7, 0) && !cp(0x7, v));
    assert!(cp(0x8, c) && !cp(0x8, c | z) && !cp(0x8, 0));
    assert!(cp(0x9, 0) && cp(0x9, c | z) && !cp(0x9, c));
    assert!(cp(0xA, n | v) && cp(0xA, 0) && !cp(0xA, n));
    assert!(cp(0xB, n) && cp(0xB, v) && !cp(0xB, n | v));
    assert!(cp(0xC, 0) && cp(0xC, n | v) && !cp(0xC, z) && !cp(0xC, n));
    assert!(cp(0xD, z) && cp(0xD, n) && !cp(0xD, 0) && !cp(0xD, n | v));
    assert!(cp(0xE, 0) && cp(0xE, n | z | c | v));
    assert!(!cp(0xF, 0) && !cp(0xF, n | z | c | v));
}

#[test]
fn multiply_cycle_counts() {
    assert_eq!(alu::multiply_cycles(0x0000_00FF, false), 1);
    assert_eq!(alu::multiply_cycles(0x0000_FFFF, false), 2);
    assert_eq!(alu::multiply_cycles(0x00FF_FFFF, false), 3);
    assert_eq!(alu::multiply_cycles(0xFFFF_FFFF, false), 4);
    assert_eq!(alu::multiply_cycles(0xFFFF_FFFF, true), 1);
    assert_eq!(alu::multiply_cycles(0xFFFF_FF00, true), 1);
    assert_eq!(alu::multiply_cycles(0xFFFF_0000, true), 2);
    assert_eq!(alu::multiply_cycles(0xFF00_0000, true), 3);
    assert_eq!(alu::multiply_cycles(0x7F00_0000, true), 4);
}
