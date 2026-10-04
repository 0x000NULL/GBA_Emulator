//! Headless harness API shared by the `gba-test` binary, the `cargo test` driver and the frontend.

pub mod checks;
pub mod golden;
pub mod input_script;

use anyhow::{anyhow, Context, Result};
use gba_core::{BiosMode, Gba, Options, SaveType, SCREEN_H, SCREEN_W};
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

/// Load a ROM from a `.gba` file or the first `.gba` entry of a `.zip`.
pub fn load_rom(path: &Path) -> Result<Vec<u8>> {
    let bytes = fs::read(path).with_context(|| format!("reading {}", path.display()))?;
    if path.extension().map_or(false, |e| e.eq_ignore_ascii_case("zip")) || bytes.starts_with(b"PK\x03\x04") {
        let cursor = std::io::Cursor::new(bytes);
        let mut archive = zip::ZipArchive::new(cursor).context("opening zip")?;
        let idx = (0..archive.len())
            .find(|&i| archive.by_index(i).map(|f| f.name().to_ascii_lowercase().ends_with(".gba")).unwrap_or(false))
            .ok_or_else(|| anyhow!("no .gba entry in {}", path.display()))?;
        let mut f = archive.by_index(idx)?;
        let mut out = Vec::with_capacity(f.size() as usize);
        f.read_to_end(&mut out)?;
        return Ok(out);
    }
    Ok(bytes)
}

/// `<rom>.sav` beside the ROM (zip or gba), e.g. `Foo (USA).zip` -> `Foo (USA).sav`.
pub fn save_path_for(rom: &Path) -> PathBuf {
    rom.with_extension("sav")
}

/// Build a machine from a ROM path and options. Reads `GBA_BIOS` if `bios` is None and the env var is set.
pub fn build_gba(rom_path: &Path, bios: Option<BiosMode>, save_type: Option<SaveType>) -> Result<Gba> {
    let rom = load_rom(rom_path)?;
    let bios = match bios {
        Some(b) => b,
        None => match std::env::var_os("GBA_BIOS") {
            Some(p) if !p.is_empty() => BiosMode::Lle(fs::read(&p).with_context(|| format!("reading BIOS {}", Path::new(&p).display()))?),
            _ => BiosMode::Hle,
        },
    };
    let mut gba = Gba::new(rom, Options { bios, save_type }).map_err(|e| anyhow!("{e}"))?;
    gba.set_rtc_clock(RTC_DEFAULT);
    Ok(gba)
}

/// Cartridge clock seed for headless runs (2026-01-01 00:00:00), so goldens are deterministic.
/// The frontend reseeds with the wall clock.
pub const RTC_DEFAULT: i64 = 20454 * 86400;

/// Parse `YYYY-MM-DD HH:MM:SS` (or `YYYY-MM-DDTHH:MM:SS`, seconds optional) into seconds since
/// 1970-01-01, the unit of `Gba::set_rtc_clock`.
pub fn parse_rtc(s: &str) -> Result<i64> {
    let bad = || anyhow!("expected YYYY-MM-DD HH:MM[:SS], got {s:?}");
    let (date, time) = s.trim().split_once([' ', 'T']).ok_or_else(bad)?;
    let d: Vec<u32> = date.split('-').map(|x| x.parse().map_err(|_| bad())).collect::<Result<_>>()?;
    let t: Vec<u32> = time.split(':').map(|x| x.parse().map_err(|_| bad())).collect::<Result<_>>()?;
    if d.len() != 3 || !(2..=3).contains(&t.len()) || !(1..=12).contains(&d[1]) || !(1..=31).contains(&d[2]) || t[0] > 23 || t[1] > 59 {
        return Err(bad());
    }
    let days = gba_core::cart::rtc::days_from_civil(d[0] as i64, d[1], d[2]);
    Ok(days * 86400 + (t[0] * 3600 + t[1] * 60 + t.get(2).copied().unwrap_or(0)) as i64)
}

/// xxh3-64 of the framebuffer as little-endian u16s.
/// The sweep's "this frame made a sound" test: any sample above +-64 on either side.
pub fn audio_active(samples: &[[i16; 2]]) -> bool {
    samples.iter().any(|[l, r]| l.unsigned_abs() > 64 || r.unsigned_abs() > 64)
}

pub fn fb_hash(fb: &[u16]) -> u64 {
    let mut bytes = Vec::with_capacity(fb.len() * 2);
    for &p in fb {
        bytes.extend_from_slice(&p.to_le_bytes());
    }
    xxhash_rust::xxh3::xxh3_64(&bytes)
}

/// Convert BGR555 to RGB888.
pub fn bgr555_to_rgb(p: u16) -> [u8; 3] {
    let r = (p & 0x1F) as u8;
    let g = ((p >> 5) & 0x1F) as u8;
    let b = ((p >> 10) & 0x1F) as u8;
    [(r << 3) | (r >> 2), (g << 3) | (g >> 2), (b << 3) | (b >> 2)]
}

pub fn write_png(path: &Path, fb: &[u16]) -> Result<()> {
    let file = fs::File::create(path)?;
    let mut enc = png::Encoder::new(std::io::BufWriter::new(file), SCREEN_W as u32, SCREEN_H as u32);
    enc.set_color(png::ColorType::Rgb);
    enc.set_depth(png::BitDepth::Eight);
    let mut w = enc.write_header()?;
    let mut data = Vec::with_capacity(fb.len() * 3);
    for &p in fb {
        data.extend_from_slice(&bgr555_to_rgb(p));
    }
    w.write_image_data(&data)?;
    Ok(())
}

/// Write 16-bit stereo PCM as a WAV file.
pub fn write_wav(path: &Path, samples: &[[i16; 2]], rate: u32) -> Result<()> {
    let data_len = (samples.len() * 4) as u32;
    let mut out = Vec::with_capacity(44 + data_len as usize);
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(36 + data_len).to_le_bytes());
    out.extend_from_slice(b"WAVEfmt ");
    out.extend_from_slice(&16u32.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes()); // PCM
    out.extend_from_slice(&2u16.to_le_bytes()); // stereo
    out.extend_from_slice(&rate.to_le_bytes());
    out.extend_from_slice(&(rate * 4).to_le_bytes());
    out.extend_from_slice(&4u16.to_le_bytes());
    out.extend_from_slice(&16u16.to_le_bytes());
    out.extend_from_slice(b"data");
    out.extend_from_slice(&data_len.to_le_bytes());
    for s in samples {
        out.extend_from_slice(&s[0].to_le_bytes());
        out.extend_from_slice(&s[1].to_le_bytes());
    }
    fs::write(path, out)?;
    Ok(())
}

/// Format the CPU state for the harness output.
pub fn cpu_state_line(gba: &Gba) -> String {
    let r = &gba.cpu.regs;
    let mut s = String::new();
    for i in 0..16 {
        s.push_str(&format!("r{i}={:08X} ", r.r[i]));
    }
    s.push_str(&format!("cpsr={:08X} spsr={:08X} cycles={} instrs={}", r.cpsr, r.spsr(), gba.bus.sched.now, gba.cpu.instr_count));
    s
}

/// Directory holding the test ROMs (`gba-tests/` and `suite/`), from `GBA_TESTS_DIR`.
pub fn tests_dir() -> Option<PathBuf> {
    std::env::var_os("GBA_TESTS_DIR").filter(|p| !p.is_empty()).map(PathBuf::from)
}

/// Repository root (the directory holding the workspace `Cargo.toml`), for locating `tests/golden.toml`.
pub fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap().to_path_buf()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn load_rom_takes_the_gba_entry_of_a_zip() {
        let dir = std::env::temp_dir().join(format!("gba-test-zip-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let rom: Vec<u8> = (0..0x2000u32).map(|i| (i * 7) as u8).collect();
        let zip_path = dir.join("Pack (USA).zip");
        let mut w = zip::ZipWriter::new(fs::File::create(&zip_path).unwrap());
        let opts = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
        w.start_file("readme.txt", opts).unwrap();
        w.write_all(b"not a rom").unwrap();
        w.start_file("Inner Name.GBA", opts).unwrap();
        w.write_all(&rom).unwrap();
        w.finish().unwrap();
        assert_eq!(load_rom(&zip_path).unwrap(), rom);
        assert_eq!(save_path_for(&zip_path), dir.join("Pack (USA).sav"));
        let _ = fs::remove_dir_all(&dir);
    }
}
