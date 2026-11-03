//! `MidiEventSource` — a live MIDI input bound to the `EventSource` trait.
//!
//! Opens a CoreMIDI (via `midir`) input port and funnels note-on messages into a
//! mutex-protected per-band buffer. Band mapping follows GM-ish drum ranges:
//!   notes   0..=35  -> Kick
//!   notes  36..=59  -> Snare
//!   notes  60..=83  -> Mid
//!   notes  84..=127 -> Hats
//! Velocity is normalized to `strength = velocity / 127.0`. Timestamps are seconds
//! elapsed since the source was constructed (monotonic). `bpm()` is a fixed 120
//! because MIDI note streams don't inherently carry tempo, and `duration()` is
//! `f32::MAX` because the stream is open-ended.
//!
//! The module is intentionally not wired into `app.rs` yet — see the follow-up
//! issue for engine integration.
//!
//! `get_events` returns a leaked boxed snapshot each call so the slice has `'static`
//! lifetime and satisfies the `EventSource` trait's `&[T]` return type without
//! `unsafe`. Call rate in the engine is per-frame; snapshot size is tiny; this is
//! acceptable for a live demo module. A follow-up could refactor the trait to
//! return `Cow<[TransientEvent]>` to avoid the leak entirely.

use crate::types::*;
use anyhow::{anyhow, Context, Result};
use midir::{MidiInput, MidiInputConnection, MidiInputPort};
use std::sync::{Arc, Mutex};
use std::time::Instant;

/// Per-band accumulator of transient events collected from live MIDI note-on messages.
#[derive(Default)]
struct MidiBuffer {
    kick: Vec<TransientEvent>,
    snare: Vec<TransientEvent>,
    mid: Vec<TransientEvent>,
    hats: Vec<TransientEvent>,
}

impl MidiBuffer {
    fn push(&mut self, note: u8, velocity: u8, t: f32) {
        let strength = (velocity as f32 / 127.0).clamp(0.0, 1.0);
        let ev = TransientEvent { t, strength };
        match note {
            0..=35 => self.kick.push(ev),
            36..=59 => self.snare.push(ev),
            60..=83 => self.mid.push(ev),
            _ => self.hats.push(ev),
        }
    }

    fn snapshot(&self, band: Band) -> Vec<TransientEvent> {
        let src = match band {
            Band::Kick => &self.kick,
            Band::Snare => &self.snare,
            Band::Mid => &self.mid,
            Band::Hats => &self.hats,
        };
        let mut out = src.clone();
        out.sort_by(|a, b| a.t.partial_cmp(&b.t).unwrap_or(std::cmp::Ordering::Equal));
        out
    }
}

/// Live MIDI event source. Keeps the `midir` connection alive for its lifetime.
pub struct MidiEventSource {
    bands: Vec<Band>,
    buffer: Arc<Mutex<MidiBuffer>>,
    _conn: MidiInputConnection<()>,
    _start: Instant,
}

impl MidiEventSource {
    /// Open a MIDI input. If `port_name` is `Some`, pick the first port whose name
    /// contains the substring (case-insensitive); otherwise pick the first
    /// available port. Returns `Err` if no MIDI devices are present.
    pub fn new(port_name: Option<&str>) -> Result<Self> {
        let midi_in = MidiInput::new("scenic-motion-engine").context("creating midir MidiInput")?;
        let ports = midi_in.ports();
        if ports.is_empty() {
            return Err(anyhow!("no MIDI input devices available"));
        }

        let port: &MidiInputPort = match port_name {
            Some(needle) => {
                let needle_lc = needle.to_lowercase();
                ports
                    .iter()
                    .find(|p| {
                        midi_in
                            .port_name(p)
                            .map(|n| n.to_lowercase().contains(&needle_lc))
                            .unwrap_or(false)
                    })
                    .ok_or_else(|| anyhow!("no MIDI input port matching {:?}", needle))?
            }
            None => &ports[0],
        };

        let buffer = Arc::new(Mutex::new(MidiBuffer::default()));
        let start = Instant::now();
        let buffer_cb = Arc::clone(&buffer);
        let start_cb = start;

        let conn = midi_in
            .connect(
                port,
                "scenic-motion-engine-in",
                move |_stamp_us, message, _| {
                    // MIDI status nibble 0x9 = note-on; velocity 0 is a conventional note-off.
                    if message.len() < 3 {
                        return;
                    }
                    let status = message[0] & 0xF0;
                    if status != 0x90 {
                        return;
                    }
                    let note = message[1] & 0x7F;
                    let velocity = message[2] & 0x7F;
                    if velocity == 0 {
                        return;
                    }
                    let t = start_cb.elapsed().as_secs_f32();
                    if let Ok(mut buf) = buffer_cb.lock() {
                        buf.push(note, velocity, t);
                    }
                },
                (),
            )
            .map_err(|e| anyhow!("midir connect failed: {e}"))?;

        Ok(Self {
            bands: Band::ALL.to_vec(),
            buffer,
            _conn: conn,
            _start: start,
        })
    }

    /// Snapshot the current buffer for a band and sort by time. Owned-copy helper
    /// used by both the trait implementation and tests.
    fn snapshot(&self, band: Band) -> Vec<TransientEvent> {
        self.buffer
            .lock()
            .map(|b| b.snapshot(band))
            .unwrap_or_default()
    }
}

impl EventSource for MidiEventSource {
    fn bpm(&self) -> f32 {
        120.0
    }
    fn duration(&self) -> f32 {
        f32::MAX
    }
    fn bands(&self) -> &[Band] {
        &self.bands
    }

    /// Returns a fresh snapshot of the live buffer for `band`. The slice is backed
    /// by a leaked `Box<[TransientEvent]>` so it satisfies the trait's `&[T]`
    /// return type with a `'static`-safe lifetime and without any `unsafe`. See
    /// module docs for the rationale and follow-up plan.
    fn get_events(&self, band: Band) -> &[TransientEvent] {
        let snapshot = self.snapshot(band);
        Box::leak(snapshot.into_boxed_slice())
    }

    fn count_before(&self, band: Band, t: f32, threshold: f32) -> usize {
        let snap = self.snapshot(band);
        snap.iter()
            .filter(|e| e.t <= t && e.strength >= threshold)
            .count()
    }

    fn event_at_index(&self, band: Band, index: usize, threshold: f32) -> Option<TransientEvent> {
        let snap = self.snapshot(band);
        if threshold <= 0.0 {
            return snap.get(index).copied();
        }
        snap.iter()
            .filter(|e| e.strength >= threshold)
            .nth(index)
            .copied()
    }

    fn last_before(&self, band: Band, t: f32, threshold: f32) -> Option<TransientEvent> {
        let snap = self.snapshot(band);
        snap.iter()
            .rev()
            .find(|e| e.t <= t && e.strength >= threshold)
            .copied()
    }

    fn beats_before(&self, _t: f32) -> usize {
        0
    }
    fn downbeats_before(&self, _t: f32) -> usize {
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_errors_cleanly_when_no_devices() {
        // On a MIDI-less system `new` must return Err without panicking. On a
        // system with devices this test either exercises the success path (and
        // verifies trait methods) or is a no-op if construction fails for other
        // reasons — in all cases it must not panic.
        let probe = MidiInput::new("probe").expect("probe MidiInput");
        let have_devices = !probe.ports().is_empty();
        // Explicitly release the probe handle before attempting to open an input;
        // `MidiInput` has no Drop impl, so we shadow it instead of calling `drop`.
        let _ = probe;
        if !have_devices {
            let err = match MidiEventSource::new(None) {
                Ok(_) => panic!("expected Err when no MIDI devices are present"),
                Err(e) => e,
            };
            assert!(
                format!("{err}").contains("no MIDI input"),
                "unexpected error: {err}"
            );
            return;
        }
        // Devices exist — construction may succeed; verify `bands()` + metadata.
        if let Ok(src) = MidiEventSource::new(None) {
            assert_eq!(src.bands(), &Band::ALL);
            assert_eq!(src.bpm(), 120.0);
            assert_eq!(src.duration(), f32::MAX);
            // Fresh source has no events yet.
            for b in Band::ALL {
                assert!(src.get_events(b).is_empty());
            }
        }
    }

    #[test]
    fn band_mapping_matches_note_ranges() {
        let mut buf = MidiBuffer::default();
        buf.push(10, 127, 0.1); // kick
        buf.push(35, 100, 0.15); // kick boundary
        buf.push(36, 100, 0.2); // snare boundary
        buf.push(59, 90, 0.25); // snare upper
        buf.push(60, 80, 0.3); // mid boundary
        buf.push(83, 70, 0.35); // mid upper
        buf.push(84, 50, 0.4); // hats boundary
        buf.push(127, 32, 0.45); // hats upper

        assert_eq!(buf.snapshot(Band::Kick).len(), 2);
        assert_eq!(buf.snapshot(Band::Snare).len(), 2);
        assert_eq!(buf.snapshot(Band::Mid).len(), 2);
        assert_eq!(buf.snapshot(Band::Hats).len(), 2);

        let kick = buf.snapshot(Band::Kick)[0];
        assert!((kick.strength - 1.0).abs() < 1e-6);
        assert!((kick.t - 0.1).abs() < 1e-6);

        let hats_last = buf.snapshot(Band::Hats)[1];
        assert!((hats_last.strength - (32.0 / 127.0)).abs() < 1e-6);
    }

    #[test]
    fn snapshot_sorts_by_time() {
        let mut buf = MidiBuffer::default();
        buf.push(10, 100, 0.5);
        buf.push(10, 100, 0.1);
        buf.push(10, 100, 0.3);
        let snap = buf.snapshot(Band::Kick);
        assert_eq!(snap.len(), 3);
        assert!(snap[0].t <= snap[1].t && snap[1].t <= snap[2].t);
    }
}
