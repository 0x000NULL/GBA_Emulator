# GUI design language: paper, ink, one red

The reference renders are `mockups/*.png` (1280×800). Exact values are in `tokens.json` (since
the fruit-basket split it lives in that repo as `basket-ui/tokens.json`, beside the code it
checks, together with `pencil-ring.svg` and the fonts); this file
explains the rules behind them. When a mockup and this file disagree, this file wins. When either
one disagrees with how the emulator actually behaves, the emulator wins: change the design, not the
behaviour, and note it in `PROGRESS.md`.

## The idea

The interface is the printed matter that came with a game in 2001: the cartridge, the instruction
manual, the photos of your save points. It never competes with the game. **Colour belongs to the
games.**

## Brand

The player is **Strawberry**, Fruit Basket No. 2 (`docs/branding/`, 2026-10-02). The mark is a
16x16 sprite in the Pixel palette (cream, shade, ink, red: fifteen seeds), drawn at whole-number
scales only and never smoothed; `-night` variants (outline in cream, seeds in red, body clear)
go on dark backgrounds. `logo/strawberry-mark.svg` is the source `ui/brand.rs` parses; the
wordmark is outlined Archivo ExtraBold at 72 % width, and over a game the pixel font. The mark
appears on the window and the exe, in the empty Library (board 05) and nowhere else in the
chrome. The `00-system` mockup's "ADVANCE" header predates the name. Renders of every screen
as built live in `docs/screenshots/` (written by the ignored `render_` tests).

## Rules

1. **Chrome uses paper, ink and the spot red only.** All other colour comes from a game: its pixels
   (screenshots, state thumbnails) or its cartridge label.
2. **Red points.** It marks the section number in the header, the current selection, and callout
   dots. It is never a background fill, never decoration, never used for a second thing on a screen
   where it already marks the selection. Destructive actions use red text, not red buttons.
3. **Every action shows its key**, drawn as a keycap. The foot of every screen lists the keys that
   work there. Icons never appear without a word or a keycap beside them.
4. **Pixel art is shown at whole-number scales only**, nearest-neighbour, never smoothed or
   fractionally scaled. A 240×160 frame is shown at 1×, 2×, 3×, 4×… (thumbnails are the one
   exception: 48×32 is an exact 1/5 sample).
5. **Around a running game the paper goes dark** (the Night palette), so the picture is always the
   brightest thing on screen. Menus away from a game use Paper.
6. **Two resolutions.** Anything drawn *over a running game* (pause menu, slot picker, toasts) is
   drawn at native 240×160 into the frame, in the Pixel palette, and scaled with it. Everything
   else (library, save states, settings, bench) is drawn at window resolution.

## Palettes

| Palette | Used for | bg | bg2 | fg | fg2 | line | spot |
|---|---|---|---|---|---|---|---|
| Paper | Library, Save states, Settings | `#F4EFE3` | `#E7E0CF` | `#1A1A1A` | `#5A5448` | `#D6CDB9` | `#C8322A` |
| Night | In-game chrome, letterbox, Paused, Bench | `#141414` | `#1D1C1A` | `#EDE6D6` | `#9C958A` | `#2F2C27` | `#E5483C` |

Pixel palette (in-game overlay; every channel is a multiple of 8, so each value is exact in
BGR555): cream `#F0E8D8` (0x6FBE), shade `#B8B0A0` (0x52D7), ink `#181818` (0x0C63), red `#D03828`
(0x14FA), dim `#686058` (0x2D8D), empty `#D8D0C0` (0x635B).

Contrast: fg2 on bg is at least 4.5:1 in both Paper and Night; spot on bg is at least 4.5:1 and may
carry text. Never put fg2 on bg2 for body text.

## Type

| Role | Face | Use |
|---|---|---|
| Display | Archivo, condensed (wdth ≈ 66) Black, UPPERCASE, line height 0.9 | screen titles, game titles, slot names, cart label titles |
| Interface | Archivo SemiBold/Bold 13–15 | buttons, options, list rows |
| Label | Archivo Bold 10.5, UPPERCASE, tracking 1.8, fg2 | field labels, section captions, header context |
| Reading | Source Serif 4, 13.5–15 | descriptions, help, notes: anything that reads as a sentence |
| Data | IBM Plex Mono 10.5–12.5 | game codes, frame numbers, times, sizes, file names, registers, keycaps |
| Pixel | bitmap font, Silkscreen-like 5×5 caps, 6 px advance, 8 px line | in-game overlay only |

All four outline faces are SIL Open Font License: ship the font files with the app and include
their `OFL.txt`. If the chosen GUI crate cannot select a variable-font axis, use the static
instances (`Archivo ExtraCondensed Black` for Display). The pixel font is our own glyph table in
source (uppercase, digits, `: · / - × ▸ ( ) . ,`); lowercase input is drawn as uppercase.

Numbers that people compare (frames, times, sizes) are always mono. Group frame numbers in threes
with a thin space or a space: `027 400`.

## Frame of every window-resolution screen

- **Head** (56 px, 40 px side margins, 1.5 px rule in fg underneath): section number in spot
  (Display 30), title (Display 22), optional context in Label style, flexible space, then
  right-aligned context (Data or Label). Section numbers: `01` Library, `02` Paused, `03` Save
  states, `04` Settings, `D` Bench.
- **Foot** (44 px, 1 px rule in line above): keycaps + verb for every key that works on this screen,
  left to right in order of importance; *Back*/*Quit* sits alone at the right.
- Body between them. Design size 1280×800; minimum 960×640; layouts are fluid in width, not scaled.

## Components

- **Keycap**: 22 px high, min 22 wide, 1.5 px fg border with a 3 px bottom border, radius 3, Data
  11 px semibold. Hover/active: border and text in spot. A keycap is also a rebind target in
  Settings.
- **Tick box**: 14 px square, 1.5 px fg border; checked = fg fill with a 2 px bg inset. Used for
  both single-choice and multi-choice options (the printed-form look); groups are laid out in a row.
- **Buttons**: primary = fg fill, bg text, 44 px high, no radius, with its key in Data at 65 %
  opacity (`Load F3`). Secondary = 1.5 px fg outline. Destructive = spot text, underlined, no box.
- **Cartridge** (Library): grey shell 156×108 with a ridged grip and a label inset; the label is
  game art (52 px wide strip) plus title (Display 13, max 2 lines), subtitle (1 line) and
  `CODE · SAVE` (Data 8.5, 1 line). Label colours come from `tokens.json color.label`, chosen by a
  stable hash of the game code. Hover lifts the cart 4 px in 120 ms.
- **Film strip** (Save states): `#161616` strip, sprocket holes in bands of 18 px top and bottom,
  frames at 1× (240×160) with a 12 px gap, edge print in Data 10.5 `#CFC6B4`:
  `▸ 4   FR 027 400   7:38`. Strips are tilted −0.4° / +0.3° alternately.
- **Grease-pencil ring**: a hand-drawn open ellipse (3.5 px, spot) around the selected frame. This
  is the only selection mark on that screen.
- **Loupe**: 250 px circle, 11 px dark ring, showing the selected frame at 3× centred on the
  pointer when it hovers a frame, else on the frame centre.
- **Pixel window** (in-game overlay, native px): 1 px ink, 1 px cream, 1 px shade border, cream
  fill, 4 px padding. Menu items on a 14 px pitch; the cursor is a 4×8 red triangle; the selected
  item's text is red. Slot thumbnails 48×32 with a 1 px ink outline, red when selected.
- **Dimming behind the overlay**: a brightness decrease on a *copy* of the frame, the same maths as
  the hardware's BLDY (`I − I·EVY/16`, EVY = 9). Never modify the core's framebuffer.

## Copy

- Plain words a player would use: "Picture size", not "Integer scaling factor"; "System program
  (BIOS)", not "BIOS mode: HLE".
- Sentence case for descriptions, UPPERCASE only where the style says so.
- Times as `m:ss` or `h:mm:ss` play time; dates as `today 14:22`, `yesterday 21:40`, else
  `Sep 28 21:04`.
- No exclamation marks, no emoji.

## Text that does not fit

The renders show where this bites (cart labels): `CRASH BANDICOOT` and `NFS UNDERGROUND 2`
overflow, and `FLASH 128K` wraps. Rules:
- Cart title: up to 2 lines at Display 13; if still too long, step down to 12 then 11; then
  truncate with `…`. Subtitle: 1 line, ellipsis. `CODE · SAVE`: 1 line; abbreviate the save type
  (`SRAM 32K`, `FL 64K`, `FL 128K`, `EEPROM`, `NONE`).
- Head context: ellipsis in the middle of long ROM names, never wrap.
- Pixel overlay: 38 characters per line at most; truncate with `…` (one glyph).
