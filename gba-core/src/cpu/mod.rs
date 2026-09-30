//! ARM7TDMI interpreter. Owned by the CPU agent (M1) except `regs.rs`, whose API is frozen.
//!
//! Contract with the bus (see `bus::MemoryBus`):
//! - `step` executes exactly one instruction (or takes one exception) and returns the cycles it
//!   consumed. All memory accesses go through the bus, which advances the scheduler clock itself.
//! - IRQs are sampled at the top of `step`: if `bus.irq_pending()` and CPSR.I is clear, the CPU
//!   takes the IRQ exception instead of executing an instruction.
//! - SWI: if `bus.swi_hle(num, &mut regs)` returns true the BIOS call was serviced in HLE and no
//!   exception is taken; otherwise enter SVC mode at vector 0x08.
//! - When about to execute from the BIOS region (PC < 0x4000) and `bus.is_hle_bios()`, call
//!   `bus.bios_hle_vector(&mut regs)` first; if it returns true the "instruction" was emulated
//!   (e.g. the IRQ dispatcher at 0x18 or its return stub at 0x138) and `step` is done.
//!
//! M0 status: STUB. `step` fetches and skips one word so time advances. Replaced in M1.

pub mod regs;

pub use regs::Regs;

use crate::bus::{Access, MemoryBus};

/// Exception vectors.
pub const VEC_RESET: u32 = 0x00;
pub const VEC_UNDEFINED: u32 = 0x04;
pub const VEC_SWI: u32 = 0x08;
pub const VEC_PREFETCH_ABORT: u32 = 0x0C;
pub const VEC_DATA_ABORT: u32 = 0x10;
pub const VEC_IRQ: u32 = 0x18;
pub const VEC_FIQ: u32 = 0x1C;

#[derive(Clone, Debug)]
pub struct Cpu {
    pub regs: Regs,
    /// Two-slot fetched pipeline: [decode, fetch]. The instruction at `regs.r[15]` is being
    /// executed; the pipeline holds the words at PC+4/PC+8 (ARM) or PC+2/PC+4 (Thumb).
    pub pipeline: [u32; 2],
    /// Next fetch should be sequential.
    pub next_fetch_seq: bool,
    /// Instructions executed since reset.
    pub instr_count: u64,
    /// Opcode of the instruction most recently executed (for tracing).
    pub last_opcode: u32,
}

impl Default for Cpu {
    fn default() -> Self {
        Self::new()
    }
}

impl Cpu {
    pub fn new() -> Self {
        Cpu { regs: Regs::new(), pipeline: [0; 2], next_fetch_seq: false, instr_count: 0, last_opcode: 0 }
    }

    /// Post-BIOS register state (GBATEK "BIOS RAM Usage" / mGBA GBASkipBIOS):
    /// System mode, IRQs enabled, SP_svc=03007FE0, SP_irq=03007FA0, SP_sys=03007F00, PC=08000000.
    pub fn reset_hle(&mut self) {
        self.regs = Regs::new();
        self.regs.set_cpsr(regs::MODE_SVC);
        self.regs.r[13] = 0x0300_7FE0;
        self.regs.switch_mode(regs::MODE_IRQ);
        self.regs.r[13] = 0x0300_7FA0;
        self.regs.switch_mode(regs::MODE_SYS);
        self.regs.r[13] = 0x0300_7F00;
        self.regs.r[14] = 0x0800_0000;
        self.regs.r[15] = 0x0800_0000;
        self.pipeline = [0; 2];
        self.next_fetch_seq = false;
    }

    /// Real reset: SVC mode, IRQ/FIQ disabled, PC=0 (LLE BIOS).
    pub fn reset_lle(&mut self) {
        self.regs = Regs::new();
        self.regs.r[15] = 0;
        self.pipeline = [0; 2];
        self.next_fetch_seq = false;
    }

    /// Address of the instruction about to execute.
    #[inline]
    pub fn pc(&self) -> u32 {
        self.regs.r[15]
    }

    /// Flush the pipeline after a PC write (branch). The CPU agent refills on the next `step`.
    pub fn flush_pipeline(&mut self) {
        self.next_fetch_seq = false;
    }

    /// Execute one instruction. Returns cycles consumed (informational; the bus keeps the clock).
    pub fn step<B: MemoryBus>(&mut self, bus: &mut B) -> u32 {
        let start = bus.now();
        if bus.irq_pending() && !self.regs.irq_disabled() {
            self.enter_exception(bus, VEC_IRQ, regs::MODE_IRQ);
            return (bus.now() - start) as u32;
        }
        if self.regs.r[15] < 0x4000 && bus.is_hle_bios() && bus.bios_hle_vector(&mut self.regs) {
            self.flush_pipeline();
            return (bus.now() - start) as u32;
        }
        // STUB: fetch and skip. Replaced by the ARM/Thumb interpreter in M1.
        let pc = self.regs.r[15];
        let access = if self.next_fetch_seq { Access::S } else { Access::N };
        if self.regs.thumb() {
            self.last_opcode = bus.fetch16(pc, access) as u32;
            self.regs.r[15] = pc.wrapping_add(2);
        } else {
            self.last_opcode = bus.fetch32(pc, access);
            self.regs.r[15] = pc.wrapping_add(4);
        }
        self.next_fetch_seq = true;
        self.instr_count += 1;
        (bus.now() - start) as u32
    }

    /// Exception entry: SPSR_mode = CPSR, LR_mode = return address, switch mode, ARM state,
    /// I set (and F set for FIQ/reset), PC = vector. The return address stored in LR follows
    /// the ARM7TDMI convention for each exception (IRQ: address of next instr + 4; SWI/UND: next instr).
    pub fn enter_exception<B: MemoryBus>(&mut self, bus: &mut B, vector: u32, mode: u32) {
        let old_cpsr = self.regs.cpsr;
        let pc = self.regs.r[15];
        let thumb = self.regs.thumb();
        let lr = match vector {
            // IRQ: the interrupted instruction is at PC; hardware stores PC+4 (ARM) / PC+4 (Thumb) so
            // that `subs pc, lr, #4` returns to it.
            VEC_IRQ | VEC_FIQ => pc.wrapping_add(4),
            // SWI/UND: return to the following instruction (`movs pc, lr`).
            VEC_SWI | VEC_UNDEFINED => pc.wrapping_add(if thumb { 2 } else { 4 }),
            _ => pc.wrapping_add(4),
        };
        self.regs.switch_mode(mode);
        self.regs.set_spsr(old_cpsr);
        self.regs.r[14] = lr;
        self.regs.cpsr &= !regs::FLAG_T;
        self.regs.cpsr |= regs::FLAG_I;
        if vector == VEC_FIQ || vector == VEC_RESET {
            self.regs.cpsr |= regs::FLAG_F;
        }
        self.regs.r[15] = vector;
        self.flush_pipeline();
        bus.idle(2);
    }

    /// Hash of the architectural state (r0-r15, CPSR, SPSR) for trace diffing.
    pub fn state_hash(&self) -> u64 {
        // FNV-1a 64 over the register words.
        let mut h: u64 = 0xcbf29ce484222325;
        let mut mix = |v: u32| {
            for b in v.to_le_bytes() {
                h ^= b as u64;
                h = h.wrapping_mul(0x100000001b3);
            }
        };
        for &v in &self.regs.r {
            mix(v);
        }
        mix(self.regs.cpsr);
        mix(self.regs.spsr());
        h
    }

    /// One trace line in mGBA's `ARMDebuggerFormatRegisters` layout: sixteen `%08X` words then
    /// `cpsr: %08X`. r15 is reported as PC+8 (ARM) / PC+4 (Thumb) to match mGBA's pipeline view.
    pub fn trace_line(&self) -> String {
        let mut s = String::with_capacity(180);
        let pipeline_pc = self.regs.r[15].wrapping_add(if self.regs.thumb() { 4 } else { 8 });
        for i in 0..15 {
            s.push_str(&format!("{:08X} ", self.regs.r[i]));
        }
        s.push_str(&format!("{:08X} cpsr: {:08X}", pipeline_pc, self.regs.cpsr));
        s
    }
}
