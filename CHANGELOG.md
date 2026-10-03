# Changelog

## v1.4.0 — 2026-10-02

- `--slot N` (1–8) starts the ROM from that save-state slot before the first frame is shown,
  for launchers (Fruit Basket couch mode's Load button). `--slot` needs a ROM; an empty or
  unreadable slot prints one line to stderr and the game plays on from a fresh boot. The slot
  files are unchanged.
- Developer tooling: `tools/mgba-ref.exe` (built by `scripts/build-mgba.ps1`) captures mGBA's
  pictures, memory, registers, IO changes and instruction traces headless, and
  `scripts/compare-trace.py` diffs instruction timing against ours. gba-test gained `--png-at`,
  `--regs`, `--poll`, `--trace-frames` and `--trace-cycles`.
- Checked against mGBA: the Initial D garage's blank phone and technique screens and the "0"
  under the NES Zelda file name are pixel-identical (game content, not bugs). The music start
  offsets of the audio comparison come from mGBA's own timing: it scores 1768/2020 on the
  hardware-recorded timing tests that Strawberry passes in full.
- `scripts/build-mgba.ps1`: quote the CMake policy argument (PowerShell split it at the dot).

## v1.3.1 — 2026-10-02

**fruit-basket.** The frontend's console-independent half (renderer and design tokens, fonts,
audio, pacing, saves, save-state slots, settings file, library, gamepads, window icon) moved to
the shared crates of the new public repo `0x000NULL/fruit-basket` (v0.1.0), which the Crabapple
NES emulator uses too. Nothing a player sees changes: every screen, the slot files, settings.toml
and the library index are byte-identical to v1.3.0 (pinned by `gba-frontend/src/pinned.rs`).
- README: the lockup follows the reader's light or dark theme, a Screens section shows every
  screen (`docs/screenshots/`), and a Name and mark section describes the brand; stale status
  text brought up to v1.2.0.
- The settings file's header comment names Strawberry (it still said Advance).
- Release workflow: `actions/checkout` v7, `upload-artifact` v7, `download-artifact` v8 and
  `action-gh-release` v3, all on the Node 24 runtime the self-hosted runners already provide.

## v1.3.0 — 2026-10-02

**Strawberry.** The player has a name and a mark (`docs/branding/`: a 16x16 four-colour
sprite, Fruit Basket No. 2).
- The window is titled Strawberry and carries the mark as its icon on Windows and X11; the
  Windows exe has the icon and a version block too (macOS and Wayland take no runtime icon).
- The binary and the release archives are `strawberry` / `strawberry-vX.Y.Z-<os>`.
- Settings and the library cache live in `strawberry/` under the user's config and cache
  folders; an existing `gba-advance/` is moved there once on first start.
- The empty Library is the brand screen from board 05: three rows of placeholders, the
  Strawberry cartridge in the handheld with the mark on its screen, and `Open <folder>` (Z) /
  `Settings` buttons. Settings is now reachable from the Library.

## v1.2.0 — 2026-10-02

**Player**
- Gamepad buttons can be rebound. The Controls page shows a second keycap for every GBA button;
  click it and press a pad button (a button already in use swaps, like the keyboard). The map is
  stored as `[gamepad]` in settings.toml; the left stick stays a D-pad. The default shoulder
  buttons are the bumpers (LB/RB; earlier docs called them LT/RT).

**Tooling**
- `scripts/build-mgba.ps1` builds `tools/mgba-wav.exe`, a headless libmgba audio capture, and
  `scripts/compare-audio.py` records a title set with both emulators and reports level, balance
  and alignment figures (`tools/audio-ref/report.md`); `scripts/compare-wav.py` now uses the
  same analysis. `gba-test --mute-channels MASK` silences PSG or FIFO channels in a capture.
- Finding, no change: our FIFO and PSG levels follow GBATEK's stated maxima; mGBA 0.10.5 mixes
  about 3 dB lower overall and its PSG another 3.5 dB lower relative to FIFO (PROGRESS.md
  "Reference audio comparison").

## v1.1.0 — 2026-10-01

**Accuracy**
- The BIOS sound driver is emulated (SWI 1A-1E, 28, 29, 2A and the 36 SoundGetJumpList
  routines): the six library titles that were silent without a BIOS file (Atari Anniversary
  Advance, Lizzie McGuire: On the Go!, Pocket Professor: Kwik Notes, Rampage: Puzzle Attack,
  X-Men: Reign of Apocalypse, Saibara Rieko no Dendou Mahjong) now play music; 1,528 of 1,543
  library titles boot with sound. Save states made by 1.0.0 are not loadable (format 4).

**Player**
- Save-state slots written by an older format are shown as `CAN'T LOAD` and can be deleted;
  the library, settings and `.sav` files are unaffected.

## v1.0.0 — 2026-10-01

First release. A from-scratch Game Boy Advance emulator in Rust: ARM7TDMI interpreter, PPU, APU,
DMA, timers, cartridge backup memory (SRAM, Flash, EEPROM) and a high-level-emulated BIOS, so no
BIOS file is needed.

**Player**
- Library of cartridges scanned from your game folders, with captured title pictures.
- In-game pause menu drawn inside the frame: resume, eight save-state slots with thumbnails,
  settings, bench, quit.
- Save states screen (film-strip thumbnails, loupe, delete), settings as an open manual (key
  rebinding, picture size and filters, fast-forward limit, volume, BIOS file, folders, audio
  device, autosave), and a developer bench (per-layer pictures, channel meters, scanline event
  timeline, CPU registers and disassembly).
- Resizable window, whole-number picture scaling on a dark letterbox, audio-clocked pacing,
  keyboard and XInput gamepads. Windows, macOS (Intel and Apple Silicon) and Linux x64.

**Accuracy**
- mgba-suite: Timing 2020/2020, Memory 1552/1552, DMA 1244/1244, Timer count-up 936/936,
  Timer IRQ 90/90, I/O 130/130, SIO 90/90 and 4/4; every jsmolka test ROM passes.
- Library sweep of 1,543 titles: 1,522 boot to a moving picture with sound; 10 stop at a menu
  the automated sweep does not answer; 6 run silent because their music needs the BIOS sound
  driver; 1 needs special cartridge hardware; 1 is not a game; 3 are open bugs
  (`docs/library-compat.md`).
- Kingdom Hearts: Chain of Memories, The Minish Cap, A Link to the Past & Four Swords, both
  Classic NES Zelda titles and Initial D: Another Stage reach controllable gameplay under scripted
  headless runs that gate every build.

**Known gaps**
- The BIOS sound driver (SWI 1A–1E, 2A) is not emulated: six titles that rely on it play without
  music. A BIOS dump restores it. (Fixed in the next release.)
- Hello Kitty Collection: Miracle Fashion Maker and both Tennis no Ouji-sama 2004 titles do not
  boot (details in PROGRESS.md).
- Gamepad buttons cannot be rebound yet (keyboard can).
