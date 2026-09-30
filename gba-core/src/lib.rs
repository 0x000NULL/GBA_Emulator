//! gba-core: a cycle-scheduled Game Boy Advance emulator core with no I/O dependencies.
//!
//! Ownership tree (no `Rc<RefCell>`): [`Gba`] owns a [`Cpu`] and a [`Bus`]; the bus owns every
//! other subsystem. Subsystems never receive `&mut Bus`; event handlers get a [`SysCtx`] naming
//! the siblings they may touch. See `docs/ARCHITECTURE.md` for the timing contract.

pub mod apu;
pub mod bios;
pub mod bus;
pub mod cart;
pub mod cpu;
pub mod dma;
pub mod gba;
pub mod irq;
pub mod keypad;
pub mod ppu;
pub mod sched;
#[cfg(feature = "savestate")]
pub mod state;
pub mod timers;
pub mod timing;

pub use bus::{Access, Bus, MemoryBus, SysCtx};
pub use cart::SaveType;
pub use cpu::{Cpu, Regs};
pub use gba::{BiosMode, Gba, Options, StopReason};
pub use irq::Irq;
pub use keypad::Keys;
pub use sched::Event;

/// Screen width in pixels.
pub const SCREEN_W: usize = 240;
/// Screen height in pixels.
pub const SCREEN_H: usize = 160;
/// Framebuffer type: BGR555 (bit 15 clear) as the PPU emits it.
pub type FrameBuffer = [u16; SCREEN_W * SCREEN_H];

#[derive(Debug)]
pub enum CoreError {
    RomTooLarge(usize),
    RomTooSmall(usize),
    BadBios(usize),
}

impl std::fmt::Display for CoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CoreError::RomTooLarge(n) => write!(f, "ROM is {n} bytes; max is 32 MiB"),
            CoreError::RomTooSmall(n) => write!(f, "ROM is {n} bytes; need at least a 192-byte header"),
            CoreError::BadBios(n) => write!(f, "BIOS is {n} bytes; expected 16384"),
        }
    }
}
impl std::error::Error for CoreError {}
