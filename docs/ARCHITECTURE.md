# Architecture

## Ownership tree
```
Gba { cpu: Cpu, bus: Bus }
Bus { sched, irq, ppu, apu, dma, timers, keypad, cart, bios, ewram, iwram, waitcnt, open_bus, debug, ... }
```
No `Rc<RefCell>`, no trait objects. `Cpu::step(&mut self, bus: &mut impl MemoryBus)` executes one
instruction. Subsystem event handlers receive `SysCtx { sched, irq, dma }`, never `&mut Bus`.
Cross-subsystem effects go through the bus's event dispatcher (`Bus::process_events`).

## MemoryBus (the CPU's view of the world)
`read8/16/32(addr, Access)`, `write8/16/32`, `fetch16/32` (opcode fetch; updates open bus and
`last_fetch`), `idle(cycles)`, `irq_pending()`, `now()`, plus the HLE hooks `is_hle_bios()`,
`swi_hle(num, &mut Regs)`, `bios_hle_vector(&mut Regs)`. The bus advances the clock by the
waitstate cost of every access; the CPU never computes waitstates. `Access::N`/`S` select the
non-sequential/sequential ROM waitstate from WAITCNT.

### CPU/HLE contract
- IRQ sampled at the top of `step`; exception entry sets SPSR/LR, ARM state, I flag, PC=vector.
- `swi n`: call `bus.swi_hle(n, regs)` with `regs.r[15]` = address of the following instruction.
  Returns true when serviced (no exception). Waiting SWIs rewind r15 and halt so the SWI re-executes.
- PC < 0x4000 under HLE: `bus.bios_hle_vector(regs)` emulates the BIOS IRQ dispatcher (vector 0x18:
  push r0-r3,r12,lr; r0=0x04000000; lr=0x138; pc=[0x03007FFC]) and the return stub at 0x138
  (pop; `subs pc, lr, #4`).
- BIOS-region reads from outside the BIOS return the GBATEK open-bus values (E129F000 after boot,
  E3A02004 after SWI, E25EF004 during IRQ, E55EC002 after IRQ return).

## Scheduler and main loop
`Scheduler { now, heap }` with `Event { HBlank, LineEnd, TimerOverflow(n), DmaStart(n), ApuSample,
ApuFrameSeq, KeypadPoll, IrqLatch }`. `Gba::run_until(target)`:
```
while now < target:
    deadline = min(next_event, target)
    while now < deadline:
        if dma.pending: dma_run(); continue          # CPU stalled, bus time advances inside
        if halted: wake if (IE & IF) != 0 else now = deadline; break
        cpu.step(bus)                                # may overshoot by one instruction
    process_events()                                 # pops everything with at <= now
```
DMA runs to completion between instructions (2N + 2(n-1)S + xI cycles, x=2 or 4 when both
addresses are in the game pak); immediate DMA starts 2 cycles after the enable write.

## Timing (timing.rs)
1232 cycles/line, H-Blank flag at 1006, 228 lines, 160 visible, V-Blank flag lines 160..=226,
280,896 cycles/frame (59.7275 Hz). Per line: `HBlank` renders line `vcount` (if < 160) with the
registers as they are at that moment, sets the flag, raises the H-Blank IRQ on every line, triggers
H-Blank DMA for lines < 160 and video-capture DMA for lines 2..=162. `LineEnd` clears the flag,
increments VCOUNT, handles the V-Count match, sets V-Blank (IRQ + DMA + frame ready) at 160, clears
it at 227, latches BG2/BG3 reference points at line 0. HLE boot starts at VCOUNT 0x7E with LineEnd
117 cycles away (mGBA GBASkipBIOS) so early traces align.

## Memory map and quirks (bus/mod.rs)
BIOS 0x00 (16K, protected), EWRAM 0x02 (256K, 16-bit bus, 3 cycles/halfword), IWRAM 0x03 (32K),
IO 0x04 (dispatch in bus/io.rs; mGBA debug regs at 0x04FFF600/700/780), PAL 0x05, VRAM 0x06
(96K mirrored into 128K), OAM 0x07, ROM 0x08-0x0D (three waitstate regions; out-of-range reads
return `(addr >> 1) & 0xFFFF`; per-cart mirroring for game code `F`), SRAM/Flash 0x0E-0x0F (8-bit
bus). 8-bit writes: PAL and BG-VRAM duplicate into the halfword, OBJ-VRAM and OAM ignore them.
EEPROM is reached through 0x0D (whole region for carts <= 16 MB, 0x0DFFFF00+ for larger).

## Harness (gba-test)
`load_rom` (gba/zip), `fb_hash` = xxh3-64 of the framebuffer as LE u16s, `checks::check_jsmolka`
(spin loop + r12 == 0), `checks::check_mgba_suite` (parse `END: p/t` from the debug log),
`golden::GoldenFile` (`tests/golden.toml`), `input_script::InputScript` (`{frame, press/release}`).
`tests/roms.rs` runs every entry under `cargo test`.

## Savestates (feature `savestate`)
Every state struct carries `#[cfg_attr(feature = "savestate", derive(Serialize, Deserialize))]`;
`state.rs` provides byte-array helpers for the big buffers and `Gba::save_state/load_state`
(magic `GBAS`, version, bincode body). Not stored: ROM, BIOS image (reattached from the live
machine), framebuffer and PPU scratch (regenerated), audio sample queue, debug hooks.

## Debug tooling
`--trace` writes mGBA's register layout (sixteen `%08X`, then `cpsr: %08X`, r15 = PC+8/PC+4) per
instruction; `--trace-hash` writes `cycle pc hash`. `--break PC`, `--watch LO..HI`, `--exit-on-swi N`,
`--dump-io`, `--png`, `--pcm`. Hooks live in `bus/debug.rs` behind a single `enabled` flag so the
hot path pays one branch.
