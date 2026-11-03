//! Keyboard-shortcut cheatsheet overlay.
//!
//! A centered modal-ish card that lists every global shortcut the app honors.
//! Toggled by `?` from `app.rs`; dismissed by `?` or Esc.

use egui::{Align2, Color32, Context, Grid, Order, RichText, Window};

/// Overlay state. `visible` is public so the hosting `App` can flip it when it
/// detects the `?` keypress.
#[derive(Default)]
pub struct ShortcutOverlay {
    pub visible: bool,
}

impl ShortcutOverlay {
    pub fn toggle(&mut self) {
        self.visible = !self.visible;
    }

    pub fn ui(&mut self, ctx: &Context) {
        if !self.visible {
            return;
        }

        // Dim the stage behind the card. We paint a translucent black rect over
        // the whole screen in a foreground `Area` so it sits above the central
        // panel but below the window.
        let screen = ctx.screen_rect();
        egui::Area::new(egui::Id::new("shortcuts-backdrop"))
            .fixed_pos(screen.min)
            .order(Order::Foreground)
            .show(ctx, |ui| {
                ui.painter()
                    .rect_filled(screen, 0.0, Color32::from_black_alpha(140));
            });

        let mut open = true;
        Window::new("Keyboard Shortcuts")
            .collapsible(false)
            .resizable(false)
            .movable(false)
            .anchor(Align2::CENTER_CENTER, [0.0, 0.0])
            .default_size([520.0, 480.0])
            .open(&mut open)
            .show(ctx, |ui| {
                ui.add_space(6.0);
                Grid::new("shortcuts-grid")
                    .num_columns(2)
                    .spacing([24.0, 10.0])
                    .striped(false)
                    .show(ui, |ui| {
                        for (key, desc) in SHORTCUTS {
                            key_cell(ui, key);
                            ui.label(*desc);
                            ui.end_row();
                        }
                    });

                ui.add_space(14.0);
                ui.separator();
                ui.add_space(8.0);
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

/// Render a single key cap: a monospace label with a dark background pill.
fn key_cell(ui: &mut egui::Ui, label: &str) {
    let text = RichText::new(label)
        .monospace()
        .background_color(Color32::from_gray(40))
        .color(Color32::WHITE);
    // The RichText background doesn't give us padding; the Frame below adds the
    // 2 px rounded padding the brief asks for.
    egui::Frame::none()
        .fill(Color32::from_gray(40))
        .rounding(4.0)
        .inner_margin(egui::Margin::symmetric(6.0, 2.0))
        .show(ui, |ui| {
            ui.label(text);
        });
}

/// Static table of shortcut rows. Edit here to add/remove entries.
const SHORTCUTS: &[(&str, &str)] = &[
    ("Space", "Play / pause"),
    ("\u{2318}O", "Open audio file"),
    ("\u{2318}R", "Reload analysis JSON"),
    ("\u{2318}.", "Stop (seek to loop start)"),
    ("\u{2190} / \u{2192}", "Seek -5s / +5s (hold Shift for \u{00B1}1s)"),
    ("Home", "Seek to loop start"),
    ("End", "Seek to end"),
    ("L", "Toggle loop"),
    ("H", "Toggle HUD"),
    ("?", "Show this cheatsheet"),
    ("Esc", "Close this cheatsheet"),
];
