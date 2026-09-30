# CLAUDE.md — GBA emulator (Rust)

A from-scratch Game Boy Advance emulator. Success criterion (met): boot and play
**Kingdom Hearts: Chain of Memories (USA)**; then Zelda titles and Initial D.
`PROGRESS.md` is the handoff document (status, compat table, known gaps, fix log, next steps).
`docs/ARCHITECTURE.md` is the design contract. `docs/gbatek/` holds the spec. `README.md` is the
user-facing overview.

## Ground rules
- **GBATEK is the spec.** Read the relevant `docs/gbatek/*.md` before touching a
  subsystem. Never guess register semantics or timing.
- **Gates are gates.** A milestone is done only when its gate passes headless via
  `cargo test`. Never weaken, skip or special-case a test to pass it. Goldens in
  `tests/golden.toml` and `tests/compat.toml` are re-pinned only after reviewing the `--png`.
- **ROMs and BIOS are never committed.** `Games/` and `tools/` are gitignored.
  Paths come from env: `GBA_ROM`, `GBA_BIOS`, `GBA_TESTS_DIR` (see `.env.example`).
- **No BIOS dump exists here.** HLE is the boot path; LLE loads if `GBA_BIOS` is set.
- **Interpreter only.** Correctness first.
- **Commits:** no attribution trailers of any kind (no Co-Authored-By, no
  Claude-Session, no "Generated with"). Author is Ethan Aldrich. Commit after every
  green milestone gate and push to `origin` (private GitHub repo
  `0x000NULL/GBA_Emulator`, created 2026-09-30).

## Layout
```
gba-core/      no deps by default (feature `savestate` adds serde + bincode). cpu/ (ARM7TDMI), ppu/,
               apu/, bus/ (memory map, IO dispatch, resumable DMA exec, debug hooks), dma.rs, timers.rs,
               irq.rs, keypad.rs, cart/ (ROM, SRAM/Flash/EEPROM), bios/ (LLE image + HLE SWIs),
               sched.rs, timing.rs, gba.rs (main loop), state.rs (savestates)
gba-test/      lib: load_rom (.gba/.zip), fb_hash (xxh3-64), checks (jsmolka / mgba-suite), manifest
               schema, input scripts; bin: headless CLI; tests/roms.rs: cargo test driver
gba-frontend/  minifb window + cpal audio + gilrs gamepad + pacing + saves + savestates (bin: gba)
tests/golden.toml   test-suite ROMs: check strategy + golden hashes
tests/compat.toml   commercial ROM goldens (skipped when Games/ is absent)
tests/inputs/       scripted input logs (khcom-newgame.toml, minish-newgame.toml)
scripts/fetch-tests.ps1   clones jsmolka/gba-tests + mgba suite, downloads portable mGBA, GBATEK
scripts/gbatek-split.py   regenerates docs/gbatek from tools/gbatek.htm
scripts/compare-wav.py    aligns and compares a --pcm capture with a reference WAV
tools/         gitignored: tests/, mgba/, gbatek.htm
```

## Commands
```powershell
scripts/fetch-tests.ps1                      # once; then set $env:GBA_TESTS_DIR = "<repo>\tools\tests"
cargo build --workspace
cargo test --workspace                       # unit tests + every [[rom]] in tests/golden.toml and tests/compat.toml
cargo run -p gba-test -- <rom|zip> --frames 32 --check jsmolka        # one test ROM
cargo run -p gba-test -- <rom|zip> --frames 600 --png out.png --dump-io
cargo run -p gba-test -- <rom> --frames 5 --trace trace.txt          # mGBA-format register trace
cargo run -p gba-test -- <rom> --frames N --watch 0x040000C4..0x040000C7 --watch-log   # log every write to a range (omit --watch-log to stop at the first)
cargo run -p gba-test -- <rom> --frames 1800 --pcm out.wav --stats-from 1200   # audio capture + underruns
cargo run -p gba-test -- <rom> --frames 27400 --input tests/inputs/khcom-newgame.toml --save-state kh.state
cargo run -p gba-test -- <rom> --frames 300 --load-state kh.state    # resume a savestate headless
cargo run -p gba-test -- <rom> --frames N --record                   # print a manifest stanza
cargo run --release -p gba-frontend -- <rom|zip>                      # play
cargo run --release -p gba-frontend -- <rom> --bench 1200             # headless speed
```
Harness exit codes: 0 pass, 1 fail, 2 error. `GBA_TESTS_REQUIRED=1` turns the
"GBA_TESTS_DIR unset" skip into a failure. Builds from concurrent shells should use their own
`CARGO_TARGET_DIR` (e.g. `target/integ`) to avoid the cargo lock.

Frontend keys: Z=A X=B A=L S=R Enter=Start RShift=Select arrows=D-pad; Tab fast-forward;
P pause; N frame-step; F1/F3 save/load state (`<rom>.state`); F5 write `.sav`; F10 IO dump;
F11 screenshot; Esc quit. Hidden `--exit-after N` ends a windowed run and prints timing stats.

## Working on a subsystem
The initial build split the code among parallel agents by directory (`cpu/`, `ppu/`, `apu/`,
`gba-frontend/`, with the integrator owning `bus/`, `gba.rs`, `sched.rs`, `timing.rs`, `dma.rs`,
`timers.rs`, `irq.rs`, `keypad.rs`, `cart/`, `bios/`, `cpu/regs.rs`, `gba-test`). The contracts
those boundaries rely on are documented at the top of each `mod.rs` and in
`docs/ARCHITECTURE.md`; keep them intact when changing one side. Debugging recipe: reproduce
headless with `gba-test`, narrow with `--break`/`--watch`/`--trace`, add a unit test that pins the
behaviour, then fix. Log non-obvious findings in `PROGRESS.md`.

## Reference emulator
`tools/mgba/mGBA-0.10.5-win64/mGBA.exe` (portable). This Windows build has no CLI debugger and no
headless audio dump: trace diffing needs a build with the CLI debugger (`-d`, then
`trace N file`); our `--trace` prints the same sixteen-`%08X` + `cpsr:` prefix so the files diff
directly once such a build exists. Audio comparison is manual (see `scripts/compare-wav.py`).
