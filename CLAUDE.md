# CLAUDE.md — GBA emulator (Rust)

Strawberry: a from-scratch Game Boy Advance emulator. Success criterion (met): boot and play
**Kingdom Hearts: Chain of Memories (USA)**; then Zelda titles and Initial D.
`PROGRESS.md` is the handoff document (status, compat table, known gaps, fix log, next steps).
`docs/ARCHITECTURE.md` is the design contract. `docs/gbatek/` holds the spec. `docs/gui/` is the
GUI design package (DESIGN.md wins over the mockups; the emulator's behaviour wins over both, and
deviations are logged in PROGRESS.md). `README.md` is the user-facing overview.

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
               irq.rs, keypad.rs, cart/ (ROM, SRAM/Flash/EEPROM), bios/ (LLE image + HLE SWIs; call.rs
               guest callbacks from HLE routines; sound/ the BIOS sound driver: work area, mixer,
               SoundGetJumpList track-command handlers), sched.rs, timing.rs, gba.rs (main loop),
               state.rs (savestates)
gba-test/      lib: load_rom (.gba/.zip), fb_hash (xxh3-64), checks (jsmolka / mgba-suite), manifest
               schema, input scripts; bin: headless CLI; tests/roms.rs: cargo test driver
gba-frontend/  bin `strawberry`: app.rs (screens + game session), ui/ (software renderer: tiny-skia + fontdue,
               tokens, widgets, SVG drawings, input), overlay/ (native 240x160 pause menu + toasts),
               screens/ (library, states, settings, bench), library/ (scan, names, index, captures),
               slots.rs (8 state slots per game), settings.rs (TOML in the config dir), window.rs
               (resizable minifb, letterbox fast path, filters), audio.rs (cpal, gain), pacing.rs,
               saves.rs, debug_ui.rs (hotkey map); assets/fonts/ (OFL faces, embedded)
docs/gui/      DESIGN.md, SCREENS.md, tokens.json, mockups/*.png, drawings/*.svg
docs/branding/ the Strawberry mark (SVG sprite, source of truth for ui/brand.rs), wordmark, lockups, .ico,
               PNG icons, boards (02 mark rules, 05 empty library); README.txt is the designer's note
tests/golden.toml   test-suite ROMs: check strategy + golden hashes
tests/compat.toml   commercial ROM goldens (skipped when Games/ is absent); `audio_frames_min` rows gate the sound driver
tests/inputs/       scripted input logs (khcom-newgame.toml, minish-newgame.toml, ...; triage.toml = the sweep's schedule)
scripts/fetch-tests.ps1   clones jsmolka/gba-tests + mgba suite, downloads portable mGBA, GBATEK
scripts/gbatek-split.py   regenerates docs/gbatek from tools/gbatek.htm
scripts/compare-wav.py    aligns and compares a --pcm capture with a reference WAV (numpy/scipy)
scripts/build-mgba.ps1    clones mGBA 0.10.5 into tools/mgba-src and builds tools/mgba-wav.exe (headless libmgba audio capture)
scripts/mgba-wav/         the capture tool's CMakeLists.txt + main.c (public libmgba API only)
scripts/compare-audio.py  records a title set with gba-test (full / PSG-only / FIFO-only) and mgba-wav, fits gains, writes tools/audio-ref/report.md
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
cargo run -p gba-test -- <rom> --frames N --watch-read 0x04000000..0x040003FE --watch-log   # log every read of a range
cargo run -p gba-test -- <rom> --frames N --dump-mem 0x03007D80..0x03007E40   # word dump of guest memory at the end
cargo run -p gba-test -- <rom> --frames 1800 --pcm out.wav --stats-from 1200   # audio capture + underruns
cargo run -p gba-test -- <rom> --frames 1900 --fifo-stats 1800                # per-frame FIFO pushed/popped/underruns
cargo run -p gba-test -- <rom> --frames 1800 --pcm psg.wav --mute-channels 30 # PSG only (mask: 1 SQ1 2 SQ2 4 WAVE 8 NOISE 10 FIFO A 20 FIFO B)
scripts/build-mgba.ps1; python scripts/compare-audio.py --titles kh,minish    # reference audio comparison -> tools/audio-ref/report.md
cargo run -p gba-test -- <rom> --frames 300 --log-swi                         # every HLE SWI call (and sound-driver jump-list call) as a guest: line
cargo run -p gba-test -- <rom> --frames 27400 --input tests/inputs/khcom-newgame.toml --save-state kh.state
cargo run -p gba-test -- <rom> --frames 300 --load-state kh.state    # resume a savestate headless
cargo run -p gba-test -- <rom> --frames N --record                   # print a manifest stanza
cargo run --release -p gba-frontend                                   # the library (folders from settings / --library)
cargo run --release -p gba-frontend -- <rom|zip>                      # play straight away
cargo run --release -p gba-frontend -- <rom> --scale 3                # fixed 3x picture in a 720x480 window
cargo run --release -p gba-frontend -- <rom> --bench 1200             # headless speed
cargo test -p gba-frontend --release -- --ignored render_             # review renders of every screen -> target/ui-renders/*.png
python scripts/triage.py Games --jobs 8                               # library sweep -> tools/triage/{triage.csv,triage.md,png/}
python scripts/triage.py --report tools/triage/triage.csv --doc docs/library-compat.md --notes tools/triage/notes.csv
```
**Agent worktrees and `Games`.** Worktrees under `.claude/worktrees/` get a directory junction
`Games -> <repo>\Games` so compat tests can run. **Never `git worktree remove` such a worktree while
the junction exists**: git follows the junction and deletes the real ROM folder's contents (this
happened on 2026-10-01). Remove the link first with `cmd /c rmdir <worktree>\Games` (deletes only
the junction), then remove the worktree.

Harness exit codes: 0 pass, 1 fail, 2 error. `GBA_TESTS_REQUIRED=1` turns the
"GBA_TESTS_DIR unset" skip into a failure. Builds from concurrent shells should use their own
`CARGO_TARGET_DIR` (e.g. `target/integ`) to avoid the cargo lock.

Frontend keys (defaults, rebindable in Settings): Z=A X=B A=L S=R Enter=Start RShift=Select
arrows=D-pad; gamepad South=A East=B LB=L RB=R Start Select D-pad/left stick (pad buttons rebind
too, stored as `[gamepad]`); Tab fast-forward; Esc or P pause menu (resume / save / load / settings / bench /
quit); N frame-step; F1/F3 save/load the current slot (`<rom stem>.s<N>.state`); F2 save states
screen; F5 write `.sav`; F10 IO dump; F11 screenshot; F12 bench. Menus navigate with the GBA
bindings (A confirm, B back, L/R pages, D-pad). Hidden `--exit-after N` ends a windowed run and
prints timing stats. The settings file lives in `%APPDATA%\strawberry\settings.toml`, the
library index and title captures in `%LOCALAPPDATA%\strawberry\`.

## Releasing
Bump `[workspace.package] version` in `Cargo.toml`, add the `## vX.Y.Z — date` section to
`CHANGELOG.md` (it becomes the release body), commit, then `git tag -a vX.Y.Z -m "Strawberry X.Y.Z"`
and `git push origin vX.Y.Z`. `.github/workflows/release.yml` runs the unit tests, builds the player
with `--profile dist` for Windows x64, Linux x64, macOS x64 and arm64, and attaches
`strawberry-vX.Y.Z-<os>.{zip,tar.gz}` (binary + README, LICENSE, QUICKSTART, CHANGELOG) to the GitHub
release. Watch it with `gh run watch`; on a matrix failure fix the workflow, delete the tag locally
and remotely, and push it again. Every job runs on the user's self-hosted runners (since v1.1.0:
a root Linux container without `sudo`, the Windows box BULV-BULV, and the Mac); GitHub-hosted
runners are blocked by the account's spending limit, so do not switch jobs back to them.

## Working on a subsystem
The initial build split the code among parallel agents by directory (`cpu/`, `ppu/`, `apu/`,
`gba-frontend/`, with the integrator owning `bus/`, `gba.rs`, `sched.rs`, `timing.rs`, `dma.rs`,
`timers.rs`, `irq.rs`, `keypad.rs`, `cart/`, `bios/`, `cpu/regs.rs`, `gba-test`). The contracts
those boundaries rely on are documented at the top of each `mod.rs` and in
`docs/ARCHITECTURE.md`; keep them intact when changing one side. Debugging recipe: reproduce
headless with `gba-test`, narrow with `--break`/`--watch`/`--trace`, add a unit test that pins the
behaviour, then fix. Log non-obvious findings in `PROGRESS.md`.

## Reference emulator
`tools/mgba/mGBA-0.10.5-win64/mGBA.exe` (portable, from `fetch-tests.ps1`) for playing by hand, and
`tools/mgba-wav.exe` (from `scripts/build-mgba.ps1`: libmgba 0.10.5 built from source with MinGW
gcc, static, no dependencies) for headless audio captures: `tools/mgba-wav.exe rom.gba out.wav
<frames> <rate> [inputs.txt]`, driven by `scripts/compare-audio.py`. mGBA's HLE BIOS stubs the
sound driver SWIs, so the six BIOS-driver titles are silent there; compare ROM-mixer titles only.
No CLI debugger on Windows (`USE_EDITLINE` is non-Windows only): trace diffing still needs a Linux
build with `-d`; our `--trace` prints the same sixteen-`%08X` + `cpsr:` prefix so the files would
diff directly. Clean-room rule for the capture tool: read only mGBA's `include/`, CMake files and
`src/platform/`, never `src/gba/`.
