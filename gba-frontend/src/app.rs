//! Application state machine: which screen is up, the running game session, and the per-frame
//! update/render split that `main.rs` drives.
//!
//! Screens (`docs/gui/SCREENS.md`): Library (01), Game with its pixel pause overlay (02), Save
//! states (03), Settings (04), Bench (D). Only the Game screen (unpaused) and the Bench (running)
//! advance the emulator.

use crate::audio::AudioOut;
use crate::config::{self, Cli, AUDIO_PACE_MAX_ADJUST, CPU_HZ, FAST_FORWARD_PRESENT, SAVE_INTERVAL_FRAMES, TITLE_REFRESH};
use crate::debug_ui::{self, Hotkey, HotkeyMap, TitleInfo};
use crate::pacing::{paced_period, FpsCounter, Pacer};
use crate::saves::SaveManager;
use crate::ui::input::{KeyMap, UiInput};
use crate::ui::{self, Canvas, Fonts, NIGHT};
use crate::window::Video;
use anyhow::Result;
use gba_core::{BiosMode, Gba, Keys};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Back {
    Library,
    Game,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Screen {
    Library,
    Game,
    SaveStates { back: Back },
    Settings { back: Back },
    Bench,
}

/// A loaded game and everything that runs it.
pub struct Session {
    pub gba: Gba,
    pub rom: PathBuf,
    pub game: String,
    pub code: String,
    pub saves: SaveManager,
    pub audio: Option<AudioOut>,
    pub pacer: Pacer,
    pub fps: FpsCounter,
    pub paused: bool,
    pub fast: bool,
    was_fast: bool,
    /// Frames emulated in this session (also the autosave clock).
    pub frames: u64,
    samples: Vec<[i16; 2]>,
    last_present: Instant,
    last_title: Instant,
    pub started: Instant,
}

impl Session {
    pub fn new(rom: &Path, bios: Option<BiosMode>, no_audio: bool, mute: bool) -> Result<Self> {
        let gba = gba_test::build_gba(rom, bios, None)?;
        let header = gba.bus.cart.header.clone();
        let game = if header.title.is_empty() {
            rom.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_else(|| "GBA".into())
        } else {
            header.title.clone()
        };
        let code = header.game_code_str();
        println!("rom: {} [{code}] v{} save {:?}", game, header.version, gba.bus.cart.save_type);
        let mut gba = gba;
        let saves = SaveManager::new(gba_test::save_path_for(rom));
        match saves.load(&mut gba) {
            Ok(true) => println!("loaded save {}", saves.path().display()),
            Ok(false) => {}
            Err(e) => eprintln!("warning: {e:#}"),
        }
        let audio = if no_audio {
            None
        } else {
            match AudioOut::open(src_rate(&gba), mute) {
                Ok(a) => Some(a),
                Err(e) => {
                    eprintln!("warning: audio unavailable ({e:#}); running without sound");
                    None
                }
            }
        };
        let now = Instant::now();
        Ok(Session {
            gba,
            rom: rom.to_path_buf(),
            game,
            code,
            saves,
            audio,
            pacer: Pacer::new(),
            fps: FpsCounter::new(),
            paused: false,
            fast: false,
            was_fast: false,
            frames: 0,
            samples: Vec::with_capacity(4096),
            last_present: now,
            last_title: now,
            started: now,
        })
    }

    /// Run one emulated frame and feed audio/saves; `fast` drops audio.
    pub fn step(&mut self) {
        self.gba.run_frame();
        self.frames += 1;
        self.gba.drain_audio(&mut self.samples);
        if let Some(a) = &mut self.audio {
            if !self.fast {
                a.set_src_rate(src_rate(&self.gba));
                a.push(&self.samples);
            }
        }
        self.samples.clear();
        debug_ui::drain_debug_log(&mut self.gba);
        self.saves.poll(&mut self.gba);
        if self.frames.is_multiple_of(SAVE_INTERVAL_FRAMES) {
            self.flush_save();
        }
        self.fps.tick();
    }

    pub fn flush_save(&mut self) {
        match self.saves.flush(&mut self.gba) {
            Ok(true) => println!("save written: {}", self.saves.path().display()),
            Ok(false) => {}
            Err(e) => eprintln!("save failed: {e:#}"),
        }
    }

    /// Audio queue reset after a state load or a fast-forward edge.
    pub fn resync(&mut self) {
        if let Some(a) = &mut self.audio {
            a.flush();
        }
        self.pacer.reset();
        self.fps.reset();
    }

    pub fn set_fast(&mut self, fast: bool) {
        self.fast = fast && !self.paused;
        if self.fast != self.was_fast {
            self.resync();
            self.was_fast = self.fast;
        }
    }

    pub fn set_paused(&mut self, paused: bool) {
        if self.paused != paused {
            self.paused = paused;
            if !paused {
                self.pacer.reset();
                self.fps.reset();
            }
        }
    }

    pub fn title(&self) -> String {
        debug_ui::format_title(&TitleInfo {
            game: &self.game,
            code: &self.code,
            fps: self.fps.fps,
            paused: self.paused,
            fast_forward: self.fast,
            audio: self.audio.as_ref().map(|a| (a.device_rate(), a.underruns())),
        })
    }
}

/// The core's current output sample rate in Hz (SOUNDBIAS-dependent).
pub fn src_rate(gba: &Gba) -> f64 {
    CPU_HZ / gba.bus.apu.cycles_per_sample() as f64
}

/// Run-time options that came from the command line.
#[derive(Clone, Debug)]
pub struct RunOptions {
    pub bios: Option<BiosMode>,
    pub no_audio: bool,
    pub mute: bool,
    pub fullspeed: bool,
    pub exit_after: Option<u64>,
    /// `--scale N`: a fixed picture scale and a window that fits it exactly.
    pub scale: Option<u32>,
}

pub struct App {
    pub screen: Screen,
    pub session: Option<Session>,
    pub keymap: KeyMap,
    pub hotkeys: HotkeyMap,
    pub opts: RunOptions,
    pub canvas: Canvas,
    pub quit: bool,
    /// Frame step requested while paused (N).
    step: bool,
}

impl App {
    pub fn new(cli: &Cli) -> Result<Self> {
        let bios = match (&cli.bios, cli.hle) {
            (Some(p), _) => Some(BiosMode::Lle(std::fs::read(p).map_err(|e| anyhow::anyhow!("reading BIOS {}: {e}", p.display()))?)),
            (None, true) => Some(BiosMode::Hle),
            (None, false) => None,
        };
        let opts = RunOptions { bios, no_audio: cli.no_audio, mute: cli.mute, fullspeed: cli.fullspeed, exit_after: cli.exit_after, scale: cli.scale };
        let mut app = App {
            screen: Screen::Library,
            session: None,
            keymap: KeyMap::default(),
            hotkeys: HotkeyMap::default(),
            opts,
            canvas: Canvas::new(ui::tokens::layout::DESIGN_W, ui::tokens::layout::DESIGN_H, Fonts::new()),
            quit: false,
            step: false,
        };
        if let Some(rom) = &cli.rom {
            app.start_game(rom)?;
        }
        app
            .session
            .as_ref()
            .map(|_| ())
            .ok_or_else(|| anyhow::anyhow!("no ROM given (the library screen arrives in the next milestone)"))?;
        Ok(app)
    }

    /// Boot `rom` and switch to the Game screen.
    pub fn start_game(&mut self, rom: &Path) -> Result<()> {
        let s = Session::new(rom, self.opts.bios.clone(), self.opts.no_audio, self.opts.mute)?;
        self.session = Some(s);
        self.screen = Screen::Game;
        Ok(())
    }

    /// Initial window size: `--scale N` means exactly 240N x 160N, else the design size.
    pub fn initial_window_size(&self) -> (usize, usize) {
        match self.opts.scale {
            Some(s) => (config::SCREEN_W * s as usize, config::SCREEN_H * s as usize),
            None => (ui::tokens::layout::DESIGN_W as usize, ui::tokens::layout::DESIGN_H as usize),
        }
    }

    /// Picture scale for the game on a `w` x `h` window.
    pub fn picture_scale(&self, w: u32, h: u32) -> u32 {
        match self.opts.scale {
            Some(s) => s.min(ui::fit_scale(w, h)).max(1),
            None => ui::fit_scale(w, h),
        }
    }

    /// One iteration: react to input, advance the emulator if it is running.
    pub fn update(&mut self, input: &UiInput, video: &Video) -> Result<()> {
        self.step = false;
        match self.screen {
            Screen::Game => self.update_game(input, video)?,
            Screen::Library | Screen::SaveStates { .. } | Screen::Settings { .. } | Screen::Bench => {
                // later milestones
            }
        }
        Ok(())
    }

    fn update_game(&mut self, input: &UiInput, video: &Video) -> Result<()> {
        let hotkeys: Vec<Hotkey> = self.hotkeys.pressed(input);
        let Some(s) = self.session.as_mut() else { return Ok(()) };
        for hk in hotkeys {
            match hk {
                Hotkey::Pause => {
                    let p = !s.paused;
                    s.set_paused(p);
                    if p {
                        println!("paused (N = frame step, P/Esc = resume)");
                    }
                }
                Hotkey::FrameStep => self.step = s.paused,
                Hotkey::DumpIo => debug_ui::dump_io(&mut s.gba),
                Hotkey::Screenshot => match debug_ui::screenshot(&s.gba, &s.rom) {
                    Ok(p) => println!("screenshot: {}", p.display()),
                    Err(e) => eprintln!("screenshot failed: {e:#}"),
                },
                Hotkey::SaveNow => match s.saves.force_write(&mut s.gba) {
                    Ok(true) => println!("save written: {}", s.saves.path().display()),
                    Ok(false) => println!("this cartridge has no save memory"),
                    Err(e) => eprintln!("save failed: {e:#}"),
                },
                Hotkey::SaveState => match debug_ui::save_state(&s.gba, &s.rom) {
                    Ok(p) => println!("state saved: {}", p.display()),
                    Err(e) => eprintln!("state save failed: {e:#}"),
                },
                Hotkey::LoadState => match debug_ui::load_state(&mut s.gba, &s.rom) {
                    Ok(p) => {
                        println!("state loaded: {}", p.display());
                        s.resync();
                    }
                    Err(e) => eprintln!("state load failed: {e:#}"),
                },
                Hotkey::Bench | Hotkey::SaveStates => {}
            }
        }
        s.set_fast(video.is_key_down(self.hotkeys.fast_forward));
        s.gba.set_keys(Keys(input.game_mask));
        if !s.paused || self.step {
            s.step();
            if self.opts.exit_after.is_some_and(|n| s.frames >= n) {
                self.quit = true;
            }
        }
        Ok(())
    }

    /// Draw the current screen and present it.
    pub fn render(&mut self, video: &mut Video) -> Result<()> {
        let (w, h) = video.size();
        match self.screen {
            Screen::Game => {
                let scale = self.picture_scale(w as u32, h as u32);
                let Some(s) = self.session.as_mut() else { return Ok(()) };
                if !s.fast || s.last_present.elapsed() >= FAST_FORWARD_PRESENT {
                    video.present_game(&s.gba.framebuffer()[..], scale, NIGHT.bg)?;
                    s.last_present = Instant::now();
                } else {
                    video.poll();
                }
                if s.last_title.elapsed() >= TITLE_REFRESH {
                    video.set_title(&s.title());
                    s.last_title = Instant::now();
                }
            }
            _ => {
                self.canvas.resize(w as u32, h as u32);
                self.canvas.clear(ui::PAPER.bg);
                video.present_canvas(&self.canvas)?;
            }
        }
        Ok(())
    }

    /// Sleep until the next frame is due.
    pub fn pace(&mut self) {
        let base_period = config::frame_period();
        match (self.screen, self.session.as_mut()) {
            (Screen::Game, Some(s)) if !s.paused => {
                if s.fast || self.opts.fullspeed {
                    return;
                }
                let period = match &s.audio {
                    Some(a) => paced_period(base_period, a.fill(), a.target_fill(), AUDIO_PACE_MAX_ADJUST),
                    None => base_period,
                };
                s.pacer.wait(period);
            }
            (_, s) => {
                if let Some(s) = s {
                    s.pacer.reset();
                }
                std::thread::sleep(Duration::from_millis(8));
            }
        }
    }

    /// Final save flush and the exit statistics line.
    pub fn shutdown(&mut self) {
        if let Some(s) = self.session.as_mut() {
            s.flush_save();
            let audio_stats = match &s.audio {
                Some(a) => format!("audio underruns {}, dropped frames {}", a.underruns(), a.dropped()),
                None => "no audio".to_string(),
            };
            let wall = s.started.elapsed().as_secs_f64();
            println!(
                "exit after {} frames in {wall:.2}s (avg {:.2} fps); late frames {}; sleep margin {:?}; {audio_stats}",
                s.frames,
                s.frames as f64 / wall.max(1e-9),
                s.pacer.late_frames,
                s.pacer.margin()
            );
        }
    }
}
