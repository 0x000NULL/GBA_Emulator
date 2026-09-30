//! ARM7TDMI interpreter. `regs.rs` is shared with the HLE BIOS, so its API is frozen.
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
//! Pipeline model: `regs.r[15]` is the address of the instruction being executed for the whole of
//! `step`. `pipeline[0]` holds the (already fetched) word at PC and `pipeline[1]` the word at PC+4
//! (PC+2 in THUMB). Each step first prefetches PC+8 (PC+4), shifts the pipeline, then executes the
//! word that was fetched two steps earlier, so a store that overwrites an already-fetched
//! instruction has no effect on it (Classic NES Series relies on this). Any write to PC
//! invalidates the pipeline; the same `step` ends by refilling it with a non-sequential fetch at
//! the target followed by a sequential one, which together with the branch's own prefetch gives
//! GBATEK's 2S+1N. The refill belongs to the branch (the ARM7TDMI only samples nIRQ once the
//! instruction, refill included, has finished), so an IRQ that is pending when an exception return
//! re-enables interrupts is taken after the refill and before the target instruction runs.
//! Exception entry is the exception: it leaves the pipeline empty and the vector code (or the HLE
//! vector emulation) pays for the vector fetches.
//!
//! Opcode fetch sequentiality (the `Access` passed to `fetch16/32`) follows the ARM7TDMI SEQ
//! signal as the GBA memory controller sees it: a fetch is sequential only when the bus cycle
//! right before it was the fetch of the preceding opcode. After a data access (STR/STM/SWP: the
//! datasheet's "2N") or after an internal cycle (LDR/LDM/MUL/shift-by-register: GBATEK's
//! "prefetch disable bug", which turns the 1S opcode fetch into 1N when the game pak prefetch
//! buffer is off) the next fetch is non-sequential. Since the pipeline fetch is the first bus
//! cycle of every step, "the instruction advanced the clock during execute" is exactly "it did a
//! data access or an internal cycle", which is how `next_fetch_n` is derived. The first halfword
//! of each 128K game pak block is also non-sequential (GBATEK "GBA System Control", WAITCNT
//! notes). With the prefetch buffer enabled the bus decides hits by address and may ignore the
//! flag for ROM fetches.
//!
//! Sources: GBATEK "ARM CPU Reference" (register set, flags, exceptions, memory alignments, every
//! ARM and THUMB opcode chapter, instruction cycle times).

pub mod alu;
pub mod arm;
pub mod decode;
pub mod disasm;
pub mod regs;
pub mod thumb;

#[cfg(test)]
mod tests;

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

#[cfg_attr(feature = "savestate", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug)]
pub struct Cpu {
    pub regs: Regs,
    /// Two-slot fetched pipeline: `[word at PC, word at PC+4]` (PC+2 in THUMB) while
    /// `next_fetch_seq` is true. The instruction at `regs.r[15]` is the one being executed.
    pub pipeline: [u32; 2],
    /// True while the pipeline contents are valid. Cleared by every PC write; the next `step`
    /// refills with an N fetch then an S fetch.
    pub next_fetch_seq: bool,
    /// True when the last bus cycle of the previous instruction was a data access or an internal
    /// cycle, which makes the next opcode fetch non-sequential (see the module docs).
    pub next_fetch_n: bool,
    /// The next pipeline refill starts sequentially: set after an HLE SWI whose real handler
    /// ends with a game pak data read (`ldrb r12, [lr, #-2]`) right before `movs pc, lr`; the
    /// return address is the next halfword, so the cartridge sees a sequential address.
    pub refill_seq: bool,
    /// Instructions executed since reset (exception entries are not counted).
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
        Cpu {
            regs: Regs::new(),
            pipeline: [0; 2],
            next_fetch_seq: false,
            next_fetch_n: false,
            refill_seq: false,
            instr_count: 0,
            last_opcode: 0,
        }
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
        self.next_fetch_n = false;
    }

    /// Real reset: SVC mode, IRQ/FIQ disabled, PC=0 (LLE BIOS).
    pub fn reset_lle(&mut self) {
        self.regs = Regs::new();
        self.regs.r[15] = 0;
        self.pipeline = [0; 2];
        self.next_fetch_seq = false;
        self.next_fetch_n = false;
    }

    /// Address of the instruction about to execute.
    #[inline]
    pub fn pc(&self) -> u32 {
        self.regs.r[15]
    }

    /// Flush the pipeline after a PC write (branch). Refilled on the next `step`.
    #[inline]
    pub fn flush_pipeline(&mut self) {
        self.next_fetch_seq = false;
    }

    /// Write a new PC (already computed by the instruction), force-align it for the current state
    /// and invalidate the pipeline. Source: GBATEK "Mis-aligned PC/R15".
    #[inline]
    pub(crate) fn set_pc(&mut self, target: u32) {
        self.regs.r[15] = if self.regs.thumb() {
            target & !1
        } else {
            target & !3
        };
        self.next_fetch_seq = false;
    }

    /// Execute one instruction. Returns cycles consumed (informational; the bus keeps the clock).
    pub fn step<B: MemoryBus>(&mut self, bus: &mut B) -> u32 {
        let start = bus.now();
        if bus.irq_pending() && !self.regs.irq_disabled() {
            self.enter_exception(bus, VEC_IRQ, regs::MODE_IRQ);
            // Exception entry is 2S + 1N on the ARM7TDMI: the vector fetch pair (paid by the
            // refill at 0x18, or by the HLE vector emulation) plus one cycle saving state, and
            // one more cycle to sample the IRQ input before the vector fetch (hardware-recorded
            // mgba-suite "Timer IRQ tests": the value latched in the handler is one cycle later
            // than the BIOS instruction sequence alone accounts for).
            bus.idle(2);
            return (bus.now() - start) as u32;
        }
        if self.regs.r[15] < 0x4000 && bus.is_hle_bios() && bus.bios_hle_vector(&mut self.regs) {
            self.flush_pipeline();
            self.refill_pipeline(bus);
            return (bus.now() - start) as u32;
        }
        if self.regs.thumb() {
            self.step_thumb(bus);
        } else {
            self.step_arm(bus);
        }
        self.instr_count += 1;
        if !self.next_fetch_seq {
            self.refill_pipeline(bus);
        }
        (bus.now() - start) as u32
    }

    /// Refill the pipeline after a PC write: N fetch at PC, S fetch at PC+size.
    fn refill_pipeline<B: MemoryBus>(&mut self, bus: &mut B) {
        if self.regs.thumb() {
            let pc = self.regs.r[15] & !1;
            self.regs.r[15] = pc;
            self.pipeline[0] = bus.fetch16(pc, Access::N) as u32;
            let next = pc.wrapping_add(2);
            self.pipeline[1] = bus.fetch16(next, Self::seq_unless_block_start(next)) as u32;
        } else {
            let pc = self.regs.r[15] & !3;
            self.regs.r[15] = pc;
            self.pipeline[0] = bus.fetch32(pc, Access::N);
            let next = pc.wrapping_add(4);
            self.pipeline[1] = bus.fetch32(next, Self::seq_unless_block_start(next));
        }
        self.next_fetch_seq = true;
    }

    /// Access type of the pipeline fetch at `addr` when the pipeline is already valid: sequential
    /// unless the previous instruction ended with a data access or an internal cycle, or `addr`
    /// starts a 128K game pak block.
    #[inline]
    fn fetch_access(&self, addr: u32) -> Access {
        if self.next_fetch_n {
            Access::N
        } else {
            Self::seq_unless_block_start(addr)
        }
    }

    /// Sequential access unless `addr` is the first halfword of a 128K game pak block, which the
    /// GBA always times as non-sequential (GBATEK "GBA System Control", WAITCNT notes).
    #[inline]
    pub(crate) fn seq_unless_block_start(addr: u32) -> Access {
        if addr & 0x1_FFFF == 0 {
            Access::N
        } else {
            Access::S
        }
    }

    fn step_arm<B: MemoryBus>(&mut self, bus: &mut B) {
        let pc = self.regs.r[15] & !3;
        self.regs.r[15] = pc;
        if !self.next_fetch_seq {
            let first = if std::mem::take(&mut self.refill_seq) { Self::seq_unless_block_start(pc) } else { Access::N };
            self.pipeline[0] = bus.fetch32(pc, first);
            let next = pc.wrapping_add(4);
            self.pipeline[1] = bus.fetch32(next, Self::seq_unless_block_start(next));
            self.next_fetch_seq = true;
        }
        let op = self.pipeline[0];
        self.pipeline[0] = self.pipeline[1];
        let next = pc.wrapping_add(8);
        self.pipeline[1] = bus.fetch32(next, self.fetch_access(next));
        self.last_opcode = op;
        let before = bus.now();
        arm::execute(self, bus, op);
        self.next_fetch_n = bus.now() != before;
        if self.next_fetch_seq {
            self.regs.r[15] = pc.wrapping_add(4);
        }
    }

    fn step_thumb<B: MemoryBus>(&mut self, bus: &mut B) {
        let pc = self.regs.r[15] & !1;
        self.regs.r[15] = pc;
        if !self.next_fetch_seq {
            let first = if std::mem::take(&mut self.refill_seq) { Self::seq_unless_block_start(pc) } else { Access::N };
            self.pipeline[0] = bus.fetch16(pc, first) as u32;
            let next = pc.wrapping_add(2);
            self.pipeline[1] = bus.fetch16(next, Self::seq_unless_block_start(next)) as u32;
            self.next_fetch_seq = true;
        }
        let op = self.pipeline[0] as u16;
        self.pipeline[0] = self.pipeline[1];
        let next = pc.wrapping_add(4);
        self.pipeline[1] = bus.fetch16(next, self.fetch_access(next)) as u32;
        self.last_opcode = op as u32;
        let before = bus.now();
        thumb::execute(self, bus, op);
        self.next_fetch_n = bus.now() != before;
        if self.next_fetch_seq {
            self.regs.r[15] = pc.wrapping_add(2);
        }
    }

    /// Exception entry: SPSR_mode = CPSR, LR_mode = return address, switch mode, ARM state,
    /// I set (and F set for FIQ/reset), PC = vector, pipeline flushed. `regs.r[15]` must hold the
    /// address of the instruction being executed / interrupted. The return address stored in LR
    /// follows the ARM7TDMI convention for each exception (IRQ: PC+4 so `subs pc, lr, #4` resumes
    /// the interrupted instruction in either state; SWI/UND: the following instruction).
    pub fn enter_exception<B: MemoryBus>(&mut self, _bus: &mut B, vector: u32, mode: u32) {
        let old_cpsr = self.regs.cpsr;
        let pc = self.regs.r[15];
        let thumb = self.regs.thumb();
        let lr = match vector {
            VEC_IRQ | VEC_FIQ => pc.wrapping_add(4),
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
    }

    /// Software interrupt from either state. `pc` is the SWI's own address. Tries the HLE BIOS
    /// first with r15 pointing at the following instruction; falls back to the real exception.
    pub(crate) fn software_interrupt<B: MemoryBus>(&mut self, bus: &mut B, num: u8) {
        let pc = self.regs.r[15];
        let next = pc.wrapping_add(if self.regs.thumb() { 2 } else { 4 });
        self.regs.r[15] = next;
        if bus.swi_hle(num, &mut self.regs) {
            if self.regs.r[15] != next {
                // A waiting SWI rewound r15 (or the HLE redirected); refetch there.
                self.flush_pipeline();
            } else if let Some(seq) = bus.swi_took_refill() {
                // The real handler returns with `movs pc, lr`: the pipeline refills from the
                // return address (N then S fetch in the caller's region).
                self.regs.r[15] = next;
                self.flush_pipeline();
                self.refill_seq = seq;
            } else {
                self.regs.r[15] = pc;
            }
            return;
        }
        self.regs.r[15] = pc;
        self.enter_exception(bus, VEC_SWI, regs::MODE_SVC);
    }

    /// Undefined instruction trap (vector 0x04). 2S+1I+1N: one I cycle here, the rest is fetches.
    pub(crate) fn undefined_instruction<B: MemoryBus>(&mut self, bus: &mut B) {
        bus.idle(1);
        self.enter_exception(bus, VEC_UNDEFINED, regs::MODE_UND);
    }

    /// Restore CPSR from the current mode's SPSR (S-bit ALU with Rd=PC, LDM with PC and S).
    /// Has no effect in User/System mode, which have no SPSR (behaviour is unpredictable there).
    #[inline]
    pub(crate) fn restore_cpsr_from_spsr(&mut self) {
        if regs::bank_index(self.regs.mode()) != 0 {
            let spsr = self.regs.spsr();
            self.regs.set_cpsr(spsr);
        }
    }

    // ------------------------------------------------------------------ data access helpers

    /// LDR: word read with the ARM7 rotation for mis-aligned addresses. The unaligned address
    /// goes out on the bus as is: 16/32-bit memories ignore the low bits, but the 8-bit SRAM bus
    /// serves the byte those bits select, replicated on every lane (mgba-suite "SRAM load").
    #[inline]
    pub(crate) fn load_word<B: MemoryBus>(&mut self, bus: &mut B, addr: u32, a: Access) -> u32 {
        let v = bus.read32(addr, a);
        v.rotate_right((addr & 3) * 8)
    }

    /// LDRH: halfword read; an odd address yields the aligned halfword rotated right by 8
    /// (or, on the SRAM bus, the odd byte replicated then rotated).
    #[inline]
    pub(crate) fn load_half<B: MemoryBus>(&mut self, bus: &mut B, addr: u32, a: Access) -> u32 {
        let v = bus.read16(addr, a) as u32;
        if addr & 1 != 0 {
            v.rotate_right(8)
        } else {
            v
        }
    }

    /// LDRSH: sign-extended halfword; an odd address degrades to a sign-extended byte.
    #[inline]
    pub(crate) fn load_signed_half<B: MemoryBus>(
        &mut self,
        bus: &mut B,
        addr: u32,
        a: Access,
    ) -> u32 {
        if addr & 1 != 0 {
            bus.read8(addr, a) as i8 as i32 as u32
        } else {
            bus.read16(addr, a) as i16 as i32 as u32
        }
    }

    /// Block transfer shared by ARM LDM/STM and THUMB LDMIA/STMIA/PUSH/POP.
    ///
    /// Semantics (GBATEK "Block Data Transfer", ARMv4 column): registers are always processed
    /// from the lowest address upwards; an empty list transfers R15 and moves the base by 0x40;
    /// STM with the base in the list stores the old base if it is the first register, otherwise
    /// the new one; LDM with the base in the list lets the loaded value win over writeback; with
    /// `user_bank` set, registers are the User-mode bank (S bit without PC in list); with
    /// `restore_cpsr` set, CPSR = SPSR when PC is loaded (S bit with PC in list).
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn block_transfer<B: MemoryBus>(
        &mut self,
        bus: &mut B,
        rn: usize,
        mut rlist: u32,
        load: bool,
        pre: bool,
        up: bool,
        writeback: bool,
        user_bank: bool,
        restore_cpsr: bool,
    ) {
        let base = self.regs.r[rn];
        let count = if rlist == 0 {
            rlist = 1 << 15;
            16
        } else {
            rlist.count_ones()
        };
        let span = count * 4;
        let new_base = if up {
            base.wrapping_add(span)
        } else {
            base.wrapping_sub(span)
        };
        let mut addr = match (pre, up) {
            (false, true) => base,                                     // IA
            (true, true) => base.wrapping_add(4),                      // IB
            (false, false) => base.wrapping_sub(span).wrapping_add(4), // DA
            (true, false) => base.wrapping_sub(span),                  // DB
        };
        let thumb = self.regs.thumb();
        let pc_store = self.regs.r[15].wrapping_add(if thumb { 6 } else { 12 });
        // The first word is non-sequential; so is any word that starts a 128K game pak block,
        // which is also where a transfer running off the end of OAM enters the ROM (GBATEK WAITCNT
        // notes: "LDMIA [801fff8h],r0-r7 will have non-sequential timing at 8020000h").
        let mut access = Access::N;
        if load {
            if writeback {
                self.regs.r[rn] = new_base;
            }
            let mut loaded_pc = None;
            for i in 0..16 {
                if rlist & (1 << i) == 0 {
                    continue;
                }
                let v = bus.read32(addr & !3, access);
                addr = addr.wrapping_add(4);
                access = Self::seq_unless_block_start(addr);
                if i == 15 {
                    loaded_pc = Some(v);
                } else if user_bank {
                    self.regs.set_user_reg(i, v);
                } else {
                    self.regs.r[i] = v;
                }
            }
            bus.idle(1);
            if let Some(v) = loaded_pc {
                if restore_cpsr {
                    self.restore_cpsr_from_spsr();
                }
                self.set_pc(v);
            }
        } else {
            let mut first = true;
            for i in 0..16 {
                if rlist & (1 << i) == 0 {
                    continue;
                }
                let v = if i == 15 {
                    pc_store
                } else if user_bank {
                    self.regs.user_reg(i)
                } else {
                    self.regs.r[i]
                };
                bus.write32(addr & !3, v, access);
                addr = addr.wrapping_add(4);
                access = Self::seq_unless_block_start(addr);
                if first && writeback {
                    // Writeback happens after the first store, so a base that is not the first
                    // register in the list is stored with its new value.
                    self.regs.r[rn] = new_base;
                }
                first = false;
            }
        }
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
