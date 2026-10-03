//! Pass/fail strategies for the test suites, and the shared "run one golden entry" routine.
//!
//! jsmolka/gba-tests (verified from `lib/macros.inc` `m_test_eval` and each ROM's `m_exit`):
//! every ROM ends in `idle: b idle` after `m_test_eval`, which does `movs r12, reg`; r12 == 0 means
//! all tests passed, otherwise r12 is the number of the first failed test (thumb.gba uses r7 as the
//! source, but r12 receives the copy). On failure the digits are also stored at 0x03000000/4/8.
//!
//! mgba-emu/suite: menu-driven (A runs the highlighted suite). Results are reported through the
//! mGBA debug registers as `END: passes/total` (and to SRAM via savprintf).

use crate::golden::{Check, GoldenFile, RomEntry};
use crate::input_script::InputScript;
use crate::{build_gba, cpu_state_line, fb_hash};
use anyhow::{anyhow, Result};
use gba_core::{BiosMode, Gba, StopReason};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct RunOutcome {
    pub frames: u32,
    pub hash: u64,
    pub cpu_state: String,
    pub debug_log: Vec<String>,
    pub check: CheckResult,
    pub golden_ok: Option<bool>,
    /// Frames whose audio output had a sample above +-64 (`gba_test::audio_active`).
    pub audio_frames: u32,
    /// `audio_frames >= audio_frames_min` when the entry sets a minimum.
    pub audio_ok: Option<bool>,
}

impl RunOutcome {
    pub fn passed(&self) -> bool {
        self.check.passed() && self.golden_ok != Some(false) && self.audio_ok != Some(false)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CheckResult {
    NotChecked,
    Pass(String),
    Fail(String),
}

impl CheckResult {
    pub fn passed(&self) -> bool {
        !matches!(self, CheckResult::Fail(_))
    }
}

/// Resolve a golden.toml path: absolute, or relative to `GBA_TESTS_DIR`, or relative to the repo root.
pub fn resolve_rom_path(p: &str) -> Result<PathBuf> {
    let path = Path::new(p);
    if path.is_absolute() && path.exists() {
        return Ok(path.to_path_buf());
    }
    if let Some(dir) = crate::tests_dir() {
        let c = dir.join(path);
        if c.exists() {
            return Ok(c);
        }
    }
    let c = crate::repo_root().join(path);
    if c.exists() {
        return Ok(c);
    }
    Err(anyhow!("ROM not found: {p} (GBA_TESTS_DIR={:?})", crate::tests_dir()))
}

/// Run `gba` for `frames` frames applying `script`, then evaluate the requested check.
pub fn run_and_check(gba: &mut Gba, frames: u32, script: &InputScript, check: &Check, reg: Option<&str>, expect: Option<&str>) -> Result<RunOutcome> {
    let mut log = Vec::new();
    let mut audio_frames = 0;
    for f in 0..frames {
        if !script.is_empty() {
            gba.set_keys(script.keys_at(f)?);
            if let Some(level) = script.sun_at(f) {
                gba.set_sun_level(level);
            }
        }
        match gba.run_frame() {
            StopReason::Done => {}
            StopReason::Breakpoint(h) => return Err(anyhow!("unexpected breakpoint {h:?} at frame {f}")),
        }
        if crate::audio_active(&gba.bus.apu.samples) {
            audio_frames += 1;
        }
        gba.bus.apu.samples.clear();
        log.append(&mut gba.take_debug_log());
    }
    let check = match check {
        Check::None => CheckResult::NotChecked,
        Check::Jsmolka => check_jsmolka(gba, reg.unwrap_or("r12")),
        Check::MgbaSuite => check_mgba_suite(&log, expect),
    };
    Ok(RunOutcome {
        frames,
        hash: fb_hash(&gba.bus.ppu.fb[..]),
        cpu_state: cpu_state_line(gba),
        debug_log: log,
        check,
        golden_ok: None,
        audio_frames,
        audio_ok: None,
    })
}

/// jsmolka: the CPU must be spinning on a branch-to-self and the result register must be 0.
pub fn check_jsmolka(gba: &mut Gba, reg: &str) -> CheckResult {
    let idx: usize = match reg.trim_start_matches('r').parse() {
        Ok(i) if i < 16 => i,
        _ => return CheckResult::Fail(format!("bad register name {reg}")),
    };
    // Spin check: PC must not move over a few hundred more instructions.
    let pc0 = gba.cpu.pc();
    let instr0 = gba.cpu.instr_count;
    gba.run_cycles(4000);
    let pc1 = gba.cpu.pc();
    let spinning = pc1 == pc0 && gba.cpu.instr_count > instr0;
    let opcode = gba.bus.peek32(pc0 & !3);
    let is_b_self = if gba.cpu.regs.thumb() {
        gba.bus.peek16(pc0 & !1) == 0xE7FE
    } else {
        opcode == 0xEAFF_FFFE
    };
    if !spinning || !is_b_self {
        return CheckResult::Fail(format!(
            "CPU not in the end-of-test spin loop (pc {pc0:08X} -> {pc1:08X}, opcode {opcode:08X}); {} tests pending",
            gba.cpu.regs.r[idx]
        ));
    }
    let v = gba.cpu.regs.r[idx];
    if v == 0 {
        CheckResult::Pass("all tests passed".into())
    } else {
        CheckResult::Fail(format!("failed test {v}"))
    }
}

/// mgba-suite: find `END: p/t` in the debug log and compare with `expect`.
pub fn check_mgba_suite(log: &[String], expect: Option<&str>) -> CheckResult {
    let end = log.iter().rev().find_map(|l| l.split("END: ").nth(1).map(|s| s.trim().to_string()));
    match end {
        None => CheckResult::Fail("no END: line in debug log (suite not run or debug regs unsupported)".into()),
        Some(res) => match expect {
            Some(e) if e.trim() == res => CheckResult::Pass(res),
            Some(e) => CheckResult::Fail(format!("got {res}, expected {e}")),
            None => {
                let mut it = res.split('/');
                let p: u32 = it.next().and_then(|s| s.parse().ok()).unwrap_or(0);
                let t: u32 = it.next().and_then(|s| s.parse().ok()).unwrap_or(1);
                if p == t {
                    CheckResult::Pass(res)
                } else {
                    CheckResult::Fail(format!("{res} passed"))
                }
            }
        },
    }
}

/// Run one `[[rom]]` entry from golden.toml.
pub fn run_entry(file: &GoldenFile, e: &RomEntry) -> Result<RunOutcome> {
    let path = resolve_rom_path(&e.path)?;
    let bios = if file.defaults.bios == "hle" { Some(BiosMode::Hle) } else { None };
    let mut gba = build_gba(&path, bios, None)?;
    let mut steps = e.input.clone();
    if let Some(f) = &e.input_file {
        let p = crate::repo_root().join(f);
        steps.extend(InputScript::load(&p)?.into_steps());
    }
    let script = InputScript::from_steps(steps);
    let frames = file.frames_for(e);
    let mut out = run_and_check(&mut gba, frames, &script, &e.check, e.reg.as_deref(), e.expect.as_deref())?;
    if let Some(g) = &e.golden {
        let want = u64::from_str_radix(g.trim(), 16).map_err(|_| anyhow!("bad golden hash {g:?}"))?;
        out.golden_ok = Some(want == out.hash);
    }
    if let Some(min) = e.audio_frames_min {
        out.audio_ok = Some(out.audio_frames >= min);
    }
    Ok(out)
}
