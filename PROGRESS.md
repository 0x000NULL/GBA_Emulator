# PROGRESS.md — handoff document

Target: boot and play **Kingdom Hearts: Chain of Memories (USA)**, then Zelda (Minish Cap,
ALttP+FS, Classic NES Zelda 1/2) and Initial D Another Stage (J). Spec: `docs/gbatek/`.
Design: `docs/ARCHITECTURE.md`. Rules and commands: `CLAUDE.md`. Overview: `README.md`.

## Where things stand (2026-09-30)

- **Success criterion met.** KH:CoM boots under the HLE BIOS, plays the intro, reaches Castle
  Oblivion and the battle tutorial with player control. Headless proof: `tests/inputs/khcom-newgame.toml`
  plus the two KH goldens in `tests/compat.toml` (title at frame 1800, gameplay at frame 27400).
- **All jsmolka ROMs pass** (arm, thumb, memory, bios, nes, save/none, sram, flash64, flash128;
  ppu hello/shades/stripes by golden hash). `cargo test --workspace` is green: 231 core unit
  tests, 25 frontend tests, 2 ROM-driver tests (~60-80 s with the commercial ROMs present).
- **Compat**: see the table below. Every priority title renders its title screen; KH and Minish
  Cap are verified controllable (LEFT/RIGHT/no-input runs diverge).
- **Open**: frontend audio only unit-tested (no audio device here); reference-emulator audio
  comparison manual; mgba-suite gates blocked (needs devkitARM); prefetch/timing polish deferred.

## Milestone status

| M | Scope | Gate | Status |
|---|---|---|---|
| M0 | Workspace, contracts, harness, test suites fetched | `cargo build --workspace`; harness runs every golden entry | **DONE 2026-09-29** |
| M1 | ARM7TDMI: all ARM+Thumb, modes, exceptions, flags | jsmolka `arm.gba`, `thumb.gba` | **DONE 2026-09-29**: 65 CPU unit tests; real 2-slot pipeline; mGBA-format trace |
| M2 | Memory map, waitstates, DMA, timers, IRQ, HALTCNT | jsmolka `memory.gba`, `bios.gba`, `nes.gba`; mgba-suite memory/timing | **DONE (jsmolka side) 2026-09-29**; mgba-suite part blocked on a built `suite.gba` |
| M3 | PPU modes 0-5, OBJ, windows, blending, mosaic, timing | jsmolka ppu/* golden hashes; mgba-suite video | **DONE (jsmolka side) 2026-09-29**: goldens pinned, 56 PPU unit tests. Gaps: per-line OBJ cycle budget not modelled, OAM evaluated on the rendered line rather than one line early; mgba-suite video blocked as above |
| M4 | BIOS HLE, every listed SWI | `bios.gba` under HLE; per-SWI unit tests | **DONE 2026-09-29**: `bios.gba` passes; all listed SWIs + IRQ vector/return emulation; 14 unit tests incl. LZ77/RL/Huffman encoder round trips. Sound-driver SWIs (1A-1E, 28, 29) are logged no-ops. LLE differential deferred (no BIOS image) |
| M5 | Save detection, SRAM/Flash/EEPROM, `<rom>.sav` | jsmolka save/*; round trip | **DONE 2026-09-29**: none/sram/flash64/flash128 pass (flash entries need 400 frames: tests 10/11 program 64K bytes one command sequence each); 8 cart unit tests incl. EEPROM protocol; `.sav` persistence in the frontend |
| M6 | KH:CoM title -> new game -> first room | title golden + input script | **DONE 2026-09-29**: see "Where things stand". Other priority titles reach title/file screens; Minish Cap reaches gameplay (golden + script) |
| M7 | APU PSG + FIFO + mixing | PCM capture vs mGBA | **DONE (underrun half) 2026-09-29**: 70 APU unit tests; KH title capture (`--pcm`, 65536 Hz) has 0 underruns over frames 1200-1800, clean stereo music, no clipping. **Reference comparison deferred** (manual: `scripts/compare-wav.py`) |
| M8 | Prefetch, timing polish, savestates, fast-forward | mgba-suite timing improves | **PARTIAL 2026-09-29**: savestates (feature `savestate`, ~429 KB, KH resume bit-exact vs straight run; F1/F3; harness `--save-state/--load-state`), Tab fast-forward, audio-clocked pacing, ~1900 fps headless release on KH. **Deferred**: game-pak prefetch buffer, per-instruction timing refinement (no runnable gate) |

## Compatibility (headless, HLE BIOS)

| Game | Code | Save | Reached | Pinned |
|---|---|---|---|---|
| Kingdom Hearts: Chain of Memories (USA) | B8CE | SRAM_F 32K | Battle tutorial, controllable; music plays | title 1800, gameplay 27400 |
| Zelda: The Minish Cap (USA) | BZME | EEPROM 8K | Link's house with HUD, controllable | title 1800, gameplay 12000 |
| Zelda: A Link to the Past & Four Swords (USA) | AZLE | EEPROM 512 | CHOOSE A FILE screen | 1800 |
| Classic NES Series: Zelda (USA/EU) | FZLE | EEPROM (forced) | Story scroll (ROM mirroring, VRAM execution OK) | 1800 |
| Classic NES Series: Zelda II (USA/EU) | FLBE | EEPROM (forced) | Title | 1800 |
| Initial D: Another Stage (J) | AINJ | SRAM 32K | Name entry, car select, story dialogue | title 1800 |
| Final Fantasy IV Advance (USA) | | | Health warning screen; executes `swi 0xFF` (logged) | no |

## Environment facts
- Rust 1.95 stable-msvc; no devkitARM; no BIOS dump. HLE is the only boot path.
- Frontend stack: pure Rust `minifb` + `cpal` + `gilrs` (user decision).
- Reference emulator: portable mGBA 0.10.5 at `tools/mgba/mGBA-0.10.5-win64/`. Its Windows build has
  **no CLI debugger** (`-d`), only GDB (`-g`), and no headless audio dump, so trace diffing and audio
  comparison need a different build or a manual GUI session.
- Test ROMs: `tools/tests/gba-tests` (jsmolka, prebuilt). `tools/tests/suite` is mgba's suite source;
  it has no releases and needs devkitARM (or its `docker-build.sh`) to build. Drop a built
  `suite.gba` at `tools/tests/suite/suite.gba` and remove the `skip` in `tests/golden.toml` to enable it.
- The development session was RDP without an audio endpoint: cpal's stream, audio-clocked pacing and
  the underrun counter have unit tests only. Verify once at a console: `cargo run --release -p gba-frontend -- <rom>`,
  title bar should show `59.7 fps` with underruns at 0. Hidden flag `--exit-after N` ends a windowed
  run after N frames and prints timing stats.

## Pass/fail conventions encoded in the harness (verified from suite sources)
- **jsmolka**: ROM ends spinning on `b .` (ARM `EAFFFFFE` / Thumb `E7FE`) after `m_test_eval`,
  which copies the result into **r12**: 0 = all passed, else the failing test number
  (`lib/macros.inc`; thumb.gba uses r7 as the source but r12 receives it). Harness: `--check jsmolka`.
- **mgba-suite**: menu-driven; results via mGBA debug registers (`0x04FFF600` text, `0x04FFF700`
  flags with bit 8 = flush, `0x04FFF780` = 0xC0DE enables, reads 0x1DEA) as `END: p/t`.
  Harness: `--check mgba-suite --input <script> --expect p/t`. Debug regs implemented in `bus/debug.rs`.

## Known gaps / judgement calls
- PPU: no per-scanline OBJ cycle budget; OAM evaluated on the rendered line; forced blank does not
  advance affine references (mGBA-like); window `x1 > 240 && x1 > x2` covers the whole line (mGBA).
- APU: PSG mix scale derived from GBATEK maxima, not measured against hardware; wave RAM
  shift-register read-back quirk not modelled; duty position not reset on restart.
- CPU: MUL/MLA leave C unchanged (GBATEK says "destroyed"); cycle counts are GBATEK-approximate;
  no game-pak prefetch buffer.
- BIOS HLE: BgAffineSet/ObjAffineSet use f64 trig instead of the BIOS sine table (tiny rounding
  differences possible); division by zero returns mGBA's values instead of hanging.
- Cartridge: no GPIO/RTC (none of the target games use it); SRAM open-bus when the backend is EEPROM
  reads 0xFF.

## Fix log (integration, 2026-09-29)
- `Gba::run_until` computed the event deadline once per outer loop, so an event scheduled by an
  IO write (immediate DMA start, +2 cycles) fired only after the old deadline. nes.gba test 2 (STMDA
  into DMA0SAD/DAD/CNT then read the destination) caught it. Now the deadline is re-read every step.
- CPU STRH/STR masked the address before `write16/write32`; the 8-bit SRAM/Flash bus selects the
  byte lane from the unaligned address (sram/flash tests 6 and 8). Alignment is the bus's job.
- Missing backup chip reads 0xFF, not 0 (save/none test 1).
- DMAxDAD byte writes used the raw register offset as the shift (bus unit test).
- **Frame drift** (found via KH audio clicks): `Ppu::on_line_end` timed the next line from
  `sched.now`, which is past the event's scheduled time whenever the CPU or a DMA overshot, so
  every frame ran a little long and drifted against the 15768 Hz sound timer; the m4a driver's
  6-frame DMA restart then let the FIFO fetch 16 bytes past its buffer (periodic left-channel
  clicks in the silent intro). Scanline and APU events are now scheduled from the event time.
- **DMA priority**: a long low-priority transfer (e.g. a DMA3 graphics copy) ran to completion
  while timer overflows queued up, then a burst of FIFO pops hit an empty FIFO (FF IV Advance:
  58-89 underruns per quarter frame). Transfers now fire due events after every unit and yield to
  a higher-priority pending channel, resuming afterwards (GBATEK: higher-priority DMA may
  offhold sound DMA). FF IV underruns over 900 frames dropped to 270 (all in its boot phase).
- `--pcm` WAV rate follows SOUNDBIAS (KH selects 65536 Hz).

## Next steps, in order of value
1. Console check of frontend audio (see Environment facts).
2. Build `suite.gba` (Docker + devkitARM) to unblock the mgba-suite memory/timing/video gates, then
   the deferred M8 work (prefetch buffer, timing) has a measurable gate.
3. Deeper compat runs: ALttP into gameplay (H-Blank register tricks), Initial D into a race
   (affine road), Zelda II past the title; pin goldens in `tests/compat.toml`.
4. Investigate FF IV Advance's `swi 0xFF` (what the real BIOS does for out-of-range SWI numbers).
