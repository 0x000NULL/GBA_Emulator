//! How `gba_core` plugs into the generic frontend traits.

use crate::saves::SaveRam;
use gba_core::Gba;

impl SaveRam for Gba {
    fn save_dirty(&mut self) -> bool {
        Gba::save_dirty(self)
    }
    fn save_data(&self) -> Option<Vec<u8>> {
        Gba::save_data(self)
    }
    fn load_save(&mut self, data: &[u8]) {
        Gba::load_save(self, data)
    }
}

#[cfg(test)]
mod tests {
    use crate::saves::SaveManager;
    use std::fs;

    /// Round trip through a real SRAM cart: modify save memory, flush, reload into a fresh machine.
    #[test]
    fn save_round_trip_through_core() {
        use gba_core::{BiosMode, Gba, MemoryBus, Options, SaveType};

        let d = std::env::temp_dir().join(format!("strawberry-glue-roundtrip-{}", std::process::id()));
        let _ = fs::remove_dir_all(&d);
        fs::create_dir_all(&d).unwrap();
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
