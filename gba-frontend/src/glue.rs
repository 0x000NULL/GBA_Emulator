//! How `gba_core` plugs into the generic frontend traits.

use crate::library::{Art, Platform, RomInfo};
use crate::saves::{write_atomic, SaveRam};
use crate::slots::{slot_path, SlotFormat, Snapshot};
use anyhow::{anyhow, Context, Result};
use gba_core::cart::{Cartridge, Header};
use gba_core::{BiosMode, Gba, SCREEN_H, SCREEN_W};
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

/// Strawberry's slot files: `GBSL`, the core's state version, a 240x160 BGR555 picture.
pub const GBA_SLOTS: SlotFormat = SlotFormat {
    magic: *b"GBSL",
    core_version: gba_core::state::STATE_VERSION,
    emulator: env!("CARGO_PKG_VERSION"),
    frame_w: SCREEN_W,
    frame_h: SCREEN_H,
};

impl Snapshot for Gba {
    fn save_state(&self) -> Vec<u8> {
        Gba::save_state(self)
    }
    fn load_state(&mut self, body: &[u8]) -> Result<(), String> {
        Gba::load_state(self, body).map_err(|e| e.to_string())
    }
    fn frame_count(&self) -> u64 {
        self.bus.ppu.frame_count
    }
    fn picture(&self) -> &[u16] {
        &self.framebuffer()[..]
    }
}

/// The legacy single-state path written by builds before the slots.
pub fn legacy_path(rom: &Path) -> PathBuf {
    rom.with_extension("state")
}

/// Copy `<rom>.state` into slot 1 when slot 1 is empty; the original is left untouched.
/// `rom_bytes` builds a scratch machine for the picture and frame count.
pub fn import_legacy(rom: &Path, rom_bytes: &[u8]) -> Result<bool> {
    let legacy = legacy_path(rom);
    if !legacy.is_file() || slot_path(rom, 1).exists() {
        return Ok(false);
    }
    let body = std::fs::read(&legacy)?;
    let mut scratch = Gba::new(rom_bytes.to_vec(), gba_core::Options::default()).map_err(|e| anyhow!("{e}"))?;
    scratch.load_state(&body).map_err(|e| anyhow!("legacy state: {e}"))?;
    scratch.run_frame();
    let saved_at = std::fs::metadata(&legacy)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_secs() as i64)
        .unwrap_or_else(crate::slots::now_secs);
    let play = crate::ui::fmt::frames_to_secs(scratch.bus.ppu.frame_count, crate::config::GBA_FPS);
    let bytes = GBA_SLOTS.encode(scratch.bus.ppu.frame_count, saved_at, play as u32, &scratch.framebuffer()[..], &body);
    write_atomic(&slot_path(rom, 1), &bytes)?;
    println!("imported {} into slot 1", legacy.display());
    Ok(true)
}

/// The slots of `rom`, after importing a legacy state when `rom_bytes` is given.
pub fn scan_slots(rom: &Path, rom_bytes: Option<&[u8]>) -> crate::slots::SlotSet {
    if let Some(bytes) = rom_bytes {
        if let Err(e) = import_legacy(rom, bytes) {
            eprintln!("warning: legacy state import: {e:#}");
        }
    }
    GBA_SLOTS.scan(rom)
}

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
        GBA_SLOTS.scan(rom).latest().map(|m| art_from_frame(&m.frame_full[..]))
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
        let bytes = GBA_SLOTS.encode(10, 100, 5, &fb, b"body");
        fs::write(slot_path(&rom, 3), &bytes).unwrap();
        let art = GbaPlatform.slot_art(&rom).expect("slot 3 has a frame");
        assert_eq!(&art.rgb[..3], &[255, 0, 0]);
        let _ = fs::remove_dir_all(&dir);
    }

    /// A minimal cart: header + an infinite loop at the entry point.
    fn loop_rom() -> Vec<u8> {
        let mut rom = vec![0u8; 0x1000];
        rom[0..4].copy_from_slice(&0xEAFF_FFFEu32.to_le_bytes()); // b .
        rom[0xA0..0xAC].copy_from_slice(b"SLOTTEST\0\0\0\0");
        rom[0xAC..0xB0].copy_from_slice(b"TSTE");
        rom[0xB2] = 0x96;
        rom
    }

    #[test]
    fn slots_round_trip_a_real_machine() {
        use crate::slots::SlotError;
        let rom = temp_dir("slots-rt").join("game.gba");
        let mut gba = Gba::new(loop_rom(), gba_core::Options::default()).unwrap();
        for _ in 0..3 {
            gba.run_frame();
        }
        let frame = gba.bus.ppu.frame_count;
        let meta = GBA_SLOTS.save(&gba, &rom, 2, 458).unwrap();
        assert_eq!((meta.slot, meta.frame, meta.play_secs), (2, frame, 458));
        assert!(meta.broken.is_none());
        assert_eq!(meta.core_ver, gba_core::state::STATE_VERSION);
        assert_eq!(meta.emulator, env!("CARGO_PKG_VERSION"));
        assert!(meta.size > 76_800);
        assert_eq!(meta.frame_full, gba.framebuffer().to_vec());
        for _ in 0..5 {
            gba.run_frame();
        }
        assert_ne!(gba.bus.ppu.frame_count, frame);
        let m2 = GBA_SLOTS.load(&mut gba, &rom, 2).unwrap();
        assert_eq!((gba.bus.ppu.frame_count, m2.frame), (frame, frame));
        // a future core version is listed, not loaded
        let mut bytes = GBA_SLOTS.encode(10, 0, 1, &vec![0u16; SCREEN_W * SCREEN_H], &gba.save_state());
        bytes[32..36].copy_from_slice(&(gba_core::state::STATE_VERSION + 1).to_le_bytes());
        fs::write(slot_path(&rom, 3), &bytes).unwrap();
        assert!(GBA_SLOTS.read_meta(&rom, 3).unwrap().broken.unwrap().contains("state format"));
        let pause = crate::overlay::pause::slot_infos(&GBA_SLOTS.scan(&rom));
        assert!(pause[1].as_ref().unwrap().broken.is_none());
        assert!(pause[2].as_ref().unwrap().broken.is_some());
        fs::write(slot_path(&rom, 4), b"nonsense").unwrap();
        assert!(matches!(GBA_SLOTS.load(&mut gba, &rom, 4), Err(SlotError::Corrupt(_))));
    }

    #[test]
    fn legacy_state_is_imported_into_slot_one_once() {
        let rom = temp_dir("legacy").join("game.gba");
        assert_eq!(legacy_path(Path::new("C:/Games/x.gba")), PathBuf::from("C:/Games/x.state"));
        let bytes = loop_rom();
        let mut gba = Gba::new(bytes.clone(), gba_core::Options::default()).unwrap();
        for _ in 0..4 {
            gba.run_frame();
        }
        let legacy = legacy_path(&rom);
        fs::write(&legacy, gba.save_state()).unwrap();
        let set = scan_slots(&rom, Some(&bytes));
        let m = set.get(1).expect("imported");
        assert_eq!(m.frame, gba.bus.ppu.frame_count + 1, "one frame run for the picture");
        assert!(legacy.is_file(), "original untouched");
        // a later scan does not overwrite slot 1
        GBA_SLOTS.save(&gba, &rom, 1, 7).unwrap();
        let set2 = scan_slots(&rom, Some(&bytes));
        assert_eq!(set2.get(1).unwrap().play_secs, 7);
    }
}
