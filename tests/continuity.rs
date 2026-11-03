//! Port of `scenicsynths/tests/continuity.test.ts`.
//!
//! The frozen-contribution-across-phases model: once an event is superseded
//! by the next event, its action is "frozen" at `(gap / duration)` progress.

use approx::assert_relative_eq;
use scenic_motion_engine::engine::{evaluate, EngineInputs};
use scenic_motion_engine::motion::paths::register_default_paths;
use scenic_motion_engine::types::*;
use std::collections::BTreeMap;

#[path = "common/mod.rs"]
mod common;

use common::{base_config, fake_layout, make_source};

fn config_two_phase() -> Config {
    register_default_paths();
    let mut cfg = base_config();
    cfg.tickers = vec![TickerConfig {
        id: "k".into(),
        band: Band::Kick,
        modulo: 2,
        strength_threshold: Some(0.0),
        offset: None,
        step: None,
        step_offset: None,
    }];
    let mut p0: Params = BTreeMap::new();
    p0.insert("units".into(), ParamValue::Number(1.0));
    let mut p1: Params = BTreeMap::new();
    p1.insert("units".into(), ParamValue::Number(0.5));
    cfg.bindings = vec![Binding {
        ticker_id: "k".into(),
        target: BindingTarget::Rows,
        rows: Some(RowSelector::Indices(vec![0])),
        sequence: PhaseSequence {
            actions: vec![
                PhaseAction {
                    path: "slideStep".into(),
                    params: p0,
                    duration: 0.5,
                    easing: Some(Easing::Named(EasingName::EaseOutCubic)),
                },
                PhaseAction {
                    path: "reverseNudge".into(),
                    params: p1,
                    duration: 0.5,
                    easing: Some(Easing::Named(EasingName::EaseOutCubic)),
                },
            ],
        },
        compose: ComposeMode::Add,
        respect_alternation: Some(false),
    }];
    cfg
}

fn three_kicks() -> scenic_motion_engine::audio::event_source::JsonEventSource {
    make_source(vec![
        TransientEvent {
            t: 0.0,
            strength: 1.0,
        },
        TransientEvent {
            t: 1.0,
            strength: 1.0,
        },
        TransientEvent {
            t: 2.0,
            strength: 1.0,
        },
    ])
}

#[test]
fn settles_to_plus_100_at_end_of_first_action() {
    register_default_paths();
    let cfg = config_two_phase();
    let layout = fake_layout();
    let src = three_kicks();
    let state = evaluate(EngineInputs {
        t: 0.5,
        config: &cfg,
        source: &src,
        layout: &layout,
        row_count: 1,
        key_row: 0,
    });
    assert_relative_eq!(state.rows[0].dx, 100.0, max_relative = 0.01);
}

#[test]
fn continuity_at_t_1_the_new_action_contributes_0_prior_frozen_is_plus_100() {
    register_default_paths();
    let cfg = config_two_phase();
    let layout = fake_layout();
    let src = three_kicks();
    let state = evaluate(EngineInputs {
        t: 1.0,
        config: &cfg,
        source: &src,
        layout: &layout,
        row_count: 1,
        key_row: 0,
    });
    assert_relative_eq!(state.rows[0].dx, 100.0, max_relative = 0.01);
}

#[test]
fn at_t_1_5_the_second_reverse_action_is_fully_settled_plus_50() {
    register_default_paths();
    let cfg = config_two_phase();
    let layout = fake_layout();
    let src = three_kicks();
    let state = evaluate(EngineInputs {
        t: 1.5,
        config: &cfg,
        source: &src,
        layout: &layout,
        row_count: 1,
        key_row: 0,
    });
    assert_relative_eq!(state.rows[0].dx, 50.0, max_relative = 0.01);
}

#[test]
fn truncation_freezes_prior_action_at_gap_over_duration_with_easing() {
    // Single-phase slideStep (0.5s, +100). Events at t=0, t=0.25.
    // First event: frozen progress = 0.25/0.5 = 0.5 -> easeOutCubic(0.5) = 1 - (1-0.5)^3 = 0.875
    //   -> dx contribution = 100 * 0.875 = 87.5.
    // Second event: progress = 0 -> dx = 0. Total = 87.5.
    register_default_paths();
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
        rows: Some(RowSelector::Indices(vec![0])),
        sequence: PhaseSequence {
            actions: vec![PhaseAction {
                path: "slideStep".into(),
                params: p,
                duration: 0.5,
                easing: Some(Easing::Named(EasingName::EaseOutCubic)),
            }],
        },
        compose: ComposeMode::Add,
        respect_alternation: Some(false),
    }];
    let layout = fake_layout();
    let src = make_source(vec![
        TransientEvent {
            t: 0.0,
            strength: 1.0,
        },
        TransientEvent {
            t: 0.25,
            strength: 1.0,
        },
    ]);
    let state = evaluate(EngineInputs {
        t: 0.25,
        config: &cfg,
        source: &src,
        layout: &layout,
        row_count: 1,
        key_row: 0,
    });
    assert_relative_eq!(state.rows[0].dx, 87.5, max_relative = 0.01);
}
