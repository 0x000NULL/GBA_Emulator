//! Label art for the cartridges.
//!
//! Order of preference per game: the newest save-state picture ([`Platform::slot_art`]) -> a
//! cached title capture `<cache_dir>/captures/<code>-<hash of path>.png` -> none (the cartridge
//! gets a plain colour label). Captures ([`Platform::capture_title`]) are produced by ONE
//! background thread, one game at a time, only after every cheap lookup has been answered.

use super::Platform;
use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver, Sender, TryRecvError};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

static NEXT_ART_ID: AtomicU64 = AtomicU64::new(1);

/// A decoded picture: packed RGB8.
#[derive(Clone, Debug)]
pub struct Art {
    pub w: usize,
    pub h: usize,
    pub rgb: Vec<u8>,
    /// Unique per decoded picture, so screens can cache derived images.
    pub id: u64,
}

impl Art {
    pub fn new(w: usize, h: usize, rgb: Vec<u8>) -> Art {
        debug_assert_eq!(rgb.len(), w * h * 3);
        Art { w, h, rgb, id: NEXT_ART_ID.fetch_add(1, Ordering::Relaxed) }
    }

    /// Nearest-neighbour "cover" sample: scale until the picture fills `dw` x `dh`, crop the
    /// overhang evenly on both sides. Returns packed RGB8 of exactly `dw * dh`.
    pub fn cover(&self, dw: usize, dh: usize) -> Vec<u8> {
        let mut out = vec![0u8; dw * dh * 3];
        if self.w == 0 || self.h == 0 || dw == 0 || dh == 0 {
            return out;
        }
        let scale = (dw as f32 / self.w as f32).max(dh as f32 / self.h as f32);
        let ox = (self.w as f32 - dw as f32 / scale) / 2.0;
        let oy = (self.h as f32 - dh as f32 / scale) / 2.0;
        for dy in 0..dh {
            let sy = ((oy + (dy as f32 + 0.5) / scale) as usize).min(self.h - 1);
            for dx in 0..dw {
                let sx = ((ox + (dx as f32 + 0.5) / scale) as usize).min(self.w - 1);
                let s = (sy * self.w + sx) * 3;
                let d = (dy * dw + dx) * 3;
                out[d..d + 3].copy_from_slice(&self.rgb[s..s + 3]);
            }
        }
        out
    }
}

/// FNV-1a 64 over the path text, as 16 hex digits.
pub fn path_hash(path: &Path) -> String {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in path.to_string_lossy().bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("{h:016x}")
}

/// `<code>-<hash of path>.png`; the code is reduced to file-name-safe characters.
pub fn capture_file_name(code: &str, path: &Path) -> String {
    let safe: String = code.chars().map(|c| if c.is_ascii_alphanumeric() { c } else { '_' }).collect();
    let safe = if safe.is_empty() { "XXXX".to_string() } else { safe };
    format!("{safe}-{}.png", path_hash(path))
}

pub fn captures_dir(cache_dir: &Path) -> PathBuf {
    cache_dir.join("captures")
}

/// Decode a capture PNG (8-bit RGB or RGBA).
pub fn read_png(path: &Path) -> Option<Art> {
    let file = std::fs::File::open(path).ok()?;
    let mut reader = png::Decoder::new(std::io::BufReader::new(file)).read_info().ok()?;
    let mut buf = vec![0u8; reader.output_buffer_size()];
    let info = reader.next_frame(&mut buf).ok()?;
    if info.bit_depth != png::BitDepth::Eight {
        return None;
    }
    let (w, h) = (info.width as usize, info.height as usize);
    let px = &buf[..info.buffer_size()];
    let rgb: Vec<u8> = match info.color_type {
        png::ColorType::Rgb => px.to_vec(),
        png::ColorType::Rgba => px.chunks_exact(4).flat_map(|p| [p[0], p[1], p[2]]).collect(),
        png::ColorType::Grayscale => px.iter().flat_map(|&g| [g, g, g]).collect(),
        _ => return None,
    };
    (rgb.len() == w * h * 3 && w > 0 && h > 0).then(|| Art::new(w, h, rgb))
}

#[derive(Debug)]
pub enum ArtJob {
    /// Look the art up (slot, then cached capture); queue a capture if there is none.
    Resolve { path: PathBuf, code: String },
    /// A save state may have appeared: look at the slots only.
    Refresh { path: PathBuf },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ArtSource {
    Slot,
    Capture,
}

pub struct ArtMsg {
    pub path: PathBuf,
    pub art: Art,
    pub source: ArtSource,
}

/// Start the capture thread. `captures` None = never run the emulator for art (tests).
pub fn spawn(captures: Option<PathBuf>, platform: Arc<dyn Platform>) -> (Sender<ArtJob>, Receiver<ArtMsg>) {
    let (job_tx, job_rx) = channel::<ArtJob>();
    let (msg_tx, msg_rx) = channel::<ArtMsg>();
    let spawned = std::thread::Builder::new().name("library-art".into()).spawn(move || worker(job_rx, msg_tx, captures, &*platform));
    if let Err(e) = spawned {
        eprintln!("warning: could not start the label-art thread: {e}");
    }
    (job_tx, msg_rx)
}

fn worker(rx: Receiver<ArtJob>, tx: Sender<ArtMsg>, captures: Option<PathBuf>, platform: &dyn Platform) {
    // games with neither a slot nor a cached capture, waiting for their turn
    let mut heavy: VecDeque<(PathBuf, String)> = VecDeque::new();
    loop {
        let job = if heavy.is_empty() {
            match rx.recv() {
                Ok(j) => Some(j),
                Err(_) => return,
            }
        } else {
            match rx.try_recv() {
                Ok(j) => Some(j),
                Err(TryRecvError::Empty) => None,
                Err(TryRecvError::Disconnected) => return,
            }
        };
        let send = |path: PathBuf, art: Art, source: ArtSource| tx.send(ArtMsg { path, art, source }).is_ok();
        match job {
            Some(ArtJob::Resolve { path, code }) => {
                if let Some(art) = platform.slot_art(&path) {
                    if !send(path, art, ArtSource::Slot) {
                        return;
                    }
                    continue;
                }
                let Some(dir) = &captures else { continue };
                let file = dir.join(capture_file_name(&code, &path));
                if let Some(art) = read_png(&file) {
                    if !send(path, art, ArtSource::Capture) {
                        return;
                    }
                } else {
                    heavy.push_back((path, code));
                }
            }
            Some(ArtJob::Refresh { path }) => {
                if let Some(art) = platform.slot_art(&path) {
                    if !send(path, art, ArtSource::Slot) {
                        return;
                    }
                }
            }
            None => {
                let Some((path, code)) = heavy.pop_front() else { continue };
                let Some(dir) = &captures else { continue };
                let file = dir.join(capture_file_name(&code, &path));
                // a core panic on some odd ROM must not end the thread
                let run = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| platform.capture_title(&path, &file)));
                match run {
                    Ok(Ok(art)) => {
                        if !send(path, art, ArtSource::Capture) {
                            return;
                        }
                    }
                    Ok(Err(e)) => eprintln!("warning: title capture for {}: {e:#}", path.display()),
                    Err(_) => eprintln!("warning: title capture for {} panicked", path.display()),
                }
                // be a polite background citizen: std has no thread priority, so yield a little
                std::thread::sleep(std::time::Duration::from_millis(40));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_name_is_code_dash_hash() {
        let p = Path::new("C:/Games/Kingdom Hearts - Chain of Memories (USA).zip");
        let n = capture_file_name("B8CE", p);
        assert!(n.starts_with("B8CE-") && n.ends_with(".png"), "{n}");
        assert_eq!(n.len(), "B8CE-".len() + 16 + ".png".len());
        assert_eq!(n, capture_file_name("B8CE", p), "stable");
        assert_ne!(n, capture_file_name("B8CE", Path::new("D:/other/Kingdom Hearts.zip")), "path-specific");
        assert_eq!(capture_file_name("A/B?", p).split('-').next().unwrap(), "A_B_");
        assert!(capture_file_name("", p).starts_with("XXXX-"));
    }

    #[test]
    fn fnv1a_known_vectors() {
        assert_eq!(path_hash(Path::new("")), "cbf29ce484222325");
        assert_eq!(path_hash(Path::new("a")), "af63dc4c8601ec8c");
    }

    #[test]
    fn cover_crops_instead_of_squashing() {
        // left half red, right half blue
        let mut rgb = Vec::new();
        for _y in 0..160 {
            for x in 0..240 {
                rgb.extend_from_slice(if x < 120 { &[255, 0, 0] } else { &[0, 0, 255] });
            }
        }
        let art = Art::new(240, 160, rgb);
        // a tall strip keeps the aspect: crop the middle, so both colours meet in the centre
        let strip = art.cover(52, 78);
        assert_eq!(strip.len(), 52 * 78 * 3);
        assert_eq!(&strip[(10 * 52 + 2) * 3..][..3], &[255, 0, 0]);
        assert_eq!(&strip[(10 * 52 + 49) * 3..][..3], &[0, 0, 255]);
        // identity size is a copy
        assert_eq!(art.cover(240, 160), art.rgb);
        assert_ne!(art.id, Art::new(1, 1, vec![0; 3]).id);
    }
}
