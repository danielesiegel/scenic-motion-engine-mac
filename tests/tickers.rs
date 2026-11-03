//! Port of `scenicsynths/tests/tickers.test.ts`.
//!
//! Covers:
//!  - `JsonEventSource::count_before` monotonicity + threshold filtering
//!  - `JsonEventSource::event_at_index` under threshold filtering
//!  - Ticker phase cycling: `((latest_idx + offset) % modulo + modulo) % modulo`
//!  - Threshold-filtered events renumber the phase sequence

use scenic_motion_engine::types::{Band, EventSource, TransientEvent};

#[path = "common/mod.rs"]
mod common;

use common::make_source;

// ---------------------------------------------------------------------------
// count_before
// ---------------------------------------------------------------------------

#[test]
fn count_before_returns_0_before_any_event_and_increments_as_t_crosses_each_event() {
    let src = make_source(vec![
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
    ]);
    assert_eq!(src.count_before(Band::Kick, -0.1, 0.0), 0);
    assert_eq!(src.count_before(Band::Kick, 0.0, 0.0), 1);
    assert_eq!(src.count_before(Band::Kick, 0.5, 0.0), 1);
    assert_eq!(src.count_before(Band::Kick, 1.0, 0.0), 2);
    assert_eq!(src.count_before(Band::Kick, 1.999, 0.0), 2);
    assert_eq!(src.count_before(Band::Kick, 2.0, 0.0), 3);
    assert_eq!(src.count_before(Band::Kick, 100.0, 0.0), 3);
}

#[test]
fn count_before_threshold_filtering_excludes_weak_events() {
    let src = make_source(vec![
        TransientEvent {
            t: 0.0,
            strength: 0.3,
        },
        TransientEvent {
            t: 1.0,
            strength: 0.8,
        },
        TransientEvent {
            t: 2.0,
            strength: 0.3,
        },
        TransientEvent {
            t: 3.0,
            strength: 0.9,
        },
    ]);
    // threshold 0: everything.
    assert_eq!(src.count_before(Band::Kick, 10.0, 0.0), 4);
    // threshold 0.5: 0.3-strength events excluded.
    assert_eq!(src.count_before(Band::Kick, 10.0, 0.5), 2);
    assert_eq!(src.count_before(Band::Kick, 1.0, 0.5), 1);
    assert_eq!(src.count_before(Band::Kick, 2.0, 0.5), 1); // 0.3 at t=2 filtered
    assert_eq!(src.count_before(Band::Kick, 3.0, 0.5), 2);
}

#[test]
fn event_at_index_respects_threshold_filtering() {
    let src = make_source(vec![
        TransientEvent {
            t: 0.0,
            strength: 0.3,
        },
        TransientEvent {
            t: 1.0,
            strength: 0.8,
        },
        TransientEvent {
            t: 2.0,
            strength: 0.3,
        },
        TransientEvent {
            t: 3.0,
            strength: 0.9,
        },
    ]);
    assert_eq!(
        src.event_at_index(Band::Kick, 0, 0.5).map(|e| e.t),
        Some(1.0)
    );
    assert_eq!(
        src.event_at_index(Band::Kick, 1, 0.5).map(|e| e.t),
        Some(3.0)
    );
    assert!(src.event_at_index(Band::Kick, 2, 0.5).is_none());
}

// ---------------------------------------------------------------------------
// Phase cycling
// ---------------------------------------------------------------------------

#[test]
fn phase_at_event_i_eq_i_mod_4_for_modulo_4_zero_offset() {
    let events: Vec<TransientEvent> = (0..10)
        .map(|i| TransientEvent {
            t: i as f32,
            strength: 1.0,
        })
        .collect();
    let src = make_source(events.clone());
    let modulo: i64 = 4;
    let offset: i64 = 0;
    let eps: f32 = 1e-6;
    for (i, ev) in events.iter().enumerate() {
        let t = ev.t + eps;
        let count = src.count_before(Band::Kick, t, 0.0) as i64;
        let latest_idx = count - 1;
        let phase = ((latest_idx + offset) % modulo + modulo) % modulo;
        assert_eq!(count, (i as i64) + 1, "count at i={}", i);
        assert_eq!(phase, (i as i64) % 4, "phase at i={}", i);
    }
}

#[test]
fn non_zero_offset_shifts_the_phase() {
    let events: Vec<TransientEvent> = (0..8)
        .map(|i| TransientEvent {
            t: i as f32,
            strength: 1.0,
        })
        .collect();
    let src = make_source(events.clone());
    let modulo: i64 = 4;
    let offset: i64 = 2;
    for (i, ev) in events.iter().enumerate() {
        let t = ev.t + 1e-6;
        let latest_idx = src.count_before(Band::Kick, t, 0.0) as i64 - 1;
        let phase = ((latest_idx + offset) % modulo + modulo) % modulo;
        assert_eq!(phase, (i as i64 + 2) % 4, "offset-shifted phase at i={}", i);
    }
}

#[test]
fn threshold_filtered_events_renumber_the_phase_sequence() {
    let src = make_source(vec![
        TransientEvent {
            t: 0.0,
            strength: 0.3,
        }, // filtered
        TransientEvent {
            t: 1.0,
            strength: 1.0,
        }, // index 0 post-filter
        TransientEvent {
            t: 2.0,
            strength: 0.2,
        }, // filtered
        TransientEvent {
            t: 3.0,
            strength: 1.0,
        }, // index 1 post-filter
        TransientEvent {
            t: 4.0,
            strength: 1.0,
        }, // index 2 post-filter
    ]);
    let modulo: i64 = 2;
    let threshold: f32 = 0.5;
    let at_t = |t: f32| -> i64 {
        let count = src.count_before(Band::Kick, t, threshold) as i64;
        let latest_idx = count - 1;
        if latest_idx < 0 {
            -1
        } else {
            (latest_idx % modulo + modulo) % modulo
        }
    };
    assert_eq!(at_t(0.5), -1); // only weak event visible
    assert_eq!(at_t(1.5), 0);
    assert_eq!(at_t(2.5), 0); // weak event doesn't advance phase
    assert_eq!(at_t(3.5), 1);
    assert_eq!(at_t(4.5), 0);
}
