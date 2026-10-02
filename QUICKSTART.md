# Strawberry — quick start

Strawberry is a Game Boy Advance emulator with its own library, pause menu, save-state slots, settings and a
developer bench. No BIOS file is needed: the system functions are built in.

## Run it

- **Windows**: unzip and double-click `strawberry.exe`, or run it from a terminal.
- **macOS**: `tar xzf strawberry-*.tar.gz`, then `./strawberry`. The first launch may need
  *System Settings › Privacy & Security › Open Anyway* because the binary is not notarised.
- **Linux**: `tar xzf strawberry-*.tar.gz`, then `./strawberry`. Needs ALSA (`libasound2`) and an X11 or
  XWayland display.

With no arguments the **library** opens. Add your game folder in *Settings › System* (type or
paste the path and press Enter), or start with it on the command line:

```
strawberry --library "D:\Games\GBA"
strawberry "D:\Games\GBA\Some Game (USA).zip"      # play straight away (.gba or a .zip containing one)
strawberry --scale 3 "Some Game.gba"                # fixed 3x picture in a 720x480 window
```

## Keys (defaults, rebindable in Settings)

| GBA | Keyboard | Gamepad |
|---|---|---|
| A / B | Z / X | South / East |
| L / R | A / S | LB / RB |
| Start / Select | Enter / Right Shift | Start / Select |
| D-pad | Arrow keys | D-pad or left stick |

Menus use the same bindings: A confirms, B goes back, L/R turn pages. In Settings every GBA
button has a keyboard keycap and a gamepad keycap; click one and press the new key or pad button.

| Key | Action |
|---|---|
| Esc or P | pause menu (resume, save/load slot, settings, bench, quit) |
| Tab (hold) | fast-forward (speed limit in Settings) |
| F1 / F3 | save / load the current state slot |
| F2 | save states screen |
| F5 | write the game's `.sav` now (also written every second while it changes, and on quit) |
| N | advance one frame while paused |
| F11 | screenshot (PNG beside the ROM) |
| F12 | bench (developer view) |

## Where things are kept

- **Game saves**: `<rom>.sav` beside the ROM.
- **Save states**: eight slots per game, `<rom stem>.s1.state` … `.s8.state` beside the ROM.
- **Settings**: `%APPDATA%\strawberry\settings.toml` (Windows),
  `~/Library/Application Support/strawberry/settings.toml` (macOS),
  `~/.config/strawberry/settings.toml` (Linux). Plain TOML; safe to edit.
- **Library index and title pictures**: the user cache folder, `strawberry/`.

## A real BIOS

Not required. If you own a dump, point `GBA_BIOS` at it or choose *From file…* under
*Settings › System program (BIOS)*. The built-in system functions include the BIOS sound
driver, so titles whose music goes through it play without a dump (see `docs/library-compat.md`
in the source repository).
