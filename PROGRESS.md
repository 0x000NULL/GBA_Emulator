# PROGRESS.md — handoff document

Target: boot and play **Kingdom Hearts: Chain of Memories (USA)**, then Zelda (Minish Cap,
ALttP+FS, Classic NES Zelda 1/2) and Initial D Another Stage (J). Spec: `docs/gbatek/`.
Design: `docs/ARCHITECTURE.md`. Rules: `CLAUDE.md`.

## Milestone status

| M | Scope | Gate | Status |
|---|---|---|---|
| M0 | Workspace, contracts, harness, test suites fetched | `cargo build --workspace`; harness runs every golden entry | **DONE 2026-09-29** |
| M1 | ARM7TDMI: all ARM+Thumb, modes, exceptions, flags | jsmolka `arm.gba`, `thumb.gba` | in progress (CPU agent) |
| M2 | Memory map, waitstates, DMA, timers, IRQ, HALTCNT | jsmolka `memory.gba`, `bios.gba`, `nes.gba`; mgba-suite memory/timing | skeleton written (bus/dma/timers), untested |
| M3 | PPU modes 0-5, OBJ, windows, blending, mosaic, timing | jsmolka ppu/* golden hashes; mgba-suite video | stub (backdrop only); timing contract done |
| M4 | BIOS HLE, every listed SWI | `bios.gba` under HLE; per-SWI unit tests | partial: Halt/Stop/IntrWait/VBlankIntrWait/Div/DivArm/Sqrt/CpuSet/CpuFastSet/GetBiosChecksum/RegisterRamReset/SoundBias/MidiKey2Freq(f64); IRQ vector + return stub |
| M5 | Save detection, SRAM/Flash/EEPROM, `<rom>.sav` | jsmolka save/*; round trip | backends written, untested; persistence not wired in frontend |
| M6 | KH:CoM title -> new game -> first room | title golden + input script | not started |
| M7 | APU PSG + FIFO + mixing | PCM capture vs mGBA | stub (FIFOs work, silence out) |
| M8 | Prefetch, timing polish, savestates, fast-forward | mgba-suite timing improves | not started |

## Environment facts (2026-09-29)
- Rust 1.95 stable-msvc, MSVC 2022, no devkitARM, no BIOS dump. HLE is the only boot path;
  **M4's LLE differential gate is deferred** until a BIOS image is available via `GBA_BIOS`.
- Frontend stack: pure Rust `minifb` + `cpal` + `gilrs` (user decision; builds clean).
- Reference emulator: portable mGBA 0.10.5 at `tools/mgba/mGBA-0.10.5-win64/`.
- Test ROMs: `tools/tests/gba-tests` (jsmolka, 13 prebuilt .gba). `tools/tests/suite` is mgba's
  suite **source only**: it has no releases and needs devkitARM to build, so its golden entry is
  `skip`ped. Options: build in Docker (`docker-build.sh`) on a machine with Docker, or find a
  built `suite.gba` elsewhere and drop it at `tools/tests/suite/suite.gba`.
- ROM headers verified through the harness: KH `B8CE` 32 MB **SRAM_F_V103 -> Sram32K**; Minish Cap
  `BZME` 16 MB EEPROM; ALttP `AZLE` 8 MB EEPROM; Classic NES Zelda `FZLE` 1 MB EEPROM + ROM
  mirroring forced by game code `F`; Initial D `AINJ` 8 MB SRAM.

## Pass/fail conventions encoded in the harness (verified from suite sources)
- **jsmolka**: ROM ends spinning on `b .` (ARM `EAFFFFFE` / Thumb `E7FE`) after `m_test_eval`,
  which copies the result into **r12**: 0 = all passed, else the failing test number
  (`lib/macros.inc`; thumb.gba uses r7 as the source but r12 receives it). Harness: `--check jsmolka`.
- **mgba-suite**: menu-driven; results via mGBA debug registers (`0x04FFF600` text, `0x04FFF700`
  flags with bit 8 = flush, `0x04FFF780` = 0xC0DE enables, reads 0x1DEA) as `END: p/t`.
  Harness: `--check mgba-suite --input <script> --expect p/t`. Debug regs implemented in `bus/debug.rs`.

## Known bugs / open questions
- (none yet beyond "CPU is a stub")

## Attempts log
- 2026-09-29 M0: full workspace builds; `cargo test` runs 9 unit tests + the golden driver, which
  correctly fails all jsmolka entries against the stub CPU ("CPU not in the end-of-test spin loop").
