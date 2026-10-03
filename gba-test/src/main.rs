//! Headless harness CLI.
//!
//! gba-test <rom|zip> --frames N [--golden HEX] [--check jsmolka|mgba-suite] [--reg r12]
//!          [--input script.toml] [--trace [file]] [--trace-hash] [--trace-frames A..B] [--trace-cycles] [--break PC] [--watch LO..HI]
//!          [--watch-read LO..HI] [--watch-log] [--dump-mem LO..HI] [--log-swi] [--exit-on-swi N] [--dump-io] [--png out.png] [--png-every N] [--pcm out.wav] [--mute-channels MASK] [--record]
//!          [--bios file] [--hle] [--stats-from F] [--fifo-stats F] [--save-state f] [--load-state f]
//!          [--triage] [--png-at F,F,...] [--regs file]
//! Exit code: 0 pass, 1 fail, 2 error.
//!
//! `--triage` is the library sweep mode: a built-in input pattern (Start / A presses after boot)
//! and one `triage:` summary line at the end (picture activity, checkpoint hashes, CPU state,
//! audio activity) that `scripts/triage.py` classifies.

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
#[command(name = "gba-test", about = "Strawberry's headless harness: run a GBA ROM and report framebuffer hash + CPU state")]
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
    /// Trace only frames A..B (0-based, inclusive); the others run at full speed.
    #[arg(long, value_parser = parse_dec_range)]
    trace_frames: Option<(u32, u32)>,
    /// Prefix every full trace line with the cycle count (the layout of tools/mgba-ref.exe --trace).
    #[arg(long)]
    trace_cycles: bool,
    /// Step every instruction and print `poll frame cycle pc addr old new` whenever one of these
    /// halfwords changes (hex, comma-separated; the layout of tools/mgba-ref.exe --poll).
    #[arg(long, value_delimiter = ',', value_parser = parse_hex)]
    poll: Vec<u32>,
    /// Write `<png stem>-<frame>.png` after each of these frames (1-based, comma-separated).
    #[arg(long, value_delimiter = ',')]
    png_at: Vec<u32>,
    /// After every frame write `frame cycle r0..r14 pc cpsr` and key IO halfwords (the layout of
    /// tools/mgba-ref.exe --regs; pc as in the trace: PC + 8 / + 4).
    #[arg(long)]
    regs: Option<PathBuf>,
    /// Stop when the CPU is about to execute this PC (hex).
    #[arg(long = "break", value_parser = parse_hex)]
    break_pc: Vec<u32>,
    /// Stop on a write inside LO..HI (hex, inclusive).
    #[arg(long, value_parser = parse_range)]
    watch: Vec<(u32, u32)>,
    /// Stop on a read inside LO..HI (hex, inclusive); with --watch-log every read is logged.
    #[arg(long, value_parser = parse_range)]
    watch_read: Vec<(u32, u32)>,
    /// Log every --watch / --break hit (frame, cycle, PC, access) and keep running instead of stopping.
    #[arg(long)]
    watch_log: bool,
    /// Log every HLE SWI call (number, name, r0-r3, PC, scanline, cycle) as a `guest:` line.
    #[arg(long)]
    log_swi: bool,
    /// Stop when this SWI number executes (hex).
    #[arg(long, value_parser = parse_hex)]
    exit_on_swi: Option<u32>,
    /// Print the IO register dump at the end.
    #[arg(long)]
    dump_io: bool,
    /// Hex-dump guest memory LO..HI (hex, as 32-bit words, side-effect free) at the end.
    #[arg(long, value_parser = parse_range)]
    dump_mem: Vec<(u32, u32)>,
    /// Write the final framebuffer as PNG.
    #[arg(long)]
    png: Option<PathBuf>,
    /// Also write a PNG every N frames as `<png stem>-<frame>.png` (stem defaults to `frame`).
    #[arg(long, value_name = "N")]
    png_every: Option<u32>,
    /// Write captured audio as WAV (stereo at the SOUNDBIAS sample rate).
    #[arg(long)]
    pcm: Option<PathBuf>,
    /// Silence APU channels in the mix (hex mask: 1 SQ1, 2 SQ2, 4 WAVE, 8 NOISE, 10 FIFO A, 20 FIFO B);
    /// attaches the bench capture, which does not change emulation.
    #[arg(long, value_parser = parse_hex)]
    mute_channels: Option<u32>,
    /// Print a golden.toml stanza for this run.
    #[arg(long)]
    record: bool,
    /// Report FIFO underruns and audio samples counted from this frame onward (skips boot silence).
    #[arg(long)]
    stats_from: Option<u32>,
    /// From this frame on, print per-frame FIFO A/B pushed bytes, popped samples and underruns.
    #[arg(long)]
    fifo_stats: Option<u32>,
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
    /// Library sweep: built-in Start/A presses (unless --input is given) and a `triage:` summary.
    #[arg(long)]
    triage: bool,
    /// Cartridge clock at boot, `YYYY-MM-DD HH:MM[:SS]` (default 2026-01-01 00:00:00).
    #[arg(long, value_parser = gba_test::parse_rtc)]
    rtc: Option<i64>,
    /// Light on a solar-sensor cart (Boktai), 0 dark .. 10 full gauge; scripts can change it with `sun` steps.
    #[arg(long, value_name = "LEVEL")]
    sun: Option<u8>,
}

/// Triage input: Start for 10 frames in pairs 100 frames apart (700/800, 1300/1400, 1900/2000,
/// 2500/2600) and A for 10 frames at 1150 and 2350. A pair gets through "demo -> title -> game"
/// on titles whose attract loop returns to a short title screen (the Famicom Mini series), and
/// the pairs sit at different phases of a ~400-frame loop so a demo cannot swallow every press.
pub fn triage_keys(frame: u32) -> gba_core::Keys {
    let start = [700, 800, 1300, 1400, 1900, 2000, 2500, 2600].iter().any(|&f| (f..f + 10).contains(&frame));
    let a = [1150, 2350].iter().any(|&f| (f..f + 10).contains(&frame));
    let mut k = 0;
    if start {
        k |= gba_core::Keys::START;
    }
    if a {
        k |= gba_core::Keys::A;
    }
    gba_core::Keys(k)
}

/// What the triage summary accumulates per frame.
#[derive(Default)]
struct Triage {
    changes: u32,
    last_change: u32,
    last_hash: u64,
    checkpoints: Vec<(u32, u64)>,
    audio_frames: u32,
    instr_at_minus_60: u64,
    instr_last_60: u64,
}

fn pc_region(pc: u32) -> &'static str {
    match pc >> 24 {
        0x00 => "bios",
        0x02 => "ewram",
        0x03 => "iwram",
        0x08..=0x0D => "rom",
        _ => "other",
    }
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

fn parse_dec_range(s: &str) -> Result<(u32, u32), String> {
    let (a, b) = s.split_once("..").ok_or("expected A..B")?;
    Ok((a.parse().map_err(|e| format!("{e}"))?, b.parse().map_err(|e| format!("{e}"))?))
}

/// IO halfwords in a `--regs` line, the same list as tools/mgba-ref.exe: DISPCNT, DISPSTAT, VCOUNT,
/// TM0-3 counter and control, DMA0-3 CNT_H, IE, IF, IME, KEYINPUT.
const REGS_IO: [u32; 19] = [
    0x04000000, 0x04000004, 0x04000006, 0x04000100, 0x04000102, 0x04000104, 0x04000106, 0x04000108, 0x0400010A,
    0x0400010C, 0x0400010E, 0x040000BA, 0x040000C6, 0x040000D2, 0x040000DE, 0x04000200, 0x04000202, 0x04000208,
    0x04000130,
];

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
    if let Some(mask) = cli.mute_channels {
        gba.set_bench(true);
        gba.set_channel_mute(mask as u8);
    }
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
        if let Some(g) = &gba.bus.cart.gpio {
            println!("gpio: {:?}", g.devices());
        }
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
    // After a state load, so the flags win over the state's clock and light.
    if let Some(t) = cli.rtc {
        gba.set_rtc_clock(t);
    }
    if let Some(level) = cli.sun {
        gba.set_sun_level(level);
    }

    gba.bus.debug.pc_breaks = cli.break_pc.clone();
    gba.bus.debug.write_watch = cli.watch.clone();
    gba.bus.debug.read_watch = cli.watch_read.clone();
    gba.bus.debug.log_swi = cli.log_swi;
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
    let mut fifo_prev: Option<[u64; 6]> = None;
    let mut poll_prev: Vec<u16> = cli.poll.iter().map(|&a| gba.bus.peek16(a)).collect();
    let mut regs_out = match &cli.regs {
        Some(p) => Some(BufWriter::new(File::create(p)?)),
        None => None,
    };

    let mut triage = Triage::default();
    'frames: for f in 0..cli.frames {
        if !script.is_empty() {
            gba.set_keys(script.keys_at(f)?);
            if let Some(level) = script.sun_at(f) {
                gba.set_sun_level(level);
            }
        } else if cli.triage {
            gba.set_keys(triage_keys(f));
        }
        if cli.triage && f + 60 == cli.frames {
            triage.instr_at_minus_60 = gba.cpu.instr_count;
        }
        let in_window = cli.trace_frames.is_none_or(|(a, b)| (a..=b).contains(&f));
        let tracing = (trace_out.is_some() && in_window) || cli.exit_on_swi.is_some() || !cli.poll.is_empty();
        if tracing {
            // Step instruction by instruction until the frame completes.
            let frame0 = gba.bus.ppu.frame_count;
            while gba.bus.ppu.frame_count == frame0 {
                if let Some(out) = trace_out.as_mut().filter(|_| in_window) {
                    if !gba.bus.irq.halted && gba.bus.dma.pending == 0 {
                        if cli.trace_hash {
                            writeln!(out, "{} {:08X} {:016x}", gba.bus.sched.now, gba.cpu.pc(), gba.cpu.state_hash())?;
                        } else if cli.trace_cycles {
                            writeln!(out, "{} {}", gba.bus.sched.now, gba.cpu.trace_line())?;
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
                for (p, prev) in cli.poll.iter().zip(poll_prev.iter_mut()) {
                    let v = gba.bus.peek16(*p);
                    if v != *prev {
                        let pc = gba.cpu.regs.r[15].wrapping_add(if gba.cpu.regs.thumb() { 4 } else { 8 });
                        println!("poll {f} {} {pc:08X} {p:08X} {:04X} {v:04X}", gba.bus.sched.now, *prev);
                        *prev = v;
                    }
                }
            }
            gba.bus.ppu.take_frame_ready();
        } else {
            loop {
                match gba.run_frame() {
                    StopReason::Done => break,
                    StopReason::Breakpoint(h) if cli.watch_log => {
                        println!("hit frame={} line={} cycle={} pc={:08X} {:?}", f, gba.bus.ppu.vcount, gba.bus.sched.now, gba.cpu.pc(), h);
                    }
                    StopReason::Breakpoint(h) => {
                        stopped = Some(h);
                        break 'frames;
                    }
                }
            }
        }
        log.append(&mut gba.take_debug_log());
        if cli.triage {
            let h = fb_hash(&gba.bus.ppu.fb[..]);
            if h != triage.last_hash {
                if f > 0 {
                    triage.changes += 1;
                }
                triage.last_change = f;
                triage.last_hash = h;
            }
            if [600, 1200, 1800, 2400, 3000].contains(&(f + 1)) {
                triage.checkpoints.push((f + 1, h));
            }
            if gba_test::audio_active(&gba.bus.apu.samples) {
                triage.audio_frames += 1;
            }
        }
        if cli.png_at.contains(&(f + 1)) {
            let stem = cli.png.clone().unwrap_or_else(|| PathBuf::from("frame.png")).with_extension("");
            let path = PathBuf::from(format!("{}-{}.png", stem.display(), f + 1));
            write_png(&path, &gba.bus.ppu.fb[..])?;
            println!("png={} frame={}", path.display(), f + 1);
        }
        if let Some(out) = regs_out.as_mut() {
            let r = &gba.cpu.regs;
            let mut line = format!("{} {}", f + 1, gba.bus.sched.now);
            for v in &r.r[..15] {
                line.push_str(&format!(" {v:08X}"));
            }
            line.push_str(&format!(" {:08X} {:08X}", r.r[15].wrapping_add(if r.thumb() { 4 } else { 8 }), r.cpsr));
            for &a in &REGS_IO {
                line.push_str(&format!(" {:04X}", gba.bus.peek16(a)));
            }
            writeln!(out, "{line}")?;
        }
        if let Some(n) = cli.png_every {
            if n > 0 && (f + 1) % n == 0 {
                let stem = cli.png.clone().unwrap_or_else(|| PathBuf::from("frame.png")).with_extension("");
                let path = PathBuf::from(format!("{}-{:05}.png", stem.display(), f + 1));
                write_png(&path, &gba.bus.ppu.fb[..])?;
                println!("png={} frame={}", path.display(), f + 1);
            }
        }
        if cli.pcm.is_some() {
            gba.drain_audio(&mut audio);
        } else {
            gba.bus.apu.samples.clear();
        }
        if cli.stats_from == Some(f) {
            stats_base = Some((gba.bus.apu.fifo_a.underruns + gba.bus.apu.fifo_b.underruns, gba.bus.apu.sample_count));
        }
        if cli.fifo_stats.is_some_and(|from| f >= from) {
            let (a, b) = (&gba.bus.apu.fifo_a, &gba.bus.apu.fifo_b);
            let d = |cur: u64, prev: &mut u64| { let v = cur - *prev; *prev = cur; v };
            let st = fifo_prev.get_or_insert([0; 6]);
            let dma = &gba.bus.dma;
            println!(
                "fifo frame={f} A: pushed={} popped={} underruns={} len={} | B: pushed={} popped={} underruns={} len={} | dma1 ctl={:04X} dst={:08X} src={:08X} rem={} prog={} dma2 ctl={:04X} dst={:08X} pending={:02X} soundcnt_h={:04X}",
                d(a.pushed, &mut st[0]), d(a.popped, &mut st[1]), d(a.underruns, &mut st[2]), a.len(),
                d(b.pushed, &mut st[3]), d(b.popped, &mut st[4]), d(b.underruns, &mut st[5]), b.len(),
                dma.ch[1].cnt_h, dma.ch[1].dst, dma.ch[1].src, dma.ch[1].remaining, dma.ch[1].in_progress,
                dma.ch[2].cnt_h, dma.ch[2].dst, dma.pending, gba.bus.apu.read_io8(0x82) as u16 | (gba.bus.apu.read_io8(0x83) as u16) << 8
            );
        }
    }
    if let (Some(from), Some((u0, s0))) = (cli.stats_from, stats_base) {
        let u1 = gba.bus.apu.fifo_a.underruns + gba.bus.apu.fifo_b.underruns;
        println!("stats_from={from} underruns={} samples={}", u1 - u0, gba.bus.apu.sample_count - s0);
    }
    if let Some(out) = trace_out.as_mut() {
        out.flush()?;
    }
    if let Some(out) = regs_out.as_mut() {
        out.flush()?;
    }

    if let Some(p) = &cli.save_state {
        std::fs::write(p, gba.save_state())?;
        println!("state saved: {}", p.display());
    }
    let hash = fb_hash(&gba.bus.ppu.fb[..]);
    println!("fb_hash={hash:016x}");
    if cli.triage {
        triage.instr_last_60 = gba.cpu.instr_count.saturating_sub(triage.instr_at_minus_60);
        let mut seen = vec![false; 0x8000];
        let mut colours = 0u32;
        for &px in gba.bus.ppu.fb.iter() {
            let i = (px & 0x7FFF) as usize;
            if !seen[i] {
                seen[i] = true;
                colours += 1;
            }
        }
        let dispcnt = gba.bus.ppu.dispcnt();
        let cps: Vec<String> = triage.checkpoints.iter().map(|(f, h)| format!("{f}:{h:016x}")).collect();
        println!(
            "triage: colours={} changes={} last_change={} checkpoints={} audio_frames={} instr_last_60={} pc={:08X} region={} thumb={} halted={} ime={} ie={:04X} dispcnt={:04X} forced_blank={} mode={} unimpl_swi={} underruns={}",
            colours,
            triage.changes,
            triage.last_change,
            cps.join(","),
            triage.audio_frames,
            triage.instr_last_60,
            gba.cpu.pc(),
            pc_region(gba.cpu.pc()),
            gba.cpu.regs.thumb(),
            gba.bus.irq.halted,
            gba.bus.irq.ime,
            gba.bus.irq.ie,
            dispcnt,
            dispcnt & 0x80 != 0,
            dispcnt & 7,
            gba.bus.bios.unimplemented.len(),
            gba.bus.apu.fifo_a.underruns + gba.bus.apu.fifo_b.underruns
        );
    }
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
    for &(lo, hi) in &cli.dump_mem {
        let mut a = lo & !3;
        while a <= hi {
            println!("mem {a:08X}: {:08X}", gba.bus.peek32(a));
            a = a.wrapping_add(4);
            if a == 0 {
                break;
            }
        }
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
