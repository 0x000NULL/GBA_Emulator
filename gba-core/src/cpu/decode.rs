//! Instruction-class decoding. The ARM table is indexed by the 12 bits that disambiguate every
//! ARMv4 encoding (bits 27-20 and 7-4) and is built at compile time; THUMB classification keys
//! on the top byte. Source: GBATEK "ARM Binary Opcode Format", "THUMB Binary Opcode Format".

/// ARM instruction classes. Operand fields are extracted by the executor.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum ArmOp {
    /// Data processing with an immediate operand (bit 25 set).
    DataProcImm,
    /// Data processing with a register operand shifted by an immediate.
    DataProcRegImm,
    /// Data processing with a register operand shifted by a register (+1 I cycle).
    DataProcRegReg,
    Mrs,
    MsrReg,
    MsrImm,
    Bx,
    /// MUL / MLA.
    Multiply,
    /// UMULL / UMLAL / SMULL / SMLAL.
    MultiplyLong,
    Swp,
    /// LDRH / STRH / LDRSB / LDRSH (register or immediate offset, bit 22).
    HalfwordTransfer,
    /// LDR / STR / LDRB / STRB (immediate or shifted-register offset, bit 25).
    SingleTransfer,
    BlockTransfer,
    Branch,
    Swi,
    Undefined,
}

/// Index of `op` in [`ARM_TABLE`].
#[inline]
pub fn arm_index(op: u32) -> usize {
    (((op >> 16) & 0xFF0) | ((op >> 4) & 0xF)) as usize
}

const fn classify_arm(index: usize) -> ArmOp {
    let hi = (index >> 4) as u32; // bits 27-20
    let lo = (index & 0xF) as u32; // bits 7-4
    let bit24 = hi & 0x10 != 0;
    let bit23 = hi & 0x08 != 0;
    let bit22 = hi & 0x04 != 0;
    let bit21 = hi & 0x02 != 0;
    let bit20 = hi & 0x01 != 0;
    let bit7 = lo & 0x8 != 0;
    let bit4 = lo & 0x1 != 0;
    match hi >> 5 {
        0b000 => {
            if bit7 && bit4 {
                // Multiply / swap / halfword transfer space.
                if lo == 0b1001 {
                    if !bit24 {
                        if bit23 {
                            ArmOp::MultiplyLong
                        } else if !bit22 {
                            ArmOp::Multiply
                        } else {
                            ArmOp::Undefined
                        }
                    } else if !bit21 && !bit20 {
                        ArmOp::Swp
                    } else {
                        ArmOp::Undefined
                    }
                } else {
                    ArmOp::HalfwordTransfer
                }
            } else if bit24 && !bit23 && !bit20 {
                // TST/TEQ/CMP/CMN without S: PSR transfer and BX.
                if lo == 0 {
                    if bit21 {
                        ArmOp::MsrReg
                    } else {
                        ArmOp::Mrs
                    }
                } else if lo == 1 && bit21 && !bit22 {
                    ArmOp::Bx
                } else if bit4 {
                    ArmOp::DataProcRegReg
                } else {
                    ArmOp::DataProcRegImm
                }
            } else if bit4 {
                ArmOp::DataProcRegReg
            } else {
                ArmOp::DataProcRegImm
            }
        }
        0b001 => {
            if bit24 && !bit23 && !bit20 {
                if bit21 {
                    ArmOp::MsrImm
                } else {
                    ArmOp::Undefined
                }
            } else {
                ArmOp::DataProcImm
            }
        }
        0b010 => ArmOp::SingleTransfer,
        0b011 => {
            if bit4 {
                ArmOp::Undefined
            } else {
                ArmOp::SingleTransfer
            }
        }
        0b100 => ArmOp::BlockTransfer,
        0b101 => ArmOp::Branch,
        0b110 => ArmOp::Undefined,
        _ => {
            if bit24 {
                ArmOp::Swi
            } else {
                ArmOp::Undefined
            }
        }
    }
}

const fn build_arm_table() -> [ArmOp; 4096] {
    let mut t = [ArmOp::Undefined; 4096];
    let mut i = 0;
    while i < 4096 {
        t[i] = classify_arm(i);
        i += 1;
    }
    t
}

/// 4096-entry class table for ARM opcodes.
pub static ARM_TABLE: [ArmOp; 4096] = build_arm_table();

/// Decode the class of an ARM opcode.
#[inline]
pub fn decode_arm(op: u32) -> ArmOp {
    ARM_TABLE[arm_index(op)]
}

/// THUMB instruction formats (GBATEK numbering).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum ThumbOp {
    /// 1: move shifted register.
    ShiftImm,
    /// 2: add/subtract.
    AddSub,
    /// 3: move/compare/add/subtract immediate.
    Imm8,
    /// 4: ALU operations.
    Alu,
    /// 5: hi register operations / BX.
    HiReg,
    /// 6: load PC-relative.
    LdrPc,
    /// 7: load/store with register offset.
    LdStReg,
    /// 8: load/store sign-extended byte/halfword.
    LdStSigned,
    /// 9: load/store with immediate offset.
    LdStImm,
    /// 10: load/store halfword.
    LdStHalf,
    /// 11: load/store SP-relative.
    LdStSp,
    /// 12: get relative address.
    AddrPcSp,
    /// 13: add offset to stack pointer.
    AddSp,
    /// 14: push/pop.
    PushPop,
    /// 15: multiple load/store.
    LdmStm,
    /// 16: conditional branch.
    BCond,
    /// 17: software interrupt.
    Swi,
    /// 18: unconditional branch.
    B,
    /// 19: long branch with link, first half (sets LR).
    BlHi,
    /// 19: long branch with link, second half (jumps).
    BlLo,
    Undefined,
}

const fn classify_thumb(top: usize) -> ThumbOp {
    let t = top as u32; // bits 15-8
    match t >> 5 {
        0b000 => {
            if t >> 3 == 0b00011 {
                ThumbOp::AddSub
            } else {
                ThumbOp::ShiftImm
            }
        }
        0b001 => ThumbOp::Imm8,
        0b010 => match t >> 2 {
            0b010000 => ThumbOp::Alu,
            0b010001 => ThumbOp::HiReg,
            0b010010 | 0b010011 => ThumbOp::LdrPc,
            _ => {
                if t & 0x2 != 0 {
                    ThumbOp::LdStSigned
                } else {
                    ThumbOp::LdStReg
                }
            }
        },
        0b011 => ThumbOp::LdStImm,
        0b100 => {
            if t & 0x10 == 0 {
                ThumbOp::LdStHalf
            } else {
                ThumbOp::LdStSp
            }
        }
        0b101 => {
            if t & 0x10 == 0 {
                ThumbOp::AddrPcSp
            } else if t == 0xB0 {
                ThumbOp::AddSp
            } else if t & 0x06 == 0x04 {
                ThumbOp::PushPop
            } else {
                ThumbOp::Undefined
            }
        }
        0b110 => {
            if t & 0x10 == 0 {
                ThumbOp::LdmStm
            } else if t == 0xDF {
                ThumbOp::Swi
            } else if t == 0xDE {
                ThumbOp::Undefined
            } else {
                ThumbOp::BCond
            }
        }
        _ => match t >> 3 {
            0b11100 => ThumbOp::B,
            0b11101 => ThumbOp::Undefined,
            0b11110 => ThumbOp::BlHi,
            _ => ThumbOp::BlLo,
        },
    }
}

const fn build_thumb_table() -> [ThumbOp; 256] {
    let mut t = [ThumbOp::Undefined; 256];
    let mut i = 0;
    while i < 256 {
        t[i] = classify_thumb(i);
        i += 1;
    }
    t
}

/// 256-entry format table for THUMB opcodes, indexed by bits 15-8.
pub static THUMB_TABLE: [ThumbOp; 256] = build_thumb_table();

/// Decode the format of a THUMB opcode.
#[inline]
pub fn decode_thumb(op: u16) -> ThumbOp {
    THUMB_TABLE[(op >> 8) as usize]
}
