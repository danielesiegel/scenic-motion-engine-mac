//! Welcome screen shown in the central panel when no audio track is loaded.
//!
//! Centered, multi-line onboarding state: giant title, subtitle, numbered steps,
//! and a hotkey hint. The central panel already paints its own background, so
//! this draws text-only on top.

use egui::{Color32, RichText};

#[derive(Default)]
pub struct WelcomeScreen {
    /// Reserved for future interactive state (hover highlights, dismissal, etc.).
    _reserved: (),
}

impl WelcomeScreen {
    pub fn ui(&mut self, ui: &mut egui::Ui) {
        let subtle = Color32::from_rgba_unmultiplied(230, 230, 232, 160);
        let available = ui.available_rect_before_wrap();

        // Rough vertical centering: estimate block height and push the top down by
        // half of the leftover space. Numbers are hand-tuned to feel balanced for the
        // title/subtitle/steps/tip stack below.
        let estimated_block_height = 48.0 + 12.0 + 18.0 + 20.0 + 3.0 * 44.0 + 24.0 + 16.0;
        let computed_offset = ((available.height() - estimated_block_height) * 0.5).max(16.0);

        ui.vertical_centered(|ui| {
            ui.add_space(computed_offset);

            ui.label(RichText::new("scenic motion engine").size(48.0).strong());
            ui.add_space(12.0);
            ui.label(
                RichText::new("audio-reactive marquee for macOS")
                    .size(18.0)
                    .color(subtle),
            );
            ui.add_space(20.0);

            step(
                ui,
                "1.",
                "Drop a `.wav` into `assets/audio/`",
                "or drag anywhere on this window.",
                subtle,
            );
            step(
                ui,
                "2.",
                "Analyze with `scripts/analyze.py`",
                "produces `analysis/<track>.json`.",
                subtle,
            );
            step(
                ui,
                "3.",
                "Press Space",
                "the marquee starts syncing to transients.",
                subtle,
            );

            ui.add_space(24.0);
            ui.label(
                RichText::new("Press ⌘O to open an audio file.")
                    .small()
                    .monospace()
                    .color(subtle),
            );
        });
    }
}

fn step(ui: &mut egui::Ui, bullet: &str, title: &str, desc: &str, subtle: Color32) {
    ui.horizontal(|ui| {
        // Use spacing to visually center the row within vertical_centered's layout.
        ui.label(RichText::new(bullet).size(16.0).strong());
        ui.label(RichText::new(title).size(16.0).strong());
        ui.label(RichText::new("—").size(16.0).color(subtle));
        ui.label(RichText::new(desc).size(16.0).color(subtle));
    });
    ui.add_space(8.0);
}
