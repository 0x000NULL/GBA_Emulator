//! `<rom>.sav` persistence: load at start, write periodically / on demand / at exit.

use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};

/// Write `data` to `path` atomically: temp file beside it, then rename over the target.
pub fn write_atomic(path: &Path, data: &[u8]) -> Result<()> {
    let tmp = temp_path(path);
    fs::write(&tmp, data).with_context(|| format!("writing {}", tmp.display()))?;
    fs::rename(&tmp, path).with_context(|| format!("renaming {} -> {}", tmp.display(), path.display()))?;
    Ok(())
}

fn temp_path(path: &Path) -> PathBuf {
    let mut name = path.file_name().map(|n| n.to_os_string()).unwrap_or_default();
    name.push(".tmp");
    path.with_file_name(name)
}

/// A machine's battery-backed save memory, as the `.sav` file sees it.
pub trait SaveRam {
    /// True if save memory changed since the last call; clears the flag.
    fn save_dirty(&mut self) -> bool;
    /// The save memory's bytes, `None` when the cartridge has none.
    fn save_data(&self) -> Option<Vec<u8>>;
    /// Replace save memory with `data` (a `.sav` file's contents).
    fn load_save(&mut self, data: &[u8]);
}

/// Tracks whether save memory changed since the last write. [`SaveRam::save_dirty`] clears the
/// machine's flag on every call, so the frontend has to remember the answer between writes.
pub struct SaveManager {
    path: PathBuf,
    pending: bool,
    pub writes: u64,
}

impl SaveManager {
    pub fn new(path: PathBuf) -> Self {
        SaveManager { path, pending: false, writes: 0 }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Load an existing save into the machine. Returns true if one was loaded.
    pub fn load(&self, gba: &mut impl SaveRam) -> Result<bool> {
        if !self.path.is_file() {
            return Ok(false);
        }
        let data = fs::read(&self.path).with_context(|| format!("reading save {}", self.path.display()))?;
        gba.load_save(&data);
        gba.save_dirty(); // loading is not a modification
        Ok(true)
    }

    /// Poll the core's dirty flag; call once per frame.
    pub fn poll(&mut self, gba: &mut impl SaveRam) {
        if gba.save_dirty() {
            self.pending = true;
        }
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub fn is_pending(&self) -> bool {
        self.pending
    }

    /// Write if anything changed since the last write. Returns true if a file was written.
    pub fn flush(&mut self, gba: &mut impl SaveRam) -> Result<bool> {
        self.poll(gba);
        if !self.pending {
            return Ok(false);
        }
        self.force_write(gba)
    }

    /// Write regardless of the dirty flag (F5). Returns false if the cart has no save memory.
    pub fn force_write(&mut self, gba: &mut impl SaveRam) -> Result<bool> {
        let Some(data) = gba.save_data() else {
            self.pending = false;
            return Ok(false);
        };
        write_atomic(&self.path, &data)?;
        self.pending = false;
        self.writes += 1;
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("gba-frontend-{tag}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&d);
        fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn atomic_write_replaces_and_leaves_no_temp() {
        let d = temp_dir("atomic");
        let p = d.join("game.sav");
        write_atomic(&p, b"first").unwrap();
        assert_eq!(fs::read(&p).unwrap(), b"first");
        write_atomic(&p, b"second, longer").unwrap();
        assert_eq!(fs::read(&p).unwrap(), b"second, longer");
        assert!(!d.join("game.sav.tmp").exists());
        assert_eq!(fs::read_dir(&d).unwrap().count(), 1);
        fs::remove_dir_all(&d).unwrap();
    }

    #[test]
    fn temp_path_is_beside_target() {
        assert_eq!(temp_path(Path::new("C:/roms/Foo (USA).sav")), PathBuf::from("C:/roms/Foo (USA).sav.tmp"));
    }

    /// Save memory in a Vec, with the dirty flag a real machine keeps.
    #[derive(Default)]
    struct Ram {
        data: Option<Vec<u8>>,
        dirty: bool,
    }

    impl SaveRam for Ram {
        fn save_dirty(&mut self) -> bool {
            std::mem::replace(&mut self.dirty, false)
        }
        fn save_data(&self) -> Option<Vec<u8>> {
            self.data.clone()
        }
        fn load_save(&mut self, data: &[u8]) {
            self.data = Some(data.to_vec());
            self.dirty = true;
        }
    }

    #[test]
    fn writes_only_after_a_change_and_loading_is_not_one() {
        let d = temp_dir("fake");
        let sav = d.join("game.sav");
        let mut ram = Ram { data: Some(vec![0; 4]), dirty: false };
        let mut mgr = SaveManager::new(sav.clone());
        assert!(!mgr.load(&mut ram).unwrap(), "no save yet");
        assert!(!mgr.flush(&mut ram).unwrap(), "clean memory writes nothing");
        ram.data = Some(vec![1, 2, 3, 4]);
        ram.dirty = true;
        mgr.poll(&mut ram);
        assert!(mgr.is_pending());
        assert!(mgr.flush(&mut ram).unwrap());
        assert_eq!((fs::read(&sav).unwrap(), mgr.writes), (vec![1, 2, 3, 4], 1));
        let mut fresh = Ram::default();
        assert!(SaveManager::new(sav.clone()).load(&mut fresh).unwrap());
        assert_eq!(fresh.data.as_deref(), Some(&[1u8, 2, 3, 4][..]));
        assert!(!fresh.dirty, "loading does not dirty the memory");
        let mut none = Ram::default();
        assert!(!mgr.force_write(&mut none).unwrap(), "no save memory, nothing written");
        fs::remove_dir_all(&d).unwrap();
    }
}
