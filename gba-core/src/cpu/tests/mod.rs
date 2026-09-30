//! CPU unit tests: a flat test bus plus table-driven decoder, flag, memory and exception tests.
#![allow(
    clippy::too_many_arguments,
    clippy::type_complexity,
    clippy::identity_op,
    clippy::erasing_op
)]

mod arm_tests;
mod exceptions;
mod flags;
mod thumb_tests;

use super::regs::*;
use super::Cpu;
use crate::bus::{Access, MemoryBus};

/// Flat RAM covering the whole address space modulo its size, with a cycle counter and a
/// programmable IRQ line. Every access costs one cycle; `idle` adds its argument.
pub struct TestBus {
    pub mem: Vec<u8>,
    pub cycles: u64,
    pub irq: bool,
    /// Every (addr, Access) opcode fetch, in order.
    pub fetches: Vec<(u32, Access)>,
    /// Every data access as (addr, width, is_write, Access).
    pub accesses: Vec<(u32, u8, bool, Access)>,
    pub idles: u32,
    pub swi_calls: Vec<u8>,
    pub hle: bool,
}

impl TestBus {
    pub fn new() -> Self {
        TestBus {
            mem: vec![0; 0x10000],
            cycles: 0,
            irq: false,
            fetches: Vec::new(),
            accesses: Vec::new(),
            idles: 0,
            swi_calls: Vec::new(),
            hle: false,
        }
    }
    fn idx(&self, addr: u32) -> usize {
        (addr as usize) & (self.mem.len() - 1)
    }
    pub fn peek8(&self, addr: u32) -> u8 {
        self.mem[self.idx(addr)]
    }
    pub fn peek16(&self, addr: u32) -> u16 {
        let a = addr & !1;
        self.peek8(a) as u16 | (self.peek8(a + 1) as u16) << 8
    }
    pub fn peek32(&self, addr: u32) -> u32 {
        let a = addr & !3;
        self.peek16(a) as u32 | (self.peek16(a + 2) as u32) << 16
    }
    pub fn poke8(&mut self, addr: u32, v: u8) {
        let i = self.idx(addr);
        self.mem[i] = v;
    }
    pub fn poke16(&mut self, addr: u32, v: u16) {
        let a = addr & !1;
        self.poke8(a, v as u8);
        self.poke8(a + 1, (v >> 8) as u8);
    }
    pub fn poke32(&mut self, addr: u32, v: u32) {
        let a = addr & !3;
        self.poke16(a, v as u16);
        self.poke16(a + 2, (v >> 16) as u16);
    }
    /// Write a sequence of ARM words starting at `addr`.
    pub fn load_arm(&mut self, addr: u32, words: &[u32]) {
        for (i, w) in words.iter().enumerate() {
            self.poke32(addr + 4 * i as u32, *w);
        }
    }
    /// Write a sequence of THUMB halfwords starting at `addr`.
    pub fn load_thumb(&mut self, addr: u32, halves: &[u16]) {
        for (i, h) in halves.iter().enumerate() {
            self.poke16(addr + 2 * i as u32, *h);
        }
    }
}

impl MemoryBus for TestBus {
    fn read8(&mut self, addr: u32, a: Access) -> u8 {
        self.cycles += 1;
        self.accesses.push((addr, 1, false, a));
        self.peek8(addr)
    }
    fn read16(&mut self, addr: u32, a: Access) -> u16 {
        self.cycles += 1;
        self.accesses.push((addr, 2, false, a));
        self.peek16(addr)
    }
    fn read32(&mut self, addr: u32, a: Access) -> u32 {
        self.cycles += 1;
        self.accesses.push((addr, 4, false, a));
        self.peek32(addr)
    }
    fn write8(&mut self, addr: u32, v: u8, a: Access) {
        self.cycles += 1;
        self.accesses.push((addr, 1, true, a));
        self.poke8(addr, v);
    }
    fn write16(&mut self, addr: u32, v: u16, a: Access) {
        self.cycles += 1;
        self.accesses.push((addr, 2, true, a));
        self.poke16(addr, v);
    }
    fn write32(&mut self, addr: u32, v: u32, a: Access) {
        self.cycles += 1;
        self.accesses.push((addr, 4, true, a));
        self.poke32(addr, v);
    }
    fn fetch16(&mut self, addr: u32, a: Access) -> u16 {
        self.cycles += 1;
        self.fetches.push((addr, a));
        self.peek16(addr)
    }
    fn fetch32(&mut self, addr: u32, a: Access) -> u32 {
        self.cycles += 1;
        self.fetches.push((addr, a));
        self.peek32(addr)
    }
    fn idle(&mut self, cycles: u32) {
        self.cycles += cycles as u64;
        self.idles += cycles;
    }
    fn irq_pending(&self) -> bool {
        self.irq
    }
    fn now(&self) -> u64 {
        self.cycles
    }
    fn is_hle_bios(&self) -> bool {
        self.hle
    }
    fn swi_hle(&mut self, num: u8, _regs: &mut Regs) -> bool {
        self.swi_calls.push(num);
        self.hle
    }
}

/// Code is placed here in every test; leaves room below for vectors.
pub const CODE: u32 = 0x1000;
/// Scratch data area.
pub const DATA: u32 = 0x8000;

/// A CPU in System mode, ARM state, flags clear, PC at `CODE`.
pub fn arm_cpu() -> Cpu {
    let mut cpu = Cpu::new();
    cpu.regs.set_cpsr(MODE_SYS);
    cpu.regs.r[13] = 0xF000;
    cpu.regs.r[15] = CODE;
    cpu
}

/// A CPU in System mode, THUMB state, flags clear, PC at `CODE`.
pub fn thumb_cpu() -> Cpu {
    let mut cpu = arm_cpu();
    cpu.regs.cpsr |= FLAG_T;
    cpu
}

/// Load `words` at CODE and execute `n` instructions.
pub fn run_arm(cpu: &mut Cpu, bus: &mut TestBus, words: &[u32], n: usize) {
    bus.load_arm(CODE, words);
    for _ in 0..n {
        cpu.step(bus);
    }
}

pub fn run_thumb(cpu: &mut Cpu, bus: &mut TestBus, halves: &[u16], n: usize) {
    bus.load_thumb(CODE, halves);
    for _ in 0..n {
        cpu.step(bus);
    }
}

/// Execute a single ARM instruction on a fresh CPU/bus after `setup` prepares them.
pub fn arm1(op: u32, setup: impl FnOnce(&mut Cpu, &mut TestBus)) -> (Cpu, TestBus) {
    let mut cpu = arm_cpu();
    let mut bus = TestBus::new();
    setup(&mut cpu, &mut bus);
    run_arm(&mut cpu, &mut bus, &[op], 1);
    (cpu, bus)
}

/// Execute a single THUMB instruction on a fresh CPU/bus after `setup` prepares them.
pub fn thumb1(op: u16, setup: impl FnOnce(&mut Cpu, &mut TestBus)) -> (Cpu, TestBus) {
    let mut cpu = thumb_cpu();
    let mut bus = TestBus::new();
    setup(&mut cpu, &mut bus);
    run_thumb(&mut cpu, &mut bus, &[op], 1);
    (cpu, bus)
}

pub fn flags(cpu: &Cpu) -> (bool, bool, bool, bool) {
    (
        cpu.regs.flag_n(),
        cpu.regs.flag_z(),
        cpu.regs.flag_c(),
        cpu.regs.flag_v(),
    )
}

// ------------------------------------------------------------------ ARM opcode builders

pub const AL: u32 = 0xE << 28;

/// Data processing, immediate operand: `op{s} rd, rn, #imm8 ror rot*2`.
pub fn dp_imm(opcode: u32, s: bool, rd: u32, rn: u32, imm8: u32, rot: u32) -> u32 {
    AL | 1 << 25 | opcode << 21 | (s as u32) << 20 | rn << 16 | rd << 12 | rot << 8 | imm8
}

/// Data processing, register operand shifted by immediate.
pub fn dp_reg(opcode: u32, s: bool, rd: u32, rn: u32, rm: u32, shift: u32, amount: u32) -> u32 {
    AL | opcode << 21 | (s as u32) << 20 | rn << 16 | rd << 12 | amount << 7 | shift << 5 | rm
}

/// Data processing, register operand shifted by register.
pub fn dp_regreg(opcode: u32, s: bool, rd: u32, rn: u32, rm: u32, shift: u32, rs: u32) -> u32 {
    AL | opcode << 21 | (s as u32) << 20 | rn << 16 | rd << 12 | rs << 8 | shift << 5 | 1 << 4 | rm
}

pub const AND: u32 = 0;
pub const EOR: u32 = 1;
pub const SUB: u32 = 2;
pub const RSB: u32 = 3;
pub const ADD: u32 = 4;
pub const ADC: u32 = 5;
pub const SBC: u32 = 6;
pub const RSC: u32 = 7;
pub const TST: u32 = 8;
pub const TEQ: u32 = 9;
pub const CMP: u32 = 10;
pub const CMN: u32 = 11;
pub const ORR: u32 = 12;
pub const MOV: u32 = 13;
pub const BIC: u32 = 14;
pub const MVN: u32 = 15;

/// `mov rd, #imm` (imm must be encodable as imm8 ror even).
pub fn mov_imm(rd: u32, imm8: u32) -> u32 {
    dp_imm(MOV, false, rd, 0, imm8, 0)
}

/// LDR/STR: `ldr{b} rd, [rn, #imm]` with P/U/W/L flags.
pub fn ldst_imm(
    load: bool,
    byte: bool,
    pre: bool,
    up: bool,
    wb: bool,
    rd: u32,
    rn: u32,
    imm: u32,
) -> u32 {
    AL | 1 << 26
        | (pre as u32) << 24
        | (up as u32) << 23
        | (byte as u32) << 22
        | (wb as u32) << 21
        | (load as u32) << 20
        | rn << 16
        | rd << 12
        | imm
}

/// Halfword/signed transfer with immediate offset. `sh`: 1=H, 2=SB, 3=SH.
pub fn ldsth_imm(
    load: bool,
    sh: u32,
    pre: bool,
    up: bool,
    wb: bool,
    rd: u32,
    rn: u32,
    imm: u32,
) -> u32 {
    AL | (pre as u32) << 24
        | (up as u32) << 23
        | 1 << 22
        | (wb as u32) << 21
        | (load as u32) << 20
        | rn << 16
        | rd << 12
        | (imm & 0xF0) << 4
        | 1 << 7
        | sh << 5
        | 1 << 4
        | (imm & 0xF)
}

/// Block transfer.
pub fn ldstm(load: bool, pre: bool, up: bool, s: bool, wb: bool, rn: u32, rlist: u32) -> u32 {
    AL | 1 << 27
        | (pre as u32) << 24
        | (up as u32) << 23
        | (s as u32) << 22
        | (wb as u32) << 21
        | (load as u32) << 20
        | rn << 16
        | rlist
}

/// B / BL with a word offset (relative to PC+8).
pub fn branch(link: bool, offset_words: i32) -> u32 {
    AL | 0b101 << 25 | (link as u32) << 24 | (offset_words as u32 & 0x00FF_FFFF)
}

pub fn bx(rn: u32) -> u32 {
    AL | 0x012F_FF10 | rn
}

pub fn mul(s: bool, rd: u32, rm: u32, rs: u32) -> u32 {
    AL | (s as u32) << 20 | rd << 16 | rs << 8 | 0x90 | rm
}

pub fn mla(s: bool, rd: u32, rm: u32, rs: u32, rn: u32) -> u32 {
    AL | 1 << 21 | (s as u32) << 20 | rd << 16 | rn << 12 | rs << 8 | 0x90 | rm
}

/// UMULL(0)/UMLAL(1)/SMULL(2)/SMLAL(3).
pub fn mull(kind: u32, s: bool, rd_lo: u32, rd_hi: u32, rm: u32, rs: u32) -> u32 {
    AL | 1 << 23 | kind << 21 | (s as u32) << 20 | rd_hi << 16 | rd_lo << 12 | rs << 8 | 0x90 | rm
}

pub fn swp(byte: bool, rd: u32, rm: u32, rn: u32) -> u32 {
    AL | 1 << 24 | (byte as u32) << 22 | rn << 16 | rd << 12 | 0x90 | rm
}

pub fn mrs(rd: u32, spsr: bool) -> u32 {
    AL | 0x010F_0000 | (spsr as u32) << 22 | rd << 12
}

/// MSR register form with field mask bits (f=8, s=4, x=2, c=1).
pub fn msr_reg(spsr: bool, fields: u32, rm: u32) -> u32 {
    AL | 0x0120_F000 | (spsr as u32) << 22 | fields << 16 | rm
}

pub fn msr_imm(spsr: bool, fields: u32, imm8: u32, rot: u32) -> u32 {
    AL | 0x0320_F000 | (spsr as u32) << 22 | fields << 16 | rot << 8 | imm8
}

pub fn swi(n: u32) -> u32 {
    AL | 0x0F00_0000 | n << 16
}

/// `b .` (branch to self).
pub const B_SELF: u32 = 0xEAFF_FFFE;
