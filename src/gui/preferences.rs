//! Preferences window — Mac-style tabbed modal.
//!
//! Four tabs — General, Appearance, Keyboard, Record defaults — each surfacing
//! existing `Config` knobs in a familiar system-settings layout. Nothing here
//! owns persistence; the hosting `App` already autosaves `Config` every ~10s
//! and on ⌘S/quit, so toggling anything in here is written through transparently.
//!
//! Toggled from `app.rs` with ⌘,.

use egui::{Color32, ComboBox, Context, RichText, SelectableLabel, Window};

use crate::types::{Config, RecordEngine};

#[derive(Clone, Copy, PartialEq, Eq, Default)]
enum PrefsTab {
    #[default]
    General,
    Appearance,
    Keyboard,
    RecordDefaults,
}

/// Preferences-window state. `visible` is public so the hosting `App` can flip it when it
/// detects the ⌘, shortcut.
#[derive(Default)]
pub struct PreferencesWindow {
    pub visible: bool,
    tab: PrefsTab,
    /// Mirror of the "forced theme" picker. `None` = follow system, otherwise force dark/light.
    /// Not persisted — this is a session-only override on top of egui's visuals.
    theme_override: ThemeOverride,
    /// Session-only field: default audio directory. Not stored in `Config` today, so this
    /// knob is cosmetic — the browse button updates the string in-memory.
    default_audio_dir: String,
    /// Session-only field: "Restore last audio file on startup". Not stored in `Config`
    /// today — surfaced here for future wiring.
    restore_last_audio: bool,
    /// Session-only field: panel accent hex color, lazily seeded from `config.colors.key`.
    accent_rgb: Option<[u8; 3]>,
}

#[derive(Clone, Copy, PartialEq, Eq, Default)]
enum ThemeOverride {
    #[default]
    System,
    Light,
    Dark,
}

impl PreferencesWindow {
    /// Flip visibility (used by the ⌘, shortcut).
    pub fn toggle(&mut self) {
        self.visible = !self.visible;
    }

    pub fn ui(&mut self, ctx: &Context, cfg: &mut Config) {
        if !self.visible {
            return;
        }

        // Seed the audio-dir mirror from Config on first show.
        if self.default_audio_dir.is_empty() {
            if let Some(track) = cfg.audio.track.as_deref() {
                self.default_audio_dir = track.to_string();
            } else {
                self.default_audio_dir = "assets/audio".to_string();
            }
        }

        let mut open = self.visible;
        Window::new("Preferences")
            .collapsible(false)
            .resizable(false)
            .default_width(520.0)
            .default_height(360.0)
            .open(&mut open)
            .show(ctx, |ui| {
                // Tab bar — row of SelectableLabel buttons.
                ui.horizontal(|ui| {
                    tab_btn(ui, &mut self.tab, PrefsTab::General, "General");
                    tab_btn(ui, &mut self.tab, PrefsTab::Appearance, "Appearance");
                    tab_btn(ui, &mut self.tab, PrefsTab::Keyboard, "Keyboard");
                    tab_btn(
                        ui,
                        &mut self.tab,
                        PrefsTab::RecordDefaults,
                        "Record defaults",
                    );
                });
                ui.separator();
                ui.add_space(6.0);

                match self.tab {
                    PrefsTab::General => self.general_tab(ui, cfg),
                    PrefsTab::Appearance => self.appearance_tab(ui, ctx, cfg),
                    PrefsTab::Keyboard => keyboard_tab(ui),
                    PrefsTab::RecordDefaults => record_tab(ui, cfg),
                }
            });

        if !open {
            self.visible = false;
        }
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.visible = false;
        }
    }

    fn general_tab(&mut self, ui: &mut egui::Ui, cfg: &mut Config) {
        // "Save config on quit" — always true for now (autosave + on_exit do this), shown disabled.
        let mut always_true = true;
        ui.add_enabled(
            false,
            egui::Checkbox::new(&mut always_true, "Save config on quit"),
        );
        ui.checkbox(
            &mut self.restore_last_audio,
            "Restore last audio file on startup",
        );
        ui.add_space(8.0);
        ui.label(RichText::new("Default audio directory").small());
        ui.horizontal(|ui| {
            ui.add(
                egui::TextEdit::singleline(&mut self.default_audio_dir)
                    .desired_width(ui.available_width() - 90.0),
            );
            if ui.button("Browse…").clicked() {
                if let Some(dir) = rfd::FileDialog::new().pick_folder() {
                    self.default_audio_dir = dir.to_string_lossy().into_owned();
                    // Also mirror it onto Config so the autosave captures it.
                    cfg.audio.track = Some(self.default_audio_dir.clone());
                }
            }
        });
    }

    fn appearance_tab(&mut self, ui: &mut egui::Ui, ctx: &Context, cfg: &mut Config) {
        ui.label(RichText::new("Forced theme override").small());
        ui.horizontal(|ui| {
            if ui
                .selectable_label(self.theme_override == ThemeOverride::System, "System")
                .clicked()
            {
                self.theme_override = ThemeOverride::System;
            }
            if ui
                .selectable_label(self.theme_override == ThemeOverride::Light, "Light")
                .clicked()
            {
                self.theme_override = ThemeOverride::Light;
                ctx.set_visuals(egui::Visuals::light());
            }
            if ui
                .selectable_label(self.theme_override == ThemeOverride::Dark, "Dark")
                .clicked()
            {
                self.theme_override = ThemeOverride::Dark;
                ctx.set_visuals(egui::Visuals::dark());
            }
        });

        ui.add_space(12.0);
        ui.label(RichText::new("Panel accent color").small());
        // Lazily seed from Config's key color.
        let seeded = self
            .accent_rgb
            .get_or_insert_with(|| parse_hex_rgb(&cfg.colors.key).unwrap_or([10, 132, 255]));
        if egui::color_picker::color_edit_button_srgb(ui, seeded).changed() {
            cfg.colors.key = format!("#{:02x}{:02x}{:02x}", seeded[0], seeded[1], seeded[2]);
        }
    }
}

fn tab_btn(ui: &mut egui::Ui, state: &mut PrefsTab, this: PrefsTab, label: &str) {
    if ui.add(SelectableLabel::new(*state == this, label)).clicked() {
        *state = this;
    }
}

fn keyboard_tab(ui: &mut egui::Ui) {
    ui.label(RichText::new("Global shortcuts").small());
    ui.add_space(6.0);
    egui::ScrollArea::vertical()
        .max_height(260.0)
        .show(ui, |ui| {
            egui::Grid::new("prefs-keyboard-grid")
                .num_columns(2)
                .spacing([20.0, 8.0])
                .striped(true)
                .show(ui, |ui| {
                    for (key, desc) in SHORTCUTS {
                        key_cell(ui, key);
                        ui.label(*desc);
                        ui.end_row();
                    }
                });
        });
}

fn record_tab(ui: &mut egui::Ui, cfg: &mut Config) {
    ui.label(RichText::new("Default record settings").small());
    ui.add_space(6.0);

    ui.horizontal(|ui| {
        ui.label("fps");
        ComboBox::from_id_salt("prefs-fps")
            .selected_text(format!("{}", cfg.record.fps))
            .show_ui(ui, |ui| {
                for f in [24u32, 30, 48, 60, 120] {
                    ui.selectable_value(&mut cfg.record.fps, f, format!("{}", f));
                }
            });
    });

    ui.horizontal(|ui| {
        ui.label("video bitrate (Mbps)");
        ui.add(egui::DragValue::new(&mut cfg.record.video_bitrate_mbps).range(1..=200));
    });

    ui.horizontal(|ui| {
        ui.label("resolution ×");
        ComboBox::from_id_salt("prefs-resmult")
            .selected_text(format!("{}×", cfg.record.resolution_multiplier.max(1)))
            .show_ui(ui, |ui| {
                for m in [1u32, 2, 3, 4] {
                    ui.selectable_value(&mut cfg.record.resolution_multiplier, m, format!("{}×", m));
                }
            });
    });

    ui.horizontal(|ui| {
        ui.label("engine");
        ComboBox::from_id_salt("prefs-engine")
            .selected_text(match cfg.record.engine {
                RecordEngine::Auto => "auto",
                RecordEngine::Native => "native",
                RecordEngine::Ffmpeg => "ffmpeg",
            })
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut cfg.record.engine, RecordEngine::Auto, "auto");
                ui.selectable_value(&mut cfg.record.engine, RecordEngine::Native, "native");
                ui.selectable_value(&mut cfg.record.engine, RecordEngine::Ffmpeg, "ffmpeg");
            });
    });
}

/// Render a single key cap: a monospace label with a dark background pill.
/// Mirrors `gui::shortcuts::key_cell` so the look is consistent across both overlays.
fn key_cell(ui: &mut egui::Ui, label: &str) {
    let text = RichText::new(label)
        .monospace()
        .background_color(Color32::from_gray(40))
        .color(Color32::WHITE);
    egui::Frame::none()
        .fill(Color32::from_gray(40))
        .rounding(4.0)
        .inner_margin(egui::Margin::symmetric(6.0, 2.0))
        .show(ui, |ui| {
            ui.label(text);
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
    } else {
        None
    }
}

/// Static table of shortcut rows — must mirror the cheatsheet overlay.
const SHORTCUTS: &[(&str, &str)] = &[
    ("Space", "Play / pause"),
    ("\u{2318}O", "Open audio file"),
    ("\u{2318}R", "Reload analysis JSON"),
    ("\u{2318}.", "Stop (seek to loop start)"),
    ("\u{2318}S", "Save config now"),
    ("\u{2318},", "Open Preferences"),
    ("\u{2318}\u{21E7}?", "About scenic motion engine"),
    ("\u{2190} / \u{2192}", "Seek -5s / +5s (hold Shift for \u{00B1}1s)"),
    ("Home", "Seek to loop start"),
    ("End", "Seek to end"),
    ("L", "Toggle loop"),
    ("H", "Toggle HUD"),
    ("?", "Show shortcuts cheatsheet"),
    ("Esc", "Close overlays"),
];
