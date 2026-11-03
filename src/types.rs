//! Shared types — the contract every module builds against.
//!
//! If you are a sub-agent assigned to a specific module (engine, text, audio, render, gui, record),
//! you may ADD helper types inside your module but you MUST NOT change anything in this file.
//! Any new public type that other modules need goes here, discussed with the orchestrator first.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Bands + events
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Band {
    Kick,
    Snare,
    Mid,
    Hats,
}

impl Band {
    pub const ALL: [Band; 4] = [Band::Kick, Band::Snare, Band::Mid, Band::Hats];
    pub fn as_str(self) -> &'static str {
        match self {
            Band::Kick => "kick",
            Band::Snare => "snare",
            Band::Mid => "mid",
            Band::Hats => "hats",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct TransientEvent {
    pub t: f32,
    pub strength: f32,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AnalysisData {
    pub track: String,
    pub bpm: f32,
    pub duration: f32,
    #[serde(rename = "sampleRate")]
    pub sample_rate: u32,
    #[serde(default)]
    pub beats: Vec<f32>,
    #[serde(default)]
    pub downbeats: Vec<f32>,
    pub bands: HashMap<String, Vec<TransientEvent>>,
    #[serde(default)]
    pub envelopes: Option<EnvelopeData>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct EnvelopeData {
    pub fps: u32,
    pub bands: HashMap<String, Vec<f32>>,
}

// ---------------------------------------------------------------------------
// Easing
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum EasingName {
    Linear,
    EaseIn,
    EaseOut,
    EaseInOut,
    EaseOutCubic,
    EaseInOutCubic,
    EaseInQuad,
    EaseOutQuad,
    EaseInOutQuad,
    EaseInQuart,
    EaseOutQuart,
    EaseInOutQuart,
    EaseInExpo,
    EaseOutExpo,
    EaseInOutExpo,
    EaseOutBack,
    EaseInOutBack,
    EaseOutElastic,
    EaseOutBounce,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Easing {
    Named(EasingName),
    Bezier { bezier: [f32; 4] },
}

impl Default for Easing {
    fn default() -> Self {
        Easing::Named(EasingName::Linear)
    }
}

// ---------------------------------------------------------------------------
// Paths and phase actions
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ParamValue {
    Number(f32),
    Bool(bool),
    String(String),
}

impl ParamValue {
    pub fn as_f32(&self) -> Option<f32> {
        match self {
            ParamValue::Number(n) => Some(*n),
            ParamValue::Bool(b) => Some(if *b { 1.0 } else { 0.0 }),
            ParamValue::String(_) => None,
        }
    }
}

pub type Params = HashMap<String, ParamValue>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhaseAction {
    pub path: String,
    #[serde(default)]
    pub params: Params,
    pub duration: f32,
    #[serde(default)]
    pub easing: Option<Easing>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhaseSequence {
    pub actions: Vec<PhaseAction>,
}

// ---------------------------------------------------------------------------
// Tickers
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TickerConfig {
    pub id: String,
    pub band: Band,
    pub modulo: u32,
    #[serde(default, rename = "strengthThreshold")]
    pub strength_threshold: Option<f32>,
    #[serde(default)]
    pub offset: Option<i32>,
    #[serde(default)]
    pub step: Option<u32>,
    #[serde(default, rename = "stepOffset")]
    pub step_offset: Option<u32>,
}

// ---------------------------------------------------------------------------
// Row selector + Binding
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged, rename_all = "lowercase")]
pub enum RowSelector {
    Name(RowSelectorName),
    Indices(Vec<u32>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RowSelectorName {
    All,
    Even,
    Odd,
    KeyRow,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BindingTarget {
    Rows,
    Camera,
}

impl Default for BindingTarget {
    fn default() -> Self {
        BindingTarget::Rows
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ComposeMode {
    Add,
    Multiply,
    Override,
}

impl Default for ComposeMode {
    fn default() -> Self {
        ComposeMode::Add
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Binding {
    #[serde(rename = "tickerId")]
    pub ticker_id: String,
    #[serde(default)]
    pub target: BindingTarget,
    #[serde(default)]
    pub rows: Option<RowSelector>,
    pub sequence: PhaseSequence,
    #[serde(default)]
    pub compose: ComposeMode,
    #[serde(default, rename = "respectAlternation")]
    pub respect_alternation: Option<bool>,
}

// ---------------------------------------------------------------------------
// Config: everything the engine, renderer, gui, and recorder read.
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TextCase {
    Lower,
    Upper,
    #[serde(rename = "as-is")]
    AsIs,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextConfig {
    pub unit: String,
    pub key: String,
    pub case: TextCase,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(untagged)]
pub enum RowsMode {
    Count(u32),
    Auto(AutoRows),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AutoRows {
    Auto,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LayoutConfig {
    pub font_size_px: f32,
    pub leading_px: f32,
    pub rows: RowsMode,
    pub row_stagger_px: f32,
    pub rotation_deg: f32,
    pub margin_px: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColorConfig {
    pub background: String,
    pub dim: String,
    pub key: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum KeyRows {
    All,
    Even,
    Odd,
    Single,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum KeyRowValue {
    Middle(MiddleTag),
    Index(u32),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MiddleTag {
    Middle,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum KeyMode {
    Travels,
    Anchored,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KeyConfig {
    pub rows: KeyRows,
    pub single_row: KeyRowValue,
    pub frequency: u32,
    pub offset_units: i32,
    pub row_phase_offset: bool,
    pub mode: KeyMode,
    pub anchor_screen_offset_px: f32,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CanvasConfig {
    pub width: u32,
    pub height: u32,
    pub dpr: f32,
    pub render_scale: f32,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AudioConfig {
    pub track: Option<String>,
    #[serde(rename = "analysisUrl")]
    pub analysis_url: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct CameraBaseline {
    pub zoom: f32,
    #[serde(rename = "panX")]
    pub pan_x: f32,
    #[serde(rename = "panY")]
    pub pan_y: f32,
    pub tilt: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CameraConfig {
    pub baseline: CameraBaseline,
    pub row_overshoot: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FeelPreset {
    Custom,
    Default,
    Snappy,
    Smooth,
    Punchy,
    Glacial,
    Bouncy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum GlobalEasing {
    Auto,
    Named(EasingName),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RowVelocityShape {
    Flat,
    U,
    InvertedU,
    Linear,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FeelConfig {
    pub preset: FeelPreset,
    pub duration_scale: f32,
    pub global_easing: GlobalEasing,
    pub horizontal_scale: f32,
    pub row_velocity_shape: RowVelocityShape,
    pub row_velocity_edge_scale: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RecordEngine {
    Auto,
    Native,          // openh264/ffmpeg in-process; the fastest native path
    Ffmpeg,          // subprocess
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecordConfig {
    pub start_at: f32,
    pub duration: f32,
    pub fps: u32,
    pub video_bitrate_mbps: u32,
    pub resolution_multiplier: u32,
    pub engine: RecordEngine,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DebugConfig {
    pub hud: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub text: TextConfig,
    pub layout: LayoutConfig,
    pub colors: ColorConfig,
    pub key: KeyConfig,
    pub canvas: CanvasConfig,
    pub audio: AudioConfig,
    pub tickers: Vec<TickerConfig>,
    pub bindings: Vec<Binding>,
    #[serde(rename = "bandThresholds")]
    pub band_thresholds: HashMap<String, f32>,
    pub camera: CameraConfig,
    pub feel: FeelConfig,
    pub record: RecordConfig,
    pub debug: DebugConfig,
}

// ---------------------------------------------------------------------------
// Path context + output (the motion-path call interface)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dir {
    Pos,
    Neg,
}
impl Dir {
    pub fn as_f32(self) -> f32 {
        match self {
            Dir::Pos => 1.0,
            Dir::Neg => -1.0,
        }
    }
    pub fn for_row(row: u32) -> Dir {
        if row % 2 == 0 { Dir::Pos } else { Dir::Neg }
    }
}

#[derive(Debug, Clone, Default, Copy)]
pub struct PerGlyphOutput {
    pub dx: f32,
    pub dy: f32,
    pub scale: f32,
    pub rotation: f32,
    pub opacity: f32,
}

/// A motion path returns one of these per call. All fields are additive deltas; defaults mean "no change".
/// Not Clone / not Debug because `per_glyph` holds a trait object closure. Hand-impl Default below.
pub struct PathOutput {
    pub dx: f32,
    pub dy: f32,
    pub scale: f32,         // delta around 0, composed as (1 + scale) multiplier
    pub rotation: f32,      // degrees
    pub opacity: f32,       // delta added to 1
    pub tracking: f32,      // extra px per glyph
    /// Each boxed closure takes a glyph index and returns a per-glyph delta.
    pub per_glyph: Option<Box<dyn Fn(i64) -> PerGlyphOutput + Send + Sync>>,
}

impl PathOutput {
    pub fn is_empty(&self) -> bool {
        self.dx == 0.0 && self.dy == 0.0 && self.scale == 0.0 && self.rotation == 0.0
            && self.opacity == 0.0 && self.tracking == 0.0 && self.per_glyph.is_none()
    }
}

impl Default for PathOutput {
    fn default() -> Self {
        Self {
            dx: 0.0, dy: 0.0, scale: 0.0, rotation: 0.0,
            opacity: 0.0, tracking: 0.0, per_glyph: None,
        }
    }
}

impl std::fmt::Debug for PathOutput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PathOutput")
            .field("dx", &self.dx).field("dy", &self.dy).field("scale", &self.scale)
            .field("rotation", &self.rotation).field("opacity", &self.opacity)
            .field("tracking", &self.tracking)
            .field("per_glyph", &self.per_glyph.as_ref().map(|_| "<closure>"))
            .finish()
    }
}

/// Call context passed to every motion path function.
#[derive(Debug, Clone)]
pub struct PathContext {
    pub row_index: u32,
    pub row_count: u32,
    pub dir: Dir,
    pub progress: f32,
    pub eased_progress: f32,
    pub strength: f32,
    pub phase: u32,
    pub t: f32,
    pub since_last: f32,
    pub params: Params,
    pub unit_width: f32,
    pub duration: f32,
    pub canvas_width: f32,
    pub canvas_height: f32,
}

pub type PathFn = fn(&PathContext) -> PathOutput;

// ---------------------------------------------------------------------------
// Per-frame output
// ---------------------------------------------------------------------------

/// Not Clone / not Debug because `per_glyph` holds trait object closures. Hand-impl Default + Debug below.
pub struct RowMotionState {
    pub dx: f32,
    pub dy: f32,
    pub scale: f32,
    pub rotation: f32,
    pub opacity: f32,
    pub tracking: f32,
    pub per_glyph: Vec<Box<dyn Fn(i64) -> PerGlyphOutput + Send + Sync>>,
}

impl Default for RowMotionState {
    fn default() -> Self {
        Self { dx: 0.0, dy: 0.0, scale: 0.0, rotation: 0.0, opacity: 0.0, tracking: 0.0, per_glyph: Vec::new() }
    }
}

impl std::fmt::Debug for RowMotionState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RowMotionState")
            .field("dx", &self.dx).field("dy", &self.dy).field("scale", &self.scale)
            .field("rotation", &self.rotation).field("opacity", &self.opacity)
            .field("tracking", &self.tracking)
            .field("per_glyph", &format_args!("[{} closures]", self.per_glyph.len()))
            .finish()
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct CameraState {
    pub zoom: f32,
    pub pan_x: f32,
    pub pan_y: f32,
    pub tilt: f32,
}

#[derive(Debug, Clone)]
pub struct TickerSnapshot {
    pub id: String,
    pub band: Band,
    pub count: u32,
    pub phase: u32,
    pub since_last: f32,
    pub strength: f32,
    pub progress: f32,
    pub active_path: Option<String>,
}

#[derive(Debug, Default)]
pub struct FrameState {
    pub t: f32,
    pub rows: Vec<RowMotionState>,
    pub tickers: Vec<TickerSnapshot>,
    pub camera: CameraState,
}

// ---------------------------------------------------------------------------
// Pre-shaped text unit layout
// ---------------------------------------------------------------------------

/// Pre-shaped positions for one `scenicsynth` unit at a given font size.
#[derive(Debug, Clone)]
pub struct UnitLayout {
    pub unit: String,            // 'scenicsynth', 11 glyphs
    pub unit_length: u32,        // 11
    pub stride: f32,             // px from glyph 0 to glyph 11 (including inter-unit kern)
    pub unit_pos: Vec<f32>,      // length == unit_length; cumulative x within one unit
    pub ascent: f32,
    pub descent: f32,
    pub line_height: f32,
    pub size: f32,
}

impl UnitLayout {
    /// X position of glyph p in the infinite stream (p can be negative).
    pub fn glyph_x(&self, p: i64) -> f32 {
        let n = self.unit_length as i64;
        let idx = ((p % n) + n) % n;
        let unit_idx = p.div_euclid(n);
        unit_idx as f32 * self.stride + self.unit_pos[idx as usize]
    }

    pub fn glyph_char(&self, p: i64) -> char {
        let n = self.unit_length as i64;
        let idx = ((p % n) + n) % n;
        self.unit.chars().nth(idx as usize).unwrap_or('?')
    }

    /// Return [p_min, p_max] covering screen range [x_min, x_max] given a scroll offset.
    pub fn visible_range(&self, scroll_offset: f32, x_min: f32, x_max: f32) -> (i64, i64) {
        let avg = self.stride / self.unit_length as f32;
        let p_min = ((x_min - scroll_offset) / avg).floor() as i64 - 2;
        let p_max = ((x_max - scroll_offset) / avg).ceil() as i64 + 2;
        (p_min, p_max)
    }
}

// ---------------------------------------------------------------------------
// EventSource trait — audio events feed the engine.
// ---------------------------------------------------------------------------

pub trait EventSource: Send + Sync {
    fn bpm(&self) -> f32;
    fn duration(&self) -> f32;
    fn bands(&self) -> &[Band];
    fn get_events(&self, band: Band) -> &[TransientEvent];
    fn count_before(&self, band: Band, t: f32, threshold: f32) -> usize;
    fn event_at_index(&self, band: Band, index: usize, threshold: f32) -> Option<TransientEvent>;
    fn last_before(&self, band: Band, t: f32, threshold: f32) -> Option<TransientEvent>;
    fn beats_before(&self, t: f32) -> usize;
    fn downbeats_before(&self, t: f32) -> usize;
}
