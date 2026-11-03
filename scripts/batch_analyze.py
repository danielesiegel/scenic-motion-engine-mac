#!/usr/bin/env python3
"""
Batch analyzer wrapper around scripts/analyze.py.

Walks a glob of .wav files, runs the analyzer against each, writes per-track
analysis JSON + diagnostic PNG into ./analysis/, prints a summary table.

Skips tracks whose analysis JSON is newer than the audio file unless --force.

Usage:
    python3 scripts/batch_analyze.py "assets/audio/*.wav"
    python3 scripts/batch_analyze.py "assets/audio/*.wav" --force --fps 60
"""

from __future__ import annotations

import argparse
import glob
import json
import os
import subprocess
import sys
import time
from pathlib import Path
from typing import List, Optional, Tuple


SCRIPT_DIR = Path(__file__).resolve().parent
ANALYZE_PY = SCRIPT_DIR / "analyze.py"


def parse_args(argv: Optional[List[str]] = None) -> argparse.Namespace:
    p = argparse.ArgumentParser(description="Batch audio analyzer")
    p.add_argument("glob", help="glob pattern for input .wav files (quote it)")
    p.add_argument("--analysis-dir", default="analysis",
                   help="output directory for json/png (relative to CWD)")
    p.add_argument("--force", action="store_true",
                   help="re-analyze even if analysis JSON is newer than audio")
    p.add_argument("--fps", type=int, default=60)
    p.add_argument("--stems", action="store_true", help="pass --stems to analyze.py")
    p.add_argument("--python", default=sys.executable,
                   help="python interpreter to run analyze.py with")
    p.add_argument("--extra", nargs=argparse.REMAINDER, default=[],
                   help="forward remaining args verbatim to analyze.py (prefix with --)")
    return p.parse_args(argv)


def needs_rerun(wav: Path, json_out: Path, force: bool) -> bool:
    if force:
        return True
    if not json_out.exists():
        return True
    try:
        return wav.stat().st_mtime > json_out.stat().st_mtime
    except OSError:
        return True


def run_one(python: str, wav: Path, out_json: Path, out_plot: Path,
            fps: int, stems: bool, extra: List[str]) -> Tuple[int, float, str]:
    """Run analyze.py on one file. Return (rc, elapsed_s, stdout_tail)."""
    cmd = [
        python, str(ANALYZE_PY),
        "--input", str(wav),
        "--output", str(out_json),
        "--plot", str(out_plot),
        "--fps", str(fps),
    ]
    if stems:
        cmd.append("--stems")
    if extra:
        # Drop a leading "--" sentinel if argparse REMAINDER captured it
        if extra and extra[0] == "--":
            extra = extra[1:]
        cmd.extend(extra)

    t0 = time.monotonic()
    try:
        res = subprocess.run(cmd, capture_output=True, text=True)
    except FileNotFoundError as e:
        return 127, 0.0, f"spawn failed: {e}"
    elapsed = time.monotonic() - t0
    tail = (res.stdout or "").strip().splitlines()
    tail_line = tail[-1] if tail else ""
    if res.returncode != 0:
        err = (res.stderr or "").strip().splitlines()
        err_line = err[-1] if err else ""
        return res.returncode, elapsed, err_line or tail_line
    return 0, elapsed, tail_line


def summarize_from_json(path: Path) -> dict:
    try:
        with open(path) as f:
            d = json.load(f)
    except Exception:
        return {}
    bands = d.get("bands") or {}
    return {
        "bpm": d.get("bpm"),
        "duration": d.get("duration"),
        "sampleRate": d.get("sampleRate"),
        "kick": len(bands.get("kick") or []),
        "snare": len(bands.get("snare") or []),
        "mid": len(bands.get("mid") or []),
        "hats": len(bands.get("hats") or []),
    }


def print_table(rows: List[dict]) -> None:
    if not rows:
        print("no files processed")
        return
    cols = ["track", "status", "elapsed", "bpm", "duration", "sr",
            "kick", "snare", "mid", "hats"]
    widths = {c: len(c) for c in cols}
    for r in rows:
        for c in cols:
            widths[c] = max(widths[c], len(str(r.get(c, ""))))
    sep = "  "
    header = sep.join(c.ljust(widths[c]) for c in cols)
    print(header)
    print(sep.join("-" * widths[c] for c in cols))
    for r in rows:
        print(sep.join(str(r.get(c, "")).ljust(widths[c]) for c in cols))


def main(argv: Optional[List[str]] = None) -> int:
    args = parse_args(argv)

    if not ANALYZE_PY.is_file():
        print(f"error: analyzer not found at {ANALYZE_PY}", file=sys.stderr)
        return 2

    wavs = sorted(Path(p).resolve() for p in glob.glob(args.glob))
    if not wavs:
        print(f"warning: no files matched {args.glob!r}", file=sys.stderr)
        return 0

    analysis_dir = Path(args.analysis_dir).resolve()
    analysis_dir.mkdir(parents=True, exist_ok=True)

    rows: List[dict] = []
    overall_rc = 0
    for wav in wavs:
        base = wav.stem
        out_json = analysis_dir / f"{base}.json"
        out_plot = analysis_dir / f"{base}.png"

        if not needs_rerun(wav, out_json, args.force):
            info = summarize_from_json(out_json)
            rows.append({
                "track": base,
                "status": "skip",
                "elapsed": "-",
                "bpm": info.get("bpm", "?"),
                "duration": info.get("duration", "?"),
                "sr": info.get("sampleRate", "?"),
                "kick": info.get("kick", "?"),
                "snare": info.get("snare", "?"),
                "mid": info.get("mid", "?"),
                "hats": info.get("hats", "?"),
            })
            continue

        print(f"[run ] {wav.name}", flush=True)
        rc, elapsed, tail = run_one(
            args.python, wav, out_json, out_plot,
            args.fps, args.stems, args.extra or [],
        )
        if rc != 0:
            overall_rc = max(overall_rc, 1)
            print(f"[fail] {wav.name}: rc={rc} {tail}", file=sys.stderr)
            rows.append({
                "track": base,
                "status": f"FAIL({rc})",
                "elapsed": f"{elapsed:.1f}s",
                "bpm": "-", "duration": "-", "sr": "-",
                "kick": "-", "snare": "-", "mid": "-", "hats": "-",
            })
            continue

        info = summarize_from_json(out_json)
        rows.append({
            "track": base,
            "status": "ok",
            "elapsed": f"{elapsed:.1f}s",
            "bpm": info.get("bpm", "?"),
            "duration": info.get("duration", "?"),
            "sr": info.get("sampleRate", "?"),
            "kick": info.get("kick", "?"),
            "snare": info.get("snare", "?"),
            "mid": info.get("mid", "?"),
            "hats": info.get("hats", "?"),
        })

    print()
    print_table(rows)
    return overall_rc


if __name__ == "__main__":
    sys.exit(main())
