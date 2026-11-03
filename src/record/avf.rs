//! Native AVFoundation H.264 encoder — SCAFFOLD / STUB.
//!
//! This module is the placeholder for an in-process H.264 encoder on macOS that
//! would replace the `ffmpeg` subprocess used by [`crate::record::mp4::FfmpegRecorder`].
//!
//! # Target architecture (not yet implemented)
//!
//! The eventual implementation will drive the native stack directly:
//!
//! - `AVAssetWriter` + `AVAssetWriterInput` configured for `AVVideoCodecTypeH264`,
//!   writing to an `.mp4` container (`AVFileTypeMPEG4`).
//! - `AVAssetWriterInputPixelBufferAdaptor` backed by a `CVPixelBufferPool` of
//!   `kCVPixelFormatType_32BGRA` buffers at `(width, height)`.
//! - For each incoming RGBA frame: lock a pooled `CVPixelBuffer`, swizzle
//!   RGBA → BGRA into its base address (respecting `CVPixelBufferGetBytesPerRow`),
//!   append it to the adaptor with a monotonically increasing
//!   `CMTime { value: frame_index, timescale: fps }` presentation timestamp.
//! - On `finish()`: `markAsFinished`, `finishWritingWithCompletionHandler`, block
//!   on a semaphore for completion, surface writer status/error.
//!
//! Required bindings (to be added to `Cargo.toml` when implementing):
//!
//! ```toml
//! objc2              = "0.5"
//! objc2-foundation   = "0.2"
//! objc2-av-foundation = "0.2"
//! objc2-core-media   = "0.2"
//! objc2-core-video   = "0.2"
//! ```
//!
//! # Current status
//!
//! Every method returns an error pointing at the tracking issue. The surface area
//! is intentionally identical to what the real encoder will expose, so the
//! call-site wiring in [`crate::record`] can land ahead of the implementation.
//!
//! Tracking: <https://github.com/danielesiegel/scenic-motion-engine-mac/issues/18>

use anyhow::{bail, Result};
use std::path::{Path, PathBuf};

/// In-process AVFoundation H.264 encoder.
///
/// Not yet implemented — see module docs.
#[derive(Debug)]
pub struct AvfEncoder {
    output: PathBuf,
    width: u32,
    height: u32,
    fps: u32,
}

impl AvfEncoder {
    /// Begin a new encode session writing to `output` (expected `.mp4`).
    ///
    /// Currently returns `Err`; see module-level docs for the planned
    /// `AVAssetWriter` + `CVPixelBufferPool` wiring.
    pub fn start(output: &Path, width: u32, height: u32, fps: u32) -> Result<Self> {
        let _ = Self {
            output: output.to_path_buf(),
            width,
            height,
            fps,
        };
        bail!(
            "AVFoundation encoder not implemented yet — see issue #18 \
             (https://github.com/danielesiegel/scenic-motion-engine-mac/issues/18)"
        )
    }

    /// Append one RGBA8 frame. `rgba.len()` must equal `width * height * 4`.
    pub fn push_frame(&mut self, _rgba: &[u8]) -> Result<()> {
        bail!(
            "AVFoundation encoder not implemented yet — see issue #18 \
             (https://github.com/danielesiegel/scenic-motion-engine-mac/issues/18)"
        )
    }

    /// Flush, finalize the MP4 container, and release AVFoundation resources.
    pub fn finish(self) -> Result<()> {
        bail!(
            "AVFoundation encoder not implemented yet — see issue #18 \
             (https://github.com/danielesiegel/scenic-motion-engine-mac/issues/18)"
        )
    }

    /// Destination path captured at `start()`.
    pub fn output(&self) -> &Path {
        &self.output
    }

    /// Frame dimensions captured at `start()`.
    pub fn dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    /// Frame rate captured at `start()`.
    pub fn fps(&self) -> u32 {
        self.fps
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn start_returns_not_implemented_error() {
        let path = PathBuf::from("/tmp/scenic-avf-stub.mp4");
        let err = AvfEncoder::start(&path, 1920, 1080, 60).unwrap_err();
        let msg = format!("{err}");
        assert!(msg.contains("not implemented"), "unexpected error: {msg}");
        assert!(msg.contains("#18"), "unexpected error: {msg}");
    }
}
