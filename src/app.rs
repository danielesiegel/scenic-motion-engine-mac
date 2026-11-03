//! Top-level App. The orchestrator wires this up AFTER the parallel agents have populated the modules.
//! For now this is a minimal eframe::App that renders a status screen so the project compiles.

use crate::config::defaults::default_config;
use crate::types::Config;

pub struct ScenicApp {
    pub config: Config,
    pub status: String,
}

impl ScenicApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        Self {
            config: default_config(),
            status: "scenic-motion-engine: scaffold.\nModules fill in incrementally as agents land.".to_string(),
        }
    }
}

impl eframe::App for ScenicApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("scenic-motion-engine");
            ui.separator();
            ui.monospace(&self.status);
            ui.separator();
            ui.label(format!("canvas: {}×{}", self.config.canvas.width, self.config.canvas.height));
            ui.label(format!("tickers: {}", self.config.tickers.len()));
            ui.label(format!("bindings: {}", self.config.bindings.len()));
        });
    }
}
