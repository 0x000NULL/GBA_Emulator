//! Save-state slots 1..=8 per game: `<rom stem>.s<N>.state` beside the ROM.
//!
//! File format (little-endian; documented in `docs/ARCHITECTURE.md`):
//! ```text
//! "GBSL"            magic
//! u32  header_ver   1
//! u32  header_len   bytes from the start of the file to the core state
//! u64  frame        ppu.frame_count at save time
//! i64  saved_at     unix seconds
//! u32  play_secs    play time at save time
//! u32  core_ver     gba_core::state::STATE_VERSION the body was written with
//! u16  name_len + name bytes (emulator version string)
//! u16[240*160]      thumbnail, BGR555
//! ...               Gba::save_state() bytes
//! ```
//! One file keeps writes atomic (`saves::write_atomic`) and deletes simple; `read_meta` reads the
//! header and thumbnail without touching the body. The legacy single `<rom>.state` written by
//! earlier builds is imported into slot 1 the first time the slots are scanned, if slot 1 is empty.

use crate::overlay::{thumb_from_frame, Thumb};
use crate::saves::write_atomic;
use anyhow::{anyhow, Result};
use gba_core::{Gba, FrameBuffer, SCREEN_H, SCREEN_W};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

pub const SLOTS: usize = 8;
const MAGIC: &[u8; 4] = b"GBSL";
const HEADER_VER: u32 = 1;
const FRAME_LEN: usize = SCREEN_W * SCREEN_H;

/// Slot metadata read from a slot file's header.
#[derive(Clone, Debug, PartialEq)]
pub struct SlotMeta {
    pub slot: u8,
    pub path: PathBuf,
    pub frame: u64,
    pub saved_at: SystemTime,
    pub play_secs: u64,
    pub core_ver: u32,
    pub emulator: String,
    pub size: u64,
    pub thumb: Box<Thumb>,
    pub frame_full: Box<FrameBuffer>,
    /// `Some(reason)` when the body cannot be loaded by this build.
    pub broken: Option<String>,
}

#[derive(Debug)]
pub enum SlotError {
    Missing,
    Io(std::io::Error),
    Corrupt(String),
    /// The file is readable but the body cannot be loaded (core version mismatch etc.).
    CantLoad(String),
}

impl std::fmt::Display for SlotError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SlotError::Missing => write!(f, "empty slot"),
            SlotError::Io(e) => write!(f, "{e}"),
            SlotError::Corrupt(s) => write!(f, "corrupt state: {s}"),
            SlotError::CantLoad(s) => write!(f, "can't load: {s}"),
        }
    }
}

impl std::error::Error for SlotError {}

impl From<std::io::Error> for SlotError {
    fn from(e: std::io::Error) -> Self {
        SlotError::Io(e)
    }
}

/// `<rom stem>.s<N>.state` beside the ROM.
pub fn slot_path(rom: &Path, slot: u8) -> PathBuf {
    let stem = rom.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_else(|| "gba".into());
    rom.with_file_name(format!("{stem}.s{slot}.state"))
}

/// The legacy single-state path written by earlier builds.
pub fn legacy_path(rom: &Path) -> PathBuf {
    rom.with_extension("state")
}

fn now_secs() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0)
}

/// Serialise a slot file.
pub fn encode(frame: u64, saved_at: i64, play_secs: u32, fb: &[u16], body: &[u8]) -> Vec<u8> {
    let name = env!("CARGO_PKG_VERSION").as_bytes();
    let header_len = 4 + 4 + 4 + 8 + 8 + 4 + 4 + 2 + name.len() + FRAME_LEN * 2;
    let mut out = Vec::with_capacity(header_len + body.len());
    out.extend_from_slice(MAGIC);
    out.extend_from_slice(&HEADER_VER.to_le_bytes());
    out.extend_from_slice(&(header_len as u32).to_le_bytes());
    out.extend_from_slice(&frame.to_le_bytes());
    out.extend_from_slice(&saved_at.to_le_bytes());
    out.extend_from_slice(&play_secs.to_le_bytes());
    out.extend_from_slice(&gba_core::state::STATE_VERSION.to_le_bytes());
    out.extend_from_slice(&(name.len() as u16).to_le_bytes());
    out.extend_from_slice(name);
    for p in fb.iter().take(FRAME_LEN) {
        out.extend_from_slice(&(p & 0x7FFF).to_le_bytes());
    }
    out.extend_from_slice(body);
    out
}

struct Parsed {
    frame: u64,
    saved_at: i64,
    play_secs: u32,
    core_ver: u32,
    emulator: String,
    fb: Box<FrameBuffer>,
    body_at: usize,
}

fn parse_header(bytes: &[u8]) -> Result<Parsed, SlotError> {
    let need = |n: usize| -> Result<(), SlotError> {
        if bytes.len() < n {
            Err(SlotError::Corrupt(format!("file is {} bytes, header needs {n}", bytes.len())))
        } else {
            Ok(())
        }
    };
    need(12)?;
    if &bytes[0..4] != MAGIC {
        return Err(SlotError::Corrupt("bad magic".into()));
    }
    let u32_at = |i: usize| u32::from_le_bytes(bytes[i..i + 4].try_into().unwrap());
    let ver = u32_at(4);
    if ver != HEADER_VER {
        return Err(SlotError::Corrupt(format!("header version {ver}")));
    }
    let header_len = u32_at(8) as usize;
    need(header_len)?;
    let frame = u64::from_le_bytes(bytes[12..20].try_into().unwrap());
    let saved_at = i64::from_le_bytes(bytes[20..28].try_into().unwrap());
    let play_secs = u32_at(28);
    let core_ver = u32_at(32);
    let name_len = u16::from_le_bytes(bytes[36..38].try_into().unwrap()) as usize;
    let name_end = 38 + name_len;
    let fb_end = name_end + FRAME_LEN * 2;
    if fb_end != header_len {
        return Err(SlotError::Corrupt("header length mismatch".into()));
    }
    let emulator = String::from_utf8_lossy(&bytes[38..name_end]).into_owned();
    let mut fb: Box<FrameBuffer> = vec![0u16; FRAME_LEN].into_boxed_slice().try_into().map_err(|_| SlotError::Corrupt("fb".into()))?;
    for (i, p) in fb.iter_mut().enumerate() {
        let o = name_end + i * 2;
        *p = u16::from_le_bytes([bytes[o], bytes[o + 1]]);
    }
    Ok(Parsed { frame, saved_at, play_secs, core_ver, emulator, fb, body_at: header_len })
}

/// Read a slot's header and thumbnail (not its body).
pub fn read_meta(rom: &Path, slot: u8) -> Result<SlotMeta, SlotError> {
    let path = slot_path(rom, slot);
    let bytes = match std::fs::read(&path) {
        Ok(b) => b,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Err(SlotError::Missing),
        Err(e) => return Err(e.into()),
    };
    let size = bytes.len() as u64;
    let p = parse_header(&bytes)?;
    let broken = if p.core_ver != gba_core::state::STATE_VERSION {
        Some(format!("saved with state format {}, this build reads {}", p.core_ver, gba_core::state::STATE_VERSION))
    } else if bytes.len() <= p.body_at {
        Some("no state body".into())
    } else {
        None
    };
    Ok(SlotMeta {
        slot,
        path,
        frame: p.frame,
        saved_at: UNIX_EPOCH + std::time::Duration::from_secs(p.saved_at.max(0) as u64),
        play_secs: p.play_secs as u64,
        core_ver: p.core_ver,
        emulator: p.emulator,
        size,
        thumb: Box::new(thumb_from_frame(&p.fb[..])),
        frame_full: p.fb,
        broken,
    })
}

/// Write the running machine into `slot`.
pub fn save(gba: &Gba, rom: &Path, slot: u8, play_secs: u64) -> Result<SlotMeta> {
    let path = slot_path(rom, slot);
    let body = gba.save_state();
    let bytes = encode(gba.bus.ppu.frame_count, now_secs(), play_secs.min(u32::MAX as u64) as u32, &gba.framebuffer()[..], &body);
    write_atomic(&path, &bytes)?;
    read_meta(rom, slot).map_err(|e| anyhow!("{e}"))
}

/// Load `slot` into the running machine.
pub fn load(gba: &mut Gba, rom: &Path, slot: u8) -> Result<SlotMeta, SlotError> {
    let path = slot_path(rom, slot);
    let bytes = match std::fs::read(&path) {
        Ok(b) => b,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Err(SlotError::Missing),
        Err(e) => return Err(e.into()),
    };
    let p = parse_header(&bytes)?;
    if bytes.len() <= p.body_at {
        return Err(SlotError::CantLoad("no state body".into()));
    }
    gba.load_state(&bytes[p.body_at..]).map_err(|e| SlotError::CantLoad(e.to_string()))?;
    read_meta(rom, slot)
}

pub fn delete(rom: &Path, slot: u8) -> Result<()> {
    let path = slot_path(rom, slot);
    match std::fs::remove_file(&path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(anyhow!("deleting {}: {e}", path.display())),
    }
}

/// All eight slots of a game.
#[derive(Clone, Debug, Default)]
pub struct SlotSet {
    pub slots: Vec<Option<SlotMeta>>,
}

impl SlotSet {
    /// Scan the slot files; import the legacy `<rom>.state` into slot 1 first if that is empty.
    /// `rom_bytes` is needed only for the import (to build a scratch machine for the thumbnail).
    pub fn scan(rom: &Path, rom_bytes: Option<&[u8]>) -> Self {
        if let Some(bytes) = rom_bytes {
            if let Err(e) = import_legacy(rom, bytes) {
                eprintln!("warning: legacy state import: {e:#}");
            }
        }
        let mut slots = Vec::with_capacity(SLOTS);
        for n in 1..=SLOTS as u8 {
            slots.push(match read_meta(rom, n) {
                Ok(m) => Some(m),
                Err(SlotError::Missing) => None,
                Err(e) => {
                    eprintln!("warning: slot {n}: {e}");
                    // keep a stub so the file still shows up as unloadable
                    Some(SlotMeta {
                        slot: n,
                        path: slot_path(rom, n),
                        frame: 0,
                        saved_at: UNIX_EPOCH,
                        play_secs: 0,
                        core_ver: 0,
                        emulator: String::new(),
                        size: std::fs::metadata(slot_path(rom, n)).map(|m| m.len()).unwrap_or(0),
                        thumb: Box::new([0; crate::ui::tokens::pixel::THUMB_W * crate::ui::tokens::pixel::THUMB_H]),
                        frame_full: vec![0u16; FRAME_LEN].into_boxed_slice().try_into().unwrap(),
                        broken: Some(e.to_string()),
                    })
                }
            });
        }
        SlotSet { slots }
    }

    pub fn get(&self, slot: u8) -> Option<&SlotMeta> {
        self.slots.get(slot as usize - 1).and_then(|s| s.as_ref())
    }

    pub fn set(&mut self, meta: SlotMeta) {
        let i = meta.slot as usize - 1;
        if i < self.slots.len() {
            self.slots[i] = Some(meta);
        }
    }

    pub fn clear(&mut self, slot: u8) {
        if let Some(s) = self.slots.get_mut(slot as usize - 1) {
            *s = None;
        }
    }

    pub fn used(&self) -> usize {
        self.slots.iter().filter(|s| s.is_some()).count()
    }

    pub fn total_bytes(&self) -> u64 {
        self.slots.iter().flatten().map(|s| s.size).sum()
    }

    /// Most recently saved loadable slot.
    pub fn latest(&self) -> Option<&SlotMeta> {
        self.slots.iter().flatten().filter(|s| s.broken.is_none()).max_by_key(|s| s.saved_at)
    }

    /// The pause menu's view of the slots.
    pub fn infos(&self) -> [Option<crate::overlay::pause::SlotInfo>; 8] {
        let mut out: [Option<crate::overlay::pause::SlotInfo>; 8] = Default::default();
        for (i, s) in self.slots.iter().enumerate().take(8) {
            out[i] = s.as_ref().map(|m| crate::overlay::pause::SlotInfo {
                thumb: m.thumb.clone(),
                play_secs: m.play_secs,
                frame: m.frame,
                broken: m.broken.clone(),
            });
        }
        out
    }
}

/// Copy `<rom>.state` into slot 1 when slot 1 is empty; the original is left untouched.
pub fn import_legacy(rom: &Path, rom_bytes: &[u8]) -> Result<bool> {
    let legacy = legacy_path(rom);
    if !legacy.is_file() || slot_path(rom, 1).exists() {
        return Ok(false);
    }
    let body = std::fs::read(&legacy)?;
    // a scratch machine supplies the thumbnail and frame count
    let mut scratch = Gba::new(rom_bytes.to_vec(), gba_core::Options::default()).map_err(|e| anyhow!("{e}"))?;
    scratch.load_state(&body).map_err(|e| anyhow!("legacy state: {e}"))?;
    scratch.run_frame();
    let saved_at = std::fs::metadata(&legacy)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_secs() as i64)
        .unwrap_or_else(now_secs);
    let play = crate::ui::fmt::frames_to_secs(scratch.bus.ppu.frame_count);
    let bytes = encode(scratch.bus.ppu.frame_count, saved_at, play as u32, &scratch.framebuffer()[..], &body);
    write_atomic(&slot_path(rom, 1), &bytes)?;
    println!("imported {} into slot 1", legacy.display());
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use gba_core::Options;

    fn test_rom() -> Vec<u8> {
        // a minimal cart: header + an infinite loop at the entry point
        let mut rom = vec![0u8; 0x1000];
        rom[0..4].copy_from_slice(&0xEAFF_FFFEu32.to_le_bytes()); // b .
        rom[0xA0..0xAC].copy_from_slice(b"SLOTTEST\0\0\0\0");
        rom[0xAC..0xB0].copy_from_slice(b"TSTE");
        rom[0xB2] = 0x96;
        rom
    }

    fn temp_rom(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("gba-slots-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir.join("game.gba")
    }

    #[test]
    fn paths_beside_the_rom() {
        let p = slot_path(Path::new("C:/Games/Kingdom Hearts (USA).zip"), 4);
        assert_eq!(p, PathBuf::from("C:/Games/Kingdom Hearts (USA).s4.state"));
        assert_eq!(legacy_path(Path::new("C:/Games/x.gba")), PathBuf::from("C:/Games/x.state"));
    }

    #[test]
    fn round_trip_save_load_delete() {
        let rom = temp_rom("rt");
        let mut gba = Gba::new(test_rom(), Options::default()).unwrap();
        for _ in 0..3 {
            gba.run_frame();
        }
        let frame = gba.bus.ppu.frame_count;
        let meta = save(&gba, &rom, 2, 458).unwrap();
        assert_eq!((meta.slot, meta.frame, meta.play_secs), (2, frame, 458));
        assert!(meta.broken.is_none());
        assert_eq!(meta.core_ver, gba_core::state::STATE_VERSION);
        assert_eq!(meta.emulator, env!("CARGO_PKG_VERSION"));
        assert!(meta.size > 76_800);
        let set = SlotSet::scan(&rom, None);
        assert_eq!(set.used(), 1);
        assert_eq!(set.get(2).unwrap().frame, frame);
        assert!(set.get(1).is_none());
        assert_eq!(set.latest().unwrap().slot, 2);
        assert_eq!(set.total_bytes(), meta.size);
        // run on, then load back
        for _ in 0..5 {
            gba.run_frame();
        }
        assert_ne!(gba.bus.ppu.frame_count, frame);
        let m2 = load(&mut gba, &rom, 2).unwrap();
        assert_eq!(gba.bus.ppu.frame_count, frame);
        assert_eq!(m2.frame, frame);
        assert!(matches!(load(&mut gba, &rom, 3), Err(SlotError::Missing)));
        delete(&rom, 2).unwrap();
        assert!(matches!(read_meta(&rom, 2), Err(SlotError::Missing)));
        delete(&rom, 2).unwrap();
    }

    #[test]
    fn corrupt_and_mismatched_files_are_reported_not_loaded() {
        let rom = temp_rom("bad");
        std::fs::write(slot_path(&rom, 1), b"nonsense").unwrap();
        assert!(matches!(read_meta(&rom, 1), Err(SlotError::Corrupt(_))));
        let set = SlotSet::scan(&rom, None);
        let m = set.get(1).unwrap();
        assert!(m.broken.is_some());
        assert!(set.latest().is_none());
        let infos = set.infos();
        assert!(infos[0].as_ref().unwrap().broken.is_some());
        // a future core version
        let fb = vec![0u16; FRAME_LEN];
        let mut bytes = encode(10, 0, 1, &fb, b"body");
        bytes[32..36].copy_from_slice(&(gba_core::state::STATE_VERSION + 1).to_le_bytes());
        std::fs::write(slot_path(&rom, 3), &bytes).unwrap();
        let m = read_meta(&rom, 3).unwrap();
        assert!(m.broken.unwrap().contains("state format"));
        let mut gba = Gba::new(test_rom(), Options::default()).unwrap();
        assert!(matches!(load(&mut gba, &rom, 3), Err(SlotError::CantLoad(_))));
    }

    #[test]
    fn legacy_state_is_imported_into_slot_one_once() {
        let rom = temp_rom("legacy");
        let bytes = test_rom();
        let mut gba = Gba::new(bytes.clone(), Options::default()).unwrap();
        for _ in 0..4 {
            gba.run_frame();
        }
        let legacy = legacy_path(&rom);
        std::fs::write(&legacy, gba.save_state()).unwrap();
        let set = SlotSet::scan(&rom, Some(&bytes));
        let m = set.get(1).expect("imported");
        assert_eq!(m.frame, gba.bus.ppu.frame_count + 1, "one frame run for the thumbnail");
        assert!(legacy.is_file(), "original untouched");
        // second scan does not overwrite slot 1
        let before = std::fs::metadata(slot_path(&rom, 1)).unwrap().len();
        save(&gba, &rom, 1, 7).unwrap();
        let set2 = SlotSet::scan(&rom, Some(&bytes));
        assert_eq!(set2.get(1).unwrap().play_secs, 7);
        assert_ne!(std::fs::metadata(slot_path(&rom, 1)).unwrap().len(), before + 1);
    }

    #[test]
    fn header_rejects_truncation() {
        let fb = vec![0u16; FRAME_LEN];
        let bytes = encode(1, 2, 3, &fb, b"xyz");
        let p = parse_header(&bytes).unwrap();
        assert_eq!((p.frame, p.saved_at, p.play_secs), (1, 2, 3));
        assert_eq!(&bytes[p.body_at..], b"xyz");
        assert!(matches!(parse_header(&bytes[..100]), Err(SlotError::Corrupt(_))));
    }
}
