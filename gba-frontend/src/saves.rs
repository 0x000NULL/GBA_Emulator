//! `<rom>.sav` persistence: load at start, write periodically / on demand / at exit.

use anyhow::{Context, Result};
use gba_core::Gba;
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

/// Tracks whether save memory changed since the last write. `Gba::save_dirty` clears the core's
/// flag on every call, so the frontend has to remember the answer between writes.
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
    pub fn load(&self, gba: &mut Gba) -> Result<bool> {
        if !self.path.is_file() {
            return Ok(false);
        }
        let data = fs::read(&self.path).with_context(|| format!("reading save {}", self.path.display()))?;
        gba.load_save(&data);
        gba.save_dirty(); // loading is not a modification
        Ok(true)
    }

    /// Poll the core's dirty flag; call once per frame.
    pub fn poll(&mut self, gba: &mut Gba) {
        if gba.save_dirty() {
            self.pending = true;
        }
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub fn is_pending(&self) -> bool {
        self.pending
    }

    /// Write if anything changed since the last write. Returns true if a file was written.
    pub fn flush(&mut self, gba: &mut Gba) -> Result<bool> {
        self.poll(gba);
        if !self.pending {
            return Ok(false);
        }
        self.force_write(gba)
    }

    /// Write regardless of the dirty flag (F5). Returns false if the cart has no save memory.
    pub fn force_write(&mut self, gba: &mut Gba) -> Result<bool> {
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

    /// Round trip through a real SRAM cart: modify save memory, flush, reload into a fresh machine.
    #[test]
    fn save_round_trip_through_core() {
        use gba_core::{BiosMode, Gba, MemoryBus, Options, SaveType};

        let d = temp_dir("roundtrip");
        let sav = d.join("test.sav");
        let mut rom = vec![0u8; 0x100000];
        rom[0xA0..0xAC].copy_from_slice(b"SAVETEST    ");
        rom[0xAC..0xB0].copy_from_slice(b"TEST");
        let opts = Options { bios: BiosMode::Hle, save_type: Some(SaveType::Sram32K) };

        let mut gba = Gba::new(rom.clone(), opts.clone()).unwrap();
        let mut mgr = SaveManager::new(sav.clone());
        assert!(!mgr.load(&mut gba).unwrap(), "no save yet");
        assert!(!mgr.flush(&mut gba).unwrap(), "clean cart writes nothing");
        assert!(!sav.exists());

        for (i, b) in b"hello save".iter().enumerate() {
            gba.bus.write8(0x0E00_0010 + i as u32, *b, gba_core::Access::N);
        }
        mgr.poll(&mut gba);
        assert!(mgr.is_pending());
        assert!(mgr.flush(&mut gba).unwrap());
        assert!(!mgr.is_pending());
        assert!(sav.is_file());
        let on_disk = fs::read(&sav).unwrap();
        assert_eq!(&on_disk[0x10..0x1A], b"hello save");

        let mut fresh = Gba::new(rom, opts).unwrap();
        let mgr2 = SaveManager::new(sav.clone());
        assert!(mgr2.load(&mut fresh).unwrap());
        assert!(!fresh.save_dirty(), "loading does not dirty the cart");
        for (i, b) in b"hello save".iter().enumerate() {
            assert_eq!(fresh.bus.read8(0x0E00_0010 + i as u32, gba_core::Access::N), *b);
        }
        fs::remove_dir_all(&d).unwrap();
    }
}
