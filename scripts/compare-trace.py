"""Compare an instruction stream from gba-test with libmgba's, to find where the two emulators part.

Usage:
  python scripts/compare-trace.py <rom.gba> --frames N [--input script.toml] [--out tools/ref]
                                  [--from F] [--every K] [--skip-run]

Runs `gba-test --trace-hash --trace-frames F..N-1` and `tools/mgba-ref.exe --trace-pc F..N-1` (build
it with scripts/build-mgba.ps1), then walks the two `cycle pc` streams side by side:

- repeated PCs are collapsed on both sides (a halted CPU shows up as one PC repeated in mGBA's
  stream and as nothing in ours), so the walk compares the order of executed instructions;
- the first index where the PCs differ is the control-flow divergence, printed with a window of
  context from both streams;
- up to that point, the cycle offset (ours - mGBA) is reported every K instructions and wherever
  it changes, with the instruction that changed it: that is where the two timing models disagree.

The comparison is only meaningful while both run the same code, so it stops at the first
divergence. `--input` takes the usual tests/inputs/*.toml and is converted to mgba-ref's schedule.
"""
import argparse
import subprocess
import sys
import tomllib
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
KEYS = {"A": 1, "B": 2, "SELECT": 4, "START": 8, "RIGHT": 0x10, "LEFT": 0x20, "UP": 0x40, "DOWN": 0x80, "R": 0x100, "L": 0x200}


def schedule_from_toml(path, out_path):
    data = tomllib.loads(Path(path).read_text(encoding="utf-8"))
    mask = 0
    lines = []
    for step in data.get("steps", []):
        if "press" in step:
            mask |= KEYS[step["press"].upper()]
        if "release" in step:
            mask &= ~KEYS[step["release"].upper()]
        lines.append(f"{int(step['frame'])} 0x{mask:03X}")
    Path(out_path).write_text("\n".join(lines) + "\n", encoding="utf-8")


def stream(path):
    """(cycle, pc) pairs with consecutive repeats of a PC collapsed to the first. BIOS addresses are
    dropped: libmgba's HLE BIOS runs a small stub from address 0 at boot and on IRQ entry/exit, where
    ours does the same work natively, so only game code is comparable instruction by instruction
    (the stub's cost still shows up as a step in the cycle offset)."""
    last = None
    with open(path, encoding="ascii") as f:
        for line in f:
            parts = line.split()
            if len(parts) < 2:
                continue
            pc = int(parts[1], 16)
            if pc == last or pc < 0x4000:
                continue
            last = pc
            yield int(parts[0]), pc


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    ap.add_argument("rom")
    ap.add_argument("--frames", type=int, required=True)
    ap.add_argument("--from", dest="start", type=int, default=0, help="first traced frame (0-based)")
    ap.add_argument("--input")
    ap.add_argument("--out", default=str(REPO / "tools" / "ref"))
    ap.add_argument("--every", type=int, default=0, help="also print the offset every K instructions")
    ap.add_argument("--context", type=int, default=12)
    ap.add_argument("--skip-run", action="store_true")
    args = ap.parse_args()
    out = Path(args.out)
    out.mkdir(parents=True, exist_ok=True)
    stem = Path(args.rom).stem
    ours_path = out / f"{stem}.ours.pc"
    ref_path = out / f"{stem}.mgba.pc"
    window = f"{args.start}..{args.frames - 1}"
    if not args.skip_run:
        cmd = ["cargo", "run", "--release", "-q", "-p", "gba-test", "--", args.rom, "--frames", str(args.frames),
               "--trace", str(ours_path), "--trace-hash", "--trace-frames", window]
        ref = [str(REPO / "tools" / "mgba-ref.exe"), args.rom, str(args.frames), "--trace-pc", window, str(ref_path)]
        if args.input:
            cmd += ["--input", args.input]
            sched = out / f"{stem}.schedule.txt"
            schedule_from_toml(args.input, sched)
            ref += ["--input", str(sched)]
        for c in (cmd, ref):
            p = subprocess.run(c, cwd=REPO, capture_output=True, text=True)
            if p.returncode not in (0, 1):
                sys.exit(f"failed: {' '.join(c)}\n{p.stderr[-2000:]}")

    a = stream(ours_path)
    b = stream(ref_path)
    hist_a = []
    hist_b = []
    offset = None
    i = 0
    changes = 0
    for (ca, pa), (cb, pb) in zip(a, b):
        hist_a.append((ca, pa))
        hist_b.append((cb, pb))
        if len(hist_a) > args.context:
            hist_a.pop(0)
            hist_b.pop(0)
        if pa != pb:
            print(f"\ncontrol flow parts at instruction {i}: ours {pa:08X} @ {ca}, mGBA {pb:08X} @ {cb}")
            print(f"{'ours cycle':>12} {'pc':>8}   {'mGBA cycle':>12} {'pc':>8}")
            for (x, y), (u, v) in zip(hist_a, hist_b):
                print(f"{x:>12} {y:08X}   {u:>12} {v:08X}")
            more_a = [next(a, None) for _ in range(args.context)]
            more_b = [next(b, None) for _ in range(args.context)]
            for p, q in zip(more_a, more_b):
                ls = f"{p[0]:>12} {p[1]:08X}" if p else " " * 21
                rs = f"{q[0]:>12} {q[1]:08X}" if q else ""
                print(f"{ls}   {rs}")
            break
        d = ca - cb
        if d != offset:
            if changes < 400:
                prev = hist_a[-2][1] if len(hist_a) > 1 else 0
                print(f"instr {i:>9}  pc {pa:08X} (after {prev:08X})  ours {ca:>10}  mGBA {cb:>10}  offset {d:+d}"
                      + (f" ({d - offset:+d})" if offset is not None else ""))
            changes += 1
            offset = d
        elif args.every and i % args.every == 0:
            print(f"instr {i:>9}  pc {pa:08X}  offset {d:+d}")
        i += 1
    else:
        print(f"\nno divergence in {i} instructions; final offset {offset:+d}; offset changed {changes} times")


if __name__ == "__main__":
    main()
