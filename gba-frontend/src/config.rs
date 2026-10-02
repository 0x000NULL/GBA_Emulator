//! Command line and fixed tuning constants.

use clap::Parser;
use std::path::PathBuf;
use std::time::Duration;

/// The product name: window title, settings header, docs.
pub const APP_NAME: &str = "Strawberry";

/// GBA master clock.
pub const CPU_HZ: f64 = 16_777_216.0;
/// Cycles per frame (228 lines * 1232 cycles).
pub const CYCLES_PER_FRAME: f64 = 280_896.0;
/// Emulated frame rate (59.7275 Hz).
pub const GBA_FPS: f64 = CPU_HZ / CYCLES_PER_FRAME;

/// Screen dimensions.
pub const SCREEN_W: usize = gba_core::SCREEN_W;
pub const SCREEN_H: usize = gba_core::SCREEN_H;

/// One emulated frame of wall time.
pub fn frame_period() -> Duration {
    Duration::from_secs_f64(1.0 / GBA_FPS)
}

/// Audio ring buffer capacity, in seconds of device-rate output.
pub const AUDIO_RING_SECS: f64 = 0.25;
/// Audio-clocked pacing keeps the ring near this fill level (seconds of device-rate output).
pub const AUDIO_TARGET_SECS: f64 = 0.05;
/// Maximum relative speed adjustment applied by audio-clocked pacing (+-2%).
pub const AUDIO_PACE_MAX_ADJUST: f64 = 0.02;

/// Title bar refresh interval.
pub const TITLE_REFRESH: Duration = Duration::from_millis(500);
/// Minimum interval between window presents while fast-forwarding.
pub const FAST_FORWARD_PRESENT: Duration = Duration::from_millis(16);

#[derive(Parser, Debug, Clone)]
#[command(name = "strawberry", about = "Strawberry, a Game Boy Advance emulator", version)]
pub struct Cli {
    /// ROM file: a .gba, or a .zip containing one. Without it the library opens.
    pub rom: Option<PathBuf>,
    /// Fixed picture scale (240x160 * N) and a window that fits it exactly; default: the
    /// 1280x800 design size with the largest whole-number picture that fits.
    #[arg(long, value_parser = clap::value_parser!(u32).range(1..=10))]
    pub scale: Option<u32>,
    /// Extra library folder (repeatable). The folder of a ROM given on the command line is
    /// always included.
    #[arg(long, value_name = "DIR")]
    pub library: Vec<PathBuf>,
    /// 16 KiB BIOS image for low-level emulation (overrides GBA_BIOS).
    #[arg(long, conflicts_with = "hle")]
    pub bios: Option<PathBuf>,
    /// Force the high-level BIOS even when GBA_BIOS is set.
    #[arg(long)]
    pub hle: bool,
    /// Do not open an audio device; pacing uses the wall clock only.
    #[arg(long)]
    pub no_audio: bool,
    /// Open the audio device but output silence (pacing still follows the audio clock).
    #[arg(long)]
    pub mute: bool,
    /// Run N frames headless, print fps and the framebuffer hash, then exit.
    #[arg(long, value_name = "N")]
    pub bench: Option<u32>,
    /// Disable frame pacing: run as fast as the host allows.
    #[arg(long)]
    pub fullspeed: bool,
    /// Quit after N emulated frames (automated smoke tests).
    #[arg(long, hide = true, value_name = "N")]
    pub exit_after: Option<u64>,
}
