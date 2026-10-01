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
    let gba = Gba::new(rom, Options { bios, save_type }).map_err(|e| anyhow!("{e}"))?;
    Ok(gba)
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
