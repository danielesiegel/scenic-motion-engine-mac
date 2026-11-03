//! The default Config, mirroring `scenic-motion-engine-web/src/config/defaults.ts` 1:1.
//!
//! Reference: ../../scenic-motion-engine-web/src/config/defaults.ts

use std::collections::HashMap;

use crate::types::*;

fn params(entries: &[(&str, f32)]) -> Params {
    entries
        .iter()
        .map(|(k, v)| (k.to_string(), ParamValue::Number(*v)))
        .collect()
}

fn action(path: &str, p: Params, duration: f32, easing: EasingName) -> PhaseAction {
    PhaseAction {
        path: path.into(),
        params: p,
        duration,
        easing: Some(Easing::Named(easing)),
    }
}

pub fn default_config() -> Config {
    let mut band_thresholds = HashMap::new();
    band_thresholds.insert("kick".into(),  0.85);
    band_thresholds.insert("snare".into(), 0.60);
    band_thresholds.insert("mid".into(),   0.50);
    band_thresholds.insert("hats".into(),  0.40);

    let tickers = vec![
        // Row motion drivers (one per band so the different row-parities feel in-sync but distinct).
        TickerConfig {
            id: "kick-even-slide".into(),
            band: Band::Kick,
            modulo: 1,
            strength_threshold: Some(0.25),
            offset: None,
            step: None,
            step_offset: None,
        },
        TickerConfig {
            id: "snare-odd-slide".into(),
            band: Band::Snare,
            modulo: 1,
            strength_threshold: Some(0.25),
            offset: None,
            step: None,
            step_offset: None,
        },
        TickerConfig {
            id: "mid-all-drift".into(),
            band: Band::Mid,
            modulo: 2,
            strength_threshold: Some(0.15),
            offset: None,
            step: None,
            step_offset: None,
        },
        TickerConfig {
            id: "hats-all-jitter".into(),
            band: Band::Hats,
            modulo: 1,
            strength_threshold: Some(0.55),
            offset: None,
            step: None,
            step_offset: None,
        },
        // Camera tickers — fire every 4th kick for zoom/pan (indices 3,7,11,...),
        // and every 4th kick offset by 2 for tilt (indices 1,5,9,...).
        TickerConfig {
            id: "kick-camera-zoompan".into(),
            band: Band::Kick,
            modulo: 4,
            strength_threshold: Some(0.25),
            offset: None,
            step: Some(4),
            step_offset: Some(3),
        },
        // Modulo-2 oscillation. Combined with camera.baseline.tilt = -5 this see-saws between +5 and -5.
        TickerConfig {
            id: "kick-camera-tilt".into(),
            band: Band::Kick,
            modulo: 2,
            strength_threshold: Some(0.25),
            offset: None,
            step: Some(4),
            step_offset: Some(1),
        },
    ];

    let bindings = vec![
        // --- Row motion: even and odd ride different bands at different speeds ---
        Binding {
            ticker_id: "kick-even-slide".into(),
            target: BindingTarget::Rows,
            rows: Some(RowSelector::Name(RowSelectorName::Even)),
            sequence: PhaseSequence {
                actions: vec![action(
                    "slideStep",
                    params(&[("units", 1.0)]),
                    0.25,
                    EasingName::EaseOutCubic,
                )],
            },
            compose: ComposeMode::Add,
            respect_alternation: None,
        },
        Binding {
            ticker_id: "snare-odd-slide".into(),
            target: BindingTarget::Rows,
            rows: Some(RowSelector::Name(RowSelectorName::Odd)),
            sequence: PhaseSequence {
                actions: vec![action(
                    "slideStep",
                    params(&[("units", 1.5)]),
                    0.30,
                    EasingName::EaseOutCubic,
                )],
            },
            compose: ComposeMode::Add,
            respect_alternation: None,
        },
        Binding {
            ticker_id: "mid-all-drift".into(),
            target: BindingTarget::Rows,
            rows: Some(RowSelector::Name(RowSelectorName::All)),
            sequence: PhaseSequence {
                actions: vec![
                    action(
                        "driftSpeed",
                        params(&[("pxPerSec", 14.0)]),
                        2.0,
                        EasingName::Linear,
                    ),
                    action(
                        "driftSpeed",
                        params(&[("pxPerSec", -14.0)]),
                        2.0,
                        EasingName::Linear,
                    ),
                ],
            },
            compose: ComposeMode::Add,
            respect_alternation: None,
        },
        Binding {
            ticker_id: "hats-all-jitter".into(),
            target: BindingTarget::Rows,
            rows: Some(RowSelector::Name(RowSelectorName::All)),
            sequence: PhaseSequence {
                actions: vec![action(
                    "glyphJitter",
                    params(&[("amplitude", 0.6)]),
                    0.08,
                    EasingName::Linear,
                )],
            },
            compose: ComposeMode::Add,
            respect_alternation: None,
        },
        // --- Camera bindings ---
        Binding {
            ticker_id: "kick-camera-zoompan".into(),
            target: BindingTarget::Camera,
            rows: None,
            sequence: PhaseSequence {
                actions: vec![
                    action(
                        "cameraZoomDeltaFocused",
                        params(&[("deltaScale", 0.10), ("focusFrac", 0.6)]),
                        0.45,
                        EasingName::EaseOutCubic,
                    ),
                    action(
                        "cameraZoomDeltaFocused",
                        params(&[("deltaScale", -0.10), ("focusFrac", 0.6)]),
                        0.45,
                        EasingName::EaseOutCubic,
                    ),
                    action(
                        "cameraPanDelta",
                        params(&[("deltaPx", 80.0)]),
                        0.45,
                        EasingName::EaseOutCubic,
                    ),
                    action(
                        "cameraPanDelta",
                        params(&[("deltaPx", -80.0)]),
                        0.45,
                        EasingName::EaseOutCubic,
                    ),
                ],
            },
            compose: ComposeMode::Add,
            respect_alternation: None,
        },
        Binding {
            ticker_id: "kick-camera-tilt".into(),
            target: BindingTarget::Camera,
            rows: None,
            sequence: PhaseSequence {
                actions: vec![
                    action(
                        "cameraTiltDelta",
                        params(&[("deltaDegrees", 10.0)]),
                        0.40,
                        EasingName::EaseOutCubic,
                    ),
                    action(
                        "cameraTiltDelta",
                        params(&[("deltaDegrees", -10.0)]),
                        0.40,
                        EasingName::EaseOutCubic,
                    ),
                ],
            },
            compose: ComposeMode::Add,
            respect_alternation: None,
        },
    ];

    Config {
        text: TextConfig {
            unit: "scenicsynth".into(),
            key: "scenicsynths".into(),
            case: TextCase::Lower,
        },
        layout: LayoutConfig {
            font_size_px: 72.0,
            leading_px: 0.0,
            rows: RowsMode::Auto(AutoRows::Auto),
            row_stagger_px: 0.33,
            rotation_deg: 0.0,
            margin_px: 0.0,
        },
        colors: ColorConfig {
            background: "#0b0b0b".into(),
            dim: "#2c2c2c".into(),
            key: "#d6d6d6".into(),
        },
        key: KeyConfig {
            rows: KeyRows::All,
            single_row: KeyRowValue::Middle(MiddleTag::Middle),
            frequency: 10,
            offset_units: 0,
            row_phase_offset: true,
            mode: KeyMode::Travels,
            anchor_screen_offset_px: 0.0,
        },
        canvas: CanvasConfig {
            width: 1920,
            height: 1080,
            dpr: 2.0,
            render_scale: 2.0,
        },
        audio: AudioConfig::default(),
        tickers,
        bindings,
        band_thresholds,
        camera: CameraConfig {
            baseline: CameraBaseline { zoom: 1.0, pan_x: 0.0, pan_y: 0.0, tilt: -5.0 },
            row_overshoot: 3,
        },
        feel: FeelConfig {
            preset: FeelPreset::Default,
            duration_scale: 1.0,
            global_easing: GlobalEasing::Auto,
            horizontal_scale: 1.0,
            row_velocity_shape: RowVelocityShape::U,
            row_velocity_edge_scale: 2.0,
        },
        record: RecordConfig {
            start_at: 0.0,
            duration: 10.0,
            fps: 60,
            video_bitrate_mbps: 20,
            resolution_multiplier: 2,
            engine: RecordEngine::Auto,
        },
        debug: DebugConfig::default(),
    }
}
