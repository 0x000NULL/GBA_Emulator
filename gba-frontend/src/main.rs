//! Strawberry, the desktop player for gba-core: minifb window, cpal audio, gilrs gamepads, and the GUI
//! described in `docs/gui/` (library, pause overlay, save states, settings, bench).
//!
//! Layout: `app` (screens and the game session), `ui` (window-resolution rendering), `window`
//! (minifb, input snapshot, letterbox fast path), `input` (GBA key masks, gamepads), `audio`
//! (ring buffer, resampler, cpal stream), `pacing` (frame clock), `saves` (`<rom>.sav`),
//! `debug_ui` (hotkeys, IO dump, screenshots, title bar), `config` (CLI + constants).
//!
//! Default keys: Z=A X=B A=L S=R Enter=Start RShift=Select arrows=D-pad; Tab (held) fast-forward;
//! Esc/P pause menu; N frame-step; F5 write save; F1/F3 save/load state; F10 dump IO;
//! F11 screenshot; F12 bench; F2 save states. Gamepad defaults: South=A East=B LB=L RB=R Start
//! Select D-pad (the left stick is always a D-pad too). Keys and pad buttons are rebindable in
//! Settings.

mod app;
mod audio;
mod config;
mod debug_ui;
mod icon;
mod input;
mod library;
mod overlay;
mod pacing;
mod pinned;
mod pixels;
mod render_tests;
mod saves;
mod settings;
mod screens;
mod slots;
mod ui;
mod window;

use anyhow::Result;
use clap::Parser;
use config::Cli;
use gba_core::{BiosMode, Gba};
use std::time::Instant;

fn main() -> Result<()> {
    let cli = Cli::parse();
    if let Some(n) = cli.bench {
        let Some(rom) = &cli.rom else { anyhow::bail!("--bench needs a ROM") };
        let bios = match (&cli.bios, cli.hle) {
            (Some(p), _) => Some(BiosMode::Lle(std::fs::read(p).map_err(|e| anyhow::anyhow!("reading BIOS {}: {e}", p.display()))?)),
            (None, true) => Some(BiosMode::Hle),
            (None, false) => None,
        };
        let mut gba = gba_test::build_gba(rom, bios, None)?;
        return bench(&mut gba, n);
    }
    run(&cli)
}

/// Headless speed measurement; the output format is consumed by performance tracking.
fn bench(gba: &mut Gba, frames: u32) -> Result<()> {
    let t0 = Instant::now();
    for _ in 0..frames {
        gba.run_frame();
        gba.bus.apu.samples.clear();
    }
    let dt = t0.elapsed().as_secs_f64();
    println!("{frames} frames in {dt:.2}s = {:.1} fps (hash {:016x})", frames as f64 / dt, gba_test::fb_hash(&gba.framebuffer()[..]));
    Ok(())
}

fn run(cli: &Cli) -> Result<()> {
    let mut app = app::App::new(cli)?;
    let (w, h) = app.initial_window_size();
    let title = match &app.session {
        Some(s) => format!("{} [{}]", s.game, s.code),
        None => config::APP_NAME.to_string(),
    };
    if app.session.is_none() {
        println!("library: {} folder(s)", app.library.folders().len());
    }
    let mut video = window::Video::new(&title, w, h)?;
    video.set_brand_icon(app::App::cache_dir().as_deref());
    println!("window: {w}x{h}");
    let mut pads = input::Gamepads::new();

    while video.is_open() && !app.quit {
        let input = video.gather_input(&app.keymap, pads.poll(&app.padmap), Instant::now());
        app.update(&input, &video)?;
        if app.quit {
            break;
        }
        if app.settings_dirty {
            app.apply_settings(&mut video);
        }
        app.render(&mut video)?;
        app.pace();
    }
    let size = video.size();
    app.shutdown(size);
    Ok(())
}
