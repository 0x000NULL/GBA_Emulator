//! Save-state slots 1..=8 per game: `<rom stem>.s<N>.state` beside the ROM.
//!
//! File format (little-endian; documented in `docs/ARCHITECTURE.md`):
//! ```text
//! magic             4 bytes, the app's SlotFormat::magic ("GBSL" for Strawberry)
//! u32  header_ver   1
//! u32  header_len   bytes from the start of the file to the core state
//! u64  frame        the core's frame counter at save time
//! i64  saved_at     unix seconds
//! u32  play_secs    play time at save time
//! u32  core_ver     SlotFormat::core_version, the state format the body was written with
//! u16  name_len + name bytes (SlotFormat::emulator, the app's version string)
//! u16[w*h]          the picture at save time, in the app's pixel format (bit 15 cleared)
//! ...               the core's state bytes
//! ```
//! The picture's size is the format's, not the file's: a file whose header length disagrees is
//! corrupt. One file keeps writes atomic (`saves::write_atomic`) and deletes simple; `read_meta`
//! reads the header and picture without touching the body.

use crate::saves::write_atomic;
use anyhow::{anyhow, Result};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

pub const SLOTS: usize = 8;
const HEADER_VER: u32 = 1;

/// What the app fixes about its slot files.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SlotFormat {
    pub magic: [u8; 4],
    /// The core's state format version; a slot written with another one is listed but not loaded.
    pub core_version: u32,
    /// The app's version string, written into every file.
    pub emulator: &'static str,
    /// Picture size in dots.
    pub frame_w: usize,
    pub frame_h: usize,
}

/// A machine that can be saved into a slot and loaded from one.
pub trait Snapshot {
    fn save_state(&self) -> Vec<u8>;
    fn load_state(&mut self, body: &[u8]) -> Result<(), String>;
    /// The frame counter stored in the header.
    fn frame_count(&self) -> u64;
    /// The current picture, `frame_w * frame_h` texels.
    fn picture(&self) -> &[u16];
}

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
    /// The picture at save time, `frame_w * frame_h` texels (zeros for an unreadable file).
    pub frame_full: Vec<u16>,
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
    let stem = rom.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_else(|| "game".into());
    rom.with_file_name(format!("{stem}.s{slot}.state"))
}

pub fn now_secs() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0)
}

struct Parsed {
    frame: u64,
    saved_at: i64,
    play_secs: u32,
    core_ver: u32,
    emulator: String,
    fb: Vec<u16>,
    body_at: usize,
}

fn read_file(path: &Path) -> Result<Vec<u8>, SlotError> {
    match std::fs::read(path) {
        Ok(b) => Ok(b),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Err(SlotError::Missing),
        Err(e) => Err(e.into()),
    }
}

impl SlotFormat {
    fn frame_len(&self) -> usize {
        self.frame_w * self.frame_h
    }

    /// Serialise a slot file.
    pub fn encode(&self, frame: u64, saved_at: i64, play_secs: u32, fb: &[u16], body: &[u8]) -> Vec<u8> {
        let name = self.emulator.as_bytes();
        let header_len = 4 + 4 + 4 + 8 + 8 + 4 + 4 + 2 + name.len() + self.frame_len() * 2;
        let mut out = Vec::with_capacity(header_len + body.len());
        out.extend_from_slice(&self.magic);
        out.extend_from_slice(&HEADER_VER.to_le_bytes());
        out.extend_from_slice(&(header_len as u32).to_le_bytes());
        out.extend_from_slice(&frame.to_le_bytes());
        out.extend_from_slice(&saved_at.to_le_bytes());
        out.extend_from_slice(&play_secs.to_le_bytes());
        out.extend_from_slice(&self.core_version.to_le_bytes());
        out.extend_from_slice(&(name.len() as u16).to_le_bytes());
        out.extend_from_slice(name);
        for p in fb.iter().take(self.frame_len()) {
            out.extend_from_slice(&(p & 0x7FFF).to_le_bytes());
        }
        out.extend_from_slice(body);
        out
    }

    fn parse_header(&self, bytes: &[u8]) -> Result<Parsed, SlotError> {
        let need = |n: usize| -> Result<(), SlotError> {
            if bytes.len() < n {
                Err(SlotError::Corrupt(format!("file is {} bytes, header needs {n}", bytes.len())))
            } else {
                Ok(())
            }
        };
        need(12)?;
        if bytes[0..4] != self.magic {
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
        let fb_end = name_end + self.frame_len() * 2;
        if fb_end != header_len {
            return Err(SlotError::Corrupt("header length mismatch".into()));
        }
        let emulator = String::from_utf8_lossy(&bytes[38..name_end]).into_owned();
        let fb = bytes[name_end..fb_end].chunks_exact(2).map(|b| u16::from_le_bytes([b[0], b[1]])).collect();
        Ok(Parsed { frame, saved_at, play_secs, core_ver, emulator, fb, body_at: header_len })
    }

    /// Read a slot's header and picture (not its body).
    pub fn read_meta(&self, rom: &Path, slot: u8) -> Result<SlotMeta, SlotError> {
        let path = slot_path(rom, slot);
        let bytes = read_file(&path)?;
        let size = bytes.len() as u64;
        let p = self.parse_header(&bytes)?;
        let broken = if p.core_ver != self.core_version {
            Some(format!("saved with state format {}, this build reads {}", p.core_ver, self.core_version))
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
            frame_full: p.fb,
            broken,
        })
    }

    /// Write the running machine into `slot`.
    pub fn save(&self, machine: &impl Snapshot, rom: &Path, slot: u8, play_secs: u64) -> Result<SlotMeta> {
        let path = slot_path(rom, slot);
        let body = machine.save_state();
        let bytes = self.encode(machine.frame_count(), now_secs(), play_secs.min(u32::MAX as u64) as u32, machine.picture(), &body);
        write_atomic(&path, &bytes)?;
        self.read_meta(rom, slot).map_err(|e| anyhow!("{e}"))
    }

    /// Load `slot` into the running machine.
    pub fn load(&self, machine: &mut impl Snapshot, rom: &Path, slot: u8) -> Result<SlotMeta, SlotError> {
        let bytes = read_file(&slot_path(rom, slot))?;
        let p = self.parse_header(&bytes)?;
        if bytes.len() <= p.body_at {
            return Err(SlotError::CantLoad("no state body".into()));
        }
        machine.load_state(&bytes[p.body_at..]).map_err(SlotError::CantLoad)?;
        self.read_meta(rom, slot)
    }

    /// Read all eight slots of a game.
    pub fn scan(&self, rom: &Path) -> SlotSet {
        let mut slots = Vec::with_capacity(SLOTS);
        for n in 1..=SLOTS as u8 {
            slots.push(match self.read_meta(rom, n) {
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
                        frame_full: vec![0u16; self.frame_len()],
                        broken: Some(e.to_string()),
                    })
                }
            });
        }
        SlotSet { slots }
    }
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
    #[cfg_attr(not(test), allow(dead_code))]
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
}

#[cfg(test)]
mod tests {
    use super::*;

    const FMT: SlotFormat = SlotFormat { magic: *b"TSSL", core_version: 7, emulator: "test 0.1", frame_w: 4, frame_h: 2 };

    /// A machine whose state is a counter.
    struct Counter {
        n: u64,
        pic: Vec<u16>,
    }

    impl Snapshot for Counter {
        fn save_state(&self) -> Vec<u8> {
            self.n.to_le_bytes().to_vec()
        }
        fn load_state(&mut self, body: &[u8]) -> Result<(), String> {
            self.n = u64::from_le_bytes(body.try_into().map_err(|_| "bad body".to_string())?);
            Ok(())
        }
        fn frame_count(&self) -> u64 {
            self.n
        }
        fn picture(&self) -> &[u16] {
            &self.pic
        }
    }

    fn temp_rom(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("gba-slots-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir.join("game.rom")
    }

    #[test]
    fn paths_beside_the_rom() {
        let p = slot_path(Path::new("C:/Games/Kingdom Hearts (USA).zip"), 4);
        assert_eq!(p, PathBuf::from("C:/Games/Kingdom Hearts (USA).s4.state"));
    }

    #[test]
    fn round_trip_save_load_delete() {
        let rom = temp_rom("rt");
        let mut m = Counter { n: 3, pic: vec![1, 2, 3, 4, 5, 6, 7, 0x8008] };
        let meta = FMT.save(&m, &rom, 2, 458).unwrap();
        assert_eq!((meta.slot, meta.frame, meta.play_secs, meta.core_ver), (2, 3, 458, 7));
        assert_eq!(meta.emulator, "test 0.1");
        assert_eq!(meta.frame_full, vec![1, 2, 3, 4, 5, 6, 7, 8], "bit 15 is not stored");
        assert!(meta.broken.is_none());
        let set = FMT.scan(&rom);
        assert_eq!(set.used(), 1);
        assert_eq!(set.get(2).unwrap().frame, 3);
        assert!(set.get(1).is_none());
        assert_eq!(set.latest().unwrap().slot, 2);
        assert_eq!(set.total_bytes(), meta.size);
        m.n = 9;
        let m2 = FMT.load(&mut m, &rom, 2).unwrap();
        assert_eq!((m.n, m2.frame), (3, 3));
        assert!(matches!(FMT.load(&mut m, &rom, 3), Err(SlotError::Missing)));
        delete(&rom, 2).unwrap();
        assert!(matches!(FMT.read_meta(&rom, 2), Err(SlotError::Missing)));
        delete(&rom, 2).unwrap();
    }

    #[test]
    fn corrupt_and_mismatched_files_are_reported_not_loaded() {
        let rom = temp_rom("bad");
        std::fs::write(slot_path(&rom, 1), b"nonsense").unwrap();
        assert!(matches!(FMT.read_meta(&rom, 1), Err(SlotError::Corrupt(_))));
        let set = FMT.scan(&rom);
        let m = set.get(1).unwrap();
        assert!(m.broken.is_some());
        assert_eq!(m.frame_full.len(), 8);
        assert!(set.latest().is_none());
        // a future core version
        let mut bytes = FMT.encode(10, 0, 1, &[0; 8], &[0; 8]);
        bytes[32..36].copy_from_slice(&8u32.to_le_bytes());
        std::fs::write(slot_path(&rom, 3), &bytes).unwrap();
        assert!(FMT.read_meta(&rom, 3).unwrap().broken.unwrap().contains("state format 8, this build reads 7"));
        // another app's file, and a body the machine rejects
        let other = SlotFormat { magic: *b"XXSL", ..FMT };
        std::fs::write(slot_path(&rom, 4), other.encode(1, 0, 1, &[0; 8], &[0; 8])).unwrap();
        assert!(matches!(FMT.read_meta(&rom, 4), Err(SlotError::Corrupt(_))));
        std::fs::write(slot_path(&rom, 5), FMT.encode(1, 0, 1, &[0; 8], b"short")).unwrap();
        let mut m = Counter { n: 0, pic: vec![0; 8] };
        assert!(matches!(FMT.load(&mut m, &rom, 5), Err(SlotError::CantLoad(_))));
        // a different picture size is a different format
        let big = SlotFormat { frame_w: 8, ..FMT };
        std::fs::write(slot_path(&rom, 6), big.encode(1, 0, 1, &[0; 16], &[0; 8])).unwrap();
        assert!(matches!(FMT.read_meta(&rom, 6), Err(SlotError::Corrupt(_))));
    }

    #[test]
    fn header_rejects_truncation() {
        let bytes = FMT.encode(1, 2, 3, &[0; 8], b"xyz");
        let p = FMT.parse_header(&bytes).unwrap();
        assert_eq!((p.frame, p.saved_at, p.play_secs), (1, 2, 3));
        assert_eq!(&bytes[p.body_at..], b"xyz");
        assert!(matches!(FMT.parse_header(&bytes[..40]), Err(SlotError::Corrupt(_))));
    }
}
