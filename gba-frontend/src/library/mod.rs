//! The game library: which ROMs exist, what they are called, how they look, how long they were
//! played. Backs screen 01 (`screens::library`).
//!
//! * `scan` walks the configured folders on a background thread, cheapest first (the index cache
//!   answers for unchanged files; only new or changed ROMs are read).
//! * `index` is the TOML cache (`<cache_dir>/index.toml`): scan results plus play statistics.
//!   A warm start shows the whole case from it before the scan has confirmed anything.
//! * `capture` finds label art: newest save-state frame, else a cached title capture, else none;
//!   missing captures are made by one background thread running the ROM headless.
//! * `names` turns file names into titles and subtitles.
//!
//! The owner calls [`Library::poll`] once per frame and [`Library::record_played`] when a game
//! ends.
//!
//! This module is wired in by the integrator; until then most of it is unused.
#![allow(dead_code)]

pub mod capture;
pub mod index;
pub mod names;
pub mod scan;

pub use capture::Art;

use gba_core::SaveType;
use index::Index;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, Sender};
use std::time::{SystemTime, UNIX_EPOCH};

/// One ROM file as the library knows it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LibraryEntry {
    pub path: PathBuf,
    /// File size in bytes (index key).
    pub size: u64,
    /// File modification time, unix seconds (index key).
    pub mtime: i64,
    pub title: String,
    pub subtitle: String,
    /// Four-character game code from the header, e.g. `B8CE`; empty when unreadable.
    pub code: String,
    pub version: u8,
    pub save: SaveType,
    /// Length of the ROM image itself (after unzipping).
    pub rom_len: u64,
    /// Why the ROM could not be read, if it could not.
    pub unreadable: Option<String>,
}

impl LibraryEntry {
    /// The file name, for unreadable carts and the details list.
    pub fn file_name(&self) -> String {
        self.path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default()
    }

    fn sort_key(&self) -> (String, String, PathBuf) {
        (self.title.to_lowercase(), self.subtitle.to_lowercase(), self.path.clone())
    }
}

pub struct Library {
    folders: Vec<PathBuf>,
    cache_dir: Option<PathBuf>,
    entries: Vec<LibraryEntry>,
    index: Index,
    scan_rx: Option<Receiver<scan::ScanMsg>>,
    scanning: bool,
    art_tx: Option<Sender<capture::ArtJob>>,
    art_rx: Option<Receiver<capture::ArtMsg>>,
    arts: HashMap<PathBuf, Art>,
    /// Games already handed to the art thread.
    queued: HashSet<PathBuf>,
    /// Bumped whenever entries, art or statistics change (screens can cache on it).
    revision: u64,
    /// Entries seen by the running scan, to drop stale warm-start rows when it finishes.
    seen_this_scan: HashSet<PathBuf>,
    /// Cached `index.last_played(entries)`; screens ask for it every frame.
    last_played: Option<PathBuf>,
}

fn now_secs() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0)
}

impl Library {
    /// Start a library over `folders`. `cache_dir` None = no index file and no title captures
    /// (tests). The scan and the art thread start immediately.
    pub fn new(folders: Vec<PathBuf>, cache_dir: Option<PathBuf>) -> Library {
        let mut lib = Library::bare(folders, cache_dir);
        lib.start_art_thread();
        lib.rescan();
        lib
    }

    /// A library with fixed entries and no threads (screen tests).
    pub fn from_entries(folders: Vec<PathBuf>, entries: Vec<LibraryEntry>) -> Library {
        let mut lib = Library::bare(folders, None);
        for e in &entries {
            lib.index.upsert(e);
        }
        lib.entries = entries;
        lib.sort();
        lib.refresh_last_played();
        lib
    }

    fn bare(folders: Vec<PathBuf>, cache_dir: Option<PathBuf>) -> Library {
        let index = cache_dir.as_deref().map(Index::load).unwrap_or_default();
        let mut lib = Library {
            folders,
            cache_dir,
            entries: Vec::new(),
            index,
            scan_rx: None,
            scanning: false,
            art_tx: None,
            art_rx: None,
            arts: HashMap::new(),
            queued: HashSet::new(),
            revision: 0,
            seen_this_scan: HashSet::new(),
            last_played: None,
        };
        // warm start: whatever the index remembers about files in these folders
        let known: Vec<LibraryEntry> =
            lib.index.entries.iter().map(|ie| ie.to_entry()).filter(|e| e.path.parent().is_some_and(|p| lib.in_folders(p))).collect();
        lib.entries = known;
        lib.sort();
        lib.refresh_last_played();
        lib
    }

    fn refresh_last_played(&mut self) {
        self.last_played = self.index.last_played(self.entries.iter().map(|e| e.path.as_path()));
    }

    fn in_folders(&self, dir: &Path) -> bool {
        self.folders.iter().any(|f| f.as_path() == dir)
    }

    fn start_art_thread(&mut self) {
        let captures = self.cache_dir.as_deref().map(capture::captures_dir);
        let (tx, rx) = capture::spawn(captures);
        self.art_tx = Some(tx);
        self.art_rx = Some(rx);
        self.queue_art_jobs();
    }

    fn queue_art_jobs(&mut self) {
        let Some(tx) = &self.art_tx else { return };
        for e in &self.entries {
            if e.unreadable.is_some() || self.queued.contains(&e.path) {
                continue;
            }
            self.queued.insert(e.path.clone());
            let _ = tx.send(capture::ArtJob::Resolve { path: e.path.clone(), code: e.code.clone() });
        }
    }

    /// Start (or restart) the folder scan.
    pub fn rescan(&mut self) {
        self.seen_this_scan.clear();
        self.scanning = true;
        self.scan_rx = Some(scan::spawn(self.folders.clone(), self.index.entries.clone()));
    }

    /// Ingest scan and art results. Call once per frame; never blocks.
    pub fn poll(&mut self) {
        let mut changed = false;
        let mut done: Option<Vec<PathBuf>> = None;
        if let Some(rx) = &self.scan_rx {
            while let Ok(msg) = rx.try_recv() {
                match msg {
                    scan::ScanMsg::Entry(e) => {
                        self.seen_this_scan.insert(e.path.clone());
                        self.index.upsert(&e);
                        match self.entries.iter_mut().find(|x| x.path == e.path) {
                            Some(slot) => {
                                if *slot != e {
                                    *slot = e;
                                    changed = true;
                                }
                            }
                            None => {
                                self.entries.push(e);
                                changed = true;
                            }
                        }
                    }
                    scan::ScanMsg::Done { seen } => {
                        done = Some(seen);
                        break;
                    }
                }
            }
        }
        if let Some(seen) = done {
            self.scan_rx = None;
            self.scanning = false;
            let seen: HashSet<PathBuf> = seen.into_iter().collect();
            let before = self.entries.len();
            self.entries.retain(|e| seen.contains(&e.path));
            changed |= self.entries.len() != before;
            // rows for files that vanished from the scanned folders go too; stubs for games played
            // from elsewhere (not under a configured folder) are kept
            let folders = self.folders.clone();
            self.index.entries.retain(|ie| {
                let p = Path::new(&ie.path);
                seen.contains(p) || !p.parent().is_some_and(|d| folders.iter().any(|f| f.as_path() == d))
            });
            self.save_index();
        }
        if changed {
            self.refresh_last_played();
            self.sort();
            self.queue_art_jobs();
            self.revision += 1;
        }
        let mut arts = Vec::new();
        if let Some(rx) = &self.art_rx {
            while let Ok(m) = rx.try_recv() {
                arts.push(m);
            }
        }
        for m in arts {
            self.arts.insert(m.path, m.art);
            self.revision += 1;
        }
    }

    fn sort(&mut self) {
        self.entries.sort_by_key(|a| a.sort_key());
    }

    fn save_index(&self) {
        if let Some(dir) = &self.cache_dir {
            if let Err(e) = self.index.save(dir) {
                eprintln!("warning: library index: {e:#}");
            }
        }
    }

    pub fn entries(&self) -> &[LibraryEntry] {
        &self.entries
    }

    pub fn entry(&self, path: &Path) -> Option<&LibraryEntry> {
        self.entries.iter().find(|e| e.path == path)
    }

    pub fn folders(&self) -> &[PathBuf] {
        &self.folders
    }

    pub fn is_scanning(&self) -> bool {
        self.scanning
    }

    pub fn revision(&self) -> u64 {
        self.revision
    }

    /// Label art for a game, once the art thread has found or made it.
    pub fn art_for(&self, path: &Path) -> Option<&Art> {
        self.arts.get(path)
    }

    /// Total play time in seconds (0 = never played).
    pub fn played_secs(&self, path: &Path) -> u64 {
        self.index.played_secs(path)
    }

    /// The game most recently played that is still in the library.
    pub fn last_played(&self) -> Option<PathBuf> {
        self.last_played.clone()
    }

    /// Book `secs` of play time against `path` and stamp it as the last played game. Also asks
    /// the art thread to look at the game's save states again.
    pub fn record_played(&mut self, path: &Path, secs: u64) {
        self.index.record_played(path, secs, now_secs());
        self.refresh_last_played();
        self.save_index();
        if let Some(tx) = &self.art_tx {
            let _ = tx.send(capture::ArtJob::Refresh { path: path.to_path_buf() });
        }
        self.revision += 1;
    }

    /// Test hook: set a game's last-played stamp without play time.
    pub fn mark_played_at(&mut self, path: &Path, when: i64) {
        self.index.record_played(path, 0, when);
        self.refresh_last_played();
        self.revision += 1;
    }

    /// Test and render hook: attach art directly.
    pub fn set_art(&mut self, path: &Path, art: Art) {
        self.arts.insert(path.to_path_buf(), art);
        self.revision += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rom(title: &str, code: &str) -> Vec<u8> {
        let mut rom = vec![0u8; 0x1000];
        rom[0..4].copy_from_slice(&0xEAFF_FFFEu32.to_le_bytes());
        rom[0xA0..0xA0 + title.len()].copy_from_slice(title.as_bytes());
        rom[0xAC..0xB0].copy_from_slice(code.as_bytes());
        rom[0xB2] = 0x96;
        rom
    }

    fn temp_dir(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("gba-lib-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    fn wait_scan(lib: &mut Library) {
        for _ in 0..500 {
            lib.poll();
            if !lib.is_scanning() {
                return;
            }
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        panic!("scan did not finish");
    }

    #[test]
    fn scan_index_and_warm_start() {
        let games = temp_dir("games");
        let cache = temp_dir("cache");
        std::fs::write(games.join("Mario Kart - Super Circuit (USA).gba"), rom("MARIOKART", "AMKE")).unwrap();
        std::fs::write(games.join("Broken (USA).gba"), [0u8; 8]).unwrap();
        let mut lib = Library::new(vec![games.clone()], Some(cache.clone()));
        wait_scan(&mut lib);
        assert_eq!(lib.entries().len(), 2);
        assert_eq!(lib.entries()[0].title, "Broken (USA).gba", "sorted by title, unreadable by file name");
        let mk = lib.entries().iter().find(|e| e.code == "AMKE").unwrap().path.clone();
        lib.record_played(&mk, 120);
        lib.record_played(&mk, 60);
        assert_eq!(lib.played_secs(&mk), 180);
        assert_eq!(lib.last_played(), Some(mk.clone()));
        assert!(cache.join("index.toml").is_file());
        drop(lib);

        // the second start shows everything immediately, before any scan result is polled
        let lib2 = Library::new(vec![games.clone()], Some(cache.clone()));
        assert_eq!(lib2.entries().len(), 2);
        assert_eq!(lib2.played_secs(&mk), 180);
        assert_eq!(lib2.last_played(), Some(mk.clone()));
        // a different folder does not inherit them
        let other = Library::new(vec![temp_dir("other")], Some(cache.clone()));
        assert!(other.entries().is_empty());

        // deleting a file drops its entry once the scan confirms
        std::fs::remove_file(&mk).unwrap();
        let mut lib3 = Library::new(vec![games.clone()], Some(cache.clone()));
        wait_scan(&mut lib3);
        assert_eq!(lib3.entries().len(), 1);
        assert_eq!(lib3.last_played(), None);
        let _ = std::fs::remove_dir_all(&games);
        let _ = std::fs::remove_dir_all(&cache);
    }

    #[test]
    fn no_cache_dir_means_no_files_written() {
        let games = temp_dir("nocache");
        std::fs::write(games.join("A.gba"), rom("A", "AAAA")).unwrap();
        let mut lib = Library::new(vec![games.clone()], None);
        wait_scan(&mut lib);
        assert_eq!(lib.entries().len(), 1);
        lib.record_played(&lib.entries()[0].path.clone(), 5);
        assert_eq!(std::fs::read_dir(&games).unwrap().count(), 1, "nothing but the ROM");
        let _ = std::fs::remove_dir_all(&games);
    }

    #[test]
    fn slot_art_reaches_the_library() {
        let games = temp_dir("art");
        let path = games.join("A.gba");
        std::fs::write(&path, rom("A", "AAAA")).unwrap();
        let fb = vec![0x03E0u16; gba_core::SCREEN_W * gba_core::SCREEN_H]; // green
        std::fs::write(crate::slots::slot_path(&path, 1), crate::slots::encode(1, 1, 1, &fb, b"body")).unwrap();
        let mut lib = Library::new(vec![games.clone()], None);
        let mut got = None;
        for _ in 0..500 {
            lib.poll();
            if let Some(a) = lib.art_for(&path) {
                got = Some(a.rgb[..3].to_vec());
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        assert_eq!(got, Some(vec![0, 255, 0]));
        let _ = std::fs::remove_dir_all(&games);
    }
}
