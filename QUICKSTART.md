# Strawberry — quick start

Strawberry is a Game Boy Advance emulator with its own library, pause menu, save-state slots, settings and a
developer bench. No BIOS file is needed: the system functions are built in.

## Run it

- **Windows**: unzip and double-click `strawberry.exe`, or run it from a terminal.
- **macOS**: `tar xzf strawberry-*.tar.gz`, then `./strawberry`. The first launch may need
  *System Settings › Privacy & Security › Open Anyway* because the binary is not notarised.
  There is no app bundle, so the Dock shows a generic icon.
- **Linux**: `tar xzf strawberry-*.tar.gz`, then `./strawberry`. Needs ALSA (`libasound2`) and an X11 or
  XWayland display.

With no arguments the **library** opens. Add your game folder in *Settings › System* (type or
paste the path and press Enter), or start with it on the command line:

```
strawberry --library "D:\Games\GBA"
strawberry "D:\Games\GBA\Some Game (USA).zip"      # play straight away (.gba or a .zip containing one)
strawberry --scale 3 "Some Game.gba"                # fixed 3x picture in a 720x480 window
strawberry --slot 2 "Some Game.gba"                 # start from save-state slot 2 (an empty slot boots normally)
strawberry --fullscreen --exit-on-quit "Some Game.gba"   # couch: whole display, pause menu QUIT closes Strawberry
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
| Esc or P | pause menu (resume, save/load slot, sun level on Boktai, settings, bench, quit) |
| Guide, or Select+Start held 1 s (gamepad) | pause menu |
| Tab (hold) | fast-forward (speed limit in Settings) |
| F1 / F3 | save / load the current state slot |
| F2 | save states screen |
| F5 | write the game's `.sav` now (also written every second while it changes, and on quit) |
| N | advance one frame while paused |
| F11 | screenshot (PNG beside the ROM) |
| F12 | bench (developer view) |
| = / - | more / less sunlight on Boktai's solar sensor |

## Clock and sunlight

Games with a clock in the cartridge (Pokémon Ruby, Sapphire and Emerald, Rockman EXE 4.5, the
Boktai games and a few more) read your computer's local time. A time you set inside such a game
is remembered in its `.sav`.

The Boktai games also have a solar sensor. Strawberry gives it a sun level from 0 (dark) to 10
(the in-game gauge fills): change it with **=** and **-** while playing, or on the **SUN** row of
the pause menu (LEFT/RIGHT). The level is remembered in the settings file (`sun_level`).

## Where things are kept

- **Game saves**: `<rom>.sav` beside the ROM (games with a clock add 16 bytes at the end for it).
- **Save states**: eight slots per game, `<rom stem>.s1.state` … `.s8.state` beside the ROM.
  States from before v1.6.0 show CAN'T LOAD (the state format changed); delete them from the
  save states screen.
- **Settings**: `%APPDATA%\strawberry\settings.toml` (Windows),
  `~/Library/Application Support/strawberry/settings.toml` (macOS),
  `~/.config/strawberry/settings.toml` (Linux). Plain TOML; safe to edit.
- **Library index and title pictures**: the user cache folder, `strawberry/`. Releases before
  the name used `gba-advance/` for both; the first start moves those folders over.

## A real BIOS

Not required. If you own a dump, point `GBA_BIOS` at it or choose *From file…* under
*Settings › System program (BIOS)*. The built-in system functions include the BIOS sound
driver, so titles whose music goes through it play without a dump (see `docs/library-compat.md`
in the source repository).
