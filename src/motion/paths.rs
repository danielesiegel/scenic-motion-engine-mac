//! Motion-path registry. Ports every path from `scenic-motion-engine-web/src/motion/paths.ts`.
//!
//! Contract: `register_default_paths()` populates the global registry with every path name used
//! in the default config. Lookups via `get_path(name)` return a `PathFn` or `None`.

use crate::types::{PathContext, PathFn, PathOutput, PerGlyphOutput};
use once_cell::sync::Lazy;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::f32::consts::PI;

static REGISTRY: Lazy<RwLock<HashMap<String, PathFn>>> = Lazy::new(|| RwLock::new(HashMap::new()));

pub fn register_path(name: &str, f: PathFn) {
    REGISTRY.write().insert(name.to_string(), f);
}

pub fn get_path(name: &str) -> Option<PathFn> {
    REGISTRY.read().get(name).copied()
}

pub fn list_paths() -> Vec<String> {
    let mut keys: Vec<String> = REGISTRY.read().keys().cloned().collect();
    keys.sort();
    keys
}

// ---------------------------------------------------------------------------
// Deterministic hash in [0,1), mirroring the TS `hash01` xmur3-ish scalar.
// JS semantics: Math.imul = i32 wrapping mul; `>>> 13`, `>>> 16`, `>>> 0` = unsigned shifts.
// ---------------------------------------------------------------------------

fn hash01(seed: u32) -> f32 {
    let mut h = (seed ^ 0x9e3779b1).wrapping_mul(0x85ebca6b);
    h ^= h >> 13;
    h = h.wrapping_mul(0xc2b2ae35);
    h ^= h >> 16;
    ((h % 1_000_003) as f32) / 1_000_003.0
}

/// Convert an arbitrary i64 seed value to the u32 that JS's `| 0` / Math.imul would see.
#[inline]
fn seed_u32(v: i64) -> u32 {
    v as u32
}

// ---------------------------------------------------------------------------
// Paths
// ---------------------------------------------------------------------------

fn path_hold(_ctx: &PathContext) -> PathOutput {
    PathOutput::default()
}

fn path_slide_step(ctx: &PathContext) -> PathOutput {
    let units = ctx.params.get("units").and_then(|v| v.as_f32()).unwrap_or(1.0);
    let unit_width = ctx
        .params
        .get("unitWidth")
        .and_then(|v| v.as_f32())
        .unwrap_or(ctx.unit_width);
    let target = units * unit_width;
    PathOutput {
        dx: target * ctx.eased_progress,
        ..Default::default()
    }
}

fn path_reverse_nudge(ctx: &PathContext) -> PathOutput {
    let units = ctx.params.get("units").and_then(|v| v.as_f32()).unwrap_or(0.5);
    let unit_width = ctx
        .params
        .get("unitWidth")
        .and_then(|v| v.as_f32())
        .unwrap_or(ctx.unit_width);
    let target = -units * unit_width;
    PathOutput {
        dx: target * ctx.eased_progress,
        ..Default::default()
    }
}

fn path_stutter(ctx: &PathContext) -> PathOutput {
    let count_raw = ctx.params.get("count").and_then(|v| v.as_f32()).unwrap_or(4.0);
    let count = (count_raw.floor() as i32).max(1) as f32;
    let amplitude = ctx
        .params
        .get("amplitude")
        .and_then(|v| v.as_f32())
        .unwrap_or(6.0);
    let envelope = 1.0 - ctx.eased_progress;
    let phase = ctx.eased_progress * count * 2.0 * PI;
    PathOutput {
        dx: phase.sin() * amplitude * envelope,
        ..Default::default()
    }
}

fn path_glyph_jitter(ctx: &PathContext) -> PathOutput {
    let amp_base = ctx
        .params
        .get("amplitude")
        .and_then(|v| v.as_f32())
        .unwrap_or(2.0);
    let amplitude = amp_base * ctx.strength;
    let envelope = 1.0 - ctx.eased_progress;
    let row = ctx.row_index as i64;
    let t_frame = (ctx.t * 60.0).floor() as i64;
    PathOutput {
        per_glyph: Some(Box::new(move |glyph_index: i64| {
            let seed_n = seed_u32(glyph_index.wrapping_mul(1013)
                .wrapping_add(row.wrapping_mul(7919))
                .wrapping_add(t_frame));
            let seed_m = seed_u32(glyph_index.wrapping_mul(7919)
                .wrapping_add(row.wrapping_mul(1013))
                .wrapping_add(t_frame)
                .wrapping_add(1));
            let n = hash01(seed_n);
            let m = hash01(seed_m);
            PerGlyphOutput {
                dx: (n - 0.5) * 2.0 * amplitude * envelope,
                dy: (m - 0.5) * 2.0 * amplitude * envelope,
                ..Default::default()
            }
        })),
        ..Default::default()
    }
}

fn path_drift_speed(ctx: &PathContext) -> PathOutput {
    let px_per_sec = ctx
        .params
        .get("pxPerSec")
        .and_then(|v| v.as_f32())
        .unwrap_or(0.0);
    let duration = ctx
        .params
        .get("duration")
        .and_then(|v| v.as_f32())
        .unwrap_or(1.0)
        .max(1e-6);
    PathOutput {
        dx: px_per_sec * duration * ctx.eased_progress,
        ..Default::default()
    }
}

fn path_scale_pulse(ctx: &PathContext) -> PathOutput {
    let magnitude = ctx
        .params
        .get("magnitude")
        .and_then(|v| v.as_f32())
        .unwrap_or(0.03)
        * ctx.strength;
    let envelope = (ctx.eased_progress * PI).sin();
    PathOutput {
        scale: magnitude * envelope,
        ..Default::default()
    }
}

fn path_tracking_pulse(ctx: &PathContext) -> PathOutput {
    let magnitude = ctx
        .params
        .get("magnitude")
        .and_then(|v| v.as_f32())
        .unwrap_or(4.0)
        * ctx.strength;
    let envelope = (ctx.eased_progress * PI).sin();
    PathOutput {
        tracking: magnitude * envelope,
        ..Default::default()
    }
}

fn path_opacity_dip(ctx: &PathContext) -> PathOutput {
    let depth = ctx
        .params
        .get("depth")
        .and_then(|v| v.as_f32())
        .unwrap_or(0.3)
        * ctx.strength;
    let envelope = (ctx.eased_progress * PI).sin();
    PathOutput {
        opacity: -depth * envelope,
        ..Default::default()
    }
}

fn path_rotation_flicker(ctx: &PathContext) -> PathOutput {
    let magnitude = ctx
        .params
        .get("magnitude")
        .and_then(|v| v.as_f32())
        .unwrap_or(2.0)
        * ctx.strength;
    let envelope = 1.0 - ctx.eased_progress;
    let phase = ctx.eased_progress * PI * 6.0;
    PathOutput {
        rotation: phase.sin() * magnitude * envelope,
        ..Default::default()
    }
}

fn path_camera_zoom_delta(ctx: &PathContext) -> PathOutput {
    let delta = ctx
        .params
        .get("deltaScale")
        .and_then(|v| v.as_f32())
        .unwrap_or(0.1);
    PathOutput {
        scale: delta * ctx.eased_progress,
        ..Default::default()
    }
}

fn path_camera_pan_delta(ctx: &PathContext) -> PathOutput {
    let dx = ctx
        .params
        .get("deltaPx")
        .and_then(|v| v.as_f32())
        .unwrap_or(60.0);
    PathOutput {
        dx: dx * ctx.eased_progress,
        ..Default::default()
    }
}

fn path_camera_pan_y_delta(ctx: &PathContext) -> PathOutput {
    let dy = ctx
        .params
        .get("deltaPx")
        .and_then(|v| v.as_f32())
        .unwrap_or(40.0);
    PathOutput {
        dy: dy * ctx.eased_progress,
        ..Default::default()
    }
}

fn path_camera_tilt_delta(ctx: &PathContext) -> PathOutput {
    let deg = ctx
        .params
        .get("deltaDegrees")
        .and_then(|v| v.as_f32())
        .unwrap_or(10.0);
    PathOutput {
        rotation: deg * ctx.eased_progress,
        ..Default::default()
    }
}

fn path_camera_zoom_pulse(ctx: &PathContext) -> PathOutput {
    let amp_base = ctx
        .params
        .get("amplitude")
        .and_then(|v| v.as_f32())
        .unwrap_or(0.08);
    let strength_mul = if ctx.strength != 0.0 { ctx.strength } else { 1.0 };
    let amp = amp_base * strength_mul;
    PathOutput {
        scale: (ctx.eased_progress * PI).sin() * amp,
        ..Default::default()
    }
}

fn path_camera_zoom_delta_focused(ctx: &PathContext) -> PathOutput {
    let delta = ctx
        .params
        .get("deltaScale")
        .and_then(|v| v.as_f32())
        .unwrap_or(0.1);
    let focus_frac = ctx
        .params
        .get("focusFrac")
        .and_then(|v| v.as_f32())
        .unwrap_or(0.6)
        .clamp(0.0, 1.0);
    let w = ctx
        .params
        .get("canvasWidth")
        .and_then(|v| v.as_f32())
        .unwrap_or(ctx.canvas_width.max(1920.0));
    let h = ctx
        .params
        .get("canvasHeight")
        .and_then(|v| v.as_f32())
        .unwrap_or(ctx.canvas_height.max(1080.0));
    let cx = w / 2.0;
    let cy = h / 2.0;

    let z = 1.0 + delta;
    if z <= 0.0 {
        return PathOutput {
            scale: delta * ctx.eased_progress,
            ..Default::default()
        };
    }

    let max_dx_bound = (w * (1.0 - 1.0 / z) / 2.0).abs();
    let max_dy_bound = (h * (1.0 - 1.0 / z) / 2.0).abs();
    let max_dx = max_dx_bound * focus_frac;
    let max_dy = max_dy_bound * focus_frac;

    let event_time = ctx.t - ctx.since_last;
    let seed = ((event_time * 1000.0).floor() as i64).wrapping_add(1);
    // JS: hash01(seed * 2654435761) and hash01(seed * 40503 + 1)
    // Math.imul truncates to i32 -> we mimic via u32 wrapping_mul.
    let seed_u = seed as u32;
    let sx = seed_u.wrapping_mul(2654435761);
    let sy = seed_u.wrapping_mul(40503).wrapping_add(1);
    let hx = hash01(sx);
    let hy = hash01(sy);
    let offset_x = (hx - 0.5) * 2.0 * max_dx;
    let offset_y = (hy - 0.5) * 2.0 * max_dy;
    let fx = cx + offset_x;
    let fy = cy + offset_y;

    let dx_target = (cx - fx) * delta;
    let dy_target = (cy - fy) * delta;
    PathOutput {
        scale: delta * ctx.eased_progress,
        dx: dx_target * ctx.eased_progress,
        dy: dy_target * ctx.eased_progress,
        ..Default::default()
    }
}

pub fn register_default_paths() {
    register_path("hold", path_hold);
    register_path("slideStep", path_slide_step);
    register_path("reverseNudge", path_reverse_nudge);
    register_path("stutter", path_stutter);
    register_path("glyphJitter", path_glyph_jitter);
    register_path("driftSpeed", path_drift_speed);
    register_path("scalePulse", path_scale_pulse);
    register_path("trackingPulse", path_tracking_pulse);
    register_path("opacityDip", path_opacity_dip);
    register_path("rotationFlicker", path_rotation_flicker);
    register_path("cameraZoomDelta", path_camera_zoom_delta);
    register_path("cameraPanDelta", path_camera_pan_delta);
    register_path("cameraPanYDelta", path_camera_pan_y_delta);
    register_path("cameraTiltDelta", path_camera_tilt_delta);
    register_path("cameraZoomPulse", path_camera_zoom_pulse);
    register_path("cameraZoomDeltaFocused", path_camera_zoom_delta_focused);
}
