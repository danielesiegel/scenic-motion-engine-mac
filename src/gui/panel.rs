//! egui control panel. Polished to feel like a macOS inspector pane (Logic / Final Cut style).
//!
//! Contract:
//! - `ControlPanel::ui(&mut self, ui, config, callbacks)` renders the side panel. All controls
//!   mutate `config` in place; buttons invoke `callbacks` (record, copy_config, reload_analysis).

use crate::gui::style::{secondary_label, soft_divider};
use crate::types::{
    Band, Binding, CameraBaseline, CameraConfig, ColorConfig, Config, DebugConfig, Easing,
    EasingName, FeelConfig, FeelPreset, GlobalEasing, KeyConfig, KeyMode, KeyRowValue, KeyRows,
    LayoutConfig, MiddleTag, PhaseAction, RecordConfig, RecordEngine, RowVelocityShape, TextCase,
    TextConfig, TickerConfig,
};
use egui::{Color32, ComboBox, RichText, Slider};

// -------------------------------------------------------------------
// Visual constants (macOS inspector feel)
// -------------------------------------------------------------------

/// Width of the leading icon/label column for rows that *can't* use `Slider::text()`
/// (checkboxes, text edits, color pickers, custom combo rows).
const LEAD_COL_WIDTH: f32 = 150.0;
/// Vertical padding around section-local dividers.
const SECTION_PAD: f32 = 2.0;

/// macOS-ish selection blue used for the primary record button — muted a shade from `#007aff`.
const SELECTION_BLUE: Color32 = Color32::from_rgb(0x00, 0x7A, 0xCC);

fn band_color(band: Band) -> Color32 {
    match band {
        Band::Kick => Color32::from_rgb(0x3A, 0x7A, 0xFF), // blue
        Band::Snare => Color32::from_rgb(0x44, 0xCC, 0x88), // green
        Band::Mid => Color32::from_rgb(0xFF, 0xAA, 0x33),  // orange
        Band::Hats => Color32::from_rgb(0xFF, 0x55, 0x66), // red
    }
}

/// Paint a 6px band color dot inline with the current row.
fn band_dot(ui: &mut egui::Ui, band: Band) {
    let size = egui::vec2(10.0, ui.spacing().interact_size.y);
    let (rect, _resp) = ui.allocate_exact_size(size, egui::Sense::hover());
    let center = egui::pos2(rect.left() + 5.0, rect.center().y);
    ui.painter().circle_filled(center, 3.0, band_color(band));
}

/// Half-width ComboBox (right-aligned alignment target). Mirrors Logic Pro's inspector
/// where enum rows put the control in the right half of the pane.
fn combo_half_width(ui: &egui::Ui) -> f32 {
    (ui.available_width() * 0.5).max(100.0)
}

fn band_from_str(s: &str) -> Option<Band> {
    match s {
        "kick" => Some(Band::Kick),
        "snare" => Some(Band::Snare),
        "mid" => Some(Band::Mid),
        "hats" => Some(Band::Hats),
        _ => None,
    }
}

// -------------------------------------------------------------------
// Public API
// -------------------------------------------------------------------

pub struct ControlPanel {
    /// Buffer for key.single_row ("middle" or an integer as string).
    single_row_buf: String,
    /// Cubic-bezier draft that the "apply bezier" button would use (not yet wired up —
    /// GlobalEasing enum has no Bezier variant).
    bezier: [f32; 4],
}

pub struct PanelCallbacks<'a> {
    pub on_record: Option<Box<dyn FnMut() + 'a>>,
    pub on_copy_config: Option<Box<dyn FnMut() + 'a>>,
    pub on_reload_analysis: Option<Box<dyn FnMut() + 'a>>,
}

impl Default for ControlPanel {
    fn default() -> Self {
        Self {
            single_row_buf: String::from("middle"),
            bezier: [0.33, 0.0, 0.33, 1.0],
        }
    }
}

impl ControlPanel {
    pub fn ui(&mut self, ui: &mut egui::Ui, config: &mut Config, callbacks: &mut PanelCallbacks) {
        // Sync single-row buffer the first time (or when it drifts to empty).
        if self.single_row_buf.is_empty() {
            self.single_row_buf = key_row_to_string(&config.key.single_row);
        }

        egui::ScrollArea::vertical()
            .auto_shrink([false; 2])
            .show(ui, |ui| {
                ui.heading("scenicsynths");

                self.section_feel(ui, &mut config.feel);
                self.section_layout(ui, &mut config.layout, &mut config.canvas.render_scale);
                self.section_colors(ui, &mut config.colors);
                self.section_spotlight(ui, &mut config.key);
                self.section_text(ui, &mut config.text);
                self.section_band_thresholds(ui, &mut config.band_thresholds);
                self.section_tickers(ui, &mut config.tickers);
                self.section_bindings(ui, &mut config.bindings);
                self.section_camera(ui, &mut config.camera);
                self.section_record(ui, &mut config.record, callbacks);

                // Debug last, with a soft hairline above.
                ui.add_space(SECTION_PAD);
                soft_divider(ui);
                ui.add_space(SECTION_PAD);
                self.section_debug(ui, &mut config.debug, callbacks);
            });
    }

    // ---------------- feel ----------------
    fn section_feel(&mut self, ui: &mut egui::Ui, feel: &mut FeelConfig) {
        section_header(ui, "Feel", true, |ui| {
            // preset
            let mut preset = feel.preset;
            let preset_label = feel_preset_label(preset);
            ComboBox::from_label("preset")
                .width(combo_half_width(ui))
                .selected_text(preset_label)
                .show_ui(ui, |ui| {
                    for p in [
                        FeelPreset::Custom,
                        FeelPreset::Default,
                        FeelPreset::Snappy,
                        FeelPreset::Smooth,
                        FeelPreset::Punchy,
                        FeelPreset::Glacial,
                        FeelPreset::Bouncy,
                    ] {
                        ui.selectable_value(&mut preset, p, feel_preset_label(p));
                    }
                });
            if preset != feel.preset {
                feel.preset = preset;
                apply_preset(feel, preset);
            }

            egui::Grid::new("feel-sliders")
                .num_columns(1)
                .spacing([8.0, 4.0])
                .show(ui, |ui| {
                    dense_slider(
                        ui,
                        "horizontal x",
                        Slider::new(&mut feel.horizontal_scale, 0.0..=4.0)
                            .step_by(0.01)
                            .custom_formatter(|n, _| format!("{:.2}", n))
                            .text(""),
                        "horizontal x: multiplies every horizontal motion delta globally (rows + camera pan)",
                    );
                    ui.end_row();
                });

            // row velocity shape
            let mut shape = feel.row_velocity_shape;
            ComboBox::from_label("row vel curve")
                .width(combo_half_width(ui))
                .selected_text(row_velocity_shape_label(shape))
                .show_ui(ui, |ui| {
                    for s in [
                        RowVelocityShape::Flat,
                        RowVelocityShape::U,
                        RowVelocityShape::InvertedU,
                        RowVelocityShape::Linear,
                    ] {
                        ui.selectable_value(&mut shape, s, row_velocity_shape_label(s));
                    }
                });
            if shape != feel.row_velocity_shape {
                feel.row_velocity_shape = shape;
                feel.preset = FeelPreset::Custom;
            }

            egui::Grid::new("feel-sliders-2")
                .num_columns(1)
                .spacing([8.0, 4.0])
                .show(ui, |ui| {
                    dense_slider(
                        ui,
                        "edge x",
                        Slider::new(&mut feel.row_velocity_edge_scale, 0.25..=4.0)
                            .step_by(0.05)
                            .custom_formatter(|n, _| format!("{:.2}", n))
                            .text(""),
                        "edge x: scales the velocity curve at edge rows relative to center rows",
                    );
                    ui.end_row();

                    let ds_before = feel.duration_scale;
                    dense_slider(
                        ui,
                        "duration scale",
                        Slider::new(&mut feel.duration_scale, 0.1..=3.0)
                            .step_by(0.01)
                            .custom_formatter(|n, _| format!("{:.2}", n))
                            .text(""),
                        "duration scale: multiplies every action's duration globally",
                    );
                    ui.end_row();
                    if (feel.duration_scale - ds_before).abs() > f32::EPSILON {
                        feel.preset = FeelPreset::Custom;
                    }
                });

            // global easing
            let mut ge = feel.global_easing;
            ComboBox::from_label("global easing")
                .width(combo_half_width(ui))
                .selected_text(global_easing_label(ge))
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut ge, GlobalEasing::Auto, "auto");
                    for e in ALL_EASING_NAMES {
                        let v = GlobalEasing::Named(e);
                        ui.selectable_value(&mut ge, v, easing_name_label(e));
                    }
                });
            if ge != feel.global_easing {
                feel.global_easing = ge;
                feel.preset = FeelPreset::Custom;
            }

            // cubic-bezier override (collapsing, default collapsed)
            egui::CollapsingHeader::new("cubic-bezier override")
                .default_open(false)
                .show(ui, |ui| {
                    egui::Grid::new("feel-bezier")
                        .num_columns(1)
                        .spacing([8.0, 4.0])
                        .show(ui, |ui| {
                            dense_slider(
                                ui,
                                "x1",
                                Slider::new(&mut self.bezier[0], 0.0..=1.0)
                                    .step_by(0.01)
                                    .custom_formatter(|n, _| format!("{:.2}", n))
                                    .text(""),
                                "cubic-bezier x1 (first control point x)",
                            );
                            ui.end_row();
                            dense_slider(
                                ui,
                                "y1",
                                Slider::new(&mut self.bezier[1], -0.5..=1.5)
                                    .step_by(0.01)
                                    .custom_formatter(|n, _| format!("{:.2}", n))
                                    .text(""),
                                "cubic-bezier y1 (first control point y)",
                            );
                            ui.end_row();
                            dense_slider(
                                ui,
                                "x2",
                                Slider::new(&mut self.bezier[2], 0.0..=1.0)
                                    .step_by(0.01)
                                    .custom_formatter(|n, _| format!("{:.2}", n))
                                    .text(""),
                                "cubic-bezier x2 (second control point x)",
                            );
                            ui.end_row();
                            dense_slider(
                                ui,
                                "y2",
                                Slider::new(&mut self.bezier[3], -0.5..=1.5)
                                    .step_by(0.01)
                                    .custom_formatter(|n, _| format!("{:.2}", n))
                                    .text(""),
                                "cubic-bezier y2 (second control point y)",
                            );
                            ui.end_row();
                        });
                    // TODO: extend GlobalEasing with Bezier so this button can actually set it.
                    let _ = ui.button("apply bezier as global easing");
                });
        });
    }

    // ---------------- layout ----------------
    fn section_layout(
        &mut self,
        ui: &mut egui::Ui,
        layout: &mut LayoutConfig,
        render_scale: &mut f32,
    ) {
        section_header(ui, "Layout", false, |ui| {
            egui::Grid::new("layout-sliders")
                .num_columns(1)
                .spacing([8.0, 4.0])
                .show(ui, |ui| {
                    dense_slider(
                        ui,
                        "fontSizePx",
                        Slider::new(&mut layout.font_size_px, 12.0..=220.0)
                            .custom_formatter(|n, _| format!("{:.2}", n))
                            .text(""),
                        "font size in pixels at the base canvas resolution",
                    );
                    ui.end_row();
                    dense_slider(
                        ui,
                        "leadingPx",
                        Slider::new(&mut layout.leading_px, 0.0..=220.0)
                            .custom_formatter(|n, _| format!("{:.2}", n))
                            .text(""),
                        "vertical spacing between rows (in base pixels)",
                    );
                    ui.end_row();
                    dense_slider(
                        ui,
                        "rotationDeg",
                        Slider::new(&mut layout.rotation_deg, -45.0..=45.0)
                            .step_by(0.1)
                            .suffix("\u{00B0}")
                            .custom_formatter(|n, _| format!("{:.2}", n))
                            .text(""),
                        "whole-canvas rotation around center, in degrees",
                    );
                    ui.end_row();
                    dense_slider(
                        ui,
                        "rowStaggerPx",
                        Slider::new(&mut layout.row_stagger_px, 0.0..=1.0)
                            .step_by(0.01)
                            .custom_formatter(|n, _| format!("{:.2}", n))
                            .text(""),
                        "horizontal offset per row for a staggered look (fraction of unit width)",
                    );
                    ui.end_row();
                    dense_slider(
                        ui,
                        "marginPx",
                        Slider::new(&mut layout.margin_px, 0.0..=240.0)
                            .custom_formatter(|n, _| format!("{:.2}", n))
                            .text(""),
                        "outer margin around all rows (base pixels)",
                    );
                    ui.end_row();
                    dense_slider(
                        ui,
                        "renderScale (live)",
                        Slider::new(render_scale, 1.0..=4.0).step_by(1.0).text(""),
                        "live GPU render supersampling multiplier",
                    );
                    ui.end_row();
                });
        });
    }

    // ---------------- colors ----------------
    fn section_colors(&mut self, ui: &mut egui::Ui, colors: &mut ColorConfig) {
        section_header(ui, "Colors", false, |ui| {
            rgb_edit(ui, "background", &mut colors.background);
            rgb_edit(ui, "dim", &mut colors.dim);
            rgb_edit(ui, "key", &mut colors.key);
        });
    }

    // ---------------- spotlight ----------------
    fn section_spotlight(&mut self, ui: &mut egui::Ui, key: &mut KeyConfig) {
        section_header(ui, "Spotlight", false, |ui| {
            // rows mode
            let mut rows = key.rows;
            ComboBox::from_label("rows")
                .width(combo_half_width(ui))
                .selected_text(key_rows_label(rows))
                .show_ui(ui, |ui| {
                    for r in [KeyRows::All, KeyRows::Even, KeyRows::Odd, KeyRows::Single] {
                        ui.selectable_value(&mut rows, r, key_rows_label(r));
                    }
                });
            key.rows = rows;

            egui::Grid::new("spotlight-sliders")
                .num_columns(1)
                .spacing([8.0, 4.0])
                .show(ui, |ui| {
                    dense_slider(
                        ui,
                        "frequency",
                        Slider::new(&mut key.frequency, 1..=50).text(""),
                        "how often (in units) the spotlight color repeats",
                    );
                    ui.end_row();
                    dense_slider(
                        ui,
                        "offsetUnits",
                        Slider::new(&mut key.offset_units, -50..=50).text(""),
                        "offset of the spotlight pattern in units (shift left/right)",
                    );
                    ui.end_row();
                });

            ui.checkbox(&mut key.row_phase_offset, "rowPhaseOffset")
                .on_hover_text("stagger the spotlight phase across rows");

            let mut mode = key.mode;
            ComboBox::from_label("mode")
                .width(combo_half_width(ui))
                .selected_text(key_mode_label(mode))
                .show_ui(ui, |ui| {
                    for m in [KeyMode::Travels, KeyMode::Anchored] {
                        ui.selectable_value(&mut mode, m, key_mode_label(m));
                    }
                });
            key.mode = mode;

            egui::Grid::new("spotlight-anchor")
                .num_columns(1)
                .spacing([8.0, 4.0])
                .show(ui, |ui| {
                    dense_slider(
                        ui,
                        "anchorScreenOffsetPx",
                        Slider::new(&mut key.anchor_screen_offset_px, -1000.0..=1000.0)
                            .custom_formatter(|n, _| format!("{:.2}", n))
                            .text(""),
                        "when mode=anchored, px offset of the spotlight from screen center",
                    );
                    ui.end_row();
                });

            ui.horizontal(|ui| {
                ui.set_width(LEAD_COL_WIDTH);
                ui.label("singleRow");
                let resp = ui
                    .text_edit_singleline(&mut self.single_row_buf)
                    .on_hover_text(
                        "when rows=single, pick which row: 'middle' or an integer index",
                    );
                if resp.lost_focus() || resp.changed() {
                    let v = self.single_row_buf.trim();
                    if v.eq_ignore_ascii_case("middle") {
                        key.single_row = KeyRowValue::Middle(MiddleTag::Middle);
                    } else if let Ok(n) = v.parse::<i64>() {
                        let n = n.max(0) as u32;
                        key.single_row = KeyRowValue::Index(n);
                    }
                }
            });
        });
    }

    // ---------------- text ----------------
    fn section_text(&mut self, ui: &mut egui::Ui, text: &mut TextConfig) {
        section_header(ui, "Text", false, |ui| {
            let mut case = text.case;
            ComboBox::from_label("case")
                .width(combo_half_width(ui))
                .selected_text(text_case_label(case))
                .show_ui(ui, |ui| {
                    for c in [TextCase::Lower, TextCase::Upper, TextCase::AsIs] {
                        ui.selectable_value(&mut case, c, text_case_label(c));
                    }
                });
            text.case = case;
        });
    }

    // ---------------- band thresholds ----------------
    fn section_band_thresholds(
        &mut self,
        ui: &mut egui::Ui,
        thresholds: &mut std::collections::BTreeMap<String, f32>,
    ) {
        section_header(ui, "Band thresholds", false, |ui| {
            // Iterate Band::ALL for consistent kick->snare->mid->hats order (item 8).
            egui::Grid::new("band-threshold-sliders")
                .num_columns(1)
                .spacing([8.0, 4.0])
                .show(ui, |ui| {
                    for band in Band::ALL.iter() {
                        let key = band.as_str().to_string();
                        let entry = thresholds.entry(key.clone()).or_insert(0.0);
                        ui.horizontal(|ui| {
                            band_dot(ui, *band);
                            ui.add(
                                Slider::new(entry, 0.0..=1.0)
                                    .step_by(0.01)
                                    .custom_formatter(|n, _| format!("{:.2}", n))
                                    .text(band.as_str()),
                            )
                            .on_hover_text(format!(
                                "{}: minimum transient strength to fire for this band",
                                band.as_str()
                            ));
                        });
                        ui.end_row();
                    }
                });
        });
    }

    // ---------------- tickers ----------------
    fn section_tickers(&mut self, ui: &mut egui::Ui, tickers: &mut [TickerConfig]) {
        section_header(ui, "Tickers", false, |ui| {
            for t in tickers.iter_mut() {
                // Collapsing header per ticker; band color dot is painted inside the body.
                egui::CollapsingHeader::new(RichText::new(&t.id).strong())
                    .default_open(false)
                    .id_salt(format!("ticker-{}", t.id))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            band_dot(ui, t.band);
                            secondary_label(ui, t.band.as_str());
                        });

                        egui::Grid::new(format!("ticker-grid-{}", t.id))
                            .num_columns(1)
                            .spacing([8.0, 4.0])
                            .show(ui, |ui| {
                                dense_slider(
                                    ui,
                                    "modulo",
                                    Slider::new(&mut t.modulo, 1..=32).text(""),
                                    "fire once every N events of the selected band",
                                );
                                ui.end_row();

                                // band
                                let mut band = t.band;
                                ui.horizontal(|ui| {
                                    ui.set_width(LEAD_COL_WIDTH);
                                    ui.label("band");
                                });
                                ComboBox::from_id_salt(format!("ticker-band-{}", t.id))
                                    .width(combo_half_width(ui))
                                    .selected_text(band.as_str())
                                    .show_ui(ui, |ui| {
                                        for b in Band::ALL {
                                            ui.selectable_value(&mut band, b, b.as_str());
                                        }
                                    });
                                t.band = band;
                                ui.end_row();

                                // strength threshold (Option<f32>)
                                let mut st = t.strength_threshold.unwrap_or(0.0);
                                ui.horizontal(|ui| {
                                    ui.set_width(LEAD_COL_WIDTH);
                                    ui.label("strengthThreshold");
                                });
                                if ui
                                    .add(
                                        Slider::new(&mut st, 0.0..=1.0)
                                            .step_by(0.01)
                                            .custom_formatter(|n, _| format!("{:.2}", n))
                                            .text(""),
                                    )
                                    .on_hover_text(
                                        "per-ticker minimum transient strength (overrides band threshold)",
                                    )
                                    .changed()
                                {
                                    t.strength_threshold = Some(st);
                                }
                                ui.end_row();

                                // step (Option<u32>)
                                let mut step = t.step.unwrap_or(1);
                                ui.horizontal(|ui| {
                                    ui.set_width(LEAD_COL_WIDTH);
                                    ui.label("step");
                                });
                                if ui
                                    .add(Slider::new(&mut step, 1..=32).text(""))
                                    .on_hover_text("stride through phases (advance by N per fire)")
                                    .changed()
                                {
                                    t.step = Some(step);
                                }
                                ui.end_row();

                                // step offset (Option<u32>)
                                let mut so = t.step_offset.unwrap_or(0);
                                ui.horizontal(|ui| {
                                    ui.set_width(LEAD_COL_WIDTH);
                                    ui.label("stepOffset");
                                });
                                if ui
                                    .add(Slider::new(&mut so, 0..=32).text(""))
                                    .on_hover_text("initial step offset")
                                    .changed()
                                {
                                    t.step_offset = Some(so);
                                }
                                ui.end_row();

                                // offset (Option<i32>)
                                let mut off = t.offset.unwrap_or(0);
                                ui.horizontal(|ui| {
                                    ui.set_width(LEAD_COL_WIDTH);
                                    ui.label("offset");
                                });
                                if ui
                                    .add(Slider::new(&mut off, -16..=16).text(""))
                                    .on_hover_text("initial event count offset")
                                    .changed()
                                {
                                    t.offset = Some(off);
                                }
                                ui.end_row();
                            });
                    });
            }
        });
    }

    // ---------------- bindings ----------------
    fn section_bindings(&mut self, ui: &mut egui::Ui, bindings: &mut [Binding]) {
        section_header(ui, "Bindings", false, |ui| {
            for (idx, b) in bindings.iter_mut().enumerate() {
                let target = match b.target {
                    crate::types::BindingTarget::Rows => "rows",
                    crate::types::BindingTarget::Camera => "camera",
                };
                // Alternation chip (item 10): check if respect_alternation != Some(false).
                let alt_mark = if b.respect_alternation == Some(false) {
                    "\u{2014}"
                } else {
                    "\u{2713}"
                };
                let title = format!("{}  {}: {} → {}", alt_mark, idx, b.ticker_id, target);
                egui::CollapsingHeader::new(title)
                    .default_open(false)
                    .id_salt(format!("binding-{}", idx))
                    .show(ui, |ui| {
                        for (i, action) in b.sequence.actions.iter_mut().enumerate() {
                            let phase_title = format!("phase {}: {}", i, action.path);
                            egui::CollapsingHeader::new(phase_title)
                                .default_open(false)
                                .id_salt(format!("binding-{}-phase-{}", idx, i))
                                .show(ui, |ui| {
                                    phase_action_ui(ui, action);
                                });
                        }
                    });
            }
        });
    }

    // ---------------- camera ----------------
    fn section_camera(&mut self, ui: &mut egui::Ui, camera: &mut CameraConfig) {
        section_header(ui, "Camera baseline", false, |ui| {
            let b: &mut CameraBaseline = &mut camera.baseline;
            egui::Grid::new("camera-sliders")
                .num_columns(1)
                .spacing([8.0, 4.0])
                .show(ui, |ui| {
                    dense_slider(
                        ui,
                        "zoom",
                        Slider::new(&mut b.zoom, 0.1..=4.0)
                            .step_by(0.01)
                            .custom_formatter(|n, _| format!("{:.2}", n))
                            .text(""),
                        "camera zoom multiplier (1.0 = fit, >1 = closer)",
                    );
                    ui.end_row();
                    dense_slider(
                        ui,
                        "panX",
                        Slider::new(&mut b.pan_x, -2000.0..=2000.0)
                            .custom_formatter(|n, _| format!("{:.2}", n))
                            .text(""),
                        "horizontal pan baseline (base pixels)",
                    );
                    ui.end_row();
                    dense_slider(
                        ui,
                        "panY",
                        Slider::new(&mut b.pan_y, -2000.0..=2000.0)
                            .custom_formatter(|n, _| format!("{:.2}", n))
                            .text(""),
                        "vertical pan baseline (base pixels)",
                    );
                    ui.end_row();
                    dense_slider(
                        ui,
                        "tilt",
                        Slider::new(&mut b.tilt, -180.0..=180.0)
                            .step_by(0.1)
                            .suffix("\u{00B0}")
                            .custom_formatter(|n, _| format!("{:.2}", n))
                            .text(""),
                        "camera tilt (rotation) baseline, in degrees",
                    );
                    ui.end_row();
                    dense_slider(
                        ui,
                        "rowOvershoot",
                        Slider::new(&mut camera.row_overshoot, 0..=10).text(""),
                        "extra rows rendered beyond viewport edges",
                    );
                    ui.end_row();
                });
        });
    }

    // ---------------- record ----------------
    fn section_record(
        &mut self,
        ui: &mut egui::Ui,
        record: &mut RecordConfig,
        callbacks: &mut PanelCallbacks,
    ) {
        section_header(ui, "Record", true, |ui| {
            egui::Grid::new("record-sliders")
                .num_columns(1)
                .spacing([8.0, 4.0])
                .show(ui, |ui| {
                    dense_slider(
                        ui,
                        "start (s)",
                        Slider::new(&mut record.start_at, 0.0..=600.0)
                            .step_by(0.1)
                            .custom_formatter(|n, _| format!("{:.2}", n))
                            .text(""),
                        "start timestamp of the export (seconds)",
                    );
                    ui.end_row();
                    dense_slider(
                        ui,
                        "duration (s)",
                        Slider::new(&mut record.duration, 0.5..=300.0)
                            .step_by(0.5)
                            .custom_formatter(|n, _| format!("{:.2}", n))
                            .text(""),
                        "length of the export in seconds",
                    );
                    ui.end_row();
                });

            let mut fps = record.fps;
            ComboBox::from_label("fps")
                .width(combo_half_width(ui))
                .selected_text(format!("{}", fps))
                .show_ui(ui, |ui| {
                    for f in [24u32, 30, 60] {
                        ui.selectable_value(&mut fps, f, format!("{}", f));
                    }
                });
            record.fps = fps;

            egui::Grid::new("record-sliders-2")
                .num_columns(1)
                .spacing([8.0, 4.0])
                .show(ui, |ui| {
                    dense_slider(
                        ui,
                        "bitrate (Mbps)",
                        Slider::new(&mut record.video_bitrate_mbps, 2..=80).text(""),
                        "H.264 target video bitrate in megabits per second",
                    );
                    ui.end_row();
                    dense_slider(
                        ui,
                        "res x",
                        Slider::new(&mut record.resolution_multiplier, 1..=4).text(""),
                        "resolution multiplier applied to the canvas for export",
                    );
                    ui.end_row();
                });

            let mut engine = record.engine;
            ComboBox::from_label("engine")
                .width(combo_half_width(ui))
                .selected_text(record_engine_label(engine))
                .show_ui(ui, |ui| {
                    for e in [
                        RecordEngine::Auto,
                        RecordEngine::Native,
                        RecordEngine::Ffmpeg,
                    ] {
                        ui.selectable_value(&mut engine, e, record_engine_label(e));
                    }
                });
            record.engine = engine;

            ui.add_space(SECTION_PAD);
            soft_divider(ui);
            ui.add_space(SECTION_PAD);

            // 120px pill-shaped record button, right-aligned.
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let primary =
                    egui::Button::new(RichText::new("Record").strong().color(Color32::WHITE))
                        .fill(SELECTION_BLUE)
                        .rounding(14.0);
                if ui
                    .add_sized([120.0, 28.0], primary)
                    .on_hover_text(
                        "Export the current config to an .mp4 using the selected engine.\n\
                         Starts at `start (s)`, runs for `duration (s)` at the chosen fps.",
                    )
                    .clicked()
                {
                    if let Some(cb) = callbacks.on_record.as_mut() {
                        cb();
                    }
                }
            });

            ui.horizontal(|ui| {
                if ui
                    .button("copy config JSON")
                    .on_hover_text(
                        "Serialize the full Config to JSON and copy it to the clipboard.",
                    )
                    .clicked()
                {
                    if let Some(cb) = callbacks.on_copy_config.as_mut() {
                        cb();
                    }
                }
                if ui
                    .button("reload analysis")
                    .on_hover_text(
                        "Re-read analysis/<track>.json for the currently-loaded audio file (⌘R).",
                    )
                    .clicked()
                {
                    if let Some(cb) = callbacks.on_reload_analysis.as_mut() {
                        cb();
                    }
                }
            });
        });
    }

    // ---------------- debug ----------------
    fn section_debug(
        &mut self,
        ui: &mut egui::Ui,
        debug: &mut DebugConfig,
        _callbacks: &mut PanelCallbacks,
    ) {
        section_header(ui, "Debug", false, |ui| {
            ui.checkbox(&mut debug.hud, "hud")
                .on_hover_text("show on-canvas HUD (fps, timing, ticker phase)");
        });
    }
}

// -------------------------------------------------------------------
// Helpers
// -------------------------------------------------------------------

/// Draws a bare CollapsingHeader (no frame, no leading separator) whose title is
/// sentence-case, strong, 13px — Logic Pro inspector style. Tightens vertical rhythm
/// inside the body.
fn section_header<R>(
    ui: &mut egui::Ui,
    title: &str,
    default_open: bool,
    body: impl FnOnce(&mut egui::Ui) -> R,
) -> Option<R> {
    let title = RichText::new(title).strong().size(13.0);
    let mut out = None;
    egui::CollapsingHeader::new(title)
        .default_open(default_open)
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 4.0;
            out = Some(body(ui));
        });
    out
}

/// Logic Pro–style slider row: label trails the slider (via `Slider::text`), value is
/// shown inline by Slider by default. One widget per row — no label column.
fn dense_slider<'a>(
    ui: &mut egui::Ui,
    label: &str,
    mut slider: Slider<'a>,
    tooltip: &str,
) -> egui::Response {
    // If the Slider was pre-built with `.text("")`, overwrite with the real label so
    // callers don't need to be updated.
    slider = slider.text(label);
    ui.add(slider.show_value(true)).on_hover_text(tooltip)
}

const ALL_EASING_NAMES: [EasingName; 19] = [
    EasingName::Linear,
    EasingName::EaseIn,
    EasingName::EaseOut,
    EasingName::EaseInOut,
    EasingName::EaseOutCubic,
    EasingName::EaseInOutCubic,
    EasingName::EaseInQuad,
    EasingName::EaseOutQuad,
    EasingName::EaseInOutQuad,
    EasingName::EaseInQuart,
    EasingName::EaseOutQuart,
    EasingName::EaseInOutQuart,
    EasingName::EaseInExpo,
    EasingName::EaseOutExpo,
    EasingName::EaseInOutExpo,
    EasingName::EaseOutBack,
    EasingName::EaseInOutBack,
    EasingName::EaseOutElastic,
    EasingName::EaseOutBounce,
];

fn feel_preset_label(p: FeelPreset) -> &'static str {
    match p {
        FeelPreset::Custom => "custom",
        FeelPreset::Default => "default",
        FeelPreset::Snappy => "snappy",
        FeelPreset::Smooth => "smooth",
        FeelPreset::Punchy => "punchy",
        FeelPreset::Glacial => "glacial",
        FeelPreset::Bouncy => "bouncy",
    }
}

fn row_velocity_shape_label(s: RowVelocityShape) -> &'static str {
    match s {
        RowVelocityShape::Flat => "flat",
        RowVelocityShape::U => "u",
        RowVelocityShape::InvertedU => "inverted-u",
        RowVelocityShape::Linear => "linear",
    }
}

fn global_easing_label(g: GlobalEasing) -> &'static str {
    match g {
        GlobalEasing::Auto => "auto",
        GlobalEasing::Named(n) => easing_name_label(n),
    }
}

fn easing_name_label(e: EasingName) -> &'static str {
    match e {
        EasingName::Linear => "linear",
        EasingName::EaseIn => "easeIn",
        EasingName::EaseOut => "easeOut",
        EasingName::EaseInOut => "easeInOut",
        EasingName::EaseOutCubic => "easeOutCubic",
        EasingName::EaseInOutCubic => "easeInOutCubic",
        EasingName::EaseInQuad => "easeInQuad",
        EasingName::EaseOutQuad => "easeOutQuad",
        EasingName::EaseInOutQuad => "easeInOutQuad",
        EasingName::EaseInQuart => "easeInQuart",
        EasingName::EaseOutQuart => "easeOutQuart",
        EasingName::EaseInOutQuart => "easeInOutQuart",
        EasingName::EaseInExpo => "easeInExpo",
        EasingName::EaseOutExpo => "easeOutExpo",
        EasingName::EaseInOutExpo => "easeInOutExpo",
        EasingName::EaseOutBack => "easeOutBack",
        EasingName::EaseInOutBack => "easeInOutBack",
        EasingName::EaseOutElastic => "easeOutElastic",
        EasingName::EaseOutBounce => "easeOutBounce",
    }
}

fn key_rows_label(r: KeyRows) -> &'static str {
    match r {
        KeyRows::All => "all",
        KeyRows::Even => "even",
        KeyRows::Odd => "odd",
        KeyRows::Single => "single",
    }
}

fn key_mode_label(m: KeyMode) -> &'static str {
    match m {
        KeyMode::Travels => "travels",
        KeyMode::Anchored => "anchored",
    }
}

fn text_case_label(c: TextCase) -> &'static str {
    match c {
        TextCase::Lower => "lower",
        TextCase::Upper => "upper",
        TextCase::AsIs => "as-is",
    }
}

fn record_engine_label(e: RecordEngine) -> &'static str {
    match e {
        RecordEngine::Auto => "auto",
        RecordEngine::Native => "native",
        RecordEngine::Ffmpeg => "ffmpeg",
    }
}

fn key_row_to_string(v: &KeyRowValue) -> String {
    match v {
        KeyRowValue::Middle(_) => "middle".to_string(),
        KeyRowValue::Index(i) => i.to_string(),
    }
}

/// Apply a feel preset (durationScale + globalEasing). Mirrors the TS FEEL_PRESETS table.
fn apply_preset(feel: &mut FeelConfig, preset: FeelPreset) {
    let (ds, ge) = match preset {
        FeelPreset::Custom => return,
        FeelPreset::Default => (1.0, GlobalEasing::Auto),
        FeelPreset::Snappy => (0.55, GlobalEasing::Named(EasingName::EaseOutCubic)),
        FeelPreset::Smooth => (1.4, GlobalEasing::Named(EasingName::EaseInOutCubic)),
        FeelPreset::Punchy => (0.75, GlobalEasing::Named(EasingName::EaseOutExpo)),
        FeelPreset::Glacial => (2.6, GlobalEasing::Named(EasingName::EaseInOutQuart)),
        FeelPreset::Bouncy => (1.1, GlobalEasing::Named(EasingName::EaseOutBack)),
    };
    feel.duration_scale = ds;
    feel.global_easing = ge;
}

/// Parse "#rrggbb" → [u8;3], mutate via egui color picker, write back as lowercase "#rrggbb".
fn rgb_edit(ui: &mut egui::Ui, label: &str, hex_mut: &mut String) {
    let mut rgb = parse_hex_rgb(hex_mut).unwrap_or([0, 0, 0]);
    ui.horizontal(|ui| {
        ui.set_width(LEAD_COL_WIDTH);
        ui.label(label);
        if egui::color_picker::color_edit_button_srgb(ui, &mut rgb).changed() {
            *hex_mut = format!("#{:02x}{:02x}{:02x}", rgb[0], rgb[1], rgb[2]);
        }
    });
}

fn parse_hex_rgb(s: &str) -> Option<[u8; 3]> {
    let s = s.trim();
    let s = s.strip_prefix('#').unwrap_or(s);
    if s.len() == 6 {
        let r = u8::from_str_radix(&s[0..2], 16).ok()?;
        let g = u8::from_str_radix(&s[2..4], 16).ok()?;
        let b = u8::from_str_radix(&s[4..6], 16).ok()?;
        Some([r, g, b])
    } else if s.len() == 3 {
        let r = u8::from_str_radix(&s[0..1], 16).ok()? * 17;
        let g = u8::from_str_radix(&s[1..2], 16).ok()? * 17;
        let b = u8::from_str_radix(&s[2..3], 16).ok()? * 17;
        Some([r, g, b])
    } else {
        None
    }
}

/// UI for one PhaseAction: path (free-text), duration, easing dropdown.
fn phase_action_ui(ui: &mut egui::Ui, action: &mut PhaseAction) {
    ui.horizontal(|ui| {
        ui.set_width(LEAD_COL_WIDTH);
        ui.label("path");
        ui.text_edit_singleline(&mut action.path)
            .on_hover_text("path identifier for this phase (e.g. 'rows.horizontalScroll')");
    });
    ui.horizontal(|ui| {
        ui.set_width(LEAD_COL_WIDTH);
        ui.label("duration");
        ui.add(
            Slider::new(&mut action.duration, 0.01..=4.0)
                .step_by(0.01)
                .custom_formatter(|n, _| format!("{:.2}", n))
                .text(""),
        )
        .on_hover_text("length of this phase in beats (before feel.duration_scale)");
    });

    // easing: Option<Easing>. We expose "linear" placeholder for None and all 19 named easings.
    // (Bezier easings are not editable here; a Bezier value is preserved on write-back by not clobbering it until the user picks a new named value.)
    let current_label: &'static str = match action.easing {
        None => "linear",
        Some(Easing::Named(n)) => easing_name_label(n),
        Some(Easing::Bezier { .. }) => "bezier",
    };

    #[derive(Copy, Clone, PartialEq, Eq)]
    enum EasingChoice {
        None,
        Named(EasingName),
    }
    let mut choice = match action.easing {
        None => EasingChoice::None,
        Some(Easing::Named(n)) => EasingChoice::Named(n),
        // Treat bezier as "None" sentinel for the dropdown; user must pick a named value to override.
        Some(Easing::Bezier { .. }) => EasingChoice::None,
    };

    ComboBox::from_label("easing")
        .width(combo_half_width(ui))
        .selected_text(current_label)
        .show_ui(ui, |ui| {
            ui.selectable_value(&mut choice, EasingChoice::None, "linear");
            for e in ALL_EASING_NAMES {
                ui.selectable_value(&mut choice, EasingChoice::Named(e), easing_name_label(e));
            }
        });

    match choice {
        EasingChoice::None => {
            // Only overwrite if it wasn't already None, and don't clobber a Bezier override silently.
            if matches!(action.easing, Some(Easing::Named(_))) {
                action.easing = None;
            }
        }
        EasingChoice::Named(n) => {
            action.easing = Some(Easing::Named(n));
        }
    }
}

// band_from_str is unused for now; keep it available for future binding/filter UI.
#[allow(dead_code)]
fn _band_from_str_use(_: &str) -> Option<Band> {
    band_from_str("")
}
