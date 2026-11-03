//! Benchmark `build_unit_layout` — the cosmic-text shaping cost.
//!
//! Relevant because the TypeScript renderer re-shaped per frame. In the Rust port
//! we shape once at startup (and whenever font size changes), so this measurement
//! is the one-shot cost that replaces the per-frame TS shaping bill.

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use scenic_motion_engine::text::build_unit_layout;
use scenic_motion_engine::text::FontLoader;

fn bench_build_unit_layout(c: &mut Criterion) {
    c.bench_function("build_unit_layout", |b| {
        b.iter_batched(
            // Fresh FontLoader per iteration so cosmic-text's internal caches don't
            // make the second-and-onwards iterations trivially cheap. This measures
            // the realistic cold-shape cost the engine pays at startup.
            FontLoader::new,
            |mut loader| {
                let layout = build_unit_layout(&mut loader, 72.0, "scenicsynth");
                black_box(layout)
            },
            criterion::BatchSize::SmallInput,
        )
    });
}

criterion_group!(benches, bench_build_unit_layout);
criterion_main!(benches);
