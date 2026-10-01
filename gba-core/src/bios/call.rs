//! Guest callbacks from HLE BIOS routines.
//!
//! Some BIOS functions call back into the game: `SoundDriverMain` runs the music player the game
//! registered in its sound work area (`func(intp)`), then the game's CGB channel routine, and only
//! then mixes. An HLE routine is a Rust function that must return to the CPU loop before guest
//! code can run, so a call is split into stages: the routine records what it still has to do in a
//! `Continuation`, loads the arguments, points `lr` at `HLE_CALL_RETURN_ADDR` and jumps to the
//! guest function. When the guest returns to that address the CPU's BIOS-region trap
//! (`hle_vector`) pops the continuation and runs the routine's next stage. The last stage restores
//! the caller's registers and PC exactly as the real SWI handler's `movs pc, lr` would.
//!
//! Interrupts stay live during a callback (the guest code runs as ordinary code; an IRQ taken at
//! `pc = HLE_CALL_RETURN_ADDR` returns there and the trap fires afterwards). Calls nest: a stage
//! may call another guest function, and a jump-list handler reached from inside a callback may
//! call one too; the guest returns inner calls first, so the stack is strictly LIFO.
//!
//! Source: GBATEK "BIOS Functions" (the SWI handler runs in System mode on the user stack, so a
//! callback issued from an IRQ handler's SWI must not run on the IRQ stack) and "BIOS Sound
//! Functions" (`func`/`intp` in the SoundArea).

use super::HLE_CALL_RETURN_ADDR;
use crate::bus::{Bus, MemoryBus};
use crate::cpu::regs::{Regs, FLAG_T, MODE_SYS, MODE_USR};

/// The caller's registers an HLE routine must put back when it finishes.
#[cfg_attr(feature = "savestate", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SavedFrame {
    /// r0-r3 at entry (restored only when the routine returns nothing).
    pub r0_3: [u32; 4],
    pub r12: u32,
    /// Address execution resumes at, aligned for the T bit in `cpsr`.
    pub pc: u32,
    /// The caller's CPSR (mode, flags, T).
    pub cpsr: u32,
    /// Run callbacks in System mode (an SWI's contract) rather than in the current mode (a
    /// jump-list handler reached by `bx`).
    pub sys_mode: bool,
    /// `lr` of the mode the callbacks run in, captured when the first call clobbers it.
    pub callee_lr: Option<u32>,
}

impl SavedFrame {
    /// Frame of an SWI: `regs.r[15]` is already the address after the SWI instruction.
    pub fn from_swi(regs: &Regs) -> Self {
        SavedFrame {
            r0_3: [regs.r[0], regs.r[1], regs.r[2], regs.r[3]],
            r12: regs.r[12],
            pc: regs.r[15],
            cpsr: regs.cpsr,
            sys_mode: true,
            callee_lr: None,
        }
    }

    /// Frame of a routine reached by `bx`/`blx` (a jump-list slot): the return address and state
    /// are in `lr`.
    pub fn from_bx_lr(regs: &Regs) -> Self {
        let lr = regs.r[14];
        let (pc, cpsr) = if lr & 1 != 0 { (lr & !1, regs.cpsr | FLAG_T) } else { (lr & !3, regs.cpsr & !FLAG_T) };
        SavedFrame { r0_3: [regs.r[0], regs.r[1], regs.r[2], regs.r[3]], r12: regs.r[12], pc, cpsr, sys_mode: false, callee_lr: Some(lr) }
    }
}

/// Stages of SoundDriverMain (SWI 1Ch) after a callback returns.
#[cfg_attr(feature = "savestate", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SoundMainStage {
    /// Nothing called yet.
    Start,
    /// `func(intp)` has returned; call `CgbSound` next.
    AfterFunc,
    /// `CgbSound` has returned; mix.
    AfterCgb,
}

/// What an HLE routine still has to do once the guest function it called returns.
#[cfg_attr(feature = "savestate", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pending {
    SoundMain { area: u32, stage: SoundMainStage },
    /// A SoundGetJumpList handler: slot, its own stage number and scratch words.
    JumpList { slot: u8, stage: u8, data: [u32; 4] },
}

#[cfg_attr(feature = "savestate", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Continuation {
    pub saved: SavedFrame,
    pub pending: Pending,
}

/// Cycles the return trampoline charges (a `bx lr`-sized epilogue, as the IRQ return stub).
const CALL_RETURN_CYCLES: u32 = 2;

impl Bus {
    /// Enter guest `func` with `args` in r0.., arranging for `pending` to run when it returns.
    /// `saved` is the frame of the routine that is being suspended; a multi-stage routine passes
    /// the frame it was given so the original caller's registers survive every stage.
    pub(crate) fn hle_call_guest(&mut self, regs: &mut Regs, mut saved: SavedFrame, func: u32, args: &[u32], pending: Pending) {
        if saved.sys_mode && !matches!(regs.mode(), MODE_SYS | MODE_USR) {
            // GBATEK: the SWI handler switches to System mode so that the callee uses the user
            // stack; the caller's mode (with its banked sp/lr) comes back in `hle_call_finish`.
            regs.set_cpsr((regs.cpsr & !0x1F) | MODE_SYS);
        }
        if saved.callee_lr.is_none() {
            saved.callee_lr = Some(regs.r[14]);
        }
        self.bios.calls.push(Continuation { saved, pending });
        for (i, &a) in args.iter().enumerate().take(4) {
            regs.r[i] = a;
        }
        regs.r[14] = HLE_CALL_RETURN_ADDR;
        if func & 1 != 0 {
            regs.set_cpsr(regs.cpsr | FLAG_T);
            regs.r[15] = func & !1;
        } else {
            regs.set_cpsr(regs.cpsr & !FLAG_T);
            regs.r[15] = func & !3;
        }
    }

    /// The trap at `HLE_CALL_RETURN_ADDR`: resume the innermost suspended routine. False when
    /// nothing is pending (the guest jumped there on its own; the open-bus word executes).
    pub(crate) fn hle_call_return(&mut self, regs: &mut Regs) -> bool {
        let Some(c) = self.bios.calls.pop() else { return false };
        self.idle(CALL_RETURN_CYCLES);
        match c.pending {
            Pending::SoundMain { area, stage } => self.sound_main_resume(regs, c.saved, area, stage),
            Pending::JumpList { slot, stage, data } => self.jumplist_resume(regs, c.saved, slot, stage, data),
        }
        true
    }

    /// Final stage of a routine that called guest code: put the caller's registers, mode and PC
    /// back. `restore_r0_3` for routines that return nothing (every sound SWI).
    pub(crate) fn hle_call_finish(&mut self, regs: &mut Regs, saved: &SavedFrame, restore_r0_3: bool) {
        if let Some(lr) = saved.callee_lr {
            regs.r[14] = lr;
        }
        regs.set_cpsr(saved.cpsr);
        regs.r[12] = saved.r12;
        if restore_r0_3 {
            regs.r[..4].copy_from_slice(&saved.r0_3);
        }
        regs.r[15] = saved.pc;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bios::test_util::bus;
    use crate::cpu::regs::{FLAG_I, MODE_IRQ};

    fn sys_regs() -> Regs {
        let mut regs = Regs::new();
        regs.set_cpsr(MODE_SYS);
        regs
    }

    #[test]
    fn call_round_trip_thumb_and_arm() {
        for &(func, thumb) in &[(0x0803_B4A9u32, true), (0x0803_B4A8u32, false)] {
            let mut b = bus();
            let mut regs = sys_regs();
            regs.r[0] = 0x11;
            regs.r[3] = 0x33;
            regs.r[12] = 0xCC;
            regs.r[13] = 0x0300_7F00;
            regs.r[14] = 0x0800_0F01;
            regs.r[15] = 0x0800_0104; // post-SWI address
            let saved = SavedFrame::from_swi(&regs);
            let pending = Pending::SoundMain { area: 0x0300_5C90, stage: SoundMainStage::AfterFunc };
            b.hle_call_guest(&mut regs, saved, func, &[0x0300_6E50], pending);
            assert_eq!(regs.r[0], 0x0300_6E50, "argument in r0");
            assert_eq!(regs.r[14], HLE_CALL_RETURN_ADDR);
            assert_eq!(regs.r[15], func & !1);
            assert_eq!(regs.thumb(), thumb);
            assert_eq!(regs.mode(), MODE_SYS);
            assert_eq!(b.bios.calls.len(), 1);
            assert_eq!(b.bios.calls[0].saved.callee_lr, Some(0x0800_0F01));
            // The guest clobbers scratch registers and returns.
            regs.r[0] = 0;
            regs.r[3] = 0;
            regs.r[12] = 0;
            regs.r[14] = HLE_CALL_RETURN_ADDR;
            regs.r[15] = HLE_CALL_RETURN_ADDR;
            // Give the area a valid ident so the SoundMain stage runs to completion: with no
            // CgbSound pointer it mixes (nothing active) and finishes.
            b.write32(0x0300_7FF0, 0x0300_5C90, crate::Access::N);
            b.write32(0x0300_5C90, crate::bios::sound::ID_NUMBER + 1, crate::Access::N);
            b.write8(0x0300_5C90 + 0x0B, 7, crate::Access::N);
            b.write8(0x0300_5C90 + 0x04, 7, crate::Access::N);
            b.write32(0x0300_5C90 + 0x10, 224, crate::Access::N);
            b.write32(0x0300_5C90 + 0x14, 13379, crate::Access::N);
            assert!(b.hle_vector(&mut regs));
            assert!(b.bios.calls.is_empty());
            assert_eq!(regs.r[0], 0x11);
            assert_eq!(regs.r[3], 0x33);
            assert_eq!(regs.r[12], 0xCC);
            assert_eq!(regs.r[14], 0x0800_0F01, "caller's lr back");
            assert_eq!(regs.r[15], 0x0800_0104);
            assert!(!regs.thumb());
            assert_eq!(regs.mode(), MODE_SYS);
            assert_eq!(b.peek32(0x0300_5C90), crate::bios::sound::ID_NUMBER, "unlocked");
        }
    }

    #[test]
    fn call_from_irq_mode_runs_in_system_mode_and_restores_irq_mode() {
        let mut b = bus();
        let mut regs = sys_regs();
        regs.r[13] = 0x0300_7F00; // user sp
        regs.r[14] = 0x0800_0F01; // user lr
        regs.switch_mode(MODE_IRQ);
        regs.cpsr |= FLAG_I;
        regs.r[13] = 0x0300_7FA0; // irq sp
        regs.r[14] = 0x0800_0204; // irq lr (return into the interrupted code)
        regs.r[15] = 0x0300_0104;
        let saved = SavedFrame::from_swi(&regs);
        b.hle_call_guest(&mut regs, saved, 0x0800_1001, &[], Pending::SoundMain { area: 0, stage: SoundMainStage::AfterCgb });
        assert_eq!(regs.mode(), MODE_SYS, "callback runs in System mode");
        assert!(regs.irq_disabled(), "I flag kept");
        assert_eq!(regs.r[13], 0x0300_7F00, "on the user stack");
        assert_eq!(regs.r[14], HLE_CALL_RETURN_ADDR);
        assert_eq!(regs.lr_of(MODE_IRQ), 0x0800_0204, "IRQ-mode lr untouched");
        let frame = b.bios.calls[0].saved;
        b.hle_call_finish(&mut regs, &frame, true);
        assert_eq!(regs.mode(), MODE_IRQ);
        assert_eq!(regs.r[13], 0x0300_7FA0);
        assert_eq!(regs.r[14], 0x0800_0204);
        assert_eq!(regs.lr_of(MODE_SYS), 0x0800_0F01, "user lr restored");
        assert_eq!(regs.r[15], 0x0300_0104);
    }

    #[test]
    fn return_trap_without_a_pending_call_is_not_handled() {
        let mut b = bus();
        let mut regs = sys_regs();
        regs.r[15] = HLE_CALL_RETURN_ADDR;
        assert!(!b.hle_vector(&mut regs));
    }

    #[test]
    fn nested_calls_pop_in_lifo_order() {
        let mut b = bus();
        let mut regs = sys_regs();
        regs.r[15] = 0x0800_0104;
        let outer = SavedFrame::from_swi(&regs);
        b.hle_call_guest(&mut regs, outer, 0x0800_2000, &[], Pending::SoundMain { area: 1, stage: SoundMainStage::AfterFunc });
        // Inside the callback a jump-list handler calls another guest routine.
        regs.r[14] = 0x0803_B5AB;
        let inner = SavedFrame::from_bx_lr(&regs);
        assert_eq!(inner.pc, 0x0803_B5AA);
        assert!(inner.cpsr & FLAG_T != 0);
        b.hle_call_guest(&mut regs, inner, 0x0803_B711, &[1, 2], Pending::JumpList { slot: 0, stage: 1, data: [0; 4] });
        assert_eq!(b.bios.calls.len(), 2);
        assert!(matches!(b.bios.calls[1].pending, Pending::JumpList { .. }));
        assert!(matches!(b.bios.calls[0].pending, Pending::SoundMain { .. }));
        let top = b.bios.calls.pop().unwrap();
        assert_eq!(top.saved.callee_lr, Some(0x0803_B5AB));
    }

    #[test]
    fn from_bx_lr_frame_restores_the_callers_state() {
        let mut b = bus();
        let mut regs = sys_regs();
        regs.r[0] = 0x0300_6DD0;
        regs.r[1] = 0x0300_0FA8;
        regs.r[14] = 0x0803_B5AB;
        regs.r[15] = 0x0000_0200;
        let saved = SavedFrame::from_bx_lr(&regs);
        b.hle_call_finish(&mut regs, &saved, true);
        assert_eq!(regs.r[15], 0x0803_B5AA);
        assert!(regs.thumb());
        assert_eq!(regs.r[0], 0x0300_6DD0);
        assert_eq!(regs.r[14], 0x0803_B5AB);
    }
}
