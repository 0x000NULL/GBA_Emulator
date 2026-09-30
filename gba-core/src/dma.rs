//! DMA0-3 registers and trigger logic. Transfer execution lives in `bus/dma_exec.rs` because it
//! needs the whole bus. Source: GBATEK "GBA DMA Transfers".

use crate::sched::{Event, Scheduler};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DmaTrigger {
    /// Start timing 0: as soon as enabled (2-cycle latency via `Event::DmaStart`).
    Immediate(u8),
    VBlank,
    HBlank,
    /// DMA1/2 in "special" timing: sound FIFO A refill request.
    FifoA,
    FifoB,
    /// DMA3 in "special" timing: video capture, H-Blank of lines 2..=162.
    VideoCapture,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct DmaChannel {
    /// Register values as written.
    pub sad: u32,
    pub dad: u32,
    /// Word count register (CNT_L); 0 means max (0x4000, or 0x10000 for DMA3).
    pub count: u16,
    pub cnt_h: u16,
    /// Internal latched source/destination and remaining count, reloaded per GBATEK rules.
    pub src: u32,
    pub dst: u32,
    pub remaining: u32,
}

impl DmaChannel {
    #[inline]
    pub fn enabled(&self) -> bool {
        self.cnt_h & 0x8000 != 0
    }
    #[inline]
    pub fn dst_ctrl(&self) -> u16 {
        (self.cnt_h >> 5) & 3
    }
    #[inline]
    pub fn src_ctrl(&self) -> u16 {
        (self.cnt_h >> 7) & 3
    }
    #[inline]
    pub fn repeat(&self) -> bool {
        self.cnt_h & 0x0200 != 0
    }
    #[inline]
    pub fn word32(&self) -> bool {
        self.cnt_h & 0x0400 != 0
    }
    /// 0 immediate, 1 vblank, 2 hblank, 3 special.
    #[inline]
    pub fn timing(&self) -> u16 {
        (self.cnt_h >> 12) & 3
    }
    #[inline]
    pub fn irq_on_end(&self) -> bool {
        self.cnt_h & 0x4000 != 0
    }
    /// Game-pak DRQ bit (DMA3 only); ignored.
    #[inline]
    pub fn gamepak_drq(&self) -> bool {
        self.cnt_h & 0x0800 != 0
    }
}

#[derive(Clone, Debug, Default)]
pub struct DmaController {
    pub ch: [DmaChannel; 4],
    /// Bitmask of channels with a transfer requested and not yet executed.
    pub pending: u8,
}

impl DmaController {
    pub fn new() -> Self {
        Self::default()
    }

    fn count_max(ch: usize) -> u32 {
        if ch == 3 {
            0x1_0000
        } else {
            0x4000
        }
    }

    /// Latch a channel's internal registers on enable (GBATEK: SAD/DAD/count are reloaded when
    /// the enable bit goes 0->1).
    fn latch(&mut self, ch: usize) {
        let src_mask = if ch == 0 { 0x07FF_FFFF } else { 0x0FFF_FFFF };
        let dst_mask = if ch == 3 { 0x0FFF_FFFF } else { 0x07FF_FFFF };
        let c = &mut self.ch[ch];
        c.src = c.sad & src_mask;
        c.dst = c.dad & dst_mask;
        c.remaining = if c.count == 0 { Self::count_max(ch) } else { c.count as u32 };
    }

    /// Reload the count (and destination if dst_ctrl == 3) for a repeating transfer.
    pub(crate) fn reload_repeat(&mut self, ch: usize) {
        let dst_mask = if ch == 3 { 0x0FFF_FFFF } else { 0x07FF_FFFF };
        let c = &mut self.ch[ch];
        c.remaining = if c.count == 0 { Self::count_max(ch) } else { c.count as u32 };
        if c.dst_ctrl() == 3 {
            c.dst = c.dad & dst_mask;
        }
    }

    pub fn read8(&self, off: u16) -> u8 {
        if !(0xB0..0xE0).contains(&off) {
            return 0;
        }
        let ch = ((off - 0xB0) / 12) as usize;
        let r = (off - 0xB0) % 12;
        let c = &self.ch[ch];
        match r {
            // SAD/DAD/CNT_L are write-only (read as 0).
            0..=9 => 0,
            10 => c.cnt_h as u8,
            _ => (c.cnt_h >> 8) as u8,
        }
    }

    pub fn write8(&mut self, off: u16, v: u8, sched: &mut Scheduler) {
        if !(0xB0..0xE0).contains(&off) {
            return;
        }
        let ch = ((off - 0xB0) / 12) as usize;
        let r = (off - 0xB0) % 12;
        let c = &mut self.ch[ch];
        let set_byte = |reg: &mut u32, i: u32, v: u8| {
            let sh = i * 8;
            *reg = (*reg & !(0xFF << sh)) | ((v as u32) << sh);
        };
        match r {
            0..=3 => set_byte(&mut c.sad, r as u32, v),
            4..=7 => set_byte(&mut c.dad, (r - 4) as u32, v),
            8 => c.count = (c.count & 0xFF00) | v as u16,
            9 => c.count = (c.count & 0x00FF) | ((v as u16) << 8),
            10 => c.cnt_h = (c.cnt_h & 0xFF00) | (v as u16 & 0xE0),
            _ => {
                let was_enabled = c.enabled();
                let mask: u16 = if ch == 3 { 0xFF } else { 0xF7 };
                c.cnt_h = (c.cnt_h & 0x00FF) | (((v & mask as u8) as u16) << 8);
                let now_enabled = c.enabled();
                if !was_enabled && now_enabled {
                    self.latch(ch);
                    if self.ch[ch].timing() == 0 {
                        sched.schedule_in(Event::DmaStart(ch as u8), 2);
                    }
                } else if was_enabled && !now_enabled {
                    self.pending &= !(1 << ch);
                    sched.cancel(Event::DmaStart(ch as u8));
                }
            }
        }
    }

    /// Immediate-mode start latency elapsed.
    pub fn on_start_event(&mut self, ch: u8) {
        let ch = ch as usize;
        if self.ch[ch].enabled() && self.ch[ch].timing() == 0 {
            self.pending |= 1 << ch;
        }
    }

    /// Mark every enabled channel whose start timing matches `t` as pending.
    pub fn trigger(&mut self, t: DmaTrigger) {
        for ch in 0..4 {
            let c = &self.ch[ch];
            if !c.enabled() {
                continue;
            }
            let hit = match (t, c.timing()) {
                (DmaTrigger::Immediate(n), 0) => n as usize == ch,
                (DmaTrigger::VBlank, 1) => true,
                (DmaTrigger::HBlank, 2) => true,
                (DmaTrigger::FifoA, 3) => ch == 1 || ch == 2,
                (DmaTrigger::FifoB, 3) => ch == 1 || ch == 2,
                (DmaTrigger::VideoCapture, 3) => ch == 3,
                _ => false,
            };
            // FIFO DMAs only serve the FIFO their destination points at.
            let hit = match t {
                DmaTrigger::FifoA => hit && c.dst == 0x0400_00A0,
                DmaTrigger::FifoB => hit && c.dst == 0x0400_00A4,
                _ => hit,
            };
            if hit {
                self.pending |= 1 << ch;
            }
        }
    }

    /// True when a channel is a sound-FIFO DMA (DMA1/2, special timing, destination FIFO A/B).
    pub fn is_fifo(&self, ch: usize) -> bool {
        let c = &self.ch[ch];
        (ch == 1 || ch == 2) && c.timing() == 3 && (c.dst == 0x0400_00A0 || c.dst == 0x0400_00A4)
    }
}
