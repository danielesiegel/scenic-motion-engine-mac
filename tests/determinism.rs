//! Port of `scenicsynths/tests/determinism.test.ts`.
//!
//! Covers:
//!  - Three evaluate() calls at the same t produce structurally equal rows
//!  - Walking through intermediate t values doesn't leak state
//!  - Row velocity U-curve shape with edge=2 for rowCount=5
//!  - Row velocity 'flat' leaves rows at 1x regardless of edge
//!  - respectAlternation: true alternates row sign
//!  - respectAlternation: false shares the same sign

use approx::assert_relative_eq;
use scenic_motion_engine::engine::{evaluate, EngineInputs};
use scenic_motion_engine::motion::paths::register_default_paths;
use scenic_motion_engine::types::*;

#[path = "common/mod.rs"]
mod common;

use common::{fake_layout, make_source, slide_step_config};

/// Scalars we compare per row (excludes the per_glyph closure vector).
#[derive(Debug, Clone, Copy)]
struct RowScalars {
    dx: f32,
    dy: f32,
    scale: f32,
    rotation: f32,
    opacity: f32,
    tracking: f32,
}

fn pick_scalars(r: &RowMotionState) -> RowScalars {
    RowScalars {
        dx: r.dx,
        dy: r.dy,
        scale: r.scale,
        rotation: r.rotation,
        opacity: r.opacity,
        tracking: r.tracking,
    }
}

fn assert_rows_close(a: &[RowScalars], b: &[RowScalars]) {
    assert_eq!(a.len(), b.len(), "row count mismatch");
    for (i, (ra, rb)) in a.iter().zip(b.iter()).enumerate() {
        // Compare each scalar — allow near-zero exact ties.
        assert_relative_eq!(ra.dx, rb.dx, max_relative = 1e-6, epsilon = 1e-6);
        assert_relative_eq!(ra.dy, rb.dy, max_relative = 1e-6, epsilon = 1e-6);
        assert_relative_eq!(ra.scale, rb.scale, max_relative = 1e-6, epsilon = 1e-6);
        assert_relative_eq!(
            ra.rotation,
            rb.rotation,
            max_relative = 1e-6,
            epsilon = 1e-6
        );
        assert_relative_eq!(ra.opacity, rb.opacity, max_relative = 1e-6, epsilon = 1e-6);
        assert_relative_eq!(
            ra.tracking,
            rb.tracking,
            max_relative = 1e-6,
            epsilon = 1e-6
        );
        let _ = i;
    }
}

#[test]
fn three_calls_at_same_t_produce_structurally_equal_rows() {
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
    let t = 1.2;
    let s1 = evaluate(EngineInputs {
        t,
        config: &cfg,
        source: &src,
        layout: &layout,
        row_count: 4,
        key_row: 2,
    });
    let s2 = evaluate(EngineInputs {
        t,
        config: &cfg,
        source: &src,
        layout: &layout,
        row_count: 4,
        key_row: 2,
    });
    let s3 = evaluate(EngineInputs {
        t,
        config: &cfg,
        source: &src,
        layout: &layout,
        row_count: 4,
        key_row: 2,
    });

    let r1: Vec<_> = s1.rows.iter().map(pick_scalars).collect();
    let r2: Vec<_> = s2.rows.iter().map(pick_scalars).collect();
    let r3: Vec<_> = s3.rows.iter().map(pick_scalars).collect();
    assert_rows_close(&r1, &r2);
    assert_rows_close(&r2, &r3);
}

#[test]
fn no_mutation_leaks_between_calls_direct_vs_after_walk() {
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

    let direct = evaluate(EngineInputs {
        t: 1.5,
        config: &cfg,
        source: &src,
        layout: &layout,
        row_count: 4,
        key_row: 2,
    });
    // Walk through t=0.1, 0.8 then evaluate at 1.5.
    let _ = evaluate(EngineInputs {
        t: 0.1,
        config: &cfg,
        source: &src,
        layout: &layout,
        row_count: 4,
        key_row: 2,
    });
    let _ = evaluate(EngineInputs {
        t: 0.8,
        config: &cfg,
        source: &src,
        layout: &layout,
        row_count: 4,
        key_row: 2,
    });
    let after_walk = evaluate(EngineInputs {
        t: 1.5,
        config: &cfg,
        source: &src,
        layout: &layout,
        row_count: 4,
        key_row: 2,
    });

    let a: Vec<_> = direct.rows.iter().map(pick_scalars).collect();
    let b: Vec<_> = after_walk.rows.iter().map(pick_scalars).collect();
    assert_rows_close(&a, &b);
}

#[test]
fn u_shape_edge_2_scales_edges_by_2x_center_by_1x_for_row_count_5() {
    register_default_paths();
    let mut cfg = slide_step_config(false);
    cfg.feel.row_velocity_shape = RowVelocityShape::U;
    cfg.feel.row_velocity_edge_scale = 2.0;
    let layout = fake_layout();
    let src = make_source(vec![TransientEvent {
        t: 0.0,
        strength: 1.0,
    }]);
    let state = evaluate(EngineInputs {
        t: 0.25,
        config: &cfg,
        source: &src,
        layout: &layout,
        row_count: 5,
        key_row: 2,
    });
    // At t=0.25 with duration=0.25, easeOutCubic(1)=1 -> baseline dx=100.
    // U-curve: scale = 1 + (edge-1)*(2p-1)^2, p = i/(n-1).
    // p = [0, 0.25, 0.5, 0.75, 1] -> (2p-1)^2 = [1, 0.25, 0, 0.25, 1] -> scale = [2, 1.25, 1, 1.25, 2].
    assert_relative_eq!(state.rows[0].dx, 200.0, max_relative = 0.01);
    assert_relative_eq!(state.rows[1].dx, 125.0, max_relative = 0.01);
    assert_relative_eq!(state.rows[2].dx, 100.0, max_relative = 0.01);
    assert_relative_eq!(state.rows[3].dx, 125.0, max_relative = 0.01);
    assert_relative_eq!(state.rows[4].dx, 200.0, max_relative = 0.01);
}

#[test]
fn flat_shape_leaves_rows_at_1x_regardless_of_edge_value() {
    register_default_paths();
    let mut cfg = slide_step_config(false);
    cfg.feel.row_velocity_shape = RowVelocityShape::Flat;
    cfg.feel.row_velocity_edge_scale = 3.0;
    let layout = fake_layout();
    let src = make_source(vec![TransientEvent {
        t: 0.0,
        strength: 1.0,
    }]);
    let state = evaluate(EngineInputs {
        t: 0.25,
        config: &cfg,
        source: &src,
        layout: &layout,
        row_count: 5,
        key_row: 2,
    });
    for r in &state.rows {
        assert_relative_eq!(r.dx, 100.0, max_relative = 0.01);
    }
}

#[test]
fn respect_alternation_true_alternates_row_sign() {
    register_default_paths();
    let cfg = slide_step_config(true);
    let layout = fake_layout();
    let src = make_source(vec![TransientEvent {
        t: 0.0,
        strength: 1.0,
    }]);
    let state = evaluate(EngineInputs {
        t: 0.25,
        config: &cfg,
        source: &src,
        layout: &layout,
        row_count: 4,
        key_row: 2,
    });
    assert_relative_eq!(state.rows[0].dx, 100.0, max_relative = 0.01);
    assert_relative_eq!(state.rows[1].dx, -100.0, max_relative = 0.01);
    assert_relative_eq!(state.rows[2].dx, 100.0, max_relative = 0.01);
    assert_relative_eq!(state.rows[3].dx, -100.0, max_relative = 0.01);
}

#[test]
fn respect_alternation_false_rows_share_sign() {
    register_default_paths();
    let cfg = slide_step_config(false);
    let layout = fake_layout();
    let src = make_source(vec![TransientEvent {
        t: 0.0,
        strength: 1.0,
    }]);
    let state = evaluate(EngineInputs {
        t: 0.25,
        config: &cfg,
        source: &src,
        layout: &layout,
        row_count: 4,
        key_row: 2,
    });
    assert_relative_eq!(state.rows[0].dx, 100.0, max_relative = 0.01);
    assert_relative_eq!(state.rows[1].dx, 100.0, max_relative = 0.01);
    assert_relative_eq!(state.rows[2].dx, 100.0, max_relative = 0.01);
    assert_relative_eq!(state.rows[3].dx, 100.0, max_relative = 0.01);
}
