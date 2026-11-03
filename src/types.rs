//! Shared types — the contract every module builds against.
//!
//! If you are a sub-agent assigned to a specific module (engine, text, audio, render, gui, record),
//! you may ADD helper types inside your module but you MUST NOT change anything in this file.
//! Any new public type that other modules need goes here, discussed with the orchestrator first.

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};

// ---------------------------------------------------------------------------
// Bands + events
// ---------------------------------------------------------------------------

/// Frequency band label for a transient event stream (mirrors scenicsynths/src/types.ts Band).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Band {
    /// Low-frequency band (kick drum range).
    Kick,
    /// Mid-low band (snare range).
    Snare,
    /// Mid band (vocals, melodic content).
    Mid,
    /// High band (hi-hats, cymbals).
    Hats,
}

impl Band {
    /// All band variants in canonical order; iterate when you need to visit every band.
    pub const ALL: [Band; 4] = [Band::Kick, Band::Snare, Band::Mid, Band::Hats];
    /// Returns the lowercase string key for this band (matches the JSON serde representation).
    pub fn as_str(self) -> &'static str {
        match self {
            Band::Kick => "kick",
            Band::Snare => "snare",
            Band::Mid => "mid",
            Band::Hats => "hats",
        }
    }
}

/// A single detected transient on an audio band (mirrors scenicsynths/src/types.ts TransientEvent).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct TransientEvent {
    /// Event time in seconds from the start of the track.
    pub t: f32,
    /// Normalized onset strength in [0, 1].
    pub strength: f32,
}

/// Full analysis payload loaded from JSON (mirrors scenicsynths/src/types.ts AnalysisData).
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AnalysisData {
    /// Track identifier or filename this analysis belongs to.
    pub track: String,
    /// Estimated tempo in beats per minute.
    pub bpm: f32,
    /// Track duration in seconds.
    pub duration: f32,
    /// Source audio sample rate in Hz.
    #[serde(rename = "sampleRate")]
    pub sample_rate: u32,
    /// Beat times in seconds.
    #[serde(default)]
    pub beats: Vec<f32>,
    /// Downbeat (bar start) times in seconds.
    #[serde(default)]
    pub downbeats: Vec<f32>,
    /// Transient events grouped by band name (lowercase keys matching `Band::as_str`).
    pub bands: HashMap<String, Vec<TransientEvent>>,
    /// Optional per-band continuous envelopes, if provided by the analyzer.
    #[serde(default)]
    pub envelopes: Option<EnvelopeData>,
}

/// Per-band continuous amplitude envelopes sampled at a fixed frame rate.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct EnvelopeData {
    /// Envelope sample rate in frames per second.
    pub fps: u32,
    /// Envelope sample arrays keyed by band name.
    pub bands: HashMap<String, Vec<f32>>,
}

// ---------------------------------------------------------------------------
// Easing
// ---------------------------------------------------------------------------

/// Named easing curve identifier (mirrors scenicsynths/src/types.ts EasingName).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum EasingName {
    /// Straight linear interpolation.
    Linear,
    /// Quadratic ease-in (slow start).
    EaseIn,
    /// Quadratic ease-out (slow end).
    EaseOut,
    /// Quadratic ease-in-out (slow start and end).
    EaseInOut,
    /// Cubic ease-out.
    EaseOutCubic,
    /// Cubic ease-in-out.
    EaseInOutCubic,
    /// Quadratic ease-in (explicit alias).
    EaseInQuad,
    /// Quadratic ease-out (explicit alias).
    EaseOutQuad,
    /// Quadratic ease-in-out (explicit alias).
    EaseInOutQuad,
    /// Quartic ease-in.
    EaseInQuart,
    /// Quartic ease-out.
    EaseOutQuart,
    /// Quartic ease-in-out.
    EaseInOutQuart,
    /// Exponential ease-in.
    EaseInExpo,
    /// Exponential ease-out.
    EaseOutExpo,
    /// Exponential ease-in-out.
    EaseInOutExpo,
    /// Back ease-out (slight overshoot at the end).
    EaseOutBack,
    /// Back ease-in-out (overshoot at both ends).
    EaseInOutBack,
    /// Elastic ease-out (springy settle).
    EaseOutElastic,
    /// Bounce ease-out (bouncing settle).
    EaseOutBounce,
}

/// Easing specification: either a named curve or a cubic Bezier control-point tuple.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Easing {
    /// A built-in named easing.
    Named(EasingName),
    /// Custom cubic Bezier with control points `[x1, y1, x2, y2]`.
    Bezier {
        /// Cubic Bezier control points `[x1, y1, x2, y2]`.
        bezier: [f32; 4],
    },
}

impl Default for Easing {
    fn default() -> Self {
        Easing::Named(EasingName::Linear)
    }
}

// ---------------------------------------------------------------------------
// Paths and phase actions
// ---------------------------------------------------------------------------

/// Loosely-typed parameter value for motion-path params (mirrors TS ParamValue).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ParamValue {
    /// Numeric parameter.
    Number(f32),
    /// Boolean parameter.
    Bool(bool),
    /// String parameter (e.g. a label or enum tag).
    String(String),
}

impl ParamValue {
    /// Coerce to `f32` where possible; booleans become 1.0/0.0, strings return `None`.
    pub fn as_f32(&self) -> Option<f32> {
        match self {
            ParamValue::Number(n) => Some(*n),
            ParamValue::Bool(b) => Some(if *b { 1.0 } else { 0.0 }),
            ParamValue::String(_) => None,
        }
    }
}

/// Ordered map so JSON serialization is deterministic. (HashMap iteration order is random, which
/// breaks byte-stable config roundtrips.)
pub type Params = BTreeMap<String, ParamValue>;

/// One motion-path invocation inside a phase sequence (mirrors TS PhaseAction).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhaseAction {
    /// Registered motion-path identifier to invoke.
    pub path: String,
    /// Parameters forwarded to the motion path.
    #[serde(default)]
    pub params: Params,
    /// Phase duration in seconds.
    pub duration: f32,
    /// Optional per-action easing override; falls back to the global easing when absent.
    #[serde(default)]
    pub easing: Option<Easing>,
}

/// Ordered list of phase actions driven by a ticker (mirrors TS PhaseSequence).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhaseSequence {
    /// The action phases, executed in order on each ticker fire.
    pub actions: Vec<PhaseAction>,
}

// ---------------------------------------------------------------------------
// Tickers
// ---------------------------------------------------------------------------

/// Ticker definition that converts band events into phase progressions (mirrors TS TickerConfig).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TickerConfig {
    /// Unique identifier referenced by bindings.
    pub id: String,
    /// Audio band this ticker listens to.
    pub band: Band,
    /// Fire on every Nth qualifying event (modulo cycle length).
    pub modulo: u32,
    /// Optional minimum strength required to count an event.
    #[serde(default, rename = "strengthThreshold")]
    pub strength_threshold: Option<f32>,
    /// Optional modulo offset in events.
    #[serde(default)]
    pub offset: Option<i32>,
    /// Optional phase step size.
    #[serde(default)]
    pub step: Option<u32>,
    /// Optional starting step offset within the sequence.
    #[serde(default, rename = "stepOffset")]
    pub step_offset: Option<u32>,
}

// ---------------------------------------------------------------------------
// Row selector + Binding
// ---------------------------------------------------------------------------

/// Selects which rows a binding applies to (mirrors TS RowSelector).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged, rename_all = "lowercase")]
pub enum RowSelector {
    /// Symbolic row selector (all/even/odd/keyRow).
    Name(RowSelectorName),
    /// Explicit list of row indices.
    Indices(Vec<u32>),
}

/// Symbolic row selector tag (mirrors TS RowSelector name union).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RowSelectorName {
    /// All rows.
    All,
    /// Even-indexed rows.
    Even,
    /// Odd-indexed rows.
    Odd,
    /// Only the current key row.
    KeyRow,
}

/// What a binding animates: text rows or the camera (mirrors TS BindingTarget).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BindingTarget {
    /// Binding animates text rows.
    Rows,
    /// Binding animates the camera.
    Camera,
}

// Explicit impl preserves the public signature per the project's "do not
// change public type signatures in src/types.rs" rule.
#[allow(clippy::derivable_impls)]
impl Default for BindingTarget {
    fn default() -> Self {
        BindingTarget::Rows
    }
}

/// How a binding's output composes with existing motion state (mirrors TS ComposeMode).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ComposeMode {
    /// Add to the current motion state.
    Add,
    /// Multiply with the current motion state.
    Multiply,
    /// Replace the current motion state.
    Override,
}

// Explicit impl preserves the public signature per the project's "do not
// change public type signatures in src/types.rs" rule.
#[allow(clippy::derivable_impls)]
impl Default for ComposeMode {
    fn default() -> Self {
        ComposeMode::Add
    }
}

/// Links a ticker to a phase sequence targeting rows or camera (mirrors TS Binding).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Binding {
    /// ID of the ticker driving this binding.
    #[serde(rename = "tickerId")]
    pub ticker_id: String,
    /// Target domain (rows or camera).
    #[serde(default)]
    pub target: BindingTarget,
    /// Optional row selector; required only when target is rows.
    #[serde(default)]
    pub rows: Option<RowSelector>,
    /// Phase sequence played on each ticker fire.
    pub sequence: PhaseSequence,
    /// How this binding's output composes with existing state.
    #[serde(default)]
    pub compose: ComposeMode,
    /// If true, alternate scroll direction on alternating rows.
    #[serde(default, rename = "respectAlternation")]
    pub respect_alternation: Option<bool>,
}

// ---------------------------------------------------------------------------
// Config: everything the engine, renderer, gui, and recorder read.
// ---------------------------------------------------------------------------

/// Letter-case policy for the rendered text unit (mirrors TS TextCase).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TextCase {
    /// Force lowercase.
    Lower,
    /// Force uppercase.
    Upper,
    /// Preserve case as written.
    #[serde(rename = "as-is")]
    AsIs,
}

/// Text content configuration (mirrors TS TextConfig).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextConfig {
    /// The repeating text unit (e.g. `"scenicsynth"`).
    pub unit: String,
    /// The highlighted sub-string within the unit.
    pub key: String,
    /// Case transformation applied to the unit.
    pub case: TextCase,
}

/// Row count mode: either an explicit integer or `"auto"` (mirrors TS LayoutConfig.rows).
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(untagged)]
pub enum RowsMode {
    /// Fixed row count.
    Count(u32),
    /// Automatically compute the row count from canvas size.
    Auto(AutoRows),
}

/// Marker variant for the `"auto"` row-count mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AutoRows {
    /// The literal `"auto"` tag.
    Auto,
}

/// Layout parameters for row/glyph placement (mirrors TS LayoutConfig).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LayoutConfig {
    /// Font size in CSS pixels.
    pub font_size_px: f32,
    /// Leading (line height) in CSS pixels.
    pub leading_px: f32,
    /// Row count mode.
    pub rows: RowsMode,
    /// Horizontal stagger offset between consecutive rows, in pixels.
    pub row_stagger_px: f32,
    /// Overall canvas rotation in degrees.
    pub rotation_deg: f32,
    /// Edge margin in pixels.
    pub margin_px: f32,
}

/// Color palette (mirrors TS ColorConfig). Values are CSS color strings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColorConfig {
    /// Background color.
    pub background: String,
    /// Dim (non-key) glyph color.
    pub dim: String,
    /// Key (highlight) glyph color.
    pub key: String,
}

/// Which rows participate in the key highlight (mirrors TS KeyConfig.rows).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum KeyRows {
    /// All rows show the key.
    All,
    /// Even-indexed rows only.
    Even,
    /// Odd-indexed rows only.
    Odd,
    /// A single row chosen by `KeyConfig::single_row`.
    Single,
}

/// Specifier for the single key row: either a middle tag or an explicit index.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum KeyRowValue {
    /// The literal `"middle"` tag.
    Middle(MiddleTag),
    /// Explicit row index.
    Index(u32),
}

/// Marker variant for `"middle"` single-row selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MiddleTag {
    /// The literal `"middle"` tag.
    Middle,
}

/// Key-row motion mode (mirrors TS KeyMode).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum KeyMode {
    /// Key travels along with the scroll.
    Travels,
    /// Key is anchored at a fixed screen position.
    Anchored,
}

/// Key-highlight configuration (mirrors TS KeyConfig).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KeyConfig {
    /// Which rows carry the key highlight.
    pub rows: KeyRows,
    /// Row selector used when `rows` is `Single`.
    pub single_row: KeyRowValue,
    /// Key repeats every N units.
    pub frequency: u32,
    /// Offset in units applied to the key alignment.
    pub offset_units: i32,
    /// If true, offset the phase per row.
    pub row_phase_offset: bool,
    /// Key motion mode (travels vs anchored).
    pub mode: KeyMode,
    /// Horizontal screen anchor offset in pixels when `mode == Anchored`.
    pub anchor_screen_offset_px: f32,
}

/// Canvas dimensions and scale (mirrors TS CanvasConfig).
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CanvasConfig {
    /// Logical canvas width in CSS pixels.
    pub width: u32,
    /// Logical canvas height in CSS pixels.
    pub height: u32,
    /// Device pixel ratio.
    pub dpr: f32,
    /// Extra render scale multiplier applied on top of DPR.
    pub render_scale: f32,
}

/// Audio input configuration (mirrors TS AudioConfig).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AudioConfig {
    /// Optional track filename or identifier.
    pub track: Option<String>,
    /// Optional URL/path to the analysis JSON.
    #[serde(rename = "analysisUrl")]
    pub analysis_url: Option<String>,
}

/// Baseline camera state applied before any binding deltas (mirrors TS CameraBaseline).
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct CameraBaseline {
    /// Baseline zoom factor.
    pub zoom: f32,
    /// Baseline horizontal pan in pixels.
    #[serde(rename = "panX")]
    pub pan_x: f32,
    /// Baseline vertical pan in pixels.
    #[serde(rename = "panY")]
    pub pan_y: f32,
    /// Baseline tilt in degrees.
    pub tilt: f32,
}

/// Camera configuration (mirrors TS CameraConfig).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CameraConfig {
    /// Baseline camera state.
    pub baseline: CameraBaseline,
    /// Number of extra rows to render beyond the viewport for overshoot.
    pub row_overshoot: u32,
}

/// Named feel preset bundling easing/duration tweaks (mirrors TS FeelPreset).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FeelPreset {
    /// Fully custom; respect the explicit feel fields.
    Custom,
    /// Default feel preset.
    Default,
    /// Snappy preset (fast, tight).
    Snappy,
    /// Smooth preset (gentle easing).
    Smooth,
    /// Punchy preset (strong accents).
    Punchy,
    /// Glacial preset (very slow).
    Glacial,
    /// Bouncy preset (overshoot/elastic).
    Bouncy,
}

/// Global easing selector: either auto-derived or an explicit named easing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum GlobalEasing {
    /// Choose easing automatically from the preset.
    Auto,
    /// Use this explicit named easing.
    Named(EasingName),
}

/// Shape of the per-row velocity profile (mirrors TS RowVelocityShape).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RowVelocityShape {
    /// All rows move at the same velocity.
    Flat,
    /// Edges slow, middle fast (U-shape inverse).
    U,
    /// Middle slow, edges fast (inverted-U).
    InvertedU,
    /// Linear gradient top-to-bottom.
    Linear,
}

/// Feel configuration bundling global timing/shape knobs (mirrors TS FeelConfig).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FeelConfig {
    /// Named preset (`Custom` means use the explicit fields verbatim).
    pub preset: FeelPreset,
    /// Multiplier applied to all phase durations.
    pub duration_scale: f32,
    /// Global easing selector.
    pub global_easing: GlobalEasing,
    /// Multiplier applied to horizontal motion deltas.
    pub horizontal_scale: f32,
    /// Per-row velocity profile shape.
    pub row_velocity_shape: RowVelocityShape,
    /// Scale applied to the edge rows under non-flat row-velocity profiles.
    pub row_velocity_edge_scale: f32,
}

/// Which recording backend to use (mirrors TS RecordEngine).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RecordEngine {
    /// Pick the best available engine automatically.
    Auto,
    /// In-process native encoder (openh264/ffmpeg) — fastest path.
    Native, // openh264/ffmpeg in-process; the fastest native path
    /// External ffmpeg subprocess.
    Ffmpeg, // subprocess
}

/// Recording configuration (mirrors TS RecordConfig).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecordConfig {
    /// Recording start time in seconds from track start.
    pub start_at: f32,
    /// Recording duration in seconds.
    pub duration: f32,
    /// Output frame rate.
    pub fps: u32,
    /// Target video bitrate in megabits per second.
    pub video_bitrate_mbps: u32,
    /// Resolution multiplier applied over the canvas size.
    pub resolution_multiplier: u32,
    /// Encoding backend.
    pub engine: RecordEngine,
}

/// Debug/diagnostic toggles (mirrors TS DebugConfig).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DebugConfig {
    /// Show the on-screen heads-up display.
    pub hud: bool,
    /// Whether the Preferences window is open. Persisted across launches so the
    /// window reopens in the state the user left it (closes #23).
    #[serde(default)]
    pub prefs_open: bool,
    /// Whether the About window is open. Persisted across launches so the window
    /// reopens in the state the user left it (closes #23).
    #[serde(default)]
    pub about_open: bool,
}

/// Top-level configuration consumed by every subsystem (mirrors TS Config).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Text content configuration.
    pub text: TextConfig,
    /// Row/glyph layout configuration.
    pub layout: LayoutConfig,
    /// Color palette.
    pub colors: ColorConfig,
    /// Key-highlight configuration.
    pub key: KeyConfig,
    /// Canvas dimensions and scale.
    pub canvas: CanvasConfig,
    /// Audio input configuration.
    pub audio: AudioConfig,
    /// Registered tickers that drive bindings.
    pub tickers: Vec<TickerConfig>,
    /// Ticker-to-sequence bindings.
    pub bindings: Vec<Binding>,
    /// Per-band onset strength thresholds (keyed by band name).
    #[serde(rename = "bandThresholds")]
    pub band_thresholds: BTreeMap<String, f32>,
    /// Camera configuration.
    pub camera: CameraConfig,
    /// Feel/global-timing configuration.
    pub feel: FeelConfig,
    /// Recording configuration.
    pub record: RecordConfig,
    /// Debug toggles.
    pub debug: DebugConfig,
}

// ---------------------------------------------------------------------------
// Path context + output (the motion-path call interface)
// ---------------------------------------------------------------------------

/// Row direction sign used for alternating-row motion.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dir {
    /// Positive direction (+1).
    Pos,
    /// Negative direction (-1).
    Neg,
}
impl Dir {
    /// Convert this direction into its signed scalar (+1.0 or -1.0).
    pub fn as_f32(self) -> f32 {
        match self {
            Dir::Pos => 1.0,
            Dir::Neg => -1.0,
        }
    }
    /// Alternating direction for a given row: even rows go `Pos`, odd rows go `Neg`.
    pub fn for_row(row: u32) -> Dir {
        if row.is_multiple_of(2) {
            Dir::Pos
        } else {
            Dir::Neg
        }
    }
}

/// Per-glyph motion delta returned by a glyph closure.
#[derive(Debug, Clone, Default, Copy)]
pub struct PerGlyphOutput {
    /// Horizontal delta in pixels.
    pub dx: f32,
    /// Vertical delta in pixels.
    pub dy: f32,
    /// Scale delta added to the baseline 1.0.
    pub scale: f32,
    /// Rotation delta in degrees.
    pub rotation: f32,
    /// Opacity delta added to the baseline 1.0.
    pub opacity: f32,
}

/// A motion path returns one of these per call. All fields are additive deltas; defaults mean "no change".
/// Not Clone / not Debug because `per_glyph` holds a trait object closure. Hand-impl Default below.
pub struct PathOutput {
    /// Horizontal delta in pixels.
    pub dx: f32,
    /// Vertical delta in pixels.
    pub dy: f32,
    /// Scale delta composed as `(1 + scale)`.
    pub scale: f32, // delta around 0, composed as (1 + scale) multiplier
    /// Rotation delta in degrees.
    pub rotation: f32, // degrees
    /// Opacity delta added to the baseline 1.0.
    pub opacity: f32, // delta added to 1
    /// Extra tracking (letter-spacing) added per glyph, in pixels.
    pub tracking: f32, // extra px per glyph
    /// Each boxed closure takes a glyph index and returns a per-glyph delta.
    pub per_glyph: Option<Box<dyn Fn(i64) -> PerGlyphOutput + Send + Sync>>,
}

impl PathOutput {
    /// Returns true when every delta is zero and no per-glyph closure is set.
    pub fn is_empty(&self) -> bool {
        self.dx == 0.0
            && self.dy == 0.0
            && self.scale == 0.0
            && self.rotation == 0.0
            && self.opacity == 0.0
            && self.tracking == 0.0
            && self.per_glyph.is_none()
    }
}

impl Default for PathOutput {
    fn default() -> Self {
        Self {
            dx: 0.0,
            dy: 0.0,
            scale: 0.0,
            rotation: 0.0,
            opacity: 0.0,
            tracking: 0.0,
            per_glyph: None,
        }
    }
}

impl std::fmt::Debug for PathOutput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PathOutput")
            .field("dx", &self.dx)
            .field("dy", &self.dy)
            .field("scale", &self.scale)
            .field("rotation", &self.rotation)
            .field("opacity", &self.opacity)
            .field("tracking", &self.tracking)
            .field("per_glyph", &self.per_glyph.as_ref().map(|_| "<closure>"))
            .finish()
    }
}

/// Call context passed to every motion path function. Mirrors scenicsynths/src/types.ts PathContext.
#[derive(Debug, Clone)]
pub struct PathContext {
    /// Row index (0-based) being evaluated.
    pub row_index: u32,
    /// Total number of rows.
    pub row_count: u32,
    /// Direction sign for this row.
    pub dir: Dir,
    /// Raw phase progress in [0, 1].
    pub progress: f32,
    /// Eased phase progress in [0, 1].
    pub eased_progress: f32,
    /// Driving event strength in [0, 1].
    pub strength: f32,
    /// Current phase index within the sequence.
    pub phase: u32,
    /// Current wall/track time in seconds.
    pub t: f32,
    /// Seconds elapsed since the last ticker fire.
    pub since_last: f32,
    /// Phase action parameters.
    pub params: Params,
    /// Width of one text unit in pixels.
    pub unit_width: f32,
    /// Phase duration in seconds.
    pub duration: f32,
    /// Canvas width in pixels.
    pub canvas_width: f32,
    /// Canvas height in pixels.
    pub canvas_height: f32,
}

/// Function pointer type for a motion path handler.
pub type PathFn = fn(&PathContext) -> PathOutput;

// ---------------------------------------------------------------------------
// Per-frame output
// ---------------------------------------------------------------------------

/// Per-row accumulated motion for one frame.
/// Not Clone / not Debug because `per_glyph` holds trait object closures. Hand-impl Default + Debug below.
pub struct RowMotionState {
    /// Accumulated horizontal delta in pixels.
    pub dx: f32,
    /// Accumulated vertical delta in pixels.
    pub dy: f32,
    /// Accumulated scale delta.
    pub scale: f32,
    /// Accumulated rotation delta in degrees.
    pub rotation: f32,
    /// Accumulated opacity delta.
    pub opacity: f32,
    /// Accumulated tracking (letter-spacing) delta in pixels.
    pub tracking: f32,
    /// Stack of per-glyph closures contributed by active paths this frame.
    pub per_glyph: Vec<Box<dyn Fn(i64) -> PerGlyphOutput + Send + Sync>>,
}

impl Default for RowMotionState {
    fn default() -> Self {
        Self {
            dx: 0.0,
            dy: 0.0,
            scale: 0.0,
            rotation: 0.0,
            opacity: 0.0,
            tracking: 0.0,
            per_glyph: Vec::new(),
        }
    }
}

impl std::fmt::Debug for RowMotionState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RowMotionState")
            .field("dx", &self.dx)
            .field("dy", &self.dy)
            .field("scale", &self.scale)
            .field("rotation", &self.rotation)
            .field("opacity", &self.opacity)
            .field("tracking", &self.tracking)
            .field(
                "per_glyph",
                &format_args!("[{} closures]", self.per_glyph.len()),
            )
            .finish()
    }
}

/// Camera state for one frame (deltas composed over `CameraBaseline`).
#[derive(Debug, Clone, Copy, Default)]
pub struct CameraState {
    /// Zoom delta.
    pub zoom: f32,
    /// Horizontal pan delta in pixels.
    pub pan_x: f32,
    /// Vertical pan delta in pixels.
    pub pan_y: f32,
    /// Tilt delta in degrees.
    pub tilt: f32,
}

/// Debug snapshot of a ticker's state at a point in time (used by the HUD).
#[derive(Debug, Clone)]
pub struct TickerSnapshot {
    /// Ticker identifier.
    pub id: String,
    /// Band this ticker listens to.
    pub band: Band,
    /// Number of qualifying events seen so far.
    pub count: u32,
    /// Current phase index within the sequence.
    pub phase: u32,
    /// Seconds since the last fire.
    pub since_last: f32,
    /// Strength of the last firing event.
    pub strength: f32,
    /// Progress through the current phase in [0, 1].
    pub progress: f32,
    /// Name of the currently active motion path, if any.
    pub active_path: Option<String>,
}

/// Full per-frame engine output consumed by the renderer and HUD.
#[derive(Debug, Default)]
pub struct FrameState {
    /// Wall/track time in seconds.
    pub t: f32,
    /// Per-row motion state.
    pub rows: Vec<RowMotionState>,
    /// Snapshots of each ticker (for the HUD).
    pub tickers: Vec<TickerSnapshot>,
    /// Camera state for this frame.
    pub camera: CameraState,
}

// ---------------------------------------------------------------------------
// Pre-shaped text unit layout
// ---------------------------------------------------------------------------

/// Pre-shaped positions for one `scenicsynth` unit at a given font size.
#[derive(Debug, Clone)]
pub struct UnitLayout {
    /// The text unit shaped (e.g. `"scenicsynth"`, 11 glyphs).
    pub unit: String, // 'scenicsynth', 11 glyphs
    /// Length of the unit in glyphs.
    pub unit_length: u32, // 11
    /// Horizontal stride in pixels from glyph 0 to glyph `unit_length` (includes inter-unit kern).
    pub stride: f32, // px from glyph 0 to glyph 11 (including inter-unit kern)
    /// Cumulative glyph x-positions within one unit (length == `unit_length`).
    pub unit_pos: Vec<f32>, // length == unit_length; cumulative x within one unit
    /// Font ascent in pixels.
    pub ascent: f32,
    /// Font descent in pixels.
    pub descent: f32,
    /// Line height in pixels.
    pub line_height: f32,
    /// Font size in pixels this layout was shaped at.
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

    /// Return the character at glyph index `p` in the infinite repeating stream.
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

/// Audio-event provider interface consumed by the engine's tickers.
pub trait EventSource: Send + Sync {
    /// Tempo in beats per minute.
    fn bpm(&self) -> f32;
    /// Track duration in seconds.
    fn duration(&self) -> f32;
    /// All bands this source exposes.
    fn bands(&self) -> &[Band];
    /// All events for a given band in time-sorted order.
    fn get_events(&self, band: Band) -> &[TransientEvent];
    /// Count of events on `band` strictly before time `t` whose strength passes `threshold`.
    fn count_before(&self, band: Band, t: f32, threshold: f32) -> usize;
    /// The i-th event on `band` passing `threshold`, if any.
    fn event_at_index(&self, band: Band, index: usize, threshold: f32) -> Option<TransientEvent>;
    /// Most recent event on `band` before time `t` passing `threshold`, if any.
    fn last_before(&self, band: Band, t: f32, threshold: f32) -> Option<TransientEvent>;
    /// Number of beats strictly before time `t`.
    fn beats_before(&self, t: f32) -> usize;
    /// Number of downbeats strictly before time `t`.
    fn downbeats_before(&self, t: f32) -> usize;
}
