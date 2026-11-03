//! Floating HUD overlay drawn on top of the stage glyphs.
//!
//! Mirrors `scenicsynths/src/preview/hud.ts` — shows transport info (t, bpm,
//! beat#, bar#) and one row per ticker with a band-colored dot, a flash
//! indicator, and the active path name.

use egui::{vec2, Color32, FontId, Rect, Rounding, Stroke};

use crate::types::{Band, EventSource, FrameState};

#[derive(Default)]
pub struct HudOverlay;

impl HudOverlay {
    pub fn ui(&mut self, ui: &mut egui::Ui, state: &FrameState, source: &dyn EventSource) {
        let origin = ui.min_rect().min + vec2(12.0, 12.0);
        let size = vec2(380.0, 320.0);
        let rect = Rect::from_min_size(origin, size);

        ui.allocate_new_ui(egui::UiBuilder::new().max_rect(rect), |ui| {
            let frame = egui::Frame::popup(ui.style())
                .fill(Color32::from_rgba_unmultiplied(16, 16, 18, 215))
                .rounding(Rounding::same(8.0))
                .inner_margin(egui::Margin::same(10.0))
                .stroke(Stroke::new(
                    1.0,
                    Color32::from_rgba_unmultiplied(255, 255, 255, 40),
                ));

            frame.show(ui, |ui| {
                ui.set_min_size(size - vec2(20.0, 20.0));
                let beats = source.beats_before(state.t);
                let downbeats = source.downbeats_before(state.t);
                let bpm = source.bpm();

                let header = format!(
                    "t = {:.3}s  bpm = {:.1}  beat# = {}  bar# = {}",
                    state.t, bpm, beats, downbeats
                );
                ui.label(
                    egui::RichText::new(header)
                        .monospace()
                        .color(Color32::from_rgb(230, 230, 235)),
                );

                ui.separator();

                let mono = FontId::monospace(12.0);
                for tk in &state.tickers {
                    ui.horizontal(|ui| {
                        // Band-colored dot.
                        let color = band_color(tk.band);
                        let (dot_rect, _) =
                            ui.allocate_exact_size(vec2(12.0, 12.0), egui::Sense::hover());
                        ui.painter().circle_filled(dot_rect.center(), 4.0, color);

                        // Flash indicator.
                        let flash_bright = tk.since_last < 0.08;
                        let flash_color = if flash_bright {
                            Color32::from_rgb(255, 240, 180)
                        } else {
                            Color32::from_rgba_unmultiplied(180, 180, 180, 90)
                        };
                        ui.label(
                            egui::RichText::new("●")
                                .font(mono.clone())
                                .color(flash_color),
                        );

                        // Fixed-column text.
                        let path = tk.active_path.as_deref().unwrap_or("—");
                        let id_col = truncate_pad(&tk.id, 16);
                        let line = format!(
                            "{} count={:<4} phase={} prog={:.2} str={:.2} path={}",
                            id_col, tk.count, tk.phase, tk.progress, tk.strength, path
                        );
                        ui.label(
                            egui::RichText::new(line)
                                .font(mono.clone())
                                .color(Color32::from_rgb(220, 220, 225)),
                        );
                    });
                }
            });
        });
    }
}

fn band_color(band: Band) -> Color32 {
    match band {
        Band::Kick => Color32::from_rgb(0x3A, 0x7A, 0xFF),
        Band::Snare => Color32::from_rgb(0x44, 0xCC, 0x88),
        Band::Mid => Color32::from_rgb(0xFF, 0xAA, 0x33),
        Band::Hats => Color32::from_rgb(0xFF, 0x55, 0x66),
    }
}

fn truncate_pad(s: &str, n: usize) -> String {
    let chars: Vec<char> = s.chars().collect();
    if chars.len() > n {
        chars.into_iter().take(n).collect()
    } else {
        let mut out: String = s.to_string();
        for _ in chars.len()..n {
            out.push(' ');
        }
        out
    }
}
