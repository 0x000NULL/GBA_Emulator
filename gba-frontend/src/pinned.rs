//! Pinned output: xxh3-64 hashes of every screen's render, the in-game overlay, the window's
//! blit paths and the slot file bytes, plus the exact text of `settings.toml` and `index.toml`.
//! They guard refactors that must not change a single pixel or byte, such as the move of the
//! generic frontend code into the shared crates.
//!
//! Every input is synthetic (no `GBA_ROM`, no `Games/`, no disk paths with a process id) and
//! dates go through [`crate::ui::fmt::test_clock`], so the hashes hold on any host and time zone.
//! A failing pin prints the new hash. Re-pin only after reviewing the matching
//! `--ignored render_` PNG (or the printed text), and say why in the commit.

#![cfg(test)]

use crate::overlay::pause::{Mode, PauseMenu, SlotInfo};
use crate::overlay::{thumb_from_frame, Overlay};
use crate::ui::tokens::pixel::DIM_EVY;
use crate::ui::Canvas;
use gba_core::{FrameBuffer, SCREEN_H, SCREEN_W};
use std::time::Instant;

/// Unix time "now" for pinned renders: 2026-10-01 12:00:00 UTC.
pub const NOW: i64 = 1_790_856_000;

const PINS: &[(&str, u64)] = &[
    ("bench-idle", 0xCEC44F3A989DE90F),
    ("bench-running", 0xD02FB531C3D18B6D),
    ("library", 0xB70429B1E7F4047D),
    ("library-960", 0x76CA26BC47399E86),
    ("library-empty", 0x7AB885CB694D589B),
    ("library-empty-nofolder", 0xFA3D6D222951E955),
    ("library-unreadable", 0x0C59863B260A5536),
    ("overlay-load", 0x37882EA813DFFCB5),
    ("overlay-load-p2", 0x4BA5844C2821F586),
    ("overlay-main", 0x99A2D47457769970),
    ("overlay-message", 0xF7C1C8022AE71BA2),
    ("overlay-save", 0x72918DBE1D5A78DB),
    ("overlay-thumb", 0x3FE94E2CB45400F7),
    ("overlay-toast", 0xB35C878FCA2B6083),
    ("settings-960-controls", 0x49E38495BAC4AC58),
    ("settings-960-system", 0xC31A07A2A0DE2A80),
    ("settings-controls", 0xA9B3400648D96EE9),
    ("settings-rebind", 0xB2577C958C180F93),
    ("settings-saves", 0x7FAFF1239CDC8764),
    ("settings-system", 0xDFE03407EBC0847A),
    ("slot-file", 0x15EC6EB5758DD2F4),
    ("states", 0x43082D0160F208A2),
    ("states-960", 0x12E0C0515C158322),
    ("states-dialog", 0x46BE183FAD3FB336),
    ("states-scrolled", 0x09650FF9B531ADCC),
    ("window-frame-blend", 0xA03A7B06C30F7F2E),
    ("window-lcd-grid", 0xE9A668669DD5F470),
    ("window-lut", 0xF157671290AFE09C),
    ("window-scale4", 0x3DD0853A6CED2973),
];

pub fn hash_bytes(b: &[u8]) -> u64 {
    xxhash_rust::xxh3::xxh3_64(b)
}

pub fn canvas_hash(cv: &Canvas) -> u64 {
    hash_bytes(cv.data())
}

/// [`check`] a canvas; in record mode also write it to `target/ui-renders/pinned/<name>.png`.
pub fn check_canvas(name: &str, cv: &Canvas) {
    if std::env::var_os("STRAWBERRY_PIN_RECORD").is_some() {
        let d = crate::render_tests::out_dir().join("pinned");
        std::fs::create_dir_all(&d).unwrap();
        cv.save_png(&d.join(format!("{name}.png"))).unwrap();
    }
    check(name, canvas_hash(cv));
}

fn u32_hash(v: &[u32]) -> u64 {
    hash_bytes(&v.iter().flat_map(|p| p.to_le_bytes()).collect::<Vec<u8>>())
}

/// Fail unless `got` is the pinned hash for `name`. `STRAWBERRY_PIN_RECORD=1` prints every
/// hash as a `PINS` row instead (`--nocapture`), for collecting new pins after review.
pub fn check(name: &str, got: u64) {
    let want = PINS.iter().find(|(n, _)| *n == name).map(|p| p.1);
    if std::env::var_os("STRAWBERRY_PIN_RECORD").is_some() {
        println!("PIN (\"{name}\", 0x{got:016X}),{}", if want == Some(got) { "" } else { " // changed" });
        return;
    }
    assert!(
        want == Some(got),
        "pin {name}: got 0x{got:016X}, pinned {}. Review the render before re-pinning in src/pinned.rs",
        want.map_or("nothing".to_string(), |w| format!("0x{w:016X}"))
    );
}

/// A deterministic 240x160 BGR555 picture: a checkerboard with a gradient, varied by `seed`.
pub fn test_frame(seed: u32) -> Box<FrameBuffer> {
    let mut fb: Box<FrameBuffer> = vec![0u16; SCREEN_W * SCREEN_H].into_boxed_slice().try_into().unwrap();
    for y in 0..SCREEN_H {
        for x in 0..SCREEN_W {
            let checker = ((x / 16) + (y / 16) + seed as usize) % 2 == 0;
            let r = ((x * 31) / SCREEN_W) as u16;
            let g = ((y * 31) / SCREEN_H) as u16;
            let b = if checker { 24 } else { (seed * 5 % 32) as u16 };
            fb[y * SCREEN_W + x] = r | g << 5 | b << 10;
        }
    }
    fb
}

#[test]
fn pinned_overlay() {
    let base = test_frame(0);
    let mut slots: [Option<SlotInfo>; 8] = Default::default();
    for (i, frames) in [(0usize, 300u32), (1, 900), (2, 1500)] {
        let f = test_frame(i as u32 + 1);
        slots[i] = Some(SlotInfo { thumb: Box::new(thumb_from_frame(&f[..])), play_secs: frames as u64 / 60, frame: frames as u64, broken: None });
    }
    slots[4] = Some(SlotInfo { broken: Some("version".into()), ..slots[0].clone().unwrap() });
    let now = Instant::now();
    for (name, mode, sel, page) in [
        ("overlay-main", Mode::Main, 1usize, 0usize),
        ("overlay-save", Mode::Save, 1, 0),
        ("overlay-load", Mode::Load, 2, 0),
        ("overlay-load-p2", Mode::Load, 2, 1),
    ] {
        let mut fb = base.clone();
        let mut o = Overlay::new(&mut fb);
        o.dim(DIM_EVY);
        let menu = PauseMenu { sel, mode, page, slot_sel: 1, message: None };
        menu.draw(&mut o, &slots, now);
        check(name, gba_test::fb_hash(&fb[..]));
    }
    let mut fb = base.clone();
    let mut o = Overlay::new(&mut fb);
    o.toast("SAVED TO SLOT 2");
    check("overlay-toast", gba_test::fb_hash(&fb[..]));
    let mut fb = base.clone();
    let mut m = PauseMenu { sel: 0, mode: Mode::Main, page: 0, slot_sel: 0, message: None };
    m.show_message("NOTHING IN SLOT 4", now);
    m.draw(&mut Overlay::new(&mut fb), &slots, now);
    check("overlay-message", gba_test::fb_hash(&fb[..]));
    check("overlay-thumb", gba_test::fb_hash(&thumb_from_frame(&base[..])));
}

#[test]
fn pinned_window_blits() {
    use crate::window::{build_lut, frame_blend, lcd_grid, scale_into_at};
    let lut = build_lut();
    check("window-lut", u32_hash(&lut));
    let fb = test_frame(3);
    let (w, h) = (1280usize, 800usize);
    let mut out = vec![0x141414u32; w * h];
    scale_into_at(&fb[..], &lut, 4, &mut out, w, 160, 80);
    check("window-scale4", u32_hash(&out));
    lcd_grid(&mut out, w, 160, 80, 4, SCREEN_W, SCREEN_H);
    check("window-lcd-grid", u32_hash(&out));
    let mut prev = vec![0x00FF00FFu32; w * h];
    frame_blend(&mut out, &mut prev);
    check("window-frame-blend", u32_hash(&out));
}

#[test]
fn pinned_slot_file() {
    let fb = test_frame(5);
    let bytes = crate::slots::encode(27_400, 1_790_000_000, 458, &fb[..], b"core state body");
    // The emulator version string sits at byte 38 and moves header_len (bytes 8..12) with its
    // length; both follow the release, so they are checked here and left out of the hash.
    let name = env!("CARGO_PKG_VERSION").as_bytes();
    assert_eq!(&bytes[36..38], &(name.len() as u16).to_le_bytes());
    assert_eq!(&bytes[38..38 + name.len()], name);
    let header_len = u32::from_le_bytes(bytes[8..12].try_into().unwrap()) as usize;
    assert_eq!(header_len, 38 + name.len() + SCREEN_W * SCREEN_H * 2);
    let mut stable = bytes[..8].to_vec();
    stable.extend_from_slice(&bytes[12..36]);
    stable.extend_from_slice(&bytes[38 + name.len()..]);
    check("slot-file", hash_bytes(&stable));
}

#[test]
fn pinned_settings_file() {
    use crate::settings::Settings;
    use std::path::PathBuf;
    let dir = std::env::temp_dir().join(format!("strawberry-pin-settings-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let src = dir.join("in.toml");
    // an unknown key and table survive; a bad value falls back to its default
    std::fs::write(&src, "volume = 99\nfilter = \"lcd_grid\"\nfuture_key = \"kept\"\n\n[future_table]\nx = 1\n").unwrap();
    let mut s = Settings::load(&src);
    s.library_folders = vec![PathBuf::from("C:/Games"), PathBuf::from("D:/Emulation/GBA")];
    s.audio_device = Some("Speakers (Realtek Audio)".into());
    s.window = Some((1280, 800));
    let out = dir.join("settings.toml");
    s.save(&out).unwrap();
    let text = std::fs::read_to_string(&out).unwrap();
    let _ = std::fs::remove_dir_all(&dir);
    assert_eq!(text, SETTINGS_TOML, "settings.toml changed:\n{text}");
}

const SETTINGS_TOML: &str = r#"# Strawberry settings. Edited by the Settings screen; unknown keys are kept.
audio = "on"
audio_device = "Speakers (Realtek Audio)"
autosave_frames = 60
bios = "built-in"
ff_limit = "x4"
filter = "lcd_grid"
future_key = "kept"
library_folders = [
    "C:/Games",
    "D:/Emulation/GBA",
]
pacing = "audio_clocked"
picture = "largest"
volume = 10
window = [
    1280,
    800,
]

[future_table]
x = 1

[gamepad]
A = "South"
B = "East"
DOWN = "D↓"
L = "LB"
LEFT = "D←"
R = "RB"
RIGHT = "D→"
SELECT = "Select"
START = "Start"
UP = "D↑"

[hotkeys]
fast_forward = "Tab"

[hotkeys.keys]
bench = "F12"
dump_io = "F10"
frame_step = "N"
load_state = "F3"
pause = "P"
save_now = "F5"
save_state = "F1"
save_states = "F2"
screenshot = "F11"

[keymap]
A = "Z"
B = "X"
DOWN = "↓"
L = "A"
LEFT = "←"
R = "S"
RIGHT = "→"
SELECT = "R Shift"
START = "Enter"
UP = "↑"
"#;
