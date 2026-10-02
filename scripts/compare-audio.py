"""Reference audio comparison: our `--pcm` captures against mGBA's, headless.

Usage:
  python scripts/compare-audio.py [--titles kh,minish,...] [--frames 1800] [--out tools/audio-ref]
                                  [--lowpass 8000] [--skip-run]

Needs tools/mgba-wav.exe (scripts/build-mgba.ps1), the ROM zips in Games/ and numpy + scipy.

For every title the script extracts the .gba, converts the input script to the capture tool's
`frame mask` schedule, records three captures from our emulator (full mix, PSG only, FIFO only
via `--mute-channels`) and one from libmgba at the same sample rate, then reports:

- shift: the alignment found by cross-correlating the low-passed mono signals (both emulators
  start from reset with the same inputs, so this should be a few samples at most);
- corr / gain: Pearson correlation and the RMS ratio ours/ref per channel, raw and after the
  low-pass (mGBA's blip_buf resampling is band-limited, our FIFO path is a zero-order hold, so the
  raw figures differ by construction at the top of the band);
- a least-squares fit of the reference on our two components, `ref ~ a*psg + b*fifo`: a and b are
  the gains mGBA applies relative to ours for the PSG and FIFO paths. a == b means a plain scale
  difference; a != b means the PSG/FIFO balance differs;
- a per-second gain table to separate a constant scale from drift or one-off events;
- clipping counts on both sides.

Results go to <out>/report.md and stdout. No reference is "right" in absolute terms (the
hardware's output level is not documented); the point is the relative figures.
"""
import argparse
import glob
import os
import re
import subprocess
import sys
import tomllib
import wave
import zipfile
from pathlib import Path

import numpy as np
from scipy import signal
from scipy.optimize import nnls

REPO = Path(__file__).resolve().parent.parent
KEYS = {"A": 1, "B": 2, "SELECT": 4, "START": 8, "RIGHT": 0x10, "LEFT": 0x20, "UP": 0x40, "DOWN": 0x80, "R": 0x100, "L": 0x200}
PSG_MUTE = 0x0F  # mutes SQ1, SQ2, WAVE, NOISE -> FIFO only
FIFO_MUTE = 0x30  # mutes FIFO A, FIFO B -> PSG only

# slug -> (zip glob, frames, input script or None, note)
TITLES = {
    "kh": ("Kingdom Hearts - Chain of Memories (USA).zip", 1800, None, "FIFO music, SOUNDBIAS 65536 Hz"),
    "kh-game": ("Kingdom Hearts - Chain of Memories (USA).zip", 4000, "tests/inputs/khcom-newgame.toml", "new game: menu sounds + FMV"),
    "minish": ("Legend of Zelda, The - The Minish Cap (USA).zip", 1800, "tests/inputs/triage.toml", "m4a ROM mixer + PSG"),
    "alttp": ("Legend of Zelda, The - A Link to the Past & Four Swords (USA).zip", 1800, "tests/inputs/triage.toml", "m4a + PSG"),
    "mariokart": ("Mario Kart - Super Circuit (USA).zip", 1800, "tests/inputs/triage.toml", "PSG-heavy"),
    "fzero": ("F-Zero - Maximum Velocity (USA, Europe).zip", 1800, "tests/inputs/triage.toml", "PSG-heavy"),
    "advancewars": ("Advance Wars (USA) (Rev 1).zip", 1800, "tests/inputs/triage.toml", "m4a + PSG"),
    "cotm": ("Castlevania - Circle of the Moon (USA).zip", 1800, "tests/inputs/triage.toml", "Konami driver"),
    "goldensun": ("Golden Sun (USA, Europe).zip", 1800, "tests/inputs/triage.toml", "FIFO-heavy"),
    "nes-zelda": ("Classic NES Series - The Legend of Zelda (USA, Europe).zip", 1800, "tests/inputs/triage.toml", "FIFO A from a timer IRQ"),
    "zoids": ("Cyberdrive Zoids - Kijuu no Senshi Hyuu (Japan).zip", 1500, "tests/inputs/triage.toml", "BIOS sound driver: mGBA HLE is silent (sanity row)"),
}


def read_wav(path):
    with wave.open(str(path), "rb") as w:
        rate = w.getframerate()
        ch = w.getnchannels()
        if w.getsampwidth() != 2:
            raise SystemExit(f"{path}: need 16-bit PCM")
        a = np.frombuffer(w.readframes(w.getnframes()), dtype=np.int16).astype(np.float64)
    a = a.reshape(-1, ch)
    if ch == 1:
        a = np.repeat(a, 2, axis=1)
    return rate, a


def lowpass(x, rate, cutoff):
    if cutoff is None or cutoff <= 0 or cutoff >= rate / 2:
        return x
    sos = signal.butter(4, cutoff / (rate / 2), output="sos")
    return signal.sosfiltfilt(sos, x, axis=0)


def best_shift(a, b, rate, max_shift_s=0.5):
    """Lag (samples) of `a` behind `b`: `a[t] ~ b[t - lag]`, so a negative lag means `b` leads."""
    n = min(len(a), len(b))
    x = a[:n] - a[:n].mean()
    y = b[:n] - b[:n].mean()
    corr = signal.fftconvolve(x, y[::-1], mode="full")
    lags = np.arange(-n + 1, n)
    m = int(max_shift_s * rate)
    keep = (lags >= -m) & (lags <= m)
    return int(lags[keep][np.argmax(corr[keep])])


def align(ours, ref, shift):
    """Trim so that ref[i] corresponds to ours[i] (`shift` = `best_shift(ours, ref)`)."""
    if shift > 0:
        ours = ours[shift:]
    elif shift < 0:
        ref = ref[-shift:]
    n = min(len(ours), len(ref))
    return ours[:n], ref[:n]


def rms(x):
    return float(np.sqrt(np.mean(np.square(x)))) if len(x) else 0.0


def pearson(x, y):
    if len(x) < 2 or np.std(x) == 0 or np.std(y) == 0:
        return 0.0
    return float(np.corrcoef(x, y)[0, 1])


def lsq_gain(ours, ref):
    d = float(np.dot(ours, ours))
    return float(np.dot(ours, ref) / d) if d else 0.0


def fit_components(psg, fifo, ref):
    """ref ~ a*psg + b*fifo (least squares over both channels stacked). Sample-exact, so a
    component whose phase differs between the emulators (PSG square waves) fits low."""
    x = np.column_stack([psg.ravel(), fifo.ravel()])
    y = ref.ravel()
    coef, *_ = np.linalg.lstsq(x, y, rcond=None)
    resid = y - x @ coef
    return float(coef[0]), float(coef[1]), rms(resid) / (rms(y) or 1.0)


def envelope(x, rate, win_s=0.02):
    """RMS per `win_s` window, both channels pooled."""
    n = int(rate * win_s)
    m = len(x) // n
    return np.sqrt(np.mean(np.square(x[: m * n]).reshape(m, -1), axis=1))


def power_fit(psg, fifo, ref, rate):
    """ref_power ~ a^2*psg_power + b^2*fifo_power on 20 ms windows (non-negative least squares):
    insensitive to phase, so it is the figure to trust for the PSG/FIFO balance."""
    ep, ef, er = (np.square(envelope(v, rate)) for v in (psg, fifo, ref))
    m = min(len(ep), len(ef), len(er))
    x = np.column_stack([ep[:m], ef[:m]])
    coef, resid = nnls(x, er[:m])
    return float(np.sqrt(coef[0])), float(np.sqrt(coef[1])), float(resid / (np.linalg.norm(er[:m]) or 1.0))


def solo_gain(this, other, ref, rate, floor=50.0, ratio=0.1):
    """RMS gain ref/this over the seconds where `other` is below `ratio` of `this` (one component
    alone), or None when no such second exists."""
    sec = int(min(len(this), len(other), len(ref)) // rate)
    num = den = 0.0
    for i in range(sec):
        sl = slice(i * rate, (i + 1) * rate)
        t, o, r = rms(this[sl]), rms(other[sl]), rms(ref[sl])
        if t > floor and o < ratio * t:
            num += r * r
            den += t * t
    return float(np.sqrt(num / den)) if den else None


def window_lags(a, b, rate, win_s=2.0, max_shift_s=0.5):
    """Best lag of mono `a` behind `b` per window (None where either side is quiet)."""
    n = int(rate * win_s)
    out = []
    for i in range(min(len(a), len(b)) // n):
        x = a[i * n:(i + 1) * n]
        y = b[i * n:(i + 1) * n]
        if rms(x) < 50 or rms(y) < 50:
            out.append(None)
            continue
        out.append(best_shift(x, y, rate, max_shift_s))
    return out


def analyse(ours_path, ref_path, psg_path=None, fifo_path=None, cutoff=8000):
    rate, ours = read_wav(ours_path)
    rate_r, ref = read_wav(ref_path)
    if rate != rate_r:
        raise SystemExit(f"rates differ: {ours_path} {rate} vs {ref_path} {rate_r}")
    ours_lp = lowpass(ours, rate, cutoff)
    ref_lp = lowpass(ref, rate, cutoff)
    shift = best_shift(ours_lp.sum(axis=1), ref_lp.sum(axis=1), rate)
    o, r = align(ours, ref, shift)
    olp, rlp = align(ours_lp, ref_lp, shift)
    out = {"rate": rate, "seconds": len(o) / rate, "shift": shift, "clip_ours": int((np.abs(o) >= 32767).sum()), "clip_ref": int((np.abs(r) >= 32767).sum())}
    lags = [l for l in window_lags(olp.sum(axis=1), rlp.sum(axis=1), rate) if l is not None]
    out["lag_min"], out["lag_med"], out["lag_max"] = (int(min(lags)), int(np.median(lags)), int(max(lags))) if lags else (None, None, None)
    for name, (x, y) in {"raw": (o, r), "lp": (olp, rlp)}.items():
        for ch, label in enumerate("LR"):
            g = rms(x[:, ch]) / (rms(y[:, ch]) or 1.0)
            out[f"{name}_corr_{label}"] = pearson(x[:, ch], y[:, ch])
            out[f"{name}_gain_{label}"] = g
            lg = lsq_gain(x[:, ch], y[:, ch])
            out[f"{name}_resid_{label}"] = rms(y[:, ch] - lg * x[:, ch]) / (rms(y[:, ch]) or 1.0)
    # per-second gain (low-passed, left+right)
    sec = int(len(olp) // rate)
    per_sec = []
    for i in range(sec):
        a = olp[i * rate:(i + 1) * rate]
        b = rlp[i * rate:(i + 1) * rate]
        rb = rms(b)
        per_sec.append(round(rms(a) / rb, 2) if rb > 50 else None)
    out["per_second_gain"] = per_sec
    if psg_path and fifo_path:
        _, psg = read_wav(psg_path)
        _, fifo = read_wav(fifo_path)
        n = min(len(psg), len(fifo), len(ours))
        psg_lp = lowpass(psg[:n], rate, cutoff)
        fifo_lp = lowpass(fifo[:n], rate, cutoff)
        p, _ = align(psg_lp, ref_lp, shift)
        f, rr = align(fifo_lp, ref_lp, shift)
        m = min(len(p), len(f), len(rr))
        out["psg_rms"] = rms(p[:m])
        out["fifo_rms"] = rms(f[:m])
        out["power_fit"] = power_fit(p[:m], f[:m], rr[:m], rate)
        out["psg_solo"] = solo_gain(p[:m], f[:m], rr[:m], rate)
        out["fifo_solo"] = solo_gain(f[:m], p[:m], rr[:m], rate)
        if out["psg_rms"] > 20 and out["fifo_rms"] > 20:
            out["fit"] = fit_components(p[:m], f[:m], rr[:m])
        elif out["fifo_rms"] > 20:
            out["fit"] = (None, lsq_gain(f[:m].ravel(), rr[:m].ravel()), None)
        elif out["psg_rms"] > 20:
            out["fit"] = (lsq_gain(p[:m].ravel(), rr[:m].ravel()), None, None)
        else:
            out["fit"] = (None, None, None)
    return out


def schedule_from_toml(path, out_path):
    """`steps = [{frame, press|release}]` -> `frame mask` lines (mask held from that frame on)."""
    data = tomllib.loads(Path(path).read_text(encoding="utf-8"))
    mask = 0
    lines = []
    for step in data.get("steps", []):
        frame = int(step["frame"])
        if "press" in step:
            mask |= KEYS[step["press"].upper()]
        if "release" in step:
            mask &= ~KEYS[step["release"].upper()]
        lines.append(f"{frame} 0x{mask:03X}")
    Path(out_path).write_text("\n".join(lines) + "\n", encoding="utf-8")


def extract_rom(zip_path, out_path):
    if out_path.exists():
        return
    with zipfile.ZipFile(zip_path) as z:
        name = next(n for n in z.namelist() if n.lower().endswith(".gba"))
        out_path.write_bytes(z.read(name))


def run(cmd, cwd=REPO):
    proc = subprocess.run(cmd, cwd=cwd, capture_output=True, text=True)
    if proc.returncode not in (0, 1):  # gba-test exits 1 on a failed check, not an error
        print(proc.stdout[-2000:], proc.stderr[-2000:], file=sys.stderr)
        raise SystemExit(f"command failed ({proc.returncode}): {' '.join(map(str, cmd))}")
    return proc.stdout


def capture_ours(zip_path, frames, input_toml, wav, mute=None):
    cmd = ["cargo", "run", "--release", "-q", "-p", "gba-test", "--", str(zip_path), "--frames", str(frames), "--pcm", str(wav)]
    if input_toml:
        cmd += ["--input", str(input_toml)]
    if mute is not None:
        cmd += ["--mute-channels", f"{mute:x}"]
    out = run(cmd)
    m = re.search(r"pcm=.* samples=(\d+) rate=(\d+)", out)
    if not m:
        raise SystemExit(f"no pcm line in harness output for {zip_path}:\n{out[-1500:]}")
    return int(m.group(2))


def fmt(v, nd=2):
    return "-" if v is None else f"{v:.{nd}f}"


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    ap.add_argument("--titles", default=",".join(TITLES), help="comma-separated slugs (default: all)")
    ap.add_argument("--frames", type=int, default=None, help="override every title's frame count")
    ap.add_argument("--out", default=str(REPO / "tools" / "audio-ref"))
    ap.add_argument("--lowpass", type=float, default=8000.0, help="Hz; 0 disables")
    ap.add_argument("--skip-run", action="store_true", help="reuse the captures already in --out")
    args = ap.parse_args()
    out = Path(args.out)
    out.mkdir(parents=True, exist_ok=True)
    mgba_wav = REPO / "tools" / "mgba-wav.exe"
    if not args.skip_run and not mgba_wav.exists():
        raise SystemExit("tools/mgba-wav.exe missing: run scripts/build-mgba.ps1")
    cutoff = args.lowpass if args.lowpass > 0 else None

    rows = []
    for slug in args.titles.split(","):
        slug = slug.strip()
        if slug not in TITLES:
            raise SystemExit(f"unknown title {slug}; known: {', '.join(TITLES)}")
        zip_glob, frames, input_toml, note = TITLES[slug]
        frames = args.frames or frames
        zips = glob.glob(str(REPO / "Games" / zip_glob))
        if not zips:
            print(f"{slug}: {zip_glob} not in Games/, skipped")
            continue
        zip_path = Path(zips[0])
        rom = out / f"{slug}.gba"
        ours = out / f"{slug}-ours.wav"
        psg = out / f"{slug}-ours-psg.wav"
        fifo = out / f"{slug}-ours-fifo.wav"
        ref = out / f"{slug}-ref.wav"
        sched = out / f"{slug}.inputs.txt"
        if not args.skip_run:
            print(f"{slug}: capturing {frames} frames", flush=True)
            extract_rom(zip_path, rom)
            sched_arg = []
            if input_toml:
                schedule_from_toml(REPO / input_toml, sched)
                sched_arg = [str(sched)]
            inp = REPO / input_toml if input_toml else None
            rate = capture_ours(zip_path, frames, inp, ours)
            capture_ours(zip_path, frames, inp, psg, mute=FIFO_MUTE)
            capture_ours(zip_path, frames, inp, fifo, mute=PSG_MUTE)
            run([str(mgba_wav), str(rom), str(ref), str(frames), str(rate)] + sched_arg)
        res = analyse(ours, ref, psg, fifo, cutoff)
        res["slug"] = slug
        res["note"] = note
        res["frames"] = frames
        rows.append(res)
        a, b, resid = res.get("fit", (None, None, None))
        pa, pb, presid = res.get("power_fit", (None, None, None))
        print(
            f"{slug}: rate={res['rate']} shift={res['shift']} lags {res['lag_min']}..{res['lag_max']} (med {res['lag_med']}) "
            f"lp corr L/R={fmt(res['lp_corr_L'])}/{fmt(res['lp_corr_R'])} gain L/R={fmt(res['lp_gain_L'])}/{fmt(res['lp_gain_R'])} "
            f"fit psg={fmt(a)} fifo={fmt(b)} | power fit psg={fmt(pa)} fifo={fmt(pb)} resid={fmt(presid)} | "
            f"solo ref/ours psg={fmt(res.get('psg_solo'))} fifo={fmt(res.get('fifo_solo'))} | clip ours/ref={res['clip_ours']}/{res['clip_ref']}",
            flush=True,
        )

    lines = ["# Reference audio comparison (ours vs mGBA 0.10.5 libmgba, HLE BIOS on both sides)", ""]
    lines.append(f"Low-pass for the lp columns: {cutoff or 'none'} Hz. gain = RMS ours / RMS ref. fit: ref ~ a*psg + b*fifo on our component captures.")
    lines.append("")
    lines.append("power fit: ref power ~ a^2*psg + b^2*fifo on 20 ms RMS windows (phase-insensitive; the balance figure to trust). solo: RMS ref/ours over the seconds where only that component sounds. lags: per-2-second alignment range (a spread means drift).")
    lines.append("")
    lines.append("| title | frames | rate | shift | lags min..max | lp corr L/R | lp gain L/R | raw gain L | fit a (PSG) | fit b (FIFO) | power a (PSG) | power b (FIFO) | power resid | solo PSG | solo FIFO | clip ours/ref | note |")
    lines.append("|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|")
    for r in rows:
        a, b, resid = r.get("fit", (None, None, None))
        pa, pb, presid = r.get("power_fit", (None, None, None))
        lines.append(
            f"| {r['slug']} | {r['frames']} | {r['rate']} | {r['shift']} | {r['lag_min']}..{r['lag_max']} | {fmt(r['lp_corr_L'])}/{fmt(r['lp_corr_R'])} | {fmt(r['lp_gain_L'])}/{fmt(r['lp_gain_R'])} "
            f"| {fmt(r['raw_gain_L'])} | {fmt(a)} | {fmt(b)} | {fmt(pa)} | {fmt(pb)} | {fmt(presid)} | {fmt(r.get('psg_solo'))} | {fmt(r.get('fifo_solo'))} | {r['clip_ours']}/{r['clip_ref']} | {r['note']} |"
        )
    lines.append("")
    for r in rows:
        lines.append(f"## {r['slug']}: per-second gain (lp, L+R; '-' = reference below -56 dBFS)")
        lines.append("")
        lines.append(" ".join(fmt(g) for g in r["per_second_gain"]))
        lines.append("")
    (out / "report.md").write_text("\n".join(lines), encoding="utf-8")
    print(f"report: {out / 'report.md'}")


if __name__ == "__main__":
    main()
