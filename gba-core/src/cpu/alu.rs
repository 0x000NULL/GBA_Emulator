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

// ---------------------------------------------------------------------------------------------
// Multiply carry flag.
//
// GBATEK says the C flag is "destroyed" by MUL/MLA/xMULL/xMLAL with S set. The ARM7TDMI
// multiplier is a Booth-recoded carry-save array that consumes 8 multiplier bits per cycle with
// early termination, and the carry flag is whatever falls out of the final shifter. The model
// below is the reconstruction published by zaydlang ("ARM7TDMI multiplication algorithm",
// https://bmchtech.github.io/post/multiply/, reference code github.com/zaydlang/multiplication-algorithm),
// verified against hardware by the mgba test suite ("Multiply long tests"). Only the carry is
// taken from it; the product itself comes from ordinary wrapping arithmetic.
// ---------------------------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum MulKind {
    /// MUL / MLA (32-bit result, signed operands).
    Short,
    /// SMULL / SMLAL.
    LongSigned,
    /// UMULL / UMLAL.
    LongUnsigned,
}

#[inline]
fn bit(x: u64, n: u32) -> u64 {
    (x >> n) & 1
}

#[inline]
fn mask(lo: u32, hi: u32) -> u64 {
    let size = hi - lo;
    if size >= 64 {
        !0u64 << lo
    } else {
        ((1u64 << size) - 1) << lo
    }
}

#[inline]
fn sign_extend(value: u64, from: u32, to: u32) -> u64 {
    if bit(value, from - 1) != 0 {
        value | mask(from, to)
    } else {
        value
    }
}

#[inline]
fn asr(value: u64, shift: u32, size: u32) -> u64 {
    let s = sign_extend(value, size, 64) as i64 >> shift;
    s as u64 & mask(0, size)
}

/// Booth recoding of one 3-bit multiplier chunk: (addend, carry-in caused by the negation).
#[inline]
fn booth_recode(input: u64, chunk: u64) -> (u64, u64) {
    let (o, c) = match chunk {
        1 | 2 => (input, 0),
        3 => (input.wrapping_mul(2), 0),
        4 => (!(input.wrapping_mul(2)), 1),
        5 | 6 => (!input, 1),
        _ => (0, 0),
    };
    (o & 0x3_FFFF_FFFF, c)
}

#[inline]
fn csa(a: u64, b: u64, c: u64) -> (u64, u64) {
    (a ^ b ^ c, (a & b) | (b & c) | (c & a))
}

/// One multiplier cycle: four CSA stages over the next 8 multiplier bits.
fn csa_array(partial_sum: u64, partial_carry: u64, multiplicand: u64, multiplier: u64, acc_shift: &mut u64) -> (u64, u64) {
    let mut out = partial_sum;
    let mut carry = partial_carry;
    let mut final_out = 0u64;
    let mut final_carry = 0u64;
    for i in 0..4 {
        let (addend, addend_carry) = booth_recode(multiplicand, (multiplier >> (2 * i)) & 7);
        out &= 0x1_FFFF_FFFF;
        carry &= 0x1_FFFF_FFFF;
        let (mut ro, mut rc) = csa(out, addend & 0x1_FFFF_FFFF, carry);
        // Inject the carry caused by the Booth negation.
        rc = (rc << 1) | addend_carry;
        // The bottom two bits are final: later addends are at least 4x larger.
        final_out |= (ro & 3) << (2 * i);
        final_carry |= (rc & 3) << (2 * i);
        ro >>= 2;
        rc >>= 2;
        // Sign-extension handling of the partial sum / carry (the "TransH"/"High" tables).
        let magic = bit(*acc_shift, 0) + (bit(carry, 32) ^ 1) + (bit(addend, 33) ^ 1);
        ro |= magic << 31;
        rc |= (bit(*acc_shift, 1) ^ 1) << 32;
        *acc_shift >>= 2;
        out = ro;
        carry = rc;
    }
    final_out |= out << 8;
    final_carry |= carry << 8;
    (final_out, final_carry)
}

#[inline]
fn ror128(lo: u64, hi: u64, shift: u32) -> (u64, u64) {
    ((lo >> shift) | (hi << (64 - shift)), (hi >> shift) | (lo << (64 - shift)))
}

#[inline]
fn adder(a: u32, b: u32, carry: bool) -> (u32, bool) {
    let real = a as u64 + b as u64 + carry as u64;
    (real as u32, real > u32::MAX as u64)
}

/// Carry flag left by an S-suffixed multiply of `multiplicand` (Rm) by `multiplier` (Rs) with
/// `accumulator` folded in (0 when not accumulating; RdHi:RdLo for xMLAL, Rn for MLA).
pub fn multiply_carry(kind: MulKind, multiplicand: u32, multiplier: u32, accumulator: u64) -> bool {
    let signed = kind != MulKind::LongUnsigned;
    let long = kind != MulKind::Short;
    let alu_carry_in = multiplier & 1 != 0;
    let mut multiplier = if signed { sign_extend(multiplier as u64, 32, 34) } else { multiplier as u64 & 0x1_FFFF_FFFF };
    let multiplicand = if signed { sign_extend(multiplicand as u64, 32, 34) } else { multiplicand as u64 & 0x1_FFFF_FFFF };

    let mut csa_out = accumulator;
    let mut csa_carry = if multiplier & 1 != 0 { !multiplicand } else { 0 };
    let mut acc_shift = accumulator >> 34;

    let (mut ps_lo, mut ps_hi) = (csa_out & 1, 0u64);
    let (mut pc_lo, mut pc_hi) = (csa_carry & 1, 0u64);
    csa_out >>= 1;
    csa_carry >>= 1;
    (ps_lo, ps_hi) = ror128(ps_lo, ps_hi, 1);
    (pc_lo, pc_hi) = ror128(pc_lo, pc_hi, 1);

    let mut iterations = 0u32;
    loop {
        (csa_out, csa_carry) = csa_array(csa_out, csa_carry, multiplicand, multiplier, &mut acc_shift);
        ps_lo |= csa_out & 0xFF;
        pc_lo |= csa_carry & 0xFF;
        csa_out >>= 8;
        csa_carry >>= 8;
        (ps_lo, ps_hi) = ror128(ps_lo, ps_hi, 8);
        (pc_lo, pc_hi) = ror128(pc_lo, pc_hi, 8);
        multiplier = asr(multiplier, 8, 33);
        iterations += 1;
        let terminate = if signed { multiplier == 0x1_FFFF_FFFF || multiplier == 0 } else { multiplier == 0 };
        if terminate {
            break;
        }
    }
    ps_lo |= csa_out;
    pc_lo |= csa_carry;

    let correction = match iterations {
        1 => 23,
        2 => 15,
        3 => 7,
        _ => 31,
    };
    (ps_lo, ps_hi) = ror128(ps_lo, ps_hi, correction);
    (pc_lo, pc_hi) = ror128(pc_lo, pc_hi, correction);
    let _ = (ps_lo, ps_hi, pc_lo, alu_carry_in, adder);

    if long || iterations != 4 {
        bit(pc_hi, 63) != 0
    } else {
        bit(pc_hi, 31) != 0
    }
}

#[cfg(test)]
mod multiply_carry_tests {
    use super::*;

    // Expected CPSR values from the mgba test suite's multiply-long table (hardware-measured).
    #[test]
    fn umull_carry_matches_hardware_table() {
        let c = |rm: u32, rs: u32| multiply_carry(MulKind::LongUnsigned, rm, rs, 0);
        assert!(!c(1, 1));
        assert!(!c(0xFFFF_FFFF, 0));
        assert!(c(0xFFFF_FFFF, 0xFFFF_FFFF)); // -1 * -1: CPSR A
        assert!(c(0x7FFF_FFFF, 0xFFFF_FFFF)); // $7F * -1: CPSR 2
        assert!(c(0xFFFF_FFFF, 0x7FFF_FFFF)); // -1 * $7F: CPSR 2
        assert!(!c(0x8000_0000, 0xFFFF_FFFF)); // $80 * -1 passed with C clear
    }

    #[test]
    fn smull_carry_matches_hardware_table() {
        let c = |rm: u32, rs: u32| multiply_carry(MulKind::LongSigned, rm, rs, 0);
        assert!(!c(0xFFFF_FFFF, 0xFFFF_FFFF)); // -1 * -1 = 1: CPSR 0
        assert!(c(0xFFFF_FFFF, 0x7FFF_FFFF)); // -1 * $7F: CPSR A
        assert!(c(0x8000_0000, 0x7FFF_FFFF)); // $80 * $7F: CPSR A
        assert!(!c(1, 1));
    }
}
