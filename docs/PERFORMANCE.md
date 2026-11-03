# Performance Notes

This document captures measured performance of the Rust engine hot path and
calls out the optimizations currently in flight plus the known remaining
hot spots. Numbers come from the Criterion benches under `benches/`.

## Measurement Environment

Captured 2026-11-02:

```
ProductName:            macOS
ProductVersion:         15.6.1
BuildVersion:           24G90

CPU: Apple M1

rustc 1.94.1 (e408947bf 2026-03-25)
```

Benches were run with the default release profile via
`cargo bench --bench engine`. The engine bench exercises
`evaluate(EngineInputs)` with the default config, a `SyntheticEventSource`,
10 rows, and the shaped `scenicsynth` layout at 72pt.

## Current Numbers

Each row is the full cost of one `evaluate` call at the given `t`. The third
column shows that cost as a fraction of a 60 fps frame (16,667 microseconds).

| Metric            | ns/op       | ns/frame budget @ 60fps |
|-------------------|-------------|--------------------------|
| evaluate_t_start  |  20,299 ns  | 0.12% of 16,667,000 ns  |
| evaluate_t_mid    | 528,330 ns  | 3.17% of 16,667,000 ns  |
| evaluate_t_end    | 1,040,300 ns| 6.24% of 16,667,000 ns  |

`t_start` is the baseline cost at `t = 0.1` (few/no past events).
`t_mid` is `t = 180.0` (~360 kicks accumulated at 120 BPM).
`t_end` is `t = 360.0` (~720 kicks accumulated at 120 BPM).

### Target Budget

A 60 fps frame has a 16.67 ms (16,667 microseconds) budget. The engine's
baseline evaluation lands well under 1% of that frame budget, which leaves
plenty of headroom for rendering, text rasterization, audio, and any
scheduling jitter. Even the worst-case `t_end` sits at ~6% of the frame,
and in practice we never run the engine at `t = 360.0` on every frame — it
is a stress scenario that measures accumulated-event replay cost rather
than steady-state usage.

The take-away: the pure-engine hot path is not the bottleneck. Attention
should flow to the rendering pipeline (see Hot Spots below).

## Known Optimizations In Place

The following optimizations are already applied in the current codebase and
are reflected in the numbers above:

- **FilterCache.** The per-event filter contributions are cached so that
  repeated calls to `evaluate` at nearby `t` values do not re-run the
  filter kernel for events whose contribution is unchanged. This is the
  dominant speedup for the `t_mid` and `t_end` cases, where hundreds of
  past events would otherwise be re-filtered every frame.
- **Fused filter.** The exponential-decay and gain stages of the filter
  were collapsed into a single pass. Previously each stage allocated an
  intermediate buffer; the fused version writes directly to the output
  and skips the intermediate.
- **Binary search `last_idx`.** The event list is sorted by time, so the
  code locates the most-recent event with `t <= now` via a binary search
  rather than a linear scan. This matters most at large `t` where the
  event list is long.
- **In-place PathContext.** The motion-path evaluation context is reused
  across calls rather than being freshly allocated. This removes a
  per-frame heap allocation on the hot path and keeps cache lines warm.

## Known Hot Spots Not Yet Addressed

These are known costs that are either not measured by the engine bench or
not yet optimized. They are documented here so the next pass has a place
to start.

- **Path registry RwLock contention.** The default motion-path registry
  is behind an `RwLock`. In the single-threaded bench this is a cheap
  read, but under the real app (where the audio thread and render thread
  both touch it) the uncontended read can still cost a cache-coherency
  round trip, and any writer blocks all readers. Candidates: swap for a
  per-thread or arc-swap-based registry, or snapshot the registry into
  the `EngineInputs` once per frame.
- **glyphon prepare cost.** The text renderer's `prepare` step uploads
  glyph atlases and vertex data to the GPU each frame. This is *not*
  measured by the engine benches (which stop before any rendering) and
  is believed to be a larger share of the per-frame cost than the engine
  itself. A dedicated bench or trace would clarify.

## Running Benches Locally

```sh
make bench
```

or, equivalently:

```sh
cargo bench
```

To run only the engine bench:

```sh
cargo bench --bench engine
```

To run only the text-layout bench (one-shot cosmic-text shaping cost):

```sh
cargo bench --bench text_layout
```

Criterion writes HTML reports to `target/criterion/` after each run. If
`gnuplot` is not installed, Criterion falls back to the `plotters` backend
automatically, which produces the same PNG summaries without the extra
dependency.

## How To Interpret Changes

When you change engine code, re-run `cargo bench --bench engine` and
compare against the table above. Criterion will also show its own
regression / improvement verdict relative to the last run stored in
`target/criterion/`. Note that:

- Small (<5%) swings on `t_start` are usually noise, since the absolute
  time is only ~20 microseconds and bench outliers are common on macOS.
- Changes to `t_mid` and `t_end` are the most informative, since those
  numbers scale with the number of accumulated events — a regression
  there usually means the filter cache or the binary-search path has
  been invalidated.
- If you add a new allocation on the hot path, expect `t_mid` and
  `t_end` to show it clearly. The in-place `PathContext` change is a
  good example of the magnitude such a regression would have.
