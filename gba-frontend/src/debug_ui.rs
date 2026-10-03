//! Hotkeys and the debug actions behind them (IO dump, screenshot, savestates, title bar text).

use anyhow::Result;
use gba_core::Gba;
use minifb::Key;
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Hotkey {
    /// Esc or P: open/close the pause menu (pauses the game)
    Pause,
    /// N (while paused)
    FrameStep,
    /// F10
    DumpIo,
    /// F11
    Screenshot,
    /// F5
    SaveNow,
    /// F1: save the current slot
    SaveState,
    /// F3: load the current slot
    LoadState,
    /// F12: developer bench view
    Bench,
    /// F2: the save states screen
    SaveStates,
    /// =: more light on a solar-sensor cart (Boktai)
    SunUp,
    /// -: less light on a solar-sensor cart
    SunDown,
}

/// Default hotkeys; `HotkeyMap` carries the live (rebindable) copy.
pub const HOTKEYS: &[(Key, Hotkey)] = &[
    (Key::Escape, Hotkey::Pause),
    (Key::P, Hotkey::Pause),
    (Key::N, Hotkey::FrameStep),
    (Key::F10, Hotkey::DumpIo),
    (Key::F11, Hotkey::Screenshot),
    (Key::F5, Hotkey::SaveNow),
    (Key::F1, Hotkey::SaveState),
    (Key::F3, Hotkey::LoadState),
    (Key::F12, Hotkey::Bench),
    (Key::F2, Hotkey::SaveStates),
    (Key::Equal, Hotkey::SunUp),
    (Key::Minus, Hotkey::SunDown),
];

/// Live hotkey bindings (Settings can change them).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HotkeyMap {
    pub keys: Vec<(Key, Hotkey)>,
    /// Held key for fast-forward.
    pub fast_forward: Key,
}

impl Default for HotkeyMap {
    fn default() -> Self {
        HotkeyMap { keys: HOTKEYS.to_vec(), fast_forward: Key::Tab }
    }
}

impl HotkeyMap {
    /// Hotkeys among this frame's edge-triggered key presses.
    pub fn pressed(&self, input: &crate::ui::input::UiInput) -> Vec<Hotkey> {
        let mut out: Vec<Hotkey> = Vec::new();
        for (k, h) in &self.keys {
            if input.pressed(*k) && !out.contains(h) {
                out.push(*h);
            }
        }
        out
    }

    /// The first key bound to `h` (for keycaps).
    pub fn key_for(&self, h: Hotkey) -> Option<Key> {
        self.keys.iter().find(|(_, x)| *x == h).map(|(k, _)| *k)
    }

}

/// F10: print the IO register dump and any guest debug-log lines.
pub fn dump_io(gba: &mut Gba) {
    println!("--- frame {} cycle {} ---", gba.bus.ppu.frame_count, gba.bus.sched.now);
    print!("{}", gba.bus.dump_io());
    println!("{}", gba_test::cpu_state_line(gba));
    for line in gba.take_debug_log() {
        println!("[guest] {line}");
    }
}

/// Print guest debug-log lines without the IO dump (called every frame so nothing is lost).
pub fn drain_debug_log(gba: &mut Gba) {
    for line in gba.take_debug_log() {
        println!("[guest] {line}");
    }
}

/// `<rom stem>-<frame>.png` beside the ROM.
pub fn screenshot_path(rom: &Path, frame: u64) -> PathBuf {
    let stem = rom.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_else(|| "gba".into());
    rom.with_file_name(format!("{stem}-{frame}.png"))
}

/// F11: write the current framebuffer as PNG. Returns the path written.
pub fn screenshot(gba: &Gba, rom: &Path) -> Result<PathBuf> {
    let path = screenshot_path(rom, gba.bus.ppu.frame_count);
    gba_test::write_png(&path, &gba.framebuffer()[..])?;
    Ok(path)
}

/// Everything the title bar shows.
pub struct TitleInfo<'a> {
    pub game: &'a str,
    pub code: &'a str,
    pub fps: f64,
    pub paused: bool,
    pub fast_forward: bool,
    /// `Some((device_rate, underruns))` when audio is on.
    pub audio: Option<(u32, u64)>,
}

pub fn format_title(t: &TitleInfo) -> String {
    let mut s = format!("{} [{}] - {:.1} fps", t.game, t.code, t.fps);
    if t.paused {
        s.push_str(" [PAUSED]");
    }
    if t.fast_forward {
        s.push_str(" [FF]");
    }
    match t.audio {
        Some((rate, underruns)) => s.push_str(&format!(" - audio {} Hz, underruns {}", rate, underruns)),
        None => s.push_str(" - no audio"),
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn screenshot_path_beside_rom() {
        let p = screenshot_path(Path::new("C:/Games/Kingdom Hearts (USA).zip"), 1234);
        assert_eq!(p, PathBuf::from("C:/Games/Kingdom Hearts (USA)-1234.png"));
        let p = screenshot_path(Path::new("rom.gba"), 0);
        assert_eq!(p, PathBuf::from("rom-0.png"));
    }

    #[test]
    fn title_text() {
        let t = TitleInfo { game: "KINGDOM HEART", code: "B8CE", fps: 59.73, paused: false, fast_forward: false, audio: Some((48000, 2)) };
        assert_eq!(format_title(&t), "KINGDOM HEART [B8CE] - 59.7 fps - audio 48000 Hz, underruns 2");
        let t = TitleInfo { game: "X", code: "AAAA", fps: 0.0, paused: true, fast_forward: true, audio: None };
        assert_eq!(format_title(&t), "X [AAAA] - 0.0 fps [PAUSED] [FF] - no audio");
    }
}

