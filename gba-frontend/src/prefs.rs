//! The mechanics of a settings file, independent of what it holds: one human-readable TOML
//! table, read so that a malformed value costs only that key ([`take`]), and written atomically
//! with the keys this version does not know kept ([`write`]). Button bindings are stored by name
//! ([`KeyBindings`], [`PadBindings`]) because neither `minifb::Key` nor `gilrs::Button` has serde
//! support.

use crate::ui::input::{bindable, key_from_name, key_name, pad_button_from_name, pad_button_name, KeyMap, PadMap};
use anyhow::{Context, Result};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// `<config dir>/<app_dir>/settings.toml`.
pub fn config_path(app_dir: &str) -> Option<PathBuf> {
    dirs::config_dir().map(|d| d.join(app_dir).join("settings.toml"))
}

/// The file's table. `None` for a missing file, and for one that is not TOML at all, which is
/// first copied to `<name>.bad`.
pub fn read(path: &Path) -> Option<toml::Table> {
    let text = std::fs::read_to_string(path).ok()?;
    match text.parse::<toml::Table>() {
        Ok(t) => Some(t),
        Err(_) => {
            let mut bad = path.as_os_str().to_owned();
            bad.push(".bad");
            let _ = std::fs::copy(path, bad);
            None
        }
    }
}

/// One key of the table, `None` when it is missing or does not parse as a `T`.
pub fn take<T: DeserializeOwned>(t: &toml::Table, key: &str) -> Option<T> {
    t.get(key).cloned().and_then(|v| v.try_into().ok())
}

/// Write `values` (a struct of the known keys) over `extra` (the table as read, so unknown keys
/// survive) to `path`, atomically, creating its folder. `known_keys` lists every key `values` may
/// write, so one it now omits (an `Option` that is `None`) is dropped from `extra` too.
pub fn write(path: &Path, app_name: &str, extra: &toml::Table, known_keys: &[&str], values: &impl Serialize) -> Result<()> {
    let mut table = extra.clone();
    for k in known_keys {
        table.remove(*k);
    }
    let known = toml::Table::try_from(values).context("encoding settings")?;
    table.extend(known);
    let text = format!("# {app_name} settings. Edited by the Settings screen; unknown keys are kept.\n{}", toml::to_string_pretty(&table).context("encoding settings")?);
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).with_context(|| format!("creating {}", dir.display()))?;
    }
    crate::saves::write_atomic(path, text.as_bytes())
}

/// Wrapper that is invisible to `PartialEq` (raw file contents and CLI bookkeeping are not
/// "settings" when comparing two `Settings`).
#[derive(Clone, Debug, Default)]
pub struct Hidden<T>(pub T);

impl<T> PartialEq for Hidden<T> {
    fn eq(&self, _: &Self) -> bool {
        true
    }
}

/// Button name (the [`ButtonSet`](crate::ui::input::ButtonSet)'s, e.g. `A`, `SELECT`) -> key name.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct KeyBindings(pub BTreeMap<String, String>);

impl KeyBindings {
    pub fn from_keymap(km: &KeyMap) -> Self {
        KeyBindings(km.set().names.iter().zip(&km.keys).map(|(n, k)| (n.to_string(), key_name(*k))).collect())
    }

    /// The live map: `defaults` with every button the file names (as a key) rebound.
    pub fn to_keymap(&self, defaults: KeyMap) -> KeyMap {
        let mut km = defaults;
        let names = km.set().names;
        for (i, name) in names.iter().enumerate() {
            if let Some(k) = self.0.get(*name).and_then(|n| key_from_name(n)).filter(|k| bindable(*k)) {
                km.keys[i] = k;
            }
        }
        km
    }
}

/// Button name -> gamepad button name (`ui::input::pad_button_name`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct PadBindings(pub BTreeMap<String, String>);

impl PadBindings {
    pub fn from_padmap(pm: &PadMap) -> Self {
        PadBindings(pm.set().names.iter().zip(&pm.buttons).map(|(n, b)| (n.to_string(), pad_button_name(*b).to_string())).collect())
    }

    /// The live map: `defaults` with every button the file names (as a pad button) rebound.
    pub fn to_padmap(&self, defaults: PadMap) -> PadMap {
        let mut pm = defaults;
        let names = pm.set().names;
        for (i, name) in names.iter().enumerate() {
            if let Some(b) = self.0.get(*name).and_then(|n| pad_button_from_name(n)) {
                pm.buttons[i] = b;
            }
        }
        pm
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::input::{ButtonSet, MenuRoles};
    use gilrs::Button;
    use minifb::Key;

    const SET: ButtonSet = ButtonSet {
        names: &["FIRE", "JUMP"],
        bits: &[1, 2],
        roles: MenuRoles { up: 0, down: 0, left: 0, right: 0, confirm: 1, back: 2, prev_page: 0, next_page: 0, start: 0 },
    };

    fn temp_dir(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("strawberry-prefs-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[derive(Serialize)]
    struct Two {
        volume: u8,
        #[serde(skip_serializing_if = "Option::is_none")]
        device: Option<String>,
    }

    #[test]
    fn unknown_keys_survive_and_known_ones_are_replaced() {
        let d = temp_dir("rw");
        let p = d.join("sub").join("settings.toml");
        assert!(read(&p).is_none(), "missing file");
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(&p, "volume = \"loud\"\ndevice = \"old\"\nother = 1\n").unwrap();
        let t = read(&p).unwrap();
        assert_eq!(take::<u8>(&t, "volume"), None, "a malformed value is just missing");
        assert_eq!(take::<String>(&t, "device").as_deref(), Some("old"));
        write(&p, "Test", &t, &["volume", "device"], &Two { volume: 3, device: None }).unwrap();
        assert_eq!(std::fs::read_to_string(&p).unwrap(), "# Test settings. Edited by the Settings screen; unknown keys are kept.\nother = 1\nvolume = 3\n");
        std::fs::write(&p, "[not toml").unwrap();
        assert!(read(&p).is_none());
        assert!(d.join("sub").join("settings.toml.bad").is_file(), "the unreadable file is kept");
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn bindings_round_trip_and_fill_gaps_from_the_defaults() {
        let km = KeyMap::new(SET, &[(Key::Space, 1), (Key::Up, 2)]);
        let kb = KeyBindings::from_keymap(&km);
        assert_eq!(kb.0.get("FIRE").map(String::as_str), Some("Space"));
        assert_eq!(kb.to_keymap(KeyMap::new(SET, &[])), km);
        let partial = KeyBindings([("JUMP".to_string(), "Z".to_string()), ("FIRE".to_string(), "Nonsense".to_string())].into_iter().collect());
        let got = partial.to_keymap(km.clone());
        assert_eq!((got.key_for(1), got.key_for(2)), (Key::Space, Key::Z), "unknown key name keeps the default");
        let pm = PadMap::new(SET, &[(Button::South, 1)]);
        let pb = PadBindings::from_padmap(&pm);
        assert_eq!(pb.to_padmap(PadMap::new(SET, &[])), pm);
        assert_eq!(PadBindings(BTreeMap::new()).to_padmap(pm.clone()), pm, "an empty table is the defaults");
    }
}
