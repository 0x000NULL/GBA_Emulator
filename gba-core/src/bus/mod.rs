//! System bus: memory map, waitstates, open bus, IO dispatch, event dispatch.
//! Source: GBATEK "GBA Memory Map", "GBA System Control" (WAITCNT), "Unpredictable Things".

pub mod debug;
pub mod dma_exec;
pub mod io;
pub mod prefetch;
#[cfg(test)]
mod prefetch_tests;
#[cfg(test)]
mod tests;

use crate::apu::Apu;
use crate::bios::Bios;
use crate::cart::Cartridge;
use crate::cpu::Regs;
use crate::dma::{DmaController, DmaTrigger};
use crate::irq::IrqController;
use crate::keypad::Keypad;
use crate::ppu::Ppu;
use crate::sched::{Event, Scheduler};
use crate::sio::Sio;
use crate::timers::Timers;
use debug::DebugHooks;
use prefetch::Prefetch;

/// Sequential vs non-sequential access (selects the S/N ROM waitstate).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Access {
    N,
    S,
}

/// What the CPU needs from the outside world. `Bus` implements it; CPU unit tests use a flat
/// test bus. Every access advances the implementor's clock by its waitstate cost.
pub trait MemoryBus {
    fn read8(&mut self, addr: u32, a: Access) -> u8;
    fn read16(&mut self, addr: u32, a: Access) -> u16;
    fn read32(&mut self, addr: u32, a: Access) -> u32;
    fn write8(&mut self, addr: u32, v: u8, a: Access);
    fn write16(&mut self, addr: u32, v: u16, a: Access);
    fn write32(&mut self, addr: u32, v: u32, a: Access);
    /// Opcode fetch (Thumb). May use the prefetch buffer; also updates open-bus state.
    fn fetch16(&mut self, addr: u32, a: Access) -> u16;
    /// Opcode fetch (ARM).
    fn fetch32(&mut self, addr: u32, a: Access) -> u32;
    /// Internal cycles (multiplies, register-specified shifts, LDM/STM internal cycle).
    fn idle(&mut self, cycles: u32);
    /// IME && (IE & IF) != 0.
    fn irq_pending(&self) -> bool;
    fn now(&self) -> u64;
    /// True when the BIOS is high-level emulated (no BIOS image).
    fn is_hle_bios(&self) -> bool {
        false
    }
    /// Service `swi num` in HLE. Returns false if the CPU must take the real SWI exception.
    fn swi_hle(&mut self, _num: u8, _regs: &mut Regs) -> bool {
        false
    }
    /// `Some(seq)` (once) when the last HLE SWI charged the real handler's cost, including its
    /// return through `movs pc, lr`: the CPU must then refill the pipeline at the return address,
    /// starting with a sequential fetch when `seq`.
    fn swi_took_refill(&mut self) -> Option<bool> {
        None
    }
    /// Emulate BIOS code at `regs.r[15]` (< 0x4000) in HLE. Returns false if nothing handled it.
    fn bios_hle_vector(&mut self, _regs: &mut Regs) -> bool {
        false
    }
}

/// Siblings a subsystem event handler may touch. Never hand out `&mut Bus`.
pub struct SysCtx<'a> {
    pub sched: &'a mut Scheduler,
    pub irq: &'a mut IrqController,
    pub dma: &'a mut DmaController,
}

pub const EWRAM_SIZE: usize = 0x4_0000;
pub const IWRAM_SIZE: usize = 0x8000;

#[cfg_attr(feature = "savestate", derive(serde::Serialize, serde::Deserialize))]
pub struct Bus {
    pub sched: Scheduler,
    pub irq: IrqController,
    pub ppu: Ppu,
    pub apu: Apu,
    pub dma: DmaController,
    pub timers: Timers,
    pub keypad: Keypad,
    pub cart: Cartridge,
    pub bios: Bios,
    #[cfg_attr(feature = "savestate", serde(with = "crate::state::bytes_box"))]
    pub ewram: Box<[u8; EWRAM_SIZE]>,
    #[cfg_attr(feature = "savestate", serde(with = "crate::state::bytes_box"))]
    pub iwram: Box<[u8; IWRAM_SIZE]>,
    /// 4000204h WAITCNT
    pub waitcnt: u16,
    /// Game pak prefetch buffer (WAITCNT bit 14).
    pub pf: Prefetch,
    /// 4000300h POSTFLG
    pub postflg: u8,
    /// Serial/RCNT/JOY registers (0x120..0x15F). No link partner is emulated.
    pub sio: Sio,
    /// Undocumented 4000800h internal memory control.
    pub imc: u32,
    /// Last value seen on the bus (returned for unmapped reads).
    pub open_bus: u32,
    /// Address of the most recent opcode fetch (BIOS reads are only allowed while executing in BIOS).
    pub last_fetch: u32,
    #[cfg_attr(feature = "savestate", serde(skip))]
    pub debug: DebugHooks,
    /// Waitstates for a non-sequential 16-bit ROM access, per ROM region 0/1/2.
    rom_n: [u32; 3],
    /// Waitstates for a sequential 16-bit ROM access, per ROM region.
    rom_s: [u32; 3],
    /// SRAM waitstates (8-bit bus).
    sram_wait: u32,
    /// True while a DMA transfer is executing (DMA accesses to ROM are sequential after the first).
    pub(crate) in_dma: bool,
}

impl Bus {
    pub fn new(cart: Cartridge, bios: Bios) -> Self {
        let mut b = Bus {
            sched: Scheduler::new(),
            irq: IrqController::new(),
            ppu: Ppu::new(),
            apu: Apu::new(),
            dma: DmaController::new(),
            timers: Timers::new(),
            keypad: Keypad::new(),
            cart,
            bios,
            ewram: vec![0u8; EWRAM_SIZE].into_boxed_slice().try_into().unwrap(),
            iwram: vec![0u8; IWRAM_SIZE].into_boxed_slice().try_into().unwrap(),
            waitcnt: 0,
            pf: Prefetch::default(),
            postflg: 0,
            sio: Sio::new(),
            imc: 0x0D00_0020,
            open_bus: 0,
            last_fetch: 0,
            debug: DebugHooks::default(),
            rom_n: [4, 4, 4],
            rom_s: [2, 4, 8],
            sram_wait: 4,
            in_dma: false,
        };
        b.update_waitstates();
        b
    }

    /// Borrow the scheduler/irq/dma trio for a subsystem callback.
    #[inline]
    pub fn ctx(&mut self) -> SysCtx<'_> {
        SysCtx { sched: &mut self.sched, irq: &mut self.irq, dma: &mut self.dma }
    }

    /// Recompute waitstate tables from WAITCNT. Source: GBATEK "GBA System Control".
    pub fn update_waitstates(&mut self) {
        const N: [u32; 4] = [4, 3, 2, 8];
        let w = self.waitcnt as u32;
        self.pf.enabled = w & (1 << 14) != 0;
        if !self.pf.enabled {
            self.pf.invalidate();
        }
        self.sram_wait = N[(w & 3) as usize];
        self.rom_n[0] = N[((w >> 2) & 3) as usize];
        self.rom_s[0] = if w & (1 << 4) != 0 { 1 } else { 2 };
        self.rom_n[1] = N[((w >> 5) & 3) as usize];
        self.rom_s[1] = if w & (1 << 7) != 0 { 1 } else { 4 };
        self.rom_n[2] = N[((w >> 8) & 3) as usize];
        self.rom_s[2] = if w & (1 << 10) != 0 { 1 } else { 8 };
    }

    /// Cycles for one access of `width` bytes at `addr`. Every region's first cycle is included.
    #[inline]
    fn wait(&self, addr: u32, width: u32, a: Access) -> u32 {
        match addr >> 24 {
            0x02 => {
                // 16-bit bus, 2 waitstates: 3 cycles per halfword.
                if width == 4 {
                    6
                } else {
                    3
                }
            }
            0x05 | 0x06 => {
                // 16-bit bus, no waitstates: 1 cycle per halfword.
                if width == 4 {
                    2
                } else {
                    1
                }
            }
            0x08..=0x0D => {
                let region = (((addr >> 24) - 8) / 2) as usize;
                let first = match a {
                    Access::S => self.rom_s[region],
                    Access::N => self.rom_n[region],
                } + 1;
                if width == 4 {
                    first + self.rom_s[region] + 1
                } else {
                    first
                }
            }
            0x0E | 0x0F => self.sram_wait + 1,
            _ => 1,
        }
    }

    /// Opcode fetch cost: through the prefetch buffer for game pak ROM when it is enabled; a fetch
    /// from anywhere else drops whatever the buffer holds.
    #[inline]
    fn fetch_cost(&mut self, addr: u32, width: u32, a: Access) -> u32 {
        if self.pf.enabled && (0x08..=0x0D).contains(&(addr >> 24)) {
            self.pf_fetch_cost(addr, width, a)
        } else {
            if self.pf.is_live() {
                self.pf.invalidate();
            }
            self.wait(addr, width, a)
        }
    }

    #[inline]
    fn tick(&mut self, cycles: u32) {
        self.sched.now += cycles as u64;
    }

    /// Bus arbitration for a CPU data access. An immediate DMA whose 2-cycle start latency
    /// (GBATEK "DMA Start Timing") elapsed on an earlier cycle owns the bus first: the CPU's
    /// access completes after the transfer and sees its result. Without this, the instruction
    /// that follows the enable write would read the destination before the (event-driven) DMA
    /// runs, because the CPU is stepped one whole instruction at a time (mgba-suite memory tests
    /// "DMAn 16/32" stores). An access on the very cycle the latency ends still belongs to the
    /// CPU: with code in IWRAM the timer read right after the enable write lands there, and the
    /// mgba-suite timing tests ("Trivial DMA", IWRAM column) expect it to precede the transfer.
    /// Opcode fetches are not arbitrated: they cannot observe the transfer and the total cycle
    /// count is the same either way.
    #[inline]
    fn arbitrate(&mut self) {
        if self.sched.now > self.dma.next_start && !self.in_dma {
            let Bus { sched, dma, .. } = self;
            if dma.fire_due_starts(sched) && dma.pending != 0 {
                self.dma_run();
            }
        }
    }

    // ---------------------------------------------------------------- raw region access

    #[inline]
    fn vram_index(addr: u32) -> usize {
        let mut a = (addr & 0x1_FFFF) as usize;
        if a >= 0x1_8000 {
            a -= 0x8000;
        }
        a
    }

    /// Read a byte without side effects or timing. Used by the debugger and by the composed
    /// 16/32-bit reads.
    pub fn peek8(&self, addr: u32) -> u8 {
        match addr >> 24 {
            0x00 => {
                if addr < 0x4000 {
                    let w = self.bios.read32(addr & !3, self.last_fetch < 0x4000);
                    (w >> ((addr & 3) * 8)) as u8
                } else {
                    (self.open_bus >> ((addr & 3) * 8)) as u8
                }
            }
            0x02 => self.ewram[(addr & 0x3_FFFF) as usize],
            0x03 => self.iwram[(addr & 0x7FFF) as usize],
            0x04 => self.io_peek8(addr),
            0x05 => self.ppu.pal[(addr & 0x3FF) as usize],
            0x06 => self.ppu.vram[Self::vram_index(addr)],
            0x07 => self.ppu.oam[(addr & 0x3FF) as usize],
            0x08..=0x0D => self.cart.peek8(addr),
            0x0E | 0x0F => self.cart.save_peek8(addr),
            _ => (self.open_bus >> ((addr & 3) * 8)) as u8,
        }
    }

    pub fn peek16(&self, addr: u32) -> u16 {
        let a = addr & !1;
        self.peek8(a) as u16 | (self.peek8(a + 1) as u16) << 8
    }

    pub fn peek32(&self, addr: u32) -> u32 {
        let a = addr & !3;
        self.peek16(a) as u32 | (self.peek16(a + 2) as u32) << 16
    }

    fn read8_inner(&mut self, addr: u32) -> u8 {
        match addr >> 24 {
            0x04 => self.io_read8(addr),
            0x08..=0x0D => self.cart.read8(addr),
            0x0E | 0x0F => self.cart.save_read8(addr),
            _ => self.peek8(addr),
        }
    }

    fn read16_inner(&mut self, addr: u32) -> u16 {
        let a = addr & !1;
        match a >> 24 {
            0x04 => self.io_read16(a),
            0x08..=0x0D => self.cart.read16(a),
            0x0E | 0x0F => {
                // 8-bit bus: the byte the (unaligned) address selects, replicated on both lanes;
                // the CPU then applies its usual misaligned rotation (mgba-suite "SRAM load").
                let b = self.cart.save_read8(addr) as u16;
                b | b << 8
            }
            _ => self.peek16(a),
        }
    }

    fn read32_inner(&mut self, addr: u32) -> u32 {
        let a = addr & !3;
        match a >> 24 {
            0x04 => self.io_read32(a),
            0x08..=0x0D => self.cart.read32(a),
            0x0E | 0x0F => {
                let b = self.cart.save_read8(addr) as u32;
                b * 0x0101_0101
            }
            _ => self.peek32(a),
        }
    }

    fn write8_inner(&mut self, addr: u32, v: u8) {
        match addr >> 24 {
            0x02 => self.ewram[(addr & 0x3_FFFF) as usize] = v,
            0x03 => self.iwram[(addr & 0x7FFF) as usize] = v,
            0x04 => self.io_write8(addr, v),
            0x05 => {
                // 8-bit writes to palette are duplicated into the halfword.
                let i = (addr & 0x3FE) as usize;
                self.ppu.pal[i] = v;
                self.ppu.pal[i + 1] = v;
            }
            0x06 => {
                // 8-bit writes to BG VRAM duplicate into the halfword; writes to OBJ VRAM are ignored.
                let i = Self::vram_index(addr);
                let obj_start = if self.ppu.dispcnt() & 7 >= 3 { 0x1_4000 } else { 0x1_0000 };
                if i < obj_start {
                    let i = i & !1;
                    self.ppu.vram[i] = v;
                    self.ppu.vram[i + 1] = v;
                }
            }
            0x07 => {} // 8-bit OAM writes are ignored.
            0x08..=0x0D => self.cart.write8(addr, v),
            0x0E | 0x0F => self.cart.save_write8(addr, v),
            _ => {}
        }
    }

    fn write16_inner(&mut self, addr: u32, v: u16) {
        let a = addr & !1;
        match a >> 24 {
            0x02 => {
                let i = (a & 0x3_FFFF) as usize;
                self.ewram[i..i + 2].copy_from_slice(&v.to_le_bytes());
            }
            0x03 => {
                let i = (a & 0x7FFF) as usize;
                self.iwram[i..i + 2].copy_from_slice(&v.to_le_bytes());
            }
            0x04 => self.io_write16(a, v),
            0x05 => {
                let i = (a & 0x3FF) as usize;
                self.ppu.pal[i..i + 2].copy_from_slice(&v.to_le_bytes());
            }
            0x06 => {
                let i = Self::vram_index(a);
                self.ppu.vram[i..i + 2].copy_from_slice(&v.to_le_bytes());
            }
            0x07 => {
                let i = (a & 0x3FF) as usize;
                self.ppu.oam[i..i + 2].copy_from_slice(&v.to_le_bytes());
            }
            0x08..=0x0D => self.cart.write16(a, v),
            0x0E | 0x0F => {
                // 8-bit bus: the lane selected by the address is written.
                let b = if addr & 1 != 0 { (v >> 8) as u8 } else { v as u8 };
                self.cart.save_write8(addr, b);
            }
            _ => {}
        }
    }

    fn write32_inner(&mut self, addr: u32, v: u32) {
        let a = addr & !3;
        match a >> 24 {
            0x02 => {
                let i = (a & 0x3_FFFF) as usize;
                self.ewram[i..i + 4].copy_from_slice(&v.to_le_bytes());
            }
            0x03 => {
                let i = (a & 0x7FFF) as usize;
                self.iwram[i..i + 4].copy_from_slice(&v.to_le_bytes());
            }
            0x04 => self.io_write32(a, v),
            0x05 => {
                let i = (a & 0x3FF) as usize;
                self.ppu.pal[i..i + 4].copy_from_slice(&v.to_le_bytes());
            }
            0x06 => {
                let i = Self::vram_index(a);
                self.ppu.vram[i..i + 4].copy_from_slice(&v.to_le_bytes());
            }
            0x07 => {
                let i = (a & 0x3FF) as usize;
                self.ppu.oam[i..i + 4].copy_from_slice(&v.to_le_bytes());
            }
            0x08..=0x0D => {
                self.cart.write16(a, v as u16);
                self.cart.write16(a + 2, (v >> 16) as u16);
            }
            0x0E | 0x0F => {
                let b = (v >> ((addr & 3) * 8)) as u8;
                self.cart.save_write8(addr, b);
            }
            _ => {}
        }
    }

    // ---------------------------------------------------------------- events

    /// Dispatch every event whose time has come.
    pub fn process_events(&mut self) {
        while let Some((ev, at)) = self.sched.pop_due() {
            match ev {
                Event::HBlank => {
                    let Bus { sched, irq, dma, ppu, .. } = self;
                    ppu.on_hblank(&mut SysCtx { sched, irq, dma });
                }
                Event::LineEnd => {
                    let Bus { sched, irq, dma, ppu, .. } = self;
                    ppu.on_line_end(&mut SysCtx { sched, irq, dma }, at);
                }
                Event::TimerOverflow(t) => {
                    let Bus { sched, irq, timers, apu, dma, .. } = self;
                    timers.on_overflow(t, sched, irq);
                    let req = apu.on_timer_overflow(t);
                    if req.a {
                        dma.trigger(DmaTrigger::FifoA);
                    }
                    if req.b {
                        dma.trigger(DmaTrigger::FifoB);
                    }
                }
                Event::DmaStart(ch) => self.dma.on_start_event(ch),
                Event::ApuSample => {
                    self.apu.on_sample();
                    self.sched.schedule(Event::ApuSample, at + self.apu.cycles_per_sample());
                }
                Event::ApuFrameSeq => {
                    self.apu.on_frame_seq();
                    self.sched.schedule(Event::ApuFrameSeq, at + crate::timing::CYCLES_PER_FRAME_SEQ);
                }
                Event::KeypadPoll => self.keypad.check_irq(&mut self.irq),
                Event::IrqLatch => {}
                Event::SioComplete => self.sio.on_complete(&mut self.irq),
            }
        }
    }
}

impl MemoryBus for Bus {
    #[inline]
    fn read8(&mut self, addr: u32, a: Access) -> u8 {
        let c = if self.pf.is_live() && (0x08..=0x0F).contains(&(addr >> 24)) {
            self.pf_data_cost(addr, 1, a)
        } else {
            self.wait(addr, 1, a)
        };
        self.tick(c);
        self.arbitrate();
        let v = self.read8_inner(addr);
        if self.debug.enabled {
            self.debug.check_read(addr, 1);
        }
        v
    }

    #[inline]
    fn read16(&mut self, addr: u32, a: Access) -> u16 {
        let c = if self.pf.is_live() && (0x08..=0x0F).contains(&(addr >> 24)) {
            self.pf_data_cost(addr, 2, a)
        } else {
            self.wait(addr, 2, a)
        };
        self.tick(c);
        self.arbitrate();
        let v = self.read16_inner(addr);
        if self.debug.enabled {
            self.debug.check_read(addr, 2);
        }
        v
    }

    #[inline]
    fn read32(&mut self, addr: u32, a: Access) -> u32 {
        let c = if self.pf.is_live() && (0x08..=0x0F).contains(&(addr >> 24)) {
            self.pf_data_cost(addr, 4, a)
        } else {
            self.wait(addr, 4, a)
        };
        self.tick(c);
        self.arbitrate();
        let v = self.read32_inner(addr);
        if self.debug.enabled {
            self.debug.check_read(addr, 4);
        }
        v
    }

    #[inline]
    fn write8(&mut self, addr: u32, v: u8, a: Access) {
        let c = if self.pf.is_live() && (0x08..=0x0F).contains(&(addr >> 24)) {
            self.pf_data_cost(addr, 1, a)
        } else {
            self.wait(addr, 1, a)
        };
        self.tick(c);
        self.arbitrate();
        if self.debug.enabled {
            self.debug.check_write(addr, 1, v as u32);
        }
        self.write8_inner(addr, v);
    }

    #[inline]
    fn write16(&mut self, addr: u32, v: u16, a: Access) {
        let c = if self.pf.is_live() && (0x08..=0x0F).contains(&(addr >> 24)) {
            self.pf_data_cost(addr, 2, a)
        } else {
            self.wait(addr, 2, a)
        };
        self.tick(c);
        self.arbitrate();
        if self.debug.enabled {
            self.debug.check_write(addr, 2, v as u32);
        }
        self.write16_inner(addr, v);
    }

    #[inline]
    fn write32(&mut self, addr: u32, v: u32, a: Access) {
        let c = if self.pf.is_live() && (0x08..=0x0F).contains(&(addr >> 24)) {
            self.pf_data_cost(addr, 4, a)
        } else {
            self.wait(addr, 4, a)
        };
        self.tick(c);
        self.arbitrate();
        if self.debug.enabled {
            self.debug.check_write(addr, 4, v);
        }
        self.write32_inner(addr, v);
    }

    #[inline]
    fn fetch16(&mut self, addr: u32, a: Access) -> u16 {
        let c = self.fetch_cost(addr, 2, a);
        self.tick(c);
        self.last_fetch = addr;
        let v = self.read16_inner(addr);
        // Open bus after a Thumb fetch: the halfword replicated (GBATEK "Unpredictable Things";
        // exact per-region rules for IWRAM/others are not modelled).
        self.open_bus = v as u32 | (v as u32) << 16;
        v
    }

    #[inline]
    fn fetch32(&mut self, addr: u32, a: Access) -> u32 {
        let c = self.fetch_cost(addr, 4, a);
        self.tick(c);
        self.last_fetch = addr;
        let v = self.read32_inner(addr);
        self.open_bus = v;
        v
    }

    #[inline]
    fn idle(&mut self, cycles: u32) {
        self.tick(cycles);
    }

    #[inline]
    fn irq_pending(&self) -> bool {
        self.irq.pending()
    }

    #[inline]
    fn now(&self) -> u64 {
        self.sched.now
    }

    fn is_hle_bios(&self) -> bool {
        self.bios.is_hle()
    }

    fn swi_hle(&mut self, num: u8, regs: &mut Regs) -> bool {
        if !self.bios.is_hle() {
            return false;
        }
        self.hle_swi(num, regs)
    }

    fn swi_took_refill(&mut self) -> Option<bool> {
        // The handler's last ROM access is the `ldrb` of the SWI comment, one halfword before the
        // return address. The cartridge sees that as sequential, except (empirically, mgba-suite
        // "Timing tests" P columns) when the prefetch buffer is enabled, which makes the first
        // refill fetch non-sequential again.
        std::mem::take(&mut self.bios.swi_refill).then_some(!self.pf.enabled)
    }

    fn bios_hle_vector(&mut self, regs: &mut Regs) -> bool {
        self.hle_vector(regs)
    }
}
