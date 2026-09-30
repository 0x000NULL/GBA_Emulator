//! Scripted input: a list of `{frame, press|release}` steps applied before the given frame runs.
//! Files are TOML: `steps = [{ frame = 4, press = "A" }, { frame = 6, release = "A" }]`.
//! Several keys may be combined with `+` ("A+START"). A press stays held until an explicit release.

use crate::golden::InputStep;
use anyhow::{anyhow, Result};
use gba_core::Keys;
use serde::Deserialize;
use std::path::Path;

#[derive(Debug, Default, Deserialize)]
struct ScriptFile {
    #[serde(default)]
    steps: Vec<InputStep>,
}

#[derive(Debug, Clone, Default)]
pub struct InputScript {
    steps: Vec<InputStep>,
}

impl InputScript {
    pub fn from_steps(mut steps: Vec<InputStep>) -> Self {
        steps.sort_by_key(|s| s.frame);
        InputScript { steps }
    }

    pub fn load(path: &Path) -> Result<Self> {
        let text = std::fs::read_to_string(path)?;
        let f: ScriptFile = toml::from_str(&text)?;
        Ok(Self::from_steps(f.steps))
    }

    pub fn is_empty(&self) -> bool {
        self.steps.is_empty()
    }

    pub fn into_steps(self) -> Vec<InputStep> {
        self.steps
    }

    /// Last frame referenced by the script.
    pub fn last_frame(&self) -> u32 {
        self.steps.last().map(|s| s.frame).unwrap_or(0)
    }

    /// Key state that should be held while `frame` runs.
    pub fn keys_at(&self, frame: u32) -> Result<Keys> {
        let mut mask: u16 = 0;
        for s in self.steps.iter().take_while(|s| s.frame <= frame) {
            if let Some(p) = &s.press {
                mask |= parse_keys(p)?;
            }
            if let Some(r) = &s.release {
                mask &= !parse_keys(r)?;
            }
        }
        Ok(Keys(mask))
    }
}

pub fn parse_keys(s: &str) -> Result<u16> {
    let mut m = 0;
    for name in s.split('+') {
        let name = name.trim();
        if name.is_empty() {
            continue;
        }
        m |= Keys::from_name(name).ok_or_else(|| anyhow!("unknown key {name:?}"))?;
    }
    Ok(m)
}
