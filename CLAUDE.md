# CLAUDE.md — GBA emulator (Rust)

A from-scratch Game Boy Advance emulator. Success criterion: boot and play
**Kingdom Hearts: Chain of Memories (USA)**; then Zelda titles and Initial D.
`PROGRESS.md` is the handoff document (milestone status, known bugs, attempts).
`docs/ARCHITECTURE.md` is the design contract. `docs/gbatek/` holds the spec.

## Ground rules
- **GBATEK is the spec.** Read the relevant `docs/gbatek/*.md` before touching a
  subsystem. Never guess register semantics or timing.
- **Gates are gates.** A milestone is done only when its gate passes headless via
  `cargo test`. Never weaken, skip or special-case a test to pass it.
- **ROMs and BIOS are never committed.** `Games/` and `tools/` are gitignored.
  Paths come from env: `GBA_ROM`, `GBA_BIOS`, `GBA_TESTS_DIR` (see `.env.example`).
- **No BIOS dump exists here.** HLE is the boot path; LLE loads if `GBA_BIOS` is set.
- **Interpreter only.** Correctness first; optimize only after M6.
- **Commits:** no attribution trailers of any kind (no Co-Authored-By, no
  Claude-Session, no "Generated with"). Author is Ethan Aldrich. Commit after every
  green milestone gate.

## Layout
```
gba-core/      no deps. cpu/ (ARM7TDMI), ppu/, apu/, bus/ (memory map, IO dispatch, DMA exec,
               debug hooks), dma.rs, timers.rs, irq.rs, keypad.rs, cart/ (ROM, SRAM/Flash/EEPROM),
               bios/ (LLE image + HLE SWIs), sched.rs, timing.rs, gba.rs (main loop)
gba-test/      lib: load_rom (.gba/.zip), fb_hash (xxh3-64), checks (jsmolka / mgba-suite), golden.toml
               schema, input scripts; bin: headless CLI; tests/roms.rs: cargo test driver
gba-frontend/  minifb window + cpal audio + gilrs gamepad (bin name: gba)
tests/golden.toml   golden hashes and check strategy per test ROM
tests/inputs/       scripted input logs
scripts/fetch-tests.ps1   clones jsmolka/gba-tests + mgba suite, downloads portable mGBA, GBATEK
tools/         gitignored: tests/, mgba/, gbatek.htm
```

## Commands
```powershell
scripts/fetch-tests.ps1                      # once; then set $env:GBA_TESTS_DIR = "<repo>\tools\tests"
cargo build --workspace
cargo test --workspace                       # unit tests + every [[rom]] in tests/golden.toml
cargo run -p gba-test -- <rom|zip> --frames 32 --check jsmolka        # one test ROM
cargo run -p gba-test -- <rom|zip> --frames 600 --png out.png --dump-io
cargo run -p gba-test -- <rom> --frames 5 --trace trace.txt          # mGBA-format register trace
cargo run -p gba-test -- <rom> --frames 1800 --pcm out.wav --stats-from 1200   # audio capture + underruns
cargo run -p gba-test -- <rom> --frames 27400 --input tests/inputs/khcom-newgame.toml --save-state kh.state
cargo run -p gba-test -- <rom> --frames 300 --load-state kh.state    # resume a savestate headless
cargo run --release -p gba-frontend -- <rom|zip>                      # play
```
Frontend keys: Z=A X=B A=L S=R Enter=Start RShift=Select arrows=D-pad; Tab fast-forward;
P pause; N frame-step; F1/F3 save/load state (`<rom>.state`); F5 write `.sav`; F10 IO dump;
F11 screenshot; Esc quit. Savestates need the `savestate` feature on gba-core (enabled by
gba-frontend and gba-test; the core itself stays dependency-free by default).
Harness exit codes: 0 pass, 1 fail, 2 error. `GBA_TESTS_REQUIRED=1` turns the
"GBA_TESTS_DIR unset" skip into a failure.

## File ownership (parallel agents)
Integrator owns `bus/`, `gba.rs`, `sched.rs`, `timing.rs`, `irq.rs`, `dma.rs`,
`timers.rs`, `keypad.rs`, `cart/`, `bios/`, `cpu/regs.rs`, all of `gba-test`, and
the root docs. CPU agent owns `cpu/` (except `regs.rs`), PPU agent owns `ppu/`,
APU agent owns `apu/`, frontend agent owns `gba-frontend/`. Cross-module needs
(new IO register, event, trigger) are requests to the integrator.

## Reference emulator
`tools/mgba/mGBA-0.10.5-win64/mGBA.exe` (portable). Trace: `mGBA.exe -d rom.gba`,
then `trace 100000 out.txt` in its debugger; our `--trace` prints the same
sixteen-`%08X` + `cpsr:` prefix so the files diff directly.
