# Architecture

## Ownership tree
```
Gba { cpu: Cpu, bus: Bus }
Bus { sched, irq, ppu, apu, dma, timers, keypad, cart, bios, ewram, iwram, waitcnt, sio, open_bus, debug, ... }
```
No `Rc<RefCell>`, no trait objects. `Cpu::step(&mut self, bus: &mut impl MemoryBus)` executes one
instruction. Subsystem event handlers receive `SysCtx { sched, irq, dma }`, never `&mut Bus`.
Cross-subsystem effects go through the bus's event dispatcher (`Bus::process_events`).

Module map (gba-core/src): `cpu/` (regs, alu, decode tables, arm, thumb, disasm, exceptions),
`ppu/` (regs views, bg_text, bg_affine, bitmap, obj, compose; timing contract in mod.rs),
`apu/` (regs, psg, fifo, mixer), `bus/` (memory map + waitstates in mod.rs, IO dispatch in io.rs,
DMA execution in dma_exec.rs, debugger hooks in debug.rs), `dma.rs`, `timers.rs`, `irq.rs`,
`keypad.rs`, `cart/` (header, save detection, SRAM/flash/eeprom), `bios/` (LLE image + HLE SWIs in
hle.rs, cycle costs in swi_timing.rs, guest callbacks in call.rs, the sound driver in sound/),
`sched.rs`, `timing.rs`, `gba.rs` (main loop), `state.rs` (savestates, feature-gated).

## MemoryBus (the CPU's view of the world)
`read8/16/32(addr, Access)`, `write8/16/32`, `fetch16/32` (opcode fetch; updates open bus and
`last_fetch`), `idle(cycles)`, `irq_pending()`, `now()`, plus the HLE hooks `is_hle_bios()`,
`swi_hle(num, &mut Regs)`, `bios_hle_vector(&mut Regs)`. The bus advances the clock by the
waitstate cost of every access; the CPU never computes waitstates. `Access::N`/`S` select the
non-sequential/sequential ROM waitstate from WAITCNT. The CPU passes **unaligned** addresses
through: RAM regions align them, the 8-bit SRAM/Flash bus uses the low bits to pick the byte lane.

### CPU/HLE contract
- IRQ sampled at the top of `step`; exception entry sets SPSR/LR, ARM state, I flag, PC=vector.
- `swi n`: call `bus.swi_hle(n, regs)` with `regs.r[15]` = address of the following instruction.
  Returns true when serviced (no exception). Waiting SWIs (IntrWait) rewind r15 and halt so the SWI
  re-executes after the interrupt; a flag in `Bios` suppresses the "discard old flags" step on re-entry.
- PC < 0x4000 under HLE: `bus.bios_hle_vector(regs)` emulates the BIOS IRQ dispatcher (vector 0x18:
  push r0-r3,r12,lr; r0=0x04000000; lr=0x138; pc=[0x03007FFC]) and the return stub at 0x138
  (pop; `subs pc, lr, #4`); 0x150 is a plain `bx lr`; 0x200+4i (i < 36) are the SoundGetJumpList
  routines (`bios/sound/jumplist.rs`, r0 = player, r1 = track, return via lr).
- HLE routines call guest code (SoundDriverMain runs the game's music player and CGB routine) by
  pushing a `Continuation` on `Bios::calls`, setting lr = 0x140 and jumping (`bios/call.rs`); the
  trap at 0x140 pops it and runs the routine's next stage, and the final stage restores the
  caller's r0-r3, r12, lr, CPSR and PC. Callbacks from an SWI run in System mode (the real
  handler's mode), IRQs stay live during them, calls nest LIFO, and the stack is savestated.
  The driver keeps all of its state in the game's work area (pointer at 0x03007FF0).
- BIOS-region reads from outside the BIOS return the GBATEK open-bus values (E129F000 after boot,
  E3A02004 after SWI, E25EF004 during IRQ, E55EC002 after IRQ return).
- Pipeline: two fetched words; the executing instruction came from an earlier fetch, so a store
  over PC+8 does not change what runs (Classic NES Series depends on it). PC writes flush; the step that wrote
  PC ends with the refill (an N then an S fetch), so an IRQ sampled at the next step boundary is
  taken after the branch/exception-return has paid for it.
- Fetch `Access`: the CPU passes `S` only when the bus cycle before the fetch was the fetch of the
  preceding opcode. After a data access (STR/STM: the datasheet's "2N") or an internal cycle
  (LDR/LDM/MUL/shift-by-register: GBATEK's "prefetch disable bug") it passes `N`, and the first
  halfword of every 128K game pak block is `N` for fetches and LDM/STM words alike. With the
  prefetch buffer enabled (WAITCNT bit 14, `bus/prefetch.rs`) the bus decides ROM opcode-fetch cost
  by address and ignores the flag on a buffer hit; a miss uses the flag as usual.

## BIOS sound driver (bios/sound/)
SWI 1Ah-1Eh, 28h, 29h and 2Ah plus the 36 SoundGetJumpList routines, written clean room from
GBATEK, the games' observed memory traffic and the published m4a structure layouts (no BIOS
code). `area.rs` holds the work-area layout (`SoundInfo` header, twelve 0x40-byte channels at
+0x50, two 0x630-byte PCM halves at +0x350: right for FIFO A, left for FIFO B), the channel status
bits and the twelve-rate table derived from GBATEK's frequencies (timer period = 16777216/Hz,
samples per V-Blank = 280896/period, DMA period = 0x630/samples). `mod.rs` has the SWI bodies:
Init programs SOUNDCNT_X/H, SOUNDBIAS, timer 0 and both FIFO DMAs; VSync counts V-Blanks and
restarts the DMAs at the buffer start every period (prime four words immediately, stop, re-enable
in FIFO mode, the library's own sequence); Main locks the area (`ident + 1`), calls the game's
`func(intp)` and `CgbSound` through `call.rs`, then mixes. `mixer.rs` writes one frame into ring
slot `(period - count + 1) % period` with the GBATEK envelope (STOP stays set during the release),
loops, nearest-sample resampling, fixed-rate voices (`type` bit 3, copied 1:1), reverb from the
slot being overwritten and master-volume scaling. `jumplist.rs` serves the trap addresses
0x200+4i: slots 0-29 are the track commands (r0 = player, r1 = track, `cmdPtr` already past the
command byte, return via lr, `flags == 0` ends the track), 30-35 the helpers; `player.rs` holds
the `MusicPlayerInfo`/`MusicPlayerTrack` offsets. All driver state lives in the game's work area
(pointer at 0x03007FF0), so savestates and the game see one picture. The design choices that
have no hardware reference are listed in PROGRESS.md's fix log.

## Scheduler and main loop
`Scheduler { now, heap }` with `Event { HBlank, LineEnd, TimerOverflow(n), DmaStart(n), ApuSample,
ApuFrameSeq, KeypadPoll, IrqLatch }`. `Gba::run_until(target)`:
```
while now < target:
    if now >= next_event:  process_events(); continue     # deadline re-read every iteration
    if dma.pending:        dma_run(); continue            # CPU stalled
    if halted/stopped:     wake if (IE & IF) != 0 else now = min(next_event, target); continue
    cpu.step(bus)                                         # may overshoot by one instruction
```
Re-reading the deadline every iteration matters: an IO write can schedule an event two cycles
ahead (immediate DMA start), and games read the DMA result on the very next instruction.

Every periodic handler schedules its successor from the event's **scheduled** time (`at`), never
from `now`, so CPU/DMA overshoot cannot lengthen a frame or drift the APU sample clock.

## DMA (dma.rs, bus/dma_exec.rs)
Registers and trigger matching live in `DmaController`; execution in `Bus::dma_run`, lowest channel
first. Cost 2N + 2(n-1)S + xI (x = 2, or 4 with both addresses in the game pak); immediate mode
starts 2 cycles after the enable write via `Event::DmaStart`. Transfers are **resumable**: after
every unit the bus fires due events, and if a higher-priority channel became pending the running
one saves `src/dst/remaining` and yields (`in_progress`), so sound FIFO DMAs are served during long
graphics copies. FIFO mode forces 4 words to a fixed destination; the request comes from
`Apu::on_timer_overflow` when a FIFO drops to 16 bytes. A DMA into the EEPROM region hints the
EEPROM size from its length (9/73 halfwords = 512 B, 17/81 = 8 KB).

## Timing (timing.rs, ppu/mod.rs)
1232 cycles/line, H-Blank flag at 1006, 228 lines, 160 visible, V-Blank flag lines 160..=226,
280,896 cycles/frame (59.7275 Hz). `HBlank` renders line `vcount` (if < 160) with the registers as
they are at that moment, sets the flag, raises the H-Blank IRQ on every line, triggers H-Blank DMA
for lines < 160 and video-capture DMA for lines 2..=162. `LineEnd(at)` clears the flag, increments
VCOUNT, handles the V-Count match, sets V-Blank (IRQ + DMA + frame ready) at 160, clears it at 227,
latches BG2/BG3 reference points at line 0, and schedules the next `HBlank`/`LineEnd` from `at`.
HLE boot starts at VCOUNT 0x7E with LineEnd 117 cycles away (mGBA GBASkipBIOS) so early traces align.
Timers are event-driven (one overflow event per running timer; reads derive the count from time;
cascade timers are bumped by the previous timer's overflow).

## Memory map and quirks (bus/mod.rs)
BIOS 0x00 (16K, protected), EWRAM 0x02 (256K, 16-bit bus, 3 cycles/halfword), IWRAM 0x03 (32K),
IO 0x04 (dispatch in bus/io.rs; mGBA debug regs at 0x04FFF600/700/780), PAL 0x05, VRAM 0x06
(96K mirrored into 128K), OAM 0x07, ROM 0x08-0x0D (three waitstate regions; out-of-range reads
return `(addr >> 1) & 0xFFFF`; per-cart mirroring for game code `F`), SRAM/Flash 0x0E-0x0F (8-bit
bus, byte replicated on wider reads, byte lane selected by address on wider writes; 0xFF when no
chip). 8-bit writes: PAL and BG-VRAM duplicate into the halfword, OBJ-VRAM and OAM ignore them.
EEPROM is reached through 0x0D (whole region for carts <= 16 MB, 0x0DFFFF00+ for larger).

## Harness (gba-test)
`load_rom` (gba/zip), `fb_hash` = xxh3-64 of the framebuffer as LE u16s, `checks::check_jsmolka`
(spin loop + r12 == 0), `checks::check_mgba_suite` (parse `END: p/t` from the debug log),
`golden::GoldenFile` (`tests/golden.toml` and `tests/compat.toml`; entries take `check`, `reg`,
`golden`, `frames`, inline `input` steps or an `input_file`, `expect`, `audio_frames_min` (frames
with a sample above +-64 that the run must reach: `gba_test::audio_active`, the sweep's measure),
`skip`), `input_script::InputScript` (`{frame, press/release}` with `A+START` combos;
`tests/inputs/triage.toml` is the sweep's schedule). `tests/roms.rs` runs every entry under
`cargo test`; compat entries skip when the ROM is absent.
CLI extras: `--stats-from F` (underruns/samples counted from frame F), `--pcm` (WAV at the
SOUNDBIAS rate), `--mute-channels MASK` (silence PSG/FIFO channels in the mix through the bench
capture hooks; emulation is unchanged), `--fifo-stats F` (per-frame FIFO pushed/popped/underruns), `--save-state/--load-state`,
`--record` (prints a manifest stanza), `--triage` (the sweep's input schedule and one-line
classification summary), `--log-swi` (every HLE SWI, sound-driver Init/Mode and jump-list call as
a `guest:` line), `--dump-mem LO..HI`, `--watch-read LO..HI`.

## Savestates (feature `savestate`)
Every state struct carries `#[cfg_attr(feature = "savestate", derive(Serialize, Deserialize))]`;
`state.rs` provides byte-array helpers for the big buffers and `Gba::save_state/load_state`
(magic `GBAS`, version, bincode body). Not stored: ROM, BIOS image (reattached from the live
machine), framebuffer and PPU scratch (regenerated), audio sample queue, debug hooks. Resuming
KH:CoM from a state reproduces the straight run's framebuffer hash exactly. Format 4 (2026-10-01)
added the HLE call stack `Bios::calls`, so a state saved while the sound driver waits on the
game's music player resumes it; a version mismatch is rejected (the frontend shows such slots as
`CAN'T LOAD`).

## Frontend (gba-frontend)
The GUI follows `docs/gui/` (DESIGN.md is the design language, SCREENS.md the per-screen
contract, tokens.json the constants, mockups/ the reference renders). It is a software renderer on
the minifb window: no widget toolkit.

**Generic and GBA halves.** The console-independent half of the frontend lives in the shared
`fruit-basket` crates (public repo `0x000NULL/fruit-basket`, also used by the crabapple NES
emulator): `basket-ui` is `ui` (tokens with `tokens.json`, canvas, text and the fonts, widgets,
drawings, mark, fmt, input) and `basket-app` is `audio`, `pacing`, `saves`, `prefs`, `slots`,
`icon`, `library` and `pads` (gamepads to per-port masks; Strawberry uses `Ports::Shared`);
`basket-build` embeds the exe icon from `build.rs`. `main.rs` imports them under the old module
names, so app code still says `crate::ui::...`. The app hands them the console through small
types and traits:
`ui::Screen` (picture size), a colour table for `Canvas::blit_indexed` (`pixels::bgr555_lut`),
`ui::input::ButtonSet` (button names, mask bits, `MenuRoles` for the menus; `input::GBA_BUTTONS`),
`audio::Buffering`, `saves::SaveRam`, `library::Platform` (ROM extensions, `probe -> RomInfo`,
slot art, title capture; `glue::GbaPlatform`), `slots::SlotFormat` + `slots::Snapshot`
(`glue::GBA_SLOTS`), `icon::Icon` (`brand::ICON`), the app name and folder for `prefs`, and the
GBA-sized tokens as functions of the picture (`component::film_frame(screen, 1)`,
`pixel::thumb_size(screen, 5)`). `glue.rs` implements the traits for `gba_core` through the
`glue::Machine` wrapper (both trait and type are foreign to the app crate); `gba_copy.rs` holds the GBA save type names and
ids; `brand.rs` the Strawberry mark, icons and handheld drawings. `src/pinned.rs` pins the
frontend's output (render hashes, slot bytes, settings and index text) so the split is checked
byte for byte.

- `app.rs`: the state machine. `Screen` is Library (01), Game (02, with the pixel pause overlay),
  SaveStates (03), Settings (04) or Bench (D). `Session` owns the running `Gba`, its `.sav`
  manager, slot set, audio, pacer and overlay state. Only Game (unpaused) and Bench (running)
  call `Session::step`. `App::update` reacts to the frame's `UiInput`, `App::render` presents.
- `ui/` (basket-ui): `tokens` (constants, checked against tokens.json by a test), `canvas` (tiny-skia pixmap
  for vector shapes, direct pixel writes for text and framebuffer blits, clip rect, conversion to
  minifb's 0RGB), `text` (seven embedded OFL faces rasterised by fontdue, glyph cache, ellipsis
  and wrapping), `widgets` (keycap, tick, buttons, head/foot frame, tabs, numbered items, ruler),
  `drawings` (a parser for the SVG subset of the line drawings), `mark` (a pixel mark parsed with
  that parser into a 16x16 four-colour sprite, paper and night BGR555 textures; `brand.rs` holds
  the Strawberry mark from `docs/branding/logo/strawberry-mark.svg`, the embedded `.ico` and PNG
  icons, the name strings and the handheld drawings; the empty Library draws the mark), `fmt`
  (copy rules), `input` (per-frame snapshot; `KeyMap` and `PadMap`, the rebindable keyboard and
  gamepad layouts over a `ButtonSet`; menu navigation derives from the bindings through
  `MenuRoles` so a rebinding moves the menus too; gamepad edge/repeat).
- `overlay/`: everything drawn over a running game is drawn at native 240x160 into a *copy* of
  the frame in the Pixel palette (5x5 bitmap font, windows, dim = BLDY maths with EVY 9, 48x32
  thumbnails = exact 1/5 samples, toasts) and scaled with it. `overlay::pause` is the pause menu
  state machine.
- `slots.rs`: eight save-state slots per game, `<rom stem>.s<N>.state` beside the ROM. One file
  per slot: little-endian header (`GBSL`, header version, header length, frame count, saved-at
  unix seconds, play seconds, core `STATE_VERSION`, emulator version string, 240x160 BGR555
  picture) followed by `Gba::save_state()` bytes; the magic, versions and picture size are
  `glue::GBA_SLOTS` (a `SlotFormat`). Headers and pictures are read without the body; a
  core-version mismatch marks the slot `CAN'T LOAD` (deletable, never loaded); the pause menu
  derives its 48x32 thumbnails from the pictures (`pause::slot_infos`). The legacy single
  `<rom>.state` is imported into slot 1 on first scan when slot 1 is empty (`glue::scan_slots`;
  a scratch machine supplies the picture); the original is left alone. Writes go through
  `saves::write_atomic`.
- `screens/`: `library` (01), `states` (03), `settings` (04), `bench` (D), each a struct with
  `update(&UiInput, ctx) -> Outcome` and `render(&mut Canvas, ctx)`; the app owns the wiring.
  The bench switches the core capture on only while it is open (`Gba::set_bench`).
- `window.rs` (resizable minifb window; `present_game` is the fast path: BGR555 LUT + integer
  upscale onto the Night letterbox, optional LCD-grid / frame-blend filters; `present_canvas`
  for the window-resolution screens; `gather_input`), `input.rs` (default keyboard and gamepad
  tables; gilrs polling returns the key mask through the live `PadMap` plus the buttons that went
  down, for rebinding),
  `audio.rs` (SPSC ring, linear resampler from the core's SOUNDBIAS rate to the device rate, cpal
  stream, underrun counter, a gain stage in the callback for the volume setting), `pacing.rs`
  (1/59.7275 s frame clock, adaptive sleep margin, +-2% audio-fill correction), `saves.rs`
  (atomic `<rom>.sav` writes every 60 frames/F5/exit), `debug_ui.rs` (hotkey map, IO dump,
  screenshots, title bar), `settings.rs` (Strawberry's fields of the TOML settings file at
  `<config dir>/strawberry/`; `prefs.rs` reads and writes it: a malformed value costs only its
  key, unknown keys preserved; keyboard, gamepad and hotkey bindings stored by name; CLI flags
  override for the run only; `App::new` first moves a pre-1.3.0 `gba-advance/` config or cache
  folder to `strawberry/` when the new one is absent), `library/` (folder scan thread, display
  names from file names, index cache under `<cache dir>/strawberry/`, background title captures
  for the cartridge labels; the GBA parts behind `library::Platform`), `icon.rs` (the window icon: Windows loads the embedded `.ico` after
  it is written to the cache folder, X11 gets `_NET_WM_ICON` from the icon PNGs; Wayland and
  macOS are skipped because minifb's `set_icon` panics there), and `build.rs` (on Windows only,
  the exe's icon and version block through `winresource`; a missing `rc.exe` warns, never fails).
  Runs without an audio device. Review renders: `cargo test -p gba-frontend --release --
  --ignored render_` writes every screen as PNG under `target/ui-renders/`; the committed copies
  in `docs/screenshots/` are what the README shows.

## Debug tooling
`--trace` writes mGBA's register layout (sixteen `%08X`, then `cpsr: %08X`, r15 = PC+8/PC+4) per
instruction; `--trace-hash` writes `cycle pc hash`. `--break PC`, `--watch LO..HI` and
`--watch-read LO..HI` (with `--watch-log` to log every hit instead of stopping), `--dump-mem LO..HI`,
`--exit-on-swi N`, `--dump-io`, `--png`, `--log-swi`. Hooks live in `bus/debug.rs` behind a single
`enabled` flag so the hot path pays one branch. Guest `mgba` debug prints, HLE SWI calls and the
sound driver's `sound init/mode` and `jl slot=` lines are collected and echoed as `guest:` lines.
The BIOS-region PC traps (0x18, 0x138, 0x140, 0x150, 0x200+4i) are listed under the CPU/HLE
contract. Reference audio: `scripts/build-mgba.ps1` builds `tools/mgba-wav.exe` (libmgba, public
core API only) and `scripts/compare-audio.py` records a title set with it and with `gba-test
--pcm` (full, PSG-only, FIFO-only via `--mute-channels`), then reports alignment, correlation and
per-component gain fits; findings live in PROGRESS.md.

Bench capture (the developer "Bench" view: per-layer pictures, channel levels, scanline event log,
layer/channel mutes) follows an ownership rule. The PPU and APU cannot reach `DebugHooks`
(`render_scanline` and `on_sample` take no context), so each owns its capture behind
`pub debug: Option<Box<..>>` (`ppu::capture::PpuCapture`, `apu::capture::ApuCapture`) and the Bus
owns the published `BenchCapture` in `DebugHooks::bench`. `Bus::set_bench` attaches or drops all
three together. Capture is written during rendering/mixing; the Bus publishes once per frame at
the `LineEnd` that makes `vcount == 160` (where `Gba::run_frame` returns), swapping the PPU's
finished `LayerFrame` in and snapshotting sound state. Nothing is serialised (savestate bytes and
`STATE_VERSION` are unchanged; `load_state` carries the captures over like the other debug state).
Mutes only edit what is composed or mixed (the layer set handed to `compose_line`, the SOUNDCNT
copies handed to the mixer), never registers, FIFOs or underrun counts, so a muted run is
indistinguishable to the guest. With the bench off the cost is one `is_some()` per rendered line,
audio sample, line end, Timer 0 overflow and DMA start; the bench is deliberately not part of
`DebugHooks::enabled`, which also gates the per-access watch scans.
