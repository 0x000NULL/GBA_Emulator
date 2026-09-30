//! DMA transfer execution. Runs between CPU instructions; the CPU is stalled while any channel
//! is active. Source: GBATEK "GBA DMA Transfers" (timing: 2N + 2(n-1)S + xI; x = 2, or 4 when
//! both source and destination are in the game pak; "DMA channels of higher priority may
//! offhold sound DMA" i.e. a running low-priority transfer is paused to serve a higher one).
//!
//! Transfers are resumable: after every unit the scheduler is checked, due events fire (so
//! timers keep clocking the sound FIFOs and the PPU keeps rendering during a long copy), and if a
//! higher-priority channel became pending this one saves its progress and yields.
//!
//! Data-path quirks pinned by the mgba-suite memory/DMA tests: a read from the BIOS/unused area
//! (which is where DMA0's 27-bit source address puts the game pak) returns the channel's own
//! latch rather than the CPU open bus, and a game-pak source advances after every unit whatever
//! the source control says. Immediate starts may also pre-empt a CPU data access
//! (`Bus::arbitrate`).

use super::{Access, Bus, MemoryBus};
use crate::irq::Irq;

impl Bus {
    /// Execute pending channels, lowest number (highest priority) first, until none remain.
    pub fn dma_run(&mut self) {
        while self.dma.pending != 0 {
            let ch = self.dma.pending.trailing_zeros() as usize;
            self.dma.pending &= !(1 << ch);
            if !self.dma.ch[ch].enabled() {
                self.dma.ch[ch].in_progress = false;
                continue;
            }
            self.dma_transfer(ch);
        }
    }

    fn dma_transfer(&mut self, ch: usize) {
        let fifo = self.dma.is_fifo(ch);
        let word32 = fifo || self.dma.ch[ch].word32();
        let step: u32 = if word32 { 4 } else { 2 };
        let src_ctrl = self.dma.ch[ch].src_ctrl();
        let dst_ctrl = if fifo { 2 } else { self.dma.ch[ch].dst_ctrl() };

        if !self.dma.ch[ch].in_progress {
            let c = &mut self.dma.ch[ch];
            c.in_progress = true;
            if fifo {
                c.remaining = 4;
            }
            c.src &= if word32 { !3 } else { !1 };
            c.dst &= if word32 { !3 } else { !1 };
            let (src, dst, count) = (c.src, c.dst, c.remaining);
            // A DMA into the EEPROM region is a serial request; its length reveals the EEPROM size.
            if dst >> 24 == 0x0D && !word32 {
                self.cart.eeprom_dma_hint(count);
            }
            let both_pak = (0x08..=0x0D).contains(&(src >> 24)) && (0x08..=0x0D).contains(&(dst >> 24));
            self.idle(if both_pak { 4 } else { 2 });
        }

        let c = self.dma.ch[ch];
        let mut src = c.src;
        let mut dst = c.dst;
        let mut remaining = c.remaining;
        let mut latch = c.latch;
        let higher = (1u8 << ch) - 1;
        // Game-pak reads are sequential bus cycles: the cartridge's address counter advances
        // after every unit no matter what the source control says, so a fixed or decrementing
        // ROM source still delivers consecutive units (mgba-suite "=ROM"/"-ROM" rows expect the
        // same result as "+ROM"). Mirrors mGBA's handling of cart sources.
        let src_ctrl = if (0x08..=0x0D).contains(&(src >> 24)) { 0 } else { src_ctrl };

        self.in_dma = true;
        let mut access = Access::N;
        while remaining > 0 {
            // Reads from the BIOS/unused area return the channel's own latch (low half for a
            // 16-bit unit) and leave it unchanged; only mapped reads reload it.
            let unmapped = src < 0x0200_0000;
            if word32 {
                let bus_v = self.read32(src, access);
                if !unmapped {
                    latch = bus_v;
                }
                self.write32(dst, latch, access);
                self.open_bus = latch;
            } else {
                let bus_v = self.read16(src, access);
                if !unmapped {
                    latch = bus_v as u32 | (bus_v as u32) << 16;
                }
                self.write16(dst, latch as u16, access);
                self.open_bus = latch;
            }
            access = Access::S;
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
            remaining -= 1;

            if self.sched.now >= self.sched.next_deadline() {
                self.process_events();
                if remaining > 0 && self.dma.pending & higher != 0 {
                    // Yield to the higher-priority channel; resume later from here.
                    let c = &mut self.dma.ch[ch];
                    c.src = src;
                    c.dst = dst;
                    c.remaining = remaining;
                    c.latch = latch;
                    self.dma.pending |= 1 << ch;
                    self.in_dma = false;
                    return;
                }
            }
        }
        self.in_dma = false;

        let (irq_on_end, repeat_on) = {
            let c = &mut self.dma.ch[ch];
            c.src = src;
            c.dst = dst;
            c.remaining = 0;
            c.latch = latch;
            c.in_progress = false;
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
