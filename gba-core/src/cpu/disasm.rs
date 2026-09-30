//! Minimal ARM/THUMB disassembler for `--trace` and test diagnostics. Mnemonics follow GBATEK's
//! classic syntax; operands are complete for every ARMv4T class but no attempt is made at
//! assembler-perfect formatting.

use super::decode::{decode_arm, decode_thumb, ArmOp, ThumbOp};

const CONDS: [&str; 16] = [
    "eq", "ne", "cs", "cc", "mi", "pl", "vs", "vc", "hi", "ls", "ge", "lt", "gt", "le", "", "nv",
];
const ALU: [&str; 16] = [
    "and", "eor", "sub", "rsb", "add", "adc", "sbc", "rsc", "tst", "teq", "cmp", "cmn", "orr",
    "mov", "bic", "mvn",
];
const SHIFTS: [&str; 4] = ["lsl", "lsr", "asr", "ror"];

fn reg(r: u32) -> String {
    match r {
        13 => "sp".into(),
        14 => "lr".into(),
        15 => "pc".into(),
        _ => format!("r{r}"),
    }
}

fn rlist(mut list: u32) -> String {
    let mut parts = Vec::new();
    let mut i = 0;
    while list != 0 {
        if list & 1 != 0 {
            let start = i;
            while list & 2 != 0 {
                list >>= 1;
                i += 1;
            }
            if i > start + 1 {
                parts.push(format!("{}-{}", reg(start), reg(i)));
            } else if i == start + 1 {
                parts.push(reg(start));
                parts.push(reg(i));
            } else {
                parts.push(reg(start));
            }
        }
        list >>= 1;
        i += 1;
    }
    format!("{{{}}}", parts.join(","))
}

fn shifted_reg(op: u32) -> String {
    let rm = reg(op & 0xF);
    let ty = (op >> 5) & 3;
    if op & 0x10 != 0 {
        format!("{rm},{} {}", SHIFTS[ty as usize], reg((op >> 8) & 0xF))
    } else {
        let amount = (op >> 7) & 0x1F;
        match (ty, amount) {
            (0, 0) => rm,
            (3, 0) => format!("{rm},rrx"),
            (_, 0) => format!("{rm},{} #32", SHIFTS[ty as usize]),
            _ => format!("{rm},{} #{amount}", SHIFTS[ty as usize]),
        }
    }
}

/// Disassemble an ARM opcode at `pc`.
pub fn arm(op: u32, pc: u32) -> String {
    let cond = CONDS[(op >> 28) as usize];
    let rn = reg((op >> 16) & 0xF);
    let rd = reg((op >> 12) & 0xF);
    match decode_arm(op) {
        ArmOp::DataProcImm | ArmOp::DataProcRegImm | ArmOp::DataProcRegReg => {
            let opcode = ((op >> 21) & 0xF) as usize;
            let s = if op & (1 << 20) != 0 { "s" } else { "" };
            let op2 = if op & (1 << 25) != 0 {
                format!("#0x{:X}", (op & 0xFF).rotate_right(((op >> 8) & 0xF) * 2))
            } else {
                shifted_reg(op)
            };
            match opcode {
                8..=11 => format!("{}{cond} {rn},{op2}", ALU[opcode]),
                13 | 15 => format!("{}{cond}{s} {rd},{op2}", ALU[opcode]),
                _ => format!("{}{cond}{s} {rd},{rn},{op2}", ALU[opcode]),
            }
        }
        ArmOp::Mrs => format!(
            "mrs{cond} {rd},{}",
            if op & (1 << 22) != 0 { "spsr" } else { "cpsr" }
        ),
        ArmOp::MsrReg | ArmOp::MsrImm => {
            let mut fields = String::new();
            for (bit, ch) in [(19, 'f'), (18, 's'), (17, 'x'), (16, 'c')] {
                if op & (1 << bit) != 0 {
                    fields.push(ch);
                }
            }
            let src = if op & (1 << 25) != 0 {
                format!("#0x{:X}", (op & 0xFF).rotate_right(((op >> 8) & 0xF) * 2))
            } else {
                reg(op & 0xF)
            };
            format!(
                "msr{cond} {}_{fields},{src}",
                if op & (1 << 22) != 0 { "spsr" } else { "cpsr" }
            )
        }
        ArmOp::Bx => format!("bx{cond} {}", reg(op & 0xF)),
        ArmOp::Multiply => {
            let s = if op & (1 << 20) != 0 { "s" } else { "" };
            let rm = reg(op & 0xF);
            let rs = reg((op >> 8) & 0xF);
            if op & (1 << 21) != 0 {
                format!("mla{cond}{s} {rn},{rm},{rs},{rd}")
            } else {
                format!("mul{cond}{s} {rn},{rm},{rs}")
            }
        }
        ArmOp::MultiplyLong => {
            let s = if op & (1 << 20) != 0 { "s" } else { "" };
            let name = match (op >> 21) & 3 {
                0 => "umull",
                1 => "umlal",
                2 => "smull",
                _ => "smlal",
            };
            format!(
                "{name}{cond}{s} {rd},{rn},{},{}",
                reg(op & 0xF),
                reg((op >> 8) & 0xF)
            )
        }
        ArmOp::Swp => {
            let b = if op & (1 << 22) != 0 { "b" } else { "" };
            format!("swp{cond}{b} {rd},{},[{rn}]", reg(op & 0xF))
        }
        ArmOp::HalfwordTransfer => {
            let load = op & (1 << 20) != 0;
            let name = match ((op >> 5) & 3, load) {
                (1, false) => "strh",
                (1, true) => "ldrh",
                (2, true) => "ldrsb",
                (3, true) => "ldrsh",
                _ => "str?",
            };
            let offset = if op & (1 << 22) != 0 {
                format!(
                    "#{}0x{:X}",
                    if op & (1 << 23) != 0 { "" } else { "-" },
                    ((op >> 4) & 0xF0) | (op & 0xF)
                )
            } else {
                format!(
                    "{}{}",
                    if op & (1 << 23) != 0 { "" } else { "-" },
                    reg(op & 0xF)
                )
            };
            format!("{name}{cond} {rd},{}", address(op, &rn, &offset))
        }
        ArmOp::SingleTransfer => {
            let name = if op & (1 << 20) != 0 { "ldr" } else { "str" };
            let b = if op & (1 << 22) != 0 { "b" } else { "" };
            let t = if op & (1 << 24) == 0 && op & (1 << 21) != 0 {
                "t"
            } else {
                ""
            };
            let sign = if op & (1 << 23) != 0 { "" } else { "-" };
            let offset = if op & (1 << 25) != 0 {
                format!("{sign}{}", shifted_reg(op & !0x10))
            } else {
                format!("#{sign}0x{:X}", op & 0xFFF)
            };
            format!("{name}{cond}{b}{t} {rd},{}", address(op, &rn, &offset))
        }
        ArmOp::BlockTransfer => {
            let name = if op & (1 << 20) != 0 { "ldm" } else { "stm" };
            let amod = match (op >> 23) & 3 {
                0 => "da",
                1 => "ia",
                2 => "db",
                _ => "ib",
            };
            let wb = if op & (1 << 21) != 0 { "!" } else { "" };
            let s = if op & (1 << 22) != 0 { "^" } else { "" };
            format!("{name}{cond}{amod} {rn}{wb},{}{s}", rlist(op & 0xFFFF))
        }
        ArmOp::Branch => {
            let offset = ((op & 0x00FF_FFFF) << 8) as i32 >> 6;
            let target = pc.wrapping_add(8).wrapping_add(offset as u32);
            let l = if op & (1 << 24) != 0 { "l" } else { "" };
            format!("b{l}{cond} 0x{target:08X}")
        }
        ArmOp::Swi => format!("swi{cond} 0x{:X}", op & 0x00FF_FFFF),
        ArmOp::Undefined => format!("undefined 0x{op:08X}"),
    }
}

fn address(op: u32, rn: &str, offset: &str) -> String {
    let pre = op & (1 << 24) != 0;
    let wb = if op & (1 << 21) != 0 { "!" } else { "" };
    if pre {
        format!("[{rn},{offset}]{wb}")
    } else {
        format!("[{rn}],{offset}")
    }
}

/// Disassemble a THUMB opcode at `pc`.
pub fn thumb(op: u16, pc: u32) -> String {
    let op = op as u32;
    let rd = reg(op & 7);
    let rs = reg((op >> 3) & 7);
    match decode_thumb(op as u16) {
        ThumbOp::ShiftImm => {
            format!(
                "{} {rd},{rs},#{}",
                SHIFTS[((op >> 11) & 3) as usize],
                (op >> 6) & 0x1F
            )
        }
        ThumbOp::AddSub => {
            let name = if op & (1 << 9) != 0 { "sub" } else { "add" };
            if op & (1 << 10) != 0 {
                format!("{name} {rd},{rs},#{}", (op >> 6) & 7)
            } else {
                format!("{name} {rd},{rs},{}", reg((op >> 6) & 7))
            }
        }
        ThumbOp::Imm8 => {
            let name = ["mov", "cmp", "add", "sub"][((op >> 11) & 3) as usize];
            format!("{name} {},#0x{:X}", reg((op >> 8) & 7), op & 0xFF)
        }
        ThumbOp::Alu => {
            let name = [
                "and", "eor", "lsl", "lsr", "asr", "adc", "sbc", "ror", "tst", "neg", "cmp", "cmn",
                "orr", "mul", "bic", "mvn",
            ][((op >> 6) & 0xF) as usize];
            format!("{name} {rd},{rs}")
        }
        ThumbOp::HiReg => {
            let rd = reg((op & 7) | ((op >> 4) & 8));
            let rs = reg(((op >> 3) & 7) | ((op >> 3) & 8));
            match (op >> 8) & 3 {
                0 => format!("add {rd},{rs}"),
                1 => format!("cmp {rd},{rs}"),
                2 => format!("mov {rd},{rs}"),
                _ => format!("bx {rs}"),
            }
        }
        ThumbOp::LdrPc => {
            let addr = (pc.wrapping_add(4) & !2).wrapping_add((op & 0xFF) << 2);
            format!(
                "ldr {},[pc,#0x{:X}] ; 0x{addr:08X}",
                reg((op >> 8) & 7),
                (op & 0xFF) << 2
            )
        }
        ThumbOp::LdStReg => {
            let name = ["str", "strb", "ldr", "ldrb"][((op >> 10) & 3) as usize];
            format!("{name} {rd},[{rs},{}]", reg((op >> 6) & 7))
        }
        ThumbOp::LdStSigned => {
            let name = ["strh", "ldsb", "ldrh", "ldsh"][((op >> 10) & 3) as usize];
            format!("{name} {rd},[{rs},{}]", reg((op >> 6) & 7))
        }
        ThumbOp::LdStImm => {
            let name = ["str", "ldr", "strb", "ldrb"][((op >> 11) & 3) as usize];
            let imm = (op >> 6) & 0x1F;
            let imm = if op & (1 << 12) != 0 { imm } else { imm << 2 };
            format!("{name} {rd},[{rs},#0x{imm:X}]")
        }
        ThumbOp::LdStHalf => {
            let name = if op & (1 << 11) != 0 { "ldrh" } else { "strh" };
            format!("{name} {rd},[{rs},#0x{:X}]", ((op >> 6) & 0x1F) << 1)
        }
        ThumbOp::LdStSp => {
            let name = if op & (1 << 11) != 0 { "ldr" } else { "str" };
            format!(
                "{name} {},[sp,#0x{:X}]",
                reg((op >> 8) & 7),
                (op & 0xFF) << 2
            )
        }
        ThumbOp::AddrPcSp => {
            let base = if op & (1 << 11) != 0 { "sp" } else { "pc" };
            format!(
                "add {},{base},#0x{:X}",
                reg((op >> 8) & 7),
                (op & 0xFF) << 2
            )
        }
        ThumbOp::AddSp => {
            let sign = if op & (1 << 7) != 0 { "-" } else { "" };
            format!("add sp,#{sign}0x{:X}", (op & 0x7F) << 2)
        }
        ThumbOp::PushPop => {
            let mut list = op & 0xFF;
            if op & (1 << 11) != 0 {
                if op & (1 << 8) != 0 {
                    list |= 1 << 15;
                }
                format!("pop {}", rlist(list))
            } else {
                if op & (1 << 8) != 0 {
                    list |= 1 << 14;
                }
                format!("push {}", rlist(list))
            }
        }
        ThumbOp::LdmStm => {
            let name = if op & (1 << 11) != 0 {
                "ldmia"
            } else {
                "stmia"
            };
            format!("{name} {}!,{}", reg((op >> 8) & 7), rlist(op & 0xFF))
        }
        ThumbOp::BCond => {
            let offset = ((op & 0xFF) as i8 as i32) << 1;
            format!(
                "b{} 0x{:08X}",
                CONDS[((op >> 8) & 0xF) as usize],
                pc.wrapping_add(4).wrapping_add(offset as u32)
            )
        }
        ThumbOp::Swi => format!("swi 0x{:X}", op & 0xFF),
        ThumbOp::B => {
            let offset = (((op & 0x7FF) << 21) as i32) >> 20;
            format!("b 0x{:08X}", pc.wrapping_add(4).wrapping_add(offset as u32))
        }
        ThumbOp::BlHi => format!("bl.hi #0x{:X}", (op & 0x7FF) << 12),
        ThumbOp::BlLo => format!("bl.lo #0x{:X}", (op & 0x7FF) << 1),
        ThumbOp::Undefined => format!("undefined 0x{op:04X}"),
    }
}
