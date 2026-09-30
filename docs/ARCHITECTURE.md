# Architecture

## Ownership tree
```
Gba { cpu: Cpu, bus: Bus }
Bus { sched, irq, ppu, apu, dma, timers, keypad, cart, bios, ewram, iwram, waitcnt, sio, open_bus, debug, ... }
```
No `Rc<RefCell>`, no trait objects. `Cpu::step(&mut self, bus: &mut impl MemoryBus)` executes one
instruction. Subsystem event handlers receive `SysCtx { sched, irq, dma }`, never `&mut Bus`.
Cross-subsystem effects go through the bus's event dispatcher (`Bus::process_events`).

Module map (gba-core/src): `cpu/` (regs, alu, decode tables, arm, thumb, disasm, exceptions),
`ppu/` (regs views, bg_text, bg_affine, bitmap, obj, compose; timing contract in mod.rs),
`apu/` (regs, psg, fifo, mixer), `bus/` (memory map + waitstates in mod.rs, IO dispatch in io.rs,
DMA execution in dma_exec.rs, debugger hooks in debug.rs), `dma.rs`, `timers.rs`, `irq.rs`,
`keypad.rs`, `cart/` (header, save detection, SRAM/flash/eeprom), `bios/` (LLE image + HLE SWIs),
`sched.rs`, `timing.rs`, `gba.rs` (main loop), `state.rs` (savestates, feature-gated).

## MemoryBus (the CPU's view of the world)
`read8/16/32(addr, Access)`, `write8/16/32`, `fetch16/32` (opcode fetch; updates open bus and
`last_fetch`), `idle(cycles)`, `irq_pending()`, `now()`, plus the HLE hooks `is_hle_bios()`,
`swi_hle(num, &mut Regs)`, `bios_hle_vector(&mut Regs)`. The bus advances the clock by the
waitstate cost of every access; the CPU never computes waitstates. `Access::N`/`S` select the
non-sequential/sequential ROM waitstate from WAITCNT. The CPU passes **unaligned** addresses
through: RAM regions align them, the 8-bit SRAM/Flash bus uses the low bits to pick the byte lane.

### CPU/HLE contract
- IRQ sampled at the top of `step`; exception entry sets SPSR/LR, ARM state, I flag, PC=vector.
- `swi n`: call `bus.swi_hle(n, regs)` with `regs.r[15]` = address of the following instruction.
  Returns true when serviced (no exception). Waiting SWIs (IntrWait) rewind r15 and halt so the SWI
  re-executes after the interrupt; a flag in `Bios` suppresses the "discard old flags" step on re-entry.
- PC < 0x4000 under HLE: `bus.bios_hle_vector(regs)` emulates the BIOS IRQ dispatcher (vector 0x18:
  push r0-r3,r12,lr; r0=0x04000000; lr=0x138; pc=[0x03007FFC]) and the return stub at 0x138
  (pop; `subs pc, lr, #4`).
- BIOS-region reads from outside the BIOS return the GBATEK open-bus values (E129F000 after boot,
  E3A02004 after SWI, E25EF004 during IRQ, E55EC002 after IRQ return).
- Pipeline: two fetched words; the executing instruction came from an earlier fetch, so a store
  over PC+8 does not change what runs (Classic NES Series depends on it). PC writes flush; the next
  step refills with an N then an S fetch.

## Scheduler and main loop
`Scheduler { now, heap }` with `Event { HBlank, LineEnd, TimerOverflow(n), DmaStart(n), ApuSample,
ApuFrameSeq, KeypadPoll, IrqLatch }`. `Gba::run_until(target)`:
```
while now < target:
    if now >= next_event:  process_events(); continue     # deadline re-read every iteration
    if dma.pending:        dma_run(); continue            # CPU stalled
    if halted/stopped:     wake if (IE & IF) != 0 else now = min(next_event, target); continue
    cpu.step(bus)                                         # may overshoot by one instruction
```
Re-reading the deadline every iteration matters: an IO write can schedule an event two cycles
ahead (immediate DMA start), and games read the DMA result on the very next instruction.

Every periodic handler schedules its successor from the event's **scheduled** time (`at`), never
from `now`, so CPU/DMA overshoot cannot lengthen a frame or drift the APU sample clock.

## DMA (dma.rs, bus/dma_exec.rs)
Registers and trigger matching live in `DmaController`; execution in `Bus::dma_run`, lowest channel
first. Cost 2N + 2(n-1)S + xI (x = 2, or 4 with both addresses in the game pak); immediate mode
starts 2 cycles after the enable write via `Event::DmaStart`. Transfers are **resumable**: after
every unit the bus fires due events, and if a higher-priority channel became pending the running
one saves `src/dst/remaining` and yields (`in_progress`), so sound FIFO DMAs are served during long
graphics copies. FIFO mode forces 4 words to a fixed destination; the request comes from
`Apu::on_timer_overflow` when a FIFO drops to 16 bytes. A DMA into the EEPROM region hints the
EEPROM size from its length (9/73 halfwords = 512 B, 17/81 = 8 KB).

## Timing (timing.rs, ppu/mod.rs)
1232 cycles/line, H-Blank flag at 1006, 228 lines, 160 visible, V-Blank flag lines 160..=226,
280,896 cycles/frame (59.7275 Hz). `HBlank` renders line `vcount` (if < 160) with the registers as
they are at that moment, sets the flag, raises the H-Blank IRQ on every line, triggers H-Blank DMA
for lines < 160 and video-capture DMA for lines 2..=162. `LineEnd(at)` clears the flag, increments
VCOUNT, handles the V-Count match, sets V-Blank (IRQ + DMA + frame ready) at 160, clears it at 227,
latches BG2/BG3 reference points at line 0, and schedules the next `HBlank`/`LineEnd` from `at`.
HLE boot starts at VCOUNT 0x7E with LineEnd 117 cycles away (mGBA GBASkipBIOS) so early traces align.
Timers are event-driven (one overflow event per running timer; reads derive the count from time;
cascade timers are bumped by the previous timer's overflow).

## Memory map and quirks (bus/mod.rs)
BIOS 0x00 (16K, protected), EWRAM 0x02 (256K, 16-bit bus, 3 cycles/halfword), IWRAM 0x03 (32K),
IO 0x04 (dispatch in bus/io.rs; mGBA debug regs at 0x04FFF600/700/780), PAL 0x05, VRAM 0x06
(96K mirrored into 128K), OAM 0x07, ROM 0x08-0x0D (three waitstate regions; out-of-range reads
return `(addr >> 1) & 0xFFFF`; per-cart mirroring for game code `F`), SRAM/Flash 0x0E-0x0F (8-bit
bus, byte replicated on wider reads, byte lane selected by address on wider writes; 0xFF when no
chip). 8-bit writes: PAL and BG-VRAM duplicate into the halfword, OBJ-VRAM and OAM ignore them.
EEPROM is reached through 0x0D (whole region for carts <= 16 MB, 0x0DFFFF00+ for larger).

## Harness (gba-test)
`load_rom` (gba/zip), `fb_hash` = xxh3-64 of the framebuffer as LE u16s, `checks::check_jsmolka`
(spin loop + r12 == 0), `checks::check_mgba_suite` (parse `END: p/t` from the debug log),
`golden::GoldenFile` (`tests/golden.toml` and `tests/compat.toml`; entries take `check`, `reg`,
`golden`, `frames`, inline `input` steps or an `input_file`, `expect`, `skip`),
`input_script::InputScript` (`{frame, press/release}` with `A+START` combos).
`tests/roms.rs` runs every entry under `cargo test`; compat entries skip when the ROM is absent.
CLI extras: `--stats-from F` (underruns/samples counted from frame F), `--pcm` (WAV at the
SOUNDBIAS rate), `--save-state/--load-state`, `--record` (prints a manifest stanza).

## Savestates (feature `savestate`)
Every state struct carries `#[cfg_attr(feature = "savestate", derive(Serialize, Deserialize))]`;
`state.rs` provides byte-array helpers for the big buffers and `Gba::save_state/load_state`
(magic `GBAS`, version, bincode body). Not stored: ROM, BIOS image (reattached from the live
machine), framebuffer and PPU scratch (regenerated), audio sample queue, debug hooks. Resuming
KH:CoM from a state reproduces the straight run's framebuffer hash exactly.

## Frontend (gba-frontend)
`window.rs` (BGR555 LUT, integer upscale, minifb), `input.rs` (keyboard + gilrs), `audio.rs`
(SPSC ring, linear resampler from the core's SOUNDBIAS rate to the device rate, cpal stream,
underrun counter), `pacing.rs` (1/59.7275 s frame clock, adaptive sleep margin, +-2% audio-fill
correction), `saves.rs` (atomic `<rom>.sav` writes every 60 frames/F5/exit), `debug_ui.rs`
(hotkeys, IO dump, screenshots, savestates). Runs without an audio device.

## Debug tooling
`--trace` writes mGBA's register layout (sixteen `%08X`, then `cpsr: %08X`, r15 = PC+8/PC+4) per
instruction; `--trace-hash` writes `cycle pc hash`. `--break PC`, `--watch LO..HI`, `--exit-on-swi N`,
`--dump-io`, `--png`. Hooks live in `bus/debug.rs` behind a single `enabled` flag so the hot path
pays one branch. Guest `mgba` debug prints are collected and echoed as `guest:` lines.
