# Motion Architecture (Rust port)

## Overview

Everything is a pure function:

```
engine::evaluate(EngineInputs) -> FrameState
```

No mutable accumulators. No frame-driven state. Given the same `EngineInputs`
the engine returns the same `FrameState`, which means scrubbing and offline
rendering are frame-identical to live playback. Time is the only driver.

`EngineInputs` lives in `src/engine.rs`:

```rust
pub struct EngineInputs<'a> {
    pub t: f32,
    pub config: &'a Config,
    pub source: &'a dyn EventSource,
    pub layout: &'a UnitLayout,
    pub row_count: u32,
    pub key_row: u32,
}
```

## The chain

```
EventSource -> Tickers -> PhaseSequences -> PathFn registry -> Bindings -> render::TextRenderer
```

- **EventSource** (`src/audio/event_source.rs`, trait in `src/types.rs`):
  wraps the analysis JSON. Answers `get_events(band)`, `count_before(band, t,
  threshold)`, `event_at_index(band, i, threshold)`,
  `last_before(band, t, threshold)`, `beats_before(t)`, `downbeats_before(t)`.
- **Tickers** (`src/motion/tickers.rs`, `TickerConfig` in `src/types.rs`):
  a ticker is `(id, band, modulo, strength_threshold, offset, step,
  step_offset)`. Events are optionally step-subsampled and strength-filtered,
  then modulo-cycled into a `phase` index.
- **PhaseSequences** (`PhaseSequence { actions: Vec<PhaseAction> }`): an
  ordered list of `PhaseAction`s whose length must equal `ticker.modulo`. Each
  action names a motion path (`String`), its `Params`, `duration` in seconds,
  and an optional `Easing`.
- **PathFn registry** (`src/motion/paths.rs`): a path is
  `fn(&PathContext) -> PathOutput`. Paths emit the delta they contribute at
  the given `progress`. Registered in a `once_cell::Lazy<RwLock<HashMap>>`;
  lookup via `get_path(name)`.
- **Bindings** (`src/motion/bindings.rs`, `Binding` in `src/types.rs`):
  a binding attaches a sequence to a `BindingTarget` (`Rows` or `Camera`)
  with a `RowSelector`, `ComposeMode`, and `respect_alternation` flag.
- **Renderer** (`src/render/text_renderer.rs`): consumes `FrameState.rows[i]`
  and draws, treating `dx`, `dy`, `scale`, `rotation`, `opacity`, `tracking`,
  and `per_glyph` outputs as transforms per row/glyph via glyphon on wgpu/Metal.

## Alternation invariant

Row direction is fixed by index:

```rust
pub enum Dir { Pos, Neg }

impl Dir {
    pub fn for_row(row: u32) -> Dir {
        if row % 2 == 0 { Dir::Pos } else { Dir::Neg }
    }
    pub fn as_f32(self) -> f32 {
        match self { Dir::Pos => 1.0, Dir::Neg => -1.0 }
    }
}
```

Paths are written **direction-agnostic**: they emit positive values and let
the engine sign them. In `src/engine.rs`, `apply_dir()` multiplies the path's
`dx` and `per_glyph.dx` by `dir` when `respect_alternation` is true and the
target is `Rows`:

```rust
fn apply_dir(mut out: PathOutput, dir: Dir, respect_alternation: bool) -> PathOutput {
    if !respect_alternation || matches!(dir, Dir::Pos) { return out; }
    let d = dir.as_f32();
    out.dx *= d;
    if let Some(orig) = out.per_glyph.take() {
        out.per_glyph = Some(Box::new(move |g| {
            let mut r = orig(g); r.dx *= d; r
        }));
    }
    out
}
```

Camera bindings always evaluate with `Dir::Pos` and ignore alternation.

## Continuity model

Output at time `t` is a pure function of past events, not of integration
history.

For a given ticker, let the filtered event times be `t[0] < t[1] < ... < t[K-1]`
with `t[K-1] <= t < t[K]`. Let `duration_i` be the duration of the action
triggered by event `i` (after `feel.duration_scale`).

The total output of the binding at time `t` is:

```
output(t) = sum_{i in 0..=K-2} path_i(frozen_progress_i)
          + path_{K-1}(active_progress)
```

where

```
frozen_progress_i = clamp((t[i+1] - t[i]) / duration_i, 0, 1)
active_progress   = clamp((t - t[K-1]) / duration_{K-1}, 0, 1)
```

Interpretation:

- Each past event is evaluated **once**, at the moment the next event fired
  (cut short if events were tighter than `duration_i`).
- The latest event is evaluated at the actual elapsed time.
- Oscillation paths (`stutter`, `glyphJitter`, `scalePulse`, `trackingPulse`,
  `opacityDip`, `rotationFlicker`) return 0 at `progress = 1`, so their
  frozen contributions vanish: they leave **no residue**.
- Slide-style paths (`slideStep`, `reverseNudge`, `driftSpeed`) return their
  full delta at `progress = 1`, so their frozen contributions **accumulate**.
  Continuous scroll emerges from discrete events.

Implementation lives in `evaluate_binding_contribution()` in `src/engine.rs`.

## How to add a new motion path

Register the path in `src/motion/paths.rs`. The context passed in is
`PathContext` from `src/types.rs`:

| field            | meaning                                                 |
| ---------------- | ------------------------------------------------------- |
| `row_index`      | 0-based row                                             |
| `row_count`      | total rows                                              |
| `dir`            | `Dir::Pos` or `Dir::Neg`                                |
| `progress`       | 0..1 across the active action's `duration`              |
| `eased_progress` | `progress` after the action's easing                    |
| `strength`       | 0..1, event strength that triggered this action         |
| `phase`          | current ticker phase                                    |
| `t`              | global time (seconds)                                   |
| `since_last`     | seconds since the triggering event                      |
| `params`         | binding `Params` (`HashMap<String, ParamValue>`)        |
| `unit_width`     | `layout.stride` (engine-injected)                       |
| `duration`       | effective action duration after `feel.duration_scale`   |
| `canvas_width`   | `config.canvas.width as f32`                            |
| `canvas_height`  | `config.canvas.height as f32`                           |

Return a `PathOutput` with any subset of
`{ dx, dy, scale, rotation, opacity, tracking, per_glyph }` set; defaults are
zero / `None`.

```rust
fn path_my_wiggle(ctx: &PathContext) -> PathOutput {
    let amp = ctx.params.get("amplitude").and_then(|v| v.as_f32()).unwrap_or(5.0);
    PathOutput {
        dy: (ctx.eased_progress * std::f32::consts::PI * 2.0).sin()
            * amp
            * (1.0 - ctx.eased_progress),
        ..Default::default()
    }
}
// then in register_default_paths():
//     register_path("myWiggle", path_my_wiggle);
```

The envelope `(1.0 - ctx.eased_progress)` guarantees the path returns 0 at
`progress = 1`, so it leaves no residue — standard shape for oscillators.

## How to rebind a ticker

Edit `src/config/defaults.rs` and add or modify an entry in the `bindings`
`Vec<Binding>`:

```rust
Binding {
    ticker_id: "kick-even-slide".into(),
    target: BindingTarget::Rows,
    rows: Some(RowSelector::Name(RowSelectorName::KeyRow)),
    sequence: PhaseSequence {
        actions: vec![action(
            "myWiggle",
            params(&[("amplitude", 8.0)]),
            0.3,
            EasingName::EaseOutCubic,
        )],
    },
    compose: ComposeMode::Add,
    respect_alternation: None,
}
```

Rules:

- `sequence.actions.len()` must equal the ticker's `modulo`.
- Each action's `path` must match a name registered in `src/motion/paths.rs`.
- `rows` accepts the selectors listed below.

## Compose modes

Set on the binding; default is `ComposeMode::Add`. Composition for rows is in
`compose_into()` (`src/engine.rs`); camera uses `compose_camera()`.

| mode       | dx/dy/rotation/tracking | scale (delta around 0)         | opacity                      |
| ---------- | ----------------------- | ------------------------------- | ---------------------------- |
| `Add`      | sum                     | `state.scale *= (1 + d)`        | `state.opacity += d` clamped |
| `Multiply` | product                 | `state.scale *= (1 + d)`        | `state.opacity *= (1 + d)`   |
| `Override` | replace                 | `state.scale = 1 + d`           | `state.opacity = 1 + d`      |

`scale` is emitted as a delta around 0 and composed around a row baseline of
1. A path emitting `scale: 0.03` means +3%. `opacity` is also a delta added to
a baseline of 1.

## Params injected by the engine

The engine fills these `PathContext` fields directly — **read them from the
struct, not from `params`**:

| field           | value                                           |
| --------------- | ----------------------------------------------- |
| `unit_width`    | `layout.stride` in pixels (unit span)           |
| `duration`      | the active action's duration after feel scaling |
| `row_count`     | total rows                                      |
| `row_index`     | current row                                     |
| `canvas_width`  | `config.canvas.width as f32`                    |
| `canvas_height` | `config.canvas.height as f32`                   |

A few legacy paths still accept same-named overrides in `params`
(e.g. `path_slide_step` reads `params["unitWidth"]` as a fallback). Prefer the
struct field; treat these keys as reserved in binding params.

## Row selectors

`RowSelector` from `src/types.rs`:

```rust
RowSelector::Name(RowSelectorName::All)       // every row
RowSelector::Name(RowSelectorName::Even)      // i % 2 == 0
RowSelector::Name(RowSelectorName::Odd)       // i % 2 == 1
RowSelector::Name(RowSelectorName::KeyRow)    // only the row holding the key span
RowSelector::Indices(vec![2, 5, 7])           // explicit indices
```

Resolution is in `select_rows()` (`src/motion/bindings.rs`); it returns a
`Vec<bool>` of length `row_count`.

## Per-glyph outputs

A path can set `PathOutput.per_glyph` to a boxed closure:

```rust
per_glyph: Some(Box::new(|glyph_index: i64| PerGlyphOutput {
    dx: /* ... */ 0.0,
    dy: 0.0,
    scale: 0.0,
    rotation: 0.0,
    opacity: 0.0,
})),
```

The engine stacks per-glyph closures (`Running.per_glyph: Vec<Arc<...>>`) and
exposes a composite closure on `RowMotionState.per_glyph` which the renderer
calls per glyph. Rules:

- Prefer **deterministic** functions. Hash `(glyph_index, row_index,
  floor(t * 60))` so results are stable per frame but change smoothly over
  time. `path_glyph_jitter` demonstrates this via `hash01()`.
- `scale` and `opacity` are deltas (added to a baseline of 1 at render time).
- Horizontal components (`per_glyph.dx`) are sign-flipped by `apply_dir()`
  when `respect_alternation` is true.

## Registered paths

All 16 paths registered by `register_default_paths()` in
`src/motion/paths.rs`. The first ten are row paths; the last six target the
camera.

| path                       | one-liner                                                                       |
| -------------------------- | ------------------------------------------------------------------------------- |
| `hold`                     | no-op; rest phase                                                               |
| `slideStep`                | slide `units * unit_width` px; settled delta persists                           |
| `reverseNudge`             | slide back by a fraction of a unit; settled delta persists                      |
| `stutter`                  | damped horizontal oscillation; returns 0 at `progress = 1`                      |
| `glyphJitter`              | per-glyph x/y jitter scaled by strength; no residue                             |
| `driftSpeed`               | integrated constant drift; residue = `pxPerSec * duration`                      |
| `scalePulse`               | sine pulse on scale, baseline 1; no residue                                     |
| `trackingPulse`            | sine pulse on letter spacing; no residue                                        |
| `opacityDip`               | sine dip on row opacity; no residue                                             |
| `rotationFlicker`          | damped rotation oscillation; no residue                                         |
| `cameraZoomDelta`          | settled zoom offset proportional to `deltaScale`                                |
| `cameraPanDelta`           | horizontal camera nudge of `deltaPx`                                            |
| `cameraPanYDelta`          | vertical camera nudge of `deltaPx`                                              |
| `cameraTiltDelta`          | tilt by `deltaDegrees`                                                          |
| `cameraZoomPulse`          | sine zoom pulse of `amplitude * strength`; no residue                           |
| `cameraZoomDeltaFocused`   | zoom + focal pan toward a hashed point bounded by `focusFrac`; residue in both  |

These are plausible placeholders. Replace when motion design is specified.
