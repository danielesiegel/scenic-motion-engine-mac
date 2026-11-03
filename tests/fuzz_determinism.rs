//! Fuzz-style determinism test. Hammer the engine with 100 random (t, row_count, key_row)
//! triples backed by synthetic `AnalysisData` carrying 200 random kicks + 200 random snares.
//! Verify:
//!
//!  1. For each triple: three consecutive `evaluate()` calls produce bit-identical row scalars.
//!  2. Evaluating all 100 triples in sorted-by-t order vs. shuffled order yields the same
//!     per-triple outputs — proving no cross-call state leaks (engine is a pure function of t).
//!
//! Uses a fixed seed so failures are reproducible.

use rand::prelude::*;
use rand::rngs::StdRng;
use rand::SeedableRng;
use scenic_motion_engine::audio::event_source::JsonEventSource;
use scenic_motion_engine::engine::{evaluate, EngineInputs};
use scenic_motion_engine::motion::paths::register_default_paths;
use scenic_motion_engine::types::*;
use std::collections::HashMap;

#[path = "common/mod.rs"]
mod common;

use common::{base_config, fake_layout};

const SEED: u64 = 0x5CE_1C0DE_u64;
const TRIPLES: usize = 100;
const EVENTS_PER_BAND: usize = 200;
const T_MAX: f32 = 300.0;

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

fn scalars(state: &FrameState) -> Vec<RowScalars> {
    state.rows.iter().map(pick).collect()
}

fn random_events(rng: &mut StdRng, n: usize) -> Vec<TransientEvent> {
    let mut v: Vec<TransientEvent> = (0..n)
        .map(|_| TransientEvent {
            t: rng.gen_range(0.0..T_MAX),
            strength: rng.gen_range(0.0..1.0),
        })
        .collect();
    v.sort_by(|a, b| a.t.partial_cmp(&b.t).unwrap_or(std::cmp::Ordering::Equal));
    v
}

fn build_source(rng: &mut StdRng) -> JsonEventSource {
    let mut bands: HashMap<String, Vec<TransientEvent>> = HashMap::new();
    bands.insert("kick".into(), random_events(rng, EVENTS_PER_BAND));
    bands.insert("snare".into(), random_events(rng, EVENTS_PER_BAND));
    bands.insert("mid".into(), Vec::new());
    bands.insert("hats".into(), Vec::new());
    JsonEventSource::from_analysis(AnalysisData {
        track: "fuzz".into(),
        bpm: 120.0,
        duration: T_MAX,
        sample_rate: 44100,
        beats: Vec::new(),
        downbeats: Vec::new(),
        bands,
        envelopes: None,
    })
}

/// Config with a kick-driven slideStep + a snare-driven slideStep. Keeps the hot
/// code path broad: multiple tickers, multiple bindings, direction alternation.
fn fuzz_config() -> Config {
    use std::collections::BTreeMap as Map;
    let mut cfg = base_config();
    cfg.tickers = vec![
        TickerConfig {
            id: "k".into(),
            band: Band::Kick,
            modulo: 1,
            strength_threshold: Some(0.0),
            offset: None,
            step: None,
            step_offset: None,
        },
        TickerConfig {
            id: "s".into(),
            band: Band::Snare,
            modulo: 2,
            strength_threshold: Some(0.0),
            offset: None,
            step: None,
            step_offset: None,
        },
    ];

    let mut p_k: Params = Map::new();
    p_k.insert("units".into(), ParamValue::Number(1.0));
    let mut p_s: Params = Map::new();
    p_s.insert("units".into(), ParamValue::Number(0.75));

    cfg.bindings = vec![
        Binding {
            ticker_id: "k".into(),
            target: BindingTarget::Rows,
            rows: Some(RowSelector::Name(RowSelectorName::All)),
            sequence: PhaseSequence {
                actions: vec![PhaseAction {
                    path: "slideStep".into(),
                    params: p_k,
                    duration: 0.25,
                    easing: Some(Easing::Named(EasingName::EaseOutCubic)),
                }],
            },
            compose: ComposeMode::Add,
            respect_alternation: Some(true),
        },
        Binding {
            ticker_id: "s".into(),
            target: BindingTarget::Rows,
            rows: Some(RowSelector::Name(RowSelectorName::All)),
            sequence: PhaseSequence {
                actions: vec![PhaseAction {
                    path: "slideStep".into(),
                    params: p_s,
                    duration: 0.30,
                    easing: Some(Easing::Named(EasingName::EaseOutCubic)),
                }],
            },
            compose: ComposeMode::Add,
            respect_alternation: Some(false),
        },
    ];
    cfg.feel.row_velocity_shape = RowVelocityShape::Flat;
    cfg
}

#[derive(Clone, Copy, Debug)]
struct Triple {
    t: f32,
    row_count: u32,
    key_row: u32,
}

fn gen_triples(rng: &mut StdRng) -> Vec<Triple> {
    (0..TRIPLES)
        .map(|_| {
            let row_count = rng.gen_range(1..=24_u32);
            let key_row = rng.gen_range(0..row_count);
            Triple {
                t: rng.gen_range(0.0..T_MAX),
                row_count,
                key_row,
            }
        })
        .collect()
}

fn eval_triple(
    tr: Triple,
    cfg: &Config,
    src: &JsonEventSource,
    layout: &UnitLayout,
) -> Vec<RowScalars> {
    let state = evaluate(EngineInputs {
        t: tr.t,
        config: cfg,
        source: src,
        layout,
        row_count: tr.row_count,
        key_row: tr.key_row,
    });
    scalars(&state)
}

#[test]
fn fuzz_three_calls_match_per_triple() {
    register_default_paths();
    let mut rng = StdRng::seed_from_u64(SEED);
    let src = build_source(&mut rng);
    let cfg = fuzz_config();
    let layout = fake_layout();
    let triples = gen_triples(&mut rng);

    for (i, tr) in triples.iter().copied().enumerate() {
        let a = eval_triple(tr, &cfg, &src, &layout);
        let b = eval_triple(tr, &cfg, &src, &layout);
        let c = eval_triple(tr, &cfg, &src, &layout);
        assert_eq!(a, b, "triple #{} a vs b differ: {:?}", i, tr);
        assert_eq!(b, c, "triple #{} b vs c differ: {:?}", i, tr);
    }
}

#[test]
fn fuzz_order_independence_sorted_vs_shuffled() {
    register_default_paths();
    let mut rng = StdRng::seed_from_u64(SEED);
    let src = build_source(&mut rng);
    let cfg = fuzz_config();
    let layout = fake_layout();
    let triples = gen_triples(&mut rng);

    // Sorted ascending by t.
    let mut sorted = triples.clone();
    sorted.sort_by(|a, b| a.t.partial_cmp(&b.t).unwrap_or(std::cmp::Ordering::Equal));

    // Shuffled order (independent RNG keyed off same seed+1).
    let mut shuffled = triples.clone();
    let mut shuffle_rng = StdRng::seed_from_u64(SEED.wrapping_add(1));
    shuffled.shuffle(&mut shuffle_rng);

    // Collect outputs keyed by triple identity (idx of first occurrence in `triples`).
    // Because t can repeat, use the original index as the key: tag each triple before
    // reordering.
    let tagged: Vec<(usize, Triple)> = triples.iter().copied().enumerate().collect();
    let mut tagged_sorted = tagged.clone();
    tagged_sorted.sort_by(|a, b| {
        a.1.t
            .partial_cmp(&b.1.t)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let mut tagged_shuffled = tagged.clone();
    tagged_shuffled.shuffle(&mut StdRng::seed_from_u64(SEED.wrapping_add(2)));

    let mut sorted_out: Vec<Option<Vec<RowScalars>>> = vec![None; TRIPLES];
    for (idx, tr) in &tagged_sorted {
        sorted_out[*idx] = Some(eval_triple(*tr, &cfg, &src, &layout));
    }
    let mut shuffled_out: Vec<Option<Vec<RowScalars>>> = vec![None; TRIPLES];
    for (idx, tr) in &tagged_shuffled {
        shuffled_out[*idx] = Some(eval_triple(*tr, &cfg, &src, &layout));
    }

    for i in 0..TRIPLES {
        let s = sorted_out[i].as_ref().expect("sorted missing");
        let sh = shuffled_out[i].as_ref().expect("shuffled missing");
        assert_eq!(s, sh, "ordering leak at triple #{}: sorted != shuffled", i);
    }
}
