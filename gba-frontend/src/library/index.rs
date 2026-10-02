//! The library index: `<cache_dir>/index.toml`.
//!
//! One `[[entry]]` per ROM file, keyed by path + size + mtime, so a warm start shows the whole
//! case before the scan has confirmed anything and never re-reads a 32 MB ROM that has not
//! changed. The per-game play statistics (`played_secs`, `last_played`) live here too.

use super::LibraryEntry;
use crate::saves::write_atomic;
use anyhow::{Context, Result};
use gba_core::SaveType;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

pub const FILE_NAME: &str = "index.toml";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct IndexEntry {
    pub path: String,
    pub size: u64,
    /// Modification time, unix seconds.
    pub mtime: i64,
    pub title: String,
    #[serde(default)]
    pub subtitle: String,
    #[serde(default)]
    pub code: String,
    #[serde(default)]
    pub version: u8,
    #[serde(default = "default_save")]
    pub save: String,
    #[serde(default)]
    pub rom_len: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unreadable: Option<String>,
    /// Total play time in seconds.
    #[serde(default)]
    pub played_secs: u64,
    /// Unix seconds of the last time the game was played; 0 = never.
    #[serde(default)]
    pub last_played: i64,
}

fn default_save() -> String {
    save_to_str(SaveType::None).into()
}

pub fn save_to_str(t: SaveType) -> &'static str {
    match t {
        SaveType::None => "none",
        SaveType::Sram32K => "sram32k",
        SaveType::Flash64K => "flash64k",
        SaveType::Flash128K => "flash128k",
        SaveType::Eeprom => "eeprom",
    }
}

pub fn save_from_str(s: &str) -> SaveType {
    match s {
        "sram32k" => SaveType::Sram32K,
        "flash64k" => SaveType::Flash64K,
        "flash128k" => SaveType::Flash128K,
        "eeprom" => SaveType::Eeprom,
        _ => SaveType::None,
    }
}

impl IndexEntry {
    /// The scan result for this index row.
    pub fn to_entry(&self) -> LibraryEntry {
        LibraryEntry {
            path: PathBuf::from(&self.path),
            size: self.size,
            mtime: self.mtime,
            title: self.title.clone(),
            subtitle: self.subtitle.clone(),
            code: self.code.clone(),
            version: self.version,
            save: save_from_str(&self.save),
            rom_len: self.rom_len,
            unreadable: self.unreadable.clone(),
        }
    }

    /// Copy the scan fields from `e`, keeping the play statistics.
    pub fn update_from(&mut self, e: &LibraryEntry) {
        self.path = e.path.to_string_lossy().into_owned();
        self.size = e.size;
        self.mtime = e.mtime;
        self.title = e.title.clone();
        self.subtitle = e.subtitle.clone();
        self.code = e.code.clone();
        self.version = e.version;
        self.save = save_to_str(e.save).into();
        self.rom_len = e.rom_len;
        self.unreadable = e.unreadable.clone();
    }

    pub fn from_entry(e: &LibraryEntry) -> Self {
        let mut ie = IndexEntry::default();
        ie.update_from(e);
        ie
    }

    pub fn matches(&self, path: &Path, size: u64, mtime: i64) -> bool {
        self.size == size && self.mtime == mtime && Path::new(&self.path) == path
    }
}

#[derive(Serialize, Deserialize, Default)]
struct IndexFile {
    #[serde(default, rename = "entry")]
    entries: Vec<IndexEntry>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Index {
    pub entries: Vec<IndexEntry>,
}

impl Index {
    /// Read `<dir>/index.toml`. A missing or unreadable file is an empty index: the cache is only
    /// ever an accelerator.
    pub fn load(dir: &Path) -> Index {
        let path = dir.join(FILE_NAME);
        let Ok(text) = std::fs::read_to_string(&path) else { return Index::default() };
        match toml::from_str::<IndexFile>(&text) {
            Ok(f) => Index { entries: f.entries },
            Err(e) => {
                eprintln!("warning: ignoring {}: {e}", path.display());
                Index::default()
            }
        }
    }

    /// Write `<dir>/index.toml` atomically.
    pub fn save(&self, dir: &Path) -> Result<()> {
        std::fs::create_dir_all(dir).with_context(|| format!("creating {}", dir.display()))?;
        let text = toml::to_string_pretty(&IndexFile { entries: self.entries.clone() }).context("serialising the library index")?;
        write_atomic(&dir.join(FILE_NAME), text.as_bytes())
    }

    pub fn get(&self, path: &Path) -> Option<&IndexEntry> {
        self.entries.iter().find(|e| Path::new(&e.path) == path)
    }

    pub fn get_mut(&mut self, path: &Path) -> Option<&mut IndexEntry> {
        self.entries.iter_mut().find(|e| Path::new(&e.path) == path)
    }

    /// Insert or refresh the scan fields of `e`, keeping the play statistics of an existing row.
    pub fn upsert(&mut self, e: &LibraryEntry) {
        match self.get_mut(&e.path) {
            Some(ie) => ie.update_from(e),
            None => self.entries.push(IndexEntry::from_entry(e)),
        }
    }

    /// Add `secs` of play time and stamp the last-played time. Creates a stub row for a path the
    /// scan has not seen (a ROM given on the command line from outside the folders).
    pub fn record_played(&mut self, path: &Path, secs: u64, now: i64) {
        if self.get(path).is_none() {
            let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
            let dn = super::names::from_file_name(&name);
            self.entries.push(IndexEntry {
                path: path.to_string_lossy().into_owned(),
                title: dn.map(|d| d.title).unwrap_or(name),
                save: default_save(),
                ..IndexEntry::default()
            });
        }
        if let Some(e) = self.get_mut(path) {
            e.played_secs = e.played_secs.saturating_add(secs);
            e.last_played = now;
        }
    }

    pub fn played_secs(&self, path: &Path) -> u64 {
        self.get(path).map_or(0, |e| e.played_secs)
    }

    /// Path of the most recently played game among `candidates` (the scanned entries).
    pub fn last_played<'a>(&self, candidates: impl Iterator<Item = &'a Path>) -> Option<PathBuf> {
        candidates
            .filter_map(|p| self.get(p).filter(|e| e.last_played > 0).map(|e| (e.last_played, p)))
            .max_by_key(|(t, _)| *t)
            .map(|(_, p)| p.to_path_buf())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(name: &str) -> LibraryEntry {
        LibraryEntry {
            path: PathBuf::from(format!("C:/Games/{name}.zip")),
            size: 33_554_432,
            mtime: 1_790_000_000,
            title: name.into(),
            subtitle: "Chain of Memories · USA".into(),
            code: "B8CE".into(),
            version: 1,
            save: SaveType::Sram32K,
            rom_len: 33_554_432,
            unreadable: None,
        }
    }

    fn temp_dir(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("gba-index-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        d
    }

    #[test]
    fn pinned_index_file() {
        let dir = temp_dir("pin");
        let mut idx = Index::default();
        idx.upsert(&entry("Kingdom Hearts"));
        let mut bad = entry("Broken");
        bad.unreadable = Some("no .gba entry in Broken.zip".into());
        bad.save = SaveType::None;
        idx.upsert(&bad);
        let mut e = entry("Minish Cap");
        e.save = SaveType::Eeprom;
        idx.upsert(&e);
        idx.record_played(&entry("Kingdom Hearts").path, 458, 1_790_000_500);
        idx.save(&dir).unwrap();
        let text = std::fs::read_to_string(dir.join("index.toml")).unwrap();
        let _ = std::fs::remove_dir_all(&dir);
        assert_eq!(text, INDEX_TOML, "index.toml changed:\n{text}");
    }

    const INDEX_TOML: &str = r#"[[entry]]
path = "C:/Games/Kingdom Hearts.zip"
size = 33554432
mtime = 1790000000
title = "Kingdom Hearts"
subtitle = "Chain of Memories · USA"
code = "B8CE"
version = 1
save = "sram32k"
rom_len = 33554432
played_secs = 458
last_played = 1790000500

[[entry]]
path = "C:/Games/Broken.zip"
size = 33554432
mtime = 1790000000
title = "Broken"
subtitle = "Chain of Memories · USA"
code = "B8CE"
version = 1
save = "none"
rom_len = 33554432
unreadable = "no .gba entry in Broken.zip"
played_secs = 0
last_played = 0

[[entry]]
path = "C:/Games/Minish Cap.zip"
size = 33554432
mtime = 1790000000
title = "Minish Cap"
subtitle = "Chain of Memories · USA"
code = "B8CE"
version = 1
save = "eeprom"
rom_len = 33554432
played_secs = 0
last_played = 0
"#;

    #[test]
    fn round_trip_through_toml() {
        let dir = temp_dir("rt");
        let mut idx = Index::default();
        idx.upsert(&entry("Kingdom Hearts"));
        let mut bad = entry("Broken");
        bad.unreadable = Some("no .gba entry in Broken.zip".into());
        bad.save = SaveType::None;
        idx.upsert(&bad);
        idx.record_played(&entry("Kingdom Hearts").path, 458, 1_790_000_500);
        idx.save(&dir).unwrap();
        assert!(!dir.join("index.toml.tmp").exists(), "temp file renamed away");
        let back = Index::load(&dir);
        assert_eq!(back, idx);
        let kh = back.get(&entry("Kingdom Hearts").path).unwrap();
        assert_eq!((kh.played_secs, kh.last_played), (458, 1_790_000_500));
        assert_eq!(kh.to_entry(), {
            let mut e = entry("Kingdom Hearts");
            e.title = "Kingdom Hearts".into();
            e
        });
        assert_eq!(back.get(&bad.path).unwrap().to_entry().unreadable.as_deref(), Some("no .gba entry in Broken.zip"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn upsert_keeps_play_statistics_and_key_matching() {
        let mut idx = Index::default();
        let mut e = entry("A");
        idx.upsert(&e);
        idx.record_played(&e.path, 60, 10);
        idx.record_played(&e.path, 30, 20);
        e.mtime += 1;
        e.title = "A2".into();
        idx.upsert(&e);
        assert_eq!(idx.entries.len(), 1);
        let row = idx.get(&e.path).unwrap();
        assert_eq!((row.played_secs, row.last_played, row.title.as_str()), (90, 20, "A2"));
        assert!(row.matches(&e.path, e.size, e.mtime));
        assert!(!row.matches(&e.path, e.size, e.mtime - 1));
        assert!(!row.matches(Path::new("other.zip"), e.size, e.mtime));
    }

    #[test]
    fn last_played_picks_the_newest_known_game() {
        let mut idx = Index::default();
        let (a, b, c) = (entry("A"), entry("B"), entry("C"));
        for e in [&a, &b, &c] {
            idx.upsert(e);
        }
        assert_eq!(idx.last_played([a.path.as_path(), b.path.as_path()].into_iter()), None);
        idx.record_played(&a.path, 1, 100);
        idx.record_played(&b.path, 1, 300);
        idx.record_played(&c.path, 1, 200);
        assert_eq!(idx.last_played([a.path.as_path(), c.path.as_path()].into_iter()), Some(c.path.clone()));
        assert_eq!(idx.last_played([a.path.as_path(), b.path.as_path(), c.path.as_path()].into_iter()), Some(b.path.clone()));
    }

    #[test]
    fn missing_and_corrupt_files_are_empty_indexes() {
        let dir = temp_dir("bad");
        assert!(Index::load(&dir).entries.is_empty());
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join(FILE_NAME), "this is [not toml").unwrap();
        assert!(Index::load(&dir).entries.is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn unknown_played_path_gets_a_stub_row() {
        let mut idx = Index::default();
        idx.record_played(Path::new("D:/x/Mario Kart - Super Circuit (USA).zip"), 5, 1);
        let row = &idx.entries[0];
        assert_eq!(row.title, "Mario Kart");
        assert_eq!(idx.played_secs(Path::new("D:/x/Mario Kart - Super Circuit (USA).zip")), 5);
    }

    #[test]
    fn save_type_strings_round_trip() {
        for t in [SaveType::None, SaveType::Sram32K, SaveType::Flash64K, SaveType::Flash128K, SaveType::Eeprom] {
            assert_eq!(save_from_str(save_to_str(t)), t);
        }
    }
}
