//! ARM (32-bit) instruction execution. Source: GBATEK "ARM Opcodes: *" chapters and "ARM CPU
//! Instruction Cycle Times". `cpu.regs.r[15]` holds the address of the executing instruction;
//! operand reads of R15 see PC+8 (PC+12 for register-shifted operands and stored PC).

use super::alu::{self, ArithResult};
use super::decode::{decode_arm, ArmOp};
use super::regs::{self, FLAG_C, FLAG_T};
use super::Cpu;
use crate::bus::{Access, MemoryBus};

/// Execute one ARM opcode. `cpu.regs.r[15]` = its address.
pub fn execute<B: MemoryBus>(cpu: &mut Cpu, bus: &mut B, op: u32) {
    let cond = op >> 28;
    if cond != 0xE && !alu::condition_passed(cond, cpu.regs.cpsr) {
        return;
    }
    match decode_arm(op) {
        ArmOp::DataProcImm => data_processing(cpu, bus, op, Operand::Imm),
        ArmOp::DataProcRegImm => data_processing(cpu, bus, op, Operand::RegImm),
        ArmOp::DataProcRegReg => data_processing(cpu, bus, op, Operand::RegReg),
        ArmOp::Mrs => mrs(cpu, op),
        ArmOp::MsrReg => {
            let v = cpu.regs.r[(op & 0xF) as usize];
            msr(cpu, op, v);
        }
        ArmOp::MsrImm => {
            let (v, _) = alu::imm_operand(op & 0xFF, (op >> 8) & 0xF, cpu.regs.flag_c());
            msr(cpu, op, v);
        }
        ArmOp::Bx => branch_exchange(cpu, op),
        ArmOp::Multiply => multiply(cpu, bus, op),
        ArmOp::MultiplyLong => multiply_long(cpu, bus, op),
        ArmOp::Swp => swap(cpu, bus, op),
        ArmOp::HalfwordTransfer => halfword_transfer(cpu, bus, op),
        ArmOp::SingleTransfer => single_transfer(cpu, bus, op),
        ArmOp::BlockTransfer => block_transfer(cpu, bus, op),
        ArmOp::Branch => branch(cpu, op),
        ArmOp::Swi => cpu.software_interrupt(bus, ((op >> 16) & 0xFF) as u8),
        ArmOp::Undefined => cpu.undefined_instruction(bus),
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Operand {
    Imm,
    RegImm,
    RegReg,
}

/// Value of register `r` as an operand: PC reads as `pc_value` (PC+8 or PC+12).
#[inline]
fn reg(cpu: &Cpu, r: usize, pc_value: u32) -> u32 {
    if r == 15 {
        pc_value
    } else {
        cpu.regs.r[r]
    }
}

/// Resolve the shifted-register / immediate second operand. Returns (value, shifter carry-out).
#[inline]
fn operand2(cpu: &Cpu, op: u32, kind: Operand) -> (u32, bool) {
    let carry_in = cpu.regs.flag_c();
    match kind {
        Operand::Imm => alu::imm_operand(op & 0xFF, (op >> 8) & 0xF, carry_in),
        Operand::RegImm => {
            let rm = reg(cpu, (op & 0xF) as usize, cpu.regs.r[15].wrapping_add(8));
            alu::shift_imm(rm, (op >> 5) & 3, (op >> 7) & 0x1F, carry_in)
        }
        Operand::RegReg => {
            let pc12 = cpu.regs.r[15].wrapping_add(12);
            let rm = reg(cpu, (op & 0xF) as usize, pc12);
            let rs = reg(cpu, ((op >> 8) & 0xF) as usize, pc12) & 0xFF;
            alu::shift_reg(rm, (op >> 5) & 3, rs, carry_in)
        }
    }
}

fn data_processing<B: MemoryBus>(cpu: &mut Cpu, bus: &mut B, op: u32, kind: Operand) {
    let opcode = (op >> 21) & 0xF;
    let s = op & (1 << 20) != 0;
    let rn_idx = ((op >> 16) & 0xF) as usize;
    let rd = ((op >> 12) & 0xF) as usize;
    let pc_value = cpu.regs.r[15].wrapping_add(if kind == Operand::RegReg { 12 } else { 8 });
    let rn = reg(cpu, rn_idx, pc_value);
    let (op2, shifter_carry) = operand2(cpu, op, kind);
    if kind == Operand::RegReg {
        bus.idle(1);
    }
    let cpsr = cpu.regs.cpsr;
    let c_in = cpsr & FLAG_C != 0;

    // (result, new CPSR if S, writes Rd?)
    let (result, new_cpsr, write) = match opcode {
        0x0 => {
            let r = rn & op2;
            (r, alu::set_nzc(cpsr, r, shifter_carry), true)
        }
        0x1 => {
            let r = rn ^ op2;
            (r, alu::set_nzc(cpsr, r, shifter_carry), true)
        }
        0x2 => arith(cpsr, alu::sub(rn, op2), true),
        0x3 => arith(cpsr, alu::sub(op2, rn), true),
        0x4 => arith(cpsr, alu::add_with_carry(rn, op2, false), true),
        0x5 => arith(cpsr, alu::add_with_carry(rn, op2, c_in), true),
        0x6 => arith(cpsr, alu::sub_with_carry(rn, op2, c_in), true),
        0x7 => arith(cpsr, alu::sub_with_carry(op2, rn, c_in), true),
        0x8 => {
            let r = rn & op2;
            (r, alu::set_nzc(cpsr, r, shifter_carry), false)
        }
        0x9 => {
            let r = rn ^ op2;
            (r, alu::set_nzc(cpsr, r, shifter_carry), false)
        }
        0xA => arith(cpsr, alu::sub(rn, op2), false),
        0xB => arith(cpsr, alu::add_with_carry(rn, op2, false), false),
        0xC => {
            let r = rn | op2;
            (r, alu::set_nzc(cpsr, r, shifter_carry), true)
        }
        0xD => (op2, alu::set_nzc(cpsr, op2, shifter_carry), true),
        0xE => {
            let r = rn & !op2;
            (r, alu::set_nzc(cpsr, r, shifter_carry), true)
        }
        _ => {
            let r = !op2;
            (r, alu::set_nzc(cpsr, r, shifter_carry), true)
        }
    };

    if rd == 15 {
        if write {
            if s {
                // MOVS PC, LR and friends: CPSR = SPSR (mode/state switch), then branch in the new
                // state. Unpredictable in User/System mode; left as a plain PC write there.
                cpu.restore_cpsr_from_spsr();
            }
            cpu.set_pc(result);
        } else if s {
            // TSTP/TEQP/CMPP/CMNP (Rd=15, S=1): ARMv2 PSR write; on the ARM7TDMI the ARMv4 manual
            // says CPSR = SPSR. Follow that in privileged modes; flags otherwise.
            if regs::bank_index(cpu.regs.mode()) != 0 {
                cpu.restore_cpsr_from_spsr();
            } else {
                cpu.regs.cpsr = new_cpsr;
            }
        }
        return;
    }
    if s {
        cpu.regs.cpsr = new_cpsr;
    }
    if write {
        cpu.regs.r[rd] = result;
    }
}

#[inline]
fn arith(cpsr: u32, r: ArithResult, write: bool) -> (u32, u32, bool) {
    (r.value, alu::set_nzcv(cpsr, r), write)
}

fn mrs(cpu: &mut Cpu, op: u32) {
    let rd = ((op >> 12) & 0xF) as usize;
    let v = if op & (1 << 22) != 0 {
        cpu.regs.spsr()
    } else {
        cpu.regs.cpsr
    };
    if rd != 15 {
        cpu.regs.r[rd] = v;
    }
}

/// MSR with field mask (bits 19-16 = f,s,x,c). In User mode only the flag field is writable, and
/// the T bit is never changed by MSR (GBATEK: use BX for state switches).
fn msr(cpu: &mut Cpu, op: u32, value: u32) {
    let mut mask = 0u32;
    if op & (1 << 19) != 0 {
        mask |= 0xFF00_0000;
    }
    if op & (1 << 18) != 0 {
        mask |= 0x00FF_0000;
    }
    if op & (1 << 17) != 0 {
        mask |= 0x0000_FF00;
    }
    if op & (1 << 16) != 0 {
        mask |= 0x0000_00FF;
    }
    if op & (1 << 22) != 0 {
        let old = cpu.regs.spsr();
        cpu.regs.set_spsr((old & !mask) | (value & mask));
    } else {
        if cpu.regs.mode() == regs::MODE_USR {
            mask &= 0xFF00_0000;
        }
        mask &= !FLAG_T;
        let old = cpu.regs.cpsr;
        cpu.regs.set_cpsr((old & !mask) | (value & mask));
    }
}

fn branch_exchange(cpu: &mut Cpu, op: u32) {
    let rn = (op & 0xF) as usize;
    // BX R15 behaves as BX $+8 (GBATEK).
    let target = reg(cpu, rn, cpu.regs.r[15].wrapping_add(8));
    if target & 1 != 0 {
        cpu.regs.cpsr |= FLAG_T;
    } else {
        cpu.regs.cpsr &= !FLAG_T;
    }
    cpu.set_pc(target);
}

fn branch(cpu: &mut Cpu, op: u32) {
    let offset = ((op & 0x00FF_FFFF) << 8) as i32 >> 6; // sign-extend 24 bits, *4
    let pc = cpu.regs.r[15];
    if op & (1 << 24) != 0 {
        cpu.regs.r[14] = pc.wrapping_add(4);
    }
    cpu.set_pc(pc.wrapping_add(8).wrapping_add(offset as u32));
}

fn multiply<B: MemoryBus>(cpu: &mut Cpu, bus: &mut B, op: u32) {
    let rd = ((op >> 16) & 0xF) as usize;
    let rn = ((op >> 12) & 0xF) as usize;
    let rs = cpu.regs.r[((op >> 8) & 0xF) as usize];
    let rm = cpu.regs.r[(op & 0xF) as usize];
    let accumulate = op & (1 << 21) != 0;
    let mut cycles = alu::multiply_cycles(rs, true);
    let mut result = rm.wrapping_mul(rs);
    if accumulate {
        result = result.wrapping_add(cpu.regs.r[rn]);
        cycles += 1;
    }
    bus.idle(cycles);
    if rd != 15 {
        cpu.regs.r[rd] = result;
    }
    if op & (1 << 20) != 0 {
        // N, Z from the result; C is "destroyed" on ARMv4 (left unchanged here), V unaffected.
        cpu.regs.set_nz(result);
    }
}

fn multiply_long<B: MemoryBus>(cpu: &mut Cpu, bus: &mut B, op: u32) {
    let rd_hi = ((op >> 16) & 0xF) as usize;
    let rd_lo = ((op >> 12) & 0xF) as usize;
    let rs = cpu.regs.r[((op >> 8) & 0xF) as usize];
    let rm = cpu.regs.r[(op & 0xF) as usize];
    let signed = op & (1 << 22) != 0;
    let accumulate = op & (1 << 21) != 0;
    let mut cycles = alu::multiply_cycles(rs, signed) + 1;
    let mut result: u64 = if signed {
        (rm as i32 as i64).wrapping_mul(rs as i32 as i64) as u64
    } else {
        rm as u64 * rs as u64
    };
    if accumulate {
        let acc = (cpu.regs.r[rd_hi] as u64) << 32 | cpu.regs.r[rd_lo] as u64;
        result = result.wrapping_add(acc);
        cycles += 1;
    }
    bus.idle(cycles);
    if rd_lo != 15 {
        cpu.regs.r[rd_lo] = result as u32;
    }
    if rd_hi != 15 {
        cpu.regs.r[rd_hi] = (result >> 32) as u32;
    }
    if op & (1 << 20) != 0 {
        let n = result >> 63 != 0;
        cpu.regs.set_flag(regs::FLAG_N, n);
        cpu.regs.set_flag(regs::FLAG_Z, result == 0);
    }
}

fn swap<B: MemoryBus>(cpu: &mut Cpu, bus: &mut B, op: u32) {
    let rn = cpu.regs.r[((op >> 16) & 0xF) as usize];
    let rd = ((op >> 12) & 0xF) as usize;
    let rm = cpu.regs.r[(op & 0xF) as usize];
    let value = if op & (1 << 22) != 0 {
        let v = bus.read8(rn, Access::N) as u32;
        bus.write8(rn, rm as u8, Access::N);
        v
    } else {
        let v = cpu.load_word(bus, rn, Access::N);
        bus.write32(rn & !3, rm, Access::N);
        v
    };
    bus.idle(1);
    if rd != 15 {
        cpu.regs.r[rd] = value;
    } else {
        cpu.set_pc(value);
    }
}

/// Common addressing for single/halfword transfers: returns (transfer address, final base).
#[inline]
fn index_address(base: u32, offset: u32, pre: bool, up: bool) -> (u32, u32) {
    let indexed = if up {
        base.wrapping_add(offset)
    } else {
        base.wrapping_sub(offset)
    };
    if pre {
        (indexed, indexed)
    } else {
        (base, indexed)
    }
}

fn single_transfer<B: MemoryBus>(cpu: &mut Cpu, bus: &mut B, op: u32) {
    let pre = op & (1 << 24) != 0;
    let up = op & (1 << 23) != 0;
    let byte = op & (1 << 22) != 0;
    let writeback = !pre || op & (1 << 21) != 0;
    let load = op & (1 << 20) != 0;
    let rn = ((op >> 16) & 0xF) as usize;
    let rd = ((op >> 12) & 0xF) as usize;
    let pc = cpu.regs.r[15];
    let base = reg(cpu, rn, pc.wrapping_add(8));
    let offset = if op & (1 << 25) != 0 {
        let rm = cpu.regs.r[(op & 0xF) as usize];
        alu::shift_imm(rm, (op >> 5) & 3, (op >> 7) & 0x1F, cpu.regs.flag_c()).0
    } else {
        op & 0xFFF
    };
    let (addr, new_base) = index_address(base, offset, pre, up);
    if load {
        let v = if byte {
            bus.read8(addr, Access::N) as u32
        } else {
            cpu.load_word(bus, addr, Access::N)
        };
        bus.idle(1);
        if writeback {
            write_base(cpu, rn, new_base);
        }
        // The loaded value wins over writeback when Rd == Rn.
        if rd == 15 {
            cpu.set_pc(v);
        } else {
            cpu.regs.r[rd] = v;
        }
    } else {
        // Stored PC reads as PC+12; the stored value is Rd before any writeback.
        let v = reg(cpu, rd, pc.wrapping_add(12));
        if byte {
            bus.write8(addr, v as u8, Access::N);
        } else {
            // Unaligned address goes on the bus (SRAM byte-lane selection); RAM aligns it.
            bus.write32(addr, v, Access::N);
        }
        if writeback {
            write_base(cpu, rn, new_base);
        }
    }
}

/// Base writeback; a writeback into R15 is a branch.
#[inline]
fn write_base(cpu: &mut Cpu, rn: usize, v: u32) {
    if rn == 15 {
        cpu.set_pc(v);
    } else {
        cpu.regs.r[rn] = v;
    }
}

fn halfword_transfer<B: MemoryBus>(cpu: &mut Cpu, bus: &mut B, op: u32) {
    let pre = op & (1 << 24) != 0;
    let up = op & (1 << 23) != 0;
    let imm = op & (1 << 22) != 0;
    let writeback = !pre || op & (1 << 21) != 0;
    let load = op & (1 << 20) != 0;
    let rn = ((op >> 16) & 0xF) as usize;
    let rd = ((op >> 12) & 0xF) as usize;
    let sh = (op >> 5) & 3;
    let pc = cpu.regs.r[15];
    let base = reg(cpu, rn, pc.wrapping_add(8));
    let offset = if imm {
        ((op >> 4) & 0xF0) | (op & 0xF)
    } else {
        cpu.regs.r[(op & 0xF) as usize]
    };
    let (addr, new_base) = index_address(base, offset, pre, up);
    if load {
        let v = match sh {
            1 => cpu.load_half(bus, addr, Access::N),
            2 => bus.read8(addr, Access::N) as i8 as i32 as u32,
            3 => cpu.load_signed_half(bus, addr, Access::N),
            // SH=00 with L=1 is reserved; behaves as LDRH here.
            _ => cpu.load_half(bus, addr, Access::N),
        };
        bus.idle(1);
        if writeback {
            write_base(cpu, rn, new_base);
        }
        if rd == 15 {
            cpu.set_pc(v);
        } else {
            cpu.regs.r[rd] = v;
        }
    } else {
        // STRH (SH=01). SH=10/11 are LDRD/STRD on ARMv5 only; the ARM7 stores a halfword.
        let v = reg(cpu, rd, pc.wrapping_add(12));
        // The unaligned address goes on the bus: RAM ignores bit 0, but the 8-bit SRAM/Flash bus
        // uses it to pick the byte lane (jsmolka save tests, test 6).
        bus.write16(addr, v as u16, Access::N);
        if writeback {
            write_base(cpu, rn, new_base);
        }
    }
}

fn block_transfer<B: MemoryBus>(cpu: &mut Cpu, bus: &mut B, op: u32) {
    let pre = op & (1 << 24) != 0;
    let up = op & (1 << 23) != 0;
    let s = op & (1 << 22) != 0;
    let writeback = op & (1 << 21) != 0;
    let load = op & (1 << 20) != 0;
    let rn = ((op >> 16) & 0xF) as usize;
    let rlist = op & 0xFFFF;
    let pc_in_list = rlist & (1 << 15) != 0;
    // S bit: LDM with PC restores CPSR; otherwise transfer the User bank.
    let restore_cpsr = s && load && pc_in_list;
    let user_bank = s && !restore_cpsr;
    cpu.block_transfer(
        bus,
        rn,
        rlist,
        load,
        pre,
        up,
        writeback,
        user_bank,
        restore_cpsr,
    );
}
