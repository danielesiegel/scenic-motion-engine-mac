//! About panel — Mac-style standard About window.
//!
//! Shows app name, version (from `CARGO_PKG_VERSION`), a one-line description,
//! author, license, and a link to the GitHub repo. Toggled from `app.rs`
//! (⌘⇧? by default). Modeless, non-resizable, centered.

use egui::{Align2, Context, RichText, Window};

/// About-window state. `visible` is public so the hosting `App` can flip it when it
/// detects the About shortcut (⌘⇧?).
#[derive(Default)]
pub struct AboutPanel {
    pub visible: bool,
}

impl AboutPanel {
    /// Flip visibility (used by the ⌘⇧? shortcut).
    pub fn toggle(&mut self) {
        self.visible = !self.visible;
    }

    pub fn ui(&mut self, ctx: &Context) {
        if !self.visible {
            return;
        }

        let mut open = self.visible;
        Window::new("About scenic motion engine")
            .collapsible(false)
            .resizable(false)
            .movable(true)
            .anchor(Align2::CENTER_CENTER, [0.0, 0.0])
            .default_size([420.0, 260.0])
            .open(&mut open)
            .show(ctx, |ui| {
                ui.add_space(10.0);
                ui.vertical_centered(|ui| {
                    ui.label(RichText::new("scenic motion engine").size(24.0).strong());
                    ui.add_space(6.0);
                    ui.label(
                        RichText::new(format!("Version {}", env!("CARGO_PKG_VERSION"))).small(),
                    );
                    ui.add_space(14.0);
                    ui.label("Audio-reactive marquee for macOS.");
                    ui.add_space(14.0);
                    ui.label(RichText::new("Daniel Siegel").strong());
                    ui.add_space(4.0);
                    ui.label(RichText::new("MIT License").small());
                    ui.add_space(10.0);
                    ui.hyperlink_to(
                        "github.com/danielesiegel/scenic-motion-engine-mac",
                        "https://github.com/danielesiegel/scenic-motion-engine-mac",
                    );
                });
                ui.add_space(14.0);
                ui.separator();
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    ui.with_layout(
                        egui::Layout::right_to_left(egui::Align::Center),
                        |ui| {
                            if ui.button("Close").clicked() {
                                self.visible = false;
                            }
                        },
                    );
                });
            });

        // Window's own close (X) button also dismisses.
        if !open {
            self.visible = false;
        }

        // Esc closes.
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.visible = false;
        }
    }
}
