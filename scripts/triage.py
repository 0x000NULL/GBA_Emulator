#!/usr/bin/env python3
"""Library compatibility sweep.

Runs `gba-test --triage` over every ROM in a folder in parallel, classifies each boot from the
`triage:` summary line, and writes a CSV plus a Markdown report. PNGs of the final frame go next
to the CSV so a category can be eyeballed.

    python scripts/triage.py Games --out tools/triage --jobs 8 --frames 2400
    python scripts/triage.py Games --out tools/triage --only "Zelda"      # substring filter
    python scripts/triage.py --report tools/triage/triage.csv              # re-render the report only

Categories (in order of checking):
  crash      the harness exited with an error (panic or unreadable ROM)
  black      the final picture has at most 2 colours and the picture never changed (after boot)
  frozen     the picture stopped changing more than 1200 frames before the end and the CPU is idle
             or spinning (fewer than 1000 instructions in the last 60 frames, or halted with IE=0)
  static     a picture is shown but it did not change in the last 1200 frames (title waiting for
             a key the pattern does not press, or a real hang with a busy CPU)
  silent     the picture moves but no audio sample above the noise floor was produced
  ok         picture with colour and motion, audio present
"""
import argparse
import csv
import os
import re
import subprocess
import sys
import time
from concurrent.futures import ThreadPoolExecutor, as_completed
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
EXE = ROOT / "target" / "me" / "release" / "gba-test.exe"


def run_one(rom: Path, out_dir: Path, frames: int, exe: Path) -> dict:
    stem = rom.stem
    png = out_dir / "png" / (stem + ".png")
    png.parent.mkdir(parents=True, exist_ok=True)
    cmd = [str(exe), str(rom), "--frames", str(frames), "--hle", "--triage", "--png", str(png)]
    t0 = time.time()
    try:
        p = subprocess.run(cmd, capture_output=True, text=True, timeout=600, cwd=ROOT)
    except subprocess.TimeoutExpired:
        return {"rom": rom.name, "category": "crash", "note": "timeout", "secs": round(time.time() - t0, 1)}
    row = {"rom": rom.name, "secs": round(time.time() - t0, 1), "exit": p.returncode}
    m = re.search(r"^rom: (.*)$", p.stdout, re.M)
    if m:
        for kv in re.findall(r"(\w+)=(\"[^\"]*\"|\S+)", m.group(1)):
            row[kv[0]] = kv[1].strip('"')
    m = re.search(r"^triage: (.*)$", p.stdout, re.M)
    if m:
        for kv in re.findall(r"(\w+)=(\S+)", m.group(1)):
            row[kv[0]] = kv[1]
    m = re.search(r"^unimplemented_swi=(.*)$", p.stdout, re.M)
    row["unimplemented_swi"] = m.group(1) if m else ""
    if p.returncode not in (0, 1) or "triage:" not in p.stdout:
        err = (p.stderr.strip().splitlines() or [""])[-1]
        row["category"] = "crash"
        row["note"] = err[:200] or "no triage line"
        return row
    row["category"] = classify(row, frames)
    return row


def classify(r: dict, frames: int) -> str:
    try:
        colours = int(r.get("colours", 0))
        changes = int(r.get("changes", 0))
        last_change = int(r.get("last_change", 0))
        audio = int(r.get("audio_frames", 0))
        instr = int(r.get("instr_last_60", 0))
        halted = r.get("halted") == "true"
        ie = int(r.get("ie", "0"), 16)
    except ValueError:
        return "crash"
    if r.get("exit", 0) not in (0, 1):
        return "crash"
    if colours <= 2 and changes <= 2:
        return "black"
    idle = instr < 1000 or (halted and ie == 0)
    stale = last_change < frames - 1200
    if stale and idle:
        return "frozen"
    if stale:
        return "static"
    if audio == 0:
        return "silent"
    return "ok"


def needs_bios_driver(r: dict) -> bool:
    """Titles whose only audio path is the BIOS sound driver (SWIs 1A-1E, 28-2A): a known HLE gap."""
    swi = r.get("unimplemented_swi", "") or ""
    return r.get("category") == "silent" and any(k in swi for k in ("1A:", "1B:", "1C:", "1D:", "1E:", "2A:"))


def write_doc(rows, path: Path, when: str):
    """The committed per-title compatibility list (docs/library-compat.md)."""
    n = len(rows)
    for r in rows:
        if needs_bios_driver(r):
            r["category"] = "needs-bios-sound-driver"
    cats = ["ok", "silent", "needs-bios-sound-driver", "static", "frozen", "black", "crash"]
    counts = {c: sum(1 for r in rows if r.get("category") == c) for c in cats}
    lines = [f"# Library compatibility ({n} titles, {when})", ""]
    lines.append("Boot sweep by `scripts/triage.py` (harness `gba-test --triage`): every ROM runs 2400 frames (40 s) under the")
    lines.append("HLE BIOS with Start pressed at frames 700/1100/1500/1900/2300 and A at 1300/2100, then the final picture, its")
    lines.append("motion, the audio and the CPU state are classified. `ok` = a moving, coloured picture with audio. This is a boot")
    lines.append("and title/menu check, not a play-through; the scripted gameplay goldens live in `tests/compat.toml`.")
    lines.append("")
    lines.append("| Category | Titles | Meaning |")
    lines.append("|---|---|---|")
    meaning = {
        "ok": "moving picture with colour and audio",
        "silent": "moving picture, no audio in the first 40 s (see the per-title note)",
        "needs-bios-sound-driver": "runs, but its music needs the BIOS sound driver (SWI 1A-1E/2A), which the HLE BIOS does not provide",
        "static": "a picture that stopped changing before the last 20 s",
        "frozen": "picture stopped and the CPU is idle or spinning",
        "black": "no picture at all",
        "crash": "the harness failed (panic / unreadable ROM)",
    }
    for c in cats:
        lines.append(f"| {c} | {counts[c]} | {meaning[c]} |")
    lines.append("")
    lines.append("| Title | Code | Save | Result | Note |")
    lines.append("|---|---|---|---|---|")
    for r in sorted(rows, key=lambda r: r["rom"].lower()):
        name = r["rom"][:-4] if r["rom"].lower().endswith(".zip") else r["rom"]
        note = r.get("note", "") or ""
        if note == "no triage line" and r.get("category") != "crash":
            note = ""
        lines.append(f"| {name} | {r.get('code','')} | {r.get('save','')} | {r.get('category','')} | {note} |")
    lines.append("")
    path.write_text("\n".join(lines), encoding="utf-8")


def write_report(rows, out_dir: Path):
    cats = ["ok", "silent", "static", "frozen", "black", "crash"]
    by = {c: [r for r in rows if r.get("category") == c] for c in cats}
    lines = [f"# Library triage ({len(rows)} ROMs)", ""]
    lines.append("| Category | Count |")
    lines.append("|---|---|")
    for c in cats:
        lines.append(f"| {c} | {len(by[c])} |")
    lines.append("")
    for c in cats[1:]:
        if not by[c]:
            continue
        lines.append(f"## {c} ({len(by[c])})")
        lines.append("")
        lines.append("| ROM | code | save | colours | changes | last change | audio | instr/60f | pc | halted | IE | note |")
        lines.append("|---|---|---|---|---|---|---|---|---|---|---|---|")
        for r in sorted(by[c], key=lambda r: r["rom"]):
            lines.append(
                f"| {r['rom']} | {r.get('code','')} | {r.get('save','')} | {r.get('colours','')} | {r.get('changes','')} | "
                f"{r.get('last_change','')} | {r.get('audio_frames','')} | {r.get('instr_last_60','')} | {r.get('pc','')} {r.get('region','')} | "
                f"{r.get('halted','')} | {r.get('ie','')} | {r.get('note','') or r.get('unimplemented_swi','')} |"
            )
        lines.append("")
    (out_dir / "triage.md").write_text("\n".join(lines), encoding="utf-8")
    return by


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("folder", nargs="?", default="Games")
    ap.add_argument("--out", default="tools/triage")
    ap.add_argument("--jobs", type=int, default=max(1, (os.cpu_count() or 4) - 2))
    ap.add_argument("--frames", type=int, default=2400)
    ap.add_argument("--only", default=None, help="substring filter on the file name")
    ap.add_argument("--exe", default=str(EXE))
    ap.add_argument("--report", default=None, help="CSV to re-render into a report")
    ap.add_argument("--doc", default=None, help="also write the committed per-title list to this Markdown path")
    ap.add_argument("--notes", default=None, help="CSV of `rom,note` lines merged into the doc (hand-written verdicts)")
    args = ap.parse_args()
    out_dir = (ROOT / args.out).resolve()
    out_dir.mkdir(parents=True, exist_ok=True)

    if args.report:
        with open(args.report, newline="", encoding="utf-8") as f:
            rows = list(csv.DictReader(f))
        for r in rows:
            r["exit"] = int(r.get("exit") or 0)
        by = write_report(rows, out_dir)
        print({c: len(v) for c, v in by.items()})
        if args.doc:
            if args.notes:
                with open(args.notes, newline="", encoding="utf-8") as f:
                    notes = {n["rom"]: n["note"] for n in csv.DictReader(f)}
                for r in rows:
                    if r["rom"] in notes:
                        r["note"] = notes[r["rom"]]
            write_doc(rows, Path(args.doc), time.strftime("%Y-%m-%d"))
            print(f"doc: {args.doc}")
        return

    roms = sorted(p for p in (ROOT / args.folder).iterdir() if p.suffix.lower() in (".zip", ".gba"))
    if args.only:
        roms = [r for r in roms if args.only.lower() in r.name.lower()]
    print(f"{len(roms)} ROMs, {args.jobs} jobs, {args.frames} frames each")
    rows = []
    t0 = time.time()
    with ThreadPoolExecutor(max_workers=args.jobs) as ex:
        futs = {ex.submit(run_one, r, out_dir, args.frames, Path(args.exe)): r for r in roms}
        for i, fut in enumerate(as_completed(futs), 1):
            row = fut.result()
            rows.append(row)
            if i % 50 == 0 or i == len(roms):
                done = time.time() - t0
                print(f"  {i}/{len(roms)}  {done:.0f}s  last: {row['rom'][:50]} -> {row['category']}")
    fields = sorted({k for r in rows for k in r})
    with open(out_dir / "triage.csv", "w", newline="", encoding="utf-8") as f:
        w = csv.DictWriter(f, fieldnames=fields)
        w.writeheader()
        for r in sorted(rows, key=lambda r: r["rom"]):
            w.writerow(r)
    by = write_report(rows, out_dir)
    print({c: len(v) for c, v in by.items()})
    print(f"report: {out_dir / 'triage.md'}")


if __name__ == "__main__":
    main()
