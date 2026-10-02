//! Application state machine: which screen is up, the running game session, and the per-frame
//! update/render split that `main.rs` drives.
//!
//! Screens (`docs/gui/SCREENS.md`): Library (01), Game with its pixel pause overlay (02), Save
//! states (03), Settings (04), Bench (D). Only the Game screen (unpaused) and the Bench (running)
//! advance the emulator.

use crate::audio::AudioOut;
use crate::config::{self, Cli, AUDIO_PACE_MAX_ADJUST, CPU_HZ, FAST_FORWARD_PRESENT, TITLE_REFRESH};
use crate::debug_ui::{self, Hotkey, HotkeyMap, TitleInfo};
use crate::overlay::pause::{Outcome, PauseMenu};
use crate::overlay::Overlay;
use crate::pacing::{paced_period, FpsCounter, Pacer};
use crate::saves::SaveManager;
use crate::library::Library;
use crate::screens::bench::{BenchCtx, BenchOutcome, BenchScreen};
use crate::screens::library::{LibraryOutcome, LibraryScreen};
use crate::screens::settings::{SettingsCtx, SettingsOutcome, SettingsScreen};
use crate::screens::states::{StatesCtx, StatesOutcome, StatesScreen};
use crate::settings::{AudioMode, Bios, PictureSize, Settings};
use crate::window::Filter;
use crate::slots::{self, SlotSet};
use crate::ui::fmt::frames_to_secs;
use crate::ui::input::{KeyMap, PadMap, UiInput};
use crate::ui::tokens::pixel::DIM_EVY;
use crate::ui::{self, Canvas, Fonts, NIGHT};
use crate::window::Video;
use anyhow::Result;
use gba_core::{BiosMode, FrameBuffer, Gba, Keys};
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

pub const TOAST_TIME: Duration = Duration::from_millis(1500);

/// A loaded game and everything that runs it.
pub struct Session {
    pub gba: Gba,
    pub rom: PathBuf,
    pub game: String,
    pub code: String,
    pub saves: SaveManager,
    pub slots: SlotSet,
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
    pub menu: Option<PauseMenu>,
    pub toast: Option<(String, Instant)>,
    /// Scratch frame the overlay draws into (never the core's own framebuffer).
    scratch: Box<FrameBuffer>,
    /// Host time of the last `step()` and the wall time between the last two steps (Bench).
    pub host_ms: f64,
    pub paced_ms: f64,
    last_step: Option<Instant>,
    /// Frames between `.sav` writes (Settings › Saves).
    pub autosave_frames: u64,
}

impl Session {
    pub fn new(rom: &Path, bios: Option<BiosMode>, audio_mode: AudioMode, device: Option<&str>, volume: u8) -> Result<Self> {
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
        let slots = SlotSet::scan(rom, Some(gba.bus.cart.rom()));
        let audio = if audio_mode == AudioMode::Off {
            None
        } else {
            match AudioOut::open_on(src_rate(&gba), audio_mode == AudioMode::Muted, device) {
                Ok(a) => {
                    a.set_gain(crate::audio::volume_gain(volume));
                    Some(a)
                }
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
            slots,
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
            menu: None,
            toast: None,
            scratch: vec![0u16; gba_core::SCREEN_W * gba_core::SCREEN_H].into_boxed_slice().try_into().expect("frame"),
            host_ms: 0.0,
            paced_ms: 0.0,
            last_step: None,
            autosave_frames: 60,
        })
    }

    /// Run one emulated frame and feed audio/saves; `fast` drops audio.
    pub fn step(&mut self) {
        let t0 = Instant::now();
        if let Some(last) = self.last_step {
            self.paced_ms = t0.duration_since(last).as_secs_f64() * 1000.0;
        }
        self.last_step = Some(t0);
        self.gba.run_frame();
        self.host_ms = t0.elapsed().as_secs_f64() * 1000.0;
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
        if self.frames.is_multiple_of(self.autosave_frames.max(1)) {
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

    /// Play time as the design counts it: frames rendered by the machine.
    pub fn play_secs(&self) -> u64 {
        frames_to_secs(self.gba.bus.ppu.frame_count)
    }

    pub fn show_toast(&mut self, text: impl Into<String>) {
        let text = text.into();
        println!("{text}");
        self.toast = Some((text, Instant::now()));
    }

    /// Save the machine into `slot`; reports through the pause bar or a toast.
    pub fn save_slot(&mut self, slot: u8) -> Result<()> {
        let meta = slots::save(&self.gba, &self.rom, slot, self.play_secs())?;
        self.slots.set(meta);
        Ok(())
    }

    pub fn load_slot(&mut self, slot: u8) -> Result<(), slots::SlotError> {
        let meta = slots::load(&mut self.gba, &self.rom, slot)?;
        self.slots.set(meta);
        self.resync();
        Ok(())
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

    /// The frame to present: the live framebuffer, or a copy with the overlay drawn on it.
    fn frame_to_present(&mut self, now: Instant) -> &[u16] {
        if let Some((_, t)) = self.toast {
            if now.duration_since(t) >= TOAST_TIME {
                self.toast = None;
            }
        }
        if self.menu.is_none() && self.toast.is_none() {
            return &self.gba.framebuffer()[..];
        }
        self.scratch.copy_from_slice(&self.gba.framebuffer()[..]);
        let mut o = Overlay::new(&mut self.scratch);
        if let Some(menu) = &self.menu {
            o.dim(DIM_EVY);
            let infos = self.slots.infos();
            menu.draw(&mut o, &infos, now);
        } else if let Some((text, _)) = &self.toast {
            o.toast(text);
        }
        &self.scratch[..]
    }
}

/// The core's current output sample rate in Hz (SOUNDBIAS-dependent).
pub fn src_rate(gba: &Gba) -> f64 {
    CPU_HZ / gba.bus.apu.cycles_per_sample() as f64
}

/// Run-time options: the command line plus what the settings file resolved to.
#[derive(Clone, Debug)]
pub struct RunOptions {
    pub bios: Option<BiosMode>,
    pub fullspeed: bool,
    pub exit_after: Option<u64>,
    /// Fixed picture scale (`--scale N` or Settings › Picture size); None = largest that fits.
    pub scale: Option<u32>,
    /// `--scale N` was given: the window opens at exactly 240N x 160N.
    pub exact_window: bool,
}

pub struct App {
    pub screen: Screen,
    pub session: Option<Session>,
    pub keymap: KeyMap,
    pub padmap: PadMap,
    pub hotkeys: HotkeyMap,
    pub opts: RunOptions,
    pub canvas: Canvas,
    pub quit: bool,
    /// Current slot (1..=8): what in-game F1/F3 act on; the last slot used in any menu.
    pub slot_cursor: u8,
    pub bench: BenchScreen,
    pub library: Library,
    library_screen: LibraryScreen,
    /// The window title names the game; in the library it is set once.
    title_is_library: bool,
    pub settings: Settings,
    settings_path: Option<PathBuf>,
    settings_screen: SettingsScreen,
    audio_devices: Vec<String>,
    /// Settings changed: re-apply to the video/audio/session on the next loop iteration.
    pub settings_dirty: bool,
    /// Fast-forward speed limit as a multiple of real time (None = as fast as the host allows).
    pub ff_limit: Option<u32>,
    /// The save-states screen while it is open, and (without a session) the game it shows.
    states: Option<StatesScreen>,
    states_rom: Option<(PathBuf, String, SlotSet)>,
    /// Frame step requested while paused (N).
    step: bool,
}

/// The folder name under the user's config and cache dirs.
pub const APP_DIR: &str = "strawberry";
/// The folder name releases before the rename used (1.0.0 to 1.2.0).
const LEGACY_DIR: &str = "gba-advance";

/// Move `gba-advance/` to `strawberry/` under the config and cache dirs, once: only when the new
/// folder does not exist yet. Never deletes; a failure is reported and the defaults are used.
fn migrate_legacy_dirs() {
    for base in [dirs::config_dir(), dirs::cache_dir()].into_iter().flatten() {
        migrate_dir(&base.join(LEGACY_DIR), &base.join(APP_DIR));
    }
}

/// Rename `old` to `new` when `old` exists and `new` does not. Returns whether it moved.
fn migrate_dir(old: &Path, new: &Path) -> bool {
    if new.exists() || !old.is_dir() {
        return false;
    }
    match std::fs::rename(old, new) {
        Ok(()) => {
            println!("moved {} to {}", old.display(), new.display());
            true
        }
        Err(e) => {
            eprintln!("warning: could not move {} to {}: {e}", old.display(), new.display());
            false
        }
    }
}

impl App {
    pub fn new(cli: &Cli) -> Result<Self> {
        migrate_legacy_dirs();
        let settings_path = Settings::default_path();
        let mut settings = settings_path.as_deref().map(Settings::load).unwrap_or_default();
        settings.apply_cli(cli);
        let bios = match (&cli.bios, cli.hle, &settings.bios) {
            (Some(p), _, _) => Some(BiosMode::Lle(std::fs::read(p).map_err(|e| anyhow::anyhow!("reading BIOS {}: {e}", p.display()))?)),
            (None, true, _) => Some(BiosMode::Hle),
            (None, false, Bios::File(p)) if !p.as_os_str().is_empty() => match std::fs::read(p) {
                Ok(b) => Some(BiosMode::Lle(b)),
                Err(e) => {
                    eprintln!("warning: BIOS file {}: {e}; using the built-in one", p.display());
                    Some(BiosMode::Hle)
                }
            },
            (None, false, _) => None,
        };
        let scale = match (cli.scale, settings.picture) {
            (Some(n), _) => Some(n),
            (None, PictureSize::Fixed(n)) => Some(n),
            (None, PictureSize::Largest) => None,
        };
        let opts = RunOptions { bios, fullspeed: cli.fullspeed, exit_after: cli.exit_after, scale, exact_window: cli.scale.is_some() };
        let keymap = settings.to_keymap();
        let padmap = settings.to_padmap();
        let hotkeys = settings.to_hotkeys();
        let ff_limit = settings.ff_limit.factor();
        let folders = Self::library_folders(&settings, cli.rom.as_deref());
        let library = Library::new(folders, Self::cache_dir());
        let mut app = App {
            screen: Screen::Library,
            session: None,
            keymap,
            padmap,
            hotkeys,
            opts,
            canvas: Canvas::new(ui::tokens::layout::DESIGN_W, ui::tokens::layout::DESIGN_H, Fonts::new()),
            quit: false,
            slot_cursor: 1,
            bench: BenchScreen::new(),
            library,
            library_screen: LibraryScreen::new(),
            title_is_library: false,
            settings,
            settings_path,
            settings_screen: SettingsScreen::new(),
            audio_devices: AudioOut::list_devices(),
            settings_dirty: false,
            ff_limit,
            states: None,
            states_rom: None,
            step: false,
        };
        if let Some(rom) = &cli.rom {
            app.start_game(rom)?;
            app.library_screen.select(rom);
        }
        Ok(app)
    }

    /// `<cache dir>/strawberry`: the library index and title captures.
    pub fn cache_dir() -> Option<PathBuf> {
        dirs::cache_dir().map(|d| d.join(APP_DIR))
    }

    /// Settings folders, then the command-line ROM's folder, else `./Games` when it exists.
    fn library_folders(settings: &Settings, rom: Option<&Path>) -> Vec<PathBuf> {
        let mut folders: Vec<PathBuf> = settings.library_folders.clone();
        if let Some(parent) = rom.and_then(|r| r.parent()) {
            let p = if parent.as_os_str().is_empty() { PathBuf::from(".") } else { parent.to_path_buf() };
            if !folders.contains(&p) {
                folders.push(p);
            }
        }
        if folders.is_empty() {
            let games = PathBuf::from("Games");
            if games.is_dir() {
                folders.push(games);
            }
        }
        folders
    }

    fn update_library(&mut self, input: &UiInput) -> Result<()> {
        self.library.poll();
        match self.library_screen.update(input, &self.library) {
            LibraryOutcome::None => {}
            LibraryOutcome::Quit => self.quit = true,
            LibraryOutcome::Play(p) => {
                if let Err(e) = self.start_game(&p) {
                    eprintln!("cannot start {}: {e:#}", p.display());
                }
            }
            LibraryOutcome::ResumeLatest(p) => match self.start_game(&p) {
                Ok(()) => {
                    let latest = self.session.as_ref().and_then(|s| s.slots.latest().map(|m| m.slot));
                    if let (Some(n), Some(s)) = (latest, self.session.as_mut()) {
                        self.slot_cursor = n;
                        match s.load_slot(n) {
                            Ok(()) => println!("resumed slot {n}"),
                            Err(e) => eprintln!("state load failed: {e}"),
                        }
                    }
                }
                Err(e) => eprintln!("cannot start {}: {e:#}", p.display()),
            },
            LibraryOutcome::OpenStates(p) => self.open_states(Back::Library, Some(&p)),
        }
        Ok(())
    }

    /// Boot `rom` and switch to the Game screen.
    pub fn start_game(&mut self, rom: &Path) -> Result<()> {
        let mut s = Session::new(rom, self.opts.bios.clone(), self.settings.audio, self.settings.audio_device.as_deref(), self.settings.volume)?;
        s.autosave_frames = self.settings.autosave_frames;
        self.title_is_library = false;
        self.slot_cursor = s.slots.latest().map(|m| m.slot).unwrap_or(1);
        self.session = Some(s);
        self.screen = Screen::Game;
        Ok(())
    }

    /// Flush saves and drop the session (QUIT in the pause menu); back to the library.
    pub fn end_game(&mut self) {
        if let Some(mut s) = self.session.take() {
            s.flush_save();
            let played = frames_to_secs(s.frames);
            self.library.record_played(&s.rom, played);
            self.library_screen.select(&s.rom);
            self.library_screen.refresh_slots();
        }
        self.title_is_library = false;
        self.screen = Screen::Library;
    }

    /// Initial window size: `--scale N` means exactly 240N x 160N, else the design size.
    pub fn initial_window_size(&self) -> (usize, usize) {
        match (self.opts.exact_window, self.opts.scale, self.settings.window) {
            (true, Some(s), _) => (config::SCREEN_W * s as usize, config::SCREEN_H * s as usize),
            (_, _, Some((w, h))) if w >= 320 && h >= 240 => (w as usize, h as usize),
            _ => (ui::tokens::layout::DESIGN_W as usize, ui::tokens::layout::DESIGN_H as usize),
        }
    }

    /// Push the current settings into the video, audio and session; persist them.
    pub fn apply_settings(&mut self, video: &mut Video) {
        self.settings_dirty = false;
        self.opts.scale = match self.settings.picture {
            PictureSize::Fixed(n) => Some(n),
            PictureSize::Largest => None,
        };
        video.filter = match self.settings.filter {
            crate::settings::Filter::None => Filter::None,
            crate::settings::Filter::LcdGrid => Filter::LcdGrid,
            crate::settings::Filter::FrameBlend => Filter::FrameBlend,
        };
        self.ff_limit = self.settings.ff_limit.factor();
        if let Some(s) = self.session.as_mut() {
            s.autosave_frames = self.settings.autosave_frames;
            let gain = crate::audio::volume_gain(self.settings.volume);
            match self.settings.audio {
                AudioMode::Off => {
                    s.audio = None;
                }
                mode => {
                    let want_device = self.settings.audio_device.clone();
                    let reopen = s.audio.is_none() || s.audio.as_ref().map(|a| a.device_name() != want_device.as_deref().unwrap_or("")).unwrap_or(false) && want_device.is_some();
                    if reopen {
                        match AudioOut::open_on(src_rate(&s.gba), mode == AudioMode::Muted, want_device.as_deref()) {
                            Ok(a) => s.audio = Some(a),
                            Err(e) => eprintln!("warning: audio unavailable ({e:#})"),
                        }
                        s.resync();
                    }
                    if let Some(a) = s.audio.as_mut() {
                        a.set_muted(mode == AudioMode::Muted);
                        a.set_gain(gain);
                    }
                }
            }
        }
        self.keymap = self.settings.to_keymap();
        self.padmap = self.settings.to_padmap();
        self.hotkeys = self.settings.to_hotkeys();
        let rom = self.session.as_ref().map(|s| s.rom.clone());
        let folders = Self::library_folders(&self.settings, rom.as_deref());
        if folders != self.library.folders() {
            self.library = Library::new(folders, Self::cache_dir());
            self.library_screen = LibraryScreen::new();
        }
        self.save_settings();
    }

    fn save_settings(&self) {
        if let Some(p) = &self.settings_path {
            if let Err(e) = self.settings.save(p) {
                eprintln!("warning: settings not saved: {e:#}");
            }
        }
    }

    /// Open the settings manual from the game or the library.
    pub fn open_settings(&mut self, back: Back) {
        if let Some(s) = self.session.as_mut() {
            s.menu = None;
            s.set_paused(true);
        }
        self.settings_screen.reset();
        let dir = self.session.as_ref().and_then(|s| s.rom.parent().map(|p| p.to_path_buf()));
        self.settings_screen.set_game_dir(dir);
        self.screen = Screen::Settings { back };
    }

    fn update_settings(&mut self, input: &UiInput, back: Back) {
        let outcome = {
            let mut ctx = SettingsCtx {
                settings: &mut self.settings,
                keymap: &mut self.keymap,
                padmap: &mut self.padmap,
                hotkeys: &mut self.hotkeys,
                audio_devices: &self.audio_devices,
                rebinding: false,
            };
            self.settings_screen.update(input, &mut ctx)
        };
        match outcome {
            SettingsOutcome::None => {}
            SettingsOutcome::Changed => {
                self.settings.from_keymap(&self.keymap);
                self.settings.from_padmap(&self.padmap);
                self.settings.from_hotkeys(&self.hotkeys);
                self.settings_dirty = true;
            }
            SettingsOutcome::Back => {
                self.settings_screen.reset();
                self.go_back(back);
            }
            SettingsOutcome::OpenFolder(p) => {
                #[cfg(windows)]
                {
                    if let Err(e) = std::process::Command::new("explorer").arg(&p).spawn() {
                        eprintln!("open folder: {e}");
                    }
                }
                #[cfg(not(windows))]
                println!("folder: {}", p.display());
            }
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
            Screen::Library => self.update_library(input)?,
            Screen::SaveStates { back } => self.update_states(input, back)?,
            Screen::Settings { back } => self.update_settings(input, back),
            Screen::Bench => self.update_bench(input, video)?,
        }
        Ok(())
    }

    /// Open the save-states screen for the running game (or for `rom` from the library).
    pub fn open_states(&mut self, back: Back, rom: Option<&Path>) {
        if let Some(s) = self.session.as_mut() {
            s.menu = None;
            s.set_paused(true);
            self.states_rom = None;
        } else if let Some(rom) = rom {
            let name = rom.file_stem().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
            self.states_rom = Some((rom.to_path_buf(), name, SlotSet::scan(rom, None)));
        } else {
            return;
        }
        self.states = Some(StatesScreen::new(self.slot_cursor));
        self.screen = Screen::SaveStates { back };
    }

    fn update_states(&mut self, input: &UiInput, back: Back) -> Result<()> {
        let Some(mut screen) = self.states.take() else {
            self.go_back(back);
            return Ok(());
        };
        let outcome = {
            let (slots, game, running) = match (&self.session, &self.states_rom) {
                (Some(s), _) => (&s.slots, s.game.as_str(), true),
                (None, Some((_, name, slots))) => (slots, name.as_str(), false),
                (None, None) => {
                    self.go_back(back);
                    return Ok(());
                }
            };
            let ctx = StatesCtx { slots, game, running };
            let mut o = screen.update(input, &ctx);
            if o == StatesOutcome::None && !screen.dialog_open() && input.pressed(minifb::Key::Escape) {
                o = StatesOutcome::Back;
            }
            o
        };
        self.slot_cursor = screen.selected().clamp(1, 8);
        let mut keep = true;
        match outcome {
            StatesOutcome::None => {}
            StatesOutcome::Back => {
                keep = false;
                self.go_back(back);
            }
            StatesOutcome::Load(n) => {
                self.slot_cursor = n;
                if self.session.is_none() {
                    if let Some((rom, _, _)) = self.states_rom.take() {
                        self.start_game(&rom)?;
                    }
                }
                if let Some(s) = self.session.as_mut() {
                    match s.load_slot(n) {
                        Ok(()) => {
                            println!("state loaded from slot {n}");
                            s.set_paused(false);
                            keep = false;
                            self.screen = Screen::Game;
                        }
                        Err(e) => eprintln!("state load failed: {e}"),
                    }
                }
            }
            StatesOutcome::SaveOver(n) => {
                self.slot_cursor = n;
                if let Some(s) = self.session.as_mut() {
                    match s.save_slot(n) {
                        Ok(()) => println!("state saved to slot {n}"),
                        Err(e) => eprintln!("state save failed: {e:#}"),
                    }
                }
            }
            StatesOutcome::Delete(n) => {
                let rom = self.session.as_ref().map(|s| s.rom.clone()).or_else(|| self.states_rom.as_ref().map(|(r, _, _)| r.clone()));
                if let Some(rom) = rom {
                    match slots::delete(&rom, n) {
                        Ok(()) => {
                            println!("deleted slot {n}");
                            if let Some(s) = self.session.as_mut() {
                                s.slots.clear(n);
                            }
                            if let Some((_, _, set)) = self.states_rom.as_mut() {
                                set.clear(n);
                            }
                        }
                        Err(e) => eprintln!("{e:#}"),
                    }
                }
            }
        }
        if keep {
            self.states = Some(screen);
        } else {
            self.states = None;
        }
        Ok(())
    }

    /// Switch to the Bench: the core capture runs only while it is open.
    pub fn open_bench(&mut self) {
        if let Some(s) = self.session.as_mut() {
            s.menu = None;
            s.set_paused(true);
            s.gba.set_bench(true);
            s.gba.set_layer_mute(self.bench.layer_mute);
            s.gba.set_channel_mute(self.bench.channel_mute);
            self.screen = Screen::Bench;
        }
    }

    pub fn close_bench(&mut self) {
        if let Some(s) = self.session.as_mut() {
            s.gba.set_bench(false);
            s.set_paused(false);
        }
        self.screen = Screen::Game;
    }

    fn update_bench(&mut self, input: &UiInput, video: &Video) -> Result<()> {
        let hotkeys = self.hotkeys.pressed(input);
        let slot = self.slot_cursor;
        if hotkeys.contains(&Hotkey::Bench) {
            self.close_bench();
            return Ok(());
        }
        if self.bench.update(input) == BenchOutcome::MutesChanged {
            if let Some(s) = self.session.as_mut() {
                s.gba.set_layer_mute(self.bench.layer_mute);
                s.gba.set_channel_mute(self.bench.channel_mute);
            }
        }
        let Some(s) = self.session.as_mut() else { return Ok(()) };
        for hk in hotkeys {
            match hk {
                Hotkey::Pause => {
                    let p = !s.paused;
                    s.set_paused(p);
                }
                Hotkey::FrameStep => self.step = true,
                Hotkey::DumpIo => debug_ui::dump_io(&mut s.gba),
                Hotkey::SaveState => match s.save_slot(slot) {
                    Ok(()) => println!("state saved to slot {slot}"),
                    Err(e) => eprintln!("state save failed: {e:#}"),
                },
                Hotkey::LoadState => match s.load_slot(slot) {
                    Ok(()) => println!("state loaded from slot {slot}"),
                    Err(e) => eprintln!("state load failed: {e}"),
                },
                Hotkey::Screenshot => match debug_ui::screenshot(&s.gba, &s.rom) {
                    Ok(p) => println!("screenshot: {}", p.display()),
                    Err(e) => eprintln!("screenshot failed: {e:#}"),
                },
                _ => {}
            }
        }
        s.set_fast(video.is_key_down(self.hotkeys.fast_forward));
        s.gba.set_keys(Keys(input.game_mask));
        if !s.paused || self.step {
            s.step();
        }
        Ok(())
    }

    fn go_back(&mut self, back: Back) {
        self.screen = match back {
            Back::Game if self.session.is_some() => {
                if let Some(s) = self.session.as_mut() {
                    s.set_paused(false);
                }
                Screen::Game
            }
            _ => Screen::Library,
        };
    }

    fn update_game(&mut self, input: &UiInput, video: &Video) -> Result<()> {
        let hotkeys: Vec<Hotkey> = self.hotkeys.pressed(input);
        let now = Instant::now();
        let slot = self.slot_cursor;
        let Some(s) = self.session.as_mut() else { return Ok(()) };

        // --- pause menu ----------------------------------------------------------------------
        if let Some(mut menu) = s.menu.take() {
            let mut keep = true;
            if hotkeys.contains(&Hotkey::Pause) {
                keep = false;
            }
            if hotkeys.contains(&Hotkey::Bench) {
                s.menu = Some(menu);
                self.open_bench();
                return Ok(());
            }
            if hotkeys.contains(&Hotkey::SaveStates) {
                s.menu = Some(menu);
                self.open_states(Back::Game, None);
                return Ok(());
            }
            if hotkeys.contains(&Hotkey::FrameStep) {
                self.step = true;
            }
            let infos = s.slots.infos();
            match menu.handle(input, &infos, now) {
                Outcome::None => {}
                Outcome::Resume => keep = false,
                Outcome::SaveSlot(n) => {
                    self.slot_cursor = n;
                    match s.save_slot(n) {
                        Ok(()) => menu.show_message(format!("SAVED TO SLOT {n}"), now),
                        Err(e) => {
                            eprintln!("state save failed: {e:#}");
                            menu.show_message(format!("SAVE FAILED: {e}"), now);
                        }
                    }
                }
                Outcome::LoadSlot(n) => {
                    self.slot_cursor = n;
                    match s.load_slot(n) {
                        Ok(()) => {
                            println!("state loaded from slot {n}");
                            keep = false;
                        }
                        Err(e) => {
                            eprintln!("state load failed: {e}");
                            menu.show_message(format!("LOAD FAILED: {e}"), now);
                        }
                    }
                }
                Outcome::Settings => {
                    s.menu = Some(menu);
                    self.open_settings(Back::Game);
                    return Ok(());
                }
                Outcome::Bench => {
                    self.open_bench();
                    return Ok(());
                }
                Outcome::Quit => {
                    self.end_game();
                    return Ok(());
                }
            }
            if keep {
                s.menu = Some(menu);
            } else {
                s.set_paused(false);
            }
            if self.step {
                s.step();
            }
            return Ok(());
        }

        // --- hotkeys while playing -------------------------------------------------------------
        for hk in hotkeys {
            match hk {
                Hotkey::Pause => {
                    s.set_paused(true);
                    s.menu = Some(PauseMenu::new(slot));
                }
                Hotkey::FrameStep => self.step = s.paused,
                Hotkey::DumpIo => debug_ui::dump_io(&mut s.gba),
                Hotkey::Screenshot => match debug_ui::screenshot(&s.gba, &s.rom) {
                    Ok(p) => {
                        let name = p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
                        s.show_toast(format!("SCREENSHOT {name}"));
                    }
                    Err(e) => eprintln!("screenshot failed: {e:#}"),
                },
                Hotkey::SaveNow => match s.saves.force_write(&mut s.gba) {
                    Ok(true) => s.show_toast(".SAV WRITTEN"),
                    Ok(false) => s.show_toast("NO SAVE MEMORY"),
                    Err(e) => eprintln!("save failed: {e:#}"),
                },
                Hotkey::SaveState => match s.save_slot(slot) {
                    Ok(()) => s.show_toast(format!("SAVED TO SLOT {slot}")),
                    Err(e) => eprintln!("state save failed: {e:#}"),
                },
                Hotkey::LoadState => match s.load_slot(slot) {
                    Ok(()) => s.show_toast(format!("LOADED SLOT {slot}")),
                    Err(e) => {
                        eprintln!("state load failed: {e}");
                        s.show_toast(format!("LOAD FAILED: {e}"));
                    }
                },
                Hotkey::Bench => {
                    self.open_bench();
                    return Ok(());
                }
                Hotkey::SaveStates => {
                    self.open_states(Back::Game, None);
                    return Ok(());
                }
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
                let now = Instant::now();
                if !s.fast || s.last_present.elapsed() >= FAST_FORWARD_PRESENT {
                    let fb = s.frame_to_present(now);
                    video.present_game(fb, scale, NIGHT.bg)?;
                    s.last_present = Instant::now();
                } else {
                    video.poll();
                }
                if s.last_title.elapsed() >= TITLE_REFRESH {
                    video.set_title(&s.title());
                    s.last_title = Instant::now();
                }
            }
            Screen::SaveStates { .. } => {
                self.canvas.resize(w as u32, h as u32);
                let (slots, game, running) = match (&self.session, &self.states_rom) {
                    (Some(s), _) => (&s.slots, s.game.as_str(), true),
                    (None, Some((_, name, slots))) => (slots, name.as_str(), false),
                    (None, None) => return Ok(()),
                };
                let ctx = StatesCtx { slots, game, running };
                if let Some(screen) = self.states.as_mut() {
                    screen.render(&mut self.canvas, &ctx);
                }
                video.present_canvas(&self.canvas)?;
            }
            Screen::Settings { .. } => {
                self.canvas.resize(w as u32, h as u32);
                let ctx = SettingsCtx {
                    settings: &mut self.settings,
                    keymap: &mut self.keymap,
                    padmap: &mut self.padmap,
                    hotkeys: &mut self.hotkeys,
                    audio_devices: &self.audio_devices,
                    rebinding: false,
                };
                self.settings_screen.render(&mut self.canvas, &ctx);
                video.present_canvas(&self.canvas)?;
            }
            Screen::Library => {
                self.canvas.resize(w as u32, h as u32);
                self.library.poll();
                self.library_screen.render(&mut self.canvas, &self.library);
                video.present_canvas(&self.canvas)?;
                if !self.title_is_library {
                    video.set_title(config::APP_NAME);
                    self.title_is_library = true;
                }
            }
            Screen::Bench => {
                let Some(s) = self.session.as_mut() else { return Ok(()) };
                self.canvas.resize(w as u32, h as u32);
                let ctx = BenchCtx {
                    gba: &s.gba,
                    code: &s.code,
                    save: s.gba.bus.cart.save_type,
                    hle: s.gba.bus.bios.is_hle(),
                    host_ms: s.host_ms,
                    paced_ms: s.paced_ms,
                    audio_underruns: s.audio.as_ref().map(|a| a.underruns()),
                    running: !s.paused,
                };
                self.bench.render(&mut self.canvas, &ctx);
                video.present_canvas(&self.canvas)?;
                if s.last_title.elapsed() >= TITLE_REFRESH {
                    video.set_title(&s.title());
                    s.last_title = Instant::now();
                }
            }
        }
        Ok(())
    }

    /// Sleep until the next frame is due.
    pub fn pace(&mut self) {
        let base_period = config::frame_period();
        match (self.screen, self.session.as_mut()) {
            (Screen::Game | Screen::Bench, Some(s)) if !s.paused => {
                if self.opts.fullspeed {
                    return;
                }
                if s.fast {
                    match self.ff_limit {
                        Some(n) if n > 1 => s.pacer.wait(base_period / n),
                        _ => {}
                    }
                    return;
                }
                let period = match (&s.audio, self.settings.pacing) {
                    (Some(a), crate::settings::Pacing::AudioClocked) => paced_period(base_period, a.fill(), a.target_fill(), AUDIO_PACE_MAX_ADJUST),
                    _ => base_period,
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

    /// Final save flush, remembered window size and the exit statistics line.
    pub fn shutdown(&mut self, window: (usize, usize)) {
        if !self.opts.exact_window {
            self.settings.window = Some((window.0 as u32, window.1 as u32));
            self.save_settings();
        }
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

#[cfg(test)]
mod tests {
    use super::migrate_dir;

    #[test]
    fn legacy_dir_moves_once_and_never_overwrites() {
        let root = std::env::temp_dir().join(format!("strawberry-migrate-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let (old, new) = (root.join("gba-advance"), root.join("strawberry"));
        // neither: nothing to do
        assert!(!migrate_dir(&old, &new));
        assert!(!new.exists());
        // old only: moved with its contents
        std::fs::create_dir_all(&old).unwrap();
        std::fs::write(old.join("settings.toml"), "volume = 7
").unwrap();
        assert!(migrate_dir(&old, &new));
        assert!(!old.exists());
        assert_eq!(std::fs::read_to_string(new.join("settings.toml")).unwrap(), "volume = 7
");
        // both: the new one is left alone
        std::fs::create_dir_all(&old).unwrap();
        std::fs::write(old.join("settings.toml"), "volume = 1
").unwrap();
        assert!(!migrate_dir(&old, &new));
        assert_eq!(std::fs::read_to_string(new.join("settings.toml")).unwrap(), "volume = 7
");
        assert!(old.exists());
        let _ = std::fs::remove_dir_all(&root);
    }
}
