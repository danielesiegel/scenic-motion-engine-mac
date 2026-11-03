//! Top-level App — eframe::App implementation that wires every module together.
//!
//! TextRenderer ownership: lives inside `egui_wgpu`'s `CallbackResources` type-map, not inside this
//! struct. Both lifetimes work out that way — the `prepare` phase mutably borrows it alongside the
//! wgpu Device/Queue, and the `paint` phase immutably borrows it inside the actual RenderPass with
//! the right `'pass` lifetime for glyphon's draw calls.

use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use crate::audio::{JsonEventSource, Player, SyntheticEventSource};
use crate::engine::{evaluate, EngineInputs};
use crate::gui::panel::{ControlPanel, PanelCallbacks};
use crate::gui::{
    AboutPanel, HudOverlay, PreferencesWindow, ShortcutOverlay, TransportBar, WaveformStrip,
    WelcomeScreen,
};
use crate::motion::paths::register_default_paths;
use crate::record::{OfflineRenderOptions, OfflineRenderer};
use crate::render::gpu::GpuContext;
use crate::render::text_renderer::TextRenderer;
use crate::text::{build_unit_layout, FontLoader};
use crate::types::*;

const ASSETS_FONTS: &str = "assets/fonts";
const ASSETS_AUDIO: &str = "assets/audio";
const ASSETS_ANALYSIS: &str = "analysis";

/// Default background we hand out when the user hasn't touched it yet, keyed to system theme.
/// Hex strings so they slot straight into `ColorConfig::background`.
const DEFAULT_BG_DARK: &str = "#0a0a0a";
const DEFAULT_BG_LIGHT: &str = "#f6f6f6";

pub struct ScenicApp {
    pub config: Config,
    pub status: String,

    font_loader: FontLoader,
    layout: UnitLayout,
    source: Arc<dyn EventSource>,
    player: Player,

    panel: ControlPanel,
    transport: TransportBar,
    waveform: WaveformStrip,
    hud: HudOverlay,
    welcome: WelcomeScreen,
    shortcuts: ShortcutOverlay,
    about: AboutPanel,
    prefs: PreferencesWindow,

    text_renderer_installed: bool,
    last_font_size: f32,
    last_render_scale: f32,

    audio_track_name: Option<String>,

    /// Tracks whether we've already seeded a system-theme-appropriate background,
    /// so we don't clobber a user-picked color on every frame.
    theme_bg_seeded: bool,
    /// Default background we installed on first frame — treated as "untouched" when the
    /// theme flips. Any other string means the user has set their own color.
    last_default_bg: Option<String>,

    /// Timestamp of the last autosave. Drives the 10-second autosave cadence in `update`.
    last_autosave: Instant,

    /// Flag set from the panel's `on_record` closure. Polled in `update` with `&mut self`
    /// access to actually spawn the record thread — this indirection is needed because
    /// `PanelCallbacks::on_record` is a `Box<dyn FnMut()>` that cannot borrow `&mut self`.
    record_req: Arc<Mutex<RecordRequest>>,
    /// Pending record job: spawned on a background thread so the GUI stays responsive.
    /// Set when the user clicks the record button; cleared when the thread finishes.
    recording_thread: Option<std::thread::JoinHandle<anyhow::Result<std::path::PathBuf>>>,
    /// Frames rendered so far on the active job (shared with the worker thread).
    recording_progress: Arc<AtomicU32>,
    /// Total frames in the active job (shared with the worker thread).
    recording_total: Arc<AtomicU32>,
    /// Target FPS for the active job — used to format progress as seconds.
    recording_fps: u32,
}

#[derive(Default)]
struct RecordRequest {
    requested: bool,
}

impl ScenicApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        register_default_paths();

        // Mac-style egui theme: fonts, spacing, visuals, scrollbars.
        crate::gui::install_mac_style(cc.egui_ctx.clone());

        let mut font_loader = find_font_loader();
        let config = crate::config::load_or_default();
        let layout = build_unit_layout(
            &mut font_loader,
            config.layout.font_size_px,
            &config.text.unit,
        );

        let (source, player, audio_track_name, mut status) = load_audio_and_analysis(&config);

        // Install TextRenderer into egui_wgpu's CallbackResources type-map. This is the only
        // ownership pattern that gives the paint callback a `&'pass TextRenderer` which lives long
        // enough for glyphon's render call inside the egui-wgpu RenderPass.
        let mut text_renderer_installed = false;
        if let Some(rs) = cc.wgpu_render_state.as_ref() {
            let gpu = GpuContext::from_render_state(rs);
            match TextRenderer::new(gpu, &mut font_loader.font_system) {
                Ok(mut tr) => {
                    tr.set_font_family(Some(font_loader.primary_family.clone()));
                    let scale = config.canvas.render_scale.max(1.0);
                    let w = (config.canvas.width as f32 * scale) as u32;
                    let h = (config.canvas.height as f32 * scale) as u32;
                    tr.resize(w, h);
                    rs.renderer.write().callback_resources.insert(tr);
                    text_renderer_installed = true;
                }
                Err(err) => {
                    status.push_str(&format!("\nTextRenderer init failed: {}", err));
                }
            }
        } else {
            status.push_str("\nwgpu RenderState unavailable — stage will show placeholder only.");
        }

        let last_font_size = config.layout.font_size_px;
        let last_render_scale = config.canvas.render_scale;

        Self {
            config,
            status,
            font_loader,
            layout,
            source,
            player,
            panel: ControlPanel::default(),
            transport: TransportBar::default(),
            waveform: WaveformStrip::default(),
            hud: HudOverlay,
            welcome: WelcomeScreen::default(),
            shortcuts: ShortcutOverlay::default(),
            about: AboutPanel::default(),
            prefs: PreferencesWindow::default(),
            text_renderer_installed,
            last_font_size,
            last_render_scale,
            audio_track_name,
            theme_bg_seeded: false,
            last_default_bg: None,
            last_autosave: Instant::now(),
            record_req: Arc::new(Mutex::new(RecordRequest::default())),
            recording_thread: None,
            recording_progress: Arc::new(AtomicU32::new(0)),
            recording_total: Arc::new(AtomicU32::new(0)),
            recording_fps: 0,
        }
    }

    /// Persist the live `Config` to the platform's default config path.
    /// Never panics — any filesystem/serde failure is logged and swallowed, so a bad
    /// config dir can't take down the app at exit or on ⌘S.
    fn save_config(&self) {
        let path =
            crate::config::default_config_path().unwrap_or_else(|_| PathBuf::from("config.json"));
        match crate::config::save_config(&self.config, &path) {
            Ok(()) => log::info!("[config] saved to {}", path.display()),
            Err(err) => log::warn!("[config] save to {} failed: {}", path.display(), err),
        }
    }

    /// Match `config.colors.background` to the current egui theme, but only if the
    /// user hasn't customised it. First call always seeds; later calls only re-seed
    /// when the stored value is exactly the previously-installed default (meaning
    /// the user hasn't typed over it in the color picker).
    fn sync_default_background(&mut self, ctx: &egui::Context) {
        let is_dark = ctx.style().visuals.dark_mode;
        let desired = if is_dark {
            DEFAULT_BG_DARK
        } else {
            DEFAULT_BG_LIGHT
        };
        if !self.theme_bg_seeded {
            self.config.colors.background = desired.to_string();
            self.last_default_bg = Some(desired.to_string());
            self.theme_bg_seeded = true;
            return;
        }
        if let Some(prev) = self.last_default_bg.as_deref() {
            if self.config.colors.background == prev && prev != desired {
                self.config.colors.background = desired.to_string();
                self.last_default_bg = Some(desired.to_string());
            }
        }
    }

    /// Handle Space/⌘-O/⌘-R/⌘-. shortcuts. Returns whether we should request a reload-analysis.
    fn handle_shortcuts(&mut self, ctx: &egui::Context) -> ShortcutActions {
        let mut actions = ShortcutActions::default();
        ctx.input(|i| {
            // Space = play/pause.
            if i.key_pressed(egui::Key::Space) {
                actions.toggle_playback = true;
            }
            // Cmd+O = open audio file.
            if i.modifiers.mac_cmd && i.key_pressed(egui::Key::O) {
                actions.pick_audio = true;
            }
            // Cmd+R = reload analysis.
            if i.modifiers.mac_cmd && i.key_pressed(egui::Key::R) {
                actions.reload_analysis = true;
            }
            // Cmd+. = stop (seek to record.startAt).
            if i.modifiers.mac_cmd && i.key_pressed(egui::Key::Period) {
                actions.stop = true;
            }
            // H = toggle HUD (bare, no modifiers — Cmd+H is reserved for Mac hide).
            if i.key_pressed(egui::Key::H) && !i.modifiers.mac_cmd && !i.modifiers.shift {
                actions.toggle_hud = true;
            }
            // L = toggle loop (bare, no modifiers).
            if i.key_pressed(egui::Key::L) && !i.modifiers.mac_cmd && !i.modifiers.shift {
                actions.toggle_loop = true;
            }
        });
        actions
    }

    fn open_audio_dialog(&mut self) {
        let Some(path) = rfd::FileDialog::new()
            .add_filter("audio", &["wav", "mp3", "flac"])
            .pick_file()
        else {
            return;
        };
        match self.player.load_file(&path) {
            Ok(_) => {
                let name = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("audio")
                    .to_string();
                self.audio_track_name = Some(name.clone());
                self.status = format!(
                    "audio: {} ({:.1}s @ {}Hz)\n",
                    name,
                    self.player.duration(),
                    self.player.sample_rate()
                );
                self.reload_matching_analysis();
            }
            Err(err) => {
                self.status = format!("audio load failed: {}\n", err);
            }
        }
    }

    fn reload_matching_analysis(&mut self) {
        let Some(name) = self.audio_track_name.as_deref() else {
            self.source = Arc::new(SyntheticEventSource::default());
            self.status
                .push_str("analysis: synthetic (no track loaded)\n");
            return;
        };
        let analysis_path = PathBuf::from(ASSETS_ANALYSIS).join(format!("{}.json", name));
        match JsonEventSource::load(&analysis_path) {
            Ok(src) => {
                self.status.push_str(&format!(
                    "analysis: {} ({} bands)\n",
                    analysis_path.display(),
                    src.bands().len()
                ));
                self.source = Arc::new(src);
            }
            Err(err) => {
                self.status.push_str(&format!(
                    "analysis fallback → synthetic ({}): {}\n",
                    analysis_path.display(),
                    err
                ));
                self.source = Arc::new(SyntheticEventSource::default());
            }
        }
    }

    fn stop_to_start_at(&mut self) {
        self.player.pause();
        let start = self.config.record.start_at;
        self.player.seek(start);
    }

    fn maybe_rebuild_layout(&mut self) {
        if (self.config.layout.font_size_px - self.last_font_size).abs() > f32::EPSILON {
            self.last_font_size = self.config.layout.font_size_px;
            self.layout = build_unit_layout(
                &mut self.font_loader,
                self.config.layout.font_size_px,
                &self.config.text.unit,
            );
        }
    }

    /// Spawn an [`OfflineRenderer`] on a background thread with the live Config.
    /// No-ops if a recording job is already in flight.
    fn start_recording(&mut self) {
        if self.recording_thread.is_some() {
            log::info!("[record] already recording — ignoring click");
            return;
        }

        // Snapshot everything the worker needs. The worker owns a clone of Config and the
        // Arc<dyn EventSource>, so the GUI can keep mutating its own copies while the job runs.
        let config = self.config.clone();
        let source: Arc<dyn EventSource> = self.source.clone();
        let font_dir = PathBuf::from(ASSETS_FONTS);

        // Resolve dimensions = canvas * record.resolution_multiplier.
        let mult = self.config.record.resolution_multiplier.max(1);
        let width = self.config.canvas.width.saturating_mul(mult);
        let height = self.config.canvas.height.saturating_mul(mult);

        // Audio for mux: assets/audio/<track>.wav if a track is loaded.
        let audio_in = self.audio_track_name.as_deref().and_then(|name| {
            let p = PathBuf::from(ASSETS_AUDIO).join(format!("{}.wav", name));
            if p.is_file() {
                Some(p)
            } else {
                None
            }
        });

        // Output: renders/<track>_<start>s-<duration>s_<timestamp>.mp4.
        let _ = std::fs::create_dir_all("renders");
        let track = self.audio_track_name.as_deref().unwrap_or("scenic");
        let ts = chrono::Local::now().format("%Y%m%d-%H%M%S").to_string();
        let output = PathBuf::from("renders").join(format!(
            "{}_{:.1}s-{:.1}s_{}.mp4",
            track, self.config.record.start_at, self.config.record.duration, ts
        ));

        let opts = OfflineRenderOptions {
            start_at: self.config.record.start_at,
            duration: self.config.record.duration,
            fps: self.config.record.fps,
            width,
            height,
            output: output.clone(),
            audio_in,
        };

        self.recording_fps = opts.fps;
        self.recording_progress.store(0, Ordering::Relaxed);
        self.recording_total.store(
            (opts.duration * opts.fps as f32).round() as u32,
            Ordering::Relaxed,
        );

        let progress = self.recording_progress.clone();
        let total = self.recording_total.clone();

        log::info!(
            "[record] starting → {} ({}x{} @ {}fps for {:.2}s)",
            output.display(),
            width,
            height,
            opts.fps,
            opts.duration
        );

        let handle = std::thread::spawn(move || -> anyhow::Result<PathBuf> {
            let mut renderer = OfflineRenderer::new(config, source, &font_dir)?;
            renderer.render(&opts, |done, tot| {
                progress.store(done, Ordering::Relaxed);
                total.store(tot, Ordering::Relaxed);
            })?;
            Ok(opts.output)
        });
        self.recording_thread = Some(handle);
    }

    /// If the recording worker finished, join it and surface the outcome via `self.status`.
    fn poll_recording_thread(&mut self) {
        let finished = self
            .recording_thread
            .as_ref()
            .map(|h| h.is_finished())
            .unwrap_or(false);
        if !finished {
            return;
        }
        let Some(handle) = self.recording_thread.take() else {
            return;
        };
        match handle.join() {
            Ok(Ok(path)) => {
                log::info!("[record] saved to {}", path.display());
                self.status = format!("recorded: {}\n", path.display());
            }
            Ok(Err(err)) => {
                log::warn!("[record] failed: {:#}", err);
                self.status = format!("record failed: {}\n", err);
            }
            Err(_) => {
                log::warn!("[record] thread panicked");
                self.status = "record failed: worker panicked\n".to_string();
            }
        }
        self.recording_fps = 0;
    }

    fn resolve_row_count_and_key(&self) -> (u32, u32) {
        let row_count = match self.config.layout.rows {
            RowsMode::Count(n) => n.max(1),
            RowsMode::Auto(_) => {
                let h = (self.config.canvas.height as f32 - 2.0 * self.config.layout.margin_px)
                    .max(1.0);
                let lh = if self.config.layout.leading_px > 0.0 {
                    self.config.layout.leading_px
                } else {
                    self.layout.line_height.max(self.config.layout.font_size_px)
                };
                let base = (h / lh).ceil() as u32;
                let extra = self.config.camera.row_overshoot;
                base.max(1) + 2 * extra
            }
        };
        let key_row = match self.config.key.single_row {
            KeyRowValue::Middle(_) => row_count / 2,
            KeyRowValue::Index(i) => i.min(row_count.saturating_sub(1)),
        };
        (row_count, key_row)
    }
}

#[derive(Default)]
struct ShortcutActions {
    toggle_playback: bool,
    pick_audio: bool,
    reload_analysis: bool,
    stop: bool,
    toggle_hud: bool,
    toggle_loop: bool,
}

impl eframe::App for ScenicApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        self.sync_default_background(ctx);
        crate::gui::handle_dropped_files(ctx, &mut self.player, &mut self.source);

        let actions = self.handle_shortcuts(ctx);
        if actions.toggle_playback {
            if self.player.is_playing() {
                self.player.pause();
            } else {
                let _ = self.player.play();
            }
        }
        if actions.pick_audio {
            self.open_audio_dialog();
        }
        if actions.reload_analysis {
            self.reload_matching_analysis();
        }
        if actions.stop {
            self.stop_to_start_at();
        }
        if actions.toggle_hud {
            self.config.debug.hud = !self.config.debug.hud;
        }
        if actions.toggle_loop {
            self.transport.loop_enabled = !self.transport.loop_enabled;
        }

        // ⌘S = persist the live Config to disk now.
        if ctx.input(|i| i.modifiers.mac_cmd && i.key_pressed(egui::Key::S)) {
            self.save_config();
            self.last_autosave = Instant::now();
        }

        // Periodic autosave so an unexpected quit (crash, OS kill) still preserves work.
        if self.last_autosave.elapsed().as_secs() > 10 {
            self.save_config();
            self.last_autosave = Instant::now();
        }

        // Resync the backing-store size if renderScale changed.
        if (self.config.canvas.render_scale - self.last_render_scale).abs() > f32::EPSILON {
            self.last_render_scale = self.config.canvas.render_scale;
            if let Some(rs) = frame.wgpu_render_state() {
                if let Some(tr) = rs
                    .renderer
                    .write()
                    .callback_resources
                    .get_mut::<TextRenderer>()
                {
                    let scale = self.config.canvas.render_scale.max(1.0);
                    let w = (self.config.canvas.width as f32 * scale) as u32;
                    let h = (self.config.canvas.height as f32 * scale) as u32;
                    tr.resize(w, h);
                }
            }
        }

        self.maybe_rebuild_layout();

        // --- Side panel (controls) ---
        // The panel callbacks go through flags because the panel borrows &mut self.config.
        let mut want_reload_analysis = false;
        let mut want_copy_config = false;
        let rec_req = self.record_req.clone();
        egui::SidePanel::right("control")
            .default_width(360.0)
            .resizable(true)
            .show(ctx, |ui| {
                let mut cbs = PanelCallbacks {
                    on_record: Some(Box::new(move || {
                        // Just flip the flag; the real work happens after the panel
                        // returns so we can take `&mut self`.
                        if let Ok(mut r) = rec_req.lock() {
                            r.requested = true;
                        }
                    })),
                    on_copy_config: Some(Box::new(|| {
                        want_copy_config = true;
                    })),
                    on_reload_analysis: Some(Box::new(|| {
                        want_reload_analysis = true;
                    })),
                };
                self.panel.ui(ui, &mut self.config, &mut cbs);
            });
        if want_copy_config {
            match serde_json::to_string_pretty(&self.config) {
                Ok(json) => ctx.copy_text(json),
                Err(err) => log::warn!("copy config failed: {}", err),
            }
        }
        if want_reload_analysis {
            self.reload_matching_analysis();
        }

        // Did the user click the record button? If so, start a job on a background thread.
        let want_record = self
            .record_req
            .lock()
            .map(|mut r| {
                let was = r.requested;
                r.requested = false;
                was
            })
            .unwrap_or(false);
        if want_record {
            self.start_recording();
        }

        // Poll the background job: if it finished, join and surface the result.
        self.poll_recording_thread();
        // While a record job is active, show rolling progress in the status string.
        if self.recording_thread.is_some() && self.recording_fps > 0 {
            let fps = self.recording_fps as f32;
            let done = self.recording_progress.load(Ordering::Relaxed) as f32 / fps;
            let total = self.recording_total.load(Ordering::Relaxed) as f32 / fps;
            self.status = format!("recording: {:.1}/{:.1}s\n", done, total);
            // Keep repainting so the progress string ticks even when audio is paused.
            ctx.request_repaint();
        }

        // --- Bottom transport bar ---
        egui::TopBottomPanel::bottom("transport")
            .exact_height(36.0)
            .frame(egui::Frame::default().fill(ctx.style().visuals.widgets.inactive.bg_fill))
            .show(ctx, |ui| {
                self.transport.ui(ui, &mut self.player);
            });

        // --- Waveform minimap (between transport and central stage) ---
        egui::TopBottomPanel::bottom("waveform")
            .exact_height(44.0)
            .frame(egui::Frame::default().fill(ctx.style().visuals.widgets.inactive.bg_fill))
            .show(ctx, |ui| {
                self.waveform.ui(ui, &mut self.player);
            });

        // --- Central stage ---
        // Zero margins so the GPU text / background fills the entire stage without
        // egui's style padding bleeding through.
        let bg_color = parse_bg_color(&self.config.colors.background);
        egui::CentralPanel::default()
            .frame(
                egui::Frame::default()
                    .fill(bg_color)
                    .inner_margin(egui::Margin::ZERO),
            )
            .show(ctx, |ui| {
                if self.audio_track_name.is_none() {
                    self.welcome.ui(ui);
                    return;
                }
                let t = self.player.time();
                let (row_count, key_row) = self.resolve_row_count_and_key();
                let state = evaluate(EngineInputs {
                    t,
                    config: &self.config,
                    source: self.source.as_ref(),
                    layout: &self.layout,
                    row_count,
                    key_row,
                });

                if self.text_renderer_installed {
                    // Prepare glyphon on the UI thread (needs &mut FontSystem).
                    if let Some(rs) = frame.wgpu_render_state() {
                        let mut rw = rs.renderer.write();
                        if let Some(tr) = rw.callback_resources.get_mut::<TextRenderer>() {
                            if let Err(err) = tr.prepare(
                                &self.config,
                                &self.layout,
                                &state,
                                row_count,
                                key_row,
                                &mut self.font_loader.font_system,
                            ) {
                                log::warn!("TextRenderer::prepare failed: {}", err);
                            }
                        }
                    }
                    // Dispatch an empty paint callback that only invokes TextRenderer::render
                    // against the live RenderPass with 'pass-bound CallbackResources access.
                    let rect = ui.max_rect();
                    ui.painter().add(egui_wgpu::Callback::new_paint_callback(
                        rect,
                        ScenicPaintCallback,
                    ));
                    if self.config.debug.hud {
                        self.hud.ui(ui, &state, self.source.as_ref());
                    }
                } else {
                    ui.vertical_centered(|ui| {
                        ui.add_space(40.0);
                        ui.heading("scenic-motion-engine");
                        ui.monospace(&self.status);
                        if let Some(name) = &self.audio_track_name {
                            ui.label(format!("audio: {}", name));
                        }
                        ui.label(format!("t = {:.3}s", t));
                        ui.label(format!("rows = {}, key = {}", row_count, key_row));
                        if ui
                            .button(if self.player.is_playing() {
                                "pause"
                            } else {
                                "play"
                            })
                            .clicked()
                        {
                            if self.player.is_playing() {
                                self.player.pause();
                            } else {
                                let _ = self.player.play();
                            }
                        }
                    });
                }
            });

        // --- Shortcut cheatsheet overlay ---
        // Toggle with `?` (which under US keyboards is Shift+Slash). Accept both the
        // bare Questionmark key and Shift+Slash so hardware/layout quirks don't lock
        // users out. ⌘⇧? routes to About instead.
        let (toggle_about, toggle_prefs, toggle_shortcuts) = ctx.input(|i| {
            let cmd = i.modifiers.mac_cmd;
            let shift = i.modifiers.shift;
            let qmark = i.key_pressed(egui::Key::Questionmark);
            let shift_slash = shift && i.key_pressed(egui::Key::Slash);
            let about = cmd && (qmark || shift_slash);
            let prefs = cmd && i.key_pressed(egui::Key::Comma);
            // Shortcuts cheatsheet only when Cmd isn't held (⌘⇧? is reserved for About).
            let shortcuts = !cmd && (qmark || shift_slash);
            (about, prefs, shortcuts)
        });
        if toggle_about {
            self.about.toggle();
        }
        if toggle_prefs {
            self.prefs.toggle();
        }
        if toggle_shortcuts {
            self.shortcuts.toggle();
        }
        self.shortcuts.ui(ctx);
        self.about.ui(ctx);
        self.prefs.ui(ctx, &mut self.config);

        // Keep repainting while playing so the frame clock advances.
        if self.player.is_playing() {
            ctx.request_repaint();
        } else {
            ctx.request_repaint_after(std::time::Duration::from_millis(16));
        }
    }

    /// Called once on shutdown. eframe 0.29 with the wgpu backend (no `glow` feature)
    /// uses the arg-less `on_exit` signature. Flush the live Config so quit = commit.
    fn on_exit(&mut self) {
        self.save_config();
    }
}

/// Zero-size paint callback: the TextRenderer lives in CallbackResources, so we just dispatch.
struct ScenicPaintCallback;

impl egui_wgpu::CallbackTrait for ScenicPaintCallback {
    fn paint(
        &self,
        _info: egui::PaintCallbackInfo,
        pass: &mut wgpu::RenderPass<'static>,
        resources: &egui_wgpu::CallbackResources,
    ) {
        // SAFETY: egui_wgpu's `CallbackResources` type-map lives inside its long-lived
        // `Renderer`, which outlives any individual RenderPass. The borrow checker only sees the
        // method-local lifetime of `resources`, but the backing storage is effectively 'static for
        // the duration of this paint callback. We transmute to 'static so the TextRenderer handle
        // satisfies glyphon::TextRenderer::render's `'pass`-tied signature.
        let tr: &'static TextRenderer = unsafe {
            match resources.get::<TextRenderer>() {
                Some(r) => std::mem::transmute::<&TextRenderer, &'static TextRenderer>(r),
                None => return,
            }
        };
        if let Err(err) = tr.render(pass) {
            log::warn!("TextRenderer::render failed: {}", err);
        }
    }
}

// ---------------------------------------------------------------------------
// Startup helpers
// ---------------------------------------------------------------------------

fn find_font_loader() -> FontLoader {
    let dir = PathBuf::from(ASSETS_FONTS);
    if dir.is_dir() {
        FontLoader::new_with_dir(&dir).unwrap_or_else(|err| {
            log::warn!(
                "FontLoader::new_with_dir({}) failed: {} — falling back to system fonts.",
                dir.display(),
                err
            );
            FontLoader::new()
        })
    } else {
        FontLoader::new()
    }
}

fn first_file_with_ext(dir: &str, ext: &str) -> Option<PathBuf> {
    let d = PathBuf::from(dir);
    if !d.is_dir() {
        return None;
    }
    let mut matches: Vec<PathBuf> = fs::read_dir(&d)
        .ok()?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| {
            p.is_file()
                && p.extension()
                    .and_then(|s| s.to_str())
                    .map(|s| s.eq_ignore_ascii_case(ext))
                    .unwrap_or(false)
        })
        .collect();
    matches.sort();
    matches.into_iter().next()
}

fn load_audio_and_analysis(
    config: &Config,
) -> (Arc<dyn EventSource>, Player, Option<String>, String) {
    let mut status = String::new();
    let mut player = Player::new();
    let mut track_name: Option<String> = None;

    let wav = first_file_with_ext(ASSETS_AUDIO, "wav");
    if let Some(path) = &wav {
        match player.load_file(path) {
            Ok(_) => {
                let name = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("audio")
                    .to_string();
                status.push_str(&format!(
                    "audio: {} ({:.1}s @ {}Hz)\n",
                    name,
                    player.duration(),
                    player.sample_rate()
                ));
                track_name = Some(name);
            }
            Err(err) => {
                status.push_str(&format!("audio load failed: {}\n", err));
            }
        }
    } else {
        status.push_str(&format!("no .wav in {}\n", ASSETS_AUDIO));
    }

    let source: Arc<dyn EventSource> = if let Some(ref name) = track_name {
        let analysis_path = PathBuf::from(ASSETS_ANALYSIS).join(format!("{}.json", name));
        match JsonEventSource::load(&analysis_path) {
            Ok(src) => {
                status.push_str(&format!(
                    "analysis: {} ({} bands)\n",
                    analysis_path.display(),
                    src.bands().len()
                ));
                Arc::new(src)
            }
            Err(err) => {
                status.push_str(&format!(
                    "analysis fallback → synthetic ({}): {}\n",
                    analysis_path.display(),
                    err
                ));
                Arc::new(SyntheticEventSource::default())
            }
        }
    } else {
        status.push_str("analysis: synthetic (no track loaded)\n");
        Arc::new(SyntheticEventSource::default())
    };

    status.push_str(&format!(
        "tickers: {}, bindings: {}\n",
        config.tickers.len(),
        config.bindings.len()
    ));

    (source, player, track_name, status)
}

fn parse_bg_color(hex: &str) -> egui::Color32 {
    let h = hex.trim_start_matches('#');
    if h.len() != 6 {
        return egui::Color32::BLACK;
    }
    let r = u8::from_str_radix(&h[0..2], 16).unwrap_or(0);
    let g = u8::from_str_radix(&h[2..4], 16).unwrap_or(0);
    let b = u8::from_str_radix(&h[4..6], 16).unwrap_or(0);
    egui::Color32::from_rgb(r, g, b)
}
