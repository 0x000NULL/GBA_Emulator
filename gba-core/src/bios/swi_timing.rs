//! Cycle counts of the real BIOS SWI routines, derived from their instruction sequences.
//!
//! Source: the public disassembly of the GBA BIOS (camthesaxman/gba_bios, `asm/bios.s`): the SWI
//! vector at 0x08 -> handler at 0x140, branch table, Div (0x3B4), Sqrt (0x404), ArcTan (0x474),
//! ArcTan2 (0x4FC), CpuSet (0xB4C), CpuFastSet (0xBC4).
//!
//! Model: the BIOS sits on the 32-bit bus with no waitstates and runs from ROM-less, stall-free
//! memory, so every instruction fetch is 1 cycle. Cycle counts per instruction (ARM7TDMI,
//! including the instruction's own fetch slot):
//!
//! - data processing, MRS/MSR, untaken or skipped conditional: 1
//! - taken branch / `bx` / `bl` (ARM) / write to PC: 1 + 2 refill fetches = 3
//! - Thumb `bl` (two halves): 1 + 3 = 4
//! - `ldr`/`ldrh`/`ldrb`: 1 fetch + data access + 1 internal; `str`: 1 fetch + data access
//! - `ldm` of n registers: 1 fetch + n data + 1 internal; `stm`: 1 fetch + n data
//! - `mul`: 1 + m, m = 1..=4 from the magnitude of Rs (early termination)
//!
//! Only the BIOS-internal cycles are counted here ("fetch + internal"); data accesses to
//! memory are made through the bus by the HLE copy loops, which charges the region's waitstates.
//! The two fetches that refill the pipeline in the caller's region when the handler returns
//! (`movs pc, lr`) are paid by the CPU after the HLE call, not here.
//!
//! No constants are fitted: every number below is a count of instructions in the listing.
//! Hardware-recorded values in mgba-suite "Timing tests" (BIOS Division/Sqrt/ArcTan/CpuSet) were
//! used only to confirm the sums.

/// `b swi_vector` .. `bx r12` of the SWI handler: vector fetch pair 2, `b` 3, `push {r11,r12,lr}` 4,
/// `ldrb` 3, `adr` 1, `ldr r12` 3, `mrs` 1, `stmdb {r11}` 2, `and` 1, `orr` 1, `msr` 1,
/// `push {r2,lr}` 3, `adr` 1, `bx r12` 3.
pub const SWI_ENTER: u32 = 2 + 3 + 4 + 3 + 1 + 3 + 1 + 2 + 1 + 1 + 1 + 3 + 1 + 3;

/// Cycles of the entry sequence before the data cycle of `ldrb r12, [lr, #-2]`: vector fetch
/// pair 2, `b` 3, `push {r11,r12,lr}` 4, and the `ldrb`'s own fetch slot 1. The data cycle is
/// not counted in [`SWI_ENTER`]'s 3 for the `ldrb` as a flat 1: the HLE performs the real read
/// (the comment byte lives in the caller's memory) and `SWI_ENTER` includes 1 for it.
pub const SWI_BEFORE_LDRB: u32 = 2 + 3 + 4 + 1;

/// `swi_complete` up to and including the `movs pc, lr` fetch slot (its two refill fetches from
/// the caller's region are charged by the CPU): `pop {r2,lr}` 4, `mov` 1, `msr` 1, `ldm {r11}` 3,
/// `msr` 1, `pop {r11,r12,lr}` 5, `movs pc,lr` 1.
pub const SWI_LEAVE: u32 = 4 + 1 + 1 + 3 + 1 + 5 + 1;

/// Fixed entry + exit cost of a routed SWI.
pub const SWI_FIXED: u32 = SWI_ENTER + SWI_LEAVE;

/// Multiply cycles (whole instruction) for `mul rd, rm, rs` with the given Rs value.
#[inline]
pub fn mul_cycles(rs: u32) -> u32 {
    let m = if rs & 0xFFFF_FF00 == 0 || rs & 0xFFFF_FF00 == 0xFFFF_FF00 {
        1
    } else if rs & 0xFFFF_0000 == 0 || rs & 0xFFFF_0000 == 0xFFFF_0000 {
        2
    } else if rs & 0xFF00_0000 == 0 || rs & 0xFF00_0000 == 0xFF00_0000 {
        3
    } else {
        4
    };
    1 + m
}

/// The shared unsigned divide loop (`cmp r2,r0,lsr #1 / lslls / blo` then
/// `cmp / adc / subhs / teq / lsrne / bne`) at 0x3C8 and 0x430. Returns (cycles, quotient).
pub fn udiv_loops(r0: u32, r1: u32) -> (u32, u32) {
    let mut c = 0;
    let mut r0 = r0;
    let mut r2 = r1;
    // Loop 1: align the divisor. 2 instructions + branch (3 taken, 1 not taken).
    for _ in 0..40 {
        let half = r0 >> 1;
        c += 2;
        let lo = r2 < half;
        if r2 <= half {
            r2 <<= 1;
        }
        if lo {
            c += 3;
        } else {
            c += 1;
            break;
        }
    }
    // Loop 2: shift-subtract. cmp, adc, subhs, teq = 4, then lsrne + bne.
    let mut q = 0u32;
    for _ in 0..40 {
        c += 4;
        let hs = r0 >= r2;
        q = (q << 1) | hs as u32;
        if hs {
            r0 -= r2;
        }
        if r2 == r1 {
            c += 2;
            break;
        }
        r2 >>= 1;
        c += 4;
    }
    (c, q)
}

/// Div (0x3B4; DivArm enters at 0x3A8 with three extra `mov`s). BIOS cycles of the routine body.
pub fn div_cycles(num: i32, den: i32) -> u32 {
    if den == 0 {
        // The real BIOS hangs here; charge the shortest finite path.
        return 5 + 3 + 6 + 5 + 3;
    }
    // ands, rsbmi, eors, rsbhs, movs
    let head = 5;
    let (loops, _) = udiv_loops(num.unsigned_abs(), den.unsigned_abs());
    // mov, mov, lsls, rsbhs, rsbmi, bx lr
    head + loops + 5 + 3
}

/// DivArm: r0 and r1 swapped first (3 `mov`s).
pub fn div_arm_cycles(num: i32, den: i32) -> u32 {
    3 + div_cycles(num, den)
}

/// Sqrt (0x404): a binary guess followed by Newton iterations using the same divide loop.
pub fn sqrt_cycles(x: u32) -> u32 {
    // stmdb {r4} 2, mov r12, mov r1.
    let mut c = 2 + 1 + 1;
    let mut r0 = x;
    let mut g = 1u32;
    loop {
        c += 1;
        if r0 > g {
            r0 >>= 1;
            g <<= 1;
            c += 2 + 3;
        } else {
            c += 2 + 1;
            break;
        }
    }
    for _ in 0..64 {
        // mov r0, mov r4, mov r3, mov r2
        c += 4;
        let (d, q) = udiv_loops(x, g);
        c += d;
        let next = g.wrapping_add(q) >> 1;
        // add, lsrs, cmp
        c += 3;
        if next < g {
            c += 3;
            g = next;
        } else {
            c += 1;
            break;
        }
    }
    // mov r0,r4; ldm sp!,{r4} (3); bx lr
    c + 1 + 3 + 3
}

/// ArcTan (0x474): nine multiplies with data-dependent Rs plus 27 single-cycle instructions.
pub fn arctan_cycles(i: i32) -> u32 {
    let a = (i.wrapping_mul(i) >> 14).wrapping_neg();
    let mut c = 27 + mul_cycles(i as u32) + mul_cycles(0xA9);
    let mut b = (0xA9i32.wrapping_mul(a) >> 14).wrapping_add(0x390);
    for k in [0x91Ci32, 0xFB6, 0x16AA, 0x2081, 0x3651, 0xA2F9] {
        c += mul_cycles(b as u32);
        b = (b.wrapping_mul(a) >> 14).wrapping_add(k);
    }
    c + mul_cycles(i as u32)
}

/// ArcTan2 (0x4FC, Thumb): quadrant selection around one Div and one ArcTan call.
pub fn arctan2_cycles(x: i32, y: i32) -> u32 {
    const BL: u32 = 4; // Thumb bl
    const STUB: u32 = 4; // adr + bx into the ARM routine
    let div = |n: i32, d: i32| BL + STUB + div_cycles(n, d);
    let atan = |v: i32| BL + STUB + arctan_cycles(v);
    // Shared tail at 0x59E: pop {r4-r7} 6, pop {r3} 3, bx r3 3.
    const TAIL: u32 = 6 + 3 + 3;
    // push {r4-r7,lr} 6, cmp r1.
    let pro = 6 + 1;
    // Divide then arctan: (y<<14)/x or (x<<14)/y.
    let dv = |n: i32, d: i32| {
        let q = if d == 0 { 0 } else { (n << 14).wrapping_div(d) };
        div(n << 14, d) + atan(q)
    };
    if y == 0 {
        // bne nt 1, cmp 1, then blt taken (3) + mov,lsl,b (5) or nt 1 + movs 1 + b 3
        return pro + 1 + 1 + if x < 0 { 3 + 5 } else { 1 + 1 + 3 } + TAIL;
    }
    // bne taken 3, cmp r0
    let mut c = pro + 3 + 1;
    if x == 0 {
        // bne nt 1, cmp r1, blt (taken 3 / nt 1), mov, lsl, b 3
        return c + 1 + 1 + if y < 0 { 3 } else { 1 } + 2 + 3 + TAIL;
    }
    // bne taken 3 to 0x524, then the 9-instruction setup and cmp r1,#0
    c += 3 + 9 + 1;
    if y >= 0 {
        c += 1; // blt nt
        c += 1; // cmp r0,#0
        if x >= 0 {
            c += 1; // blt nt
            c += 1; // cmp r0,r1
            if x >= y {
                // blt nt, add, add, div(y<<14, x), atan, b
                c += 1 + 2 + dv(y, x) + 3;
            } else {
                // blt taken (550): add, div(x<<14, y), atan, sub, b
                c += 3 + 1 + dv(x, y) + 1 + 3;
            }
        } else {
            c += 3; // blt taken to 0x55E
            c += 1; // cmp r4,r1
            if -x >= y {
                // blt nt -> 0x562: add, add, div, atan, add, b
                c += 1 + 2 + dv(y, x) + 1 + 3;
            } else {
                c += 3 + 1 + dv(x, y) + 1 + 3;
            }
        }
    } else {
        c += 3; // blt taken to 0x572
        c += 1; // cmp r0,#0
        if x <= 0 {
            c += 1; // bgt nt
            c += 1; // cmp r4,r5
            if -x > -y {
                // bgt taken -> 0x562
                c += 3 + 2 + dv(y, x) + 1 + 3;
            } else {
                // 0x57A: add, div, atan, add, sub, b
                c += 1 + 1 + dv(x, y) + 1 + 1 + 3;
            }
        } else {
            c += 3; // bgt taken to 0x58A
            c += 1; // cmp r0,r5
            if x >= -y {
                // blt nt: add, add, div, atan, add, add (falls into the tail)
                c += 1 + 2 + dv(y, x) + 2;
            } else {
                c += 3 + 1 + dv(x, y) + 1 + 1 + 3;
            }
        }
    }
    c + TAIL
}

/// BIOS-internal cycles of the routine that precedes the copy loop: shared by CpuSet and
/// CpuFastSet `sub_BA4` (range check) plus the decision branches. `empty`: count 0.
#[inline]
fn range_check(empty: bool) -> u32 {
    if empty {
        // cmp, beq taken, bx lr
        1 + 3 + 3
    } else {
        // cmp, beq nt, bic, add, tst, tstne, bx lr
        1 + 1 + 1 + 1 + 1 + 1 + 3
    }
}

/// Non-data BIOS cycles of a CpuSet call, excluding the per-unit loop bodies which the copy loop
/// charges through [`cpu_set_unit_cycles`].
pub fn cpu_set_setup_cycles(count: u32, word: bool, fill: bool, rejected: bool) -> u32 {
    // push {r4,r5,lr} 4, lsl, lsr, bl 4, stub (adr, mov ip, bx) 5, range check, beq.
    let mut c = 4 + 1 + 1 + 4 + 5 + range_check(count == 0);
    const POP: u32 = 4 + 3 + 3; // pop {r4,r5}, pop {r3}, bx r3
    if count == 0 || rejected {
        return c + 3 + POP;
    }
    // beq nt, mov r5, lsr r3
    c += 1 + 1 + 1;
    if word {
        // blo nt, add r5, lsr r3, blo
        c += 1 + 1 + 1;
        if fill {
            c += 1 + 2; // blo nt, ldm {r3} fetch + internal
        } else {
            c += 3; // blo taken
        }
    } else {
        // blo taken, lsr r4, lsr r3, blo
        c += 3 + 1 + 1;
        if fill {
            c += 1 + 2; // blo nt, ldrh fetch + internal
        } else {
            c += 3;
        }
    }
    // Loop exit: cmp, bge taken; then the pop sequence.
    c + 1 + 3 + POP
}

/// Non-data BIOS cycles of one CpuSet loop iteration.
#[inline]
pub fn cpu_set_unit_cycles(word: bool, fill: bool) -> u32 {
    match (word, fill) {
        // cmp, bge nt, ldm (fetch+I), stm (fetch), b
        (true, false) => 1 + 1 + 2 + 1 + 3,
        // cmp, bge nt, stm, b
        (true, true) => 1 + 1 + 1 + 3,
        // cmp, bge nt, ldrh (fetch+I), strh, add, b
        (false, false) => 1 + 1 + 2 + 1 + 1 + 3,
        // cmp, bge nt, strh, add, b
        (false, true) => 1 + 1 + 1 + 1 + 3,
    }
}

/// Non-data BIOS cycles of a CpuFastSet call outside the block loop.
pub fn cpu_fast_set_setup_cycles(count: u32, fill: bool, rejected: bool) -> u32 {
    // push of 8 registers 9, lsl, lsrs, bl 3, range check, beq.
    let mut c = 9 + 1 + 1 + 3 + range_check(count == 0);
    const POP: u32 = 10 + 3; // pop of 8 registers, bx lr
    if count == 0 || rejected {
        return c + 3 + POP;
    }
    // beq nt, add sl, lsrs r2, blo
    c += 1 + 1 + 1;
    if fill {
        // blo nt, ldr (fetch + I), 7 movs
        c += 1 + 2 + 7;
    } else {
        c += 3;
    }
    // final cmp / skipped conditional(s) / blt nt; fill ends with `b` to the epilogue.
    c + if fill { 3 + 3 } else { 4 } + POP
}

/// Non-data BIOS cycles of one 8-word CpuFastSet block.
#[inline]
pub fn cpu_fast_set_block_cycles(fill: bool) -> u32 {
    if fill {
        // cmp, stmlt (fetch), blt taken
        1 + 1 + 3
    } else {
        // cmp, ldmlt (fetch + I), stmlt (fetch), blt taken
        1 + 2 + 1 + 3
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixed_overhead_is_45_bios_cycles() {
        assert_eq!(SWI_ENTER, 29);
        assert_eq!(SWI_LEAVE, 16);
    }

    #[test]
    fn mul_early_termination() {
        assert_eq!(mul_cycles(0), 2);
        assert_eq!(mul_cycles(0xFF), 2);
        assert_eq!(mul_cycles(0x100), 3);
        assert_eq!(mul_cycles(0xFFFF_FF00), 2);
        assert_eq!(mul_cycles(0x1_0000), 4);
        assert_eq!(mul_cycles(0x0100_0000), 5);
    }
}
