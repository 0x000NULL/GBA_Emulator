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
- **mgba-suite is built and gated** (`suite.gba` via its `docker-build.sh`; 13 sub-suites in
  `tests/golden.toml`, each pinned at its current score so any regression fails `cargo test`).
  Scores after the 2026-09-30 merges: Memory 1552/1552, I/O read 130/130, Timing 1956/2020,
  Timer count-up 936/936, Timer IRQ 90/90, Shifter/Carry/Multiply-long/BIOS-math all full,
  DMA 1244/1244, SIO R/W 90/90, SIO timing 0/4, Misc edge 6/12.
- **Frontend verified windowed (2026-09-30)**: KH:CoM, 300 frames, 60.10 fps average, 1 late
  frame, 0 audio underruns on a 48 kHz WASAPI device (`--exit-after 300`).
- **Open**: reference-emulator audio comparison manual; Timing 1956/2020 (the rest: ROM-to-ROM DMA); KH:CoM's whole 27400-frame
  new-game run is FIFO-underrun-free apart from a handful at scene changes (was ~427k before
  the HLE service hook, see the 2026-09-30 fix log).

## Milestone status

| M | Scope | Gate | Status |
|---|---|---|---|
| M0 | Workspace, contracts, harness, test suites fetched | `cargo build --workspace`; harness runs every golden entry | **DONE 2026-09-29** |
| M1 | ARM7TDMI: all ARM+Thumb, modes, exceptions, flags | jsmolka `arm.gba`, `thumb.gba` | **DONE 2026-09-29**: 65 CPU unit tests; real 2-slot pipeline; mGBA-format trace |
| M2 | Memory map, waitstates, DMA, timers, IRQ, HALTCNT | jsmolka `memory.gba`, `bios.gba`, `nes.gba`; mgba-suite memory/timing | **DONE 2026-09-30**: jsmolka green; mgba-suite Memory 1552/1552, DMA 1244/1244, Timing 1956/2020 pinned (prefetch buffer + BIOS-derived HLE SWI cycle costs; the 64 left are ROM-to-ROM DMA) |
| M3 | PPU modes 0-5, OBJ, windows, blending, mosaic, timing | jsmolka ppu/* golden hashes; mgba-suite video | **DONE (jsmolka side) 2026-09-29**: goldens pinned, 56 PPU unit tests. Gaps: per-line OBJ cycle budget not modelled, OAM evaluated on the rendered line rather than one line early; mgba-suite video blocked as above |
| M4 | BIOS HLE, every listed SWI | `bios.gba` under HLE; per-SWI unit tests | **DONE 2026-09-29**: `bios.gba` passes; all listed SWIs + IRQ vector/return emulation; 14 unit tests incl. LZ77/RL/Huffman encoder round trips. Sound-driver SWIs (1A-1E, 28, 29) are logged no-ops. LLE differential deferred (no BIOS image) |
| M5 | Save detection, SRAM/Flash/EEPROM, `<rom>.sav` | jsmolka save/*; round trip | **DONE 2026-09-29**: none/sram/flash64/flash128 pass (flash entries need 400 frames: tests 10/11 program 64K bytes one command sequence each); 8 cart unit tests incl. EEPROM protocol; `.sav` persistence in the frontend |
| M6 | KH:CoM title -> new game -> first room | title golden + input script | **DONE 2026-09-29**: see "Where things stand". Other priority titles reach title/file screens; Minish Cap reaches gameplay (golden + script) |
| M7 | APU PSG + FIFO + mixing | PCM capture vs mGBA | **DONE (underrun half) 2026-09-29**: 70 APU unit tests; KH title capture (`--pcm`, 65536 Hz) has 0 underruns over frames 1200-1800, clean stereo music, no clipping. **Reference comparison deferred** (manual: `scripts/compare-wav.py`) |
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
| Final Fantasy IV Advance (USA) | BZ4E | SRAM 32K | Red Wings intro, scripted battles, Baron castle; Cecil controllable in the corridor (four directions verified). Its `swi 0xFF` is a logged no-op (GBATEK: 2Bh+ jump to garbage on hardware) | title 2400, gameplay 19800 |

## Environment facts
- Rust 1.95 stable-msvc; no devkitARM; no BIOS dump. HLE is the only boot path.
- Frontend stack: pure Rust `minifb` + `cpal` + `gilrs` (user decision).
- Reference emulator: portable mGBA 0.10.5 at `tools/mgba/mGBA-0.10.5-win64/`. Its Windows build has
  **no CLI debugger** (`-d`), only GDB (`-g`), and no headless audio dump, so trace diffing and audio
  comparison need a different build or a manual GUI session.
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
- APU: PSG mix scale derived from GBATEK maxima, not measured against hardware; wave RAM
  shift-register read-back quirk not modelled; duty position not reset on restart.
- CPU: MUL/MLA leave C unchanged (GBATEK says "destroyed"); only Div/DivArm/Sqrt/ArcTan/ArcTan2/
  CpuSet/CpuFastSet have real BIOS cycle counts (other HLE SWIs still pay a flat 10, and the
  IntrWait exit cost of 31 plus the IRQ return stub's 2 idle cycles are fitted to the Timer
  count-up rows, not derived). Prefetch buffer: the data-access stall rule is empirical
  (see the prefetch fix log) and ROM-to-ROM DMA timing rows still fail in every column.
- BIOS HLE: BgAffineSet/ObjAffineSet use f64 trig instead of the BIOS sine table (tiny rounding
  differences possible); division by zero returns mGBA's values instead of hanging.
- Cartridge: no GPIO/RTC (none of the target games use it); SRAM open-bus when the backend is EEPROM
  reads 0xFF.
- Audio: HLE SWIs still complete atomically with respect to IRQs (an IRQ raised during a BIOS
  routine is taken when the SWI returns rather than mid-routine, as the real BIOS allows).
  ALttP shows 1910 over 18000 frames, Zelda II 43452 over 4200 and Zelda 1 65242 over 2600.
  The Classic NES ones are the games' own behaviour: their driver refills FIFO A from a timer-1
  IRQ (immediate DMA1 of 4 words, then re-arm as 16-bit special DMA), and during menu
  transitions they mask that IRQ (IE=2041 with the timer request pending in IF, DMA1 left
  disabled), so the FIFO starves on hardware as well. Traced with `--watch-log` on 2026-09-30.

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

## Next steps, in order of value
1. Timing rows still open: ROM-to-ROM DMA (64 sub-tests); Misc edge H-blank bit / DMA prefetch;
   SIO timing (in progress on an agent branch, 2026-09-30).
2. Initial D garage screens (phone, driving technique) render blank content areas: verify and fix
   (see Known gaps). Remaining `Games/` titles into gameplay (in progress, 2026-09-30).
3. Reference-emulator audio comparison (needs an mGBA build with headless audio dump).
