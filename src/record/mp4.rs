//! Frame-by-frame MP4 export by piping raw RGBA frames to `ffmpeg` on stdin.
//!
//! Contract:
//! - `FfmpegRecorder::start(path, width, height, fps, audio_in, start_s, dur_s)` spawns
//!   `ffmpeg -f rawvideo -pix_fmt rgba -s WxH -r FPS -i - [-ss start -to end -i audio.wav] \
//!    -c:v libx264 -crf 16 -pix_fmt yuv420p -movflags +faststart -shortest OUT`.
//! - `push_frame(rgba)` writes one WxH*4 byte frame to the child's stdin.
//! - `finish()` closes stdin and waits for ffmpeg.
//!
//! The record-agent fills in error paths, backpressure, and progress reporting.

use anyhow::{Context, Result};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};

pub struct FfmpegRecorder {
    pub child: Child,
    pub output: PathBuf,
}

impl FfmpegRecorder {
    pub fn start(
        output: &Path,
        width: u32,
        height: u32,
        fps: u32,
        audio_in: Option<&Path>,
        start_s: f32,
        dur_s: f32,
    ) -> Result<Self> {
        let mut cmd = Command::new("ffmpeg");
        cmd.arg("-y")
            .arg("-f")
            .arg("rawvideo")
            .arg("-pix_fmt")
            .arg("rgba")
            .arg("-s")
            .arg(format!("{}x{}", width, height))
            .arg("-r")
            .arg(format!("{}", fps))
            .arg("-i")
            .arg("-");
        if let Some(audio) = audio_in {
            cmd.arg("-ss")
                .arg(format!("{}", start_s))
                .arg("-to")
                .arg(format!("{}", start_s + dur_s))
                .arg("-i")
                .arg(audio);
        }
        cmd.arg("-c:v")
            .arg("libx264")
            .arg("-crf")
            .arg("16")
            .arg("-pix_fmt")
            .arg("yuv420p")
            .arg("-movflags")
            .arg("+faststart");
        if audio_in.is_some() {
            cmd.arg("-c:a")
                .arg("aac")
                .arg("-b:a")
                .arg("192k")
                .arg("-shortest");
        }
        cmd.arg(output);
        cmd.stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::piped());
        let child = cmd.spawn().context("spawn ffmpeg")?;
        Ok(Self {
            child,
            output: output.to_path_buf(),
        })
    }

    pub fn push_frame(&mut self, rgba: &[u8]) -> Result<()> {
        let stdin = self.child.stdin.as_mut().context("ffmpeg stdin closed")?;
        stdin.write_all(rgba).context("write frame")?;
        Ok(())
    }

    pub fn finish(mut self) -> Result<()> {
        drop(self.child.stdin.take());
        let status = self.child.wait().context("wait ffmpeg")?;
        if !status.success() {
            anyhow::bail!("ffmpeg exited with status {:?}", status);
        }
        Ok(())
    }
}
