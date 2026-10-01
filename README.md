# GBA Emulator (Rust)

A from-scratch Game Boy Advance emulator: ARM7TDMI interpreter, PPU, APU, DMA, timers,
cartridge backup memory and a high-level-emulated BIOS, with a pure-Rust desktop frontend and
a headless test harness. Built against the GBATEK hardware documentation.

Kingdom Hearts: Chain of Memories, the Zelda titles (Minish Cap, A Link to the Past & Four
Swords, both Classic NES Series games), Initial D: Another Stage and twenty further commercial
titles (Final Fantasy, Mario, Crash, Need for Speed, Max Payne, ...) reach controllable gameplay
under scripted headless runs pinned in `tests/compat.toml`. A sweep of a 1,543-title library
(`docs/library-compat.md`) boots 1,522 to a moving picture with sound; the rest are menus the
sweep does not answer, six titles that need the BIOS sound driver, and three open bugs. The mgba test suite scores Timing
2020/2020, Memory 1552/1552, DMA 1244/1244 and Timer count-up 936/936. See `PROGRESS.md` for the
exact state of every subsystem.

## Build and run

Requires stable Rust (1.85+). No C toolchain, cmake or SDL is needed.

```powershell
cargo build --release --workspace
cargo run --release -p gba-frontend                              # opens the library
cargo run --release -p gba-frontend -- "path\to\game.gba"       # or play straight away (.zip containing one .gba also works)
```

The frontend is a small GUI in the style of a 2001 game box ("paper, ink, one red";
`docs/gui/`): a **library** of cartridges scanned from your game folders (add them in Settings or
with `--library DIR`; the folder of a ROM given on the command line is included), an in-game
**pause menu** drawn inside the frame (resume, save/load one of eight state slots, settings,
bench, quit), a **save states** screen with film-strip thumbnails, a **settings** manual
(controls with key rebinding, picture size and filters, fast-forward limit, volume, BIOS,
folders, audio device, autosave), and a **bench** developer view (per-layer pictures, channel
meters, scanline event timeline, CPU registers and disassembly).

Saves are written beside the ROM as `<rom>.sav`; save states as `<rom stem>.s1.state` …
`.s8.state` (an older single `<rom>.state` is picked up as slot 1). Settings live in
`%APPDATA%\gba-advance\settings.toml`. No BIOS image is required. If you have one, set
`GBA_BIOS` to its path (or pass `--bios`, or choose it in Settings) and it is used instead of the
built-in HLE.

### Controls

| GBA | Keyboard | Gamepad |
|---|---|---|
| A / B | Z / X | South / East |
| L / R | A / S | LT / RT |
| Start / Select | Enter / Right Shift | Start / Select |
| D-pad | Arrow keys | D-pad or left stick |

All of these can be rebound in Settings; the menus navigate with the same bindings (A confirm,
B back, L/R turn pages).

Hotkeys: **Esc** or **P** pause menu, **Tab** (held) fast-forward, **N** frame step,
**F1 / F3** save / load the current slot, **F2** save states screen, **F5** write the `.sav`
now, **F10** dump IO registers to the console, **F11** screenshot, **F12** bench. The window is
resizable and the picture is always drawn at a whole-number scale on a dark letterbox;
`--scale N` fixes the picture at N× in a window of exactly that size, `--no-audio` disables
sound, `--fullspeed` removes pacing, `--bench N` runs N frames headless and prints the speed.

## Workspace

| Crate | Purpose |
|---|---|
| `gba-core` | The machine. No dependencies by default; the `savestate` feature adds serde + bincode. |
| `gba-test` | Headless harness (`gba-test <rom> --frames N ...`) and the `cargo test` ROM driver. |
| `gba-frontend` | The GUI and player: window (`minifb`), software renderer (`tiny-skia`, `fontdue`), audio (`cpal`), gamepads (`gilrs`), pacing, saves, state slots, settings. |

`docs/ARCHITECTURE.md` describes the design; `docs/gbatek/` holds the spec excerpts the code
was written from; `CLAUDE.md` holds the project rules and commands.

## Tests

```powershell
scripts/fetch-tests.ps1                     # clones jsmolka/gba-tests + mgba suite, downloads portable mGBA
$env:GBA_TESTS_DIR = "$PWD\tools\tests"
cargo test --workspace
```

`cargo test` runs the core unit tests (about 330), the frontend's unit tests (about 150;
`cargo test -p gba-frontend --release -- --ignored render_` additionally writes review renders of
every screen to `target/ui-renders/`), every ROM in
`tests/golden.toml` (the jsmolka CPU, memory, BIOS, PPU and save suites, all passing) and the
commercial-ROM goldens in `tests/compat.toml` (skipped when the ROMs are not present under
`Games/`, which is gitignored). The mgba-suite entry is skipped because that suite ships as
source only and needs devkitARM to build.

## Status in one paragraph

Milestones M0-M7 of the plan are complete against their headless gates; M8 has savestates,
fast-forward and audio-clocked pacing, while the game-pak prefetch model and finer instruction
timing are deferred because the only gate that measures them (mgba-suite timing) cannot be run
here. Release speed is roughly 30x real time headless. Two things were not verifiable in the
development environment and should be checked once at a console: the live audio output path
(no audio device was available) and a reference-emulator audio comparison (mGBA's portable build
has no headless capture; `scripts/compare-wav.py` documents the manual procedure).
