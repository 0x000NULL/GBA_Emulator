# Implementing the GUI

How to build the screens in `SCREENS.md` without tying the design to one GUI crate. The crate is
chosen in phase 0 (with the user) and lives behind one trait; everything else is plain Rust that
builds and tests without it.

## Shape

```
gba-core/       unchanged until phase 7 (Bench hooks)
gba-test/       unchanged (its load_rom, write_png, fb_hash, bgr555_to_rgb are reused)
gba-ui/         NEW lib crate. Depends on gba-core (+ savestate), gba-test, serde and one of
                toml/serde_json. MUST NOT depend on any window, GPU, GUI or font crate; its
                Cargo.toml is the enforcement.
  theme.rs        tokens.json as Rust consts (Palette {bg, bg2, fg, fg2, line, spot}, sizes, pixel BGR555)
  pixel/          native 240x160 overlay: bitmap font, window, menu, slot grid, toast, dim (BLDY)
  library.rs      folder scan, header parse, save type, label art cache, index file
  slots.rs        slots 1..8: paths, save/load/delete, thumbnail + metadata, legacy migration
  settings.rs     Settings struct, defaults, load/save, CLI overrides
  keymap.rs       bindings for GBA keys and hotkeys, rebinding, conflicts
  app.rs          the state machine: Screen, Action, App::update(Input) -> Vec<Effect>
  view/           one view-model per screen: plain structs of strings, numbers, image handles,
                  already truncated/abbreviated per DESIGN.md (via a TextMeasure trait)
  backend.rs      the Backend trait below; no implementation here
gba-frontend/   the binary. Keeps audio.rs, pacing.rs, saves.rs, input.rs (gilrs); window.rs is
                replaced by backend/<crate>.rs, the ONLY module that imports the GUI crate.
```

Why a separate crate: "no GUI types outside the backend" is a rule a reviewer forgets and a
compiler does not. It also lets `cargo test -p gba-ui` run the whole UI logic headless.

## The backend contract

Start from this and refine it in phase 0 once the crate is chosen; keep it crate-neutral.

```rust
pub trait Backend {
    /// Present one frame and return what happened since the previous call: raw input already
    /// translated to our types, plus Actions for widgets the crate handled itself (clicks,
    /// text edits). Immediate-mode crates produce Actions while drawing; retained-mode crates
    /// produce them from their own event handlers. Both come back through here.
    fn frame(&mut self, frame: &Frame<'_>) -> anyhow::Result<Vec<Input>>;
    /// Current drawable size in physical pixels, and the scale factor.
    fn size(&self) -> (u32, u32, f32);
}

pub struct Frame<'a> {
    pub palette: Palette,                  // Paper or Night
    pub game: Option<GameLayer<'a>>,       // the emulator picture, if a game is loaded
    pub screen: Option<&'a ScreenVm>,      // Library / States / Settings / Bench view-model; None in game
}

pub struct GameLayer<'a> {
    /// 240x160 BGR555 with the pixel overlay already composited (a copy; never the core's buffer).
    pub pixels: &'a [u16],
    /// Integer scale and destination rectangle, computed by gba-ui (layout is not the backend's job).
    pub scale: u32,
    pub dest: Rect,
}

pub enum Input {
    Key { key: KeyCode, down: bool, repeat: bool },  // our own KeyCode enum, not the crate's
    Text(String),
    Pointer { x: f32, y: f32, button: Option<PointerButton>, down: bool },
    Wheel { dy: f32 },
    Action(Action),                                  // a widget was activated
    Resized { w: u32, h: u32 },
    FileDropped(std::path::PathBuf),
    CloseRequested,
}
```

Gamepads stay in `input.rs` (gilrs) and feed the same `Input`/`Keys` paths; they are not the GUI
crate's job.

### What the chosen crate must do (phase 0 checklist)

- Upload a 240×160 (or pre-scaled) image every frame and draw it with **nearest-neighbour**
  sampling at an integer scale, without blurring or stretching.
- Load bundled TTF/OTF fonts (Archivo incl. a condensed Black instance, Source Serif 4,
  IBM Plex Mono), several sizes, measure text (for ellipsis), letter-spacing or a workaround.
- Rectangles, 1–3.5 px strokes, rounded rectangles, circles, polylines/paths (the drawings in
  `drawings/*.svg` can instead be pre-rasterised at build time), images with alpha.
  Soft shadows and the ±0.4° film tilt are nice to have; drop them if unsupported.
- Keyboard with key-down/key-up and repeat flag, text input (Find field), pointer and wheel.
- Resizable window, minimum 960×640, HiDPI-aware.
- **Pacing:** must not force vsync or its own frame cap onto the emulation. Today the main loop
  owns timing (`pacing.rs`: audio-clocked ±2 %, sleep-then-spin) and calls `present` when it
  wants. Either the crate lets us keep owning the loop (pump events, present on demand), or
  emulation moves to its own thread, publishes finished frames (triple buffer) and the UI thread
  draws the latest one. Decide which in phase 0 and write it down.
- Builds on Windows (the primary platform, MSVC) and Linux without system packages beyond what
  `cpal`/`gilrs` already need. Note any C/C++ dependency and the licence.

### Candidates to put to the user (do not decide alone)

Present these neutrally against the checklist, with what you verified (versions, docs), and ask
which to use. Others are welcome if they meet the checklist.

| Approach | Loop ownership | Notes to verify |
|---|---|---|
| Keep minifb, draw everything in software (e.g. a 2D rasteriser + a font rasteriser) | ours | smallest change; no text input/IME or clipboard from minifb; we draw every widget |
| winit + a pixel/softbuffer surface + software drawing | ours (pump events) | like the above with better input/HiDPI; still self-drawn widgets |
| An immediate-mode GUI (e.g. egui via eframe or a custom winit integration) | crate or ours | fastest to build the screens; custom look needs styling work; texture filtering options |
| A retained/declarative GUI (e.g. iced, slint) | crate | strong layout/text; check licence terms, theming depth and custom drawing |

## Data formats (decide, then document in `docs/ARCHITECTURE.md`)

- **Settings**: one human-readable file in the platform config folder (`%APPDATA%\<App>\` on
  Windows, `$XDG_CONFIG_HOME/<app>/` on Linux); or "portable" beside the executable when a
  `portable` marker file exists there. Atomic writes (`saves::write_atomic`).
- **Library index + label art cache**: platform cache folder; safe to delete.
- **Slots**: `<rom stem>.s<N>.state` beside the ROM, plus thumbnail and metadata (frame count,
  saved-at, app version, state format version). Body is `Gba::save_state()` unchanged, so
  `gba-test --load-state` can still read the body (document how if the slot file wraps it).
- Never write inside `Games/` anything except `.sav`, slots and their sidecars (same as today).

## Phases and gates

Each phase ends green on `cargo test --workspace` and is committed (author Ethan Aldrich, **no
attribution trailers**) and pushed, per `CLAUDE.md`. Add a "GUI" milestone table to
`PROGRESS.md` (G0–G7) and fill it in as you go, with dates and anything non-obvious you learned.

| | Phase | Gate (headless, in `cargo test` unless stated) |
|---|---|---|
| G0 | **Baseline and decisions.** Read everything; record baseline: test count, `--bench 1200` fps on a ROM from `Games/` if present, and a windowed `--exit-after 3600` run's late frames and audio underruns. Propose crates; ask the user to choose; confirm the behaviour changes below. Write `docs/gui/DECISIONS.md`. | Decisions file exists and the user has answered. No code yet. |
| G1 | **`gba-ui` skeleton + theme.** Crate, workspace entry, `theme.rs`. | A test parses `docs/gui/tokens.json` (dev-dependency) and checks every colour and pixel BGR555 value against `theme.rs`. |
| G2 | **Pixel overlay.** Font, window, menu, slot grid, toast, dim. | Unit tests (glyph widths, truncation at 38 chars, dim maths matches `I − I·EVY/16` per channel). Golden: compose the Paused overlay onto a fixed synthetic frame, write a PNG to `target/gui-review/`, **review it against `mockups/02-paused.png` before pinning** its `fb_hash` in a test. |
| G3 | **Services.** library, slots, settings, keymap. | Synthetic ROMs in temp dirs (header bytes at 0xA0–0xBD, a `.zip`); index cache invalidation on size/mtime; slot round trip through a real `Gba` built from a synthetic ROM (pattern: `saves.rs` round-trip test); legacy `<rom>.state` copied to slot 1 only when slot 1 is empty; settings round trip, defaults, unknown keys preserved, CLI overrides. |
| G4 | **App model and view-models.** | Flow tests: Library → Play → Esc/P → Save state → slot 2 → Resume; Load from States; Back from every screen; F12 only with a game loaded; text-fitting rules from `DESIGN.md` using a fixed-width `TextMeasure`. |
| G5 | **Backend.** Implement `Backend` with the chosen crate; wire `App` into `gba-frontend`; delete `window.rs` once nothing uses it. | `cargo build --release`, all tests. Manual (record in PROGRESS): screenshot each screen at 1280×800 into `target/gui-review/` and compare with `docs/gui/mockups/` side by side, listing differences; repeat the G0 windowed run, late frames and underruns no worse than baseline. |
| G6 | **Settings features.** Rebinding, picture filter, fast-forward limit, volume. | Pure functions tested: gain on i16 stereo (no clipping wrap), FF limiter period, LCD-grid and frame-blend on buffers, keymap conflict swap. |
| G7 | **Bench (optional; ask before starting).** Core debug hooks + screen, per `SCREENS.md`. | Hooks off: all goldens unchanged, `--bench 1200` within 2 % of G0. Hooks on: unit tests for layer capture (mid-frame register change visible), channel peaks, event log ordering. |

## Behaviour changes to confirm with the user in G0

1. Running `gba` with no ROM opens the Library (today the ROM argument is required).
2. Esc in game opens the pause menu instead of quitting (quit = menu `QUIT` or closing the window;
   Esc still quits from the Library).
3. F1/F3 act on the *selected slot* (default 1) instead of the single `<rom>.state`.
4. The window becomes resizable with integer-scaled, letterboxed video.
5. The app's display name. The mockups use "ADVANCE" as a placeholder.
6. Where settings live (config folder vs portable).

## Out of scope

- Any change to emulation behaviour, timing or savestate contents (Bench hooks excepted, and
  those are off by default and never serialised).
- Box art downloads, online anything, achievements, netplay, shaders beyond the two filters.
- Committing ROMs, BIOS dumps, screenshots of commercial games or label-art caches.
