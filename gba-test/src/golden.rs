//! `tests/golden.toml` schema.
//!
//! ```toml
//! [defaults]
//! hash = "xxh3-64"
//! frames = 16
//! bios = "hle"
//!
//! [[rom]]
//! path = "gba-tests/arm/arm.gba"      # relative to GBA_TESTS_DIR (or absolute)
//! check = "jsmolka"                   # jsmolka | mgba-suite | none (default none)
//! reg = "r12"                         # jsmolka result register (default r12)
//! golden = "3f2a9c1d7e5b8a04"         # optional framebuffer hash after `frames`
//! frames = 16
//! input = [{ frame = 4, press = "A" }, { frame = 6, release = "A" }]
//! expect = "1552/1552"                # mgba-suite END: p/t
//! skip = "needs built suite.gba"      # skip with reason
//! ```

use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Defaults {
    #[serde(default = "default_hash")]
    pub hash: String,
    #[serde(default = "default_frames")]
    pub frames: u32,
    #[serde(default = "default_bios")]
    pub bios: String,
}

fn default_hash() -> String {
    "xxh3-64".into()
}
fn default_frames() -> u32 {
    16
}
fn default_bios() -> String {
    "hle".into()
}

impl Default for Defaults {
    fn default() -> Self {
        Defaults { hash: default_hash(), frames: default_frames(), bios: default_bios() }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum Check {
    None,
    Jsmolka,
    MgbaSuite,
}

impl Default for Check {
    fn default() -> Self {
        Check::None
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct InputStep {
    pub frame: u32,
    #[serde(default)]
    pub press: Option<String>,
    #[serde(default)]
    pub release: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RomEntry {
    pub path: String,
    #[serde(default)]
    pub check: Check,
    #[serde(default)]
    pub reg: Option<String>,
    #[serde(default)]
    pub golden: Option<String>,
    #[serde(default)]
    pub frames: Option<u32>,
    #[serde(default)]
    pub input: Vec<InputStep>,
    #[serde(default)]
    pub expect: Option<String>,
    #[serde(default)]
    pub skip: Option<String>,
    #[serde(default)]
    pub note: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct GoldenFile {
    #[serde(default)]
    pub defaults: Defaults,
    #[serde(default, rename = "rom")]
    pub roms: Vec<RomEntry>,
}

impl GoldenFile {
    pub fn load(path: &Path) -> anyhow::Result<Self> {
        let text = std::fs::read_to_string(path)?;
        Ok(toml::from_str(&text)?)
    }

    pub fn frames_for(&self, e: &RomEntry) -> u32 {
        e.frames.unwrap_or(self.defaults.frames)
    }
}

/// Render a `[[rom]]` stanza for `--record`.
pub fn stanza(path: &str, check: &Check, frames: u32, hash: u64) -> String {
    let check_s = match check {
        Check::None => String::new(),
        Check::Jsmolka => "check = \"jsmolka\"\n".into(),
        Check::MgbaSuite => "check = \"mgba-suite\"\n".into(),
    };
    format!("[[rom]]\npath = \"{path}\"\n{check_s}frames = {frames}\ngolden = \"{hash:016x}\"\n")
}
