//! Drag-and-drop file handling for the stage: audio files load into the Player, and `.json`
//! analysis files swap the `EventSource` Arc. While files are hovered over the window, a
//! translucent overlay prompts the user with the accepted extensions.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use egui::{Align2, Color32, FontId, LayerId, Order, Rect};

use crate::audio::{JsonEventSource, Player};
use crate::types::EventSource;

const DROP_HINT: &str = "Drop a .wav, .mp3, .flac, or analysis .json here";
const ANALYSIS_DIR: &str = "analysis";

/// Entry point: inspect any hovered or dropped files on this frame, act on them, and paint a
/// hint overlay while a drag is in progress.
pub fn handle_dropped_files(
    ctx: &egui::Context,
    player: &mut Player,
    source: &mut Arc<dyn EventSource>,
) {
    let (dropped, hovering) =
        ctx.input(|i| (i.raw.dropped_files.clone(), !i.raw.hovered_files.is_empty()));

    if hovering {
        paint_hint_overlay(ctx);
        ctx.request_repaint();
    }

    if dropped.is_empty() {
        return;
    }

    let mut did_work = false;
    for f in dropped.iter() {
        let Some(path) = f.path.clone() else {
            continue;
        };
        match ext_lower(&path).as_deref() {
            Some("wav") | Some("mp3") | Some("flac") => {
                load_audio(&path, player, source);
                did_work = true;
            }
            Some("json") => {
                load_analysis(&path, source);
                did_work = true;
            }
            other => {
                log::warn!(
                    "drag-drop: ignoring unsupported extension {:?} on {}",
                    other,
                    path.display()
                );
            }
        }
    }

    if did_work {
        ctx.request_repaint();
    }
}

fn ext_lower(path: &Path) -> Option<String> {
    path.extension()
        .and_then(|s| s.to_str())
        .map(|s| s.to_ascii_lowercase())
}

fn load_audio(path: &Path, player: &mut Player, source: &mut Arc<dyn EventSource>) {
    match player.load_file(path) {
        Ok(_) => {
            log::info!(
                "drag-drop: loaded audio {} ({:.1}s @ {}Hz)",
                path.display(),
                player.duration(),
                player.sample_rate()
            );
        }
        Err(err) => {
            log::warn!(
                "drag-drop: audio load failed for {}: {}",
                path.display(),
                err
            );
            return;
        }
    }
    // Try sibling analysis JSON at ../analysis/<basename>.json relative to cwd.
    let Some(stem) = path.file_stem().and_then(|s| s.to_str()) else {
        return;
    };
    let analysis_path = PathBuf::from(ANALYSIS_DIR).join(format!("{}.json", stem));
    if analysis_path.is_file() {
        load_analysis(&analysis_path, source);
    } else {
        log::info!(
            "drag-drop: no sibling analysis at {} — keeping current source",
            analysis_path.display()
        );
    }
}

fn load_analysis(path: &Path, source: &mut Arc<dyn EventSource>) {
    match JsonEventSource::load(path) {
        Ok(src) => {
            log::info!(
                "drag-drop: loaded analysis {} ({} bands)",
                path.display(),
                src.bands().len()
            );
            *source = Arc::new(src);
        }
        Err(err) => {
            log::warn!(
                "drag-drop: analysis load failed for {}: {}",
                path.display(),
                err
            );
        }
    }
}

fn paint_hint_overlay(ctx: &egui::Context) {
    let screen = ctx.screen_rect();
    let painter = ctx.layer_painter(LayerId::new(Order::Foreground, egui::Id::new("drop-hint")));

    // Translucent scrim.
    painter.rect_filled(screen, 0.0, Color32::from_black_alpha(140));

    // Central pill with the hint text.
    let text_size = 20.0;
    let pad = egui::vec2(28.0, 18.0);
    let galley = painter.layout_no_wrap(
        DROP_HINT.to_string(),
        FontId::proportional(text_size),
        Color32::WHITE,
    );
    let center = screen.center();
    let pill = Rect::from_center_size(center, galley.size() + pad * 2.0);
    painter.rect_filled(pill, 12.0, Color32::from_rgba_unmultiplied(32, 32, 32, 220));
    painter.rect_stroke(
        pill,
        12.0,
        egui::Stroke::new(1.5_f32, Color32::from_white_alpha(90)),
    );
    painter.text(
        center,
        Align2::CENTER_CENTER,
        DROP_HINT,
        FontId::proportional(text_size),
        Color32::WHITE,
    );
}
