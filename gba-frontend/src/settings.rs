//! The settings file: one human-readable TOML file in the user's config folder.
//!
//! * Loaded at start ([`Settings::load`]): a missing file, a missing key or a malformed value
//!   falls back to the default for that key alone.
//! * Written atomically on change ([`Settings::save`]). Keys this version does not know are kept:
//!   the raw table is read along with the file and the known keys are overlaid on it when saving.
//! * Command-line flags override the file for the run only ([`Settings::apply_cli`]); saving
//!   writes the file's own values back for every setting that still has its overlaid value.
//!
//! Keyboard bindings are stored by key *name* (the text printed on the keycap, see
//! [`crate::ui::input::key_name`]) because `minifb::Key` has no serde support.

// Not wired into `app` yet: the integrator connects `Settings`.
#![allow(dead_code)]

use crate::config::Cli;
use crate::debug_ui::{Hotkey, HotkeyMap, HOTKEYS};
use crate::input::{default_keymap, default_padmap};
pub use crate::prefs::{Hidden, KeyBindings, PadBindings};
use crate::prefs::take;
use crate::ui::input::{bindable, key_from_name, key_name, KeyMap, PadMap};
use anyhow::Result;
use minifb::Key;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// Size of the game picture.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PictureSize {
    /// A whole-number multiple of 240x160 (1..=10; the screen offers 1..=4).
    Fixed(u32),
    /// The largest whole-number scale that fits the window.
    Largest,
}

impl Serialize for PictureSize {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        match self {
            PictureSize::Fixed(n) => s.serialize_u32(*n),
            PictureSize::Largest => s.serialize_str("largest"),
        }
    }
}

impl<'de> Deserialize<'de> for PictureSize {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Raw {
            N(u32),
            S(String),
        }
        match Raw::deserialize(d)? {
            Raw::N(n) if (1..=10).contains(&n) => Ok(PictureSize::Fixed(n)),
            Raw::S(s) if s.eq_ignore_ascii_case("largest") => Ok(PictureSize::Largest),
            _ => Err(serde::de::Error::custom("picture must be 1..=10 or \"largest\"")),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Filter {
    #[default]
    None,
    LcdGrid,
    FrameBlend,
}

/// Fast-forward speed limit while the fast-forward key is held.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FfLimit {
    X2,
    #[default]
    X4,
    Unlimited,
}

impl FfLimit {
    /// Speed multiple, `None` = no limit.
    pub fn factor(self) -> Option<u32> {
        match self {
            FfLimit::X2 => Some(2),
            FfLimit::X4 => Some(4),
            FfLimit::Unlimited => None,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Bios {
    /// The high-level BIOS (runs every game in the library).
    #[default]
    BuiltIn,
    /// A 16 KiB BIOS dump.
    File(PathBuf),
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AudioMode {
    #[default]
    On,
    /// Device open, silence output (pacing still follows the audio clock).
    Muted,
    /// No device; pacing uses the wall clock.
    Off,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Pacing {
    #[default]
    AudioClocked,
    WallClock,
}

/// Every [`Hotkey`], in the order the settings file lists them.
pub const ALL_HOTKEYS: [Hotkey; 9] = [
    Hotkey::Pause,
    Hotkey::FrameStep,
    Hotkey::DumpIo,
    Hotkey::Screenshot,
    Hotkey::SaveNow,
    Hotkey::SaveState,
    Hotkey::LoadState,
    Hotkey::Bench,
    Hotkey::SaveStates,
];

/// Settings-file name of a hotkey.
pub fn hotkey_id(h: Hotkey) -> &'static str {
    match h {
        Hotkey::Pause => "pause",
        Hotkey::FrameStep => "frame_step",
        Hotkey::DumpIo => "dump_io",
        Hotkey::Screenshot => "screenshot",
        Hotkey::SaveNow => "save_now",
        Hotkey::SaveState => "save_state",
        Hotkey::LoadState => "load_state",
        Hotkey::Bench => "bench",
        Hotkey::SaveStates => "save_states",
    }
}

/// Hotkey -> key name, plus the held fast-forward key. Escape always pauses and is not stored.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct HotkeyBindings {
    pub keys: BTreeMap<String, String>,
    pub fast_forward: String,
}

impl Default for HotkeyBindings {
    fn default() -> Self {
        Self::from_hotkeys(&HotkeyMap::default())
    }
}

impl HotkeyBindings {
    pub fn from_hotkeys(hk: &HotkeyMap) -> Self {
        let mut keys = BTreeMap::new();
        for h in ALL_HOTKEYS {
            // Escape also pauses but is not rebindable: store the other key.
            let k = hk.keys.iter().find(|(k, x)| *x == h && *k != Key::Escape).map(|(k, _)| *k);
            if let Some(k) = k {
                keys.insert(hotkey_id(h).to_string(), key_name(k));
            }
        }
        HotkeyBindings { keys, fast_forward: key_name(hk.fast_forward) }
    }

    /// The live map; missing or unknown entries keep their default key. Escape pauses always.
    pub fn to_hotkeys(&self) -> HotkeyMap {
        let mut keys = Vec::new();
        for (dk, h) in HOTKEYS {
            if *dk == Key::Escape {
                keys.push((Key::Escape, *h));
                continue;
            }
            let k = self.keys.get(hotkey_id(*h)).and_then(|n| key_from_name(n)).filter(|k| bindable(*k) && *k != Key::Escape);
            keys.push((k.unwrap_or(*dk), *h));
        }
        let fast_forward = key_from_name(&self.fast_forward).filter(|k| bindable(*k)).unwrap_or(Key::Tab);
        HotkeyMap { keys, fast_forward }
    }
}

/// What the file held before the command line was overlaid, and the state right after.
#[derive(Clone, Debug)]
pub struct CliOverlay {
    original: Settings,
    overlaid: Settings,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub picture: PictureSize,
    pub filter: Filter,
    pub ff_limit: FfLimit,
    /// 0..=10
    pub volume: u8,
    pub bios: Bios,
    pub library_folders: Vec<PathBuf>,
    /// `None` = the default output device.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audio_device: Option<String>,
    pub audio: AudioMode,
    pub pacing: Pacing,
    /// Frames between `.sav` writes: 60, 300 or 600.
    pub autosave_frames: u64,
    pub keymap: KeyBindings,
    pub hotkeys: HotkeyBindings,
    /// Gamepad bindings, by gilrs button name.
    pub gamepad: PadBindings,
    /// Remembered window size.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub window: Option<(u32, u32)>,
    /// Keys of the file this version does not know; written back on save.
    #[serde(skip)]
    extra: Hidden<toml::Table>,
    /// Set by [`Settings::apply_cli`].
    #[serde(skip)]
    cli: Hidden<Option<Box<CliOverlay>>>,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            picture: PictureSize::Largest,
            filter: Filter::None,
            ff_limit: FfLimit::X4,
            volume: 10,
            bios: Bios::BuiltIn,
            library_folders: Vec::new(),
            audio_device: None,
            audio: AudioMode::On,
            pacing: Pacing::AudioClocked,
            autosave_frames: 60,
            keymap: KeyBindings::from_keymap(&default_keymap()),
            hotkeys: HotkeyBindings::default(),
            gamepad: PadBindings::from_padmap(&default_padmap()),
            window: None,
            extra: Hidden::default(),
            cli: Hidden::default(),
        }
    }
}

/// Every key this version writes.
const KNOWN_KEYS: [&str; 14] = [
    "picture",
    "filter",
    "ff_limit",
    "volume",
    "bios",
    "library_folders",
    "audio_device",
    "audio",
    "pacing",
    "autosave_frames",
    "keymap",
    "hotkeys",
    "gamepad",
    "window",
];

impl Settings {
    /// `<config dir>/strawberry/settings.toml`.
    pub fn default_path() -> Option<PathBuf> {
        crate::prefs::config_path(crate::app::APP_DIR)
    }

    /// Load the file. Never fails: a missing file gives the defaults, a missing or malformed key
    /// gives that key's default, unknown keys are kept for [`Settings::save`]. A file that is not
    /// TOML at all is copied to `<name>.bad` and the defaults are used.
    pub fn load(path: &Path) -> Settings {
        let mut s = Settings::default();
        let Some(table) = crate::prefs::read(path) else { return s };
        macro_rules! field {
            ($f:ident) => {
                if let Some(v) = take(&table, stringify!($f)) {
                    s.$f = v;
                }
            };
        }
        field!(picture);
        field!(filter);
        field!(ff_limit);
        field!(volume);
        field!(bios);
        field!(library_folders);
        field!(audio_device);
        field!(audio);
        field!(pacing);
        field!(autosave_frames);
        field!(keymap);
        field!(hotkeys);
        field!(gamepad);
        field!(window);
        s.volume = s.volume.min(10);
        s.autosave_frames = s.autosave_frames.max(1);
        // fill gaps (a hand-written file may name only some buttons)
        s.keymap = KeyBindings::from_keymap(&s.to_keymap());
        s.hotkeys = HotkeyBindings::from_hotkeys(&s.hotkeys.to_hotkeys());
        s.gamepad = PadBindings::from_padmap(&s.to_padmap());
        s.extra = Hidden(table);
        s
    }

    /// The values to write: the live ones, except that a setting still holding the value the
    /// command line gave it goes back to what the file had.
    fn persistable(&self) -> Settings {
        let mut out = self.clone();
        out.cli = Hidden(None);
        if let Some(c) = &self.cli.0 {
            macro_rules! keep {
                ($($f:ident),*) => { $( if out.$f == c.overlaid.$f { out.$f = c.original.$f.clone(); } )* };
            }
            keep!(picture, audio, bios, library_folders);
        }
        out
    }

    /// Write the file atomically (creating its folder). Unknown keys from the loaded file are
    /// kept; comments in it are not.
    pub fn save(&self, path: &Path) -> Result<()> {
        crate::prefs::write(path, crate::config::APP_NAME, &self.extra.0, &KNOWN_KEYS, &self.persistable())
    }

    /// Overlay `--scale`, `--mute`, `--no-audio`, `--bios`/`--hle` and `--library` for this run
    /// only: [`Settings::save`] writes the file's own values for those settings back unless the
    /// player changed them on the Settings screen meanwhile.
    pub fn apply_cli(&mut self, cli: &Cli) {
        let original = match &self.cli.0 {
            Some(c) => c.original.clone(),
            None => self.clone(),
        };
        if let Some(n) = cli.scale {
            self.picture = PictureSize::Fixed(n.clamp(1, 10));
        }
        if cli.no_audio {
            self.audio = AudioMode::Off;
        } else if cli.mute {
            self.audio = AudioMode::Muted;
        }
        if let Some(p) = &cli.bios {
            self.bios = Bios::File(p.clone());
        } else if cli.hle {
            self.bios = Bios::BuiltIn;
        }
        for d in &cli.library {
            if !self.library_folders.contains(d) {
                self.library_folders.push(d.clone());
            }
        }
        let mut overlaid = self.clone();
        overlaid.cli = Hidden(None);
        self.cli = Hidden(Some(Box::new(CliOverlay { original, overlaid })));
    }

    pub fn to_keymap(&self) -> KeyMap {
        self.keymap.to_keymap(default_keymap())
    }

    #[allow(clippy::wrong_self_convention)]
    pub fn from_keymap(&mut self, km: &KeyMap) {
        self.keymap = KeyBindings::from_keymap(km);
    }

    pub fn to_hotkeys(&self) -> HotkeyMap {
        self.hotkeys.to_hotkeys()
    }

    #[allow(clippy::wrong_self_convention)]
    pub fn from_hotkeys(&mut self, hk: &HotkeyMap) {
        self.hotkeys = HotkeyBindings::from_hotkeys(hk);
    }

    pub fn to_padmap(&self) -> PadMap {
        self.gamepad.to_padmap(default_padmap())
    }

    #[allow(clippy::wrong_self_convention)]
    pub fn from_padmap(&mut self, pm: &PadMap) {
        self.gamepad = PadBindings::from_padmap(pm);
    }
}

#[cfg(test)]
#[allow(clippy::field_reassign_with_default)]
mod tests {
    use super::*;
    use clap::Parser;
    use gba_core::Keys;
    use gilrs::Button;

    fn temp_dir(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("gba-settings-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn defaults() {
        let s = Settings::default();
        assert_eq!(s.picture, PictureSize::Largest);
        assert_eq!(s.filter, Filter::None);
        assert_eq!(s.ff_limit, FfLimit::X4);
        assert_eq!(s.volume, 10);
        assert_eq!(s.bios, Bios::BuiltIn);
        assert!(s.library_folders.is_empty());
        assert_eq!(s.audio_device, None);
        assert_eq!(s.audio, AudioMode::On);
        assert_eq!(s.pacing, Pacing::AudioClocked);
        assert_eq!(s.autosave_frames, 60);
        assert_eq!(s.window, None);
        assert_eq!(s.keymap.0["A"], "Z");
        assert_eq!(s.keymap.0["SELECT"], "R Shift");
        assert_eq!(s.gamepad.0["A"], "South");
        assert_eq!(s.gamepad.0["L"], "LB");
        assert_eq!(s.hotkeys.fast_forward, "Tab");
        assert_eq!(s.hotkeys.keys["pause"], "P");
        assert_eq!(FfLimit::X2.factor(), Some(2));
        assert_eq!(FfLimit::Unlimited.factor(), None);
    }

    #[test]
    fn round_trip_in_temp_dir() {
        let d = temp_dir("rt");
        let p = d.join("sub").join("settings.toml");
        let mut s = Settings::default();
        s.picture = PictureSize::Fixed(3);
        s.filter = Filter::LcdGrid;
        s.ff_limit = FfLimit::Unlimited;
        s.volume = 7;
        s.bios = Bios::File(PathBuf::from("C:/bios/gba_bios.bin"));
        s.library_folders = vec![PathBuf::from("C:/Games"), PathBuf::from("D:/roms")];
        s.audio_device = Some("Speakers (Realtek)".into());
        s.audio = AudioMode::Muted;
        s.pacing = Pacing::WallClock;
        s.autosave_frames = 600;
        s.window = Some((1024, 700));
        let mut km = default_keymap();
        km.rebind(Keys::A, Key::X);
        s.from_keymap(&km);
        let mut hk = HotkeyMap::default();
        hk.fast_forward = Key::Q;
        hk.keys.iter_mut().find(|(_, h)| *h == Hotkey::Screenshot).unwrap().0 = Key::F9;
        s.from_hotkeys(&hk);
        s.save(&p).unwrap();
        let back = Settings::load(&p);
        assert_eq!(back, s);
        assert_eq!(back.to_keymap(), km);
        assert_eq!(back.to_hotkeys().fast_forward, Key::Q);
        assert_eq!(back.to_hotkeys().key_for(Hotkey::Screenshot), Some(Key::F9));
        assert!(!d.join("sub").join("settings.toml.tmp").exists());
        let text = std::fs::read_to_string(&p).unwrap();
        assert!(text.contains("picture = 3"), "{text}");
        assert!(text.contains("ff_limit = \"unlimited\""), "{text}");
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn largest_and_default_bios_serialise_readably() {
        let d = temp_dir("readable");
        let p = d.join("s.toml");
        Settings::default().save(&p).unwrap();
        let text = std::fs::read_to_string(&p).unwrap();
        assert!(text.contains("picture = \"largest\""), "{text}");
        assert!(text.contains("bios = \"built-in\""), "{text}");
        assert!(!text.contains("audio_device"), "None is omitted");
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn unknown_keys_survive_a_save() {
        let d = temp_dir("unknown");
        let p = d.join("settings.toml");
        std::fs::write(&p, "future_option = \"keep me\"\nvolume = 3\naudio_device = \"Old\"\n[future_table]\nx = 1\n").unwrap();
        let mut s = Settings::load(&p);
        assert_eq!(s.volume, 3);
        s.volume = 4;
        s.audio_device = None;
        s.save(&p).unwrap();
        let t: toml::Table = std::fs::read_to_string(&p).unwrap().parse().unwrap();
        assert_eq!(t["future_option"].as_str(), Some("keep me"));
        assert_eq!(t["future_table"]["x"].as_integer(), Some(1));
        assert_eq!(t["volume"].as_integer(), Some(4));
        assert!(t.get("audio_device").is_none(), "a cleared option does not linger");
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn missing_and_bad_keys_default() {
        let d = temp_dir("missing");
        let p = d.join("settings.toml");
        assert_eq!(Settings::load(&p), Settings::default(), "no file");
        std::fs::write(&p, "volume = 3\npicture = 99\nfilter = \"sepia\"\nautosave_frames = \"soon\"\n[keymap]\nA = \"Q\"\n[gamepad]\nB = \"West\"\nL = \"Nonsense\"\n").unwrap();
        let s = Settings::load(&p);
        assert_eq!(s.volume, 3);
        assert_eq!(s.picture, PictureSize::Largest, "out of range");
        assert_eq!(s.filter, Filter::None, "unknown variant");
        assert_eq!(s.autosave_frames, 60, "wrong type");
        assert_eq!(s.pacing, Pacing::AudioClocked);
        let km = s.to_keymap();
        assert_eq!(km.key_for(Keys::A), Key::Q);
        assert_eq!(km.key_for(Keys::B), Key::X, "unnamed buttons keep their default");
        assert_eq!(s.keymap.0.len(), 10, "gaps are filled");
        let pm = s.to_padmap();
        assert_eq!(pm.button_for(Keys::B), Button::West);
        assert_eq!(pm.button_for(Keys::L), Button::LeftTrigger, "a name that is no pad button keeps the default");
        assert_eq!(pm.button_for(Keys::A), Button::South, "unnamed buttons keep their default");
        assert_eq!(s.gamepad.0.len(), 10, "gaps are filled");
        std::fs::write(&p, "volume = 99\n").unwrap();
        assert_eq!(Settings::load(&p).volume, 10, "clamped");
        std::fs::write(&p, "this is [not toml").unwrap();
        assert_eq!(Settings::load(&p), Settings::default());
        assert!(d.join("settings.toml.bad").exists(), "unreadable file is kept");
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn keymap_and_hotkey_conversions() {
        let mut s = Settings::default();
        assert_eq!(s.to_keymap(), default_keymap());
        let hk = s.to_hotkeys();
        assert_eq!(hk, HotkeyMap::default(), "defaults convert to the live defaults");
        assert_eq!(s.to_padmap(), default_padmap());
        let mut pm = default_padmap();
        pm.rebind(Keys::A, Button::West);
        s.from_padmap(&pm);
        assert_eq!(s.gamepad.0["A"], "West");
        assert_eq!(s.to_padmap(), pm);
        let mut km = default_keymap();
        km.rebind(Keys::START, Key::Space);
        s.from_keymap(&km);
        assert_eq!(s.keymap.0["START"], "Space");
        assert_eq!(s.to_keymap(), km);
        let mut hk = HotkeyMap::default();
        for (k, h) in hk.keys.iter_mut() {
            if *h == Hotkey::Pause && *k == Key::P {
                *k = Key::O;
            }
        }
        s.from_hotkeys(&hk);
        assert_eq!(s.hotkeys.keys["pause"], "O");
        let back = s.to_hotkeys();
        assert!(back.keys.contains(&(Key::Escape, Hotkey::Pause)), "Esc always pauses");
        assert!(back.keys.contains(&(Key::O, Hotkey::Pause)));
        // a name that is not a key falls back to the default
        s.hotkeys.keys.insert("bench".into(), "Nope".into());
        assert_eq!(s.to_hotkeys().key_for(Hotkey::Bench), Some(Key::F12));
    }

    #[test]
    fn cli_overlay_is_for_the_run_only() {
        let d = temp_dir("cli");
        let p = d.join("settings.toml");
        let mut s = Settings::default();
        s.picture = PictureSize::Fixed(2);
        s.library_folders = vec![PathBuf::from("C:/Games")];
        s.save(&p).unwrap();

        let cli = Cli::parse_from(["strawberry", "--scale", "3", "--mute", "--bios", "bios.bin", "--library", "E:/more"]);
        let mut s = Settings::load(&p);
        s.apply_cli(&cli);
        assert_eq!(s.picture, PictureSize::Fixed(3));
        assert_eq!(s.audio, AudioMode::Muted);
        assert_eq!(s.bios, Bios::File(PathBuf::from("bios.bin")));
        assert_eq!(s.library_folders, vec![PathBuf::from("C:/Games"), PathBuf::from("E:/more")]);

        // an unrelated change saves; the overlaid values do not reach the file
        s.volume = 5;
        s.save(&p).unwrap();
        let on_disk = Settings::load(&p);
        assert_eq!(on_disk.volume, 5);
        assert_eq!(on_disk.picture, PictureSize::Fixed(2));
        assert_eq!(on_disk.audio, AudioMode::On);
        assert_eq!(on_disk.bios, Bios::BuiltIn);
        assert_eq!(on_disk.library_folders, vec![PathBuf::from("C:/Games")]);

        // a setting the player changes on the screen does reach the file
        s.picture = PictureSize::Fixed(4);
        s.save(&p).unwrap();
        assert_eq!(Settings::load(&p).picture, PictureSize::Fixed(4));
        assert_eq!(Settings::load(&p).audio, AudioMode::On);

        let mut t = Settings::default();
        t.apply_cli(&Cli::parse_from(["strawberry", "--no-audio", "--hle"]));
        assert_eq!(t.audio, AudioMode::Off);
        assert_eq!(t.bios, Bios::BuiltIn);
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn known_keys_cover_a_full_serialisation() {
        let mut s = Settings::default();
        s.audio_device = Some("x".into());
        s.window = Some((1, 2));
        let t = toml::Table::try_from(&s).unwrap();
        let mut keys: Vec<&str> = t.keys().map(String::as_str).collect();
        keys.sort();
        let mut known = KNOWN_KEYS.to_vec();
        known.sort();
        assert_eq!(keys, known);
    }
}
