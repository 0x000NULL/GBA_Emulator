//! Top-level machine: owns the CPU and the bus and runs the main loop.

use crate::bios::Bios;
use crate::bus::bench::BenchCapture;
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
                    // No handler will run (IME or the CPSR I bit masks it): the BIOS's Halt
                    // return path follows the wake-up directly.
                    if self.bus.bios.halt_exit && (!self.bus.irq.ime || self.cpu.regs.irq_disabled()) {
                        self.bus.bios.halt_exit = false;
                        self.bus.sched.now += crate::bios::hle::HALT_EXIT_CYCLES as u64;
                    }
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

    /// The cartridge carries a real-time clock (GPIO).
    pub fn has_rtc(&self) -> bool {
        self.bus.cart.gpio.as_ref().is_some_and(|g| g.rtc.is_some())
    }

    /// The cartridge carries Boktai's solar sensor (GPIO).
    pub fn has_solar(&self) -> bool {
        self.bus.cart.gpio.as_ref().is_some_and(|g| g.solar.is_some())
    }

    /// Seed the cartridge clock with the host's wall-clock time: seconds since 1970-01-01 in local
    /// time. From here the clock advances with emulated time; a time the game set is kept as an
    /// offset. Call after boot and after loading a state. No-op without an RTC.
    pub fn set_rtc_clock(&mut self, local_secs: i64) {
        let now = self.bus.sched.now;
        if let Some(rtc) = self.bus.cart.gpio.as_mut().and_then(|g| g.rtc.as_mut()) {
            rtc.set_clock(local_secs, now);
        }
    }

    /// The clock's current time (seconds since 1970-01-01, local), or None without an RTC.
    pub fn rtc_time(&self) -> Option<i64> {
        let now = self.bus.sched.now;
        self.bus.cart.gpio.as_ref().and_then(|g| g.rtc.as_ref()).map(|r| r.now_secs(now))
    }

    /// Light on the solar sensor, 0 (dark) to `SUN_MAX` (full gauge). No-op without the sensor.
    pub fn set_sun_level(&mut self, level: u8) {
        if let Some(s) = self.bus.cart.gpio.as_mut().and_then(|g| g.solar.as_mut()) {
            s.set_level(level);
        }
    }

    pub fn sun_level(&self) -> Option<u8> {
        self.bus.cart.gpio.as_ref().and_then(|g| g.solar.as_ref()).map(|s| s.level())
    }

    /// True if save memory changed since the last call; clears the flag.
    pub fn save_dirty(&mut self) -> bool {
        std::mem::replace(&mut self.bus.cart.dirty, false)
    }

    /// Attach (`true`) or detach (`false`) the Bench capture: per-layer pictures, sound levels and
    /// the scanline event log. Detaching also clears the layer and channel mutes, so output is
    /// bit-identical to a machine that never had the bench.
    pub fn set_bench(&mut self, on: bool) {
        self.bus.set_bench(on);
    }

    /// The most recently published bench frame, or `None` while the bench is off.
    pub fn bench(&self) -> Option<&BenchCapture> {
        self.bus.debug.bench.as_deref()
    }

    /// Hide layers from the composed picture (bits 0-3 BG0-BG3, bit 4 OBJ). The bench still
    /// captures them. No-op while the bench is off.
    pub fn set_layer_mute(&mut self, mask: u8) {
        if let Some(c) = self.bus.ppu.debug.as_deref_mut() {
            c.mute = mask & 0x1F;
        }
    }

    /// Silence channels (bit `CH_*`: SQ1, SQ2, WAVE, NOISE, FIFO A, FIFO B) in the mix only.
    /// No-op while the bench is off.
    pub fn set_channel_mute(&mut self, mask: u8) {
        if let Some(c) = self.bus.apu.debug.as_deref_mut() {
            c.set_mute(mask);
        }
    }

    pub fn layer_mute(&self) -> u8 {
        self.bus.ppu.debug.as_deref().map_or(0, |c| c.mute)
    }

    pub fn channel_mute(&self) -> u8 {
        self.bus.apu.debug.as_deref().map_or(0, |c| c.mute)
    }

    /// Lines printed by the guest through the mGBA debug registers since the last call.
    pub fn take_debug_log(&mut self) -> Vec<String> {
        std::mem::take(&mut self.bus.debug.log)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Access, MemoryBus};

    /// mov r0,#1; mov r1,#0x02000000; loop: add r0,r0,#1; str r0,[r1]; b loop
    fn test_rom() -> Vec<u8> {
        let mut rom = vec![0u8; 0x4000];
        rom[0xAC..0xB0].copy_from_slice(b"TEST");
        for (i, w) in [0xE3A0_0001u32, 0xE3A0_1402, 0xE280_0001, 0xE581_0000, 0xEAFF_FFFC].iter().enumerate() {
            rom[i * 4..i * 4 + 4].copy_from_slice(&w.to_le_bytes());
        }
        rom
    }

    /// A machine showing BG0 (two colours), two sprites and a sounding square channel.
    fn machine(bench: bool) -> Gba {
        let mut g = Gba::new(test_rom(), Options::default()).unwrap();
        if bench {
            g.set_bench(true);
        }
        let b = &mut g.bus;
        b.write16(0x0500_0000, 0x1234, Access::N); // backdrop
        b.write16(0x0500_0002, 0x001F, Access::N);
        b.write16(0x0500_0004, 0x03E0, Access::N);
        b.write16(0x0500_0200 + 2, 0x7C00, Access::N); // OBJ palette 1
        for i in 0..16u32 {
            b.write16(0x0600_0020 + i * 2, 0x1212, Access::N); // BG tile 1: alternating columns
            b.write16(0x0601_0020 + i * 2, 0x1111, Access::N); // OBJ tile 1
        }
        for i in 0..64u32 {
            b.write16(0x0600_4000 + i * 2, 1, Access::N); // BG0 map: tile 1 across two rows
        }
        for i in 0..128u32 {
            b.write16(0x0700_0000 + i * 8, 0x0200, Access::N); // all OBJs disabled
        }
        for (i, x) in [(0u32, 40u16), (1, 90)] {
            b.write16(0x0700_0000 + i * 8, 20 + i as u16 * 30, Access::N);
            b.write16(0x0700_0002 + i * 8, x, Access::N);
            b.write16(0x0700_0004 + i * 8, 1, Access::N);
        }
        b.write16(0x0400_0008, 8 << 8, Access::N); // BG0CNT screen block 8
        b.write16(0x0400_0000, 1 << 8 | 1 << 12, Access::N);
        b.write8(0x0400_0084, 0x80, Access::N);
        b.write16(0x0400_0080, 0xFF77, Access::N);
        b.write16(0x0400_0082, 0x0002, Access::N);
        b.write8(0x0400_0063, 0xA0, Access::N);
        b.write8(0x0400_0065, 0x80, Access::N);
        g
    }

    #[test]
    fn run_frame_with_bench_is_bit_identical() {
        let mut plain = machine(false);
        let mut benched = machine(true);
        assert!(plain.bench().is_none());
        let (mut audio_a, mut audio_b) = (Vec::new(), Vec::new());
        for frame in 1..=10u64 {
            assert_eq!(plain.run_frame(), StopReason::Done);
            assert_eq!(benched.run_frame(), StopReason::Done);
            assert!(plain.framebuffer()[..] == benched.framebuffer()[..], "frame {frame}");
            assert_eq!(benched.bench().unwrap().frame, frame);
            plain.drain_audio(&mut audio_a);
            benched.drain_audio(&mut audio_b);
        }
        assert_eq!(plain.cpu.state_hash(), benched.cpu.state_hash());
        assert_eq!(plain.bus.sched.now, benched.bus.sched.now);
        assert_eq!(audio_a, audio_b);
        assert!(!audio_a.is_empty() && audio_a.iter().any(|&s| s != [0, 0]), "the comparison covers real audio");
        let bench = benched.bench().unwrap();
        assert_eq!(bench.frame, benched.bus.ppu.frame_count);
        assert_eq!(bench.layers.obj_drawn, 2);
        assert_eq!(bench.sound.peak[crate::apu::capture::CH_SQ1], 10);
    }

    #[test]
    fn mutes_need_the_bench_and_change_only_the_output() {
        let mut g = machine(false);
        g.set_layer_mute(0x1F);
        g.set_channel_mute(0x3F);
        assert_eq!((g.layer_mute(), g.channel_mute()), (0, 0), "no-ops while the bench is off");

        let mut plain = machine(true);
        let mut muted = machine(true);
        muted.set_layer_mute(0x01);
        muted.set_channel_mute(1 << crate::apu::capture::CH_SQ1);
        assert_eq!((muted.layer_mute(), muted.channel_mute()), (0x01, 0x01));
        // The HLE boot starts mid-frame (line 126), so the first frame is partial: run two.
        for _ in 0..2 {
            plain.run_frame();
            muted.run_frame();
        }
        assert!(plain.framebuffer()[..] != muted.framebuffer()[..], "BG0 is hidden");
        assert!(plain.bench().unwrap().layers.planes[0][..] == muted.bench().unwrap().layers.planes[0][..], "but captured");
        assert!(plain.bench().unwrap().layers.planes[0].contains(&0x001F), "and really there");
        assert_eq!(plain.cpu.state_hash(), muted.cpu.state_hash(), "the guest cannot tell");
        muted.set_bench(false);
        assert_eq!((muted.layer_mute(), muted.channel_mute()), (0, 0));
    }
}
