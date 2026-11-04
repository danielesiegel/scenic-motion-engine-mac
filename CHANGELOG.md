# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

Project context, feature matrix, and architecture live in the
[README](./README.md). Contribution flow is documented in
[CONTRIBUTING.md](./CONTRIBUTING.md).

---

## [Unreleased]

### Added

### Changed

### Fixed

- Linux MIDI source compilation while preserving the `EventSource` thread-safety contract.

---

## [0.1.0] - 2026-11-02

Initial native macOS port of
[`scenic-motion-engine-web`](https://github.com/danielesiegel/scenic-motion-engine-web).
Compiles end-to-end; most modules fully ported from the TypeScript
reference. Live wgpu callback wiring and record-UI are still WIP (see
[README status](./README.md#status)).

### Added

- **Engine** (`src/engine.rs`): deterministic `evaluate(EngineInputs) ->
  FrameState` port. All 16 path functions, 19 named easings + cubic
  bezier (Newton-Raphson), step / step_offset subsampling, horizontal
  scale, U-curve row velocity, frozen+active composition, per-row
  direction alternation.
- **GUI** (`src/gui/panel.rs`): egui side panel — one-to-one with the
  web project's Tweakpane layout. Feel presets, horizontal x scale,
  row velocity curve, duration scale, global easing, spotlight /
  tickers / bindings / camera / record folders. Scrub + loop controls
  in the bottom bar.
- **Record pipeline** (`src/record/mp4.rs`): `FfmpegRecorder` — raw
  RGBA frames piped to `ffmpeg` libx264 subprocess. UI wiring in
  progress; native AVFoundation H.264 encoder planned.
- **Audio** (`src/audio/`): `symphonia` decode -> f32 interleaved ->
  `cpal` CoreAudio playback. `JsonEventSource` +
  `SyntheticEventSource` with binary-search indexing.
- **Text** (`src/text/`): `cosmic-text` `FontSystem` loaded from
  `assets/fonts/`; unit pre-shaping and inter-unit stride measurement.
- **Render** (`src/render/`): wgpu Device/Queue/Surface wrapper atop
  eframe's wgpu backend; `glyphon`-backed GPU text renderer scaffold.
- **Config** (`src/config/defaults.rs`): `DEFAULT_CONFIG` with 6
  tickers, 6 bindings, camera, feel, record defaults.
- **Types** (`src/types.rs`): the immutable contract — every module
  builds against these types. Governs the contract-first swarm pattern
  documented in [CONTRIBUTING.md](./CONTRIBUTING.md).
- **Docs**: [README.md](./README.md), [MOTION.md](./MOTION.md) (path
  extension guide), [CONTRIBUTING.md](./CONTRIBUTING.md),
  [SECURITY.md](./SECURITY.md).
- **CI** (`.github/workflows/ci.yml`): `cargo fmt --check`, `cargo
  clippy -- -D warnings`, `cargo test`, `cargo build --release` on
  every push.
- **Makefile**: `run`, `render`, `check`, `bundle`, `analyze` targets.
  See `make help`.
- **Tests** (`tests/`): determinism, continuity, alternation suites
  ported from the TypeScript reference.
- **Benches** (`benches/`): criterion benches for the
  `engine::evaluate` hot path.

### Changed

- _(nothing — initial release)_

### Fixed

- _(nothing — initial release)_

---

[Unreleased]: https://github.com/danielesiegel/scenic-motion-engine-mac/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/danielesiegel/scenic-motion-engine-mac/releases/tag/v0.1.0
