//! Number, time and size formatting rules from `docs/gui/DESIGN.md` ("Copy").

use chrono::{DateTime, Datelike, Local};
use gba_core::SaveType;
use std::time::SystemTime;

/// Frame numbers are grouped in threes with a space and padded to six digits: `027 400`.
pub fn fmt_frames(frames: u64) -> String {
    let s = format!("{frames:06}");
    let bytes = s.as_bytes();
    let mut out = String::with_capacity(s.len() + s.len() / 3);
    let first = bytes.len() % 3;
    for (i, b) in bytes.iter().enumerate() {
        if i != 0 && (i + 3 - first) % 3 == 0 {
            out.push(' ');
        }
        out.push(*b as char);
    }
    out
}

/// Play time as `m:ss`, or `h:mm:ss` from one hour.
pub fn fmt_play(secs: u64) -> String {
    let (h, m, s) = (secs / 3600, (secs / 60) % 60, secs % 60);
    if h > 0 {
        format!("{h}:{m:02}:{s:02}")
    } else {
        format!("{m}:{s:02}")
    }
}

/// Total play time for the library row: `h:mm`.
pub fn fmt_played(secs: u64) -> String {
    format!("{}:{:02}", secs / 3600, (secs / 60) % 60)
}

/// Frames to seconds of play at the GBA frame rate.
pub fn frames_to_secs(frames: u64) -> u64 {
    (frames as f64 / crate::config::GBA_FPS).round() as u64
}

/// Dates: `today 14:22`, `yesterday 21:40`, else `Sep 28 21:04`.
pub fn fmt_when(t: SystemTime) -> String {
    fmt_when_at(DateTime::<Local>::from(t), Local::now())
}

pub fn fmt_when_at(t: DateTime<Local>, now: DateTime<Local>) -> String {
    let day = t.date_naive();
    let today = now.date_naive();
    let time = t.format("%H:%M");
    if day == today {
        format!("today {time}")
    } else if today.pred_opt() == Some(day) {
        format!("yesterday {time}")
    } else if day.year() == today.year() {
        format!("{} {time}", t.format("%b %-d"))
    } else {
        format!("{} {time}", t.format("%b %-d %Y"))
    }
}

/// Sizes: `429 KB`, `2.1 MB`, `32 MB` (whole megabytes when exact).
pub fn fmt_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = 1024 * 1024;
    if bytes < MB {
        format!("{} KB", bytes.div_ceil(KB).max(1))
    } else if bytes.is_multiple_of(MB) {
        format!("{} MB", bytes / MB)
    } else {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    }
}

/// Save type abbreviations for the cartridge label and the Bench head.
pub fn abbrev_save(t: SaveType) -> &'static str {
    match t {
        SaveType::None => "NONE",
        SaveType::Sram32K => "SRAM 32K",
        SaveType::Flash64K => "FL 64K",
        SaveType::Flash128K => "FL 128K",
        SaveType::Eeprom => "EEPROM",
    }
}

/// Longer save type names for the library details row.
pub fn save_name(t: SaveType) -> &'static str {
    match t {
        SaveType::None => "no save",
        SaveType::Sram32K => "SRAM 32K",
        SaveType::Flash64K => "FLASH 64K",
        SaveType::Flash128K => "FLASH 128K",
        SaveType::Eeprom => "EEPROM",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn frames_are_grouped_in_threes() {
        assert_eq!(fmt_frames(27_400), "027 400");
        assert_eq!(fmt_frames(1_800), "001 800");
        assert_eq!(fmt_frames(0), "000 000");
        assert_eq!(fmt_frames(1_234_567), "1 234 567");
    }

    #[test]
    fn play_time_formats() {
        assert_eq!(fmt_play(0), "0:00");
        assert_eq!(fmt_play(458), "7:38");
        assert_eq!(fmt_play(3600), "1:00:00");
        assert_eq!(fmt_play(3661), "1:01:01");
        assert_eq!(fmt_played(5400), "1:30");
        assert_eq!(frames_to_secs(27_400), 459);
    }

    #[test]
    fn dates_relative_to_now() {
        let now = Local.with_ymd_and_hms(2026, 9, 30, 15, 0, 0).unwrap();
        let t = Local.with_ymd_and_hms(2026, 9, 30, 14, 22, 0).unwrap();
        assert_eq!(fmt_when_at(t, now), "today 14:22");
        let t = Local.with_ymd_and_hms(2026, 9, 29, 21, 40, 0).unwrap();
        assert_eq!(fmt_when_at(t, now), "yesterday 21:40");
        let t = Local.with_ymd_and_hms(2026, 9, 28, 21, 4, 0).unwrap();
        assert_eq!(fmt_when_at(t, now), "Sep 28 21:04");
        let t = Local.with_ymd_and_hms(2025, 12, 1, 9, 5, 0).unwrap();
        assert_eq!(fmt_when_at(t, now), "Dec 1 2025 09:05");
    }

    #[test]
    fn sizes() {
        assert_eq!(fmt_size(429 * 1024), "429 KB");
        assert_eq!(fmt_size(100), "1 KB");
        assert_eq!(fmt_size(2_202_009), "2.1 MB");
        assert_eq!(fmt_size(32 * 1024 * 1024), "32 MB");
        assert_eq!(fmt_size(8 * 1024 * 1024), "8 MB");
    }

    #[test]
    fn save_abbreviations() {
        assert_eq!(abbrev_save(SaveType::Flash128K), "FL 128K");
        assert_eq!(abbrev_save(SaveType::Eeprom), "EEPROM");
        assert_eq!(abbrev_save(SaveType::None), "NONE");
    }
}
