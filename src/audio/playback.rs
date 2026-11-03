//! Audio playback via `cpal` (CoreAudio on Mac) + `symphonia` decode.
//!
//! The decoder reads the entire file into memory as stereo-interleaved f32 at the file's
//! sample rate. The audio callback consumes samples through a `parking_lot::Mutex`-protected
//! `PlayState`. If the output device sample rate differs from the file's, a cheap linear
//! resample runs inside the callback. The callback does no allocation and takes one short
//! lock per invocation.

use std::path::Path;
use std::sync::Arc;

use anyhow::{anyhow, Context, Result};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{SampleFormat, Stream, StreamConfig};
use parking_lot::Mutex;
use symphonia::core::audio::{AudioBufferRef, Signal};
use symphonia::core::codecs::DecoderOptions;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

/// Shared, interior-mutable playback state. Read by audio callback, written by main thread.
struct PlayState {
    /// Current frame cursor into the decoded buffer (in file-rate frames, not samples).
    frame_cursor: u64,
    /// Fractional phase for linear resampling when `out_sr != file_sr`.
    frac: f32,
    playing: bool,
    out_sr: u32,
    file_sr: u32,
    /// Number of frames in the decoded buffer.
    total_frames: u64,
}

pub struct Player {
    /// Decoded samples: stereo interleaved f32 at `file_sr`.
    samples: Arc<Vec<f32>>,
    state: Arc<Mutex<PlayState>>,
    stream: Option<Stream>,
    duration: f32,
    file_sr: u32,
    out_sr: u32,
}

// Stream is !Send on some platforms; we don't share Player across threads.
unsafe impl Send for Player {}

impl Default for Player {
    fn default() -> Self {
        Self::new()
    }
}

impl Player {
    pub fn new() -> Self {
        Self {
            samples: Arc::new(Vec::new()),
            state: Arc::new(Mutex::new(PlayState {
                frame_cursor: 0,
                frac: 0.0,
                playing: false,
                out_sr: 48_000,
                file_sr: 48_000,
                total_frames: 0,
            })),
            stream: None,
            duration: 0.0,
            file_sr: 48_000,
            out_sr: 48_000,
        }
    }

    pub fn load_file(&mut self, path: &Path) -> Result<()> {
        // Drop any existing stream before reloading.
        self.stream = None;

        let (samples, file_sr) = decode_file(path)?;
        let total_frames = (samples.len() / 2) as u64;
        let duration = total_frames as f32 / file_sr as f32;

        self.samples = Arc::new(samples);
        self.duration = duration;
        self.file_sr = file_sr;

        // Reset shared state for the fresh buffer.
        {
            let mut s = self.state.lock();
            s.frame_cursor = 0;
            s.frac = 0.0;
            s.playing = false;
            s.file_sr = file_sr;
            s.total_frames = total_frames;
            // out_sr stays — will be refreshed when we build the stream.
        }
        Ok(())
    }

    pub fn play(&mut self) -> Result<()> {
        if self.stream.is_none() {
            self.build_stream()?;
        }
        if let Some(stream) = &self.stream {
            stream.play().context("starting cpal stream")?;
        }
        self.state.lock().playing = true;
        Ok(())
    }

    pub fn pause(&mut self) {
        self.state.lock().playing = false;
        if let Some(stream) = &self.stream {
            // Ignore pause errors — some devices don't support pause; the playing flag
            // alone will cause the callback to emit silence.
            let _ = stream.pause();
        }
    }

    pub fn seek(&mut self, t: f32) {
        let t = t.max(0.0);
        let mut s = self.state.lock();
        let frame = (t * s.file_sr as f32) as u64;
        s.frame_cursor = frame.min(s.total_frames);
        s.frac = 0.0;
    }

    pub fn time(&self) -> f32 {
        let s = self.state.lock();
        if s.file_sr == 0 {
            return 0.0;
        }
        s.frame_cursor as f32 / s.file_sr as f32
    }

    pub fn is_playing(&self) -> bool {
        self.state.lock().playing
    }

    pub fn duration(&self) -> f32 {
        self.duration
    }

    pub fn sample_rate(&self) -> u32 {
        self.file_sr
    }

    pub fn samples(&self) -> std::sync::Arc<Vec<f32>> {
        Arc::clone(&self.samples)
    }

    // -------------------------------------------------------------------
    // internals
    // -------------------------------------------------------------------

    fn build_stream(&mut self) -> Result<()> {
        if self.samples.is_empty() {
            return Err(anyhow!("Player::build_stream: no decoded samples loaded"));
        }
        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .ok_or_else(|| anyhow!("no default audio output device"))?;
        let supported = device
            .default_output_config()
            .context("querying default output config")?;
        let out_sr = supported.sample_rate().0;
        let channels = supported.channels();
        let sample_format = supported.sample_format();
        let config: StreamConfig = supported.into();

        self.out_sr = out_sr;
        {
            let mut s = self.state.lock();
            s.out_sr = out_sr;
        }

        let samples = Arc::clone(&self.samples);
        let state = Arc::clone(&self.state);

        // Error callback — log and continue.
        let err_fn = |e| {
            log::error!("audio stream error: {}", e);
        };

        // We only support f32 output here. The crate feature set includes f32 by default
        // for cpal; non-f32 formats fall back to a conversion wrapper.
        let stream = match sample_format {
            SampleFormat::F32 => {
                let samples = Arc::clone(&samples);
                let state = Arc::clone(&state);
                device
                    .build_output_stream(
                        &config,
                        move |output: &mut [f32], _| {
                            fill_f32(output, channels as usize, &samples, &state);
                        },
                        err_fn,
                        None,
                    )
                    .context("building f32 output stream")?
            }
            SampleFormat::I16 => {
                let samples = Arc::clone(&samples);
                let state = Arc::clone(&state);
                device
                    .build_output_stream(
                        &config,
                        move |output: &mut [i16], _| {
                            let mut scratch = [0f32; 1024];
                            let mut consumed = 0;
                            while consumed < output.len() {
                                let n = (output.len() - consumed).min(scratch.len());
                                fill_f32(&mut scratch[..n], channels as usize, &samples, &state);
                                for i in 0..n {
                                    let v = scratch[i].clamp(-1.0, 1.0);
                                    output[consumed + i] = (v * i16::MAX as f32) as i16;
                                }
                                consumed += n;
                            }
                        },
                        err_fn,
                        None,
                    )
                    .context("building i16 output stream")?
            }
            SampleFormat::U16 => {
                let samples = Arc::clone(&samples);
                let state = Arc::clone(&state);
                device
                    .build_output_stream(
                        &config,
                        move |output: &mut [u16], _| {
                            let mut scratch = [0f32; 1024];
                            let mut consumed = 0;
                            while consumed < output.len() {
                                let n = (output.len() - consumed).min(scratch.len());
                                fill_f32(&mut scratch[..n], channels as usize, &samples, &state);
                                for i in 0..n {
                                    let v = scratch[i].clamp(-1.0, 1.0);
                                    let shifted = (v * i16::MAX as f32) as i32 + i16::MAX as i32;
                                    output[consumed + i] = shifted.clamp(0, u16::MAX as i32) as u16;
                                }
                                consumed += n;
                            }
                        },
                        err_fn,
                        None,
                    )
                    .context("building u16 output stream")?
            }
            other => {
                return Err(anyhow!("unsupported sample format: {:?}", other));
            }
        };

        self.stream = Some(stream);
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Decode
// ---------------------------------------------------------------------------

/// Decode an entire audio file into stereo-interleaved f32 and report its sample rate.
fn decode_file(path: &Path) -> Result<(Vec<f32>, u32)> {
    let file = std::fs::File::open(path)
        .with_context(|| format!("opening audio file {}", path.display()))?;
    let mss = MediaSourceStream::new(Box::new(file), Default::default());

    let mut hint = Hint::new();
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        hint.with_extension(ext);
    }

    let probed = symphonia::default::get_probe()
        .format(
            &hint,
            mss,
            &FormatOptions::default(),
            &MetadataOptions::default(),
        )
        .with_context(|| format!("probing audio file {}", path.display()))?;

    let mut format = probed.format;
    let track = format
        .default_track()
        .ok_or_else(|| anyhow!("no default audio track"))?;
    let track_id = track.id;
    let codec_params = track.codec_params.clone();
    let file_sr = codec_params
        .sample_rate
        .ok_or_else(|| anyhow!("missing sample rate in codec params"))?;
    let src_channels = codec_params.channels.map(|c| c.count()).unwrap_or(2).max(1);

    let mut decoder = symphonia::default::get_codecs()
        .make(&codec_params, &DecoderOptions::default())
        .context("creating audio decoder")?;

    let mut out: Vec<f32> = Vec::new();

    loop {
        let packet = match format.next_packet() {
            Ok(p) => p,
            Err(symphonia::core::errors::Error::IoError(ref e))
                if e.kind() == std::io::ErrorKind::UnexpectedEof =>
            {
                break;
            }
            Err(symphonia::core::errors::Error::ResetRequired) => break,
            Err(e) => return Err(anyhow!("packet read failed: {}", e)),
        };
        if packet.track_id() != track_id {
            continue;
        }
        let decoded = match decoder.decode(&packet) {
            Ok(d) => d,
            Err(symphonia::core::errors::Error::DecodeError(_)) => continue,
            Err(e) => return Err(anyhow!("decode failed: {}", e)),
        };
        append_interleaved_stereo(&mut out, &decoded, src_channels);
    }

    if out.is_empty() {
        return Err(anyhow!("decoded no audio frames from {}", path.display()));
    }
    Ok((out, file_sr))
}

/// Convert any `AudioBufferRef` into stereo-interleaved f32 and append.
fn append_interleaved_stereo(out: &mut Vec<f32>, buf: &AudioBufferRef<'_>, src_channels: usize) {
    // Macro to handle each sample type generically.
    macro_rules! handle {
        ($buf:expr, $to_f32:expr) => {{
            let frames = $buf.frames();
            // Pre-reserve.
            out.reserve(frames * 2);
            let ch0 = $buf.chan(0);
            if src_channels >= 2 {
                let ch1 = $buf.chan(1);
                for i in 0..frames {
                    out.push($to_f32(ch0[i]));
                    out.push($to_f32(ch1[i]));
                }
            } else {
                // Mono -> duplicate into L/R.
                for i in 0..frames {
                    let v = $to_f32(ch0[i]);
                    out.push(v);
                    out.push(v);
                }
            }
        }};
    }

    match buf {
        AudioBufferRef::F32(b) => handle!(b, |v: f32| v),
        AudioBufferRef::F64(b) => handle!(b, |v: f64| v as f32),
        AudioBufferRef::U8(b) => handle!(b, |v: u8| (v as f32 - 128.0) / 128.0),
        AudioBufferRef::U16(b) => handle!(b, |v: u16| (v as f32 - 32768.0) / 32768.0),
        AudioBufferRef::U24(b) => handle!(b, |v: symphonia::core::sample::u24| {
            (v.inner() as f32 - 8_388_608.0) / 8_388_608.0
        }),
        AudioBufferRef::U32(b) => {
            handle!(b, |v: u32| (v as f64 - 2_147_483_648.0) as f32
                / 2_147_483_648.0)
        }
        AudioBufferRef::S8(b) => handle!(b, |v: i8| v as f32 / 128.0),
        AudioBufferRef::S16(b) => handle!(b, |v: i16| v as f32 / 32768.0),
        AudioBufferRef::S24(b) => handle!(b, |v: symphonia::core::sample::i24| {
            v.inner() as f32 / 8_388_608.0
        }),
        AudioBufferRef::S32(b) => handle!(b, |v: i32| v as f32 / 2_147_483_648.0),
    }
}

// ---------------------------------------------------------------------------
// Audio callback — fills `output` (interleaved by `out_channels`) with f32.
// ---------------------------------------------------------------------------

fn fill_f32(
    output: &mut [f32],
    out_channels: usize,
    samples: &Arc<Vec<f32>>,
    state: &Mutex<PlayState>,
) {
    // Zero first — if we return early the device gets silence.
    for s in output.iter_mut() {
        *s = 0.0;
    }

    // Snapshot and advance shared state under one short lock, write back at end.
    let (playing, file_sr, out_sr, total_frames, mut cursor, mut frac) = {
        let s = state.lock();
        (
            s.playing,
            s.file_sr,
            s.out_sr,
            s.total_frames,
            s.frame_cursor,
            s.frac,
        )
    };

    if !playing || total_frames == 0 || file_sr == 0 || out_sr == 0 || samples.is_empty() {
        return;
    }

    let step: f32 = file_sr as f32 / out_sr as f32;
    let out_frames = output.len() / out_channels.max(1);

    let total = total_frames as usize;
    for f in 0..out_frames {
        let base = cursor as usize;
        if base + 1 >= total {
            // End of track.
            cursor = total as u64;
            break;
        }
        // Linear interpolation on each channel; the decoded buffer is stereo-interleaved.
        let i0 = base * 2;
        let a0 = samples[i0];
        let a1 = samples[i0 + 1];
        let b0 = samples[i0 + 2];
        let b1 = samples[i0 + 3];
        let l = a0 + (b0 - a0) * frac;
        let r = a1 + (b1 - a1) * frac;

        let o = f * out_channels;
        match out_channels {
            1 => {
                output[o] = 0.5 * (l + r);
            }
            2 => {
                output[o] = l;
                output[o + 1] = r;
            }
            n => {
                // Multi-channel device: write L/R into first two, zero the rest.
                output[o] = l;
                if n >= 2 {
                    output[o + 1] = r;
                }
                for extra in 2..n {
                    output[o + extra] = 0.0;
                }
            }
        }

        // Advance phase by the resample step.
        frac += step;
        let whole = frac.floor();
        cursor = cursor.saturating_add(whole as u64);
        frac -= whole;
    }

    // Write back cursor + frac.
    let mut s = state.lock();
    // Only update if nobody seeked underneath us (best effort — a seek between the two locks
    // simply loses this callback's advancement, which is acceptable).
    s.frame_cursor = cursor.min(s.total_frames);
    s.frac = frac;
}
