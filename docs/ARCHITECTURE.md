# Architecture

A deep technical overview of the scenic-motion-engine Mac app. Read alongside `src/types.rs` (the shared contract) and `src/engine.rs` (the pure core).

## Pure-function engine model

The engine is a single pure function:

```
pub fn evaluate(inp: EngineInputs<'_>) -> FrameState
```

Given a time `t` (seconds), a `&Config`, a `&dyn EventSource`, a `&UnitLayout`, a `row_count`, and a `key_row`, it returns a `FrameState` describing every visible row and the camera for that instant. The function does not read a clock, mutate globals, or touch I/O. The same inputs always yield the same output. That property is load-bearing for offline recording (frame `n` is rendered at `t = start_at + n/fps`) and for scrubbing (the UI calls `evaluate(t)` for whatever `t` the audio player reports).

`FrameState` carries additive deltas, not final transforms. Rows start at `scale = 1, opacity = 1` so `*= 1 + delta` and clamped `+= opacity` composition degrade to identity on an empty frame. The camera starts at `zoom = 1` and gets combined with `config.camera.baseline` at render time.

## Data flow

Startup wires one directed path from JSON on disk to pixels in the window:

```
analysis/<track>.json
        v
AnalysisData (serde-deserialized Config-adjacent blob)
        v
JsonEventSource  — impl of trait EventSource (src/types.rs)
        v
TickerConfig     — band + modulo + strengthThreshold + step + stepOffset + offset
   step/stepOffset subsample the band's event list; strengthThreshold filters;
   modulo + offset wrap event index -> phase. FilterCache in engine.rs
   memoizes the filtered Vec per (band, threshold, step, stepOffset).
        v
Binding          — ties a ticker to a sequence, a target (Rows or Camera),
                   a row selector, a compose mode (Add/Multiply/Override),
                   and respect_alternation.
        v
PhaseSequence[action_idx = phase % actions.len()]
        v
PathFn = fn(&PathContext) -> PathOutput        (src/motion/paths.rs)
   16 registered paths; context carries progress, eased_progress, strength,
   phase, params, unit_width, canvas dims, dir.
        v
PathOutput { dx, dy, scale, rotation, opacity, tracking, per_glyph }
   Deltas around identity. per_glyph is an optional Box<dyn Fn(i64)->...>.
        v
scale_horizontal + apply_dir (feel.horizontal_scale * row-velocity shape;
    direction mirrors horizontal components on odd rows when respect_alt).
        v
compose_into(RowMotionState, mode)   -- for Rows targets
compose_camera(CameraState, mode)    -- for Camera targets
        v
FrameState { t, rows: Vec<RowMotionState>, camera, tickers }
        v
TextRenderer::prepare (src/render/text_renderer.rs)
   Walks visible glyph range per row via UnitLayout::visible_range.
   Fast path: single cosmic_text::Buffer per row (dim strip) + 1 Buffer per
     key glyph for the colored overlay. Taken when the row has no per_glyph
     closure, zero tracking, no rotation, scale=1.
   Slow path: one Buffer per visible glyph, honoring per_glyph deltas,
     per-glyph scale, dx/dy, opacity. Row-local rotate/scale affine applies
     about the row's vertical center; camera affine applies about canvas
     center; final left/top is snapped to half-pixel when dpr*render_scale>=2.
        v
glyphon::TextRenderer::prepare -> wgpu buffer uploads
        v
ScenicPaintCallback::paint (src/app.rs)
   egui_wgpu Callback runs inside the Central panel's RenderPass; it calls
   glyphon::TextRenderer::render against that pass.
        v
eframe window
```

Audio playback is a side-car to this pipeline — the only thing it feeds back into evaluate is `self.player.time()`.

## Threading model

The UI thread owns everything mutable. The app is single-threaded by default and only breaks that invariant in two places.

- Audio runs on cpal's own output thread (CoreAudio dispatch queue on Mac). `src/audio/playback.rs` protects shared state behind a `parking_lot::Mutex<PlayState>` holding `frame_cursor`, `frac`, `playing`, `out_sr`, `file_sr`, `total_frames`. The audio callback takes one short lock to snapshot, does its linear-resample fill into the device buffer, then takes a second short lock to write back cursor + frac. The decoded sample `Arc<Vec<f32>>` is lock-free (immutable after `load_file`).
- Recording spawns a `std::thread` per click of the record button (`ScenicApp::start_recording` in `src/app.rs`). The worker clones `Config` and the `Arc<dyn EventSource>`, so the UI can keep mutating its live `Config` while the job renders. Progress is reported through two `Arc<AtomicU32>` (frames done, total). The UI polls `JoinHandle::is_finished()` each frame and joins when done.

Everything else — layout rebuild, `evaluate`, `TextRenderer::prepare`, egui redraw — happens on the UI thread.

## Determinism guarantees

- `evaluate(t)` is pure. No clock reads, no RNG seeded from time, no mutable statics touched. The path registry is written once at startup (`register_default_paths`) and read-only thereafter.
- `glyphJitter` seeds its hash from `(glyph_index, row, floor(t * 60))` — stable per frame at 60 fps.
- `cameraZoomDeltaFocused` seeds its RNG from `floor(event.t * 1000) + 1`. This is important: without it, the per-event focus point would drift between the live preview and the offline render, because floating-point `event.t - since_last` reconstruction only matches to a few decimal places. Quantizing to milliseconds pins the seed to the event, not to the sampling moment.
- The `FilterCache` hashes thresholds via `f32::to_bits`, giving exact-match caching across identical calls within a single `evaluate`.
- `BTreeMap<String, ParamValue>` (`Params`) orders keys so JSON serialization round-trips byte-stable.

Non-deterministic surfaces: the audio callback can advance `frame_cursor` between two calls to `player.time()`, so the preview jitters by one audio buffer. Offline renders bypass audio entirely and compute `t` from the frame index.

## egui_wgpu CallbackResources and the `&'static` transmute

`glyphon::TextRenderer::render(&self, &TextAtlas, &Viewport, &mut wgpu::RenderPass<'pass>)` ties the atlas and viewport lifetimes to the pass. The TextArea slice passed to `prepare` must also outlive the pass. The natural ownership story — "TextRenderer lives on the eframe::App struct" — breaks because egui_wgpu's `CallbackTrait::paint` signature is:

```rust
fn paint(
    &self,
    info: PaintCallbackInfo,
    pass: &mut wgpu::RenderPass<'static>,
    resources: &CallbackResources,
);
```

The pass is `'static`-lifetime for borrow-checking purposes (egui-wgpu internally transmutes it), but `resources` is borrowed with the method's local lifetime. We cannot pull a `&TextRenderer` out of `resources` and pass it to a function that expects something living as long as a `'static` pass without widening the lifetime.

Hence the one `unsafe` in `src/app.rs`:

```rust
let tr: &'static TextRenderer = unsafe {
    match resources.get::<TextRenderer>() {
        Some(r) => std::mem::transmute::<&TextRenderer, &'static TextRenderer>(r),
        None => return,
    }
};
tr.render(pass)?;
```

Safety invariant: `CallbackResources` is a `TypeMap` owned by `egui_wgpu::Renderer`, which is owned by `egui_wgpu::WgpuRenderer`, which lives on `eframe::Frame` for the whole app lifetime. Any single paint callback is a borrow nested inside that storage. The transmute widens a borrow that is already live for the entire paint phase; it is not creating a new borrow that outlives the storage. The `TextRenderer` is never moved out of the TypeMap during a paint, and `prepare` runs on the UI thread before the paint callback is dispatched (never concurrently). Therefore for the duration of a single paint, the pointer is valid and no aliasing writes occur.

The alternative — owning `TextRenderer` on `ScenicApp` and passing `&self` through the Callback — is forbidden by egui-wgpu's trait signature; the callback is `&self` on a zero-size type and resources are keyed by `TypeId`.

## Where to look next

- `src/engine.rs` — `evaluate`, `FilterCache`, `compose_into`, `compose_camera`, `row_velocity_scale`.
- `src/motion/paths.rs` — the 16 path functions; see `docs/PATH_REFERENCE.md` for a one-liner each.
- `src/render/text_renderer.rs` — bulk vs. per-glyph decision at `can_bulk`; camera and row affines.
- `src/audio/playback.rs` — `fill_f32` is the audio callback; everything else is boilerplate around cpal + symphonia.
- `src/app.rs` — `ScenicPaintCallback::paint` holds the `unsafe` transmute; `start_recording` holds the record-thread spawn.
