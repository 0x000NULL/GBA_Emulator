//! Hotkeys and the debug actions behind them (IO dump, screenshot, savestates, title bar text).

use anyhow::Result;
use gba_core::Gba;
use minifb::{Key, KeyRepeat, Window};
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Hotkey {
    /// Esc
    Quit,
    /// P
    TogglePause,
    /// N (while paused)
    FrameStep,
    /// F10
    DumpIo,
    /// F11
    Screenshot,
    /// F5
    SaveNow,
    /// F1: write `<rom>.state`
    SaveState,
    /// F3: load `<rom>.state`
    LoadState,
}

pub const HOTKEYS: &[(Key, Hotkey)] = &[
    (Key::Escape, Hotkey::Quit),
    (Key::P, Hotkey::TogglePause),
    (Key::N, Hotkey::FrameStep),
    (Key::F10, Hotkey::DumpIo),
    (Key::F11, Hotkey::Screenshot),
    (Key::F5, Hotkey::SaveNow),
    (Key::F1, Hotkey::SaveState),
    (Key::F3, Hotkey::LoadState),
];

/// Hotkeys pressed since the last window update (edge-triggered, no auto-repeat).
pub fn poll_hotkeys(window: &Window) -> Vec<Hotkey> {
    HOTKEYS.iter().filter(|(k, _)| window.is_key_pressed(*k, KeyRepeat::No)).map(|(_, h)| *h).collect()
}

/// Held keys that act as modifiers rather than events.
pub fn fast_forward_held(window: &Window) -> bool {
    window.is_key_down(Key::Tab)
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

/// `<rom>.state` beside the ROM.
pub fn state_path(rom: &Path) -> PathBuf {
    rom.with_extension("state")
}

/// F1: write the machine state beside the ROM.
pub fn save_state(gba: &Gba, rom: &Path) -> Result<PathBuf> {
    let p = state_path(rom);
    crate::saves::write_atomic(&p, &gba.save_state())?;
    Ok(p)
}

/// F3: restore the machine state written by F1.
pub fn load_state(gba: &mut Gba, rom: &Path) -> Result<PathBuf> {
    let p = state_path(rom);
    let bytes = std::fs::read(&p).map_err(|e| anyhow::anyhow!("reading {}: {e}", p.display()))?;
    gba.load_state(&bytes).map_err(|e| anyhow::anyhow!("{e}"))?;
    Ok(p)
}
