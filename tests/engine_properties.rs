//! Property-ish integration checks over the engine.
//!
//! - No NaN/Infinity ever in rows or camera.
//! - respect_alternation=true + row-only binding alternates row sign.
//! - Camera zoom stays in a sensible range: zoom * baseline.zoom > 0.1.
//! - Advancing t by 1 ns doesn't change rows[].dx by more than 10 * stride.

use scenic_motion_engine::audio::event_source::JsonEventSource;
use scenic_motion_engine::config::defaults::default_config;
use scenic_motion_engine::engine::{evaluate, EngineInputs};
use scenic_motion_engine::motion::paths::register_default_paths;
use scenic_motion_engine::types::*;
use std::collections::HashMap;

#[path = "common/mod.rs"]
mod common;

use common::{fake_layout, make_source, slide_step_config};

const ROW_COUNT: u32 = 11;
const KEY_ROW: u32 = 5;

fn finite_frame(state: &FrameState, t: f32, cfg_label: &str) {
    for (i, r) in state.rows.iter().enumerate() {
        for (label, v) in [
            ("dx", r.dx),
            ("dy", r.dy),
            ("scale", r.scale),
            ("rotation", r.rotation),
            ("opacity", r.opacity),
            ("tracking", r.tracking),
        ] {
            assert!(
                v.is_finite(),
                "[{}] non-finite row[{}].{} at t={}: {}",
                cfg_label,
                i,
                label,
                t,
                v
            );
        }
    }
    for (label, v) in [
        ("zoom", state.camera.zoom),
        ("pan_x", state.camera.pan_x),
        ("pan_y", state.camera.pan_y),
        ("tilt", state.camera.tilt),
    ] {
        assert!(
            v.is_finite(),
            "[{}] non-finite camera.{} at t={}: {}",
            cfg_label,
            label,
            t,
            v
        );
    }
}

/// Dense 120 BPM-ish synthetic analysis for stress testing t sweeps.
fn dense_source(duration: f32) -> JsonEventSource {
    let beat = 0.5f32;
    let mut kick = Vec::new();
    let mut snare = Vec::new();
    let mut hats = Vec::new();
    let mut mid = Vec::new();
    let mut t = 0.0;
    while t < duration {
        kick.push(TransientEvent { t, strength: 1.0 });
        hats.push(TransientEvent {
            t: t + 0.25,
            strength: 1.0,
        });
        t += beat;
    }
    let mut t = beat;
    while t < duration {
        snare.push(TransientEvent { t, strength: 1.0 });
        t += 2.0 * beat;
    }
    let mut t = 0.0;
    while t < duration {
        mid.push(TransientEvent { t, strength: 1.0 });
        t += 4.0 * beat;
    }
    let mut bands = HashMap::new();
    bands.insert("kick".into(), kick);
    bands.insert("snare".into(), snare);
    bands.insert("mid".into(), mid);
    bands.insert("hats".into(), hats);
    JsonEventSource::from_analysis(AnalysisData {
        track: "props".into(),
        bpm: 120.0,
        duration,
        sample_rate: 44100,
        beats: Vec::new(),
        downbeats: Vec::new(),
        bands,
        envelopes: None,
    })
}

#[test]
fn evaluate_never_produces_nan_or_infinity_default_config() {
    register_default_paths();
    let cfg = default_config();
    let layout = fake_layout();
    let src = dense_source(60.0);
    // Sweep t across a representative range.
    let ts: Vec<f32> = (0..=600).map(|i| i as f32 * 0.1).collect();
    for &t in &ts {
        let state = evaluate(EngineInputs {
            t,
            config: &cfg,
            source: &src,
            layout: &layout,
            row_count: ROW_COUNT,
            key_row: KEY_ROW,
        });
        finite_frame(&state, t, "default");
    }
}

#[test]
fn evaluate_never_produces_nan_or_infinity_slide_configs() {
    register_default_paths();
    let layout = fake_layout();
    let src = dense_source(30.0);
    for respect in [false, true] {
        let cfg = slide_step_config(respect);
        let label = if respect { "slide-alt" } else { "slide-noalt" };
        for i in 0..=300 {
            let t = i as f32 * 0.1;
            let state = evaluate(EngineInputs {
                t,
                config: &cfg,
                source: &src,
                layout: &layout,
                row_count: 7,
                key_row: 3,
            });
            finite_frame(&state, t, label);
        }
    }
}

#[test]
fn respect_alternation_true_row_only_pairs_equal_magnitude_opposite_sign() {
    register_default_paths();
    let cfg = slide_step_config(true);
    let layout = fake_layout();
    let src = make_source(vec![
        TransientEvent {
            t: 0.0,
            strength: 1.0,
        },
        TransientEvent {
            t: 0.5,
            strength: 1.0,
        },
        TransientEvent {
            t: 1.1,
            strength: 1.0,
        },
    ]);
    // Sweep several time points to exercise active + frozen contributions.
    for &t in &[0.1f32, 0.25, 0.5, 0.75, 1.0, 1.2, 1.5, 2.0, 3.0] {
        let state = evaluate(EngineInputs {
            t,
            config: &cfg,
            source: &src,
            layout: &layout,
            row_count: 8,
            key_row: 4,
        });
        for i in 0..state.rows.len() - 1 {
            if i % 2 == 0 {
                let a = state.rows[i].dx;
                let b = state.rows[i + 1].dx;
                assert!(
                    (a + b).abs() <= 1e-3 * (a.abs() + b.abs() + 1.0),
                    "row {} and {} not opposite-sign at t={}: {} vs {}",
                    i,
                    i + 1,
                    t,
                    a,
                    b
                );
                assert!(
                    (a.abs() - b.abs()).abs() <= 1e-3 * (a.abs() + 1.0),
                    "row {} and {} magnitude mismatch at t={}: |{}| vs |{}|",
                    i,
                    i + 1,
                    t,
                    a,
                    b
                );
            }
        }
    }
}

#[test]
fn camera_zoom_times_baseline_above_floor() {
    register_default_paths();
    let cfg = default_config();
    let baseline_zoom = cfg.camera.baseline.zoom;
    let layout = fake_layout();
    let src = dense_source(90.0);
    for i in 0..=900 {
        let t = i as f32 * 0.1;
        let state = evaluate(EngineInputs {
            t,
            config: &cfg,
            source: &src,
            layout: &layout,
            row_count: ROW_COUNT,
            key_row: KEY_ROW,
        });
        let effective = state.camera.zoom * baseline_zoom;
        assert!(
            effective > 0.1,
            "effective zoom collapsed at t={}: zoom={} * baseline={} = {}",
            t,
            state.camera.zoom,
            baseline_zoom,
            effective
        );
    }
}

#[test]
fn advancing_t_by_one_ns_does_not_cause_jumps() {
    register_default_paths();
    let cfg = default_config();
    let layout = fake_layout();
    let src = dense_source(60.0);
    let stride = layout.stride;
    let max_delta = 10.0 * stride;

    // Hit a dense grid plus right around every synthetic kick to catch
    // discontinuity at event-boundary transitions.
    let mut ts: Vec<f32> = (0..=1200).map(|i| i as f32 * 0.05).collect();
    for i in 0..60 {
        let base = 0.5 * i as f32;
        for d in [-1e-6f32, -1e-9, 0.0, 1e-9, 1e-6] {
            ts.push((base + d).max(0.0));
        }
    }
    for t in ts {
        let s0 = evaluate(EngineInputs {
            t,
            config: &cfg,
            source: &src,
            layout: &layout,
            row_count: ROW_COUNT,
            key_row: KEY_ROW,
        });
        let s1 = evaluate(EngineInputs {
            t: t + 1e-9,
            config: &cfg,
            source: &src,
            layout: &layout,
            row_count: ROW_COUNT,
            key_row: KEY_ROW,
        });
        for i in 0..s0.rows.len() {
            let d = (s1.rows[i].dx - s0.rows[i].dx).abs();
            assert!(
                d <= max_delta,
                "row[{}].dx jumped {} > {} over 1ns at t={} (before={}, after={})",
                i,
                d,
                max_delta,
                t,
                s0.rows[i].dx,
                s1.rows[i].dx
            );
        }
    }
}
