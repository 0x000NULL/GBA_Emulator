//! Desktop frontend. Owned by the frontend agent.
//!
//! M0 status: PLACEHOLDER. Loads a ROM, runs frames headless and reports speed, so the crate and
//! its dependencies (minifb, cpal, gilrs) build. The agent replaces this with the real window,
//! audio output, input mapping, frame pacing and debug views.

use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;
use std::time::Instant;

#[derive(Parser, Debug)]
#[command(name = "gba", about = "Game Boy Advance emulator")]
struct Cli {
    rom: PathBuf,
    /// Run this many frames headless and print the speed instead of opening a window.
    #[arg(long)]
    bench: Option<u32>,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let mut gba = gba_test::build_gba(&cli.rom, None, None)?;
    let frames = cli.bench.unwrap_or(600);
    let t0 = Instant::now();
    for _ in 0..frames {
        gba.run_frame();
        gba.bus.apu.samples.clear();
    }
    let dt = t0.elapsed().as_secs_f64();
    println!("{frames} frames in {dt:.2}s = {:.1} fps (hash {:016x})", frames as f64 / dt, gba_test::fb_hash(&gba.bus.ppu.fb[..]));
    Ok(())
}
