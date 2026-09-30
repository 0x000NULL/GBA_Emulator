//! THUMB (16-bit) instruction execution, formats 1-19. Source: GBATEK "THUMB Opcodes: *".
//! `cpu.regs.r[15]` holds the address of the executing instruction; R15 reads as PC+4.

use super::alu;
use super::decode::{decode_thumb, ThumbOp};
use super::regs::FLAG_T;
use super::Cpu;
use crate::bus::{Access, MemoryBus};

/// Execute one THUMB opcode. `cpu.regs.r[15]` = its address.
pub fn execute<B: MemoryBus>(cpu: &mut Cpu, bus: &mut B, op: u16) {
    let op = op as u32;
    match decode_thumb(op as u16) {
        ThumbOp::ShiftImm => shift_imm(cpu, op),
        ThumbOp::AddSub => add_sub(cpu, op),
        ThumbOp::Imm8 => imm8(cpu, op),
        ThumbOp::Alu => alu_op(cpu, bus, op),
        ThumbOp::HiReg => hi_reg(cpu, op),
        ThumbOp::LdrPc => {
            let rd = ((op >> 8) & 7) as usize;
            let addr = (cpu.regs.r[15].wrapping_add(4) & !2).wrapping_add((op & 0xFF) << 2);
            cpu.regs.r[rd] = cpu.load_word(bus, addr, Access::N);
            bus.idle(1);
        }
        ThumbOp::LdStReg => {
            let ro = cpu.regs.r[((op >> 6) & 7) as usize];
            let rb = cpu.regs.r[((op >> 3) & 7) as usize];
            let rd = (op & 7) as usize;
            let addr = rb.wrapping_add(ro);
            match (op >> 10) & 3 {
                0 => bus.write32(addr, cpu.regs.r[rd], Access::N),
                1 => bus.write8(addr, cpu.regs.r[rd] as u8, Access::N),
                2 => {
                    cpu.regs.r[rd] = cpu.load_word(bus, addr, Access::N);
                    bus.idle(1);
                }
                _ => {
                    cpu.regs.r[rd] = bus.read8(addr, Access::N) as u32;
                    bus.idle(1);
                }
            }
        }
        ThumbOp::LdStSigned => {
            let ro = cpu.regs.r[((op >> 6) & 7) as usize];
            let rb = cpu.regs.r[((op >> 3) & 7) as usize];
            let rd = (op & 7) as usize;
            let addr = rb.wrapping_add(ro);
            match (op >> 10) & 3 {
                0 => bus.write16(addr, cpu.regs.r[rd] as u16, Access::N),
                1 => {
                    cpu.regs.r[rd] = bus.read8(addr, Access::N) as i8 as i32 as u32;
                    bus.idle(1);
                }
                2 => {
                    cpu.regs.r[rd] = cpu.load_half(bus, addr, Access::N);
                    bus.idle(1);
                }
                _ => {
                    cpu.regs.r[rd] = cpu.load_signed_half(bus, addr, Access::N);
                    bus.idle(1);
                }
            }
        }
        ThumbOp::LdStImm => {
            let rb = cpu.regs.r[((op >> 3) & 7) as usize];
            let rd = (op & 7) as usize;
            let imm = (op >> 6) & 0x1F;
            match (op >> 11) & 3 {
                0 => bus.write32(rb.wrapping_add(imm << 2) & !3, cpu.regs.r[rd], Access::N),
                1 => {
                    cpu.regs.r[rd] = cpu.load_word(bus, rb.wrapping_add(imm << 2), Access::N);
                    bus.idle(1);
                }
                2 => bus.write8(rb.wrapping_add(imm), cpu.regs.r[rd] as u8, Access::N),
                _ => {
                    cpu.regs.r[rd] = bus.read8(rb.wrapping_add(imm), Access::N) as u32;
                    bus.idle(1);
                }
            }
        }
        ThumbOp::LdStHalf => {
            let rb = cpu.regs.r[((op >> 3) & 7) as usize];
            let rd = (op & 7) as usize;
            let addr = rb.wrapping_add(((op >> 6) & 0x1F) << 1);
            if op & (1 << 11) != 0 {
                cpu.regs.r[rd] = cpu.load_half(bus, addr, Access::N);
                bus.idle(1);
            } else {
                bus.write16(addr, cpu.regs.r[rd] as u16, Access::N);
            }
        }
        ThumbOp::LdStSp => {
            let rd = ((op >> 8) & 7) as usize;
            let addr = cpu.regs.r[13].wrapping_add((op & 0xFF) << 2);
            if op & (1 << 11) != 0 {
                cpu.regs.r[rd] = cpu.load_word(bus, addr, Access::N);
                bus.idle(1);
            } else {
                bus.write32(addr, cpu.regs.r[rd], Access::N);
            }
        }
        ThumbOp::AddrPcSp => {
            let rd = ((op >> 8) & 7) as usize;
            let imm = (op & 0xFF) << 2;
            cpu.regs.r[rd] = if op & (1 << 11) != 0 {
                cpu.regs.r[13].wrapping_add(imm)
            } else {
                (cpu.regs.r[15].wrapping_add(4) & !2).wrapping_add(imm)
            };
        }
        ThumbOp::AddSp => {
            let imm = (op & 0x7F) << 2;
            cpu.regs.r[13] = if op & (1 << 7) != 0 {
                cpu.regs.r[13].wrapping_sub(imm)
            } else {
                cpu.regs.r[13].wrapping_add(imm)
            };
        }
        ThumbOp::PushPop => {
            let pop = op & (1 << 11) != 0;
            let mut rlist = op & 0xFF;
            if op & (1 << 8) != 0 {
                rlist |= if pop { 1 << 15 } else { 1 << 14 };
            }
            // PUSH = STMDB SP!, POP = LDMIA SP!. POP {PC} keeps the THUMB state (ARMv4).
            cpu.block_transfer(bus, 13, rlist, pop, !pop, pop, true, false, false);
        }
        ThumbOp::LdmStm => {
            let rb = ((op >> 8) & 7) as usize;
            let load = op & (1 << 11) != 0;
            cpu.block_transfer(bus, rb, op & 0xFF, load, false, true, true, false, false);
        }
        ThumbOp::BCond => {
            let cond = (op >> 8) & 0xF;
            if alu::condition_passed(cond, cpu.regs.cpsr) {
                let offset = ((op & 0xFF) as i8 as i32) << 1;
                let pc = cpu.regs.r[15];
                cpu.set_pc(pc.wrapping_add(4).wrapping_add(offset as u32));
            }
        }
        ThumbOp::Swi => cpu.software_interrupt(bus, (op & 0xFF) as u8),
        ThumbOp::B => {
            let offset = (((op & 0x7FF) << 21) as i32) >> 20; // sign-extend 11 bits, *2
            let pc = cpu.regs.r[15];
            cpu.set_pc(pc.wrapping_add(4).wrapping_add(offset as u32));
        }
        ThumbOp::BlHi => {
            // LR = PC+4 + (nn << 12), sign-extended.
            let offset = (((op & 0x7FF) << 21) as i32) >> 9;
            cpu.regs.r[14] = cpu.regs.r[15].wrapping_add(4).wrapping_add(offset as u32);
        }
        ThumbOp::BlLo => {
            // PC = LR + (nn << 1), LR = address of next instruction | 1. Works stand-alone too
            // ("BL LR+imm", used by Mario Golf).
            let pc = cpu.regs.r[15];
            let target = cpu.regs.r[14].wrapping_add((op & 0x7FF) << 1);
            cpu.regs.r[14] = pc.wrapping_add(2) | 1;
            cpu.set_pc(target);
        }
        ThumbOp::Undefined => cpu.undefined_instruction(bus),
    }
}

/// Format 1: LSL/LSR/ASR Rd, Rs, #imm5 (flags N, Z, C; LSL#0 leaves C alone).
fn shift_imm(cpu: &mut Cpu, op: u32) {
    let ty = (op >> 11) & 3;
    let amount = (op >> 6) & 0x1F;
    let rs = cpu.regs.r[((op >> 3) & 7) as usize];
    let rd = (op & 7) as usize;
    let (v, c) = alu::shift_imm(rs, ty, amount, cpu.regs.flag_c());
    cpu.regs.r[rd] = v;
    cpu.regs.cpsr = alu::set_nzc(cpu.regs.cpsr, v, c);
}

/// Format 2: ADD/SUB Rd, Rs, Rn / #imm3.
fn add_sub(cpu: &mut Cpu, op: u32) {
    let rs = cpu.regs.r[((op >> 3) & 7) as usize];
    let rd = (op & 7) as usize;
    let operand = if op & (1 << 10) != 0 {
        (op >> 6) & 7
    } else {
        cpu.regs.r[((op >> 6) & 7) as usize]
    };
    let r = if op & (1 << 9) != 0 {
        alu::sub(rs, operand)
    } else {
        alu::add_with_carry(rs, operand, false)
    };
    cpu.regs.r[rd] = r.value;
    cpu.regs.cpsr = alu::set_nzcv(cpu.regs.cpsr, r);
}

/// Format 3: MOV/CMP/ADD/SUB Rd, #imm8.
fn imm8(cpu: &mut Cpu, op: u32) {
    let rd = ((op >> 8) & 7) as usize;
    let imm = op & 0xFF;
    match (op >> 11) & 3 {
        0 => {
            cpu.regs.r[rd] = imm;
            cpu.regs.set_nz(imm);
        }
        1 => cpu.regs.cpsr = alu::set_nzcv(cpu.regs.cpsr, alu::sub(cpu.regs.r[rd], imm)),
        2 => {
            let r = alu::add_with_carry(cpu.regs.r[rd], imm, false);
            cpu.regs.r[rd] = r.value;
            cpu.regs.cpsr = alu::set_nzcv(cpu.regs.cpsr, r);
        }
        _ => {
            let r = alu::sub(cpu.regs.r[rd], imm);
            cpu.regs.r[rd] = r.value;
            cpu.regs.cpsr = alu::set_nzcv(cpu.regs.cpsr, r);
        }
    }
}

/// Format 4: ALU operations Rd, Rs.
fn alu_op<B: MemoryBus>(cpu: &mut Cpu, bus: &mut B, op: u32) {
    let rs = cpu.regs.r[((op >> 3) & 7) as usize];
    let rd = (op & 7) as usize;
    let d = cpu.regs.r[rd];
    let cpsr = cpu.regs.cpsr;
    let c_in = cpu.regs.flag_c();
    match (op >> 6) & 0xF {
        0x0 => logical(cpu, rd, d & rs),
        0x1 => logical(cpu, rd, d ^ rs),
        0x2 | 0x3 | 0x4 | 0x7 => {
            let ty = match (op >> 6) & 0xF {
                0x2 => alu::LSL,
                0x3 => alu::LSR,
                0x4 => alu::ASR,
                _ => alu::ROR,
            };
            let (v, c) = alu::shift_reg(d, ty, rs & 0xFF, c_in);
            bus.idle(1);
            cpu.regs.r[rd] = v;
            cpu.regs.cpsr = alu::set_nzc(cpsr, v, c);
        }
        0x5 => arith_store(cpu, rd, alu::add_with_carry(d, rs, c_in)),
        0x6 => arith_store(cpu, rd, alu::sub_with_carry(d, rs, c_in)),
        0x8 => cpu.regs.set_nz(d & rs),
        0x9 => arith_store(cpu, rd, alu::sub(0, rs)),
        0xA => cpu.regs.cpsr = alu::set_nzcv(cpsr, alu::sub(d, rs)),
        0xB => cpu.regs.cpsr = alu::set_nzcv(cpsr, alu::add_with_carry(d, rs, false)),
        0xC => logical(cpu, rd, d | rs),
        0xD => {
            // MUL Rd, Rs: 1S + mI where m depends on the incoming Rd value (GBATEK).
            bus.idle(alu::multiply_cycles(d, true));
            let v = d.wrapping_mul(rs);
            cpu.regs.r[rd] = v;
            cpu.regs.set_nz(v);
        }
        0xE => logical(cpu, rd, d & !rs),
        _ => logical(cpu, rd, !rs),
    }
}

#[inline]
fn logical(cpu: &mut Cpu, rd: usize, v: u32) {
    cpu.regs.r[rd] = v;
    cpu.regs.set_nz(v);
}

#[inline]
fn arith_store(cpu: &mut Cpu, rd: usize, r: alu::ArithResult) {
    cpu.regs.r[rd] = r.value;
    cpu.regs.cpsr = alu::set_nzcv(cpu.regs.cpsr, r);
}

/// Format 5: ADD/CMP/MOV with hi registers, BX.
fn hi_reg(cpu: &mut Cpu, op: u32) {
    let rs_idx = (((op >> 3) & 7) | ((op >> 3) & 8)) as usize;
    let rd_idx = ((op & 7) | ((op >> 4) & 8)) as usize;
    let pc4 = cpu.regs.r[15].wrapping_add(4);
    let rs = if rs_idx == 15 {
        pc4
    } else {
        cpu.regs.r[rs_idx]
    };
    match (op >> 8) & 3 {
        0 => {
            let d = if rd_idx == 15 {
                pc4
            } else {
                cpu.regs.r[rd_idx]
            };
            write_reg(cpu, rd_idx, d.wrapping_add(rs));
        }
        1 => {
            let d = if rd_idx == 15 {
                pc4
            } else {
                cpu.regs.r[rd_idx]
            };
            cpu.regs.cpsr = alu::set_nzcv(cpu.regs.cpsr, alu::sub(d, rs));
        }
        2 => write_reg(cpu, rd_idx, rs),
        _ => {
            // BX Rs: bit 0 selects THUMB. BX PC switches to ARM at (PC+4) & !2.
            if rs & 1 != 0 {
                cpu.regs.cpsr |= FLAG_T;
            } else {
                cpu.regs.cpsr &= !FLAG_T;
            }
            cpu.set_pc(rs);
        }
    }
}

/// Register write from format 5; writing PC is a branch (bit 0 ignored, stays THUMB).
#[inline]
fn write_reg(cpu: &mut Cpu, rd: usize, v: u32) {
    if rd == 15 {
        cpu.set_pc(v);
    } else {
        cpu.regs.r[rd] = v;
    }
}
