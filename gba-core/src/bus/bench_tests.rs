//! Bench plumbing: attach/detach, the scanline event log and the once-per-frame publish.
//! These drive the bus directly (no CPU): events are popped one at a time like `run_until` does.

use super::bench::*;
use super::tests::{bus, finish_dma, start_dma};
use super::*;
use crate::apu::capture::CH_SQ1;

/// Schedule the first line events (non-HLE boot: vcount 0, HBlank/LineEnd pending) and audio.
fn boot(b: &mut Bus) {
    let Bus { sched, irq, dma, ppu, .. } = b;
    ppu.boot_schedule(&mut SysCtx { sched, irq, dma }, false);
    b.apu.boot_schedule(&mut b.sched);
}

/// Advance to the next scheduled event and run everything it makes due.
fn step(b: &mut Bus) {
    let at = b.sched.next_deadline();
    b.sched.now = b.sched.now.max(at);
    b.process_events();
    b.dma_run();
}

/// Run until `vcount == line` (just after the LineEnd that made it so).
fn run_to_line(b: &mut Bus, line: u32) {
    while b.ppu.vcount != line {
        step(b);
    }
}

/// Run to the next frame publish (the LineEnd that makes `vcount == 160`).
fn run_to_frame_end(b: &mut Bus) {
    loop {
        step(b);
        if b.ppu.vcount == VISIBLE_LINES {
            break;
        }
    }
}

use crate::timing::VISIBLE_LINES;

fn lines(b: &Bus) -> [u8; LINES] {
    b.debug.bench.as_ref().unwrap().lines
}

#[test]
fn bench_off_by_default() {
    let mut b = bus();
    assert!(b.debug.bench.is_none());
    assert!(b.ppu.debug.is_none());
    assert!(b.apu.debug.is_none());
    assert!(!b.debug.enabled, "the bench is not a watch/break hook");

    b.set_bench(true);
    assert!(b.debug.bench.is_some() && b.ppu.debug.is_some() && b.apu.debug.is_some());
    assert!(!b.debug.enabled, "attaching the bench does not turn on per-access scans");

    // Idempotent: a second attach keeps the existing captures (and their mutes).
    b.ppu.debug.as_deref_mut().unwrap().mute = 0x11;
    b.apu.debug.as_deref_mut().unwrap().set_mute(0x21);
    b.set_bench(true);
    assert_eq!(b.ppu.debug.as_deref().unwrap().mute, 0x11);
    assert_eq!(b.apu.debug.as_deref().unwrap().mute, 0x21);

    b.set_bench(false);
    assert!(b.debug.bench.is_none() && b.ppu.debug.is_none() && b.apu.debug.is_none());
    b.set_bench(true);
    assert_eq!(b.ppu.debug.as_deref().unwrap().mute, 0, "mutes vanished with the capture");
    assert_eq!(b.apu.debug.as_deref().unwrap().mute, 0);
}

#[test]
fn scanline_log_marks_events_on_the_current_line() {
    let mut b = bus();
    b.set_bench(true);
    boot(&mut b);
    b.write16(0x0400_0004, 0x0008, Access::N); // V-Blank IRQ enable

    // Timer 0 overflows once, 1024 cycles into line 5 (disabled again at the start of line 6).
    run_to_line(&mut b, 5);
    b.write16(0x0400_0100, 0xFC00, Access::N);
    b.write16(0x0400_0102, 0x0080, Access::N);
    run_to_line(&mut b, 6);
    b.write16(0x0400_0102, 0x0000, Access::N);

    // Immediate DMA0 on line 7, DMA3 on line 8, DMA1 and DMA2 (not FIFO) on lines 9 and 10.
    for (line, ch) in [(7, 0u32), (8, 3), (9, 1), (10, 2)] {
        run_to_line(&mut b, line);
        start_dma(&mut b, ch, 0x0200_0000, 0x0300_0000, 2, 0);
        finish_dma(&mut b);
        assert_eq!(b.ppu.vcount, line, "the transfer is short enough to stay on the line");
    }

    run_to_frame_end(&mut b);
    let l = lines(&b);
    assert_eq!(l[5], EV_TIMER0_OVF);
    assert_eq!(l[7], EV_DMA0_START);
    assert_eq!(l[8], EV_DMA3_START);
    assert_eq!(l[9], EV_DMA12_START);
    assert_eq!(l[10], EV_DMA12_START);
    for (y, &v) in l.iter().enumerate() {
        if ![5, 7, 8, 9, 10].contains(&y) {
            assert_eq!(v, 0, "line {y}");
        }
    }

    // The V-Blank IRQ is logged on line 160 as the frame boundary passes; it is published with
    // the frame that follows.
    assert_eq!(l[160], 0);
    run_to_line(&mut b, 161);
    run_to_frame_end(&mut b);
    assert_eq!(lines(&b)[160], EV_VBLANK_IRQ);
}

#[test]
fn vblank_irq_is_only_logged_when_enabled() {
    let mut b = bus();
    b.set_bench(true);
    boot(&mut b);
    run_to_frame_end(&mut b);
    run_to_line(&mut b, 161);
    run_to_frame_end(&mut b);
    assert_eq!(lines(&b)[160], 0, "DISPSTAT bit 3 clear: no V-Blank IRQ");
}

const FIFO_DMA: u16 = 0x0200 | (3 << 12) | (2 << 5) | 0x0400; // repeat, special timing, fixed dst, 32-bit

#[test]
fn fifo_dma_is_logged_as_fifo() {
    let mut b = bus();
    b.set_bench(true);
    boot(&mut b);
    start_dma(&mut b, 1, 0x0200_0000, 0x0400_00A0, 1, FIFO_DMA);
    start_dma(&mut b, 2, 0x0200_0000, 0x0400_00A4, 1, FIFO_DMA);

    run_to_line(&mut b, 30);
    b.dma.trigger(DmaTrigger::FifoA);
    b.dma_run();
    run_to_line(&mut b, 31);
    b.dma.trigger(DmaTrigger::FifoB);
    b.dma_run();
    run_to_frame_end(&mut b);
    let l = lines(&b);
    assert_eq!(l[30], EV_DMA1_FIFO, "FIFO refills are not plain DMA1 starts");
    assert_eq!(l[31], EV_DMA2_FIFO);
    assert_eq!(l.iter().filter(|&&v| v != 0).count(), 2);
}

/// A long DMA3 is paused every line by an H-Blank DMA1 and resumed; only its fresh start is logged.
#[test]
fn resumed_transfer_is_not_re_marked() {
    let mut b = bus();
    b.set_bench(true);
    boot(&mut b);
    run_to_line(&mut b, 20);
    // DMA1: H-Blank timing, repeat, 1 unit. DMA3: immediate, 0x1000 halfwords (~13 lines).
    start_dma(&mut b, 1, 0x0200_0000, 0x0300_1000, 1, 0x0200 | (2 << 12));
    start_dma(&mut b, 3, 0x0200_0000, 0x0300_0000, 0x1000, 0);
    finish_dma(&mut b);
    assert!(b.ppu.vcount >= 30, "the transfer spans many lines (now line {})", b.ppu.vcount);
    assert!(!b.dma.ch[3].enabled(), "and has completed");

    run_to_frame_end(&mut b);
    let l = lines(&b);
    assert_ne!(l[20] & EV_DMA3_START, 0, "the fresh start is logged on its line");
    for (y, &v) in l.iter().enumerate().take(30).skip(21) {
        assert_eq!(v & EV_DMA3_START, 0, "line {y}: resuming is not a start");
        assert_ne!(v & EV_DMA12_START, 0, "line {y}: the H-Blank DMA1 starts afresh every line");
    }
}

#[test]
fn frame_end_publishes_and_resets() {
    let mut b = bus();
    // BG0: tile 1 (red) at map entry 0; two visible sprites (tile 1, OBJ palette 1 = red), one
    // blank affine sprite, one with the prohibited shape, the rest disabled.
    b.ppu.pal[2..4].copy_from_slice(&0x001Fu16.to_le_bytes());
    b.ppu.pal[(256 + 1) * 2..(256 + 1) * 2 + 2].copy_from_slice(&0x001Fu16.to_le_bytes());
    b.ppu.vram[32..64].fill(0x11);
    b.ppu.vram[0x4000..0x4002].copy_from_slice(&1u16.to_le_bytes());
    b.ppu.vram[0x1_0020..0x1_0040].fill(0x11);
    for i in 0..128 {
        b.ppu.oam[i * 8..i * 8 + 2].copy_from_slice(&0x0200u16.to_le_bytes());
    }
    let mut set_obj = |i: usize, a0: u16, a1: u16, a2: u16| {
        b.ppu.oam[i * 8..i * 8 + 2].copy_from_slice(&a0.to_le_bytes());
        b.ppu.oam[i * 8 + 2..i * 8 + 4].copy_from_slice(&a1.to_le_bytes());
        b.ppu.oam[i * 8 + 4..i * 8 + 6].copy_from_slice(&a2.to_le_bytes());
    };
    set_obj(0, 0, 100, 1);
    set_obj(1, 0, 120, 1);
    set_obj(6, 0x0300, 140, 0); // affine, double size, blank tile
    set_obj(9, 0xC000, 150, 1); // prohibited shape
    b.write16(0x0400_0000, 1 << 8 | 1 << 12, Access::N);
    b.write16(0x0400_0008, 8 << 8, Access::N); // BG0CNT: screen block 8
    // Sound: SQ1 at volume 10, routed to both speakers.
    b.write8(0x0400_0084, 0x80, Access::N);
    b.write16(0x0400_0080, 0xFF77, Access::N);
    b.write8(0x0400_0063, 0xA0, Access::N);
    b.write8(0x0400_0065, 0x80, Access::N);

    b.set_bench(true);
    boot(&mut b);
    b.debug.bench.as_mut().unwrap().mark(3, EV_TIMER0_OVF);
    assert_eq!(b.debug.bench.as_ref().unwrap().frame, 0, "nothing is published before the first frame ends");

    run_to_frame_end(&mut b);
    {
        let p = b.debug.bench.as_ref().unwrap();
        assert_eq!(p.frame, b.ppu.frame_count);
        assert_eq!(p.frame, 1);
        assert_eq!(p.layers.enabled, 0b1_0001);
        assert_eq!(p.layers.planes[0][0], 0x001F);
        assert_eq!(p.layers.planes[4][240 + 100], 0x001F, "line 1 (line 0 of a booted frame has no OBJ prepass)");
        assert_eq!(p.layers.planes[4][240 + 99], 0x8000);
        assert_eq!(p.layers.obj_drawn, 2);
        assert_eq!(p.layers.obj_active, 3, "two normal + one affine; the prohibited shape and disabled entries are out");
        assert_eq!(p.sound.peak[CH_SQ1], 10);
        assert_eq!(p.sound.psg_on & 1, 1);
        assert_eq!(p.sound.underruns, [0, 0]);
        assert_eq!(p.sound.fifo_dma, [None, None]);
        assert_eq!(p.lines[3], EV_TIMER0_OVF);
    }

    // Later frames: BG0 and OBJ off, SQ1 off. Everything per-frame starts over.
    b.write16(0x0400_0000, 0, Access::N);
    b.write8(0x0400_0063, 0x00, Access::N);
    // A sample due at the very instant of the boundary lands in the next frame, so look at the frame after.
    for _ in 0..2 {
        run_to_line(&mut b, 161);
        run_to_frame_end(&mut b);
    }
    let p = b.debug.bench.as_ref().unwrap();
    assert_eq!(p.frame, 3);
    assert_eq!(p.layers.enabled, 0, "the published frame is the new one, not an accumulation");
    assert!(p.layers.planes.iter().all(|pl| pl.iter().all(|&c| c == 0x8000)));
    assert_eq!(p.layers.obj_drawn, 0);
    assert_eq!(p.sound.peak, [0; 6]);
    assert_eq!(p.lines[3], 0, "the line log restarted");
}

#[test]
fn debug_output_never_dumps_planes() {
    let mut b = bus();
    b.set_bench(true);
    let s = format!("{:?}", b.debug.bench.as_ref().unwrap());
    assert!(s.starts_with("BenchCapture") && s.len() < 200, "{s}");
    // DebugHooks stays Debug + Clone with the bench inside.
    let c = b.debug.clone();
    assert!(c.bench.is_some());
    let _ = format!("{:?}", c);
}
