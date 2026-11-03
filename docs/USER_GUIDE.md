# User Guide (first run on macOS)

Audio-reactive looping marquee. Native Metal rendering, CoreAudio playback,
`ffmpeg`-based MP4 export. This guide walks you from a fresh clone to a
recorded clip.

## Prerequisites

- **Rust** (stable, 1.75+). Install via `rustup`:
  `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`.
- **Xcode command-line tools**: `xcode-select --install`. Required for the
  system clang/metal headers that `wgpu`/`cosmic-text` link against.
- **ffmpeg** on `PATH`, for MP4 export: `brew install ffmpeg`. The app runs
  without it, but the record button will fail.

CoreAudio and Metal are system libraries; no extra installation.

## Dropping in assets

Three directories are read at startup. Create any that are missing.

```
assets/fonts/     # .otf or .ttf — ITC Avant Garde Gothic LT Demi preferred
assets/audio/     # a .wav of the track you want to drive the motion
analysis/         # a .json produced by the analyzer in the web project
```

- The analysis JSON is produced by
  `scenic-motion-engine-web/scripts/analyze.py` — it emits per-band transient
  events (`kick`, `snare`, `mid`, `hats`) plus beats/downbeats and optional
  envelopes. The schema is `AnalysisData` in `src/types.rs`.
- Font fallback: if `assets/fonts/` is empty, the shaper falls back to a
  cosmic-text system font. Glyph metrics will differ from the web reference.
- Audio and analysis file names should match (same track) so the engine's
  events line up with what you hear.

## Launching

```sh
cargo run --release
```

The first build downloads and compiles dependencies; expect several minutes.
Subsequent runs are near-instant. A single window opens with the preview
canvas on the left and the control panel on the right.

## GUI panel

The side panel mirrors the TypeScript project's Tweakpane layout one-to-one.
Sections (all collapsible):

- **Text** — unit string, key character, letter case.
- **Layout** — font size, leading, row count (fixed or auto), row stagger,
  rotation, margin.
- **Colors** — background, dim, key colors.
- **Key** — which rows show the key character, the per-row phase offset,
  travels vs. anchored mode, screen-anchor offset.
- **Camera** — zoom/pan/tilt baseline, row overshoot.
- **Feel** — preset, global duration scale, global easing override,
  horizontal scale, row velocity shape (`flat` / `u` / `inverted-u` /
  `linear`) and edge scale.
- **Band thresholds** — per-band strength gates (`kick`, `snare`, `mid`,
  `hats`).
- **Tickers** — one row per `TickerConfig`: band, modulo, strength, offset,
  step, step offset.
- **Bindings** — one card per `Binding`: ticker id, target (rows or camera),
  row selector, compose mode, respect-alternation flag, and the phase
  sequence (one action per modulo slot).
- **Record** — start time, duration, fps, bitrate (Mbps), resolution
  multiplier, engine (`auto` / `native` / `ffmpeg`).
- **Debug** — HUD toggle (ticker snapshots).

For deep per-field detail refer back to the web project's README —
field names and semantics are identical.

## Record flow

1. Scrub to roughly where you want the clip to start.
2. In **Record**, set `start_at` (seconds), `duration`, `fps`, and
   `video_bitrate_mbps`. `resolution_multiplier` scales the output above the
   on-screen canvas (e.g. 2 for a 1080p preview to render 4K).
3. Press **Record**. The engine re-evaluates each frame at
   `t = start_at + frame / fps` and streams raw RGBA into `ffmpeg` via stdin
   (`src/record/mp4.rs`). Rendering is offline: output is frame-exact
   regardless of system load.
4. The clip lands in the project root as `output.mp4` (or the configured
   path).

## Troubleshooting

- **No audio.** Check your macOS default output device in System Settings;
  the app opens the CoreAudio default. If a different device is wanted,
  change the system default before launching. The player log line printed to
  stdout on startup confirms which device opened.
- **No font / wrong metrics.** Confirm a `.otf` or `.ttf` sits directly in
  `assets/fonts/`. On fallback, cosmic-text picks a system font (SF Pro) —
  glyph widths differ from the Avant Garde reference, so layout stride will
  shift. Drop in the correct font and relaunch.
- **ffmpeg missing / record fails.** Run `which ffmpeg`. If empty, install
  with `brew install ffmpeg`. Set `record.engine = Native` to force the
  in-process encoder (openh264) when ffmpeg is unavailable; `Auto` tries
  native first and falls back.
- **Analysis file not found.** The expected path is driven by
  `config.audio.analysis_url`. If left unset, the engine falls back to a
  `SyntheticEventSource` and will animate on fake transients until a real
  JSON is dropped in.
- **Window blank / preview black.** Verify Metal is available (M-series or
  Intel-with-discrete-GPU Mac). Check stderr for wgpu adapter messages.
- **Rebuild after editing `src/config/defaults.rs`.** The default config is
  compiled in; `cargo run --release` picks up changes on the next build.
