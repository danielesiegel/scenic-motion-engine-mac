//! Config JSON round-trip test.
//!
//! Serializes `default_config()` to JSON, deserializes it, and asserts:
//!   - Every scalar in `feel`, `camera.baseline`, `canvas`, `record` round-trips bit-for-bit.
//!   - `tickers.len()` and `bindings.len()` round-trip, plus each ticker's identifying fields
//!     (`id`, `band`, `modulo`, `step`, `step_offset`) and each binding's identifying fields
//!     (`ticker_id`, `target`, `rows` presence, `sequence.actions.len()`).
//!   - Serialization is stable: serialize → deserialize → serialize produces byte-identical JSON
//!     on the second pass.

use scenic_motion_engine::config::defaults::default_config;
use scenic_motion_engine::types::*;

fn to_json(cfg: &Config) -> String {
    serde_json::to_string(cfg).expect("serialize Config")
}

fn from_json(s: &str) -> Config {
    serde_json::from_str(s).expect("deserialize Config")
}

// --- helper: compare optional rows selector by structural shape ---------------
fn rows_shape(r: &Option<RowSelector>) -> Option<&'static str> {
    match r {
        None => None,
        Some(RowSelector::Name(RowSelectorName::All)) => Some("name:all"),
        Some(RowSelector::Name(RowSelectorName::Even)) => Some("name:even"),
        Some(RowSelector::Name(RowSelectorName::Odd)) => Some("name:odd"),
        Some(RowSelector::Name(RowSelectorName::KeyRow)) => Some("name:keyRow"),
        Some(RowSelector::Indices(_)) => Some("indices"),
    }
}

#[test]
fn round_trip_preserves_feel_camera_canvas_record_scalars() {
    let orig = default_config();
    let json = to_json(&orig);
    let back = from_json(&json);

    // feel
    assert_eq!(orig.feel.preset, back.feel.preset);
    assert_eq!(orig.feel.duration_scale, back.feel.duration_scale);
    assert_eq!(orig.feel.global_easing, back.feel.global_easing);
    assert_eq!(orig.feel.horizontal_scale, back.feel.horizontal_scale);
    assert_eq!(orig.feel.row_velocity_shape, back.feel.row_velocity_shape);
    assert_eq!(
        orig.feel.row_velocity_edge_scale,
        back.feel.row_velocity_edge_scale
    );

    // camera.baseline
    assert_eq!(orig.camera.baseline.zoom, back.camera.baseline.zoom);
    assert_eq!(orig.camera.baseline.pan_x, back.camera.baseline.pan_x);
    assert_eq!(orig.camera.baseline.pan_y, back.camera.baseline.pan_y);
    assert_eq!(orig.camera.baseline.tilt, back.camera.baseline.tilt);
    assert_eq!(orig.camera.row_overshoot, back.camera.row_overshoot);

    // canvas
    assert_eq!(orig.canvas.width, back.canvas.width);
    assert_eq!(orig.canvas.height, back.canvas.height);
    assert_eq!(orig.canvas.dpr, back.canvas.dpr);
    assert_eq!(orig.canvas.render_scale, back.canvas.render_scale);

    // record
    assert_eq!(orig.record.start_at, back.record.start_at);
    assert_eq!(orig.record.duration, back.record.duration);
    assert_eq!(orig.record.fps, back.record.fps);
    assert_eq!(
        orig.record.video_bitrate_mbps,
        back.record.video_bitrate_mbps
    );
    assert_eq!(
        orig.record.resolution_multiplier,
        back.record.resolution_multiplier
    );
    assert_eq!(orig.record.engine, back.record.engine);
}

#[test]
fn round_trip_preserves_tickers_and_bindings_shape() {
    let orig = default_config();
    let json = to_json(&orig);
    let back = from_json(&json);

    // Tickers.
    assert_eq!(orig.tickers.len(), back.tickers.len(), "ticker count");
    for (a, b) in orig.tickers.iter().zip(back.tickers.iter()) {
        assert_eq!(a.id, b.id, "ticker id");
        assert_eq!(a.band, b.band, "ticker band for {}", a.id);
        assert_eq!(a.modulo, b.modulo, "ticker modulo for {}", a.id);
        assert_eq!(a.step, b.step, "ticker step for {}", a.id);
        assert_eq!(
            a.step_offset, b.step_offset,
            "ticker stepOffset for {}",
            a.id
        );
    }

    // Bindings.
    assert_eq!(orig.bindings.len(), back.bindings.len(), "binding count");
    for (i, (a, b)) in orig.bindings.iter().zip(back.bindings.iter()).enumerate() {
        assert_eq!(a.ticker_id, b.ticker_id, "binding[{}].ticker_id", i);
        assert_eq!(a.target, b.target, "binding[{}].target", i);
        assert_eq!(
            rows_shape(&a.rows),
            rows_shape(&b.rows),
            "binding[{}].rows shape",
            i
        );
        assert_eq!(
            a.sequence.actions.len(),
            b.sequence.actions.len(),
            "binding[{}].sequence.actions.len",
            i
        );
    }
}

#[test]
fn serialization_is_stable_byte_identical_on_second_pass() {
    let orig = default_config();
    let first = to_json(&orig);
    let back = from_json(&first);
    let second = to_json(&back);
    assert_eq!(
        first, second,
        "serialization is not stable across roundtrip"
    );
}
