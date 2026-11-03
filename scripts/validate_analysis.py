#!/usr/bin/env python3
"""
Validate an analysis JSON produced by scripts/analyze.py.

Checks:
  - bpm > 0
  - duration > 0
  - sampleRate in {44100, 48000, 96000, 192000}
  - bands has exactly {kick, snare, mid, hats}
  - each band's events are sorted by t (non-decreasing)
  - each event has t >= 0 and strength in [0, 1]
  - downbeats are a subset of beats (tolerant to float rounding)

Exit code:
  0 on valid, 1 on any error (with a clear bulleted error list on stderr).

Usage:
  python3 scripts/validate_analysis.py analysis/foo.json
"""

from __future__ import annotations

import argparse
import json
import math
import sys
from pathlib import Path
from typing import Any, Dict, List, Optional


ALLOWED_SAMPLE_RATES = {44100, 48000, 96000, 192000}
REQUIRED_BANDS = {"kick", "snare", "mid", "hats"}
# Max absolute time delta (seconds) for a downbeat to match a beat.
# Analyzer rounds beats to 4 decimal places, so 1e-3 is plenty.
DOWNBEAT_TOL = 1e-3


def _is_num(x: Any) -> bool:
    return isinstance(x, (int, float)) and not isinstance(x, bool) and not (
        isinstance(x, float) and (math.isnan(x) or math.isinf(x))
    )


def validate(d: Dict[str, Any]) -> List[str]:
    errors: List[str] = []

    # bpm
    bpm = d.get("bpm")
    if not _is_num(bpm) or bpm <= 0:
        errors.append(f"bpm: expected number > 0, got {bpm!r}")

    # duration
    duration = d.get("duration")
    if not _is_num(duration) or duration <= 0:
        errors.append(f"duration: expected number > 0, got {duration!r}")

    # sampleRate
    sr = d.get("sampleRate")
    if sr not in ALLOWED_SAMPLE_RATES:
        errors.append(
            f"sampleRate: expected one of {sorted(ALLOWED_SAMPLE_RATES)}, got {sr!r}"
        )

    # beats / downbeats
    beats = d.get("beats")
    downbeats = d.get("downbeats")
    if not isinstance(beats, list):
        errors.append(f"beats: expected list, got {type(beats).__name__}")
        beats = []
    if not isinstance(downbeats, list):
        errors.append(f"downbeats: expected list, got {type(downbeats).__name__}")
        downbeats = []

    for i, t in enumerate(beats):
        if not _is_num(t) or t < 0:
            errors.append(f"beats[{i}]: expected number >= 0, got {t!r}")
            break
    for i, t in enumerate(downbeats):
        if not _is_num(t) or t < 0:
            errors.append(f"downbeats[{i}]: expected number >= 0, got {t!r}")
            break

    # downbeats subset of beats (within tolerance)
    if isinstance(beats, list) and isinstance(downbeats, list) and \
            all(_is_num(x) for x in beats) and all(_is_num(x) for x in downbeats):
        beats_sorted = sorted(float(x) for x in beats)
        for i, db in enumerate(downbeats):
            db_f = float(db)
            # binary search for closest beat
            lo, hi = 0, len(beats_sorted)
            if hi == 0:
                errors.append(f"downbeats[{i}]={db_f} not found: beats is empty")
                break
            while lo < hi:
                mid = (lo + hi) // 2
                if beats_sorted[mid] < db_f:
                    lo = mid + 1
                else:
                    hi = mid
            candidates = []
            if lo < len(beats_sorted):
                candidates.append(beats_sorted[lo])
            if lo > 0:
                candidates.append(beats_sorted[lo - 1])
            best = min((abs(c - db_f) for c in candidates), default=float("inf"))
            if best > DOWNBEAT_TOL:
                errors.append(
                    f"downbeats[{i}]={db_f} not in beats (closest delta {best:.6f} > tol {DOWNBEAT_TOL})"
                )

    # bands
    bands = d.get("bands")
    if not isinstance(bands, dict):
        errors.append(f"bands: expected object, got {type(bands).__name__}")
    else:
        present = set(bands.keys())
        missing = REQUIRED_BANDS - present
        extra = present - REQUIRED_BANDS
        if missing:
            errors.append(f"bands: missing required keys {sorted(missing)}")
        if extra:
            errors.append(f"bands: unexpected keys {sorted(extra)}")

        for name in sorted(present & REQUIRED_BANDS):
            events = bands[name]
            if not isinstance(events, list):
                errors.append(f"bands.{name}: expected list, got {type(events).__name__}")
                continue
            last_t: Optional[float] = None
            for i, ev in enumerate(events):
                if not isinstance(ev, dict):
                    errors.append(f"bands.{name}[{i}]: expected object, got {type(ev).__name__}")
                    continue
                t = ev.get("t")
                s = ev.get("strength")
                if not _is_num(t) or t < 0:
                    errors.append(f"bands.{name}[{i}].t: expected number >= 0, got {t!r}")
                else:
                    if last_t is not None and float(t) < float(last_t):
                        errors.append(
                            f"bands.{name}[{i}].t={t} < previous t={last_t} (not sorted)"
                        )
                    last_t = float(t)
                if not _is_num(s) or not (0.0 <= float(s) <= 1.0):
                    errors.append(
                        f"bands.{name}[{i}].strength: expected number in [0,1], got {s!r}"
                    )

    return errors


def parse_args(argv: Optional[List[str]] = None) -> argparse.Namespace:
    p = argparse.ArgumentParser(description="Validate analysis JSON schema")
    p.add_argument("path", help="path to analysis json")
    return p.parse_args(argv)


def main(argv: Optional[List[str]] = None) -> int:
    args = parse_args(argv)
    p = Path(args.path)
    if not p.is_file():
        print(f"error: file not found: {p}", file=sys.stderr)
        return 1
    try:
        with open(p) as f:
            data = json.load(f)
    except json.JSONDecodeError as e:
        print(f"error: invalid JSON: {e}", file=sys.stderr)
        return 1
    except OSError as e:
        print(f"error: cannot read {p}: {e}", file=sys.stderr)
        return 1

    if not isinstance(data, dict):
        print("error: top-level JSON must be an object", file=sys.stderr)
        return 1

    errors = validate(data)
    if errors:
        print(f"invalid analysis: {p}", file=sys.stderr)
        for e in errors:
            print(f"  - {e}", file=sys.stderr)
        return 1

    print(f"ok: {p}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
