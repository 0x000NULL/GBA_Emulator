//! The folder scan: a background thread that turns ROM files (the [`Platform`]'s extensions) into
//! `LibraryEntry`s.
//!
//! Cheapest first: every file whose path + size + mtime match an index row is reported straight
//! from the index (no ROM bytes read); only unknown or changed files are opened. Folders are
//! scanned non-recursively.

use super::index::IndexEntry;
use super::{names, LibraryEntry, Platform, RomInfo};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver};
use std::sync::Arc;
use std::time::UNIX_EPOCH;

/// What the scan thread sends back.
#[derive(Debug)]
pub enum ScanMsg {
    Entry(LibraryEntry),
    /// The scan finished; `seen` lists every ROM file found (so vanished ones can be dropped).
    Done { seen: Vec<PathBuf> },
}

/// True for the file types the library lists (`extensions`: lowercase, no dot).
pub fn is_rom_file(path: &Path, extensions: &[&str]) -> bool {
    path.extension().and_then(|e| e.to_str()).is_some_and(|e| extensions.iter().any(|x| e.eq_ignore_ascii_case(x)))
}

/// Start a scan thread over `folders`, seeded with the index rows as the cache.
pub fn spawn(folders: Vec<PathBuf>, cached: Vec<IndexEntry>, platform: Arc<dyn Platform>) -> Receiver<ScanMsg> {
    let (tx, rx) = channel();
    let spawned = std::thread::Builder::new().name("library-scan".into()).spawn(move || {
        let tx2 = tx.clone();
        let seen = scan(&folders, &cached, &*platform, |e| tx2.send(ScanMsg::Entry(e)).is_ok());
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

fn list_candidates(folders: &[PathBuf], extensions: &[&str]) -> Vec<Candidate> {
    let mut out = Vec::new();
    for dir in folders {
        let Ok(rd) = std::fs::read_dir(dir) else { continue };
        let mut files: Vec<Candidate> = rd
            .flatten()
            .filter_map(|de| {
                let path = de.path();
                if !is_rom_file(&path, extensions) {
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
pub fn scan(folders: &[PathBuf], cached: &[IndexEntry], platform: &dyn Platform, mut sink: impl FnMut(LibraryEntry) -> bool) -> Vec<PathBuf> {
    let candidates = list_candidates(folders, platform.extensions());
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
        if !sink(read_entry(platform, &c.path, c.size, c.mtime)) {
            break;
        }
    }
    seen
}

/// Open one ROM file and describe it; failures become an `unreadable` entry.
fn read_entry(platform: &dyn Platform, path: &Path, size: u64, mtime: i64) -> LibraryEntry {
    let file_name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    let from_name = names::from_file_name(&file_name);
    // a malformed file must never take the scan thread down
    let info = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| platform.probe(path)))
        .unwrap_or_else(|_| RomInfo { unreadable: Some("the file could not be decoded".into()), ..RomInfo::default() });
    let mut entry = LibraryEntry {
        path: path.to_path_buf(),
        size,
        mtime,
        title: file_name.clone(),
        subtitle: String::new(),
        code: String::new(),
        version: 0,
        save: "none".into(),
        rom_len: info.rom_len,
        unreadable: None,
    };
    if let Some(why) = info.unreadable {
        entry.unreadable = Some(why);
        return entry;
    }
    entry.code = info.code;
    entry.version = info.version;
    entry.save = info.save;
    match from_name {
        Some(d) => (entry.title, entry.subtitle) = (d.title, d.subtitle),
        None => {
            let d = names::from_header_title(&info.header_title);
            if !d.title.is_empty() {
                entry.title = d.title;
            }
        }
    }
    entry
}

/// Test and render helper: scan synchronously and collect.
pub fn scan_collect(folders: &[PathBuf], cached: &[IndexEntry], platform: &dyn Platform) -> Vec<LibraryEntry> {
    let mut out = Vec::new();
    scan(folders, cached, platform, |e| {
        out.push(e);
        true
    });
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::library::fake::FakePlatform;

    fn temp_dir(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("gba-scan-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn scans_a_folder_with_good_bad_and_foreign_files() {
        let dir = temp_dir("mix");
        std::fs::write(dir.join("Kingdom Hearts - Chain of Memories (USA).rom"), "B8CEKINGDOM HEART").unwrap();
        std::fs::write(dir.join("Tiny.rom"), [0u8; 2]).unwrap();
        std::fs::write(dir.join("notes.txt"), b"ignored").unwrap();
        std::fs::create_dir_all(dir.join("sub.rom")).unwrap(); // a directory named like a ROM
        let got = scan_collect(std::slice::from_ref(&dir), &[], &FakePlatform);
        assert_eq!(got.len(), 2, "{got:?}");
        let kh = got.iter().find(|e| e.code == "B8CE").expect("the good ROM");
        assert_eq!(kh.title, "Kingdom Hearts");
        assert_eq!(kh.subtitle, "Chain of Memories · USA");
        assert_eq!((kh.rom_len, kh.version, kh.save.as_str()), (17, 0, "battery"));
        assert!(kh.unreadable.is_none());
        let tiny = got.iter().find(|e| e.title == "Tiny.rom").expect("too small is listed, unreadable");
        assert!(tiny.unreadable.as_deref().unwrap().contains("too small"));
        assert_eq!((tiny.rom_len, tiny.save.as_str(), tiny.subtitle.as_str()), (2, "none", ""));
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A probe that panics lists the file as unreadable instead of ending the scan.
    struct Panicky;
    impl Platform for Panicky {
        fn extensions(&self) -> &'static [&'static str] {
            &["rom"]
        }
        fn probe(&self, _: &Path) -> RomInfo {
            panic!("bad rom")
        }
        fn slot_art(&self, _: &Path) -> Option<crate::library::Art> {
            None
        }
        fn capture_title(&self, _: &Path, _: &Path) -> anyhow::Result<crate::library::Art> {
            anyhow::bail!("no")
        }
    }

    #[test]
    fn a_panicking_probe_is_an_unreadable_entry() {
        let dir = temp_dir("panic");
        std::fs::write(dir.join("A (USA).rom"), "AAAA").unwrap();
        let got = scan_collect(std::slice::from_ref(&dir), &[], &Panicky);
        assert_eq!(got.len(), 1);
        assert_eq!((got[0].title.as_str(), got[0].unreadable.as_deref()), ("A (USA).rom", Some("the file could not be decoded")));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn index_hits_skip_the_rom_and_changes_are_reread() {
        let dir = temp_dir("cache");
        let path = dir.join("Mario Kart - Super Circuit (USA).rom");
        std::fs::write(&path, "AMKEMARIOKART").unwrap();
        let first = scan_collect(std::slice::from_ref(&dir), &[], &FakePlatform);
        assert_eq!(first[0].code, "AMKE");
        // a cache row with a deliberately different code proves the ROM was not opened
        let mut row = IndexEntry::from_entry(&first[0]);
        row.code = "CACH".into();
        let hit = scan_collect(std::slice::from_ref(&dir), &[row.clone()], &FakePlatform);
        assert_eq!(hit[0].code, "CACH");
        // a changed size invalidates the row
        let mut stale = row;
        stale.size += 1;
        let miss = scan_collect(std::slice::from_ref(&dir), &[stale], &FakePlatform);
        assert_eq!(miss[0].code, "AMKE");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_thread_reports_entries_then_done() {
        let dir = temp_dir("thread");
        std::fs::write(dir.join("A (USA).rom"), "AAAAA").unwrap();
        std::fs::write(dir.join("B (USA).rom"), "BBBBB").unwrap();
        let rx = spawn(vec![dir.clone(), dir.join("missing")], Vec::new(), crate::library::fake::platform());
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
        let exts = ["gba", "zip"];
        assert!(is_rom_file(Path::new("a.GBA"), &exts));
        assert!(is_rom_file(Path::new("a.zip"), &exts));
        assert!(!is_rom_file(Path::new("a.sav"), &exts));
        assert!(!is_rom_file(Path::new("a"), &exts));
    }
}
