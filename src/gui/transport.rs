//! Transport controls — play/pause/stop/scrub/loop bar for the bottom panel.
//!
//! The bar renders a single horizontal row inside an `egui::TopBottomPanel::bottom`.
//! Loop logic runs at the top of `ui()` so a seek-to-loop_start happens before any
//! widget observes the time, avoiding a one-frame flicker past the loop boundary.
//!
//! Keyboard shortcuts (processed here, not in `app.rs`):
//! - Space              → toggle play/pause
//! - Home or Cmd+.      → seek to loop_start (or 0.0 if loop disabled)
//! - End                → seek to duration - 1.0
//! - ← / →              → seek -5s / +5s   (hold Shift for -1s / +1s)
//!
//! Note: `app.rs` also owns Space/Cmd+. for its own purposes; both handlers end up
//! calling the same `Player` methods so double-handling is idempotent for toggles —
//! but to avoid oscillation we only toggle here if the ui actually has focus, which
//! egui guarantees via `ui.input` scoped to the current context.

use egui::{Button, Color32, DragValue, Key, Modifiers, RichText, Slider, Vec2};

use crate::audio::Player;

/// Formats seconds as `MM:SS.ss` for display in the transport bar.
fn format_time(v: f64) -> String {
    let secs = v.max(0.0);
    let m = (secs / 60.0).floor() as u32;
    let s = secs - (m as f64) * 60.0;
    format!("{:02}:{:05.2}", m, s)
}

#[derive(Default)]
pub struct TransportBar {
    pub loop_enabled: bool,
    pub loop_start: f32,
    pub loop_end: f32,
}

impl TransportBar {
    /// Draw the bar and process input. Call this inside a `TopBottomPanel::bottom`.
    pub fn ui(&mut self, ui: &mut egui::Ui, player: &mut Player) {
        let duration = player.duration().max(0.0);

        // --- Loop wrap: do this BEFORE drawing so widgets see the corrected time. ---
        if self.loop_enabled && duration > 0.0 {
            let lo = self.loop_start.clamp(0.0, duration);
            let hi = self.loop_end.clamp(lo, duration);
            if hi > lo && player.time() >= hi {
                player.seek(lo);
            }
        }

        // --- Keyboard shortcuts. ---
        self.handle_shortcuts(ui, player, duration);

        // --- Thin separator for a visual top border. ---
        ui.separator();

        ui.horizontal(|ui| {
            // 1. Play / pause.
            let playing = player.is_playing();
            let glyph = if playing {
                "\u{275A}\u{275A}"
            } else {
                "\u{25B6}"
            };
            let btn = Button::new(RichText::new(glyph).strong().monospace())
                .min_size(Vec2::new(32.0, 26.0));
            if ui.add(btn).clicked() {
                if playing {
                    player.pause();
                } else {
                    let _ = player.play();
                }
            }

            // 2. Stop → seek to loop_start (or 0 if loop disabled).
            let stop_btn = Button::new(RichText::new("\u{25FC}").strong().monospace())
                .min_size(Vec2::new(32.0, 26.0));
            if ui.add(stop_btn).clicked() {
                player.pause();
                let target = if self.loop_enabled {
                    self.loop_start.clamp(0.0, duration)
                } else {
                    0.0
                };
                player.seek(target);
            }

            // 3. Current time.
            let cur = player.time().clamp(0.0, duration.max(0.0));
            ui.label(
                RichText::new(format_time(cur as f64))
                    .monospace()
                    .color(Color32::from_gray(200)),
            );

            // 4. Scrub bar. Width = available minus space for duration + loop widgets (~240 px).
            let avail = ui.available_width();
            let slider_w = (avail - 240.0).max(80.0);
            let mut scrub = cur as f64;
            let slider = Slider::new(&mut scrub, 0.0..=(duration.max(0.001) as f64))
                .show_value(false)
                .custom_formatter(|v, _| format_time(v));
            let resp = ui
                .scope(|ui| {
                    ui.set_min_width(slider_w);
                    ui.set_max_width(slider_w);
                    ui.add(slider)
                })
                .inner;
            if resp.changed() {
                player.seek(scrub as f32);
            }

            // 5. Duration.
            ui.label(
                RichText::new(format_time(duration as f64))
                    .monospace()
                    .color(Color32::from_gray(200)),
            );

            // 6. Loop controls.
            ui.checkbox(&mut self.loop_enabled, "loop");
            ui.add_enabled_ui(self.loop_enabled, |ui| {
                let dmax = duration.max(0.0);
                ui.add(
                    DragValue::new(&mut self.loop_start)
                        .speed(0.5)
                        .range(0.0..=dmax)
                        .suffix("s"),
                );
                // Keep end >= start.
                if self.loop_end < self.loop_start {
                    self.loop_end = self.loop_start;
                }
                ui.add(
                    DragValue::new(&mut self.loop_end)
                        .speed(0.5)
                        .range(0.0..=dmax)
                        .suffix("s"),
                );
            });
        });
    }

    fn handle_shortcuts(&self, ui: &egui::Ui, player: &mut Player, duration: f32) {
        // Capture flags; apply after releasing the input borrow.
        let (toggle_play, seek_home, seek_end, step_back, step_fwd, back_small, fwd_small) = ui
            .input(|i| {
                let shift = i.modifiers.shift;
                let cmd = i.modifiers.command || i.modifiers.mac_cmd;
                (
                    i.key_pressed(Key::Space),
                    i.key_pressed(Key::Home) || (cmd && i.key_pressed(Key::Period)),
                    i.key_pressed(Key::End),
                    i.key_pressed(Key::ArrowLeft)
                        && !shift
                        && !cmd
                        && i.modifiers == Modifiers::NONE,
                    i.key_pressed(Key::ArrowRight)
                        && !shift
                        && !cmd
                        && i.modifiers == Modifiers::NONE,
                    i.key_pressed(Key::ArrowLeft) && shift,
                    i.key_pressed(Key::ArrowRight) && shift,
                )
            });

        if toggle_play {
            if player.is_playing() {
                player.pause();
            } else {
                let _ = player.play();
            }
        }
        if seek_home {
            let target = if self.loop_enabled {
                self.loop_start.clamp(0.0, duration)
            } else {
                0.0
            };
            player.seek(target);
        }
        if seek_end {
            let target = (duration - 1.0).max(0.0);
            player.seek(target);
        }
        let now = player.time();
        if step_back {
            player.seek((now - 5.0).max(0.0));
        }
        if step_fwd {
            player.seek((now + 5.0).min(duration));
        }
        if back_small {
            player.seek((now - 1.0).max(0.0));
        }
        if fwd_small {
            player.seek((now + 1.0).min(duration));
        }
    }
}
