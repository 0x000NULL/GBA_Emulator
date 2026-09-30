"""Compare an emulator PCM capture against a reference recording (M7 gate, reference half).

Usage: python scripts/compare-wav.py ours.wav reference.wav [--tolerance 0.15]

Both files: 16-bit PCM WAV, stereo or mono, any rate (resampled to the lower rate). The reference
is aligned to ours by cross-correlating the first 10 s of the left channel, then the normalised
RMS difference over the overlap is reported; exit code 0 when it is within tolerance.

Producing the reference with the portable mGBA (tools/mgba/mGBA-0.10.5-win64/mGBA.exe):
  1. File > Load ROM (the .gba extracted from Games/*.zip; mGBA also opens .zip).
  2. Settings > Audio: sample rate 65536 for Kingdom Hearts (it sets SOUNDBIAS to 65536 Hz).
  3. File > Record output, choose the WAV/PCM preset, record the same span (e.g. title screen).
  4. Capture ours: cargo run -p gba-test -- <rom> --frames 1800 --pcm ours.wav
"""
import struct
import sys
import wave

import array


def read_wav(path):
    with wave.open(path, "rb") as w:
        rate = w.getframerate()
        ch = w.getnchannels()
        width = w.getsampwidth()
        frames = w.readframes(w.getnframes())
    if width != 2:
        raise SystemExit(f"{path}: need 16-bit PCM, got {width * 8}-bit")
    a = array.array("h")
    a.frombytes(frames)
    left = a[0::ch]
    return rate, [float(v) for v in left]


def resample(x, from_rate, to_rate):
    if from_rate == to_rate:
        return x
    n = int(len(x) * to_rate / from_rate)
    step = from_rate / to_rate
    return [x[min(int(i * step), len(x) - 1)] for i in range(n)]


def best_offset(a, b, window, max_shift):
    # Brute-force normalised cross-correlation on a decimated window (fast enough for 10 s).
    dec = 16
    aw = a[:window:dec]
    bw = b[:window:dec]
    best, best_shift = -2.0, 0
    for shift in range(-max_shift // dec, max_shift // dec):
        if shift >= 0:
            x, y = aw[shift:], bw[: len(aw) - shift]
        else:
            x, y = aw[: len(aw) + shift], bw[-shift:]
        n = min(len(x), len(y))
        if n < 1000:
            continue
        x, y = x[:n], y[:n]
        sx = sum(v * v for v in x) ** 0.5 or 1.0
        sy = sum(v * v for v in y) ** 0.5 or 1.0
        c = sum(p * q for p, q in zip(x, y)) / (sx * sy)
        if c > best:
            best, best_shift = c, shift * dec
    return best_shift, best


def main():
    args = [a for a in sys.argv[1:] if not a.startswith("--")]
    tol = 0.15
    if "--tolerance" in sys.argv:
        tol = float(sys.argv[sys.argv.index("--tolerance") + 1])
    if len(args) != 2:
        raise SystemExit(__doc__)
    ra, a = read_wav(args[0])
    rb, b = read_wav(args[1])
    rate = min(ra, rb)
    a = resample(a, ra, rate)
    b = resample(b, rb, rate)
    shift, corr = best_offset(a, b, window=rate * 10, max_shift=rate * 2)
    if shift >= 0:
        a, b = a[shift:], b
    else:
        a, b = a, b[-shift:]
    n = min(len(a), len(b))
    a, b = a[:n], b[:n]
    ref_rms = (sum(v * v for v in b) / n) ** 0.5 or 1.0
    diff_rms = (sum((p - q) ** 2 for p, q in zip(a, b)) / n) ** 0.5
    ratio = diff_rms / ref_rms
    print(f"rate={rate} overlap={n / rate:.2f}s shift={shift} samples corr={corr:.3f} diff/ref RMS={ratio:.3f} (tolerance {tol})")
    sys.exit(0 if ratio <= tol else 1)


if __name__ == "__main__":
    main()
