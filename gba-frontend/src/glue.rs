//! How `gba_core` plugs into the generic frontend traits.

use crate::library::{Art, Platform, RomInfo};
use crate::saves::SaveRam;
use crate::slots::SlotSet;
use anyhow::{Context, Result};
use gba_core::cart::{Cartridge, Header};
use gba_core::{BiosMode, Gba, SCREEN_H, SCREEN_W};
use std::path::Path;

/// Frames a title capture runs (about 30 s of game time: past the logos, on the title screen).
pub const CAPTURE_FRAMES: u32 = 1800;

/// The library's view of the GBA: `.gba` / `.zip` files, cartridge headers, slot frames, and title
/// captures run headless with the HLE BIOS.
pub struct GbaPlatform;

/// A 240x160 BGR555 frame as art.
pub fn art_from_frame(fb: &[u16]) -> Art {
    let mut rgb = Vec::with_capacity(SCREEN_W * SCREEN_H * 3);
    for &p in fb.iter().take(SCREEN_W * SCREEN_H) {
        rgb.extend_from_slice(&gba_test::bgr555_to_rgb(p));
    }
    Art::new(SCREEN_W, SCREEN_H, rgb)
}

impl Platform for GbaPlatform {
    fn extensions(&self) -> &'static [&'static str] {
        &["gba", "zip"]
    }

    fn probe(&self, path: &Path) -> RomInfo {
        let rom = match gba_test::load_rom(path) {
            Ok(rom) => rom,
            Err(e) => return RomInfo { unreadable: Some(format!("{e:#}")), ..RomInfo::default() },
        };
        let rom_len = rom.len() as u64;
        // Header::parse indexes up to 0xC0 and panics on anything shorter
        if rom.len() < 0xC0 {
            return RomInfo { rom_len, unreadable: Some(format!("only {} bytes: too small to be a GBA ROM", rom.len())), ..RomInfo::default() };
        }
        let header = Header::parse(&rom);
        RomInfo {
            rom_len,
            code: header.game_code_str(),
            version: header.version,
            save: crate::gba_copy::save_kind(Cartridge::detect_save_type(&rom, &header)).into(),
            header_title: header.title.clone(),
            unreadable: None,
        }
    }

    fn slot_art(&self, rom: &Path) -> Option<Art> {
        let set = SlotSet::scan(rom, None);
        set.latest().map(|m| art_from_frame(&m.frame_full[..]))
    }

    /// Run `rom` for [`CAPTURE_FRAMES`] and write the final frame to `out` (temp file + rename).
    fn capture_title(&self, rom: &Path, out: &Path) -> Result<Art> {
        let mut gba = gba_test::build_gba(rom, Some(BiosMode::Hle), None)?;
        for _ in 0..CAPTURE_FRAMES {
            gba.run_frame();
        }
        let fb = gba.framebuffer();
        if let Some(dir) = out.parent() {
            std::fs::create_dir_all(dir).with_context(|| format!("creating {}", dir.display()))?;
        }
        let tmp = out.with_extension("png.tmp");
        gba_test::write_png(&tmp, &fb[..])?;
        std::fs::rename(&tmp, out).with_context(|| format!("renaming {}", tmp.display()))?;
        Ok(art_from_frame(&fb[..]))
    }
}

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
    use super::*;
    use crate::library::capture::read_png;
    use crate::library::scan::scan_collect;
    use crate::saves::SaveManager;
    use std::fs;
    use std::path::PathBuf;

    /// A header-only test ROM like the one in `slots.rs`.
    fn test_rom(title: &str, code: &str) -> Vec<u8> {
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
        let d = std::env::temp_dir().join(format!("strawberry-glue-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&d);
        fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn probes_good_tiny_and_broken_roms() {
        let dir = temp_dir("probe");
        fs::write(dir.join("Kingdom Hearts - Chain of Memories (USA).gba"), test_rom("KINGDOM HEART", "B8CE")).unwrap();
        fs::write(dir.join("Tiny.gba"), [0u8; 16]).unwrap();
        fs::write(dir.join("Broken (USA).zip"), b"PK\x03\x04 not really a zip").unwrap();
        fs::write(dir.join("ZZZZ.zip.txt"), b"ignored").unwrap();
        let got = scan_collect(std::slice::from_ref(&dir), &[], &GbaPlatform);
        assert_eq!(got.len(), 3, "{got:?}");
        let kh = got.iter().find(|e| e.code == "B8CE").expect("the good ROM");
        assert_eq!((kh.title.as_str(), kh.subtitle.as_str()), ("Kingdom Hearts", "Chain of Memories · USA"));
        assert_eq!((kh.rom_len, kh.version, kh.save.as_str()), (0x1000, 1, "sram32k"));
        assert!(kh.unreadable.is_none());
        let tiny = got.iter().find(|e| e.title == "Tiny.gba").expect("too small is listed, unreadable");
        assert!(tiny.unreadable.as_deref().unwrap().contains("too small"));
        assert_eq!(tiny.rom_len, 16);
        let bad = got.iter().find(|e| e.title == "Broken (USA).zip").expect("bad zip is listed, unreadable");
        assert!(bad.unreadable.is_some());
        let _ = fs::remove_dir_all(&dir);
    }

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

    #[test]
    fn png_round_trip_through_gba_test() {
        let dir = std::env::temp_dir().join(format!("gba-capture-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let fb: Vec<u16> = (0..SCREEN_W * SCREEN_H).map(|i| (i as u16).wrapping_mul(7) & 0x7FFF).collect();
        let file = dir.join("x.png");
        gba_test::write_png(&file, &fb).unwrap();
        let art = read_png(&file).expect("decodes");
        let direct = art_from_frame(&fb);
        assert_eq!((art.w, art.h), (240, 160));
        assert_eq!(art.rgb, direct.rgb);
        assert!(read_png(&dir.join("missing.png")).is_none());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn slot_art_is_the_newest_slot_frame() {
        let dir = temp_dir("slotart");
        let rom = dir.join("g.gba");
        assert!(GbaPlatform.slot_art(&rom).is_none());
        let fb = vec![0x001Fu16; SCREEN_W * SCREEN_H]; // red
        let bytes = crate::slots::encode(10, 100, 5, &fb, b"body");
        fs::write(crate::slots::slot_path(&rom, 3), &bytes).unwrap();
        let art = GbaPlatform.slot_art(&rom).expect("slot 3 has a frame");
        assert_eq!(&art.rgb[..3], &[255, 0, 0]);
        let _ = fs::remove_dir_all(&dir);
    }
}
