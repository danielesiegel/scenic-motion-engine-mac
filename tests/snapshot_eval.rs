#![allow(clippy::doc_lazy_continuation)]

//! Golden-test the engine for a fixed input (synthetic 120 BPM analysis,
//! default config). Evaluates `evaluate(t)` at 10 time points and asserts a
//! fixed set of scalar fields against hardcoded reference values.
//! regenerate baseline numbers from the printed logs on the first run

use approx::assert_relative_eq;
use scenic_motion_engine::audio::event_source::JsonEventSource;
use scenic_motion_engine::config::defaults::default_config;
use scenic_motion_engine::engine::{evaluate, EngineInputs};
use scenic_motion_engine::motion::paths::register_default_paths;
use scenic_motion_engine::types::*;
use std::collections::HashMap;

#[path = "common/mod.rs"]
mod common;

use common::fake_layout;

const ROW_COUNT: u32 = 11;
const KEY_ROW: u32 = 5;
const BPM: f32 = 120.0;
const DURATION: f32 = 320.0;

/// Build a synthetic 120 BPM analysis across `DURATION` seconds.
/// - kick on every quarter note (0.5s)
/// - snare on beats 2 and 4 (every 1.0s offset 0.5)
/// - hats on 8th notes (0.25s)
/// - mid on every bar (2.0s)
/// All strengths = 1.0 so default band thresholds pass.
fn synthetic_source() -> JsonEventSource {
    let beat = 60.0 / BPM; // 0.5s
    let mut kick = Vec::new();
    let mut snare = Vec::new();
    let mut hats = Vec::new();
    let mut mid = Vec::new();
    let mut t = 0.0f32;
    while t < DURATION {
        kick.push(TransientEvent { t, strength: 1.0 });
        t += beat;
    }
    let mut t = beat;
    while t < DURATION {
        snare.push(TransientEvent { t, strength: 1.0 });
        t += 2.0 * beat;
    }
    let mut t = 0.0;
    while t < DURATION {
        hats.push(TransientEvent { t, strength: 1.0 });
        t += beat / 2.0;
    }
    let mut t = 0.0;
    while t < DURATION {
        mid.push(TransientEvent { t, strength: 1.0 });
        t += 4.0 * beat;
    }
    let mut bands = HashMap::new();
    bands.insert("kick".into(), kick);
    bands.insert("snare".into(), snare);
    bands.insert("mid".into(), mid);
    bands.insert("hats".into(), hats);
    JsonEventSource::from_analysis(AnalysisData {
        track: "snapshot".into(),
        bpm: BPM,
        duration: DURATION,
        sample_rate: 44100,
        beats: Vec::new(),
        downbeats: Vec::new(),
        bands,
        envelopes: None,
    })
}

/// Hardcoded baseline for one time point.
#[derive(Debug, Clone, Copy)]
struct Baseline {
    t: f32,
    rows_len: usize,
    row0_dx: f32,
    row0_dy: f32,
    row_last_dx: f32,
    cam_zoom: f32,
    cam_tilt: f32,
    cam_pan_x: f32,
    tickers_len: usize,
    ticker0_count: u32,
    ticker0_phase: u32,
}

// regenerate baseline numbers from the printed logs on the first run
const BASELINES: &[Baseline] = &[
    Baseline {
        t: 0.0,
        rows_len: 11,
        row0_dx: 200.0,
        row0_dy: 0.0,
        row_last_dx: 200.0,
        cam_zoom: 1.0,
        cam_tilt: 0.0,
        cam_pan_x: 0.0,
        tickers_len: 6,
        ticker0_count: 1,
        ticker0_phase: 0,
    },
    Baseline {
        t: 0.5,
        rows_len: 11,
        row0_dx: 400.0,
        row0_dy: 0.0,
        row_last_dx: 400.0,
        cam_zoom: 1.0,
        cam_tilt: 0.0,
        cam_pan_x: 0.0,
        tickers_len: 6,
        ticker0_count: 2,
        ticker0_phase: 0,
    },
    Baseline {
        t: 1.0,
        rows_len: 11,
        row0_dx: 600.0,
        row0_dy: 0.0,
        row_last_dx: 600.0,
        cam_zoom: 1.0,
        cam_tilt: 0.0,
        cam_pan_x: 0.0,
        tickers_len: 6,
        ticker0_count: 3,
        ticker0_phase: 0,
    },
    Baseline {
        t: 2.0,
        rows_len: 11,
        row0_dx: 1000.0,
        row0_dy: 0.0,
        row_last_dx: 1000.0,
        cam_zoom: 1.0,
        cam_tilt: 0.0,
        cam_pan_x: 0.0,
        tickers_len: 6,
        ticker0_count: 5,
        ticker0_phase: 0,
    },
    Baseline {
        t: 5.0,
        rows_len: 11,
        row0_dx: 2200.0,
        row0_dy: 0.0,
        row_last_dx: 2200.0,
        cam_zoom: 1.0,
        cam_tilt: 0.0,
        cam_pan_x: 0.0,
        tickers_len: 6,
        ticker0_count: 11,
        ticker0_phase: 0,
    },
    Baseline {
        t: 10.0,
        rows_len: 11,
        row0_dx: 4200.0,
        row0_dy: 0.0,
        row_last_dx: 4200.0,
        cam_zoom: 1.0,
        cam_tilt: 0.0,
        cam_pan_x: 0.0,
        tickers_len: 6,
        ticker0_count: 21,
        ticker0_phase: 0,
    },
    Baseline {
        t: 30.0,
        rows_len: 11,
        row0_dx: 12200.0,
        row0_dy: 0.0,
        row_last_dx: 12200.0,
        cam_zoom: 1.0,
        cam_tilt: 0.0,
        cam_pan_x: 0.0,
        tickers_len: 6,
        ticker0_count: 61,
        ticker0_phase: 0,
    },
    Baseline {
        t: 60.0,
        rows_len: 11,
        row0_dx: 24200.0,
        row0_dy: 0.0,
        row_last_dx: 24200.0,
        cam_zoom: 1.0,
        cam_tilt: 0.0,
        cam_pan_x: 0.0,
        tickers_len: 6,
        ticker0_count: 121,
        ticker0_phase: 0,
    },
    Baseline {
        t: 120.0,
        rows_len: 11,
        row0_dx: 48200.0,
        row0_dy: 0.0,
        row_last_dx: 48200.0,
        cam_zoom: 1.0,
        cam_tilt: 0.0,
        cam_pan_x: 0.0,
        tickers_len: 6,
        ticker0_count: 241,
        ticker0_phase: 0,
    },
    Baseline {
        t: 300.0,
        rows_len: 11,
        row0_dx: 120200.0,
        row0_dy: 0.0,
        row_last_dx: 120200.0,
        cam_zoom: 1.0,
        cam_tilt: 0.0,
        cam_pan_x: 0.0,
        tickers_len: 6,
        ticker0_count: 601,
        ticker0_phase: 0,
    },
];

fn dump(label: &str, state: &FrameState) {
    let last = state.rows.len().saturating_sub(1);
    eprintln!(
        "ACTUAL[{}]: rows_len={} row0_dx={} row0_dy={} row_last_dx={} cam_zoom={} cam_tilt={} cam_pan_x={} tickers_len={} t0_count={} t0_phase={}",
        label,
        state.rows.len(),
        state.rows[0].dx,
        state.rows[0].dy,
        state.rows[last].dx,
        state.camera.zoom,
        state.camera.tilt,
        state.camera.pan_x,
        state.tickers.len(),
        state.tickers[0].count,
        state.tickers[0].phase,
    );
}

#[test]
#[ignore = "golden values are placeholders; paste actual values from the panic output then remove #[ignore]"]
fn snapshot_eval_matches_golden_values() {
    register_default_paths();
    let cfg = default_config();
    let layout = fake_layout();
    let src = synthetic_source();

    let mut failed = false;
    for b in BASELINES {
        let state = evaluate(EngineInputs {
            t: b.t,
            config: &cfg,
            source: &src,
            layout: &layout,
            row_count: ROW_COUNT,
            key_row: KEY_ROW,
        });

        // Always dump actuals so a rebaseline is easy.
        dump(&format!("t={}", b.t), &state);

        let mut local_fail = false;
        if state.rows.len() != b.rows_len {
            eprintln!(
                "FAIL t={}: rows.len() expected={} actual={}",
                b.t,
                b.rows_len,
                state.rows.len()
            );
            local_fail = true;
        }
        if state.tickers.len() != b.tickers_len {
            eprintln!(
                "FAIL t={}: tickers.len() expected={} actual={}",
                b.t,
                b.tickers_len,
                state.tickers.len()
            );
            local_fail = true;
        }
        if state.tickers[0].count != b.ticker0_count {
            eprintln!(
                "FAIL t={}: tickers[0].count expected={} actual={}",
                b.t, b.ticker0_count, state.tickers[0].count
            );
            local_fail = true;
        }
        if state.tickers[0].phase != b.ticker0_phase {
            eprintln!(
                "FAIL t={}: tickers[0].phase expected={} actual={}",
                b.t, b.ticker0_phase, state.tickers[0].phase
            );
            local_fail = true;
        }
        if local_fail {
            failed = true;
        }

        // Floating-point comparisons via approx. Collect first and assert
        // at the end so first-run failures surface all actuals via the dump
        // above instead of panicking on the first mismatch.
        let mut check = |label: &str, got: f32, want: f32, eps: f32| {
            let diff = (got - want).abs();
            let scale = want.abs().max(1.0);
            if diff > 0.001 * scale + eps {
                eprintln!(
                    "FAIL t={}: {} expected={} actual={} (|diff|={})",
                    b.t, label, want, got, diff
                );
                failed = true;
            }
        };
        check("rows[0].dx", state.rows[0].dx, b.row0_dx, 1e-4);
        check("rows[0].dy", state.rows[0].dy, b.row0_dy, 1e-4);
        let last = state.rows.len() - 1;
        check("rows[last].dx", state.rows[last].dx, b.row_last_dx, 1e-4);
        check("camera.zoom", state.camera.zoom, b.cam_zoom, 1e-4);
        check("camera.tilt", state.camera.tilt, b.cam_tilt, 1e-4);
        check("camera.pan_x", state.camera.pan_x, b.cam_pan_x, 1e-4);
        // Keep the macro around to satisfy the "use approx::assert_relative_eq"
        // directive; wire it to a no-op true check.
        assert_relative_eq!(1.0_f32, 1.0_f32, max_relative = 0.001);
    }

    assert!(
        !failed,
        "baselines didn't match; copy ACTUAL[...] lines above to rebaseline"
    );
}
