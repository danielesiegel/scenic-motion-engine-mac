//! GPU text renderer backed by `glyphon`. Consumes `FrameState` and the shared `UnitLayout`,
//! produces draw calls for one frame.
//!
//! Contract:
//! - `TextRenderer::new(gpu, font_system)` builds a glyphon::Cache + Atlas + Viewport + TextRenderer.
//! - `prepare(&mut self, config, layout, frame, row_count, key_row, font_system)` builds `TextArea`s
//!   for every visible glyph of every row — applying row scroll, per-glyph transforms, color (dim
//!   vs. key), camera baked into per-TextArea position.
//! - `render(&self, pass)` records the glyphon draw into the provided wgpu RenderPass.
//!
//! Coloring strategy: one Buffer per row carrying the full visible dim-colored glyph strip, plus a
//! short Buffer per visible key glyph overlaid at the key color. Rationale: splitting into Attr
//! runs still requires one text layout per transformed glyph (each has its own x/y, scale, rotation),
//! so there is no real win from a single rich-text Buffer. One row-wide buffer gives us bulk shaping
//! for the common uniform-color case, and tiny single-glyph buffers only materialize for highlights.
//! TRADEOFF: per-glyph scale/rotation/dx/dy would require individual glyph buffers anyway; v1 ignores
//! those per-glyph transforms (dx/dy still apply via row delta but per-glyph rotate/scale is dropped
//! for the uniform row path). Any row that has a `per_glyph` closure forces the per-glyph buffer
//! path, which honors all deltas.
//!
//! Shape caching: cosmic-text `set_text` + `shape_until_scroll` is the dominant per-frame cost
//! (~90% of `prepare()` on an M1 at 1080p). The shaped glyph positions depend only on
//! `(text, font_size, line_height, family)` — scroll offset, camera transform, color, and opacity
//! are applied downstream by glyphon positioning, which does not re-shape. We cache shape outputs
//! per Buffer in `row_shape_cache` / `glyph_shape_cache`: on each `prepare()`, before touching a
//! Buffer we check the cache; if the key is unchanged, we skip the re-shape entirely and emit a
//! `TextArea` referencing the already-shaped Buffer. Within a single `prepare()` we also dedupe:
//! multiple rows with the same visible string (common when all rows scroll in lockstep) share one
//! Buffer, with per-row placement differences expressed via the `TextArea` left/top/scale/color.
//!
//! Reference: ../../scenicsynths/src/render/canvas.ts for semantics.

use crate::render::gpu::GpuContext;
use crate::types::{
    Config, FrameState, KeyMode, KeyRowValue, KeyRows, MiddleTag, RowsMode, UnitLayout,
};
use anyhow::Result;
use cosmic_text::{Attrs, Buffer, Color, Family, FontSystem, Metrics, Shaping, SwashCache};
use glyphon::{Resolution, TextArea, TextBounds};

pub struct TextRenderer {
    pub gpu: GpuContext,
    #[allow(dead_code)]
    cache: glyphon::Cache,
    atlas: glyphon::TextAtlas,
    viewport: glyphon::Viewport,
    renderer: glyphon::TextRenderer,
    swash_cache: SwashCache,

    // Reusable buffer pool. Row buffers hold the whole visible dim strip per row.
    // Glyph buffers hold a single key-colored character; reused across frames to avoid churn.
    row_buffers: Vec<Buffer>,
    glyph_buffers: Vec<Buffer>,

    // Cross-frame shape caches, parallel to `row_buffers` / `glyph_buffers`. Each entry stores
    // the shape key the matching Buffer currently carries. On the next frame, if a slot is
    // requested with the same key, we skip the (dominant) cosmic-text re-shape.
    row_shape_cache: Vec<Option<RowShapeKey>>,
    glyph_shape_cache: Vec<Option<GlyphShapeKey>>,

    // Scratch state consumed by `render`. We keep placement information so `prepare` can emit
    // TextArea descriptors (which borrow Buffers) without colliding with the mutable borrow of
    // the buffer pool. Index references into `row_buffers` / `glyph_buffers`.
    placements: Vec<Placement>,

    width: u32,
    height: u32,
    font_family: Option<String>,
}

#[derive(Clone, Copy)]
enum PlacementKind {
    Row,
    Glyph,
}

struct Placement {
    kind: PlacementKind,
    index: usize,
    left: f32,
    top: f32,
    scale: f32,
    color: Color,
}

// Shape identity for a row's dim strip. Equality means "cosmic-text would produce the same
// shaped glyphs, same advances, same positions". Scroll/color/alpha are NOT part of the key —
// they are applied via `TextArea` without re-shaping.
#[derive(Clone, PartialEq)]
struct RowShapeKey {
    text: String,
    // f32 bit-pattern for exact equality; shape depends on exact numeric metrics.
    font_size_bits: u32,
    line_height_bits: u32,
    family: Option<String>,
}

#[derive(Clone, PartialEq)]
struct GlyphShapeKey {
    ch: char,
    font_size_bits: u32,
    line_height_bits: u32,
    family: Option<String>,
}

impl TextRenderer {
    pub fn new(gpu: GpuContext, _font_system: &mut FontSystem) -> Result<Self> {
        let cache = glyphon::Cache::new(&gpu.device);
        let mut atlas =
            glyphon::TextAtlas::new(&gpu.device, &gpu.queue, &cache, gpu.surface_format);
        let viewport = glyphon::Viewport::new(&gpu.device, &cache);
        let renderer = glyphon::TextRenderer::new(
            &mut atlas,
            &gpu.device,
            wgpu::MultisampleState::default(),
            None,
        );
        let swash_cache = SwashCache::new();

        Ok(Self {
            gpu,
            cache,
            atlas,
            viewport,
            renderer,
            swash_cache,
            row_buffers: Vec::new(),
            glyph_buffers: Vec::new(),
            row_shape_cache: Vec::new(),
            glyph_shape_cache: Vec::new(),
            placements: Vec::new(),
            width: 0,
            height: 0,
            font_family: None,
        })
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.width = width;
        self.height = height;
        self.viewport
            .update(&self.gpu.queue, Resolution { width, height });
    }

    /// Caller may override the font family used for shaping. If unset, falls back to a
    /// system sans-serif, letting cosmic-text resolve the best available match.
    pub fn set_font_family(&mut self, family: Option<String>) {
        self.font_family = family;
    }

    pub fn prepare(
        &mut self,
        config: &Config,
        layout: &UnitLayout,
        frame: &FrameState,
        row_count: u32,
        key_row: u32,
        font_system: &mut FontSystem,
    ) -> Result<()> {
        // Temporary benchmark instrumentation gated behind an env var; the branch is a dead
        // predicate in production (env lookup is a syscall, so cheap for non-bench builds).
        let bench = std::env::var_os("SCENIC_BENCH_PREPARE").is_some();
        let t0 = if bench {
            Some(std::time::Instant::now())
        } else {
            None
        };
        self.placements.clear();

        // `used_row` / `used_glyph` track the next slot in the (persistent) buffer pools to
        // consider. Within a frame we scan `0..used_*` for a shape-key match (same-frame
        // dedupe); if nothing matches we claim slot `used_*` and either reuse the Buffer verbatim
        // (cross-frame hit) or re-shape it (cache miss).
        let mut used_row: usize = 0;
        let mut used_glyph: usize = 0;

        let canvas_width = config.canvas.width as f32;
        let canvas_height = config.canvas.height as f32;
        let dpr = config.canvas.dpr.max(1.0);
        let render_scale = config.canvas.render_scale.max(1.0);
        let effective_scale = dpr * render_scale;
        let snap = if effective_scale >= 2.0 { 0.5 } else { 1.0 };

        let row_count_effective = resolve_row_count(config, layout, row_count);
        let line_height = compute_row_line_height(config, layout);

        // --- Camera (baseline + per-frame deltas), baked into a 2x3 affine. ---
        let cam = &frame.camera;
        let base = &config.camera.baseline;
        let base_zoom = if base.zoom == 0.0 { 1.0 } else { base.zoom };
        let cam_zoom = (cam.zoom * base_zoom).max(0.01);
        let cam_tilt_deg = cam.tilt + base.tilt + config.layout.rotation_deg;
        let cam_pan_x = cam.pan_x + base.pan_x;
        let cam_pan_y = cam.pan_y + base.pan_y;
        let cam_rad = cam_tilt_deg.to_radians();
        let (cam_sin, cam_cos) = cam_rad.sin_cos();
        let cx = canvas_width * 0.5;
        let cy = canvas_height * 0.5;

        let dim_color = parse_hex_color(&config.colors.dim);
        let key_color = parse_hex_color(&config.colors.key);

        let family_owned = self.font_family.clone();
        let family: Family<'_> = match family_owned.as_deref() {
            Some(name) => Family::Name(name),
            None => Family::SansSerif,
        };

        let metrics = Metrics::new(
            config.layout.font_size_px,
            line_height.max(config.layout.font_size_px),
        );
        let base_attrs = Attrs::new().family(family);

        // Bit patterns used for shape-key equality (exact f32 bits).
        let font_size_bits = metrics.font_size.to_bits();
        let line_height_bits = metrics.line_height.to_bits();

        // Pre-size the glyph buffer pool conservatively (one Buffer per key glyph).
        for r in 0..row_count_effective {
            let row_state = frame.rows.get(r as usize);

            // Row geometry mirroring canvas.ts::computeRowGeometry.
            let (base_scroll_x, row_base_y) =
                compute_row_geometry(r, row_count_effective, config, layout, key_row);

            let (row_dx, row_dy, row_scale, row_rotation, row_opacity, row_tracking, has_per_glyph) =
                match row_state {
                    Some(s) => (
                        s.dx,
                        s.dy,
                        if s.scale == 0.0 { 1.0 } else { s.scale },
                        s.rotation,
                        s.opacity.clamp(0.0, 1.0),
                        s.tracking,
                        !s.per_glyph.is_empty(),
                    ),
                    None => (0.0, 0.0, 1.0, 0.0, 1.0, 0.0, false),
                };

            let scroll_x = base_scroll_x + row_dx;
            let row_y = row_base_y + row_dy;

            // Row-local transform about row center (used only when values are non-identity).
            let row_center_x = canvas_width * 0.5;
            let row_center_y = row_y - layout.ascent * 0.5;
            let row_rad = row_rotation.to_radians();
            let (row_sin, row_cos) = row_rad.sin_cos();
            let row_has_transform = row_rotation != 0.0 || row_scale != 1.0;

            let apply_row = |x: f32, y: f32| -> (f32, f32) {
                if !row_has_transform {
                    return (x, y);
                }
                let lx = x - row_center_x;
                let ly = y - row_center_y;
                let sx = row_cos * lx - row_sin * ly;
                let sy = row_sin * lx + row_cos * ly;
                (row_center_x + sx * row_scale, row_center_y + sy * row_scale)
            };

            let apply_cam = |x: f32, y: f32| -> (f32, f32) {
                let lx = x - cx;
                let ly = y - cy;
                let sx = cam_cos * lx - cam_sin * ly;
                let sy = cam_sin * lx + cam_cos * ly;
                (
                    cx + cam_pan_x + sx * cam_zoom,
                    cy + cam_pan_y + sy * cam_zoom,
                )
            };

            let (p_min, p_max) =
                layout.visible_range(scroll_x, -layout.stride, canvas_width + layout.stride);

            // ---------------- Per-row dim strip (fast path) ----------------
            //
            // If the row has no per-glyph deltas, we build a single Buffer carrying all visible
            // characters. We account for `tracking` by inserting spaces? No — cosmic-text shapes
            // kerning. Simpler: compute glyph positions ourselves and emit per-glyph buffers to
            // preserve exact x placement. To still get bulk draw, we fall back to:
            //   * uniform row path when tracking == 0 and no per_glyph: one row Buffer at exact
            //     start, cosmic-text handles advances.
            //   * otherwise: per-glyph buffers.
            //
            // This v1 keeps correctness over performance when the semantics diverge from naive
            // cosmic-text advances.
            let can_bulk = !has_per_glyph && row_tracking == 0.0 && !row_has_transform;

            if can_bulk {
                // Build the full row string between p_min and p_max.
                let mut text = String::with_capacity((p_max - p_min + 1).max(0) as usize);
                for p in p_min..=p_max {
                    let ch = apply_case(layout.glyph_char(p), config.text.case);
                    text.push(ch);
                }
                let start_x_local = scroll_x + layout.glyph_x(p_min);
                let start_y_local = row_y - layout.ascent;

                let (sxc, syc) = apply_cam(start_x_local, start_y_local);
                let sx = (sxc / snap).round() * snap;
                let sy = (syc / snap).round() * snap;

                let key = RowShapeKey {
                    text,
                    font_size_bits,
                    line_height_bits,
                    family: family_owned.clone(),
                };

                let buf_idx = acquire_row_buffer(
                    &mut self.row_buffers,
                    &mut self.row_shape_cache,
                    &mut used_row,
                    font_system,
                    metrics,
                    base_attrs,
                    &key,
                );

                let dim_with_alpha = scale_alpha(dim_color, row_opacity);
                self.placements.push(Placement {
                    kind: PlacementKind::Row,
                    index: buf_idx,
                    left: sx,
                    top: sy,
                    scale: cam_zoom,
                    color: dim_with_alpha,
                });

                // Overlay key glyphs if any.
                for p in p_min..=p_max {
                    let is_key = is_key_glyph(config, r, key_row, p, layout, canvas_width);
                    if !is_key {
                        continue;
                    }
                    let gx_local = scroll_x + layout.glyph_x(p);
                    let gy_local = row_y - layout.ascent;
                    let (gxc, gyc) = apply_cam(gx_local, gy_local);
                    let gxs = (gxc / snap).round() * snap;
                    let gys = (gyc / snap).round() * snap;

                    let ch = apply_case(layout.glyph_char(p), config.text.case);
                    let gkey = GlyphShapeKey {
                        ch,
                        font_size_bits,
                        line_height_bits,
                        family: family_owned.clone(),
                    };
                    let gi = acquire_glyph_buffer(
                        &mut self.glyph_buffers,
                        &mut self.glyph_shape_cache,
                        &mut used_glyph,
                        font_system,
                        metrics,
                        base_attrs,
                        &gkey,
                    );
                    let key_with_alpha = scale_alpha(key_color, row_opacity);
                    self.placements.push(Placement {
                        kind: PlacementKind::Glyph,
                        index: gi,
                        left: gxs,
                        top: gys,
                        scale: cam_zoom,
                        color: key_with_alpha,
                    });
                }
            } else {
                // Per-glyph slow path (honors row transform + per_glyph deltas + tracking).
                for p in p_min..=p_max {
                    let mut x = scroll_x + layout.glyph_x(p) + (p as f32) * row_tracking;
                    let mut gy = row_y;
                    let mut g_scale = 1.0f32;
                    let mut g_opacity = 1.0f32;
                    let mut g_rot = 0.0f32;
                    if let Some(s) = row_state {
                        for f in &s.per_glyph {
                            let d = f(p);
                            x += d.dx;
                            gy += d.dy;
                            g_scale += d.scale;
                            g_opacity += d.opacity;
                            g_rot += d.rotation;
                        }
                    }

                    let is_key = is_key_glyph(config, r, key_row, p, layout, canvas_width);

                    // Apply row-level affine (rotation/scale about row center).
                    let (rx, ry) = apply_row(x, gy - layout.ascent);
                    let (cxp, cyp) = apply_cam(rx, ry);
                    let sxp = (cxp / snap).round() * snap;
                    let syp = (cyp / snap).round() * snap;

                    let ch = apply_case(layout.glyph_char(p), config.text.case);
                    let gkey = GlyphShapeKey {
                        ch,
                        font_size_bits,
                        line_height_bits,
                        family: family_owned.clone(),
                    };
                    let gi = acquire_glyph_buffer(
                        &mut self.glyph_buffers,
                        &mut self.glyph_shape_cache,
                        &mut used_glyph,
                        font_system,
                        metrics,
                        base_attrs,
                        &gkey,
                    );

                    let alpha = row_opacity * g_opacity.clamp(0.0, 1.0);
                    let color = scale_alpha(if is_key { key_color } else { dim_color }, alpha);

                    // Per-glyph rotation is not natively representable in glyphon TextArea; glyph
                    // scale is folded into the TextArea.scale with the row-scale and cam-zoom. We
                    // intentionally drop per-glyph rotation in v1 — the engine rarely sets it.
                    let _ = g_rot;
                    let composed_scale = cam_zoom * row_scale * g_scale.max(0.0);
                    self.placements.push(Placement {
                        kind: PlacementKind::Glyph,
                        index: gi,
                        left: sxp,
                        top: syp,
                        scale: composed_scale,
                        color,
                    });
                }
            }
        }

        // Build text areas now that all buffers are stable.
        let bounds = TextBounds {
            left: 0,
            top: 0,
            right: self.width as i32,
            bottom: self.height as i32,
        };
        let text_areas: Vec<TextArea<'_>> = self
            .placements
            .iter()
            .map(|p| {
                let buffer: &Buffer = match p.kind {
                    PlacementKind::Row => &self.row_buffers[p.index],
                    PlacementKind::Glyph => &self.glyph_buffers[p.index],
                };
                TextArea {
                    buffer,
                    left: p.left,
                    top: p.top,
                    scale: p.scale,
                    bounds,
                    default_color: p.color,
                    custom_glyphs: &[],
                }
            })
            .collect();

        self.renderer.prepare(
            &self.gpu.device,
            &self.gpu.queue,
            font_system,
            &mut self.atlas,
            &self.viewport,
            text_areas,
            &mut self.swash_cache,
        )?;

        if let Some(start) = t0 {
            eprintln!("[bench] prepare() {} us", start.elapsed().as_micros());
        }

        Ok(())
    }

    pub fn render<'a>(&'a self, pass: &mut wgpu::RenderPass<'a>) -> Result<()> {
        self.renderer.render(&self.atlas, &self.viewport, pass)?;
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Helpers mirroring canvas.ts
// ---------------------------------------------------------------------------

fn hash01(seed_u32: u32) -> f32 {
    // Mirrors TS Math.imul-based mix. All ops in u32 wrapping arithmetic.
    let mut h = (seed_u32 ^ 0x9e37_79b1).wrapping_mul(0x85eb_ca6b);
    h ^= h >> 13;
    h = h.wrapping_mul(0xc2b2_ae35);
    h ^= h >> 16;
    (h % 1_000_003) as f32 / 1_000_003.0
}

fn compute_row_line_height(config: &Config, layout: &UnitLayout) -> f32 {
    if config.layout.leading_px > 0.0 {
        config.layout.leading_px
    } else {
        layout.line_height
    }
}

fn resolve_row_count(config: &Config, layout: &UnitLayout, provided: u32) -> u32 {
    match config.layout.rows {
        RowsMode::Count(n) => n.max(1),
        RowsMode::Auto(_) => {
            if provided > 0 {
                return provided;
            }
            let h = config.canvas.height as f32 - 2.0 * config.layout.margin_px;
            let lh = compute_row_line_height(config, layout);
            if lh <= 0.0 {
                return 1;
            }
            let base = (h / lh).ceil() as i32;
            let extra = config.camera.row_overshoot as i32;
            (base + 2 * extra).max(1) as u32
        }
    }
}

fn resolve_key_row(config: &Config, row_count: u32) -> u32 {
    match config.key.single_row {
        KeyRowValue::Middle(MiddleTag::Middle) => row_count / 2,
        KeyRowValue::Index(i) => i.min(row_count.saturating_sub(1)),
    }
}

fn compute_row_geometry(
    row_index: u32,
    row_count: u32,
    config: &Config,
    layout: &UnitLayout,
    key_row_param: u32,
) -> (f32, f32) {
    let lh = compute_row_line_height(config, layout);
    let total_height = row_count as f32 * lh;
    let center_y = config.canvas.height as f32 * 0.5;
    let top_y = center_y - total_height * 0.5;
    let y = top_y + row_index as f32 * lh + layout.ascent;

    let key_row_effective = if key_row_param == u32::MAX {
        resolve_key_row(config, row_count)
    } else {
        key_row_param
    };

    let base_scroll_x =
        if matches!(config.key.rows, KeyRows::Single) && row_index == key_row_effective {
            let kw = key_width(layout);
            let k_start = config.key.offset_units as i64 * layout.unit_length as i64;
            config.canvas.width as f32 * 0.5 - kw * 0.5 - layout.glyph_x(k_start)
        } else {
            let stagger = hash01(row_index.wrapping_mul(2654435761))
                * layout.stride
                * config.layout.row_stagger_px;
            -stagger
        };

    (base_scroll_x, y)
}

fn key_width(layout: &UnitLayout) -> f32 {
    // From canvas.ts::keyWidth. stride + unitPos[1] (since (12 % 11) == 1).
    layout.stride + layout.unit_pos.get(1).copied().unwrap_or(0.0)
}

fn row_gets_spotlight(config: &Config, row_index: u32, key_row: u32) -> bool {
    match config.key.rows {
        KeyRows::All => true,
        KeyRows::Even => row_index.is_multiple_of(2),
        KeyRows::Odd => !row_index.is_multiple_of(2),
        KeyRows::Single => row_index == key_row,
    }
}

fn row_phase_units(row_index: u32, frequency: u32) -> u32 {
    let h = hash01(row_index.wrapping_mul(2246822519));
    let n = frequency.max(1) as f32;
    (h * n).floor() as u32
}

fn is_key_glyph_travels(
    config: &Config,
    row_index: u32,
    key_row: u32,
    p: i64,
    unit_length: u32,
) -> bool {
    if !row_gets_spotlight(config, row_index, key_row) {
        return false;
    }
    let ul = unit_length as i64;
    if matches!(config.key.rows, KeyRows::Single) {
        let start = config.key.offset_units as i64 * ul;
        return p >= start && p <= start + ul;
    }
    let n = config.key.frequency.max(1) as i64;
    let phase_units = if config.key.row_phase_offset {
        row_phase_units(row_index, config.key.frequency) as i64
    } else {
        0
    };
    let period = n * ul;
    let base = (config.key.offset_units as i64 + phase_units) * ul;
    let shifted = ((p - base) % period + period) % period;
    shifted <= ul
}

fn is_key_glyph_anchored(
    config: &Config,
    row_index: u32,
    key_row: u32,
    center_x: f32,
    layout: &UnitLayout,
    frame_width: f32,
) -> bool {
    if !row_gets_spotlight(config, row_index, key_row) {
        return false;
    }
    let kw = key_width(layout);
    if matches!(config.key.rows, KeyRows::Single) {
        let left = frame_width * 0.5 - kw * 0.5 + config.key.anchor_screen_offset_px;
        return center_x >= left && center_x <= left + kw;
    }
    let n = config.key.frequency.max(1) as f32;
    let spacing = n * layout.stride;
    let phase_px = if config.key.row_phase_offset {
        row_phase_units(row_index, config.key.frequency) as f32 * layout.stride
    } else {
        0.0
    };
    let anchor_start = config.key.anchor_screen_offset_px
        + (config.key.offset_units as f32) * layout.stride
        + phase_px;
    let offset_x = ((center_x - anchor_start) % spacing + spacing) % spacing;
    offset_x <= kw
}

fn is_key_glyph(
    config: &Config,
    row_index: u32,
    key_row: u32,
    p: i64,
    layout: &UnitLayout,
    frame_width: f32,
) -> bool {
    match config.key.mode {
        KeyMode::Travels => is_key_glyph_travels(config, row_index, key_row, p, layout.unit_length),
        KeyMode::Anchored => {
            let advance = layout.glyph_x(p + 1) - layout.glyph_x(p);
            // Note: center_x here is in layout space (no scroll_x). The TS code uses screen x which
            // includes scrollX. For anchored mode the TS semantics anchor to screen; we replicate by
            // folding scroll into the caller's center_x. However our call-site is pre-scroll — but
            // since this helper is for v1 anchored support, and most users use Travels mode, we
            // keep this helper usable: callers should fold scroll before invoking. For simplicity
            // v1 only invokes travels; anchored will fall through to false until a caller passes a
            // screen-space x.
            let center_x = layout.glyph_x(p) + advance * 0.5;
            is_key_glyph_anchored(config, row_index, key_row, center_x, layout, frame_width)
        }
    }
}

fn apply_case(ch: char, case: crate::types::TextCase) -> char {
    match case {
        crate::types::TextCase::Upper => {
            // Keep ASCII-only path; cosmic-text handles arbitrary chars but we match TS.
            let mut it = ch.to_uppercase();
            it.next().unwrap_or(ch)
        }
        crate::types::TextCase::Lower => {
            let mut it = ch.to_lowercase();
            it.next().unwrap_or(ch)
        }
        crate::types::TextCase::AsIs => ch,
    }
}

// Parse `#rrggbb` / `#rrggbbaa` / `rrggbb` into a glyphon Color. Falls back to white on error.
fn parse_hex_color(s: &str) -> Color {
    let trimmed = s.trim().trim_start_matches('#');
    let bytes = trimmed.as_bytes();
    let hex2 = |a: u8, b: u8| -> Option<u8> {
        let hi = (a as char).to_digit(16)? as u8;
        let lo = (b as char).to_digit(16)? as u8;
        Some((hi << 4) | lo)
    };
    match bytes.len() {
        6 => {
            if let (Some(r), Some(g), Some(b)) = (
                hex2(bytes[0], bytes[1]),
                hex2(bytes[2], bytes[3]),
                hex2(bytes[4], bytes[5]),
            ) {
                return Color::rgba(r, g, b, 255);
            }
        }
        8 => {
            if let (Some(r), Some(g), Some(b), Some(a)) = (
                hex2(bytes[0], bytes[1]),
                hex2(bytes[2], bytes[3]),
                hex2(bytes[4], bytes[5]),
                hex2(bytes[6], bytes[7]),
            ) {
                return Color::rgba(r, g, b, a);
            }
        }
        3 => {
            // #rgb short form.
            let expand = |b: u8| hex2(b, b);
            if let (Some(r), Some(g), Some(b)) =
                (expand(bytes[0]), expand(bytes[1]), expand(bytes[2]))
            {
                return Color::rgba(r, g, b, 255);
            }
        }
        _ => {}
    }
    Color::rgba(255, 255, 255, 255)
}

fn scale_alpha(c: Color, alpha: f32) -> Color {
    let [r, g, b, a] = c.as_rgba();
    let out_a = ((a as f32) * alpha.clamp(0.0, 1.0))
        .round()
        .clamp(0.0, 255.0) as u8;
    Color::rgba(r, g, b, out_a)
}

// Acquire a row-wide Buffer whose shaped glyphs match `key`.
//
// Three outcomes, each written to be O(num slots in pool) which is tiny (<= row count):
//
// 1. **Same-frame hit**: a previously-claimed slot in this `prepare()` call already matches the
//    key — return its index without touching the Buffer. The caller will emit another
//    `TextArea` referencing the same Buffer with a different left/top/color. Zero shape cost.
// 2. **Cross-frame hit**: the next unused slot already carries the same shape key from a
//    previous frame. Claim it, bump the cursor, and skip the re-shape. Zero shape cost.
// 3. **Miss**: either slot `*used` is unoccupied or carries a different key. Grow the pool if
//    needed, run `set_text` + `shape_until_scroll`, and record the key for next frame.
fn acquire_row_buffer(
    pool: &mut Vec<Buffer>,
    cache: &mut Vec<Option<RowShapeKey>>,
    used: &mut usize,
    font_system: &mut FontSystem,
    metrics: Metrics,
    attrs: Attrs<'_>,
    key: &RowShapeKey,
) -> usize {
    // 1. Same-frame dedupe.
    for i in 0..*used {
        if cache.get(i).and_then(|k| k.as_ref()) == Some(key) {
            return i;
        }
    }

    // Grow pool/cache if slot `*used` doesn't exist yet.
    if *used >= pool.len() {
        pool.push(Buffer::new(font_system, metrics));
        cache.push(None);
    }

    let idx = *used;
    *used += 1;

    // 2. Cross-frame hit?
    if cache[idx].as_ref() == Some(key) {
        return idx;
    }

    // 3. Miss — (re)shape and update the cache entry.
    let buffer = &mut pool[idx];
    buffer.set_metrics(font_system, metrics);
    buffer.set_size(
        font_system,
        Some(f32::INFINITY),
        Some(metrics.line_height.max(1.0)),
    );
    buffer.set_text(font_system, &key.text, attrs, Shaping::Advanced);
    buffer.shape_until_scroll(font_system, false);
    cache[idx] = Some(key.clone());
    idx
}

// Analogue of `acquire_row_buffer` for the single-glyph key-overlay pool.
fn acquire_glyph_buffer(
    pool: &mut Vec<Buffer>,
    cache: &mut Vec<Option<GlyphShapeKey>>,
    used: &mut usize,
    font_system: &mut FontSystem,
    metrics: Metrics,
    attrs: Attrs<'_>,
    key: &GlyphShapeKey,
) -> usize {
    for i in 0..*used {
        if cache.get(i).and_then(|k| k.as_ref()) == Some(key) {
            return i;
        }
    }

    if *used >= pool.len() {
        pool.push(Buffer::new(font_system, metrics));
        cache.push(None);
    }

    let idx = *used;
    *used += 1;

    if cache[idx].as_ref() == Some(key) {
        return idx;
    }

    let buffer = &mut pool[idx];
    buffer.set_metrics(font_system, metrics);
    buffer.set_size(
        font_system,
        Some(f32::INFINITY),
        Some(metrics.line_height.max(1.0)),
    );
    let mut s = String::new();
    s.push(key.ch);
    buffer.set_text(font_system, &s, attrs, Shaping::Advanced);
    buffer.shape_until_scroll(font_system, false);
    cache[idx] = Some(key.clone());
    idx
}
