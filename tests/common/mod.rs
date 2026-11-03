//! Shared fixtures for integration tests. Mirrors the TS helpers in
//! `scenicsynths/tests/*.test.ts`.
//!
//! NOTE: Rust treats every file under `tests/` as its own crate. Modules under
//! `tests/common/` are shared via `#[path = "common/mod.rs"] mod common;`
//! from each test file. We keep this module small and dependency-light.

#![allow(dead_code)]

use scenic_motion_engine::audio::event_source::JsonEventSource;
use scenic_motion_engine::config::defaults::default_config;
use scenic_motion_engine::motion::paths::register_default_paths;
use scenic_motion_engine::types::*;
use std::collections::{BTreeMap, HashMap};

pub fn fake_layout() -> UnitLayout {
    UnitLayout {
        unit: "scenicsynth".into(),
        unit_length: 11,
        stride: 100.0,
        unit_pos: (0..11).map(|i| i as f32 * 10.0).collect(),
        ascent: 50.0,
        descent: 10.0,
        line_height: 60.0,
        size: 72.0,
    }
}

pub fn make_source(kick: Vec<TransientEvent>) -> JsonEventSource {
    let mut bands = HashMap::new();
    bands.insert("kick".into(), kick);
    bands.insert("snare".into(), Vec::new());
    bands.insert("mid".into(), Vec::new());
    bands.insert("hats".into(), Vec::new());
    JsonEventSource::from_analysis(AnalysisData {
        track: "test".into(),
        bpm: 120.0,
        duration: 20.0,
        sample_rate: 44100,
        beats: Vec::new(),
        downbeats: Vec::new(),
        bands,
        envelopes: None,
    })
}

/// Baseline config with cleared tickers/bindings and all band thresholds at 0.
/// Mirrors the TS `baseConfig()` helper used by the continuity/determinism tests.
pub fn base_config() -> Config {
    register_default_paths();
    let mut cfg = default_config();
    cfg.tickers = Vec::new();
    cfg.bindings = Vec::new();
    cfg.band_thresholds.clear();
    cfg.band_thresholds.insert("kick".into(), 0.0);
    cfg.band_thresholds.insert("snare".into(), 0.0);
    cfg.band_thresholds.insert("mid".into(), 0.0);
    cfg.band_thresholds.insert("hats".into(), 0.0);
    cfg
}

/// Single-phase slideStep binding (duration 0.25, units=1, easeOutCubic),
/// targeting rows: "all". `respect_alternation` flips the alternation flag.
/// Row velocity shape is forced flat so alternation / determinism tests
/// aren't contaminated by the U curve from `default_config`.
pub fn slide_step_config(respect_alternation: bool) -> Config {
    let mut cfg = base_config();
    cfg.tickers = vec![TickerConfig {
        id: "k".into(),
        band: Band::Kick,
        modulo: 1,
        strength_threshold: Some(0.0),
        offset: None,
        step: None,
        step_offset: None,
    }];
    let mut p: Params = BTreeMap::new();
    p.insert("units".into(), ParamValue::Number(1.0));
    cfg.bindings = vec![Binding {
        ticker_id: "k".into(),
        target: BindingTarget::Rows,
        rows: Some(RowSelector::Name(RowSelectorName::All)),
        sequence: PhaseSequence {
            actions: vec![PhaseAction {
                path: "slideStep".into(),
                params: p,
                duration: 0.25,
                easing: Some(Easing::Named(EasingName::EaseOutCubic)),
            }],
        },
        compose: ComposeMode::Add,
        respect_alternation: Some(respect_alternation),
    }];
    cfg.feel.row_velocity_shape = RowVelocityShape::Flat;
    cfg
}
