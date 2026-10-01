//! The folder scan: a background thread that turns `.gba` / `.zip` files into `LibraryEntry`s.
//!
//! Cheapest first: every file whose path + size + mtime match an index row is reported straight
//! from the index (no ROM bytes read); only unknown or changed files are opened. Folders are
//! scanned non-recursively.

use super::index::IndexEntry;
use super::{names, LibraryEntry};
use gba_core::cart::{Cartridge, Header};
use gba_core::SaveType;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver};
use std::time::UNIX_EPOCH;

/// What the scan thread sends back.
#[derive(Debug)]
pub enum ScanMsg {
    Entry(LibraryEntry),
    /// The scan finished; `seen` lists every ROM file found (so vanished ones can be dropped).
    Done { seen: Vec<PathBuf> },
}

/// True for the file types the library lists.
pub fn is_rom_file(path: &Path) -> bool {
    path.extension().and_then(|e| e.to_str()).is_some_and(|e| e.eq_ignore_ascii_case("gba") || e.eq_ignore_ascii_case("zip"))
}

/// Start a scan thread over `folders`, seeded with the index rows as the cache.
pub fn spawn(folders: Vec<PathBuf>, cached: Vec<IndexEntry>) -> Receiver<ScanMsg> {
    let (tx, rx) = channel();
    let spawned = std::thread::Builder::new().name("library-scan".into()).spawn(move || {
        let tx2 = tx.clone();
        let seen = scan(&folders, &cached, |e| tx2.send(ScanMsg::Entry(e)).is_ok());
        let _ = tx.send(ScanMsg::Done { seen });
    });
    if let Err(e) = spawned {
        eprintln!("warning: could not start the library scan: {e}");
    }
    rx
}

struct Candidate {
    path: PathBuf,
    size: u64,
    mtime: i64,
}

fn list_candidates(folders: &[PathBuf]) -> Vec<Candidate> {
    let mut out = Vec::new();
    for dir in folders {
        let Ok(rd) = std::fs::read_dir(dir) else { continue };
        let mut files: Vec<Candidate> = rd
            .flatten()
            .filter_map(|de| {
                let path = de.path();
                if !is_rom_file(&path) {
                    return None;
                }
                let md = de.metadata().ok().filter(|m| m.is_file())?;
                let mtime = md.modified().ok().and_then(|t| t.duration_since(UNIX_EPOCH).ok()).map_or(0, |d| d.as_secs() as i64);
                Some(Candidate { path, size: md.len(), mtime })
            })
            .collect();
        files.sort_by(|a, b| a.path.cmp(&b.path));
        out.extend(files);
    }
    out
}

/// Run the scan on this thread. `sink` receives entries as they are found and returns false to
/// stop early (the receiver went away). Returns the paths seen.
pub fn scan(folders: &[PathBuf], cached: &[IndexEntry], mut sink: impl FnMut(LibraryEntry) -> bool) -> Vec<PathBuf> {
    let candidates = list_candidates(folders);
    let by_path: HashMap<&Path, &IndexEntry> = cached.iter().map(|e| (Path::new(e.path.as_str()), e)).collect();
    let seen = candidates.iter().map(|c| c.path.clone()).collect();

    // pass 1: everything the index already knows
    let mut unknown = Vec::new();
    for c in candidates {
        match by_path.get(c.path.as_path()) {
            Some(ie) if ie.matches(&c.path, c.size, c.mtime) => {
                if !sink(ie.to_entry()) {
                    return seen;
                }
            }
            _ => unknown.push(c),
        }
    }
    // pass 2: read the rest
    for c in unknown {
        if !sink(read_entry(&c.path, c.size, c.mtime)) {
            break;
        }
    }
    seen
}

/// Open one ROM file and describe it; failures become an `unreadable` entry.
fn read_entry(path: &Path, size: u64, mtime: i64) -> LibraryEntry {
    let file_name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    let from_name = names::from_file_name(&file_name);
    let mut entry = LibraryEntry {
        path: path.to_path_buf(),
        size,
        mtime,
        title: from_name.as_ref().map_or_else(|| file_name.clone(), |d| d.title.clone()),
        subtitle: from_name.as_ref().map(|d| d.subtitle.clone()).unwrap_or_default(),
        code: String::new(),
        version: 0,
        save: SaveType::None,
        rom_len: 0,
        unreadable: None,
    };
    // a malformed archive must never take the scan thread down
    let loaded = std::panic::catch_unwind(|| gba_test::load_rom(path));
    let rom = match loaded {
        Ok(Ok(rom)) => rom,
        Ok(Err(e)) => {
            entry.unreadable = Some(format!("{e:#}"));
            entry.title = file_name;
            entry.subtitle.clear();
            return entry;
        }
        Err(_) => {
            entry.unreadable = Some("the file could not be decoded".into());
            entry.title = file_name;
            entry.subtitle.clear();
            return entry;
        }
    };
    entry.rom_len = rom.len() as u64;
    // Header::parse indexes up to 0xC0 and panics on anything shorter
    if rom.len() < 0xC0 {
        entry.unreadable = Some(format!("only {} bytes: too small to be a GBA ROM", rom.len()));
        entry.title = file_name;
        entry.subtitle.clear();
        return entry;
    }
    let header = Header::parse(&rom);
    entry.code = header.game_code_str();
    entry.version = header.version;
    entry.save = Cartridge::detect_save_type(&rom, &header);
    if from_name.is_none() {
        let d = names::from_header_title(&header.title);
        entry.title = if d.title.is_empty() { file_name } else { d.title };
    }
    entry
}

/// Test and render helper: scan synchronously and collect.
pub fn scan_collect(folders: &[PathBuf], cached: &[IndexEntry]) -> Vec<LibraryEntry> {
    let mut out = Vec::new();
    scan(folders, cached, |e| {
        out.push(e);
        true
    });
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A header-only test ROM like the one in `slots.rs`.
    pub fn test_rom(title: &str, code: &str) -> Vec<u8> {
        let mut rom = vec![0u8; 0x1000];
        rom[0..4].copy_from_slice(&0xEAFF_FFFEu32.to_le_bytes());
        let t = title.as_bytes();
        rom[0xA0..0xA0 + t.len().min(12)].copy_from_slice(&t[..t.len().min(12)]);
        rom[0xAC..0xB0].copy_from_slice(code.as_bytes());
        rom[0xB2] = 0x96;
        rom[0xBC] = 1;
        rom[0x200..0x208].copy_from_slice(b"SRAM_V11");
        rom
    }

    fn temp_dir(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("gba-scan-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn scans_a_folder_with_good_bad_and_foreign_files() {
        let dir = temp_dir("mix");
        std::fs::write(dir.join("Kingdom Hearts - Chain of Memories (USA).gba"), test_rom("KINGDOM HEART", "B8CE")).unwrap();
        std::fs::write(dir.join("Tiny.gba"), [0u8; 16]).unwrap();
        std::fs::write(dir.join("Broken (USA).zip"), b"PK\x03\x04 not really a zip").unwrap();
        std::fs::write(dir.join("notes.txt"), b"ignored").unwrap();
        std::fs::create_dir_all(dir.join("sub.gba")).unwrap(); // a directory named like a ROM
        let got = scan_collect(std::slice::from_ref(&dir), &[]);
        assert_eq!(got.len(), 3, "{got:?}");
        let kh = got.iter().find(|e| e.code == "B8CE").expect("the good ROM");
        assert_eq!(kh.title, "Kingdom Hearts");
        assert_eq!(kh.subtitle, "Chain of Memories · USA");
        assert_eq!((kh.rom_len, kh.version, kh.save), (0x1000, 1, SaveType::Sram32K));
        assert!(kh.unreadable.is_none());
        let tiny = got.iter().find(|e| e.title == "Tiny.gba").expect("too small is listed, unreadable");
        assert!(tiny.unreadable.as_deref().unwrap().contains("too small"));
        assert_eq!(tiny.rom_len, 16);
        let bad = got.iter().find(|e| e.title == "Broken (USA).zip").expect("bad zip is listed, unreadable");
        assert!(bad.unreadable.is_some());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn index_hits_skip_the_rom_and_changes_are_reread() {
        let dir = temp_dir("cache");
        let path = dir.join("Mario Kart - Super Circuit (USA).gba");
        std::fs::write(&path, test_rom("MARIOKART", "AMKE")).unwrap();
        let first = scan_collect(std::slice::from_ref(&dir), &[]);
        assert_eq!(first[0].code, "AMKE");
        // a cache row with a deliberately different code proves the ROM was not opened
        let mut row = IndexEntry::from_entry(&first[0]);
        row.code = "CACH".into();
        let hit = scan_collect(std::slice::from_ref(&dir), &[row.clone()]);
        assert_eq!(hit[0].code, "CACH");
        // a changed size invalidates the row
        let mut stale = row;
        stale.size += 1;
        let miss = scan_collect(std::slice::from_ref(&dir), &[stale]);
        assert_eq!(miss[0].code, "AMKE");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_thread_reports_entries_then_done() {
        let dir = temp_dir("thread");
        std::fs::write(dir.join("A (USA).gba"), test_rom("A", "AAAA")).unwrap();
        std::fs::write(dir.join("B (USA).gba"), test_rom("B", "BBBB")).unwrap();
        let rx = spawn(vec![dir.clone(), dir.join("missing")], Vec::new());
        let mut entries = 0;
        let seen = loop {
            match rx.recv_timeout(std::time::Duration::from_secs(10)).expect("scan finishes") {
                ScanMsg::Entry(_) => entries += 1,
                ScanMsg::Done { seen } => break seen,
            }
        };
        assert_eq!((entries, seen.len()), (2, 2));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn rom_file_extensions() {
        assert!(is_rom_file(Path::new("a.GBA")));
        assert!(is_rom_file(Path::new("a.zip")));
        assert!(!is_rom_file(Path::new("a.sav")));
        assert!(!is_rom_file(Path::new("a")));
    }
}
