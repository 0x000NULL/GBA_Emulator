//! Headless harness CLI.
//!
//! gba-test <rom|zip> --frames N [--golden HEX] [--check jsmolka|mgba-suite] [--reg r12]
//!          [--input script.toml] [--trace [file]] [--trace-hash] [--break PC] [--watch LO..HI]
//!          [--exit-on-swi N] [--dump-io] [--png out.png] [--pcm out.wav] [--record] [--bios file]
//! Exit code: 0 pass, 1 fail, 2 error.

use anyhow::{anyhow, Result};
use clap::Parser;
use gba_core::bus::debug::Hit;
use gba_core::{BiosMode, Gba, StopReason};
use gba_test::checks::{check_jsmolka, check_mgba_suite, CheckResult};
use gba_test::golden::Check;
use gba_test::input_script::InputScript;
use gba_test::{build_gba, cpu_state_line, fb_hash, write_png, write_wav};
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::PathBuf;
use std::process::ExitCode;

#[derive(Parser, Debug)]
#[command(name = "gba-test", about = "Run a GBA ROM headless and report framebuffer hash + CPU state")]
struct Cli {
    /// ROM (.gba) or zip containing one.
    rom: PathBuf,
    /// Frames to run.
    #[arg(long, default_value_t = 16)]
    frames: u32,
    /// Expected framebuffer hash (16 hex digits).
    #[arg(long)]
    golden: Option<String>,
    /// Test-suite pass/fail convention to apply after the run.
    #[arg(long, value_enum)]
    check: Option<CheckArg>,
    /// Register holding the jsmolka result (default r12).
    #[arg(long)]
    reg: Option<String>,
    /// Expected `p/t` for mgba-suite.
    #[arg(long)]
    expect: Option<String>,
    /// Input script TOML.
    #[arg(long)]
    input: Option<PathBuf>,
    /// Per-instruction trace to stdout, or to a file if given.
    #[arg(long, num_args = 0..=1, default_missing_value = "-")]
    trace: Option<String>,
    /// Compact `cycle pc hash` trace instead of full registers.
    #[arg(long)]
    trace_hash: bool,
    /// Stop when the CPU is about to execute this PC (hex).
    #[arg(long = "break", value_parser = parse_hex)]
    break_pc: Vec<u32>,
    /// Stop on a write inside LO..HI (hex, inclusive).
    #[arg(long, value_parser = parse_range)]
    watch: Vec<(u32, u32)>,
    /// Stop when this SWI number executes (hex).
    #[arg(long, value_parser = parse_hex)]
    exit_on_swi: Option<u32>,
    /// Print the IO register dump at the end.
    #[arg(long)]
    dump_io: bool,
    /// Write the final framebuffer as PNG.
    #[arg(long)]
    png: Option<PathBuf>,
    /// Write captured audio as WAV (32768 Hz stereo).
    #[arg(long)]
    pcm: Option<PathBuf>,
    /// Print a golden.toml stanza for this run.
    #[arg(long)]
    record: bool,
    /// Report FIFO underruns and audio samples counted from this frame onward (skips boot silence).
    #[arg(long)]
    stats_from: Option<u32>,
    /// BIOS image (default: GBA_BIOS env, else HLE).
    #[arg(long)]
    bios: Option<PathBuf>,
    /// Force HLE BIOS even if GBA_BIOS is set.
    #[arg(long)]
    hle: bool,
    /// Load a savestate (written by --save-state or the frontend's F1) before running.
    #[arg(long)]
    load_state: Option<PathBuf>,
    /// Write a savestate after the run.
    #[arg(long)]
    save_state: Option<PathBuf>,
}

#[derive(clap::ValueEnum, Clone, Debug)]
enum CheckArg {
    None,
    Jsmolka,
    MgbaSuite,
}

fn parse_hex(s: &str) -> Result<u32, String> {
    let t = s.trim_start_matches("0x").trim_start_matches("0X");
    u32::from_str_radix(t, 16).map_err(|e| e.to_string())
}

fn parse_range(s: &str) -> Result<(u32, u32), String> {
    let (a, b) = s.split_once("..").ok_or("expected LO..HI")?;
    Ok((parse_hex(a)?, parse_hex(b)?))
}

fn main() -> ExitCode {
    match run() {
        Ok(true) => ExitCode::from(0),
        Ok(false) => ExitCode::from(1),
        Err(e) => {
            eprintln!("error: {e:#}");
            ExitCode::from(2)
        }
    }
}

fn run() -> Result<bool> {
    let cli = Cli::parse();
    let bios = if cli.hle {
        Some(BiosMode::Hle)
    } else {
        cli.bios.as_ref().map(|p| std::fs::read(p).map(BiosMode::Lle)).transpose()?
    };
    let mut gba = build_gba(&cli.rom, bios, None)?;
    {
        let h = &gba.bus.cart.header;
        println!(
            "rom: title={:?} code={} maker={} rom_size={} save={:?} mirror={} bios={}",
            h.title,
            h.game_code_str(),
            String::from_utf8_lossy(&h.maker_code),
            gba.bus.cart.rom().len(),
            gba.bus.cart.save_type,
            gba.bus.cart.mirror,
            if gba.bus.bios.is_hle() { "hle" } else { "lle" }
        );
    }
    let script = match &cli.input {
        Some(p) => InputScript::load(p)?,
        None => InputScript::default(),
    };
    if let Some(p) = &cli.load_state {
        let bytes = std::fs::read(p)?;
        gba.load_state(&bytes).map_err(|e| anyhow!("{e}"))?;
        println!("state loaded: {} (frame {})", p.display(), gba.bus.ppu.frame_count);
    }

    gba.bus.debug.pc_breaks = cli.break_pc.clone();
    gba.bus.debug.write_watch = cli.watch.clone();
    gba.bus.debug.refresh_enabled();

    let mut trace_out: Option<Box<dyn Write>> = match cli.trace.as_deref() {
        Some("-") => Some(Box::new(BufWriter::new(std::io::stdout()))),
        Some(path) => Some(Box::new(BufWriter::new(File::create(path)?))),
        None => None,
    };

    let mut log = Vec::new();
    let mut audio = Vec::new();
    let mut stopped: Option<Hit> = None;
    let mut stats_base: Option<(u64, u64)> = None;
    let tracing = trace_out.is_some() || cli.exit_on_swi.is_some();

    'frames: for f in 0..cli.frames {
        if !script.is_empty() {
            gba.set_keys(script.keys_at(f)?);
        }
        if tracing {
            // Step instruction by instruction until the frame completes.
            let frame0 = gba.bus.ppu.frame_count;
            while gba.bus.ppu.frame_count == frame0 {
                if let Some(out) = trace_out.as_mut() {
                    if !gba.bus.irq.halted && gba.bus.dma.pending == 0 {
                        if cli.trace_hash {
                            writeln!(out, "{} {:08X} {:016x}", gba.bus.sched.now, gba.cpu.pc(), gba.cpu.state_hash())?;
                        } else {
                            writeln!(out, "{}", gba.cpu.trace_line())?;
                        }
                    }
                }
                if let Some(n) = cli.exit_on_swi {
                    if is_swi(&gba, n) {
                        break 'frames;
                    }
                }
                let target = gba.bus.sched.now + 1;
                if let StopReason::Breakpoint(h) = gba.run_until(target) {
                    stopped = Some(h);
                    break 'frames;
                }
            }
            gba.bus.ppu.take_frame_ready();
        } else {
            match gba.run_frame() {
                StopReason::Done => {}
                StopReason::Breakpoint(h) => {
                    stopped = Some(h);
                    break 'frames;
                }
            }
        }
        log.append(&mut gba.take_debug_log());
        if cli.pcm.is_some() {
            gba.drain_audio(&mut audio);
        } else {
            gba.bus.apu.samples.clear();
        }
        if cli.stats_from == Some(f) {
            stats_base = Some((gba.bus.apu.fifo_a.underruns + gba.bus.apu.fifo_b.underruns, gba.bus.apu.sample_count));
        }
    }
    if let (Some(from), Some((u0, s0))) = (cli.stats_from, stats_base) {
        let u1 = gba.bus.apu.fifo_a.underruns + gba.bus.apu.fifo_b.underruns;
        println!("stats_from={from} underruns={} samples={}", u1 - u0, gba.bus.apu.sample_count - s0);
    }
    if let Some(out) = trace_out.as_mut() {
        out.flush()?;
    }

    if let Some(p) = &cli.save_state {
        std::fs::write(p, gba.save_state())?;
        println!("state saved: {}", p.display());
    }
    let hash = fb_hash(&gba.bus.ppu.fb[..]);
    println!("fb_hash={hash:016x}");
    println!("{}", cpu_state_line(&gba));
    println!("frames={} vcount={} swi_count={} fifo_underruns={}", gba.bus.ppu.frame_count, gba.bus.ppu.vcount, gba.bus.bios.swi_count, gba.bus.apu.fifo_a.underruns + gba.bus.apu.fifo_b.underruns);
    if !gba.bus.bios.unimplemented.is_empty() {
        let names: Vec<String> = gba.bus.bios.unimplemented.iter().map(|&n| format!("{:02X}:{}", n, gba_core::bios::swi::name(n))).collect();
        println!("unimplemented_swi={}", names.join(","));
    }
    for l in &log {
        println!("guest: {l}");
    }
    if let Some(h) = &stopped {
        println!("stopped: {h:?}");
    }
    if cli.dump_io {
        print!("{}", gba.bus.dump_io());
    }
    if let Some(p) = &cli.png {
        write_png(p, &gba.bus.ppu.fb[..])?;
        println!("png={}", p.display());
    }
    if let Some(p) = &cli.pcm {
        // Output rate follows SOUNDBIAS (32768 Hz default; KH:CoM selects 65536 Hz).
        let rate = (gba_core::timing::CLOCK_HZ / gba.bus.apu.cycles_per_sample()) as u32;
        write_wav(p, &audio, rate)?;
        println!("pcm={} samples={} rate={}", p.display(), audio.len(), rate);
    }

    let mut ok = true;
    if let Some(g) = &cli.golden {
        let want = u64::from_str_radix(g.trim().trim_start_matches("0x"), 16).map_err(|_| anyhow!("bad golden {g:?}"))?;
        let pass = want == hash;
        println!("golden: {}", if pass { "MATCH" } else { "MISMATCH" });
        ok &= pass;
    }
    let check = match cli.check {
        Some(CheckArg::Jsmolka) => check_jsmolka(&mut gba, cli.reg.as_deref().unwrap_or("r12")),
        Some(CheckArg::MgbaSuite) => check_mgba_suite(&log, cli.expect.as_deref()),
        _ => CheckResult::NotChecked,
    };
    match &check {
        CheckResult::NotChecked => {}
        CheckResult::Pass(m) => println!("check: PASS ({m})"),
        CheckResult::Fail(m) => {
            println!("check: FAIL ({m})");
            ok = false;
        }
    }
    if cli.record {
        let c = match cli.check {
            Some(CheckArg::Jsmolka) => Check::Jsmolka,
            Some(CheckArg::MgbaSuite) => Check::MgbaSuite,
            _ => Check::None,
        };
        print!("{}", gba_test::golden::stanza(&cli.rom.to_string_lossy(), &c, cli.frames, hash));
    }
    Ok(ok && stopped.is_none())
}

/// True if the instruction at PC is `swi n` (ARM or Thumb encoding).
fn is_swi(gba: &Gba, n: u32) -> bool {
    let pc = gba.cpu.pc();
    if gba.cpu.regs.thumb() {
        let op = gba.bus.peek16(pc & !1);
        op & 0xFF00 == 0xDF00 && (op & 0xFF) as u32 == n
    } else {
        let op = gba.bus.peek32(pc & !3);
        op & 0x0F00_0000 == 0x0F00_0000 && ((op >> 16) & 0xFF) == n
    }
}
