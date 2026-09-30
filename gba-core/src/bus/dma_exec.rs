//! DMA transfer execution. Runs to completion between CPU instructions; the CPU is stalled.
//! Source: GBATEK "GBA DMA Transfers" (timing: 2N + 2(n-1)S + xI; x = 2, or 4 when both
//! source and destination are in the game pak).

use super::{Access, Bus, MemoryBus};
use crate::irq::Irq;

impl Bus {
    /// Execute every pending channel, lowest number (highest priority) first.
    pub fn dma_run(&mut self) {
        while self.dma.pending != 0 {
            let ch = self.dma.pending.trailing_zeros() as usize;
            self.dma.pending &= !(1 << ch);
            if !self.dma.ch[ch].enabled() {
                continue;
            }
            self.dma_transfer(ch);
        }
    }

    fn dma_transfer(&mut self, ch: usize) {
        let fifo = self.dma.is_fifo(ch);
        let c = self.dma.ch[ch];
        // FIFO mode: always 4 words of 32 bits, destination fixed.
        let word32 = fifo || c.word32();
        let step: u32 = if word32 { 4 } else { 2 };
        let count = if fifo { 4 } else { c.remaining };
        let src_ctrl = c.src_ctrl();
        let dst_ctrl = if fifo { 2 } else { c.dst_ctrl() };
        let mut src = c.src & if word32 { !3 } else { !1 };
        let mut dst = c.dst & if word32 { !3 } else { !1 };

        self.in_dma = true;
        let both_pak = (0x08..=0x0D).contains(&(src >> 24)) && (0x08..=0x0D).contains(&(dst >> 24));
        self.idle(if both_pak { 4 } else { 2 });

        for i in 0..count {
            let access = if i == 0 { Access::N } else { Access::S };
            if word32 {
                let v = self.read32(src, access);
                self.write32(dst, v, access);
                self.open_bus = v;
            } else {
                let v = self.read16(src, access);
                self.write16(dst, v, access);
                self.open_bus = v as u32 | (v as u32) << 16;
            }
            src = match src_ctrl {
                0 => src.wrapping_add(step),
                1 => src.wrapping_sub(step),
                _ => src,
            };
            dst = match dst_ctrl {
                0 | 3 => dst.wrapping_add(step),
                1 => dst.wrapping_sub(step),
                _ => dst,
            };
        }
        self.in_dma = false;

        let (irq_on_end, repeat_on) = {
            let c = &mut self.dma.ch[ch];
            c.src = src;
            c.dst = dst;
            c.remaining = 0;
            (c.irq_on_end(), c.repeat() && c.timing() != 0)
        };
        if irq_on_end {
            self.irq.raise(Irq::dma(ch as u8));
        }
        if repeat_on {
            self.dma.reload_repeat(ch);
        } else {
            self.dma.ch[ch].cnt_h &= !0x8000;
        }
    }
}
