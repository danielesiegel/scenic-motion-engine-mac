//! Benchmarks for the pure engine hot path: `evaluate(EngineInputs)`.
//!
//! Quantifies the perf win over the TypeScript prototype. Three scenarios probe
//! how cost scales with the number of past events the engine has to replay:
//!   - `evaluate_t_start` (t=0.1): few/no past events, baseline cost.
//!   - `evaluate_t_mid`   (t=180.0): ~360 kicks accumulated at 120 BPM.
//!   - `evaluate_t_end`   (t=360.0): ~720 kicks accumulated at 120 BPM.
//!
//! Default row_count for the auto layout isn't computed here; we pick 10 rows
//! to match the practical display, keeping the row-fanout cost realistic.

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use scenic_motion_engine::audio::SyntheticEventSource;
use scenic_motion_engine::config::defaults::default_config;
use scenic_motion_engine::engine::{evaluate, EngineInputs};
use scenic_motion_engine::motion::paths::register_default_paths;
use scenic_motion_engine::types::UnitLayout;

fn fake_layout() -> UnitLayout {
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

fn bench_evaluate(c: &mut Criterion) {
    register_default_paths();

    let config = default_config();
    let source = SyntheticEventSource::default();
    let layout = fake_layout();
    let row_count: u32 = 10;
    let key_row: u32 = row_count / 2;

    c.bench_function("evaluate_t_start", |b| {
        b.iter(|| {
            let fs = evaluate(EngineInputs {
                t: 0.1,
                config: &config,
                source: &source,
                layout: &layout,
                row_count,
                key_row,
            });
            black_box(fs)
        })
    });

    c.bench_function("evaluate_t_mid", |b| {
        b.iter(|| {
            let fs = evaluate(EngineInputs {
                t: 180.0,
                config: &config,
                source: &source,
                layout: &layout,
                row_count,
                key_row,
            });
            black_box(fs)
        })
    });

    c.bench_function("evaluate_t_end", |b| {
        b.iter(|| {
            let fs = evaluate(EngineInputs {
                t: 360.0,
                config: &config,
                source: &source,
                layout: &layout,
                row_count,
                key_row,
            });
            black_box(fs)
        })
    });
}

criterion_group!(benches, bench_evaluate);
criterion_main!(benches);
