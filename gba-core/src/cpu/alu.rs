//! Barrel shifter, arithmetic flag helpers and condition evaluation shared by the ARM and THUMB
//! interpreters. Source: GBATEK "ARM Opcodes: Data Processing (ALU)" (zero shift amount rules,
//! returned CPSR flags), "ARM CPU Flags & Condition Field", "THUMB Opcodes: Register Operations".

use super::regs::{FLAG_C, FLAG_N, FLAG_V, FLAG_Z};

/// Shift type field (bits 6-5 of an ARM data-processing operand).
pub const LSL: u32 = 0;
pub const LSR: u32 = 1;
pub const ASR: u32 = 2;
pub const ROR: u32 = 3;

/// Shift by an *immediate* amount as encoded in the opcode (0-31), applying the special meaning of
/// amount 0: LSL#0 = no shift (carry unchanged), LSR#0 = LSR#32, ASR#0 = ASR#32, ROR#0 = RRX.
/// Returns (result, shifter carry-out).
#[inline]
pub fn shift_imm(val: u32, ty: u32, amount: u32, carry_in: bool) -> (u32, bool) {
    match ty {
        LSL => {
            if amount == 0 {
                (val, carry_in)
            } else {
                (val << amount, (val >> (32 - amount)) & 1 != 0)
            }
        }
        LSR => {
            if amount == 0 {
                (0, val >> 31 != 0)
            } else {
                (val >> amount, (val >> (amount - 1)) & 1 != 0)
            }
        }
        ASR => {
            if amount == 0 {
                let neg = val >> 31 != 0;
                (if neg { 0xFFFF_FFFF } else { 0 }, neg)
            } else {
                (
                    ((val as i32) >> amount) as u32,
                    (val >> (amount - 1)) & 1 != 0,
                )
            }
        }
        _ => {
            if amount == 0 {
                // RRX
                ((val >> 1) | ((carry_in as u32) << 31), val & 1 != 0)
            } else {
                (val.rotate_right(amount), (val >> (amount - 1)) & 1 != 0)
            }
        }
    }
}

/// Shift by a *register-specified* amount (the low 8 bits of Rs, 0-255). Amount 0 leaves the value
/// and carry unchanged for every shift type; amounts of 32 and above follow the ARM7TDMI rules
/// (LSL/LSR by 32 shift out the last bit into C, larger amounts give 0 with C=0; ASR saturates to
/// the sign; ROR by a multiple of 32 leaves the value and sets C to bit 31).
#[inline]
pub fn shift_reg(val: u32, ty: u32, amount: u32, carry_in: bool) -> (u32, bool) {
    if amount == 0 {
        return (val, carry_in);
    }
    match ty {
        LSL => {
            if amount < 32 {
                (val << amount, (val >> (32 - amount)) & 1 != 0)
            } else if amount == 32 {
                (0, val & 1 != 0)
            } else {
                (0, false)
            }
        }
        LSR => {
            if amount < 32 {
                (val >> amount, (val >> (amount - 1)) & 1 != 0)
            } else if amount == 32 {
                (0, val >> 31 != 0)
            } else {
                (0, false)
            }
        }
        ASR => {
            if amount < 32 {
                (
                    ((val as i32) >> amount) as u32,
                    (val >> (amount - 1)) & 1 != 0,
                )
            } else {
                let neg = val >> 31 != 0;
                (if neg { 0xFFFF_FFFF } else { 0 }, neg)
            }
        }
        _ => {
            let r = amount & 31;
            if r == 0 {
                (val, val >> 31 != 0)
            } else {
                (val.rotate_right(r), (val >> (r - 1)) & 1 != 0)
            }
        }
    }
}

/// ARM data-processing immediate operand: 8-bit value rotated right by `rot*2`. The shifter
/// carry-out is bit 31 of the result when a rotation is applied, otherwise the incoming carry.
#[inline]
pub fn imm_operand(imm8: u32, rot: u32, carry_in: bool) -> (u32, bool) {
    let r = rot * 2;
    if r == 0 {
        (imm8, carry_in)
    } else {
        let v = imm8.rotate_right(r);
        (v, v >> 31 != 0)
    }
}

/// Pack the NZCV result of an arithmetic operation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ArithResult {
    pub value: u32,
    pub carry: bool,
    pub overflow: bool,
}

/// a + b + carry_in with carry and signed-overflow.
#[inline]
pub fn add_with_carry(a: u32, b: u32, carry_in: bool) -> ArithResult {
    let wide = a as u64 + b as u64 + carry_in as u64;
    let value = wide as u32;
    ArithResult {
        value,
        carry: wide >> 32 != 0,
        overflow: (!(a ^ b) & (a ^ value)) >> 31 != 0,
    }
}

/// a - b: carry is set when no borrow occurred (a >= b unsigned).
#[inline]
pub fn sub(a: u32, b: u32) -> ArithResult {
    let value = a.wrapping_sub(b);
    ArithResult {
        value,
        carry: a >= b,
        overflow: ((a ^ b) & (a ^ value)) >> 31 != 0,
    }
}

/// a - b - !carry_in (SBC/RSC), computed as a + NOT b + carry.
#[inline]
pub fn sub_with_carry(a: u32, b: u32, carry_in: bool) -> ArithResult {
    let wide = a as u64 + (!b) as u64 + carry_in as u64;
    let value = wide as u32;
    ArithResult {
        value,
        carry: wide >> 32 != 0,
        overflow: ((a ^ b) & (a ^ value)) >> 31 != 0,
    }
}

/// New CPSR with N and Z from `value` and C from `carry`; V untouched (logical operations).
#[inline]
pub fn set_nzc(cpsr: u32, value: u32, carry: bool) -> u32 {
    (cpsr & !(FLAG_N | FLAG_Z | FLAG_C))
        | (value & FLAG_N)
        | if value == 0 { FLAG_Z } else { 0 }
        | if carry { FLAG_C } else { 0 }
}

/// New CPSR with N, Z, C and V from an arithmetic result.
#[inline]
pub fn set_nzcv(cpsr: u32, r: ArithResult) -> u32 {
    (cpsr & !(FLAG_N | FLAG_Z | FLAG_C | FLAG_V))
        | (r.value & FLAG_N)
        | if r.value == 0 { FLAG_Z } else { 0 }
        | if r.carry { FLAG_C } else { 0 }
        | if r.overflow { FLAG_V } else { 0 }
}

/// Evaluate an ARM condition code against CPSR. Code 0xF (NV) never executes on the ARM7TDMI
/// (the ARMv5 unconditional space is not decoded).
#[inline]
pub fn condition_passed(cond: u32, cpsr: u32) -> bool {
    let n = cpsr & FLAG_N != 0;
    let z = cpsr & FLAG_Z != 0;
    let c = cpsr & FLAG_C != 0;
    let v = cpsr & FLAG_V != 0;
    match cond {
        0x0 => z,
        0x1 => !z,
        0x2 => c,
        0x3 => !c,
        0x4 => n,
        0x5 => !n,
        0x6 => v,
        0x7 => !v,
        0x8 => c && !z,
        0x9 => !c || z,
        0xA => n == v,
        0xB => n != v,
        0xC => !z && n == v,
        0xD => z || n != v,
        0xE => true,
        _ => false,
    }
}

/// Number of internal cycles a multiply spends on multiplier `rs`: 1 if bits 31-8 are all zero
/// (or, when `signed`, all one), 2 for bits 31-16, 3 for bits 31-24, otherwise 4.
/// Source: GBATEK "Multiply and Multiply-Accumulate" execution time.
#[inline]
pub fn multiply_cycles(rs: u32, signed: bool) -> u32 {
    let top = |shift: u32| -> bool {
        let hi = rs >> shift;
        hi == 0 || (signed && hi == (0xFFFF_FFFFu32 >> shift))
    };
    if top(8) {
        1
    } else if top(16) {
        2
    } else if top(24) {
        3
    } else {
        4
    }
}
