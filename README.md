# GBA Emulator (Rust)

A from-scratch Game Boy Advance emulator: ARM7TDMI interpreter, PPU, APU, DMA, timers,
cartridge backup memory and a high-level-emulated BIOS, with a pure-Rust desktop frontend and
a headless test harness. Built against the GBATEK hardware documentation.

Kingdom Hearts: Chain of Memories, The Legend of Zelda: The Minish Cap, A Link to the Past &
Four Swords, both Classic NES Series Zeldas and Initial D: Another Stage all reach controllable
gameplay under scripted headless runs (`tests/compat.toml`). See `PROGRESS.md` for the exact
state of every subsystem and the mgba-suite scores.

## Build and run

Requires stable Rust (1.85+). No C toolchain, cmake or SDL is needed.

```powershell
cargo build --release --workspace
cargo run --release -p gba-frontend -- "path\to\game.gba"     # .zip containing one .gba also works
```

Saves are written beside the ROM as `<rom>.sav`; savestates as `<rom>.state`.
No BIOS image is required. If you have one, set `GBA_BIOS` to its path (or pass `--bios`) and it
is used instead of the built-in HLE.

### Controls

| GBA | Keyboard | Gamepad |
|---|---|---|
| A / B | Z / X | South / East |
| L / R | A / S | LT / RT |
| Start / Select | Enter / Right Shift | Start / Select |
| D-pad | Arrow keys | D-pad or left stick |

Hotkeys: **Tab** (held) fast-forward, **P** pause, **N** frame step, **F1 / F3** save / load
state, **F5** write the `.sav` now, **F10** dump IO registers to the console, **F11** screenshot,
**Esc** quit. `--scale N` sets the window size (default 3x), `--no-audio` disables sound,
`--fullspeed` removes pacing, `--bench N` runs N frames headless and prints the speed.

## Workspace

| Crate | Purpose |
|---|---|
| `gba-core` | The machine. No dependencies by default; the `savestate` feature adds serde + bincode. |
| `gba-test` | Headless harness (`gba-test <rom> --frames N ...`) and the `cargo test` ROM driver. |
| `gba-frontend` | Window (`minifb`), audio (`cpal`), gamepads (`gilrs`), pacing, saves, savestates. |

`docs/ARCHITECTURE.md` describes the design; `docs/gbatek/` holds the spec excerpts the code
was written from; `CLAUDE.md` holds the project rules and commands.

## Tests

```powershell
scripts/fetch-tests.ps1                     # clones jsmolka/gba-tests + mgba suite, downloads portable mGBA
$env:GBA_TESTS_DIR = "$PWD\tools\tests"
cargo test --workspace
```

`cargo test` runs about 230 core unit tests, the frontend's unit tests, every ROM in
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
