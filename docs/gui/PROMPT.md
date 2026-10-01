You are building a desktop GUI for this Game Boy Advance emulator (Rust workspace: `gba-core`,
`gba-test`, `gba-frontend`). The design is finished. Your job is to implement it faithfully,
in phases, without changing how the emulator emulates.

## Read first, in this order

1. `CLAUDE.md`: project rules. They override anything below: gates are gates, ROMs and BIOS are
   never committed, commits are authored by Ethan Aldrich with **no attribution trailers**, commit
   and push after every green gate.
2. `docs/gui/README.md`: what is in this package.
3. `docs/gui/DESIGN.md`: the design language (paper, ink, one red), its rules and components.
4. `docs/gui/mockups/*.png`: the six reference screens at 1280×800. Look at every one.
5. `docs/gui/SCREENS.md`: per screen, what it shows, where each value comes from (exists / new),
   keys and the states the mockups do not show.
6. `docs/gui/IMPLEMENTATION.md`: architecture, the backend contract, phases G0–G7 and their gates.
7. `docs/gui/tokens.json` and `docs/gui/drawings/*.svg`: exact values and vector drawings.
8. The current frontend (`gba-frontend/src/*.rs`), `docs/ARCHITECTURE.md` (Frontend, Savestates,
   Debug tooling) and the top of `PROGRESS.md`.

## The one architectural rule

The GUI crate is **not chosen yet** and the design must not depend on one. All UI logic (theme,
pixel overlay, library, slots, settings, keymap, app state machine, view-models) goes in a new
`gba-ui` crate whose `Cargo.toml` contains no window, GPU, GUI or font crate. Only
`gba-frontend/src/backend/` may import the chosen GUI crate, through the `Backend` trait in
`IMPLEMENTATION.md`. If you find yourself wanting a GUI type in `gba-ui`, change the trait instead.

## How to work

- Follow the phases in `IMPLEMENTATION.md` in order. Each ends with its gate passing headless in
  `cargo test --workspace`, a commit and a push. Never weaken, skip or special-case a test, and
  never re-pin a golden hash without reviewing the PNG first.
- **Stop at the end of G0** and ask me: which GUI crate (give a short neutral comparison against
  the checklist, with versions you verified), the threading/pacing approach that crate implies,
  and the six behaviour changes listed in `IMPLEMENTATION.md`. Write the answers to
  `docs/gui/DECISIONS.md` before writing any code.
- G1–G4 do not need the GUI crate. Do them before G5 so the crate is only wiring.
- **Ask before starting G7** (Bench). It is the only phase that touches `gba-core`; follow the
  contracts at the top of each `mod.rs` and keep hooks off by default and out of savestates.
- Treat the mockups as the visual target and `DESIGN.md` as the rules. Where they conflict,
  `DESIGN.md` wins. Where either conflicts with real emulator behaviour, keep the behaviour, adapt
  the design, and note it in `PROGRESS.md`. The mockups contain sample data (play times, the "Runs"
  line, register values); never hard-code it.
- Keep the existing debug workflow intact: every hotkey in `debug_ui.rs`, the `--bench` and
  `--exit-after` flags, `gba-test` and its `--save-state/--load-state`.
- Performance matters: in game, the overlay is composited into a copy of the 240×160 frame, and
  nothing else may slow the emulation loop. Compare against the G0 baseline numbers.
- Builds from parallel shells use their own `CARGO_TARGET_DIR`.

## At the end of each phase, tell me

- what changed (files, new public items), in a few lines;
- the gate: the exact commands you ran and their results;
- for G2 and G5, the paths of the review PNGs in `target/gui-review/` and the differences from the
  mockups you saw;
- anything you decided or adapted, and what the next phase needs from me.

Start with G0.
