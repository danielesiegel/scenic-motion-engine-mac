//! `EventSource` implementations.
//!
//! `JsonEventSource` parses an `AnalysisData` JSON (schema in `crate::types`) and indexes its
//! per-band transient events for fast lookup by time and threshold. `SyntheticEventSource` is a
//! stable stand-in that emits deterministic kicks/snares/hats/mid events at 120 BPM, used when
//! no analysis is loaded.
//!
//! Mirrors the semantics of the TypeScript source in `scenicsynths/src/audio/eventSource.ts`.

use crate::types::*;
use anyhow::{Context, Result};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

// ---------------------------------------------------------------------------
// helpers
// ---------------------------------------------------------------------------

/// Number of events with `event.t <= t` (rightmost insertion point by time).
fn count_le_by_t(events: &[TransientEvent], t: f32) -> usize {
    // Binary search for the first index where events[i].t > t.
    let (mut lo, mut hi) = (0usize, events.len());
    while lo < hi {
        let mid = lo + (hi - lo) / 2;
        if events[mid].t <= t {
            lo = mid + 1;
        } else {
            hi = mid;
        }
    }
    lo
}

/// Number of values with `v <= t` in a sorted `&[f32]`.
fn count_le_f32(values: &[f32], t: f32) -> usize {
    let (mut lo, mut hi) = (0usize, values.len());
    while lo < hi {
        let mid = lo + (hi - lo) / 2;
        if values[mid] <= t {
            lo = mid + 1;
        } else {
            hi = mid;
        }
    }
    lo
}

fn band_from_key(key: &str) -> Option<Band> {
    match key {
        "kick" => Some(Band::Kick),
        "snare" => Some(Band::Snare),
        "mid" => Some(Band::Mid),
        "hats" => Some(Band::Hats),
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// JsonEventSource
// ---------------------------------------------------------------------------

pub struct JsonEventSource {
    bpm: f32,
    duration: f32,
    bands_order: Vec<Band>,
    events: HashMap<Band, Vec<TransientEvent>>,
    beats: Vec<f32>,
    downbeats: Vec<f32>,
    /// Empty slice used to satisfy `get_events` return type when a band is unknown.
    empty: Vec<TransientEvent>,
}

impl JsonEventSource {
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();
        let bytes =
            fs::read(path).with_context(|| format!("reading analysis JSON {}", path.display()))?;
        let data: AnalysisData = serde_json::from_slice(&bytes)
            .with_context(|| format!("parsing analysis JSON {}", path.display()))?;
        Ok(Self::from_analysis(data))
    }

    pub fn from_analysis(data: AnalysisData) -> Self {
        let AnalysisData {
            bpm,
            duration,
            beats,
            downbeats,
            bands,
            ..
        } = data;

        // Index each known band key into the enum map; sort by t; clamp strength to [0, 1].
        let mut events: HashMap<Band, Vec<TransientEvent>> = HashMap::new();
        let mut bands_order: Vec<Band> = Vec::new();
        for (key, mut arr) in bands.into_iter() {
            let Some(band) = band_from_key(&key) else {
                continue;
            };
            arr.sort_by(|a, b| a.t.partial_cmp(&b.t).unwrap_or(std::cmp::Ordering::Equal));
            for e in arr.iter_mut() {
                e.strength = e.strength.clamp(0.0, 1.0);
            }
            events.insert(band, arr);
            bands_order.push(band);
        }

        // Preserve a stable, deterministic order (matches Band::ALL).
        bands_order.sort_by_key(|b| match b {
            Band::Kick => 0,
            Band::Snare => 1,
            Band::Mid => 2,
            Band::Hats => 3,
        });

        let mut beats = beats;
        beats.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let mut downbeats = downbeats;
        downbeats.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

        Self {
            bpm,
            duration,
            bands_order,
            events,
            beats,
            downbeats,
            empty: Vec::new(),
        }
    }
}

impl EventSource for JsonEventSource {
    fn bpm(&self) -> f32 {
        self.bpm
    }
    fn duration(&self) -> f32 {
        self.duration
    }
    fn bands(&self) -> &[Band] {
        &self.bands_order
    }
    fn get_events(&self, band: Band) -> &[TransientEvent] {
        self.events
            .get(&band)
            .map(|v| v.as_slice())
            .unwrap_or(&self.empty)
    }

    fn count_before(&self, band: Band, t: f32, threshold: f32) -> usize {
        let arr = self.get_events(band);
        let upto = count_le_by_t(arr, t);
        if threshold <= 0.0 {
            return upto;
        }
        arr[..upto]
            .iter()
            .filter(|e| e.strength >= threshold)
            .count()
    }

    fn event_at_index(&self, band: Band, index: usize, threshold: f32) -> Option<TransientEvent> {
        let arr = self.get_events(band);
        if threshold <= 0.0 {
            return arr.get(index).copied();
        }
        arr.iter()
            .filter(|e| e.strength >= threshold)
            .nth(index)
            .copied()
    }

    fn last_before(&self, band: Band, t: f32, threshold: f32) -> Option<TransientEvent> {
        let arr = self.get_events(band);
        let upto = count_le_by_t(arr, t);
        for i in (0..upto).rev() {
            if arr[i].strength >= threshold {
                return Some(arr[i]);
            }
        }
        None
    }

    fn beats_before(&self, t: f32) -> usize {
        count_le_f32(&self.beats, t)
    }

    fn downbeats_before(&self, t: f32) -> usize {
        count_le_f32(&self.downbeats, t)
    }
}

// ---------------------------------------------------------------------------
// SyntheticEventSource
// ---------------------------------------------------------------------------

pub struct SyntheticEventSource {
    pub bpm: f32,
    pub duration: f32,
    pub events: HashMap<Band, Vec<TransientEvent>>,
    pub beats: Vec<f32>,
    pub downbeats: Vec<f32>,
    pub bands: Vec<Band>,
    empty: Vec<TransientEvent>,
}

impl Default for SyntheticEventSource {
    fn default() -> Self {
        let bpm: f32 = 120.0;
        let duration: f32 = 600.0;
        let beat_interval: f32 = 60.0 / bpm; // 0.5s at 120 BPM
        let steps: usize = (duration / beat_interval).floor() as usize;

        let mut kick: Vec<TransientEvent> = Vec::with_capacity(steps);
        let mut snare: Vec<TransientEvent> = Vec::with_capacity(steps / 2 + 1);
        let mut hats: Vec<TransientEvent> = Vec::with_capacity(steps * 2);
        let mut mid: Vec<TransientEvent> = Vec::with_capacity(steps / 4 + 1);
        let mut beats: Vec<f32> = Vec::with_capacity(steps);
        let mut downbeats: Vec<f32> = Vec::with_capacity(steps / 4 + 1);

        for i in 0..steps {
            let t = i as f32 * beat_interval;
            kick.push(TransientEvent { t, strength: 1.0 });
            if i % 2 == 1 {
                snare.push(TransientEvent { t, strength: 1.0 });
            }
            // Hats on 8ths.
            hats.push(TransientEvent { t, strength: 0.6 });
            hats.push(TransientEvent {
                t: t + beat_interval / 2.0,
                strength: 0.4,
            });
            // Mid sparse.
            if i % 4 == 0 {
                mid.push(TransientEvent { t, strength: 0.5 });
                downbeats.push(t);
            }
            beats.push(t);
        }

        let mut events: HashMap<Band, Vec<TransientEvent>> = HashMap::new();
        events.insert(Band::Kick, kick);
        events.insert(Band::Snare, snare);
        events.insert(Band::Mid, mid);
        events.insert(Band::Hats, hats);

        Self {
            bpm,
            duration,
            events,
            beats,
            downbeats,
            bands: Band::ALL.to_vec(),
            empty: Vec::new(),
        }
    }
}

impl EventSource for SyntheticEventSource {
    fn bpm(&self) -> f32 {
        self.bpm
    }
    fn duration(&self) -> f32 {
        self.duration
    }
    fn bands(&self) -> &[Band] {
        &self.bands
    }
    fn get_events(&self, band: Band) -> &[TransientEvent] {
        self.events
            .get(&band)
            .map(|v| v.as_slice())
            .unwrap_or(&self.empty)
    }
    fn count_before(&self, band: Band, t: f32, threshold: f32) -> usize {
        let arr = self.get_events(band);
        let upto = count_le_by_t(arr, t);
        if threshold <= 0.0 {
            return upto;
        }
        arr[..upto]
            .iter()
            .filter(|e| e.strength >= threshold)
            .count()
    }
    fn event_at_index(&self, band: Band, index: usize, threshold: f32) -> Option<TransientEvent> {
        let arr = self.get_events(band);
        if threshold <= 0.0 {
            return arr.get(index).copied();
        }
        arr.iter()
            .filter(|e| e.strength >= threshold)
            .nth(index)
            .copied()
    }
    fn last_before(&self, band: Band, t: f32, threshold: f32) -> Option<TransientEvent> {
        let arr = self.get_events(band);
        let upto = count_le_by_t(arr, t);
        for i in (0..upto).rev() {
            if arr[i].strength >= threshold {
                return Some(arr[i]);
            }
        }
        None
    }
    fn beats_before(&self, t: f32) -> usize {
        count_le_f32(&self.beats, t)
    }
    fn downbeats_before(&self, t: f32) -> usize {
        count_le_f32(&self.downbeats, t)
    }
}
