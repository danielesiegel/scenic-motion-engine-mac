# Path Reference

Every path registered by `register_default_paths()` in `src/motion/paths.rs`. All paths receive a `PathContext { progress, eased_progress, strength, phase, t, since_last, duration, params, unit_width, canvas_width, canvas_height, row_index, row_count, dir }` and return a `PathOutput` of additive deltas around identity.

"Residue at p=1" means the final value when `eased_progress = 1`. A non-zero residue means the row stays displaced after the action completes (and the next event's phase builds on top of it). "Returns to zero" means the delta ends at identity, like a bell curve. The engine freezes `progress` at the gap to the next event for past events (`continuity model` in `engine.rs`), so residues sum across history until the modulo cycles back.

| # | Path name | Params (default) | Affects | Residue at p=1 |
|---|-----------|------------------|---------|----------------|
| 1 | `hold` | — | nothing | 0 (no-op) |
| 2 | `slideStep` | `units` (1.0), `unitWidth` (ctx.unit_width) | `dx` | `units * unitWidth` (full slide — leaves residue) |
| 3 | `reverseNudge` | `units` (0.5), `unitWidth` (ctx.unit_width) | `dx` | `-units * unitWidth` (leaves residue, opposite dir) |
| 4 | `stutter` | `count` (4), `amplitude` (6.0) | `dx` | 0 (envelope `(1-p) * sin(p*count*2pi)` returns to zero) |
| 5 | `glyphJitter` | `amplitude` (2.0) | `per_glyph.dx`, `per_glyph.dy` | 0 (envelope `(1-p)` kills jitter; seed is per-frame-quantized) |
| 6 | `driftSpeed` | `pxPerSec` (0.0), `duration` (1.0) | `dx` | `pxPerSec * duration` (leaves residue — a drift) |
| 7 | `scalePulse` | `magnitude` (0.03) | `scale` | 0 (bell curve `sin(p*pi)`, scaled by strength) |
| 8 | `trackingPulse` | `magnitude` (4.0) | `tracking` | 0 (bell curve, scaled by strength) |
| 9 | `opacityDip` | `depth` (0.3) | `opacity` (negative) | 0 (bell curve dip to `-depth*strength`, returns) |
| 10 | `rotationFlicker` | `magnitude` (2.0) | `rotation` (degrees) | 0 (envelope `(1-p) * sin(p*pi*6)` returns to zero) |
| 11 | `cameraZoomDelta` | `deltaScale` (0.1) | camera `scale` -> `zoom` | `deltaScale` (leaves residue — a zoom step) |
| 12 | `cameraPanDelta` | `deltaPx` (60.0) | camera `dx` -> `pan_x` | `deltaPx` (leaves residue) |
| 13 | `cameraPanYDelta` | `deltaPx` (40.0) | camera `dy` -> `pan_y` | `deltaPx` (leaves residue) |
| 14 | `cameraTiltDelta` | `deltaDegrees` (10.0) | camera `rotation` -> `tilt` | `deltaDegrees` (leaves residue) |
| 15 | `cameraZoomPulse` | `amplitude` (0.08) | camera `scale` -> `zoom` | 0 (bell curve `sin(p*pi) * amp * (strength or 1)`) |
| 16 | `cameraZoomDeltaFocused` | `deltaScale` (0.1), `focusFrac` (0.6), `canvasWidth` (ctx.canvas_width), `canvasHeight` (ctx.canvas_height) | camera `scale`, `dx`, `dy` | `deltaScale` zoom + offset pan (leaves residue; focus RNG seeded from `floor((t - since_last) * 1000) + 1`) |

## Residue classes

**Full-residue (step/delta) paths** — the engine's continuity model expects these to stack phase-by-phase; use with `modulo` matched to the number of steps per cycle, so residues sum to the intended endpoint before cycling: `slideStep`, `reverseNudge`, `driftSpeed`, `cameraZoomDelta`, `cameraPanDelta`, `cameraPanYDelta`, `cameraTiltDelta`, `cameraZoomDeltaFocused`.

**Zero-residue (pulse) paths** — safe to fire on every event; they return to identity on their own: `stutter`, `glyphJitter`, `scalePulse`, `trackingPulse`, `opacityDip`, `rotationFlicker`, `cameraZoomPulse`.

**No-op**: `hold` (used as a filler action in a `PhaseSequence` so a ticker fires only every N-th event).

## Target compatibility

Paths that touch camera channels (11-16) are intended for `Binding { target: Camera }`. If attached to a `Rows` binding, their outputs still land on the row's `dx` / `dy` / `scale` / `rotation` because the compose code is channel-blind; this is sometimes useful (e.g. `cameraPanDelta` on a `Rows` binding becomes a hard dx step). Conversely, `slideStep` on a Camera binding becomes a pan.

`glyphJitter` is the only path that uses the `per_glyph` closure. The text renderer takes its per-glyph slow path whenever a row has at least one `per_glyph` entry, which drops bulk row shaping; keep this in mind for cost.

## Strength coupling

Paths that multiply by `ctx.strength`: `glyphJitter`, `scalePulse`, `trackingPulse`, `opacityDip`, `rotationFlicker`, `cameraZoomPulse` (treats `strength == 0` as 1.0). All other paths ignore strength and use the raw param values.

## Direction (`dir`) coupling

The engine applies `apply_dir` only when `binding.respect_alternation == true` (default) and the binding targets Rows. For odd rows (`Dir::Neg`), horizontal components are mirrored: `dx *= -1` and `per_glyph.dx *= -1`. Scale, rotation, opacity, tracking, and vertical components are never mirrored. In practice this means `slideStep` and `reverseNudge` alternate direction row-to-row; camera paths and `glyphJitter.dy` never do.
