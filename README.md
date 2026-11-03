# scenic-motion-engine

Native macOS port of [`scenic-motion-engine-web`](https://github.com/danielesiegel/scenic-motion-engine-web).
Audio-reactive looping marquee: four-band transient detection drives per-row scrolling,
camera zoom/tilt/pan, spotlight repetition. The engine is a **pure function of time** —
`engine::evaluate(t, config, source, layout) -> FrameState` has no mutable state.

**Why native:** the web prototype's path (`canvas.fillText` → `MediaRecorder` or `WebCodecs`)
drops frames under load and double-buffers audio through the browser graph. This port renders
glyphs on the GPU via `glyphon` (cosmic-text atop wgpu/Metal), decodes/plays audio directly
with `cpal` + `symphonia`, and exports MP4 by piping raw frames straight to `ffmpeg`.

## Build

```
cargo build --release
```

Requires `ffmpeg` on `PATH` for MP4 export (CoreAudio and Metal are system libraries).

## Run

```
cargo run --release
```

Then drop:
- A font (`.otf` or `.ttf`, ITC Avant Garde Gothic LT Demi preferred) into `assets/fonts/`
- A WAV into `assets/audio/`
- The matching analysis JSON into `analysis/` (produce it with the Python analyzer from the web project: `scenic-motion-engine-web/scripts/analyze.py`)

## Architecture

```
src/
├── types.rs                   # the hard contract — every module builds against these types
├── config/defaults.rs         # DEFAULT_CONFIG (6 tickers, 6 bindings, camera, feel, record)
├── audio/
│   ├── event_source.rs        # JsonEventSource + SyntheticEventSource (binary-search indexed)
│   └── playback.rs            # symphonia decode → f32 interleaved → cpal CoreAudio
├── text/
│   ├── font.rs                # cosmic-text FontSystem loaded from assets/fonts/
│   └── layout.rs              # pre-shape "scenicsynth"+"s" to get unit_pos + inter-unit stride
├── motion/
│   ├── phase.rs               # all 19 named easings + cubic bezier (Newton-Raphson)
│   ├── paths.rs               # 16 path functions, deterministic hash01 for jitter / focal zoom
│   ├── tickers.rs             # effective-threshold helper
│   └── bindings.rs            # row-selector resolution
├── engine.rs                  # evaluate() — step/offset subsample, frozen+active composition,
│                              #   per-row dir alternation, feel.horizontal_scale, U-curve
├── render/
│   ├── gpu.rs                 # wgpu Device/Queue/Surface wrapper (from eframe's wgpu backend)
│   └── text_renderer.rs       # glyphon-backed GPU text renderer
├── gui/panel.rs               # egui side panel (one-to-one with the TS Tweakpane layout)
├── record/mp4.rs              # FfmpegRecorder: raw RGBA frames piped to libx264
├── app.rs                     # top-level eframe::App
├── lib.rs                     # module declarations
└── main.rs                    # eframe entry point
```

## Status

- **Compiles end-to-end** at the module boundary; most modules fully ported from the TypeScript reference.
- **Working:** engine (all 16 paths, 19 easings, step/stepOffset, horizontal scale, U-curve rows), audio (decode + playback), text (cosmic-text shaping), config, GUI panel.
- **In progress:** GPU text renderer (`src/render/text_renderer.rs`) — glyphon integration still wiring frame-state → TextArea per-row.
- **Not yet wired:** `app.rs` is a scaffold placeholder. The orchestrator wires `AudioLoad → PlayerClock → Engine::evaluate → TextRenderer::prepare → eframe paint callback → RecordButton → FfmpegRecorder` after text_renderer is in.

## License

MIT.
