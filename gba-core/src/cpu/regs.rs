//! ARM7TDMI register file with banked registers. Source: GBATEK "ARM CPU Register Set".
//!
//! Owned jointly by the CPU agent and the integrator: the HLE BIOS manipulates `Regs` directly
//! (SWI dispatch, IRQ vector emulation), so this API is frozen.

pub const MODE_USR: u32 = 0x10;
pub const MODE_FIQ: u32 = 0x11;
pub const MODE_IRQ: u32 = 0x12;
pub const MODE_SVC: u32 = 0x13;
pub const MODE_ABT: u32 = 0x17;
pub const MODE_UND: u32 = 0x1B;
pub const MODE_SYS: u32 = 0x1F;

pub const FLAG_N: u32 = 1 << 31;
pub const FLAG_Z: u32 = 1 << 30;
pub const FLAG_C: u32 = 1 << 29;
pub const FLAG_V: u32 = 1 << 28;
pub const FLAG_I: u32 = 1 << 7;
pub const FLAG_F: u32 = 1 << 6;
pub const FLAG_T: u32 = 1 << 5;

/// Bank slot per mode: 0 usr/sys, 1 fiq, 2 irq, 3 svc, 4 abt, 5 und.
#[inline]
pub fn bank_index(mode: u32) -> usize {
    match mode & 0x1F {
        MODE_FIQ => 1,
        MODE_IRQ => 2,
        MODE_SVC => 3,
        MODE_ABT => 4,
        MODE_UND => 5,
        _ => 0,
    }
}

#[cfg_attr(feature = "savestate", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug)]
pub struct Regs {
    /// Registers visible in the current mode. `r[15]` is the PC.
    pub r: [u32; 16],
    pub cpsr: u32,
    /// r8-r12 for [usr/sys/irq/svc/abt/und, fiq].
    banked_r8_12: [[u32; 5]; 2],
    /// r13 per bank slot.
    banked_sp: [u32; 6],
    /// r14 per bank slot.
    banked_lr: [u32; 6],
    /// SPSR per bank slot (slot 0 unused).
    banked_spsr: [u32; 6],
}

impl Default for Regs {
    fn default() -> Self {
        Self::new()
    }
}

impl Regs {
    pub fn new() -> Self {
        Regs {
            r: [0; 16],
            cpsr: MODE_SVC | FLAG_I | FLAG_F,
            banked_r8_12: [[0; 5]; 2],
            banked_sp: [0; 6],
            banked_lr: [0; 6],
            banked_spsr: [0; 6],
        }
    }

    #[inline]
    pub fn mode(&self) -> u32 {
        self.cpsr & 0x1F
    }

    #[inline]
    pub fn thumb(&self) -> bool {
        self.cpsr & FLAG_T != 0
    }

    #[inline]
    pub fn irq_disabled(&self) -> bool {
        self.cpsr & FLAG_I != 0
    }

    #[inline]
    pub fn pc(&self) -> u32 {
        self.r[15]
    }

    #[inline]
    pub fn flag_n(&self) -> bool {
        self.cpsr & FLAG_N != 0
    }
    #[inline]
    pub fn flag_z(&self) -> bool {
        self.cpsr & FLAG_Z != 0
    }
    #[inline]
    pub fn flag_c(&self) -> bool {
        self.cpsr & FLAG_C != 0
    }
    #[inline]
    pub fn flag_v(&self) -> bool {
        self.cpsr & FLAG_V != 0
    }

    #[inline]
    pub fn set_flag(&mut self, flag: u32, on: bool) {
        if on {
            self.cpsr |= flag;
        } else {
            self.cpsr &= !flag;
        }
    }

    /// Set NZ from a result.
    #[inline]
    pub fn set_nz(&mut self, v: u32) {
        self.cpsr = (self.cpsr & !(FLAG_N | FLAG_Z)) | (v & FLAG_N) | if v == 0 { FLAG_Z } else { 0 };
    }

    /// SPSR of the current mode (0 in usr/sys, which have none).
    pub fn spsr(&self) -> u32 {
        self.banked_spsr[bank_index(self.mode())]
    }

    pub fn set_spsr(&mut self, v: u32) {
        let i = bank_index(self.mode());
        if i != 0 {
            self.banked_spsr[i] = v;
        }
    }

    /// SPSR of an arbitrary mode (for the HLE BIOS IRQ return path).
    pub fn spsr_of(&self, mode: u32) -> u32 {
        self.banked_spsr[bank_index(mode)]
    }

    pub fn set_spsr_of(&mut self, mode: u32, v: u32) {
        let i = bank_index(mode);
        if i != 0 {
            self.banked_spsr[i] = v;
        }
    }

    /// Banked r13 of `mode` without switching.
    pub fn sp_of(&self, mode: u32) -> u32 {
        if bank_index(mode) == bank_index(self.mode()) {
            self.r[13]
        } else {
            self.banked_sp[bank_index(mode)]
        }
    }

    pub fn set_sp_of(&mut self, mode: u32, v: u32) {
        if bank_index(mode) == bank_index(self.mode()) {
            self.r[13] = v;
        } else {
            self.banked_sp[bank_index(mode)] = v;
        }
    }

    pub fn lr_of(&self, mode: u32) -> u32 {
        if bank_index(mode) == bank_index(self.mode()) {
            self.r[14]
        } else {
            self.banked_lr[bank_index(mode)]
        }
    }

    pub fn set_lr_of(&mut self, mode: u32, v: u32) {
        if bank_index(mode) == bank_index(self.mode()) {
            self.r[14] = v;
        } else {
            self.banked_lr[bank_index(mode)] = v;
        }
    }

    /// Write CPSR, swapping register banks if the mode changes. Invalid mode bits are kept as-is
    /// (the ARM7TDMI behaviour is unpredictable; we treat them as usr).
    pub fn set_cpsr(&mut self, v: u32) {
        let old = self.mode();
        let new = v & 0x1F;
        if old != new {
            self.switch_mode_banks(old, new);
        }
        self.cpsr = v;
    }

    /// Change only the mode bits (used by exception entry).
    pub fn switch_mode(&mut self, new_mode: u32) {
        let v = (self.cpsr & !0x1F) | (new_mode & 0x1F);
        self.set_cpsr(v);
    }

    fn switch_mode_banks(&mut self, old: u32, new: u32) {
        let oi = bank_index(old);
        let ni = bank_index(new);
        if oi == ni {
            return;
        }
        // Save outgoing bank.
        self.banked_sp[oi] = self.r[13];
        self.banked_lr[oi] = self.r[14];
        let old_fiq = (oi == 1) as usize;
        let new_fiq = (ni == 1) as usize;
        if old_fiq != new_fiq {
            self.banked_r8_12[old_fiq].copy_from_slice(&self.r[8..13]);
            self.r[8..13].copy_from_slice(&self.banked_r8_12[new_fiq]);
        }
        // Load incoming bank.
        self.r[13] = self.banked_sp[ni];
        self.r[14] = self.banked_lr[ni];
    }

    /// User-bank view of `reg` (for LDM/STM with S bit in a privileged mode).
    pub fn user_reg(&self, reg: usize) -> u32 {
        let i = bank_index(self.mode());
        match reg {
            8..=12 if i == 1 => self.banked_r8_12[0][reg - 8],
            13 if i != 0 => self.banked_sp[0],
            14 if i != 0 => self.banked_lr[0],
            _ => self.r[reg],
        }
    }

    pub fn set_user_reg(&mut self, reg: usize, v: u32) {
        let i = bank_index(self.mode());
        match reg {
            8..=12 if i == 1 => self.banked_r8_12[0][reg - 8] = v,
            13 if i != 0 => self.banked_sp[0] = v,
            14 if i != 0 => self.banked_lr[0] = v,
            _ => self.r[reg] = v,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bank_switch_preserves_sp_lr() {
        let mut r = Regs::new();
        r.set_cpsr(MODE_SYS);
        r.r[13] = 0x1000;
        r.r[14] = 0x2000;
        r.switch_mode(MODE_IRQ);
        assert_eq!(r.r[13], 0);
        r.r[13] = 0x3000;
        r.switch_mode(MODE_USR);
        assert_eq!(r.r[13], 0x1000);
        assert_eq!(r.r[14], 0x2000);
        assert_eq!(r.sp_of(MODE_IRQ), 0x3000);
    }

    #[test]
    fn fiq_banks_r8_r12() {
        let mut r = Regs::new();
        r.set_cpsr(MODE_SYS);
        r.r[8] = 8;
        r.r[12] = 12;
        r.switch_mode(MODE_FIQ);
        assert_eq!(r.r[8], 0);
        r.r[8] = 88;
        assert_eq!(r.user_reg(8), 8);
        r.switch_mode(MODE_SVC);
        assert_eq!(r.r[8], 8);
        assert_eq!(r.r[12], 12);
        r.switch_mode(MODE_FIQ);
        assert_eq!(r.r[8], 88);
    }
}
