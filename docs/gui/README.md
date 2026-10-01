# GUI design package

Everything needed to build the emulator's GUI. It assumes no particular GUI crate; the crate is
picked at the start of the work.

| File | What it is |
|---|---|
| `PROMPT.md` | The prompt to give Claude Code. It points at everything else. |
| `DESIGN.md` | The design language: rules, palettes, type, components, copy, text fitting. |
| `SCREENS.md` | One spec per screen: layout, data sources (exists / new), keys, unmocked states. |
| `IMPLEMENTATION.md` | Architecture (`gba-ui` crate + `Backend` trait), crate checklist, phases G0–G7 with gates. |
| `tokens.json` | Colours (incl. BGR555 for the in-game overlay), fonts, sizes, component metrics. |
| `drawings/*.svg` | The handheld line drawings and the grease-pencil ring, with placement notes. |
| `mockups/*.png` | The six reference screens at 1280×800: 00 system, 01 library, 02 paused, 03 save states, 04 settings, D bench. |

The pixel art in the mockups (castle hall, meadow, road, dungeon, night) is original placeholder
art standing in for real game frames. The app shows the actual game's frames and state thumbnails.

## Using it with Claude Code

From the repository root, with this folder at `docs/gui/`:

```powershell
claude (Get-Content docs/gui/PROMPT.md -Raw)
```

```bash
claude "$(cat docs/gui/PROMPT.md)"
```

Or run `/gui` inside Claude Code (the command is in `.claude/commands/gui.md`). Add a phase to
resume later, e.g. `/gui continue with G3`.

Claude stops after G0 to ask you which GUI crate to use and to confirm six behaviour changes
(listed in `IMPLEMENTATION.md`). It also asks before G7 (Bench), the only phase that touches
`gba-core`.
