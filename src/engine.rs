//! `evaluate(t, …) -> FrameState` — the pure core.
//!
//! Reference: ../../scenic-motion-engine-web/src/engine.ts
//!
//! Contract:
//! - For each row + binding: iterate past filtered events, accumulate frozen contributions from
//!   past events + active contribution from the latest event (continuity model). Camera bindings
//!   evaluate without a row scope.
//! - `step` / `stepOffset` on a ticker subsamples the event list before modulo-cycling.
//! - `feel.horizontal_scale` multiplies every dx + tracking + perGlyph.dx globally.
//! - `feel.row_velocity_shape` + `edge_scale` apply a per-row horizontal multiplier.
//! - `feel.duration_scale` multiplies every action.duration; `feel.global_easing` overrides easings.

use crate::motion::paths::get_path;
use crate::motion::phase::apply_easing;
use crate::motion::tickers::resolve_threshold;
use crate::types::*;
use smallvec::SmallVec;
use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

pub struct EngineInputs<'a> {
    pub t: f32,
    pub config: &'a Config,
    pub source: &'a dyn EventSource,
    pub layout: &'a UnitLayout,
    pub row_count: u32,
    pub key_row: u32,
}

// ---------------------------------------------------------------------------
// Row / camera initialization. Rows start at scale=1, opacity=1 so that
// compose '*= 1 + delta' and 'opacity += delta clamped' behave correctly.
// Camera starts at zoom=1 (baseline is applied by the renderer).
// ---------------------------------------------------------------------------

fn empty_row() -> RowMotionState {
    RowMotionState {
        dx: 0.0,
        dy: 0.0,
        scale: 1.0,
        rotation: 0.0,
        opacity: 1.0,
        tracking: 0.0,
        per_glyph: Vec::new(),
    }
}

fn empty_camera() -> CameraState {
    CameraState {
        zoom: 1.0,
        pan_x: 0.0,
        pan_y: 0.0,
        tilt: 0.0,
    }
}

// ---------------------------------------------------------------------------
// Binary search: first index i such that events[i].t > t  (== count of events
// with t <= query_t).
// ---------------------------------------------------------------------------

#[inline]
fn count_le_by_t(events: &[TransientEvent], t: f32) -> usize {
    let (mut lo, mut hi) = (0usize, events.len());
    while lo < hi {
        let mid = lo + (hi - lo) / 2;
        if events[mid].t <= t {
            lo = mid + 1;
        } else {
            hi = mid;
        }
    }
    lo
}

// ---------------------------------------------------------------------------
// FilterCache: scope-local (one `evaluate()` call) cache keyed by
// (band, quantized threshold, step, step_offset). HUD snapshot + binding loop
// share filtered Vecs so we only build each unique filter once per frame.
// ---------------------------------------------------------------------------

type FilteredVec = SmallVec<[TransientEvent; 128]>;

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct FilterKey {
    band: Band,
    threshold_bits: u32, // f32::to_bits — exact-match cache (threshold is derived identically)
    step: u32,
    step_offset: u32,
}

struct FilterCache<'a> {
    source: &'a dyn EventSource,
    entries: Vec<(FilterKey, Arc<FilteredVec>)>,
}

impl<'a> FilterCache<'a> {
    fn new(source: &'a dyn EventSource) -> Self {
        Self {
            source,
            entries: Vec::with_capacity(8),
        }
    }

    fn get(&mut self, ticker: &TickerConfig, effective_threshold: f32) -> Arc<FilteredVec> {
        let step = ticker.step.unwrap_or(1).max(1);
        let step_offset = ticker.step_offset.unwrap_or(0);
        let key = FilterKey {
            band: ticker.band,
            threshold_bits: effective_threshold.to_bits(),
            step,
            step_offset,
        };
        for (k, v) in &self.entries {
            if *k == key {
                return v.clone();
            }
        }
        let raw = self.source.get_events(ticker.band);
        let filtered = build_filtered(raw, effective_threshold, step, step_offset);
        let arc = Arc::new(filtered);
        self.entries.push((key, arc.clone()));
        arc
    }
}

// Fused threshold + step filter in a single linear pass. Writes into a SmallVec
// to avoid heap allocation for the common case (few hundred events post-filter).
fn build_filtered(
    raw: &[TransientEvent],
    threshold: f32,
    step: u32,
    step_offset: u32,
) -> FilteredVec {
    let s = step.max(1) as i64;
    let o = (((step_offset as i64) % s) + s) % s;
    let threshold_active = threshold > 0.0;
    let step_passthrough = s == 1 && o == 0;

    let mut out: FilteredVec = SmallVec::new();

    if threshold_active {
        if step_passthrough {
            out.reserve(raw.len());
            for e in raw {
                if e.strength >= threshold {
                    out.push(*e);
                }
            }
        } else {
            // Enumerate the thresholded sequence (not the raw sequence): step applies
            // to indices in the post-threshold list, matching TS semantics.
            let mut idx: i64 = 0;
            for e in raw {
                if e.strength >= threshold {
                    if idx >= o && (((idx - o) % s + s) % s) == 0 {
                        out.push(*e);
                    }
                    idx += 1;
                }
            }
        }
    } else if step_passthrough {
        out.reserve(raw.len());
        out.extend(raw.iter().copied());
    } else {
        for (i, e) in raw.iter().enumerate() {
            let ii = i as i64;
            if ii >= o && (((ii - o) % s + s) % s) == 0 {
                out.push(*e);
            }
        }
    }
    out
}

// ---------------------------------------------------------------------------
// Effective (duration, easing) for an action under feel settings.
// ---------------------------------------------------------------------------

#[inline]
fn effective_action_timing(action: &PhaseAction, config: &Config) -> (f32, Option<Easing>) {
    let scale = config.feel.duration_scale.max(0.01);
    let duration = (action.duration * scale).max(1e-6);
    let easing = match config.feel.global_easing {
        GlobalEasing::Auto => action.easing,
        GlobalEasing::Named(name) => Some(Easing::Named(name)),
    };
    (duration, easing)
}

// ---------------------------------------------------------------------------
// Apply direction to a PathOutput. Mirrors TS applyDir: multiplies horizontal
// components by dir (only when respect_alternation AND dir is Neg).
// ---------------------------------------------------------------------------

fn apply_dir(mut out: PathOutput, dir: Dir, respect_alternation: bool) -> PathOutput {
    if !respect_alternation || matches!(dir, Dir::Pos) {
        return out;
    }
    let d = dir.as_f32();
    out.dx *= d;
    if let Some(orig) = out.per_glyph.take() {
        out.per_glyph = Some(Box::new(move |g: i64| {
            let mut r = orig(g);
            r.dx *= d;
            r
        }));
    }
    out
}

// ---------------------------------------------------------------------------
// Running accumulator (all deltas additive).
// ---------------------------------------------------------------------------

#[derive(Default)]
struct Running {
    dx: f32,
    dy: f32,
    scale: f32,
    rotation: f32,
    opacity: f32,
    tracking: f32,
    per_glyph: Vec<Arc<dyn Fn(i64) -> PerGlyphOutput + Send + Sync>>,
}

impl Running {
    #[inline]
    fn absorb(&mut self, out: PathOutput) {
        self.dx += out.dx;
        self.dy += out.dy;
        self.scale += out.scale;
        self.rotation += out.rotation;
        self.opacity += out.opacity;
        self.tracking += out.tracking;
        if let Some(f) = out.per_glyph {
            self.per_glyph.push(Arc::from(f));
        }
    }

    fn into_output(self) -> PathOutput {
        let per_glyph: Option<Box<dyn Fn(i64) -> PerGlyphOutput + Send + Sync>> =
            if self.per_glyph.is_empty() {
                None
            } else {
                let fns = self.per_glyph;
                Some(Box::new(move |g: i64| {
                    let mut acc = PerGlyphOutput::default();
                    for f in &fns {
                        let r = f(g);
                        acc.dx += r.dx;
                        acc.dy += r.dy;
                        acc.scale += r.scale;
                        acc.rotation += r.rotation;
                        acc.opacity += r.opacity;
                    }
                    acc
                }))
            };
        PathOutput {
            dx: self.dx,
            dy: self.dy,
            scale: self.scale,
            rotation: self.rotation,
            opacity: self.opacity,
            tracking: self.tracking,
            per_glyph,
        }
    }
}

// ---------------------------------------------------------------------------
// Evaluate one binding's contribution for one row (or scope-less for camera).
//
// Hot-path notes:
// - last_idx found via binary search (count_le_by_t).
// - PathContext is built once; mutated in place across events. params HashMap is
//   cloned only when the active action pointer changes — for the common case
//   (modulo <= num_actions == 1) that's one clone per call instead of N.
// ---------------------------------------------------------------------------

#[allow(clippy::too_many_arguments)]
fn evaluate_binding_contribution(
    t: f32,
    binding: &Binding,
    ticker_cfg: &TickerConfig,
    sequence: &PhaseSequence,
    filtered: &[TransientEvent],
    row_index: u32,
    row_count: u32,
    dir: Dir,
    layout: &UnitLayout,
    config: &Config,
) -> PathOutput {
    if sequence.actions.is_empty() || filtered.is_empty() {
        return PathOutput::default();
    }
    let modulo = ticker_cfg.modulo.max(1) as i64;
    let offset = ticker_cfg.offset.unwrap_or(0) as i64;

    // last_idx = largest i such that filtered[i].t <= t. Binary search.
    let upto = count_le_by_t(filtered, t);
    if upto == 0 {
        return PathOutput::default();
    }
    let last_idx: usize = upto - 1;

    let respect_alt =
        binding.respect_alternation.unwrap_or(true) && binding.target == BindingTarget::Rows;

    let mut running = Running::default();

    let unit_width = layout.stride;
    let canvas_width = config.canvas.width as f32;
    let canvas_height = config.canvas.height as f32;

    // Prebuild a PathContext we mutate in place.
    let mut ctx = PathContext {
        row_index,
        row_count,
        dir,
        progress: 0.0,
        eased_progress: 0.0,
        strength: 0.0,
        phase: 0,
        t,
        since_last: 0.0,
        params: Params::new(),
        unit_width,
        duration: 0.0,
        canvas_width,
        canvas_height,
    };

    let mut last_action_ptr: *const PhaseAction = std::ptr::null();

    // Past events: frozen progress based on gap to next event.
    for i in 0..last_idx {
        let phase_i = (((i as i64 + offset) % modulo) + modulo) % modulo;
        let action = &sequence.actions[(phase_i as usize) % sequence.actions.len()];
        let path_fn = match get_path(&action.path) {
            Some(f) => f,
            None => continue,
        };
        let (duration, easing) = effective_action_timing(action, config);
        let gap = filtered[i + 1].t - filtered[i].t;
        let frozen = (gap / duration).clamp(0.0, 1.0);
        let eased = apply_easing(easing, frozen);

        // Repopulate params only when the action pointer changes.
        let action_ptr: *const PhaseAction = action;
        if action_ptr != last_action_ptr {
            ctx.params.clone_from(&action.params);
            last_action_ptr = action_ptr;
        }
        ctx.progress = frozen;
        ctx.eased_progress = eased;
        ctx.strength = filtered[i].strength;
        ctx.phase = phase_i as u32;
        ctx.since_last = t - filtered[i].t;
        ctx.duration = duration;

        let out = apply_dir(path_fn(&ctx), dir, respect_alt);
        running.absorb(out);
    }

    // Active (latest) event.
    let i = last_idx;
    let phase_i = (((last_idx as i64 + offset) % modulo) + modulo) % modulo;
    let action = &sequence.actions[(phase_i as usize) % sequence.actions.len()];
    if let Some(path_fn) = get_path(&action.path) {
        let (duration, easing) = effective_action_timing(action, config);
        let progress = ((t - filtered[i].t) / duration).clamp(0.0, 1.0);
        let eased = apply_easing(easing, progress);
        let action_ptr: *const PhaseAction = action;
        if action_ptr != last_action_ptr {
            ctx.params.clone_from(&action.params);
        }
        ctx.progress = progress;
        ctx.eased_progress = eased;
        ctx.strength = filtered[i].strength;
        ctx.phase = phase_i as u32;
        ctx.since_last = t - filtered[i].t;
        ctx.duration = duration;

        let out = apply_dir(path_fn(&ctx), dir, respect_alt);
        running.absorb(out);
    }

    running.into_output()
}

// ---------------------------------------------------------------------------
// Row velocity shaping. Port of rowVelocityScale in engine.ts.
// ---------------------------------------------------------------------------

#[inline]
fn row_velocity_scale(row_index: u32, row_count: u32, config: &Config) -> f32 {
    let shape = config.feel.row_velocity_shape;
    let edge_raw = config.feel.row_velocity_edge_scale;
    let edge = if edge_raw.is_finite() { edge_raw } else { 1.0 };
    if matches!(shape, RowVelocityShape::Flat) || row_count <= 1 || edge == 1.0 {
        return 1.0;
    }
    let p = row_index as f32 / (row_count as f32 - 1.0);
    let q = 2.0 * p - 1.0;
    match shape {
        RowVelocityShape::Flat => 1.0,
        RowVelocityShape::U => 1.0 + (edge - 1.0) * q * q,
        RowVelocityShape::InvertedU => edge - (edge - 1.0) * q * q,
        RowVelocityShape::Linear => edge + (1.0 - edge) * p,
    }
}

// ---------------------------------------------------------------------------
// Apply horizontal scale (feel.horizontal_scale * row velocity) to a PathOutput.
// Multiplies dx, tracking, and perGlyph.dx. Matches TS scaleHorizontal.
// ---------------------------------------------------------------------------

fn scale_horizontal(mut out: PathOutput, h_scale: f32) -> PathOutput {
    if h_scale == 1.0 {
        return out;
    }
    out.dx *= h_scale;
    out.tracking *= h_scale;
    if let Some(orig) = out.per_glyph.take() {
        out.per_glyph = Some(Box::new(move |g: i64| {
            let mut r = orig(g);
            r.dx *= h_scale;
            r
        }));
    }
    out
}

// ---------------------------------------------------------------------------
// Compose a PathOutput into a RowMotionState using ComposeMode.
// ---------------------------------------------------------------------------

fn compose_into(state: &mut RowMotionState, out: PathOutput, mode: ComposeMode) {
    match mode {
        ComposeMode::Override => state.dx = out.dx,
        ComposeMode::Multiply => state.dx *= out.dx,
        ComposeMode::Add => state.dx += out.dx,
    }
    match mode {
        ComposeMode::Override => state.dy = out.dy,
        ComposeMode::Multiply => state.dy *= out.dy,
        ComposeMode::Add => state.dy += out.dy,
    }
    // scale: delta around 0; row baseline starts at 1.
    match mode {
        ComposeMode::Override => state.scale = 1.0 + out.scale,
        ComposeMode::Multiply => state.scale *= 1.0 + out.scale,
        ComposeMode::Add => state.scale *= 1.0 + out.scale,
    }
    match mode {
        ComposeMode::Override => state.rotation = out.rotation,
        ComposeMode::Multiply => state.rotation *= out.rotation,
        ComposeMode::Add => state.rotation += out.rotation,
    }
    // opacity: delta around 1 for override/multiply; add is clamped sum.
    match mode {
        ComposeMode::Override => state.opacity = 1.0 + out.opacity,
        ComposeMode::Multiply => state.opacity *= 1.0 + out.opacity,
        ComposeMode::Add => state.opacity = (state.opacity + out.opacity).clamp(0.0, 1.0),
    }
    match mode {
        ComposeMode::Override => state.tracking = out.tracking,
        ComposeMode::Multiply => state.tracking *= out.tracking,
        ComposeMode::Add => state.tracking += out.tracking,
    }
    if let Some(f) = out.per_glyph {
        state.per_glyph.push(f);
    }
}

// ---------------------------------------------------------------------------
// Compose a PathOutput into the CameraState using ComposeMode. Camera uses:
//   dx -> pan_x, dy -> pan_y, scale -> zoom (*= 1+v), rotation -> tilt.
// Tracking, opacity, perGlyph are ignored.
// ---------------------------------------------------------------------------

fn compose_camera(cam: &mut CameraState, out: &PathOutput, mode: ComposeMode) {
    match mode {
        ComposeMode::Override => cam.pan_x = out.dx,
        ComposeMode::Multiply => cam.pan_x += out.dx,
        ComposeMode::Add => cam.pan_x += out.dx,
    }
    match mode {
        ComposeMode::Override => cam.pan_y = out.dy,
        ComposeMode::Multiply => cam.pan_y += out.dy,
        ComposeMode::Add => cam.pan_y += out.dy,
    }
    match mode {
        ComposeMode::Override => cam.zoom = 1.0 + out.scale,
        ComposeMode::Multiply => cam.zoom *= 1.0 + out.scale,
        ComposeMode::Add => cam.zoom *= 1.0 + out.scale,
    }
    match mode {
        ComposeMode::Override => cam.tilt = out.rotation,
        ComposeMode::Multiply => cam.tilt += out.rotation,
        ComposeMode::Add => cam.tilt += out.rotation,
    }
}

// ---------------------------------------------------------------------------
// Build per-ticker HUD snapshots. Mirrors the loop in engine.ts.
// Shares a FilterCache with the binding loop.
// ---------------------------------------------------------------------------

fn build_ticker_snapshots(
    t: f32,
    config: &Config,
    thresholds: &BTreeMap<String, f32>,
    cache: &mut FilterCache<'_>,
) -> Vec<TickerSnapshot> {
    let mut out = Vec::with_capacity(config.tickers.len());
    for ticker in &config.tickers {
        let eff_threshold = resolve_threshold(ticker, thresholds);
        let filtered = cache.get(ticker, eff_threshold);
        let modulo = ticker.modulo.max(1) as i64;
        let offset = ticker.offset.unwrap_or(0) as i64;

        let upto = count_le_by_t(&filtered, t);
        let latest_idx: i64 = if upto == 0 { -1 } else { (upto - 1) as i64 };

        let phase: i64 = if latest_idx >= 0 {
            (((latest_idx + offset) % modulo) + modulo) % modulo
        } else {
            ((offset % modulo) + modulo) % modulo
        };

        // Lookup the action via the FIRST binding referencing this ticker (TS: find).
        let seq_action: Option<&PhaseAction> = config
            .bindings
            .iter()
            .find(|b| b.ticker_id == ticker.id)
            .and_then(|b| {
                let actions = &b.sequence.actions;
                if actions.is_empty() {
                    None
                } else {
                    Some(&actions[(phase as usize) % actions.len()])
                }
            });

        let (since_last, strength) = if latest_idx >= 0 {
            let ev = filtered[latest_idx as usize];
            (t - ev.t, ev.strength)
        } else {
            (f32::INFINITY, 0.0)
        };

        let (progress, active_path) = if let Some(action) = seq_action {
            let (duration, _) = effective_action_timing(action, config);
            let p = (since_last / duration).clamp(0.0, 1.0);
            (p, Some(action.path.clone()))
        } else {
            (0.0, None)
        };

        out.push(TickerSnapshot {
            id: ticker.id.clone(),
            band: ticker.band,
            count: (latest_idx + 1).max(0) as u32,
            phase: phase as u32,
            since_last,
            strength,
            progress,
            active_path,
        });
    }
    out
}

// ---------------------------------------------------------------------------
// Inline row-mask check without allocating a Vec<bool>.
// ---------------------------------------------------------------------------

#[inline]
fn row_selected(binding: &Binding, row: u32, row_count: u32, key_row: u32) -> bool {
    if binding.target == BindingTarget::Camera {
        return false;
    }
    if row >= row_count {
        return false;
    }
    match binding.rows.as_ref() {
        None => true,
        Some(RowSelector::Name(name)) => match name {
            RowSelectorName::All => true,
            RowSelectorName::Even => row.is_multiple_of(2),
            RowSelectorName::Odd => !row.is_multiple_of(2),
            RowSelectorName::KeyRow => row == key_row,
        },
        Some(RowSelector::Indices(ix)) => ix.contains(&row),
    }
}

// ---------------------------------------------------------------------------
// Public entrypoint.
// ---------------------------------------------------------------------------

pub fn evaluate(inp: EngineInputs<'_>) -> FrameState {
    let EngineInputs {
        t,
        config,
        source,
        layout,
        row_count,
        key_row,
    } = inp;

    let mut rows: Vec<RowMotionState> = (0..row_count).map(|_| empty_row()).collect();
    let mut camera = empty_camera();

    let h_scale = config.feel.horizontal_scale.max(0.0);
    let thresholds = &config.band_thresholds;

    // Shared filter cache across the whole evaluate() call.
    let mut cache = FilterCache::new(source);

    // Ticker snapshots for HUD (reads from cache).
    let tickers = build_ticker_snapshots(t, config, thresholds, &mut cache);

    // Index tickers by id for quick lookup.
    let ticker_by_id: HashMap<&str, &TickerConfig> =
        config.tickers.iter().map(|t| (t.id.as_str(), t)).collect();

    for binding in &config.bindings {
        let ticker_cfg = match ticker_by_id.get(binding.ticker_id.as_str()) {
            Some(tc) => *tc,
            None => continue,
        };
        let eff_threshold = resolve_threshold(ticker_cfg, thresholds);
        let filtered = cache.get(ticker_cfg, eff_threshold);
        let mode = binding.compose;

        if binding.target == BindingTarget::Camera {
            let out = evaluate_binding_contribution(
                t,
                binding,
                ticker_cfg,
                &binding.sequence,
                &filtered,
                0,
                row_count,
                Dir::Pos,
                layout,
                config,
            );
            let scaled = scale_horizontal(out, h_scale);
            compose_camera(&mut camera, &scaled, mode);
            continue;
        }

        // Rows target. Inline row selector (no Vec<bool> alloc).
        for row_index in 0..row_count {
            if !row_selected(binding, row_index, row_count, key_row) {
                continue;
            }
            let dir = Dir::for_row(row_index);
            let out = evaluate_binding_contribution(
                t,
                binding,
                ticker_cfg,
                &binding.sequence,
                &filtered,
                row_index,
                row_count,
                dir,
                layout,
                config,
            );
            let row_scale = h_scale * row_velocity_scale(row_index, row_count, config);
            let scaled = scale_horizontal(out, row_scale);
            compose_into(&mut rows[row_index as usize], scaled, mode);
        }
    }

    FrameState {
        t,
        rows,
        tickers,
        camera,
    }
}
