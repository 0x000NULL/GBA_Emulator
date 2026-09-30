//! Top-level machine: owns the CPU and the bus and runs the main loop.

use crate::bios::Bios;
use crate::bus::debug::Hit;
use crate::bus::Bus;
use crate::cart::{Cartridge, SaveType};
use crate::cpu::Cpu;
use crate::keypad::Keys;
use crate::timing::CYCLES_PER_FRAME;
use crate::{CoreError, FrameBuffer};

#[derive(Clone, Debug)]
pub enum BiosMode {
    /// High-level emulation (no BIOS image).
    Hle,
    /// Low-level emulation from a 16 KiB image.
    Lle(Vec<u8>),
}

#[derive(Clone, Debug)]
pub struct Options {
    pub bios: BiosMode,
    /// Override the save type detected from the ROM.
    pub save_type: Option<SaveType>,
}

impl Default for Options {
    fn default() -> Self {
        Options { bios: BiosMode::Hle, save_type: None }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StopReason {
    /// Reached the requested cycle/frame boundary.
    Done,
    /// A debug hook fired.
    Breakpoint(Hit),
}

#[cfg_attr(feature = "savestate", derive(serde::Serialize, serde::Deserialize))]
pub struct Gba {
    pub cpu: Cpu,
    pub bus: Bus,
}

impl Gba {
    pub fn new(rom: Vec<u8>, opts: Options) -> Result<Self, CoreError> {
        let cart = Cartridge::new(rom, opts.save_type)?;
        let (bios, hle) = match &opts.bios {
            BiosMode::Hle => (Bios::hle(), true),
            BiosMode::Lle(data) => (Bios::lle(data)?, false),
        };
        let mut bus = Bus::new(cart, bios);
        let mut cpu = Cpu::new();
        if hle {
            cpu.reset_hle();
            bus.postflg = 1;
            // RCNT = 0x8000 after the BIOS runs.
            bus.sio.reset();
        } else {
            cpu.reset_lle();
        }
        {
            let Bus { sched, irq, dma, ppu, .. } = &mut bus;
            ppu.boot_schedule(&mut crate::bus::SysCtx { sched, irq, dma }, hle);
        }
        bus.apu.boot_schedule(&mut bus.sched);
        Ok(Gba { cpu, bus })
    }

    /// Run until the start of the next V-Blank (line 160) and return the completed frame.
    pub fn run_frame(&mut self) -> StopReason {
        loop {
            // Advance one line at a time so a breakpoint inside a frame is honoured promptly.
            let target = self.bus.sched.next_deadline().min(self.bus.sched.now + CYCLES_PER_FRAME);
            let r = self.run_until(target.max(self.bus.sched.now + 1));
            if r != StopReason::Done {
                return r;
            }
            if self.bus.ppu.take_frame_ready() {
                return StopReason::Done;
            }
        }
    }

    pub fn framebuffer(&self) -> &FrameBuffer {
        &self.bus.ppu.fb
    }

    /// Run until `sched.now >= target` or a debug hook fires.
    pub fn run_until(&mut self, target: u64) -> StopReason {
        while self.bus.sched.now < target {
            // The deadline is re-read every iteration: an instruction (or DMA) may schedule an
            // event closer than the previous one (e.g. immediate DMA start 2 cycles after enable).
            if self.bus.sched.now >= self.bus.sched.next_deadline() {
                self.bus.process_events();
                continue;
            }
            if self.bus.dma.pending != 0 {
                self.bus.dma_run();
                continue;
            }
            // Requests raised with a latency (timer overflows) reach the CPU line here; an
            // `IrqLatch` event guarantees the halted-CPU sleep below ends when one matures.
            self.bus.irq.sync(self.bus.sched.now);
            if self.bus.irq.halted || self.bus.irq.stopped {
                if self.bus.irq.wake_condition() {
                    self.bus.irq.halted = false;
                    self.bus.irq.stopped = false;
                } else {
                    // Sleep until the next event (or the target).
                    self.bus.sched.now = self.bus.sched.next_deadline().min(target);
                    continue;
                }
            }
            if self.bus.debug.enabled {
                self.bus.debug.check_pc(self.cpu.pc());
                if let Some(hit) = self.bus.debug.hit.take() {
                    return StopReason::Breakpoint(hit);
                }
            }
            self.cpu.step(&mut self.bus);
            if self.bus.debug.enabled {
                if let Some(hit) = self.bus.debug.hit.take() {
                    return StopReason::Breakpoint(hit);
                }
            }
        }
        // Fire anything that became due exactly at the target.
        if self.bus.sched.now >= self.bus.sched.next_deadline() {
            self.bus.process_events();
        }
        StopReason::Done
    }

    pub fn run_cycles(&mut self, n: u64) -> StopReason {
        let target = self.bus.sched.now + n;
        self.run_until(target)
    }

    /// Run until the CPU is about to execute `pc`, or `max_cycles` elapse. Returns true if reached.
    pub fn run_until_pc(&mut self, pc: u32, max_cycles: u64) -> bool {
        let had = self.bus.debug.pc_breaks.contains(&pc);
        if !had {
            self.bus.debug.pc_breaks.push(pc);
        }
        self.bus.debug.enabled = true;
        let r = self.run_cycles(max_cycles);
        if !had {
            self.bus.debug.pc_breaks.retain(|&p| p != pc);
        }
        self.bus.debug.refresh_enabled();
        matches!(r, StopReason::Breakpoint(Hit::Pc(p)) if p == pc)
    }

    pub fn set_keys(&mut self, keys: Keys) {
        let Bus { keypad, irq, .. } = &mut self.bus;
        keypad.set_keys(keys, irq);
    }

    /// Move all pending audio samples into `out`.
    pub fn drain_audio(&mut self, out: &mut Vec<[i16; 2]>) {
        out.append(&mut self.bus.apu.samples);
    }

    pub fn save_data(&self) -> Option<Vec<u8>> {
        self.bus.cart.save_data()
    }

    pub fn load_save(&mut self, data: &[u8]) {
        self.bus.cart.load_save(data);
    }

    /// True if save memory changed since the last call; clears the flag.
    pub fn save_dirty(&mut self) -> bool {
        std::mem::replace(&mut self.bus.cart.dirty, false)
    }

    /// Lines printed by the guest through the mGBA debug registers since the last call.
    pub fn take_debug_log(&mut self) -> Vec<String> {
        std::mem::take(&mut self.bus.debug.log)
    }
}
