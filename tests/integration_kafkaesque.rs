//! Integration test against the real `kafkaesque` analysis JSON.
//!
//! This test loads the production analysis file from `../scenicsynths/analysis/kafkaesque.json`
//! and verifies that the engine evaluates sanely at several timestamps — camera within expected
//! bounds, no NaN/Inf in row outputs, determinism across three identical calls, and beat counts
//! matching the ~144 BPM of the track.
//!
//! If the analysis file is missing (clean checkout, isolated CI), the test SKIPS cleanly
//! instead of panicking.

use scenic_motion_engine::audio::event_source::JsonEventSource;
use scenic_motion_engine::config::defaults::default_config;
use scenic_motion_engine::engine::{evaluate, EngineInputs};
use scenic_motion_engine::motion::paths::register_default_paths;
use scenic_motion_engine::types::*;
use std::path::Path;

#[path = "common/mod.rs"]
mod common;

use common::fake_layout;

const ANALYSIS_PATH: &str = "../scenicsynths/analysis/kafkaesque.json";
const ROW_COUNT: u32 = 11;
const KEY_ROW: u32 = 5;

/// Scalar snapshot used for structural equality assertions across repeated calls.
#[derive(Debug, Clone, Copy, PartialEq)]
struct RowScalars {
    dx: f32,
    dy: f32,
    scale: f32,
    rotation: f32,
    opacity: f32,
    tracking: f32,
}

fn pick(r: &RowMotionState) -> RowScalars {
    RowScalars {
        dx: r.dx,
        dy: r.dy,
        scale: r.scale,
        rotation: r.rotation,
        opacity: r.opacity,
        tracking: r.tracking,
    }
}

fn finite(v: f32, label: &str, t: f32, row: usize) {
    assert!(
        v.is_finite(),
        "non-finite {} at t={} row={}: {}",
        label,
        t,
        row,
        v
    );
}

fn evaluate_at<'a>(
    t: f32,
    cfg: &'a Config,
    src: &'a JsonEventSource,
    layout: &'a UnitLayout,
) -> FrameState {
    evaluate(EngineInputs {
        t,
        config: cfg,
        source: src,
        layout,
        row_count: ROW_COUNT,
        key_row: KEY_ROW,
    })
}

#[test]
fn kafkaesque_integration_bounds_and_sanity() {
    if !Path::new(ANALYSIS_PATH).exists() {
        eprintln!(
            "skipping kafkaesque_integration_bounds_and_sanity: {} not present",
            ANALYSIS_PATH
        );
        return;
    }
    register_default_paths();
    let src = JsonEventSource::load(ANALYSIS_PATH).expect("load kafkaesque analysis");
    let cfg = default_config();
    let layout = fake_layout();

    for &t in &[0.0_f32, 60.0, 180.0, 360.0] {
        let state = evaluate_at(t, &cfg, &src, &layout);
        assert_eq!(
            state.rows.len() as u32,
            ROW_COUNT,
            "row count mismatch at t={}",
            t
        );

        // Camera bounds. zoom in [0.9, 1.11], tilt in [-20, 20].
        finite(state.camera.zoom, "camera.zoom", t, usize::MAX);
        finite(state.camera.tilt, "camera.tilt", t, usize::MAX);
        finite(state.camera.pan_x, "camera.pan_x", t, usize::MAX);
        finite(state.camera.pan_y, "camera.pan_y", t, usize::MAX);
        assert!(
            state.camera.zoom >= 0.9 && state.camera.zoom <= 1.11,
            "camera.zoom out of range at t={}: {}",
            t,
            state.camera.zoom
        );
        assert!(
            state.camera.tilt >= -20.0 && state.camera.tilt <= 20.0,
            "camera.tilt out of range at t={}: {}",
            t,
            state.camera.tilt
        );

        // Finite row scalars.
        for (i, r) in state.rows.iter().enumerate() {
            finite(r.dx, "dx", t, i);
            finite(r.dy, "dy", t, i);
            finite(r.scale, "scale", t, i);
            finite(r.rotation, "rotation", t, i);
            finite(r.opacity, "opacity", t, i);
        }
    }
}

#[test]
fn kafkaesque_integration_determinism_at_t_180() {
    if !Path::new(ANALYSIS_PATH).exists() {
        eprintln!(
            "skipping kafkaesque_integration_determinism_at_t_180: {} not present",
            ANALYSIS_PATH
        );
        return;
    }
    register_default_paths();
    let src = JsonEventSource::load(ANALYSIS_PATH).expect("load kafkaesque analysis");
    let cfg = default_config();
    let layout = fake_layout();

    let s1 = evaluate_at(180.0, &cfg, &src, &layout);
    let s2 = evaluate_at(180.0, &cfg, &src, &layout);
    let s3 = evaluate_at(180.0, &cfg, &src, &layout);

    let r1: Vec<_> = s1.rows.iter().map(pick).collect();
    let r2: Vec<_> = s2.rows.iter().map(pick).collect();
    let r3: Vec<_> = s3.rows.iter().map(pick).collect();

    assert_eq!(r1, r2, "s1 vs s2 row scalars differ");
    assert_eq!(r2, r3, "s2 vs s3 row scalars differ");

    // Also check camera determinism.
    assert_eq!(s1.camera.zoom, s2.camera.zoom);
    assert_eq!(s1.camera.zoom, s3.camera.zoom);
    assert_eq!(s1.camera.tilt, s2.camera.tilt);
    assert_eq!(s1.camera.tilt, s3.camera.tilt);
}

#[test]
fn kafkaesque_integration_beat_count_at_t_60() {
    if !Path::new(ANALYSIS_PATH).exists() {
        eprintln!(
            "skipping kafkaesque_integration_beat_count_at_t_60: {} not present",
            ANALYSIS_PATH
        );
        return;
    }
    let src = JsonEventSource::load(ANALYSIS_PATH).expect("load kafkaesque analysis");
    let beats = src.beats_before(60.0);
    // ~144 BPM over 60s → 144 beats. Allow a wide band for pickup/outro gaps.
    assert!(
        (100..=180).contains(&beats),
        "beats_before(60.0) = {} outside expected [100, 180]",
        beats
    );
}
