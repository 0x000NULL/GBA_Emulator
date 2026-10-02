"""Compare one emulator PCM capture against one reference recording.

Usage: python scripts/compare-wav.py ours.wav reference.wav [--lowpass 8000] [--tolerance 0.15]

Both files: 16-bit PCM WAV, stereo or mono, the same sample rate (`gba-test --pcm` writes the
SOUNDBIAS rate, 32768 Hz or 65536 Hz; give `mgba-wav` the rate the `pcm=` line reports). The
reference is aligned by cross-correlating the low-passed mono signals, then per channel the
Pearson correlation, the RMS gain ours/ref and the residual after a least-squares gain match are
printed; exit code 0 when both channels' residual is within the tolerance.

The reference normally comes from `tools/mgba-wav.exe` (scripts/build-mgba.ps1), e.g.
  cargo run --release -p gba-test -- <rom.zip> --frames 1800 --pcm ours.wav
  tools/mgba-wav.exe <rom.gba> ref.wav 1800 <rate> [inputs.txt]
scripts/compare-audio.py drives both for a whole title set, including PSG/FIFO component fits.
A manual mGBA GUI recording (File > Record output, WAV preset, matching sample rate) works too.
"""
import argparse
import importlib.util
from pathlib import Path


def load_analysis():
    spec = importlib.util.spec_from_file_location("compare_audio", Path(__file__).with_name("compare-audio.py"))
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    return mod


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    ap.add_argument("ours")
    ap.add_argument("reference")
    ap.add_argument("--lowpass", type=float, default=8000.0, help="Hz for the lp figures; 0 disables")
    ap.add_argument("--tolerance", type=float, default=0.15, help="max lp residual (fraction of the reference RMS)")
    args = ap.parse_args()
    ca = load_analysis()
    r = ca.analyse(args.ours, args.reference, cutoff=args.lowpass if args.lowpass > 0 else None)
    print(f"rate={r['rate']} overlap={r['seconds']:.1f}s shift={r['shift']} samples clip ours/ref={r['clip_ours']}/{r['clip_ref']}")
    for ch in "LR":
        print(
            f"  {ch}: lp corr={r[f'lp_corr_{ch}']:.3f} gain={r[f'lp_gain_{ch}']:.3f} resid={r[f'lp_resid_{ch}']:.3f} | "
            f"raw corr={r[f'raw_corr_{ch}']:.3f} gain={r[f'raw_gain_{ch}']:.3f} resid={r[f'raw_resid_{ch}']:.3f}"
        )
    ok = all(r[f"lp_resid_{ch}"] <= args.tolerance for ch in "LR")
    print("within tolerance" if ok else f"outside tolerance {args.tolerance}")
    raise SystemExit(0 if ok else 1)


if __name__ == "__main__":
    main()
