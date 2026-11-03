//! Small waveform minimap drawn as thin vertical lines across the bottom of the stage,
//! with a draggable playhead. Gives users spatial awareness of their position in the track.
//!
//! Peak cache strategy: on first paint (and whenever the audio duration or widget pixel-width
//! changes meaningfully) we bucket the stereo-interleaved f32 sample buffer into `w` bins
//! across the available width, computing the max-abs sample value per bin. The cache is a
//! `Vec<f32>` of length `w` reused frame-to-frame. The compute is capped: for very long files
//! we subsample within each bucket so total work stays O(width_px * K) with a small K.

use std::sync::Arc;

use egui::{Sense, Stroke, Vec2};

/// Threshold for considering `width_px` "significantly changed" (in pixels).
/// Avoids rebuilding peaks on every sub-pixel layout shuffle.
const WIDTH_RESIZE_THRESHOLD: u32 = 4;

/// Target waveform strip height (the ui() allocation is this + a few px of padding).
const STRIP_HEIGHT: f32 = 40.0;

/// Max frames to inspect per output pixel when the buffer is huge. Keeps the one-time
/// peak-compute bounded even for very long tracks.
const MAX_FRAMES_PER_PIXEL: usize = 4096;

#[derive(Default)]
pub struct WaveformStrip {
    /// One peak per pixel of width (max-abs value in [0,1]).
    cached_peaks: Vec<f32>,
    cached_duration: f32,
    cached_width_px: u32,
    /// Length of the samples buffer we built the cache against — used to invalidate when
    /// a new file loads with the same duration but different sample rate / length.
    cached_samples_len: usize,
}

impl WaveformStrip {
    pub fn ui(&mut self, ui: &mut egui::Ui, player: &mut crate::audio::Player) {
        let avail_w = ui.available_width().max(1.0);
        let (rect, response) =
            ui.allocate_exact_size(Vec2::new(avail_w, STRIP_HEIGHT), Sense::click_and_drag());
        let painter = ui.painter_at(rect);

        let visuals = ui.visuals();
        let bg = visuals.widgets.inactive.bg_fill;
        let line_color = visuals.widgets.inactive.fg_stroke.color;
        let playhead_color = visuals.selection.bg_fill;

        painter.rect_filled(rect, 0.0, bg);

        let samples = player.samples();
        let duration = player.duration();

        if samples.is_empty() || duration <= 0.0 {
            painter.text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                "no audio loaded",
                egui::FontId::proportional(12.0),
                visuals.weak_text_color(),
            );
            return;
        }

        let width_px = rect.width().max(1.0) as u32;
        self.ensure_cache(&samples, duration, width_px);

        let center_y = rect.center().y;
        let half_h = (STRIP_HEIGHT * 0.5) - 2.0;
        let left = rect.left();
        let stroke = Stroke::new(1.0, line_color);

        for (i, &peak) in self.cached_peaks.iter().enumerate() {
            let x = left + i as f32 + 0.5;
            let p = peak.clamp(0.0, 1.0);
            if p <= 0.0 {
                continue;
            }
            let dy = p * half_h;
            painter.line_segment(
                [egui::pos2(x, center_y - dy), egui::pos2(x, center_y + dy)],
                stroke,
            );
        }

        // Playhead.
        let t = player.time().clamp(0.0, duration);
        let frac = t / duration;
        let px = left + frac * rect.width();
        painter.line_segment(
            [egui::pos2(px, rect.top()), egui::pos2(px, rect.bottom())],
            Stroke::new(2.0, playhead_color),
        );

        // Interaction: click or drag to seek.
        if response.clicked() || response.dragged() {
            if let Some(pointer) = response.interact_pointer_pos() {
                let rel = (pointer.x - left).clamp(0.0, rect.width());
                let new_t = (rel / rect.width().max(1.0)) * duration;
                player.seek(new_t);
            }
        }
    }

    fn ensure_cache(&mut self, samples: &Arc<Vec<f32>>, duration: f32, width_px: u32) {
        let width_changed = self
            .cached_width_px
            .abs_diff(width_px)
            >= WIDTH_RESIZE_THRESHOLD;
        let dur_changed = (self.cached_duration - duration).abs() > f32::EPSILON;
        let len_changed = self.cached_samples_len != samples.len();

        if !width_changed && !dur_changed && !len_changed && !self.cached_peaks.is_empty() {
            return;
        }

        let w = width_px.max(1) as usize;
        let total_frames = samples.len() / 2;
        if total_frames == 0 || w == 0 {
            self.cached_peaks = vec![0.0; w];
            self.cached_duration = duration;
            self.cached_width_px = width_px;
            self.cached_samples_len = samples.len();
            return;
        }

        let mut peaks = vec![0.0f32; w];
        // Bucket size (frames per pixel), at least 1.
        let frames_per_bucket = total_frames.div_ceil(w).max(1);
        // If a bucket would be huge, subsample within it: step so we visit at most
        // MAX_FRAMES_PER_PIXEL samples per bucket.
        let step = frames_per_bucket.div_ceil(MAX_FRAMES_PER_PIXEL).max(1);

        for (i, slot) in peaks.iter_mut().enumerate() {
            let start = i * frames_per_bucket;
            if start >= total_frames {
                break;
            }
            let end = (start + frames_per_bucket).min(total_frames);
            let mut peak: f32 = 0.0;
            let mut f = start;
            while f < end {
                let idx = f * 2;
                // Stereo-interleaved; max over both channels.
                let l = samples[idx].abs();
                let r = samples[idx + 1].abs();
                let m = l.max(r);
                if m > peak {
                    peak = m;
                }
                f += step;
            }
            *slot = peak.min(1.0);
        }

        self.cached_peaks = peaks;
        self.cached_duration = duration;
        self.cached_width_px = width_px;
        self.cached_samples_len = samples.len();
    }
}
