//! Bench: the published side of the developer "Bench" view.
//!
//! Capture itself lives in the subsystems that can see the data (`ppu::capture`,
//! `apu::capture`: the PPU and APU cannot reach `DebugHooks`). The Bus owns the published
//! `BenchCapture` in `DebugHooks::bench`, logs bus-level events per scanline, and once per frame,
//! at the `LineEnd` that makes `vcount == 160`, swaps the PPU's finished layer frame in and
//! snapshots the sound state (`bench_end_frame`). Nothing here is serialised.
//!
//! Cost when off: one `is_some()` per line end, per Timer 0 overflow and per DMA start, one
//! per rendered scanline and per audio sample. `Bus::set_bench(true)` attaches the Bus, PPU and
//! APU halves together; `set_bench(false)` drops all three.

use super::Bus;
use crate::apu::capture::ApuCapture;
use crate::ppu::capture::{LayerFrame, PpuCapture};

pub const EV_VBLANK_IRQ: u8 = 1 << 0;
pub const EV_DMA1_FIFO: u8 = 1 << 1;
pub const EV_DMA2_FIFO: u8 = 1 << 2;
pub const EV_DMA3_START: u8 = 1 << 3;
pub const EV_TIMER0_OVF: u8 = 1 << 4;
pub const EV_DMA0_START: u8 = 1 << 5;
pub const EV_DMA12_START: u8 = 1 << 6;

/// Scanlines per frame (visible + V-Blank).
pub const LINES: usize = 228;

/// Sound state at the end of a frame.
#[derive(Clone, Copy, Debug, Default)]
pub struct SoundFrame {
    /// Peak level per channel over the frame (`CH_*`): PSG 0..=15, FIFO 0..=128.
    pub peak: [u8; 6],
    /// Cumulative underruns of FIFO A and B.
    pub underruns: [u64; 2],
    /// SOUNDCNT_X bits 0-3: PSG channel ON flags.
    pub psg_on: u8,
    /// Square 1/2 duty (0-3).
    pub duty: [u8; 2],
    pub wave_two_banks: bool,
    pub noise_width7: bool,
    /// Timer (0/1) clocking FIFO A and B.
    pub fifo_timer: [u8; 2],
    /// DMA channel (1 or 2) currently feeding FIFO A and B, if any.
    pub fifo_dma: [Option<u8>; 2],
}

/// Everything the Bench view reads, published once per frame.
#[derive(Clone)]
pub struct BenchCapture {
    /// Per-layer pictures of the last completed frame.
    pub layers: Box<LayerFrame>,
    pub sound: SoundFrame,
    /// `EV_*` bits per scanline of the last completed frame.
    pub lines: [u8; LINES],
    /// PPU frame counter at publication.
    pub frame: u64,
    lines_cur: [u8; LINES],
}

impl Default for BenchCapture {
    fn default() -> Self {
        Self::new()
    }
}

impl BenchCapture {
    pub fn new() -> Self {
        BenchCapture { layers: Box::new(LayerFrame::new()), sound: SoundFrame::default(), lines: [0; LINES], frame: 0, lines_cur: [0; LINES] }
    }

    /// Record `ev` on scanline `vcount` of the frame in progress.
    #[inline]
    pub(crate) fn mark(&mut self, vcount: u32, ev: u8) {
        if let Some(l) = self.lines_cur.get_mut(vcount as usize) {
            *l |= ev;
        }
    }
}

impl std::fmt::Debug for BenchCapture {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BenchCapture")
            .field("frame", &self.frame)
            .field("enabled", &self.layers.enabled)
            .field("peak", &self.sound.peak)
            .finish_non_exhaustive()
    }
}

impl Bus {
    /// Attach or detach all three halves of the bench capture (idempotent). Detaching drops the
    /// layer and channel mutes with them, so output is bit-identical to a machine without a bench.
    pub fn set_bench(&mut self, on: bool) {
        if on {
            self.debug.bench.get_or_insert_with(|| Box::new(BenchCapture::new()));
            self.ppu.debug.get_or_insert_with(|| Box::new(PpuCapture::new()));
            self.apu.debug.get_or_insert_with(|| Box::new(ApuCapture::new()));
        } else {
            self.debug.bench = None;
            self.ppu.debug = None;
            self.apu.debug = None;
        }
    }

    /// Publish the finished frame. Called from `process_events` at the `LineEnd` that makes
    /// `vcount == VISIBLE_LINES`, which is where `Gba::run_frame` returns.
    pub(crate) fn bench_end_frame(&mut self) {
        let Bus { debug, ppu, apu, dma, .. } = self;
        let Some(b) = debug.bench.as_deref_mut() else { return };
        if let Some(pc) = ppu.debug.as_deref_mut() {
            std::mem::swap(&mut b.layers, &mut pc.cur);
            pc.cur.reset_frame();
            b.layers.obj_drawn = (b.layers.obj_seen[0].count_ones() + b.layers.obj_seen[1].count_ones()) as u8;
            // Entries that are not disabled (attr0 bit 9 clear, or affine via bit 8) with a legal shape.
            b.layers.obj_active = ppu
                .oam
                .chunks_exact(8)
                .filter(|e| {
                    let a0 = u16::from_le_bytes([e[0], e[1]]);
                    (a0 & (1 << 8) != 0 || a0 & (1 << 9) == 0) && a0 >> 14 != 3
                })
                .count() as u8;
        }
        let s = &mut b.sound;
        if let Some(ac) = apu.debug.as_deref_mut() {
            s.peak = ac.take_peaks();
        }
        s.underruns = [apu.fifo_a.underruns, apu.fifo_b.underruns];
        s.psg_on = apu.read_io8(0x84) & 0x0F;
        s.duty = [apu.sq1.duty, apu.sq2.duty];
        s.wave_two_banks = apu.wave.two_banks;
        s.noise_width7 = apu.noise.width7;
        let cnt_h = apu.reg16(0x82);
        s.fifo_timer = [((cnt_h >> 10) & 1) as u8, ((cnt_h >> 14) & 1) as u8];
        for (fifo, dst) in [0x0400_00A0u32, 0x0400_00A4].into_iter().enumerate() {
            s.fifo_dma[fifo] = (1..=2usize).find(|&ch| dma.is_fifo(ch) && dma.ch[ch].enabled() && dma.ch[ch].dst == dst).map(|ch| ch as u8);
        }
        b.lines = std::mem::replace(&mut b.lines_cur, [0; LINES]);
        b.frame = ppu.frame_count;
    }
}
