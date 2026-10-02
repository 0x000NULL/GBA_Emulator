# PROGRESS.md — handoff document

Target: boot and play **Kingdom Hearts: Chain of Memories (USA)**, then Zelda (Minish Cap,
ALttP+FS, Classic NES Zelda 1/2) and Initial D Another Stage (J). Spec: `docs/gbatek/`.
Design: `docs/ARCHITECTURE.md`. Rules and commands: `CLAUDE.md`. Overview: `README.md`.

## Where things stand (2026-10-02)

- **Success criterion met.** KH:CoM boots under the HLE BIOS, plays the intro, reaches Castle
  Oblivion and the battle tutorial with player control. Headless proof: `tests/inputs/khcom-newgame.toml`
  plus the two KH goldens in `tests/compat.toml` (title at frame 1800, gameplay at frame 27400).
- **All jsmolka ROMs pass** (arm, thumb, memory, bios, nes, save/none, sram, flash64, flash128;
  ppu hello/shades/stripes by golden hash). `cargo test --workspace` is green: 382 core unit
  tests, 98 frontend tests (+86 in fruit-basket), 2 ROM-driver tests (~11 min with all 66 commercial goldens present).
- **v1.0.0 released (2026-10-01)**: tag `v1.0.0`; `.github/workflows/release.yml` builds the
  player for Windows x64, Linux x64, macOS x64/arm64 with `--profile dist` and attaches the
  archives to the GitHub release (`CHANGELOG.md` is the release body). Procedure in CLAUDE.md.
  **v1.1.0 (2026-10-01)**: the BIOS sound driver release; savestate format 4; the workflow now
  runs entirely on self-hosted runners (PR #2) after the hosted jobs hit the account's spending limit.
  **v1.2.0 (2026-10-02)**: gamepad rebinding and the reference audio comparison tooling.
- **Rebrand (2026-10-02)**: the player is **Strawberry** (`docs/branding/`, Fruit Basket No. 2).
  Window title, binary and archives (`strawberry`), `strawberry/` config and cache folders with a
  one-time move from `gba-advance/` (`app.rs migrate_dir`), the mark parsed from the SVG
  (`ui/brand.rs`), window icon (`icon.rs`: Windows via a `.ico` written to the cache folder, X11
  via `_NET_WM_ICON`; minifb's `set_icon` panics on Wayland and macOS so both are skipped), exe
  icon and version block (`build.rs` + `winresource`, warns without `rc.exe`), and the empty
  Library built from board 05. Crate names stay `gba-*`. Released as **v1.3.0 (2026-10-02)**.
  After the tag: README with the dark/light lockup, every screen's render in `docs/screenshots/`
  and a brand section; the settings header fixed; the release workflow's actions bumped to the
  Node 24 majors (checkout v7, upload-artifact v7, download-artifact v8, gh-release v3), first
  exercised by the next tag.
- **BIOS sound driver HLE (2026-10-01)**: SWI 1Ah-1Eh, 28h, 29h and the SoundGetJumpList
  routines are emulated (`gba-core/src/bios/sound/`, guest callbacks in `bios/call.rs`); the six
  titles the sweep left silent play music, pinned by seven `tests/compat.toml` rows with
  `audio_frames_min`. See the fix log of that date.
- **Library sweep (2026-10-01)**: the user's full library (1,543 titles) runs through
  `scripts/triage.py` (harness `gba-test --triage`): **1,528 boot to a moving picture with sound**
  (1,522 before the sound driver), 10 stop at a menu the sweep does not answer (verified by eye),
  1 needs its MP3 cartridge, 1 is DS expansion data, and **3 are open**: Hello Kitty Collection - Miracle Fashion Maker and both
  Tennis no Ouji-sama 2004 titles (see "Library sweep" below). Per-title results:
  `docs/library-compat.md`. The sweep fixed five emulator bugs (fix logs dated 2026-10-01).
- **GUI (M9, 2026-09-30)**: the frontend is now the "paper, ink, one red" interface from
  `docs/gui/`: Library, in-frame pause menu with eight state slots, Save states, Settings (with key
  rebinding and a settings file) and the Bench developer view on new core capture hooks. See the
  M9 section below.
- **Compat**: the 29 priority titles pinned in `tests/compat.toml` all boot and 28 reach
  controllable gameplay under scripted headless runs (Mario Tennis: controllable overworld, its
  first match is behind a story cutscene); seven more rows pin the BIOS sound driver titles at
  1500 frames with a picture golden and an audio minimum. See the table below.
- **mgba-suite is built and gated** (`suite.gba` via its `docker-build.sh`; 13 sub-suites in
  `tests/golden.toml`, each pinned at its current score so any regression fails `cargo test`).
  Scores after the 2026-09-30 merges: Memory 1552/1552, I/O read 130/130, Timing 2020/2020,
  Timer count-up 936/936, Timer IRQ 90/90, Shifter/Carry/Multiply-long/BIOS-math all full,
  DMA 1244/1244, SIO R/W 90/90, SIO timing 4/4, Misc edge 6/12.
- **Frontend verified windowed (2026-09-30, after the GUI)**: KH:CoM, 300 frames, 59.8 fps
  average, 3 late frames, 0 audio underruns on a 48 kHz WASAPI device (`--exit-after 300`), at
  the 1280x800 design size with the 4x letterboxed picture and at `--scale 3`.
- **Open**: Misc edge 6/12 (see "Misc edge attempt"); KH:CoM's whole 27400-frame
  new-game run is FIFO-underrun-free apart from a handful at scene changes (was ~427k before
  the HLE service hook, see the 2026-09-30 fix log).

## Milestone status

| M | Scope | Gate | Status |
|---|---|---|---|
| M0 | Workspace, contracts, harness, test suites fetched | `cargo build --workspace`; harness runs every golden entry | **DONE 2026-09-29** |
| M1 | ARM7TDMI: all ARM+Thumb, modes, exceptions, flags | jsmolka `arm.gba`, `thumb.gba` | **DONE 2026-09-29**: 65 CPU unit tests; real 2-slot pipeline; mGBA-format trace |
| M2 | Memory map, waitstates, DMA, timers, IRQ, HALTCNT | jsmolka `memory.gba`, `bios.gba`, `nes.gba`; mgba-suite memory/timing | **DONE 2026-09-30**: jsmolka green; mgba-suite Memory 1552/1552, DMA 1244/1244, Timing 2020/2020 pinned (prefetch buffer + BIOS-derived HLE SWI cycle costs + ROM-to-ROM DMA) |
| M3 | PPU modes 0-5, OBJ, windows, blending, mosaic, timing | jsmolka ppu/* golden hashes; mgba-suite video | **DONE (jsmolka side) 2026-09-29**: goldens pinned, 56 PPU unit tests. Gaps: per-line OBJ cycle budget not modelled, OAM evaluated on the rendered line rather than one line early; mgba-suite video blocked as above |
| M4 | BIOS HLE, every listed SWI | `bios.gba` under HLE; per-SWI unit tests | **DONE 2026-09-29**: `bios.gba` passes; all listed SWIs + IRQ vector/return emulation; 14 unit tests incl. LZ77/RL/Huffman encoder round trips. Sound driver (1A-1E, 28, 29, 2A and its 36 routines) added 2026-10-01 with 45 tests. LLE differential deferred (no BIOS image) |
| M5 | Save detection, SRAM/Flash/EEPROM, `<rom>.sav` | jsmolka save/*; round trip | **DONE 2026-09-29**: none/sram/flash64/flash128 pass (flash entries need 400 frames: tests 10/11 program 64K bytes one command sequence each); 8 cart unit tests incl. EEPROM protocol; `.sav` persistence in the frontend |
| M6 | KH:CoM title -> new game -> first room | title golden + input script | **DONE 2026-09-29**: see "Where things stand". Other priority titles reach title/file screens; Minish Cap reaches gameplay (golden + script) |
| M7 | APU PSG + FIFO + mixing | PCM capture vs mGBA | **DONE (underrun half) 2026-09-29**: 70 APU unit tests; KH title capture (`--pcm`, 65536 Hz) has 0 underruns over frames 1200-1800, clean stereo music, no clipping. **Reference comparison done 2026-10-02** against libmgba with `scripts/compare-audio.py`: our FIFO and PSG scales follow GBATEK's maxima, mGBA sits about 3 dB lower (see "Reference audio comparison") |
| M9 | GUI: library, pause overlay, save-state slots, settings, bench (`docs/gui/`) | `cargo test --workspace` green with the core hooks off; goldens unchanged; KH `--bench 1200` within 2 % | **DONE 2026-09-30**: all five screens built from the design package and wired; 149 frontend tests incl. review renders; core capture hooks (22 new tests) with the speed gate met (see the M9 section) |
| M8 | Prefetch, timing polish, savestates, fast-forward | mgba-suite timing improves | **PARTIAL 2026-09-29**: savestates (feature `savestate`, ~429 KB, KH resume bit-exact vs straight run; F1/F3; harness `--save-state/--load-state`), Tab fast-forward, audio-clocked pacing, ~1900 fps headless release on KH. game-pak prefetch buffer landed 2026-09-30 (`bus/prefetch.rs`). **Deferred**: per-instruction timing refinement (no runnable gate) |

## Compatibility (headless, HLE BIOS)

| Game | Code | Save | Reached | Pinned |
|---|---|---|---|---|
| Kingdom Hearts: Chain of Memories (USA) | B8CE | SRAM_F 32K | Battle tutorial, controllable; music plays | title 1800, gameplay 27400 |
| Zelda: The Minish Cap (USA) | BZME | EEPROM 8K | Link's house with HUD, controllable | title 1800, gameplay 12000 |
| Zelda: A Link to the Past & Four Swords (USA) | AZLE | EEPROM 512 | Link's house after the telepathy intro, controllable (DOWN walks Link to the door) | title 1800, gameplay 18000 |
| Classic NES Series: Zelda (USA/EU) | FZLE | EEPROM (forced) | Overworld start screen with HUD, controllable (LEFT scrolls to the next screen) | title 1800, gameplay 2600 |
| Classic NES Series: Zelda II (USA/EU) | FLBE | EEPROM (forced) | Zelda's palace, controllable (LEFT walks Link out to the overworld) | title 1800, gameplay 4200 |
| Initial D: Another Stage (J) | AINJ | SRAM 32K | Story mode through name entry, car select, garage, to the Akina tutorial race; LEFT/RIGHT change the racing line | title 1800, race 27500 |
| Mario Kart: Super Circuit (USA) | AMKE | Flash 64K | Mushroom Cup race on Peach Circuit; LEFT/RIGHT steer (kart leaves the track without input) | title 1800, race 8100 |
| Crash Bandicoot: The Huge Adventure (USA) | ACQE | EEPROM | Level 1 Jungle Jam, Crash runs on RIGHT | title 2700, gameplay 6000 |
| Need for Speed Underground 2 (USA/EU) | BNFE | EEPROM | Custom circuit race at City Hall; LEFT/RIGHT steer into the buildings/barrier | title 4200, race 11700 |
| Double Dragon Advance (USA) | BDDE | none | Mission 1 garage, Billy walks on RIGHT | title 900, gameplay 1800 |
| Crash Bandicoot 2: N-Tranced (USA) | AC8E | EEPROM | Level 1, Crash runs on RIGHT | title 1800, gameplay 4200 |
| Crash Bandicoot Purple (USA) | BD4E | EEPROM | Hub level, Crash walks LEFT/RIGHT | title 1800, gameplay 6300 |
| Backyard Baseball 2006 (USA) | BCYE | EEPROM | First at-bat, swing selector moves on DOWN/LEFT | title 1800, gameplay 3900 |
| Cars (USA/EU) | BCAE | EEPROM | Race start, LEFT/RIGHT steer, A is throttle | title 1800, race 8100 |
| Final Fantasy I & II: Dawn of Souls (USA) | BFFE | SRAM 32K | Party outside Cornelia, UP enters town | title 1800, gameplay 12600 |
| Final Fantasy V Advance (USA) | BZ5E | SRAM 32K | Bartz on chocobo on the world map, LEFT/DOWN move | title 1800, gameplay 19000 |
| Final Fantasy VI Advance (USA) | BZ6E | SRAM 32K | Terra's Magitek armor in Narshe, UP/DOWN/LEFT move | title 3000, gameplay 15300 |
| Final Fantasy Tactics Advance (USA) | AFXE | Flash 64K | Snowball fight command menu, UP moves the cursor | title 4500, gameplay 19500 |
| Mario Party Advance (USA) | B8ME | EEPROM | Shroom City map, A stops the dice | title 3000, gameplay 32100 |
| Mario Pinball Land (USA) | BMVE | EEPROM | First table; A flips, LEFT nudges | title 3000, gameplay 3400 |
| Mario Tennis: Power Tour (USA) | BTME | SRAM 32K | Story mode through character setup and the dorm to the academy courts, Clay controllable (LEFT walks to the fence); no match reachable from power-on without save data (hung black before the 2026-09-30 fetch-arbitration fix) | title 1800, overworld 30600 |
| Max Payne (USA) | BMEE | EEPROM | Roscoe St. Station, LEFT/UP move | menu 1200, gameplay 2300 |
| NFS Carbon: Own the City (USA/EU) | BN7E | EEPROM | Elimination race, LEFT/RIGHT steer | title 2700, race 6900 |
| NFS Most Wanted (USA/EU) | BNWE | EEPROM | Tollbooth race, LEFT/RIGHT steer | title 1500, race 6600 |
| NFS Underground (USA/EU) | BNSE | EEPROM | Stadium circuit, LEFT/RIGHT steer | title 2100, race 7300 |
| Super Mario Advance (USA/EU) | AMAE | EEPROM | 1-1 on the cloud, RIGHT moves Mario | title 2400, gameplay 5700 |
| Super Mario Advance 2 (USA) | AA2E | EEPROM | Yoshi's House, RIGHT walks out to the map (level was invisible before the 2026-09-30 window fix) | title 1200, card 5300, gameplay 6300 |
| Super Mario Advance 4 (USA) | AX4E | Flash 128K | 1-1 start, RIGHT moves Mario | title 1800, gameplay 8100 |
| Final Fantasy IV Advance (USA) | BZ4E | SRAM 32K | Red Wings intro, scripted battles, Baron castle; Cecil controllable in the corridor (four directions verified). Its `swi 0xFF` is a logged no-op (GBATEK: 2Bh+ jump to garbage on hardware) | title 2400, gameplay 19800 |

Seven further rows (Atari Anniversary Advance, Lizzie McGuire - On the Go!, Pocket Professor,
Rampage - Puzzle Attack, X-Men - Reign of Apocalypse, Saibara Rieko no Dendou Mahjong, Cyberdrive
Zoids) run the sweep's input for 1500 frames and pin the title or menu picture plus a minimum
count of frames with audio (`audio_frames_min`): they gate the BIOS sound driver HLE (see its fix
log). Every other title in the 1,543-title library has a boot verdict in `docs/library-compat.md`.

## Environment facts
- Rust 1.95 stable-msvc; no devkitARM; no BIOS dump. HLE is the only boot path.
- Frontend stack: pure Rust `minifb` + `cpal` + `gilrs` (user decision).
- Reference emulator: portable mGBA 0.10.5 at `tools/mgba/mGBA-0.10.5-win64/` (no CLI debugger,
  only GDB `-g`, no headless audio dump) plus `tools/mgba-wav.exe`, libmgba 0.10.5 built from
  source by `scripts/build-mgba.ps1` (MinGW gcc, static, no dependencies; `scripts/mgba-wav/main.c`
  uses the public core API only) for headless audio captures. Trace diffing still needs a Linux
  build with the CLI debugger (`USE_EDITLINE` is non-Windows only).
- `tools/` was found empty on 2026-10-02 (wiped with the 2026-10-01 worktree incident); re-run
  `scripts/fetch-tests.ps1` and set `GBA_TESTS_DIR` or the golden ROM tests silently skip.
- Test ROMs: `tools/tests/gba-tests` (jsmolka, prebuilt). `tools/tests/suite` is mgba's suite source
  plus `suite.gba` built with its `docker-build.sh` (devkitARM in Docker). Without that file the
  13 suite entries in `tests/golden.toml` fail; rebuild it with the script if `tools/` is wiped.
- Frontend audio/pacing verified on 2026-09-30 with a real 48 kHz device (see "Where things stand").
  Hidden flag `--exit-after N` ends a windowed run after N frames and prints timing stats.

## Pass/fail conventions encoded in the harness (verified from suite sources)
- **jsmolka**: ROM ends spinning on `b .` (ARM `EAFFFFFE` / Thumb `E7FE`) after `m_test_eval`,
  which copies the result into **r12**: 0 = all passed, else the failing test number
  (`lib/macros.inc`; thumb.gba uses r7 as the source but r12 receives it). Harness: `--check jsmolka`.
- **mgba-suite**: menu-driven; results via mGBA debug registers (`0x04FFF600` text, `0x04FFF700`
  flags with bit 8 = flush, `0x04FFF780` = 0xC0DE enables, reads 0x1DEA) as `END: p/t`.
  Harness: `--check mgba-suite --input <script> --expect p/t`. Debug regs implemented in `bus/debug.rs`.

## Known gaps / judgement calls
- Savestates: a reported Mario Kart save/load divergence at the character select did not
  reproduce: a state saved at 3900 and resumed with the script shifted by 3900 frames matches
  the from-zero run's hash at 4200, 4500, 4800, 4810 and 4900 (the reported divergence point).
  The original chain most likely mis-offset its inputs. Note the harness restarts the input
  script's frame numbers at 0 after `--load-state`.
- Mario Kart Peach Circuit at frame 8400 with the kart off the track: the grass shows a shaded
  blue/yellow/red diagonal ribbon scrolling with the Mode 7 ground. Possibly the course's flower
  strip; unverified against hardware.
- Initial D (garage, script frame ~22900 of `tests/inputs/initiald-race.toml`): the phone screen
  (A) draws a blank white panel with only the signal/battery icons, and the driving-technique
  screen (DOWN x2, A) has its content area solid black. Unverified against hardware; could be a
  missing text layer or a blend/window case. Classic NES Zelda: on the SELECT screen after
  registering a name (frame 2000) an extra "O" appears one row below the "A"; unverified.
- PPU: no per-scanline OBJ cycle budget; OAM evaluated on the rendered line; forced blank does not
  advance affine references (mGBA-like); window `x1 > 240 && x1 > x2` covers the whole line (mGBA).
- APU: PSG and FIFO scales follow GBATEK's "Max Output Levels" (FIFO +-200h, each PSG +-80h);
  not measured against hardware. mGBA 0.10.5 mixes about 3 dB lower overall and its PSG another
  3.5 dB lower relative to FIFO (see "Reference audio comparison"); kept ours. Wave RAM
  shift-register read-back quirk not modelled; duty position not reset on restart.
- CPU: MUL/MLA leave C unchanged (GBATEK says "destroyed"); only Div/DivArm/Sqrt/ArcTan/ArcTan2/
  CpuSet/CpuFastSet have real BIOS cycle counts (other HLE SWIs still pay a flat 10, and the
  IntrWait exit cost of 31 plus the IRQ return stub's 2 idle cycles are fitted to the Timer
  count-up rows, not derived). Prefetch buffer: the data-access stall rule is empirical
  (see the prefetch fix log) (ROM-to-ROM DMA timing was fixed later, see the ROM-to-ROM fix log).
- BIOS HLE: BgAffineSet/ObjAffineSet use f64 trig instead of the BIOS sine table (tiny rounding
  differences possible); division by zero returns mGBA's values instead of hanging. The sound
  driver's mixer is a design of ours (output scaling, reverb, pseudo-echo, fixed-rate voices,
  next-slot write; see its fix log); SWI 20h-24h (SoundWhatever0-4) stay unimplemented, and
  `ply_port`, FadeOutBody's fade-in and TrkVolPitSet are fallbacks no title seen depends on.
- Cartridge: no GPIO/RTC (none of the target games use it); SRAM open-bus when the backend is EEPROM
  reads 0xFF.
- Audio: HLE SWIs still complete atomically with respect to IRQs (an IRQ raised during a BIOS
  routine is taken when the SWI returns rather than mid-routine, as the real BIOS allows).
  ALttP shows 1910 over 18000 frames, Zelda II 43452 over 4200 and Zelda 1 65242 over 2600.
  The Classic NES ones are the games' own behaviour: their driver refills FIFO A from a timer-1
  IRQ (immediate DMA1 of 4 words, then re-arm as 16-bit special DMA), and during menu
  transitions they mask that IRQ (IE=2041 with the timer request pending in IF, DMA1 left
  disabled), so the FIFO starves on hardware as well. Traced with `--watch-log` on 2026-09-30.
  Backyard Baseball 2006 is the same pattern (28k underruns in frames 1832-1900 after START on
  the title: the game writes DMA1/2CNT_H = 0400 and polls VCOUNT while loading, timer 0 still
  running; `--fifo-stats` shows pushed=0 for both FIFOs in that window). FF Tactics Advance and
  Crash Purple log theirs only inside their intros.
- Harness frame index vs `frames=`: `gba-test` runs N `run_frame` calls, each ending at the next
  V-Blank *event*; an HLE decompression call that spans two V-Blanks (large LZ77 into VRAM) sets
  frame-ready twice inside one instruction, so `frames=` prints N+k for such games (FFTA, FF I
  in the overworld: N+2). Both frames are rendered; only the input-script frame numbers drift by
  k. Harmless for goldens (deterministic), noted so nobody chases it.

## HLE SWI timing attempt (2026-09-30): succeeded
mgba-suite "Timing tests" 1816/2020 -> 1956/2020; every other suite row unchanged (Memory 1552/1552,
I/O 130/130, Timer count-up 582/936, Timer IRQ 90/90, Shifter 140, Carry 93, Multiply long 72,
BIOS math 615, DMA 1244/1244, SIO 90/90 and 0/4, Misc edge 4/12). All BIOS Division/Division 2/
Sqrt/Sqrt 2/Sqrt 3/ArcTan/CpuSet rows pass in every column; the 64 left are the ROM-to-ROM DMA rows.
Method: the public disassembly (camthesaxman/gba_bios `asm/bios.s`) gives the exact instruction
sequences; `bios/swi_timing.rs` counts them with ARM7TDMI cycle rules (BIOS fetches 1 cycle, taken
branch 3, LDM n+2, STM n+1, MUL 1+m with Rs early termination) and simulates the data-dependent
loops (Div's shift/subtract loops, Sqrt's Newton iterations with the same divide loop, ArcTan's MUL
operands). Nothing was fitted: e.g. Div 12345678h/FFh vs FFh/12345678h differ by exactly 260 cycles
in the suite and in the simulated loops.
- Fixed entry/exit of the SWI handler (0x08 vector -> 0x140 handler: push, `ldrb` comment, table
  `ldr`, mode switch, `bx r12`, ... `movs pc, lr`) is 45 BIOS cycles (`SWI_FIXED`), replacing the
  flat `idle(10)` for the modelled SWIs only. Everything else (Halt, IntrWait, LZ77, ...) keeps 10.
- Return: the CPU now refills the pipeline at the return address after a modelled SWI (the
  `movs pc, lr`), paying the caller region's fetch N+S.
- The handler's `ldrb r12, [lr, #-2]` really reads the comment byte from the caller's memory through
  the bus, so its cost follows the region (ROM S/N, EWRAM 3, IWRAM 1).
- **Empirical bit (not derived)**: the first refill fetch after the return is sequential when the
  game pak prefetch buffer is off (the ldrb at lr-2 is the preceding halfword) and non-sequential
  when it is on. The P columns of the suite need exactly N-S extra cycles; `Bus::swi_took_refill`.
  The BIOS fetches also drop the prefetch buffer (`pf.invalidate()` at SWI entry).
- CpuSet/CpuFastSet: setup/loop/exit instruction counts from the listing (halfword copy loop 9
  BIOS cycles per unit, word 8, fills 7/6, FastSet block 7 or 5) plus the real bus data cycles
  (now N for the first word of a burst and S after, previously all S). Fill mode reads the source
  once, as the BIOS does. ArcTan2 (Thumb, quadrant paths around one Div and one ArcTan) and
  CpuFastSet have no suite row: their counts are the same method, unit-tested for loop growth only.
- No residual fitted constants. Division by zero (real BIOS hangs) charges the shortest finite path.
- Compat: KH title (frame 1800) and Initial D race prompt (frame 27500) shifted; PNGs reviewed
  (PRESS START title; Akina tutorial with the LIMIT countdown and line prompt), re-pinned. All other
  compat goldens unchanged.
## Fix log (Timer count-up 2i rows, 2026-09-30)
mgba-suite Timer count-up 582 -> 936/936 (Misc edge 4 -> 6/12 as a side effect: H-blank flips 4-6 now
match; every other suite row unchanged, jsmolka unchanged). The "354 remaining 2i rows" in the
previous notes were really two independent families:
- **216 rows (every 6b/8b/10b prescaled test, sums only)**: the test synchronises on a timer IRQ
  aligned to the free-running prescaler grid (IntrWait released by a 1024-divider timer) and then
  starts the prescaled timer, so the row pins the cycle count of the BIOS path between the IRQ and
  the caller modulo 64/256/1024. The HLE charged 10 cycles for the whole IntrWait wake-up (flat
  SWI overhead); the real BIOS SWI entry, flag check and `movs pc,lr` cost considerably more.
  `INTR_WAIT_EXIT_CYCLES = 31` is added when IntrWait completes. **This constant is fitted**, not
  read from a BIOS dump: scanning it over 0..64 (with the return-stub idle over -3..4) the
  216 rows are matched only at 31.
- **138 rows (prescale 1, 2i/4i, plus a few 1i)**: the short-period rows have the next IRQ pending
  when the first handler returns. The ARM7TDMI finishes the exception-return instruction
  (`subs pc,lr,#4`: ALU cycle plus the N+S pipeline refill at the return address) before it
  samples nIRQ, so the pending IRQ is taken after the refill and before the instruction at the
  return address runs. `Cpu::step` used to leave the refill for the next step and sample the IRQ
  first, so the refill was skipped entirely (3 cycles short in the tight loop, the "2 high /
  3 low" pattern); a branch whose refill overlaps the IRQ assertion was likewise one instruction
  late (the `0x0040 16xv 1i` row). Now every PC-writing step ends with the refill. Answer to the
  original question: the instruction at the return address does NOT execute before the pending
  IRQ; the difference is purely the refill cycles that the return instruction owns.
- Return stub idle: with the refill moved into the step, the HLE stub (`ldmfd` + `subs`) charges
  `idle(2)` (was 3). Accounting from the ARM7TDMI cycle tables says 3 (subs ALU cycle); the
  hardware-recorded rows want 2, so the ALU cycle is treated as overlapping the ldmfd internal
  cycle. Also fitted (unique: the full 936 was reached for exactly one (stub idle, exit cycles)
  pair in the scanned grid).
- Side effects: a halted CPU now refills its pipeline before sleeping rather than after waking
  (same cycles, different position relative to the wake event). Initial D race golden re-pinned
  (c2b3e7c8fdc7ca59 -> ff210aefde06663c; PNG reviewed, same screen with the LIMIT countdown a tick
  apart); no other compat golden moved.
- Tests: `irq_pending_at_exception_return_pays_the_refill_then_preempts_the_target`,
  `irq_after_a_branch_preempts_its_target` (cpu/tests/exceptions.rs);
  `alu_write_to_pc_branches_and_aligns` now asserts the refill happens inside the branch step.

## Fix log (stretch compat bugs, 2026-09-30)
- **Window with both edges past 240 wrapped over the whole line.** Super Mario Advance 2 parks
  window 0 with WIN0H=FEFFh (X1=254, X2=255) from its H-Blank handler; `apply_window` clamped X2
  to 240, left X1 at 254 and took the wrap branch, masking every pixel with WININ=0 so the level
  showed only the backdrop. GBATEK: "garbage values of X2>240 or X1>X2 are interpreted as
  X2=240"; X1 past the screen is clamped with it (mGBA does the same), giving an empty window.
  Test `window_with_both_edges_past_the_screen_is_empty`.
- **Immediate DMA vs opcode fetch.** Mario Tennis: Power Tour DMA3-copies a routine from ROM onto
  the stack and branches to it in the next instruction; hardware starts the DMA 2 cycles after
  the enable and stalls the CPU, so the code is there before the fetch. `Bus::arbitrate` ran
  only for data accesses, so the CPU fetched stale stack words, hit an undefined instruction and
  ended in the game's cart-removal loop (black screen, SWI 3 forever). Fetches from RAM/BIOS now
  arbitrate too; fetches from the game pak do not (DMA cannot write ROM, and the prefetch-buffer
  model is calibrated with the DMA landing at the next data access: arbitrating ROM fetches
  moved 32 "Trivial DMA" P-column rows by one cycle). Test
  `due_immediate_dma_takes_the_bus_before_an_opcode_fetch`.
- Super Mario Advance's ~4.4k FIFO underruns are bursts at level transitions (e.g. frames
  2801-2803: DMA1/2CNT_H written 0000 by the game with IME=0 while timer 0 keeps popping;
  `--fifo-stats` shows both channels disabled and nothing pushed), i.e. game-driven like the
  Classic NES and Backyard Baseball cases. SMA4's 117k during its flash format not checked but
  presumably the same.

## Fix log (merge of the SWI-timing and timer branches, 2026-09-30)
Merged separately each branch held its rows; together the Timing row dropped 1956 -> 1900: the
timer branch's `Cpu::refill_pipeline` (refill inside every PC-writing step) always charged the
first refill fetch as N and never consumed the SWI branch's `refill_seq` flag (BIOS `movs pc, lr`
return is S in the caller's region with prefetch off), so every BIOS row in every ROM column was
N-S cycles slow. `refill_pipeline` now honours the flag; pinned by
`swi_hle_return_refill_honours_the_sequential_flag`. All rows back at their pins.

## Fix log (IRQ entry cycles, 2026-09-30)
mgba-suite Timer IRQ 69 -> 90/90, Timer count-up 483 -> 582/936. Every remaining Timer IRQ
failure was the pre-empted case (timer value latched inside the handler 4 cycles low). The HLE
IRQ vector charged a flat 6 idle cycles; it now itemises the real BIOS path (vector fetch pair,
`b 128h`, stmfd, mov, add, `ldr r15,[r0,-4]`, less what the writes/read/refill already pay) and
the return stub likewise (ldmfd, `subs r15,r14,4`). That left exactly one cycle, charged at
exception entry as an IRQ sampling cycle on top of the ARM7TDMI's 2S+1N; the suite's
hardware-recorded values pin it.

## Fix log (unaligned loads on the SRAM bus, 2026-09-30)
mgba-suite Memory 1544 -> 1552/1552. `Cpu::load_word`/`load_half` masked the address before
handing it to the bus, so `LDR [sram+1]` fetched byte 0. The ARM7 puts the unaligned address on
the bus; 16/32-bit memories ignore the low bits, the 8-bit SRAM bus serves the selected byte on
every lane, and the CPU's rotation then acts on that. The bus already aligned for every other
region, so only the two CPU helpers changed.

## Fix log (HLE BIOS vs. sound DMA, 2026-09-30)
KH:CoM logged ~427k FIFO underruns over its new-game run, all in the post-title intro
(frames 1800-12000, ~55 per frame). Traced with the new harness `--watch-log` flag: the game's
driver restarts DMA1 (SAD = EWRAM PCM buffer, DAD = FIFO_A, CNT_H = B640) once per frame at line
160 and timer 0 runs at 13379 Hz, all correct. The loss was in the HLE BIOS: LZ77UnCompVram /
CpuFastSet calls decompressing the cutscene graphics ran as one atomic "instruction", so every
timer overflow (FIFO pop) that fell inside the call was dispatched in a burst afterwards and the
refill DMA could not run until the SWI returned; the FIFO drained on every long call. On hardware
the DMA halts the CPU at whatever bus cycle the request arrives. Fix: `Bus::hle_service`, called
once per unit inside every HLE copy/decompression loop, dispatches due events and runs pending
DMA. Intro windows: 230154 -> 1 and 196520 -> 2 underruns. Pinned by
`cpu_fast_set_services_fifo_dma_while_copying` (fails with the hook disabled).

## Fix log (game pak prefetch buffer, 2026-09-30)
mgba-suite "Timing tests" 1212/2020 -> 1816/2020; every other suite row unchanged (Memory 1544/1552,
I/O 130/130, Timer count-up 471/936, Timer IRQ 69/90, Shifter 140, Carry 93, Multiply long 72, BIOS
math 615, DMA 1244/1244, SIO 90/90 and 0/4, Misc 4/12). No variant that passed before fails now.
Every Timing row with prefetch on passes except BIOS Div/Sqrt/ArcTan/CpuSet (HLE SWIs cost no
cycles) and the ROM-to-ROM DMA rows (fail in every column, pre-existing).
- `bus/prefetch.rs`, lazy model (no per-cycle stepping): head address, completed halfwords (max 8)
  and the completion time of the in-flight one. A ROM opcode fetch at the head costs 1 cycle if the
  halfword landed, else the wait for the in-flight one; anything else is a normal N/S access that
  restarts the buffer behind it (first halfword S cost, the rest back to back). The prefetcher
  stops at a 128K block edge. `Bus::fetch_cost` routes ROM fetches there only when WAITCNT bit 14
  is set; a fetch from RAM/BIOS, a WAITCNT write clearing bit 14 or any game pak data access
  (ROM or SRAM, including DMA) drops the buffer. `Cpu::next_fetch_n` and the CPU contract are
  untouched (with the buffer on the bus decides by address and ignores the flag on a hit).
- A 32-bit ARM opcode is handed over in one cycle once its second halfword has landed
  (`pf_take_n`); charging one cycle per halfword left every ARM internal-cycle row (MUL, long MUL,
  LDM/STM) one cycle slow.
- A halfword the prefetcher would start on the very cycle the CPU wants the bus loses the
  arbitration (no stall). Otherwise a CPU data access to the pak waits for the in-flight halfword
  only when it is in its last cycle (remaining <= 1); with two or more cycles left it cancels it
  for free (LDM running off OAM into ROM, P.. and PN. columns). A DMA access always waits one
  cycle for a halfword under way (DMA to ROM, same columns). Neither rule is derived from a bus
  diagram: they are the smallest rules that satisfy every Timing row, and a blanket "wait for the
  whole halfword" (GBATEK's wording) fails the LDM-to-ROM rows.
- Savestate version bumped to 3 (new `Bus::pf`). Unit tests: `bus/prefetch_tests.rs`.
- Compat goldens KH gameplay (frame 27400) and ALttP file-select (frame 1800) shifted with the new
  timing; PNGs reviewed (KH battle tutorial with control, ALttP CHOOSE A FILE), re-pinned.
- Headless speed on KH (`--bench 1200`): 1847 fps before, about 1600-1640 after (the buffer adds
  work to every ROM opcode fetch; 1808 with WAITCNT bit 14 forced off, the rest being the extra
  checks on the fetch and data paths).

## Fix log (timers and DMA, 2026-09-30)
Two parallel agent branches, merged after each passed its unit tests and improved its suite rows
with no regression elsewhere (jsmolka, other suite rows, commercial goldens all unchanged):
- **Timers** (`timers.rs`, `irq.rs`): the prescaler is a free-running divider of the system clock
  (phase not reset on start); a 0->1 start write loads the reload and counts from `W + 2`; a
  TMxCNT_L write on the same clock edge as an overflow is not seen by that overflow; a control
  write landing on a due-but-undispatched overflow is applied after it; an overflow's IRQ reaches
  the CPU 3 cycles after IF is set (`IrqController` now keeps a CPU-side `line` separate from
  `if_`, and an ack before the line asserts drops the request). Timer count-up 370 -> 471/936,
  Timer IRQ 0 -> 69/90.
- **DMA** (`dma.rs`, `bus/dma_exec.rs`, `bus/mod.rs`): each channel keeps a read latch; reads
  from 00000000-01FFFFFF (where DMA0's 27-bit source puts ROM addresses) return the latch instead
  of the CPU open bus; a game-pak source increments whatever the source control says; an
  immediate start whose 2-cycle latency elapsed before a CPU data access takes the bus first
  (`Bus::arbitrate`). Memory 1205 -> 1544/1552, DMA 1056 -> 1244/1244, Timing +130.
- **CpuSet/CpuFastSet HLE**: reject any source below 02000000h (not just below 4000h); pass
  unaligned addresses through with LDRH rotation / LDM non-rotation semantics; SRAM byte lanes.
- Two compat goldens (KH gameplay 27400, ALttP title 1800) were already stale on the pre-merge
  tree (the CPU-timing and IO merges shifted them without a re-pin); verified against a baseline
  build of 4e25e6f, PNGs reviewed, re-pinned.

## Fix log (CPU timing, 2026-09-30)
mgba-suite "Timing tests" 408/2020 -> 1082/2020. Every variant without prefetch now matches
hardware except the timer-start offset, HLE BIOS calls and DMA-during-fetch (all outside `cpu/`).
- Opcode fetches after a data access or an internal cycle were passed as `S`. The ARM7TDMI drives
  SEQ low after a data cycle (STR/STM = "2N"), and the GBA charges N for the fetch after an I
  cycle when prefetch is off (GBATEK "prefetch disable bug"). `Cpu::next_fetch_n` now records
  "execute advanced the clock" and the next pipeline fetch is `N`. The unrolled C loop in ROM
  (17 str + 16 ldr, 2 cycles each) was exactly the 66-cycle shortfall.
- LDM/STM running off the end of OAM into 08000000h charged the first ROM word as `S`; the GBA
  times the first halfword of every 128K game pak block as non-sequential (GBATEK WAITCNT notes).
  Block transfers and pipeline fetches restart at `N` there.
- Savestate version bumped to 2 (new `Cpu` field).

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

## GUI milestone (M9, 2026-09-30)
Built from the user's design package (`docs/gui/`: DESIGN.md, SCREENS.md, tokens.json, six
mockups, three SVG drawings) as a software renderer on the existing minifb window (tiny-skia +
fontdue, all pure Rust; fonts are the OFL faces embedded from `gba-frontend/assets/fonts/`).
Decisions taken with the user: no egui; Esc opens the pause menu (quitting is the menu's QUIT,
the window close button, or Esc on the Library); the Bench in full including core hooks.
- **Screens.** 01 Library (`library/`, `screens/library.rs`: folder scan thread, display names
  from file names, TOML index cache, background title captures for the cartridge labels), 02
  Paused (`overlay/`: drawn at native 240x160 into a frame copy, Pixel palette, 5x5 bitmap font,
  BLDY-style dim), 03 Save states (`screens/states.rs`: film strips at 1x, grease-pencil ring,
  loupe, delete dialog), 04 Settings (`settings.rs`, `screens/settings.rs`: the open manual with
  key rebinding, three spreads, settings file in the config dir with unknown keys preserved and
  CLI flags overriding for the run only), D Bench (`screens/bench.rs`). `app.rs` is the state
  machine; `ui/` the toolkit (tokens checked against tokens.json by a test).
- **Slots.** Eight per game, `<rom stem>.s<N>.state` beside the ROM, one file each with a header
  (frame, time, play seconds, core state version, 240x160 thumbnail) and the core state; the old
  `<rom>.state` is imported into slot 1 once. Format in `docs/ARCHITECTURE.md`.
- **Core hooks** (`ppu/capture.rs`, `apu/capture.rs`, `bus/bench.rs`, `DebugHooks::bench`): per-layer
  BGR555 planes captured per scanline as drawn, OBJ drawn/active counts, per-channel peaks, layer
  and channel mutes applied only to what is composed/mixed, a 228-line event log (VBlank IRQ, FIFO
  DMAs, DMA3, Timer 0 overflow). Off by default; one `is_some()` per line/sample/event. Nothing in
  savestates. 22 unit tests. Speed gate (KH:CoM `--bench 1200`, release, quiet machine):
  interleaved A/B against a binary built from the pre-hooks commit 65b50b5, four runs each: baseline 1154/998/1098/1038 fps (mean 1072), with hooks 1105/1083/1042/1085 fps (mean 1079), i.e. +0.6 %, within noise; the morning's 1565 fps baseline was taken on a quieter machine and is not comparable to the evening runs. Framebuffer hash unchanged (6872ee2c47e4c346).
- **Review renders.** `cargo test -p gba-frontend --release -- --ignored render_` (with `GBA_ROM`
  and `GBA_GAMES_DIR` set) writes every screen to `target/ui-renders/`; each was compared with its
  mockup by eye at integration.
- **Deviations from the design, and why.** No native file/folder dialogs (pure Rust, no OS
  dialog crate): folders and the BIOS path are typed into underline fields in Settings; F2 opens
  the Save states screen (the design names no key); in-game F1/F3 act on the "current slot" (the
  last slot used in any menu, initially the newest); `PLAYED h:mm` replaces the mockup's "Runs"
  filler row; the gamepad map is rebound beside the keyboard keycaps (2026-10-02; the design
  offered a second spread or a toggle); the Library shows 2 columns below ~1114 px
  (3 would need a scaled drawing); the cart label art is a cover crop of the title capture; Find
  needs `/`, Tab or a click to take letters that are bound to GBA buttons; the Bench's layer
  thumbnails are exact 1/3 samples (80x53) instead of the mockup's fractional size; the pause
  bar reads `A OVERWRITE  B BACK` with the GBA button names; Settings adds the F2 row to the
  emulator-keys list; the Data face has no `▸`, so the film edge print draws the triangle as a path;
  the empty Library is the Strawberry brand screen (`docs/branding/boards/05-…`, 2026-10-02)
  instead of SCREENS.md's "No games found" paragraph, with `Open <folder>` / `Settings` buttons
  (Settings was reachable only from the pause menu before) and the whole `Settings` button when
  no folder exists; the board's handheld screen is composed at native 240x160 in BGR555, so its
  cream is the Pixel cream, not the Night chrome cream.
- **Not done.** The window has no
  minimum size enforcement (layouts clip below 960x640); thread priority for the capture thread
  (std has none; it runs one game at a time with pauses).

## Library sweep (2026-10-01): 1,543 titles
`scripts/triage.py Games` runs every ROM for 3000 frames under HLE with Start pressed in pairs
(700/800, 1300/1400, 1900/2000, 2500/2600) and A at 1150/2350, classifies the final picture, its
motion, the audio and the CPU state, and writes `tools/triage/triage.csv` (+ a PNG per title) and,
with `--doc`, `docs/library-compat.md`. Verdicts for the non-ok rows are kept in
`tools/triage/notes.csv` (gitignored with the PNGs) and merged into the document.

Emulator fixes that came out of it (details in the dated fix logs below):
- FIFO underruns counted every pop of a routed-but-never-fed FIFO (an artefact that read as
  hundreds of thousands per title); the counter now counts a routed stream running dry.
- LCD IRQs (H/V-Blank, V-Count) reach the CPU line `LCD_IRQ_DELAY` = 3 cycles after the flag, like
  the timers: games that poll VCOUNT with a long V-Blank handler (Twin Series 5, Majokko Cream-chan,
  James Pond, Winter Sports) could never see line 160. One compat row moved and was re-pinned after
  PNG review (NFS Most Wanted race, a traffic car one step further).
- Timer 0 ignores the count-up bit (GBATEK: not used in TM0CNT_H): the three Who Wants to Be a
  Millionaire titles clock their FIFO from TM0 with bit 2 set.
- IntrWait parks with r12 = 0x04000000 as the real BIOS does and restores the caller's: Taito's
  dispatcher sets the flag mirror through r12 (Bubble Bobble / Elevator Action Old & New).
- SoundGetJumpList (SWI 2A) fills its table with a returning stub instead of zeros, so m4a games
  no longer branch to address 0 (Saibara Mahjong, Cyberdrive Zoids). Superseded the same day by
  the full driver (see "Fix log (BIOS sound driver HLE)"): the six silent titles play.
Open: Hello Kitty (uninitialised heap walk through BIOS open-bus values; mGBA escapes the loop),
Tennis no Ouji-sama 2004 x2 (an uninitialised local corrupts the game's IWRAM mixer; depends on
IRQ phase).

## Next steps, in order of value
1. Hello Kitty Collection and the two Tennis no Ouji-sama 2004 titles (see "Library sweep").
2. Misc edge H-blank bit / DMA prefetch rows (6/12, see "Misc edge attempt").
3. Initial D garage screens (phone, driving technique) render blank content areas: verify and fix
   (see Known gaps). Mario Tennis: the first match is behind the story's center-court cutscene;
   not scripted.
4. Boot-path timing against mGBA: the same ROM and inputs start their music 0.02-0.43 s earlier
   under mGBA's HLE BIOS on several titles, and sounds triggered by an input land one frame apart
   (see "Reference audio comparison"). A trace of the first frames would show which BIOS routine
   or reset state differs. The BIOS sound driver's own scaling has no reference (mGBA's HLE BIOS
   stubs it).

## Pinned frontend output (2026-10-02)

Step 0 of moving the generic frontend code into the shared `fruit-basket` crates (`basket-ui`,
`basket-app`; public repo `0x000NULL/fruit-basket`, shared with the crabapple NES emulator). Before
anything moves, `gba-frontend/src/pinned.rs` pins the frontend's output so the move can be proved
byte-identical: xxh3-64 hashes of 17 screen renders (library x5, states x4, settings x6, bench x2,
960x640 variants included), the pause overlay (main/save/load/page 2/message/toast and
a thumbnail), the window's BGR555 LUT, x4 letterbox, LCD grid and frame blend, and a slot file
with the version string cut out; plus the full text of `settings.toml` (with an unknown key and
table kept and a bad value falling back) and `index.toml`. 29 hashes, two texts.
Every input is synthetic (`pinned::test_frame`, fixed paths, no `GBA_ROM`/`Games/`), and
`fmt_when` reads a per-thread frozen UTC clock in tests (`ui::fmt::test_clock`), so the pins hold
on any host and time zone: identical across three debug runs, a release run, and a Linux
container (`rust:1`, TZ=America/New_York) like the CI test job. A failing pin prints the new hash;
`STRAWBERRY_PIN_RECORD=1 cargo test -p gba-frontend pinned_ -- --nocapture` prints every row and
writes the pinned canvases to `target/ui-renders/pinned/*.png` for review. Re-pin only after
looking at them.

## fruit-basket split (2026-10-02)

Step 2: the generic half moved out to the new public repo `0x000NULL/fruit-basket` (MIT,
fresh history: its first commit is the moved code, so nothing from this repo's past or this
machine went with it). Crates: `basket-ui`, `basket-app`, and `basket-build` (the exe-icon
helper; a build-dependency of its own so build scripts do not compile cpal and minifb).
`main.rs` re-imports them under the old module names, so the app's paths did not change.
Moved with them: `docs/gui/tokens.json` (now `basket-ui/tokens.json`), `pencil-ring.svg` and the
fonts with their OFL texts. New in basket-app: `pads` (the gamepads, from `input.rs`, plus
`Ports::Assigned` for two-port consoles: listed UUIDs first, then connection order;
`uuid_text`/`parse_uuid` for a settings list). `FILM_FRAME_W/H` and `THUMB_W/H` became
`component::film_frame(screen, scale)` and `pixel::thumb_size(screen, divisor)`; Strawberry
passes 1 and 5. The orphan rule made `glue::Machine` (a wrapper around `&mut Gba`) carry the
`SaveRam`/`Snapshot` impls. Unused direct dependencies dropped (fontdue, cpal, chrono,
raw-window-handle). Every pin unchanged; the release exe still carries its icon and version
block. Frontend tests: 98 here + 86 in fruit-basket.

## Generic frontend, in place (2026-10-02)

Step 1 of the fruit-basket move, one subsystem per commit with every pin in `pinned.rs` green
throughout (no pin changed): canvas blits through a colour table; `ui::Screen`; frame rate as a
parameter of `frames_to_secs`; `audio::Buffering`; `saves::SaveRam`; `library::Platform` (one trait
for probe, slot art and title capture, since the art thread needs the console too); `ButtonSet`
key and pad maps; `prefs.rs` (settings-file mechanics, name-based bindings); `slots::SlotFormat` +
`Snapshot`; `brand.rs` and `icon::Icon`. The generic modules (see ARCHITECTURE "Generic and GBA
halves") now reference no GBA crate and no app module. Agreed with crabapple: button masks stay
`u16` (8 buttons per NES port), `RomInfo.code` carries the NES CRC32, so no separate hash field.
Behaviour changes, all invisible to Strawberry's files: `names` drops any 1-4 character
alphanumeric extension (was `.gba`/`.zip` only; the scan lists only those, so titles are
unchanged); a panicking ROM probe now lists the file as unreadable instead of ending the scan
(before, only the archive read was guarded); the slot-path fallback stem for a ROM without one is
`game`. Still to do for step 2: a `build.rs` helper for the exe icon (a build-dependency cannot
come from the app crate), pad-to-port assignment for two-port consoles (basket-app), and the
`tokens` items sized for the GBA (`FILM_FRAME_W/H`, `pixel::THUMB_W/H`), which NES screens will
need to override.

## Gamepad rebinding (2026-10-02)

The Settings Controls page shows two keycaps per GBA button: the keyboard key and the pad
button (`ui::input::PadMap`, one shared map for every connected pad; the left stick is always a
D-pad). Clicking the pad keycap waits for the next pad button (`Gamepads::poll` now keeps the
`ButtonPressed` events it used to drain, carried in `UiInput::pad_buttons`); a button already
used by another GBA button swaps, and both keycaps flash. Keyboard waits ignore pad buttons and
vice versa. The map is stored by button name as `[gamepad]` in settings.toml (gilrs's
`LeftTrigger` is the bumper, shown as LB; the analog trigger is LT); files without the table
default, unknown names keep the default button. Narrow pages (below about 0.75 figure scale)
stack the pad keycap under the keyboard one and lay the arrows out two per row. Tests: `PadMap`
default/swap/names, settings round trip and bad-name defaulting, the screen's pad rebind flow,
and the review renders (`04-settings*.png`).

## Reference audio comparison (2026-10-02)

Tooling: `scripts/build-mgba.ps1` builds `tools/mgba-wav.exe` (libmgba 0.10.5, `scripts/mgba-wav/`
is ours and uses the public core API: `mCoreFind`, `setKeys`, `runFrame`, `blip_read_samples`);
`scripts/compare-audio.py` records each title three times with `gba-test --pcm` (full mix, PSG
only and FIFO only through the new `--mute-channels` flag) and once with `mgba-wav` at the same
rate and inputs (`tests/inputs/*.toml` converted to a `frame mask` schedule), aligns by
cross-correlation and fits the reference against our two components. Report:
`tools/audio-ref/report.md`. Both sides run their HLE BIOS, so the six BIOS-sound-driver titles are
silent under mGBA (Cyberdrive Zoids is the sanity row) and cannot be compared.

Results (11 captures, 1500-4000 frames, 8 kHz low-pass for the level figures):
- **FIFO**: mGBA's level is 0.70-0.75 of ours on every title with a ROM-side mixer (Castlevania
  Circle of the Moon correlates 1.00 with residual 0.01 at 0.74; the m4a titles 0.72; Kingdom
  Hearts 0.62-0.68, its stream has more content near the top of the band where our zero-order
  hold and blip_buf's band-limiting differ). GBATEK: "each of the two FIFOs can span the FULL
  output range (+/-200h)", which is what we do (sample x4 at 100 %); mGBA leaves headroom.
- **PSG**: phase-insensitive power fits give 0.40-0.52 of ours (Golden Sun's PSG-only seconds:
  0.51), i.e. relative to FIFO mGBA's PSG is 0.69x ours, about 3.5 dB quieter. GBATEK: "each of the
  four PSGs can span one QUARTER of the output range (+/-80h)", which is our `level*(vol+1)*2`
  (max 240 of 1024 p-p); mGBA's figure corresponds to one eighth. Without a hardware measurement
  the spec wins: no change. Sample-exact PSG fits are low (0.05-0.24) because the square waves'
  phase differs between the emulators; the envelopes agree (power residual 0.05-0.07 on Mario
  Kart and F-Zero).
- **Clocks agree**: once aligned, the per-2-second lag is 0-2 samples on every title (no drift).
- **Start offsets**: the music begins earlier under mGBA by 965 samples (KH title), 5148 (F-Zero),
  9392 (Classic NES Zelda), 11926 (Minish Cap), 16500 (ALttP) and 28033 (Advance Wars) at
  65536 Hz, i.e. 15-430 ms, while Circle of the Moon and Golden Sun start 9-15 ms earlier in
  ours. Not an audio effect: the games' boot paths take different emulated time under the two
  HLE BIOSes (next steps). Where an input triggers a sound (KH new game, Advance Wars) windows
  flip by exactly one frame (1097 samples), so the two harnesses also apply a frame's keys at
  different points.
- **Clipping**: ours clips 32 samples in 4000 frames of the KH new-game run, mGBA none; the
  hardware clips the summed range to 0..3FFh as well (GBATEK), so expected at our level.

## Fix log (BIOS sound driver HLE, 2026-10-01)

The six titles the library sweep left silent (Atari Anniversary Advance, Lizzie McGuire - On the
Go!, Pocket Professor - Kwik Notes, Rampage - Puzzle Attack, X-Men - Reign of Apocalypse, Saibara
Rieko no Dendou Mahjong) now play music under HLE: SWI 1Ah-1Eh, 28h, 29h and the 36
SoundGetJumpList routines are emulated in `gba-core/src/bios/sound/` (clean room: GBATEK, the
games' own memory traffic, and the published m4a structure layouts as an interface spec; no BIOS
disassembly, no emulator source, no decompiled function bodies). Cyberdrive Zoids, the other
jump-list title, is pinned too. Seven `tests/compat.toml` rows with `audio_frames_min` gate it.

What the games do (measured before the change, `--log-swi`, `--watch`):
- Five titles use the documented work-area API (Init, Mode, Main, VSync, ChannelClear; X-Men
  calls MidiKey2Freq for every note). They write their channels at `area+0x50+0x40*n` with the
  playback rate at +0x20 and the wave pointer at +0x24, i.e. the m4a `SoundInfo`/`SoundChannel`
  layout, not the abbreviated struct GBATEK prints (`fr` at +0xC). Saibara hands the real BIOS
  an m4a-layout area (`func` +0x20, `intp` +0x24, `CgbSound` +0x28, jump table +0x34), which
  settles that the BIOS consumes that layout.
- SoundDriverMain is what runs the game's sequencer: it calls `func(intp)` (MPlayMain, chained
  through every music player) and then `CgbSound`, then mixes. With 1Ch a no-op Saibara's song
  started but never advanced and no jump-list command was ever dispatched.
- Saibara's own copies of SoundInit / SampleFreqSet / VSync (watched on the IO registers) give
  the register programming: SOUNDCNT_H = A90Eh (FIFO A right, FIFO B left, timer 0, FIFO resets),
  SOUNDBIAS high byte 42h, timer 0 stopped / reloaded / started, and the per-period DMA restart
  "DMA1/2CNT = 84400004h (four words immediately), CNT_H = 0400h, CNT_H = B600h" with the VSync
  counter running 9..1,0 -> reload 9 (restart when the decremented count reaches 0 or wraps).
  Its work area for rate 3 reads samplesPerVBlank 0xB0, pcmFreq 0x2910, pcmDmaPeriod 9,
  divFreq 0x31E: all reproduced by the derived table (period = 16777216/Hz, which divides the
  280896-cycle frame exactly for all twelve GBATEK rates; divFreq = period/2).

Design (ours, documented as such):
- **Guest callbacks** (`bios/call.rs`): an HLE routine that must run guest code pushes a
  `Continuation` (saved frame + what remains to do) on `Bios::calls`, sets `lr` = 0x140 and jumps.
  The trap at 0x140 pops it and runs the next stage; the last stage restores r0-r3, r12, lr, CPSR
  and PC like the real handler's `movs pc, lr`. Callbacks from an SWI run in System mode on the
  user stack (GBATEK), IRQs stay live, calls nest (LIFO), and the stack is in the savestate
  (`STATE_VERSION` 4).
- **Jump-list slots** are 36 distinct trap addresses 0x200+4i (SWI 2Ah writes exactly 36 words:
  the library keeps its CGB channel block right after the table). The traced contract: r0 =
  player, r1 = track, `cmdPtr` already past the command byte, return `bx lr`, `flags == 0` means
  the track ended. Slots 0-29 are the track commands (index = command byte - B1h), 30-35 the
  helpers SampleFreqSet, TrackStop, FadeOutBody, TrkVolPitSet, RealClearChain and a 64-byte
  clear. Saibara overrides 8, 17, 19, 28-33 with its own ROM copies; its title song uses
  patt/pend/tempo/keysh/voice/vol/pan/bend/bendr/fine, RealClearChain (every note end) and the
  clear (every track start). `ply_fine` delegates channel stopping to whatever slot 31 the table
  holds (the library's TrackStop, which also silences CGB oscillators) through a nested callback.
  `ply_port` (undocumented) only skips its two bytes; FadeOutBody implements the fade-out
  direction only; TrkVolPitSet's scaling is a fitted fallback (every title seen installs its own).
- **Mixer** (`bios/sound/mixer.rs`): once per Main, into ring slot `(period - cnt + 1) % period`
  (the slot the DMA fetches next; the DMA is on slot `period - cnt`). Envelope per GBATEK (attack
  adds `at`, decay and release multiply by `de`/`re` over 256, sustain holds `su`), STOP stays set
  during the release so the channel still reads as in use (the published "on" mask C7h),
  pseudo-echo after the release (`IEC`), one-shot and forward-loop waves, nearest-sample
  resampling at `fr / pcmFreq`, output `clamp((sum(sample * ev*vol>>8) * masterVolume) >> 12)`
  (one channel at full level reaches +-119), reverb = `(old_right + old_left) * reverb >> 8` from
  the bytes being overwritten, first buffer half = right (FIFO A), second = left (FIFO B).
  **Fixed-rate voices** (`type` bit 3) are copied one source sample per output sample: Pocket
  Professor pairs 22 kHz clips with the 21024 Hz mode and never writes `fr`; Rampage writes an
  `fr` of 946921, meaningless as a rate, and its wave headers carry the placeholder 01234567h.
  Decay to a sustain level of zero ends the channel (design choice). Cost: ~6 cycles per
  channel-sample and 3 per output sample, charged in 32-sample chunks with `hle_service()`.
- Mode: zero fields leave the value (Lizzie's two calls); the D/A field sets SOUNDBIAS bits 14-15;
  a rate change clears the buffer, re-arms timer 0 and restarts the DMA. VSyncOff marks the area
  (ident+10) so Main and VSync idle until VSyncOn, whose next VSync restarts the stream.

Results (1500 frames, triage input): audio frames Atari 455, Lizzie 993, Pocket Professor 76 (one
clip; nothing else sounds in that window), Rampage 825, X-Men 1495, Saibara 1270, Zoids 1094;
no clipping in any capture, peaks 17k-30k of 32k; 0-2 FIFO underruns per run. KH `--bench 1200`
unchanged. Not verified against hardware: the output scaling, reverb and pseudo-echo formulas,
the FIX rule beyond the two titles, and whether the real mixer writes the next slot or two ahead
(ours writes the next; every title calls Main well before the DMA reaches that slot).

## Fix log (ROM-to-ROM DMA, Halt exit; 2026-09-30)
mgba-suite "Timing tests" 1956/2020 -> 2020/2020 and "SIO timing tests" 0/4 -> 4/4; every other
row unchanged (Memory 1552/1552, DMA 1244/1244, Timer count-up 936/936, Timer IRQ 90/90, I/O 130/130,
Misc edge 6/12, jsmolka green).
- **ROM-to-ROM DMA** (`bus/dma_exec.rs`): when source and destination are both in the game pak the
  transfer is 2 idle cycles, read N, **write S**, then S/S for the remaining units. All 64 failing
  rows ("Trivial/Short DMA (16/32, ROM to ROM)", 20 columns each) pass. How it was pinned: the
  N- and S-waitstate columns (.N., ..S, .NS) show exactly one N and one S access inside the
  transfer for a single unit, and the from-ROM / to-ROM-only rows (already passing) fix the cost of
  a lone ROM read or write at one N. GBATEK quotes x = 4 idle cycles for this case; modelling that
  as 4 I with N+N accesses was 4 cycles slower than the hardware in every column (the first
  attempt, idle 4 with read S and write S, reached 1984/2020 and left the N/S columns off by one,
  which ruled it out). The 2 I + N + S split is an empirical fit, not derived from a bus diagram;
  the 128K block boundary and prefetch-buffer rules were not needed (DMA accesses still drop the
  buffer). Unit test: `bus::tests::rom_to_rom_dma_charges_a_sequential_write`.
- **Halt exit** (`bios/hle.rs`, `gba.rs`): the HLE Halt SWI charged its flat 10 cycles before
  sleeping and nothing after the wake, so code after `Halt()` ran 31 cycles early. The SIO timing
  rows (timer started, SIO transfer, SIO IRQ wakes Halt, timer stopped) were each exactly 31 short
  (0x25A vs 0x279 etc.). `HALT_EXIT_CYCLES = 31` (the same sum as the independently fitted
  `INTR_WAIT_EXIT_CYCLES`) is paid by the IRQ return stub that ends the waking interrupt, or at
  the wake-up itself when IME / CPSR.I mask the IRQ. Empirical (rest of the halt routine plus the
  SWI exit, which the real BIOS runs after the handler returns). Unit test:
  `bios::hle::tests::halt_exit_is_charged_by_the_irq_return_stub`. `Bios::halt_exit` is not saved
  (a savestate taken while parked loses only the exit charge).
- Goldens touched: suite Timing `2020/2020`, SIO timing `4/4` in `tests/golden.toml`; compat FFIV
  new-game entry moved from frame 19800 to 19900 (`tests/compat.toml`). The Halt exit shifts the
  scripted scene by a few frames: at 19800 the Baron corridor is mid-transition with no sprites,
  at 19900 it shows the same picture the old 19800 pin did (PNGs reviewed; the dialogue "Cecil: I'm
  sorry, Kain." at 20100 is identical to the baseline). No other compat golden moved.

## Misc edge attempt (2026-09-30): no rows moved, 6/12 kept
Rows left: DMA Prefetch Break/Read, H-blank bit start Hblank/Flip 1/2/3. (The harness prints
"Got <hardware> vs <ours>" for this sub-suite, the reverse of the timing rows.) Findings:
- **DMA Prefetch.** The break on hardware is at iteration 2725 of a 36-cycle loop, i.e. the 12th
  (k = 11) H-blank DMA of the frame, where our loop timing matches to 0.4 iteration. The open-bus
  read (`ldm r2!, {r3}` from 10000000h) returns the DMA latch only when the H-blank request lands
  between the ldm's prefetch fetch and its data read. Our DMA runs between instructions, plus an
  `arbitrate` hook that only fires on data accesses; with a loop period of 36 and a line of 1232
  the request phase advances by 8 cycles per line (9 distinct phases, 4 apart) and every mid-
  instruction hit lands in the `str`'s window, never the `ldm`'s. Tried: processing all due
  scheduler events (and the DMA they trigger) in `arbitrate` (the DMA then runs mid-instruction
  72 times in the loop, always inside the str), plus a start-latency parameter L = 0..14 on that
  path: no value produced the break. Hardware breaking only at k = 11 although the phases repeat
  every 9 lines also shows the real window is not a fixed phase set (loop or prefetch state varies);
  not modelled. Reverted.
- **H-blank flips.** `Flip 1` depends on the Halt-wake-to-TM0 read time: the Halt exit moves it in
  steps of 8 (poll loop period); hardware 135 is not reachable (we give ..128/136). `Hblank`
  (hardware 1232, ours 1235) is exactly the 3 extra `movs` between the wake and the TM0 read in the
  second Halt: on hardware they cost nothing relative to the first Halt, which no single constant
  (wake latency, H-blank flag 1006 vs 1008, IRQ delay) can explain, since both wakes are 1232 apart
  and share the path. Flip 2/3 are off by 1 and 2 cycles in the same way. Scanning the Halt exit
  0..40 never matched more than the current rows; processing due events before DISPSTAT reads
  (event lag is up to one instruction) changed nothing. Not fixed.

## Fix log (library sweep: FIFO underruns, 2026-10-01)
The 1,543-title sweep (2400 frames, `--triage`) reported 182 titles at 0 underruns but 163 above
5000 (Strawberry Shortcake 1.69M, Ultimate Winter Games 817k, Super Monkey Ball Jr. 623k ...).
Question: real starvation (our DMA/timer timing drains the FIFO) or accounting artefact?

**Diagnosis: accounting artefact; no emulator timing bug found in the six titles examined.**
`Fifo::underruns` counted every timer-overflow pop that found a speaker-routed FIFO empty,
whether or not the game ever fed it. Traced with `--fifo-stats 600` (per-frame pushed / popped /
underruns, DMA1/2, SOUNDCNT_H) and `--pcm --stats-from 600`:
- Super Monkey Ball Jr. (622,751): FIFO A is healthy in steady state (256 bytes pushed and 256
  popped per frame, 0 underruns, DMA1 = F640 special timing, len 18-25). FIFO B is routed
  (SOUNDCNT_H 330E: bits 12/13), clocked by timer 0, DMA2 = 0000, pushed = 0 for the whole run:
  256 pops of a never-fed FIFO per frame. Tiger Woods (253k) is the same: A fed (176 pushed /
  185 popped per frame, 0 underruns), B routed (730E) and never fed after boot.
- F-Zero Climax (223k) and Klonoa 2 (200k): most of the count (180k / 69k) is before the game
  first feeds the FIFOs (sound enabled, timer running, DMA1/2 = 0400, 0-300 bytes pushed). After
  the driver starts (DMA1/2 = B600) pushed = popped every frame, 0 underruns (F-Zero 304
  bytes/frame, Klonoa 224). Klonoa's few remaining events are clips that end after one 16-byte
  DMA block (DMA left at 0400).
- Sonic Pinball Party (474k) and Strawberry Shortcake (1.69M): both FIFOs routed (210E / 330C), timer
  running, 240 / 4 bytes pushed in total and never again (DMA1/2 disabled). Their PCM captures
  are exactly zero for all 2400 frames (not investigated further whether the sound driver starts
  later with different input; the FIFOs are simply not fed).
- No title among the six shows pushed > 0 with the FIFO running dry at a steady rate, so the
  DMA1/2 FIFO trigger (<= 16 bytes), timer overflow, HLE SWI service and SOUNDBIAS sample-rate
  paths are not implicated. (KH:CoM's real bug of 2026-09-30 had pushed > 0.)
- GBATEK is silent on an empty-FIFO pop; `Fifo::pop_ex` holds the last sample (an unfed FIFO
  therefore stays at its reset value 0 = silence). That behaviour is unchanged.

**Fix (counter only; audio path untouched):** `gba-core/src/apu/fifo.rs`. `underruns` now counts
audible starvation: the **first empty pop after bytes were pushed** (a routed FIFO whose stream
ran dry). A never-fed FIFO, or one that stays empty after its source stopped, adds nothing until
fed again; a FIFO reset (SOUNDCNT_H bit 11/15) disarms it. The raw count remains as
`Fifo::empty_pops`. `live` and `empty_pops` are `serde(skip)`, so the savestate format is
unchanged. The routed-speaker rule from 2026-09-30 stays. Residual signal: one event per stream
end or missed refill; a game starving at a steady rate still counts once per refill.

Tests: `apu::fifo::tests::{never_fed_fifo_does_not_underrun,
running_dry_counts_once_until_fed_again, unrouted_or_reset_fifo_does_not_count}`. Three older APU
tests (`underrun_counts_and_repeats_last_sample`, `underruns_surface_through_timer_path`,
`unrouted_fifo_pops_do_not_count_as_underruns`) pinned the old every-empty-pop count and were
updated to the new rule; what they guard (last sample repeated, routing, timer path) is unchanged.

`--triage` underruns, 2400 frames, before -> after: Super Monkey Ball Jr. 622,751 -> 3; Sonic
Pinball Party 474,294 -> 6; Klonoa 2 208,062 -> 20; F-Zero Climax 223,893 -> 3; Tiger Woods PGA
Tour Golf 253,458 -> 5; Strawberry Shortcake Summertime Adventure 1,691,801 -> 1. Earlier
game-driven cases in this file (Classic NES, Super Mario Advance, Backyard Baseball) now read as
one event per dry-out; re-sweep the library to re-bucket it.
## Fix log (library sweep: late black screens, 2026-10-01)
Seven titles from the 1,543-title `--triage` sweep ended on a black/<= 2 colour picture at frame 3000.
Six are game behaviour reached by the sweep's fixed key pattern; one was an emulator fault.
- **Waiting for input / paused / by-design (no emulator fault):**
  - Classic NES Dr. Mario: the Start at 2300 *pauses* the game (white "PAUSE" on black, halted in the
    game's pause loop). Start again at 2600 resumes the playfield (Dr. Mario, level 0, virus 63). The
    Classic NES anti-emulator tricks are not involved here.
  - R-Type III: the Start at 2300 pauses stage 1 (green "PAUSE" blinks, so a sample can land on the
    blank phase: 1 colour, changing every ~50 frames). Frames 0-2300 draw title, weapon select and the
    stage normally. (Its IO dump shows 4770h in many IO registers after the game's own clears: not a fault.)
  - CIMA - The Enemy: white-on-black story caption ("Altran. Jillani Region, Northwest 40 km.") waiting
    for A. Pressing A every 100 frames from 3100 reaches the in-engine dialogue (Jester R., "this mine was
    so deep, Chief") by 4000.
  - SMT Devil Children Puzzle de Call!: black story text waiting for A; A every 100 frames from 3100
    reaches the overworld (a tiny blue creature in a village) by 3000-4000.
  - World Advance Soccer: instruction text "press A to start the qualifiers" (CPU busy = the game's
    polling loop); A reaches the team-select screen (flags, group A-D grid).
  - Derby Stallion Advance: the sweep's Start at 2300 answers the farm prompt "back already? (yes/no)"
    with yes; the game then shows "saving data" and "Data saved, turn off the power" (flash save works,
    the game deliberately ends the session there).
- **Emulator fault, fixed: Twin Series 5 (Cake-ya-san + Wanwan Meitantei).** After the Start press
  skips the intro, the game sits on a white forced-blank screen forever, CPU busy. Cause: the game's
  VSync is `while (VCOUNT <= 0x9F) ;` (ROM 08002B2E, ~46 cycles per iteration) and its V-Blank IRQ
  handler (a 42 kHz Sappy mixer, ~62k instructions) runs ~126k cycles, so the main thread is only
  scheduled from about line 34 of each frame and has just the instant of line 160 to observe
  VCOUNT >= 160. We raised the V-Blank request on the CPU line in the same cycle VCOUNT changes, so the
  IRQ always pre-empted the poll (traced: the loop ran 3350 iterations per frame, never exiting, r1 stuck
  at 9Fh). Timer overflows already reach the CPU line `IRQ_DELAY` (3) cycles late; the PPU's V-Blank and
  V-Count requests now do the same (`ppu::on_line_end` uses `raise_at(.., at + IRQ_DELAY, ..)`; IF is
  still set immediately). Any delay from 1 to 24 cycles unblocks the game (scanned), so the exact value
  is not what matters, only that the window is non-zero; 3 is the existing, suite-fitted constant.
  H-Blank IRQs are left immediate (their handler has no `at` and no title needed it).
  Unit test: `ppu::timing_tests::vblank_and_vcount_irqs_are_delayed_on_the_cpu_line`.
  After the fix Twin Series 5 reaches the story/opening dialogue (180 colours, last change 2432) in `--triage`.
- **Gate:** `cargo test --workspace` green except one compat golden: NFS Most Wanted (frame 6600, the
  TOLLBOOTH straight) moved `5741decb77b74fc7` -> `fc8e8136e1cefd2d`. Before/after PNGs reviewed: same
  scene (59 MPH, 00:37.72, same minimap/HUD), only a traffic car and a roadside object differ by a few
  pixels, i.e. the race's chaotic sensitivity to a 3-cycle shift of the V-Blank IRQ. Re-pinned in
  `tests/compat.toml`. Every other golden/compat hash (58 compat frames, all suite rows) is unchanged.
## Fix log (library sweep: silent titles, 2026-10-01)

Ten titles from the 1,543-title `--triage` sweep ran with a moving picture and no audio sample
above +-64. Findings per title (only the Millionaire trio was an emulator bug):

- **Who Wants to Be a Millionaire (Europe), 2nd Edition, Junior: FIXED.** The games program
  `TM0CNT_H = 0x0084` (enabled, F/1, bit 2 = "count-up") as their FIFO sample timer. GBATEK:
  count-up is "Not used in TM0CNT_H" / cannot be used for timer 0 (no predecessor). `timers.rs`
  treated the bit as a cascade on every timer, so timer 0 never counted, never overflowed and
  never clocked the FIFOs (`fifo popped=0`, no DMA refills, no sound). `Timer::running(n)` now
  ignores the bit for timer 0 (the stored/read-back value is unchanged). Unit test
  `timers::tests::timer0_ignores_the_count_up_bit`. After the fix `--triage` reports
  audio_frames 1558 / 1266 / 1024; a `--pcm` capture (32768 Hz) has real content (peak +-32k,
  about 0.3% of samples at the 10-bit clamp, i.e. loud source material, not a mix bug).
- **Famicom Mini 08 Mappy, 13 Balloon Fight, 19 Twin Bee: not a bug (triage input aliasing).**
  Same sound engine as the working Famicom Mini titles (FIFO A, timer 0 + timer 1 cascade, DMA1
  special, SOUNDCNT_H=0x0304, PSG unused). They sit in an attract-mode loop of about 400 frames
  and the triage Start presses are exactly 400 frames apart (700, 1100, ...), so every press
  lands in the attract demo (which on the NES is silent and just returns to the title). The mix
  buffer is cleared and clamped every frame but never accumulated into, i.e. the NES APU
  emulation has nothing to play. With Start at frame 150 (title) and 300 all three produce
  audio (120 / 163 / 318 audio frames in 600-900 frames, 65536 Hz, no clipping).
- **Arthur and the Invisibles, Deal or No Deal, Lara Croft Tomb Raider Legend: not a bug.** At
  frame 2400 they are still in publisher/developer logo sequences (language select, health
  warning, Mystic / DSI Games / Human Soft logos); FIFO-mode DMA runs but streams zeros. With
  6000 frames the same sweep input gives audio_frames 3386 / 3151 / 3477.
- **Strawberry Shortcake Summertime Adventure: not a bug.** Sound hardware initialised at
  frame 719 (SOUNDCNT_X/L/H written, FIFO timers armed) but the game starts no sample until a
  menu choice; with an A-tap script it reaches 6546 audio frames in 9000 frames. The 1.2M
  underruns are timer-0 pops of routed but unfed FIFOs while the menu idles.
- Open question, not changed: `Apu::write_io8` drops SOUNDCNT_L (0x80/0x81) writes while the
  master enable is clear (range 0x60..=0x81 per GBATEK "registers reset to zero"). The
  Millionaire games write SOUNDCNT_L = 0x77 before SOUNDCNT_X = 0x80, so it is lost; they use
  only the FIFOs so it is inaudible there, but a PSG title with that order would be silent.
  Hardware behaviour is not documented in the GBATEK excerpt, so it was left alone.
- Sweep recommendation: make the `triage_keys` Start spacing irregular (not a multiple of 400)
  and extend past 2400 frames for slow-logo titles; both "silent" classes above are artefacts of
  the fixed schedule.
## Fix log (library sweep: stuck boots, 2026-10-01)
Six boots from the 1,543-title `scripts/triage.py` sweep were stuck after 2400 frames. Two causes were
real emulator bugs (fixed, four games move now), one is an input-gated screen, one needs cartridge
hardware, one is undefined behaviour in the game that only an exact open-bus model can reproduce.
Harness additions (gba-test): `--dump-mem LO..HI` (side-effect-free word dump at the end of the run) and
`--watch-read LO..HI` (read watch; `--watch-log` logs every read).

**1. IntrWait / VBlankIntrWait never completed: r12 must be 0x04000000 while parked (Bubble Bobble
- Old & New, Elevator Action - Old & New).** Black, 0 picture changes, halted with IE=2001, the SWI
re-executed every frame. The Taito/AGB-SDK IRQ dispatcher in IWRAM starts with
`mov r3,#0x4000000; add r3,r3,#0x200; ldr r2,[r3] ...` and sets the BIOS flag with
`strhne r0,[r12,#-8]`, i.e. it relies on r12 = 0x04000000, so the write lands in 03FFFFF8 (the
03007FF8 mirror). The real BIOS waits inside its Halt routine, whose HALTCNT access leaves
r12 = 0x04000000, and the user handler is entered from there; our HLE parks the SWI with the
caller's r12 (0x31E in Bubble Bobble), so the flag went to 0x316 and `--watch 0x03007FF8` showed
the mirror never written. Fix: `hle_intr_wait` saves the caller's r12 on the first park, sets
r12 = 0x04000000 while parked and restores the caller's r12 when the wait completes
(`Bios::intr_wait_r12`, not serialised: a state loaded mid-wait leaves the parked value, which is
scratch for the caller). Test `intr_wait_runs_the_irq_handler_with_r12_at_io_base_and_restores_the_callers`.
`Halt` (SWI 2) has the same real-BIOS r12 but returns before the IRQ, so it is not modelled.
Triage afterwards: Bubble Bobble 63 colours / 1361 changes / audio, Elevator Action 24 colours / 965
changes (both moving to frame 2400).

**2. LCD IRQs reached the CPU instantly: a main loop polling VCOUNT could never win the race
(Majokko Cream-chan no Gokko Series 2 - Kisekae Angel).** White forced-blank screen after PUSH START
from frame 734. The main thread sits in `while (REG_VCOUNT <= 159) ;` (0x08002B1E, from
`0x08002B10`), the V-Blank handler is the m4a mixer (78k of 96k instructions per frame, ~170k cycles)
and is longer than the 68 blanking lines, so the main thread resumes inside the next visible frame
and spins until line 160, where the V-Blank IRQ is taken before the poll can read 160. The wait never
ends (counter 0x0300091C reset at entry was at 420 after 840 frames). On hardware the request
reaches the CPU a few cycles after the IF flag (same synchroniser as the timers) and the poll wins
that race now and then; mGBA schedules every IRQ with a 7-cycle delay and plays the game through.
Fix: H-Blank, V-Blank and V-Count requests use `raise_at(.. at + LCD_IRQ_DELAY)` with
`LCD_IRQ_DELAY = timers::IRQ_DELAY` (3), the value already pinned for timers by the mgba-suite Timer
IRQ rows (7 also works for this game). `Ppu::on_hblank` now takes the event time like `on_line_end`.
Test `lcd_irqs_reach_the_cpu_a_few_cycles_after_the_flag`. Triage afterwards: 120 colours, 608 changes,
last change at frame 2381.
Side effect, reviewed and re-pinned: any non-zero delay moves the chaotic race rows of the compat
manifest. With all three LCD IRQs delayed by 3 only NFS Most Wanted frame 6600 changed
(5741decb77b74fc7 -> fc8e8136e1cefd2d; PNG compared: same scene, 59 MPH, 00:37.72, one traffic car
elsewhere) and the row was re-pinned. For reference, delaying only V-Blank moved NFS MW and Mario
Party Advance @3000 at 3 cycles and Mario Party only at 4, 6 and 7 (NFS MW at 1, 2 and 5; Mario Party was not checked there), and delaying all three by 7 (mGBA's `GBA_IRQ_DELAY`) moved both; the value 3 was kept
because it is the one already pinned for the timer IRQ path by the hardware-recorded mgba-suite rows,
and it disturbs the fewest pinned rows. The r12 fix above changes no golden or compat hash.
Not the cause (tried): WAITCNT 0x4317 at boot with RegisterRamReset leaving WAITCNT alone (the EEPROM
library ORs 0x0300 into whatever is there; faster ROM did not help), the EEPROM path (no DMA3 after
frame 700), SIO (only three SIOCNT writes at frame 1).

**3. Sonic Advance 2 (USA): not stuck.** The 27-colour picture is the REG. NAME screen of a blank
Flash save and it is waiting for the player. The sweep's Start at 1100 moves the cursor to END and
the A at 1300 presses END with an empty name, which does nothing on hardware either. With a script
(A, RIGHT, A) the picture changes and the name field fills in (`--png` at 1400). No emulator change.
The "only 99 audio frames" is the silent name screen.

**4. Music Recorder MP3 (Japan): needs cartridge hardware, documented only.** The game shows
"cartridge battery must be replaced" (カートリッジの電池を交換してください) at once. It drives a
memory-mapped controller in the game pak space: byte writes to 0x0B080000/0x0B0A0000 (250 clock-like
pulses of 0x00/0x81 from 0x08011E7E, a 6-register setup from 0x08011D7A, 0x0B0C0000-0x0B120000),
nothing at 0x0E000000, save type none. That is the card/MP3 interface of the cartridge; we do not
emulate the cart and the error screen is the correct reaction to a missing one.

**5. Hello Kitty Collection - Miracle Fashion Maker (Japan): unresolved, undefined behaviour in the
game (5+ attempts, notes for the next person).** Black, DISPCNT=0, the CPU runs a pointer chase at
0x080427A0 for ever. Chain: 0x08042644 is the heap init (one free node at 0x02000010, size 0x30000,
`next` = 0); the first `malloc` (0x08042674, frame 34, size 0x110) calls the coalescing routine
0x080427A0 *before* searching, and with a single node it does `r3 = node->next (0)`,
`r0 = [r3+0x14]`: a read of BIOS address 0x14 from ROM code, which returns the BIOS open-bus latch
(0xE55EC002), so the loop keeps following garbage pointers (r2 = 0xE55EC002, then 0x28002800, the
opcode halfword duplicated) in a 3-load cycle. **mGBA does exactly the same**: `mgba-sdl -l 127` on
the ROM prints `Bad BIOS Load32: 0x14`, `0x1C`, `Bad memory Load32: 0xE55EC01E`, `0xE55EC016`, then
the same 0x2800281C/0x28002814 cycle that we reach (5725 + 2878 loads), and the game still escapes
a few thousand iterations later (the log shows the chain leaving the cycle through 0x69D069xx
addresses, i.e. an open-bus value that is the halfword at X instead of X+4, and later reaching zero
words), then re-entering `Bad BIOS Load32: 0x1C/0x14` once per call, and the title screen
(pink, stars) comes up in the GUI build. We never escape. Candidates that were tried without effect:
forcing the BIOS open-bus value to zero (the game then runs on, 449 picture changes by frame 1200, but the screen was still black, so further blockers may follow), HLE IRQ return
leaving the first instruction's open bus at X+2, LCD IRQ delay 3 and 7. What mGBA does differently in
the first escape could not be pinned down from its log (it prints addresses, not values); the real
hardware path is probably a bus/DMA latch value (sound FIFO DMA is enabled) or the ROM prefetch
state. Fixing it needs a cycle-accurate open-bus/prefetch-latch model, not a game hack.
## Fix log (library sweep: crash boots, 2026-10-01)
Seven boots from the 1,543-title `--triage` sweep ended black with the CPU somewhere odd. Three
causes found and fixed or ruled out, two games left undiagnosed. Harness: `gba-test --dump-mem LO..HI`
(prints `mem AAAAAAAA: WWWWWWWW` at the end of a run) was added for this work.
- **V-Blank IRQ latency (James Pond - Codename Robocod, Winter Sports): fixed.**
  Both games sit in a main loop that waits for `VCOUNT == 160` (`ldrh r3,[VCOUNT]; cmp r3,#0xA0;
  bne`) with the V-Blank IRQ enabled; the frame is exactly a multiple of the loop period, so the
  phase never changes. The PPU raised its IRQs straight onto the CPU line at the line boundary,
  so the handler preempted before any instruction could read 160 and the loop never left (the
  game kept re-running its screen-clear; black picture, ROM-busy CPU, audio still playing). IF and
  VCOUNT still change at the boundary but the V-Blank request now reaches the CPU
  `ppu::PPU_IRQ_DELAY` = 7 cycles later (mGBA's `GBA_IRQ_DELAY`). Any delay 1..7 frees both games.
  Golden impact (scanned D = 1..7, V-Blank only, all 45 frame-pinned compat rows): D = 1, 2, 3, 5
  also move the NFS Most Wanted race hash at frame 6600 (5741decb77b74fc7), so 4, 6 or 7 are the
  only candidates; delaying H-Blank/VCount as well moves it too, so those stay immediate. EVERY
  D = 1..7 moves exactly one other row: Mario Party Advance, title at frame 3000, e010183ef13054ff
  -> 416390f06629c657 at D = 7 (the `--png` pair was reviewed: identical title screen, only the
  two balloons sit at a different animation phase; the 32100-frame row of the same game still
  matches). That golden is NOT re-pinned here; `cargo test` fails on that one row until it is.
  Test: `vblank_irq_reaches_the_cpu_after_the_vcount_change` (and `vblank_flag_and_irq_timing`
  now drains `IrqLatch`). Triage after: James Pond 118 colours / 438 changes, last change 2301
  (language select, then the Valcon/intro screens); Winter Sports 178 colours / 557 changes, last
  change 2399.
- **SoundGetJumpList, SWI 2Ah (Saibara Rieko no Dendou Mahjong, Cyberdrive Zoids): crash fixed,
  audio still absent.** The m4a library calls `MPlayJumpTableCopy` (SWI 2Ah) to fetch 36 pointers
  to the BIOS's track-command routines (`ply_*`), overrides a few, and `bx`es through the rest.
  HLE left the buffer untouched, so slot 35 was 0 and the game executed the BIOS region from
  address 0 (cpsr mode bits 0, pc=00000134 spin). Real routine addresses and bodies need a BIOS
  dump, so HLE now writes the 120h-byte buffer full of `HLE_SOUND_STUB_ADDR` (0x150), which
  `hle_vector` handles as `bx lr` (ARM or Thumb by LR bit 0). The track commands therefore do
  nothing (no music), but both games now run: Saibara 55 colours / 2054 picture changes, Zoids 23
  colours / 1037. Both still call SWI 1Ch SoundDriverMain (BIOS sound driver, not emulated; stays
  a logged no-op), so `audio_frames` is 0 / 2. Test:
  `sound_get_jump_list_fills_the_table_with_a_returning_stub`.
- **Daigassou! Band-Brothers - Request Selection: not a bootable GBA program.** The library has
  the "(DS Expansion Cartridge)" dump (no "(Demo)" file exists in `Games/`): 1 MiB whose first
  0xA0 bytes are FFFFFFFFh and whose header carries no entry branch, so the CPU executes
  conditional-NV words, takes the undefined vector and spins at 0x134 on hardware as well. It is
  slot-2 data for the DS game. No emulator bug.
- **Tennis no Ouji-sama 2004 Glorious Gold / Stylish Silver: not fixed (stuck rule).** Chain found,
  newest effect first: the CPU ends executing zeroed IWRAM (Gold) / an undefined word (Silver);
  that is the game's IWRAM sound mixer (m4a SoundMainRAM copy at 03000000, called from a ~14-line
  timer IRQ) whose code has been corrupted. The mixer's stack mix buffer pointer `r6` ran past
  03008000, the IWRAM mirror wrapped onto 03000000, and the mixer *added the sample value (-1) to
  its own code* (every halfword decremented). It ran away because its loop-counter instruction
  (`subs r4,r4,#2` at 030003B8) had already been zeroed by an earlier store. That store is the
  game's own init routine (ROM 08064F20, GCC -O0 Thumb) writing `array[3*i + j]` where `j` is the
  local at r7+0x10, which the function never initialises: it reads stale stack (0x3F3F left by an
  earlier call, so the address lands at 03070xxx, an IWRAM mirror of the mixer). The stale word is
  zero only if a mixer IRQ taken with a shallow sp has cleared that stack area since, so the
  outcome depends on the exact IRQ phase against the main thread's call depth. Changing the PPU
  IRQ delay (1..7) leaves the result identical; no register, I/O or memory-map divergence from
  hardware was found. Open: compare against a reference trace from an mGBA build with the CLI
  debugger, starting from a savestate at frame 17 (`--frames 17 --save-state`).
