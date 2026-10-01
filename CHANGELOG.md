# Changelog

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
  music. A BIOS dump restores it.
- Hello Kitty Collection: Miracle Fashion Maker and both Tennis no Ouji-sama 2004 titles do not
  boot (details in PROGRESS.md).
- Gamepad buttons cannot be rebound yet (keyboard can).
