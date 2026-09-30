//! Desktop frontend for gba-core: minifb window, cpal audio, gilrs gamepads.
//!
//! Layout: `config` (CLI + constants), `window` (LUT + upscale + minifb), `input` (key masks),
//! `audio` (ring buffer, resampler, cpal stream), `pacing` (frame clock), `saves` (`<rom>.sav`),
//! `debug_ui` (hotkeys, IO dump, screenshots, title bar).
//!
//! Controls: Z=A X=B A=L S=R Enter=Start RShift=Select arrows=D-pad; Tab (held) fast-forward;
//! Esc quit; P pause; N frame-step; F5 write save; F10 dump IO; F11 screenshot.

mod audio;
mod config;
mod debug_ui;
mod input;
mod pacing;
mod saves;
mod window;

use anyhow::{Context, Result};
use clap::Parser;
use config::{Cli, AUDIO_PACE_MAX_ADJUST, CPU_HZ, FAST_FORWARD_PRESENT, SAVE_INTERVAL_FRAMES, TITLE_REFRESH};
use debug_ui::{Hotkey, TitleInfo};
use gba_core::{BiosMode, Gba, Keys};
use pacing::{paced_period, FpsCounter, Pacer};
use std::time::{Duration, Instant};

fn main() -> Result<()> {
    let cli = Cli::parse();
    let bios = match (&cli.bios, cli.hle) {
        (Some(p), _) => Some(BiosMode::Lle(std::fs::read(p).with_context(|| format!("reading BIOS {}", p.display()))?)),
        (None, true) => Some(BiosMode::Hle),
        (None, false) => None,
    };
    let mut gba = gba_test::build_gba(&cli.rom, bios, None)?;
    if let Some(n) = cli.bench {
        return bench(&mut gba, n);
    }
    run(&cli, gba)
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

/// The core's current output sample rate in Hz (SOUNDBIAS-dependent).
fn src_rate(gba: &Gba) -> f64 {
    CPU_HZ / gba.bus.apu.cycles_per_sample() as f64
}

fn run(cli: &Cli, mut gba: Gba) -> Result<()> {
    let header = gba.bus.cart.header.clone();
    let game = if header.title.is_empty() {
        cli.rom.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_else(|| "GBA".into())
    } else {
        header.title.clone()
    };
    let code = header.game_code_str();
    println!("rom: {} [{code}] v{} save {:?}", game, header.version, gba.bus.cart.save_type);

    let mut saves = saves::SaveManager::new(gba_test::save_path_for(&cli.rom));
    match saves.load(&mut gba) {
        Ok(true) => println!("loaded save {}", saves.path().display()),
        Ok(false) => {}
        Err(e) => eprintln!("warning: {e:#}"),
    }

    let mut video = window::Video::new(&format!("{game} [{code}]"), cli.scale)?;
    let (w, h) = video.size();
    println!("window: {w}x{h} (scale {})", cli.scale);
    let mut pads = input::Gamepads::new();
    let mut audio = if cli.no_audio {
        None
    } else {
        match audio::AudioOut::open(src_rate(&gba), cli.mute) {
            Ok(a) => Some(a),
            Err(e) => {
                eprintln!("warning: audio unavailable ({e:#}); running without sound");
                None
            }
        }
    };

    let base_period = config::frame_period();
    let mut pacer = Pacer::new();
    let mut fps = FpsCounter::new();
    let mut samples: Vec<[i16; 2]> = Vec::with_capacity(4096);
    let mut paused = false;
    let mut was_fast = false;
    let mut frames: u64 = 0;
    let mut last_title = Instant::now();
    let mut last_present = Instant::now();
    let t_start = Instant::now();

    'main: while video.is_open() {
        // --- hotkeys -------------------------------------------------------------------------
        let mut step = false;
        for hk in debug_ui::poll_hotkeys(video.window()) {
            match hk {
                Hotkey::Quit => break 'main,
                Hotkey::TogglePause => {
                    paused = !paused;
                    if paused {
                        println!("paused (N = frame step, P = resume)");
                    } else {
                        pacer.reset();
                        fps.reset();
                    }
                }
                Hotkey::FrameStep => step = paused,
                Hotkey::DumpIo => debug_ui::dump_io(&mut gba),
                Hotkey::Screenshot => match debug_ui::screenshot(&gba, &cli.rom) {
                    Ok(p) => println!("screenshot: {}", p.display()),
                    Err(e) => eprintln!("screenshot failed: {e:#}"),
                },
                Hotkey::SaveNow => match saves.force_write(&mut gba) {
                    Ok(true) => println!("save written: {}", saves.path().display()),
                    Ok(false) => println!("this cartridge has no save memory"),
                    Err(e) => eprintln!("save failed: {e:#}"),
                },
            }
        }
        let fast = !paused && debug_ui::fast_forward_held(video.window());
        if fast != was_fast {
            // Entering: drop what is queued so sound stops instantly. Leaving: re-prime quietly.
            if let Some(a) = &mut audio {
                a.flush();
            }
            pacer.reset();
            fps.reset();
            was_fast = fast;
        }

        // --- input ---------------------------------------------------------------------------
        let keys = input::sanitize(input::keyboard_keys(|k| video.window().is_key_down(k)) | pads.poll());
        gba.set_keys(Keys(keys));

        // --- emulate one frame ---------------------------------------------------------------
        if !paused || step {
            gba.run_frame();
            frames += 1;
            gba.drain_audio(&mut samples);
            if let Some(a) = &mut audio {
                if !fast {
                    a.set_src_rate(src_rate(&gba));
                    a.push(&samples);
                }
            }
            samples.clear();
            debug_ui::drain_debug_log(&mut gba);
            saves.poll(&mut gba);
            if frames.is_multiple_of(SAVE_INTERVAL_FRAMES) {
                match saves.flush(&mut gba) {
                    Ok(true) => println!("save written: {}", saves.path().display()),
                    Ok(false) => {}
                    Err(e) => eprintln!("save failed: {e:#}"),
                }
            }
            fps.tick();
            if cli.exit_after.is_some_and(|n| frames >= n) {
                break 'main;
            }
        }

        // --- present -------------------------------------------------------------------------
        if !fast || last_present.elapsed() >= FAST_FORWARD_PRESENT {
            video.present(&gba.framebuffer()[..])?;
            last_present = Instant::now();
        } else {
            video.poll();
        }
        if last_title.elapsed() >= TITLE_REFRESH {
            let title = debug_ui::format_title(&TitleInfo {
                game: &game,
                code: &code,
                fps: fps.fps,
                paused,
                fast_forward: fast,
                audio: audio.as_ref().map(|a| (a.device_rate(), a.underruns())),
            });
            video.set_title(&title);
            last_title = Instant::now();
        }

        // --- pace ----------------------------------------------------------------------------
        if paused {
            std::thread::sleep(Duration::from_millis(8));
            pacer.reset();
        } else if fast || cli.fullspeed {
            // unpaced
        } else {
            let period = match &audio {
                Some(a) => paced_period(base_period, a.fill(), a.target_fill(), AUDIO_PACE_MAX_ADJUST),
                None => base_period,
            };
            pacer.wait(period);
        }
    }

    // --- exit ----------------------------------------------------------------------------------
    match saves.flush(&mut gba) {
        Ok(true) => println!("save written: {}", saves.path().display()),
        Ok(false) => {}
        Err(e) => eprintln!("save failed: {e:#}"),
    }
    let audio_stats = match &audio {
        Some(a) => format!("audio underruns {}, dropped frames {}", a.underruns(), a.dropped()),
        None => "no audio".to_string(),
    };
    let wall = t_start.elapsed().as_secs_f64();
    println!(
        "exit after {frames} frames in {wall:.2}s (avg {:.2} fps); late frames {}; sleep margin {:?}; {audio_stats}",
        frames as f64 / wall.max(1e-9),
        pacer.late_frames,
        pacer.margin()
    );
    Ok(())
}
