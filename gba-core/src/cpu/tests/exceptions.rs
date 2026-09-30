//! Exception entry/return round trips: IRQ, SWI (real and HLE), undefined instruction.

use super::*;
use crate::cpu::alu;
use crate::cpu::{VEC_IRQ, VEC_SWI, VEC_UNDEFINED};

#[test]
fn irq_entry_and_return_arm() {
    let mut cpu = arm_cpu();
    let mut bus = TestBus::new();
    cpu.regs.cpsr |= FLAG_C;
    bus.load_arm(CODE, &[mov_imm(0, 1), mov_imm(1, 2), mov_imm(2, 3)]);
    // Vector 0x18: mov r5, #9 ; subs pc, lr, #4
    bus.load_arm(VEC_IRQ, &[mov_imm(5, 9), dp_imm(SUB, true, 15, 14, 4, 0)]);
    cpu.step(&mut bus);
    bus.irq = true;
    cpu.step(&mut bus);
    assert_eq!(cpu.regs.mode(), MODE_IRQ);
    assert!(cpu.regs.irq_disabled());
    assert!(!cpu.regs.thumb());
    assert_eq!(cpu.regs.r[14], CODE + 4 + 4);
    assert_eq!(cpu.regs.spsr(), MODE_SYS | FLAG_C);
    assert_eq!(cpu.pc(), VEC_IRQ);
    assert_eq!(cpu.regs.r[1], 0, "the interrupted instruction did not run");
    // While I is set the pending IRQ is ignored.
    cpu.step(&mut bus);
    assert_eq!(cpu.regs.r[5], 9);
    bus.irq = false;
    cpu.step(&mut bus);
    assert_eq!(cpu.regs.mode(), MODE_SYS);
    assert_eq!(cpu.regs.cpsr, MODE_SYS | FLAG_C);
    assert_eq!(cpu.pc(), CODE + 4);
    cpu.step(&mut bus);
    assert_eq!(cpu.regs.r[1], 2);
}

#[test]
fn irq_entry_from_thumb_returns_to_thumb() {
    let mut cpu = thumb_cpu();
    let mut bus = TestBus::new();
    bus.load_thumb(CODE, &[0x2001, 0x2102]); // mov r0,#1 ; mov r1,#2
    bus.load_arm(VEC_IRQ, &[dp_imm(SUB, true, 15, 14, 4, 0)]);
    cpu.step(&mut bus);
    bus.irq = true;
    cpu.step(&mut bus);
    assert!(!cpu.regs.thumb());
    assert_eq!(cpu.regs.r[14], CODE + 2 + 4);
    bus.irq = false;
    cpu.step(&mut bus);
    assert!(cpu.regs.thumb());
    assert_eq!(cpu.pc(), CODE + 2);
    cpu.step(&mut bus);
    assert_eq!(cpu.regs.r[1], 2);
}

#[test]
fn swi_real_exception_and_return() {
    let mut cpu = arm_cpu();
    let mut bus = TestBus::new();
    cpu.regs.cpsr |= FLAG_Z;
    bus.load_arm(CODE, &[swi(0x42), mov_imm(1, 2)]);
    // Vector 8: mov r0, #7 ; movs pc, lr
    bus.load_arm(
        VEC_SWI,
        &[mov_imm(0, 7), dp_reg(MOV, true, 15, 0, 14, alu::LSL, 0)],
    );
    cpu.step(&mut bus);
    assert_eq!(bus.swi_calls, vec![0x42]);
    assert_eq!(cpu.regs.mode(), MODE_SVC);
    assert!(cpu.regs.irq_disabled());
    assert_eq!(cpu.regs.r[14], CODE + 4);
    assert_eq!(cpu.regs.spsr(), MODE_SYS | FLAG_Z);
    assert_eq!(cpu.pc(), VEC_SWI);
    cpu.step(&mut bus);
    cpu.step(&mut bus);
    assert_eq!(cpu.regs.r[0], 7);
    assert_eq!(cpu.regs.mode(), MODE_SYS);
    assert!(cpu.regs.flag_z());
    assert_eq!(cpu.pc(), CODE + 4);
    cpu.step(&mut bus);
    assert_eq!(cpu.regs.r[1], 2);
}

#[test]
fn swi_from_thumb_sets_lr_to_next_halfword() {
    let mut cpu = thumb_cpu();
    let mut bus = TestBus::new();
    bus.load_thumb(CODE, &[0xDF05, 0x2102]);
    bus.load_arm(VEC_SWI, &[dp_reg(MOV, true, 15, 0, 14, alu::LSL, 0)]);
    cpu.step(&mut bus);
    assert_eq!(bus.swi_calls, vec![5]);
    assert_eq!(cpu.regs.r[14], CODE + 2);
    assert!(!cpu.regs.thumb());
    cpu.step(&mut bus);
    assert!(cpu.regs.thumb());
    assert_eq!(cpu.pc(), CODE + 2);
    cpu.step(&mut bus);
    assert_eq!(cpu.regs.r[1], 2);
}

#[test]
fn swi_hle_serviced_continues_without_exception() {
    let mut cpu = arm_cpu();
    let mut bus = TestBus::new();
    bus.hle = true;
    run_arm(&mut cpu, &mut bus, &[swi(6), mov_imm(1, 2)], 2);
    assert_eq!(bus.swi_calls, vec![6]);
    assert_eq!(cpu.regs.mode(), MODE_SYS);
    assert_eq!(cpu.regs.r[1], 2);
    assert_eq!(cpu.pc(), CODE + 8);
    // No pipeline flush happened: fetches stayed sequential.
    assert!(bus.fetches.iter().skip(1).all(|f| f.1 == Access::S));
}

#[test]
fn undefined_instruction_traps_to_vector_4() {
    let mut cpu = arm_cpu();
    let mut bus = TestBus::new();
    bus.load_arm(CODE, &[0xE600_0010, mov_imm(1, 2)]);
    bus.load_arm(VEC_UNDEFINED, &[dp_reg(MOV, true, 15, 0, 14, alu::LSL, 0)]);
    cpu.step(&mut bus);
    assert_eq!(cpu.regs.mode(), MODE_UND);
    assert_eq!(cpu.regs.r[14], CODE + 4);
    assert_eq!(cpu.pc(), VEC_UNDEFINED);
    assert_eq!(bus.idles, 1);
    cpu.step(&mut bus);
    assert_eq!(cpu.pc(), CODE + 4);
    assert_eq!(cpu.regs.mode(), MODE_SYS);
    // Coprocessor instructions are undefined on the GBA.
    let (cpu, _) = arm1(0xEE07_0F10, |_, _| {}); // mcr p15, ...
    assert_eq!(cpu.regs.mode(), MODE_UND);
    // THUMB undefined (bkpt encoding) traps with LR = PC+2.
    let (cpu, _) = thumb1(0xBE00, |_, _| {});
    assert_eq!(cpu.regs.mode(), MODE_UND);
    assert_eq!(cpu.regs.r[14], CODE + 2);
    assert!(!cpu.regs.thumb());
}

#[test]
fn irq_masked_by_i_flag() {
    let mut cpu = arm_cpu();
    let mut bus = TestBus::new();
    cpu.regs.cpsr |= FLAG_I;
    bus.irq = true;
    run_arm(&mut cpu, &mut bus, &[mov_imm(0, 1)], 1);
    assert_eq!(cpu.regs.r[0], 1);
    assert_eq!(cpu.regs.mode(), MODE_SYS);
}

#[test]
fn reset_states() {
    let mut cpu = Cpu::new();
    cpu.reset_hle();
    assert_eq!(cpu.regs.mode(), MODE_SYS);
    assert_eq!(cpu.regs.r[13], 0x0300_7F00);
    assert_eq!(cpu.regs.sp_of(MODE_IRQ), 0x0300_7FA0);
    assert_eq!(cpu.regs.sp_of(MODE_SVC), 0x0300_7FE0);
    assert_eq!(cpu.pc(), 0x0800_0000);
    assert!(!cpu.regs.irq_disabled());
    cpu.reset_lle();
    assert_eq!(cpu.regs.mode(), MODE_SVC);
    assert!(cpu.regs.irq_disabled());
    assert_eq!(cpu.pc(), 0);
}
