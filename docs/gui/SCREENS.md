# Screens

One section per mockup in `mockups/`. Each lists what the screen shows, where every value comes
from (**exists** = available in the code today, **new** = has to be built), what the keys do,
and the states the mockup does not show. Read `DESIGN.md` first.

Global key conventions (keyboard defaults; the gamepad mirrors them through the GBA mapping):
Z = confirm (GBA A), X = back (GBA B), arrows = move, A/S = previous/next page or slot (GBA L/R),
Enter = Start. Menus never react to keys that are mapped to the running game while it runs.

---

## 01 Library: `mockups/01-library.png` (Paper)

**Purpose.** Pick a game. Replaces "pass a ROM path on the command line" as the normal way in; a
path on the command line still boots straight into that game.

**Layout.** Head: `01 LIBRARY`, context `N cartridges · <folder>`, right: a Find field (underline
input, Label "FIND"). Body, left: the *case*, a dark rounded block with a foam grid of
cartridges, 4 columns at 1280 wide (3 at the minimum width), scrolling vertically. The slot of the
cart that is "in the console" (last played / selected) shows a dashed outline, "IN THE CONSOLE" and
its code. Body, right (≈436 px): the handheld line drawing (same drawing style as Settings) with
the selected cart sticking out of the top and the game's picture on its screen at 1×; under it, the
title (Display 34), subtitle, a details list, and the buttons `Play Z` (primary) and
`Resume state` (secondary, hidden if the game has no states). The A button on the drawing carries
the red ring and the Z keycap; that ring is the screen's one red selection mark.

**Data.**
- Game list: **new**, a scanner over the library folders (default: the folder of the ROM given on
  the command line, else `./Games`), `.gba` and `.zip` (use `gba_test::load_rom` for zips). Cache
  the results in an index file keyed by path + size + mtime so start-up does not re-read 32 MB ROMs.
- Title, code, version: **exists**, `gba_core::cart::Header::parse` (header title is 12 chars,
  e.g. `KINGDOM HEART`). Display name: the file name with region tags kept as the subtitle
  (`Kingdom Hearts - Chain of Memories (USA)` → title `Kingdom Hearts`, subtitle
  `Chain of Memories · USA`); fall back to the header title.
- Save type: **exists**, `Cart::detect_save_type(rom, &header)`; shown abbreviated (`DESIGN.md`).
- ROM size: file/ROM length.
- Label art: **new**. Order of preference: the thumbnail of the most recent save state; else a
  title-screen capture made in the background (run the ROM headless with HLE BIOS for 1800 frames,
  ≈1 s at current speed; cache as PNG in the cache folder); else a plain label (colour only).
- Label colours: a stable hash of the game code into `tokens.json color.label`.
- Last state line: from the slot metadata (see 03): `FR 027 400 · 7:38 · today 14:22`.
- The mockup's "Runs: to the battle tutorial" line is design filler. Replace it with
  `Played h:mm` (total play time, **new**, tracked per game in the index) or drop the row.

**Keys.** Arrows choose a cart · Z play · X eject (clears the selection) · F3 resume the latest state
· typing goes to Find (Find filters by title or code; Esc clears it) · Esc quit (when Find is empty).

**States.** Empty library (`docs/branding/boards/05-strawberry-empty-library.png`): the case
shows three rows of dashed placeholders, the console slot reading `STRAWBERRY` and the next cell
`EMPTY · PUT .GBA FILES IN <folder>/`; the handheld holds a Strawberry cartridge with the mark,
the wordmark and `NO GAMES YET` on its screen (composed at 240x160 in the Pixel palette); the
right column is the product card (`STRAWBERRY`, `Fruit Basket · No. 2`, GAMES / SYSTEM PROGRAM /
SAVES rows) with `Open <folder>  Z` and `Settings`. With no folder at all the one button is
`Settings  Z` and the hints say to add one; while scanning they say so. Unreadable ROM: the cart
gets a plain grey label with the file name and `UNREADABLE`; selecting it shows the error. Scan in
progress: carts appear as they are found; never block the window.

---

## 02 Paused: `mockups/02-paused.png` (Night + Pixel overlay)

**Purpose.** Everything you can do without leaving the game: resume, save/load a slot, settings,
bench, quit to the library.

**Layout.** Window: Night head `02 PAUSED`, game name, right `FR 027 400 · 7:38 · 59.73 fps`
(Data). The game at the largest integer scale that fits the body (4× = 960×640 at 1280×800),
centred on Night bg. Inside the frame, drawn at native 240×160 *before* upscaling:
- the frame dimmed (BLDY-style, EVY 9),
- main menu window at (8, 8) 84×112: `RESUME`, `SAVE STATE`, `LOAD STATE`, `SETTINGS`, `BENCH`,
  `QUIT`,
- submenu window at (100, 8) 132×112 for Save/Load: title `SAVE TO SLOT` / `LOAD SLOT`, a 2×2 grid
  of 48×32 thumbnails with `N  m:ss` under each; slot pages with A/S (slots 1–8 = two pages),
- a bar at (8, 128) 224×24: the selected slot's summary and the two actions (`A OVERWRITE  B BACK`).

The pixel layout is authoritative in native pixels; the mockup is that layout at 4×.

**Data.** Frame number: **exists**, `gba.bus.ppu.frame_count`. Play time = frame count / 59.7275.
FPS: **exists**, `pacing::FpsCounter`. Slot thumbnails and times: slot metadata (see 03).

**Keys.** P or Esc opens and closes the menu (Esc becomes "pause" in game; quitting is `QUIT` here
or closing the window; this is a behaviour change, confirm it with the user). Arrows move, Z
selects, X backs out one level, A/S change slot page, F12 opens Bench. While the menu is open the
game is paused and receives no input. N still frame-steps while paused (the menu hides during
the step and returns after it).

**States.** Saving shows `SAVED TO SLOT N` in the bar for 1.5 s. Loading a slot closes the menu and
resumes. A failed load shows the error in the bar (38 chars max) and keeps the menu open. Empty slot
in Load mode: not selectable.

---

## 03 Save states: `mockups/03-save-states.png` (Paper)

**Purpose.** See and manage every saved moment of one game.

**Layout.** Head `03 SAVE STATES`, game name, right `5 of 8 slots · 2.1 MB`. Left: film strips,
3 frames per strip at 1×, strips stacked with 26 px between them, scrolling vertically when slots
do not fit. Each frame: the thumbnail at 240×160, edge print `▸ N   FR 027 400   m:ss`. Empty
slots: a dark frame with "Empty" (Display 22) and `F1 saves here`. The selected frame has the
grease-pencil ring. Right: the loupe (3×), then `SLOT N` (Display 34) with the save date, a details
list (Frame, Play time, Size, File) and `Load F3` / `Save over` / `Delete this state`.

**Data (all new unless noted).**
- Slots 1–8 per game: `<rom stem>.s<N>.state` beside the ROM (same folder rule as `.sav`).
  Body = `Gba::save_state()` (**exists**, ≈429 KB). Write atomically with
  `saves::write_atomic` (**exists**).
- Sidecar per slot: thumbnail (the 240×160 BGR555 framebuffer at save time, stored as PNG via
  `gba_test::write_png`, **exists**) and metadata (frame count, saved-at timestamp, emulator
  version, state format version). Store them in the slot file's own header, or as
  `<stem>.s<N>.png` + a small text/TOML file. Pick one and document it in `docs/ARCHITECTURE.md`.
- Legacy `<rom>.state` (today's F1/F3 file, **exists**): if slot 1 is empty, copy it into slot 1
  on first open and leave the original untouched. The harness flags `--save-state/--load-state`
  keep working on explicit paths.
- Size: file size. Total in the head: sum of slot files.

**Keys.** Arrows move between frames · F3 or Z load (switches to the game) · F1 save the running
game over the selected slot (only when a game is running) · Del delete (asks first in a small Paper
dialog: "Delete slot 4? This cannot be undone." `Delete` / `Keep`) · X back.

**States.** No states: one strip with the 8 empty frames and the hint in Reading style. Corrupt or
version-mismatched slot: the frame shows the thumbnail desaturated with `CAN'T LOAD` in the edge
print; details show the reason; Load disabled; Delete still works.

---

## 04 Settings: `mockups/04-settings.png` (Paper)

**Purpose.** Controls and preferences, laid out as an open instruction manual (two pages per
spread). Tabs in the head: `Controls · Screen & sound`, `System`, `Saves`. A/S turn the spread.

**Left page: Controls.**
- The handheld line drawing with leader lines and red callout dots to every GBA button, each with
  its keycap(s): L `A`, R `S`, +Control Pad `↑ ↓ ← →`, A `Z`, B `X`, START `Enter`, SELECT
  `R Shift`. Source: `input::KEYBOARD_MAP` (**exists**).
- Rebinding (**new**): click or Z on a keycap → it shows `…` in spot → the next key pressed is bound
  (Esc cancels). A key that is already used elsewhere is swapped, and both keycaps flash once.
  Gamepad layout `input::GAMEPAD_MAP` (**exists**) is shown on a second spread or a toggle.
  (Built 2026-10-02 as a second keycap beside each keyboard keycap instead, rebound the same way
  by pressing a pad button; narrow pages stack the pair. Logged in PROGRESS.md.)
- "Emulator keys" with dotted leaders: Fast-forward (hold) Tab, Save state F1, Pause P, Load state
  F3, Advance one frame N, Write .sav now F5, Screenshot F11, Bench F12. Source:
  `debug_ui::HOTKEYS` (**exists**) plus F12 (**new**). IO dump F10 stays but is listed only in Bench.

**Right page: Screen & sound.** Numbered items (red numerals), each a title, a one-line description
in Reading style and tick boxes:
1. Picture size: 1× 2× 3× 4× / Largest that fits. Today: `--scale` (**exists**, CLI only).
2. Picture filter: None / LCD grid / Frame blend (**new** post-processing on the upscaled image or
   on a copy of the frame; never in the core).
3. Fast-forward limit: 2× / 4× / No limit (**new**; today Tab is unlimited with presents throttled
   by `FAST_FORWARD_PRESENT`).
4. Volume: a ruler 0–10 with a red marker (**new**: a gain stage in `audio.rs` before the ring).
5. System program (BIOS): Built-in / From file… (**exists**: `BiosMode::Hle` / `--bios` /
   `GBA_BIOS`).
Then the NOTE box about `.sav` vs save states.

**System and Saves spreads (not mocked; build them in the same style).** System: library folders
(add/remove), audio device and `--mute` / no-audio, pacing (audio-clocked, the default, or wall
clock). Saves: where `.sav` and states are written, autosave interval (`SAVE_INTERVAL_FRAMES`,
**exists**), open folder.

**Persistence (new).** One settings file in the user's config folder, human-readable (TOML or
JSON), loaded at start, written on change (atomically). Command-line flags override the file for
that run only. Unknown keys are kept on rewrite; missing keys fall back to defaults.

---

## D Bench: `mockups/D-bench.png` (Night), developer view

**Purpose.** For people debugging the emulator, not for players. Opened with F12 or `BENCH` in the
pause menu, only while a game is loaded; closed with F12. Players never see it unless they ask.

**Layout.** Head `D BENCH`, `Developer view · code · save · BIOS mode`, right
`■ HALTED  FR 27 400  VCOUNT 160` (spot only on the status). Left: the game at 3× (720×480), the
caption line, then **Scanlines**: a 228-line timeline of the last frame with rows for VBlank IRQ,
DMA1 (FIFO A), DMA2 (FIFO B), DMA3, timer 0 overflow, a shaded VBlank region (lines 160–227) and a
spot cursor at the current VCOUNT. Right column: **Picture** (DISPCNT, mode, and one thumbnail per
enabled layer with a show/hide tick), **Sound** (six channels with 32-segment meters and on/off
ticks; segments past 25 in spot), **CPU** (r0–r15, CPSR flags N Z C V I F T with T lit in Thumb,
mode, and four lines of disassembly around PC with the current line marked by a spot bar).

**Data.**
- Registers, CPSR, mode: **exists**, `gba.cpu.regs` (see `gba_test::cpu_state_line`).
- Disassembly: **exists**, `gba_core::cpu::disasm::{arm, thumb}`; read memory without side effects
  (`Cart::peek*` exists for ROM; add a side-effect-free bus peek if needed).
- DISPCNT: **exists**, `gba.bus.ppu.dispcnt()`. Frame/VCOUNT: **exists**.
- Run / step / FF / save / load / IO dump: **exist** as hotkeys (P, N, Tab, F1/F3, F10).
- Per-layer pictures, channel levels, layer and channel mute, the scanline event log: **new, in the
  core**. These touch `ppu/`, `apu/`, `dma.rs`, `irq.rs` and `timers.rs`, whose contracts are
  documented at the top of each `mod.rs`. Add them behind the existing debug-hook pattern
  (`bus/debug.rs`: one `enabled` flag, one branch on the hot path), off by default, never in
  savestates. Layer capture must record the per-line layer buffers as `render_scanline` draws them
  (mid-frame register changes must show), not re-render at end of frame.

**Gate for this screen.** Everything in `cargo test --workspace` still passes with the hooks off;
golden hashes in `tests/golden.toml` and `tests/compat.toml` unchanged; `--bench 1200` on KH within
2 % of the speed before the change (record both numbers in `PROGRESS.md`).

---

## In game (no mockup)

While a game runs: no chrome. The frame at the largest integer scale that fits, centred on Night bg
(the letterbox). Toasts (state saved, `.sav` written, screenshot path) are drawn in the pixel
overlay style as a single-line window at the bottom of the frame for 1.5 s. The window title keeps
today's text (`debug_ui::format_title`).
