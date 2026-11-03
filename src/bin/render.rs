//! `render` — offline MP4 export CLI.
//!
//! ```text
//! cargo run --release --bin render -- \
//!     --audio assets/audio/kafkaesque.wav \
//!     --analysis analysis/kafkaesque.json \
//!     --out renders/out.mp4 \
//!     --start 0 --duration 10 --fps 60 --width 1920 --height 1080
//! ```
//!
//! Only `--analysis` and `--out` are strictly required (an audio track is optional and only
//! participates in the final mux). Omitted analysis falls back to the synthetic 120-BPM source.

use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::Arc;
use std::time::Instant;

use scenic_motion_engine::audio::{JsonEventSource, SyntheticEventSource};
use scenic_motion_engine::config::defaults::default_config;
use scenic_motion_engine::motion::paths::register_default_paths;
use scenic_motion_engine::record::{OfflineRenderOptions, OfflineRenderer};
use scenic_motion_engine::types::EventSource;

const USAGE: &str = "\
scenic-motion-engine offline renderer

USAGE:
    render [--audio PATH] [--analysis PATH] --out PATH
           [--start SECONDS] [--duration SECONDS]
           [--fps N] [--width PX] [--height PX]
           [--fonts DIR]

Defaults: start=0, duration=10, fps=60, width=1920, height=1080, fonts=assets/fonts.
If --analysis is omitted a synthetic 120-BPM source is used.
";

struct Args {
    audio: Option<PathBuf>,
    analysis: Option<PathBuf>,
    out: Option<PathBuf>,
    start: f32,
    duration: f32,
    fps: u32,
    width: u32,
    height: u32,
    fonts: PathBuf,
}

impl Default for Args {
    fn default() -> Self {
        Self {
            audio: None,
            analysis: None,
            out: None,
            start: 0.0,
            duration: 10.0,
            fps: 60,
            width: 1920,
            height: 1080,
            fonts: PathBuf::from("assets/fonts"),
        }
    }
}

fn parse_args() -> Result<Args, String> {
    let mut args = Args::default();
    let mut it = std::env::args().skip(1);
    while let Some(flag) = it.next() {
        let want_value =
            |name: &str, it: &mut std::iter::Skip<std::env::Args>| -> Result<String, String> {
                it.next()
                    .ok_or_else(|| format!("missing value for {}", name))
            };
        match flag.as_str() {
            "-h" | "--help" => {
                println!("{}", USAGE);
                std::process::exit(0);
            }
            "--audio" => args.audio = Some(PathBuf::from(want_value("--audio", &mut it)?)),
            "--analysis" => args.analysis = Some(PathBuf::from(want_value("--analysis", &mut it)?)),
            "--out" | "-o" => args.out = Some(PathBuf::from(want_value("--out", &mut it)?)),
            "--start" => {
                args.start = want_value("--start", &mut it)?
                    .parse()
                    .map_err(|e| format!("--start: {}", e))?
            }
            "--duration" => {
                args.duration = want_value("--duration", &mut it)?
                    .parse()
                    .map_err(|e| format!("--duration: {}", e))?
            }
            "--fps" => {
                args.fps = want_value("--fps", &mut it)?
                    .parse()
                    .map_err(|e| format!("--fps: {}", e))?
            }
            "--width" => {
                args.width = want_value("--width", &mut it)?
                    .parse()
                    .map_err(|e| format!("--width: {}", e))?
            }
            "--height" => {
                args.height = want_value("--height", &mut it)?
                    .parse()
                    .map_err(|e| format!("--height: {}", e))?
            }
            "--fonts" => args.fonts = PathBuf::from(want_value("--fonts", &mut it)?),
            other => return Err(format!("unknown flag: {}", other)),
        }
    }
    if args.out.is_none() {
        return Err("--out is required".to_string());
    }
    Ok(args)
}

fn main() -> ExitCode {
    env_logger::init();
    register_default_paths();

    let args = match parse_args() {
        Ok(a) => a,
        Err(err) => {
            eprintln!("error: {}\n\n{}", err, USAGE);
            return ExitCode::from(2);
        }
    };

    let out = args.out.as_ref().expect("checked");
    if let Some(parent) = out.parent() {
        if !parent.as_os_str().is_empty() {
            if let Err(err) = std::fs::create_dir_all(parent) {
                eprintln!(
                    "error: cannot create output dir {}: {}",
                    parent.display(),
                    err
                );
                return ExitCode::from(1);
            }
        }
    }

    // --- Event source ---
    let source: Arc<dyn EventSource> = match &args.analysis {
        Some(path) => {
            if !path.exists() {
                eprintln!("error: analysis file not found: {}", path.display());
                return ExitCode::from(1);
            }
            match JsonEventSource::load(path) {
                Ok(s) => Arc::new(s),
                Err(err) => {
                    eprintln!("error: load analysis {}: {}", path.display(), err);
                    return ExitCode::from(1);
                }
            }
        }
        None => {
            eprintln!("note: no --analysis provided, using synthetic 120-BPM source");
            Arc::new(SyntheticEventSource::default())
        }
    };

    if let Some(audio) = &args.audio {
        if !audio.exists() {
            eprintln!("error: audio file not found: {}", audio.display());
            return ExitCode::from(1);
        }
    }

    // --- Config + Renderer ---
    let config = default_config();
    let mut renderer = match OfflineRenderer::new(config, source, &args.fonts) {
        Ok(r) => r,
        Err(err) => {
            eprintln!("error: offline renderer init: {:#}", err);
            return ExitCode::from(1);
        }
    };

    let opts = OfflineRenderOptions {
        start_at: args.start,
        duration: args.duration,
        fps: args.fps,
        width: args.width,
        height: args.height,
        output: out.clone(),
        audio_in: args.audio.clone(),
    };

    eprintln!(
        "rendering {:.3}s @ {}fps ({}x{}) -> {}",
        opts.duration,
        opts.fps,
        opts.width,
        opts.height,
        opts.output.display()
    );

    let t0 = Instant::now();
    let mut last_log_frame: u32 = 0;
    let result = renderer.render(&opts, |done, total| {
        // Progress every ~1% or every 60 frames, whichever is coarser.
        let step = (total / 100).max(60);
        if done - last_log_frame >= step || done == total {
            eprintln!(
                "  frame {}/{} ({:.1}%) elapsed {:.1}s",
                done,
                total,
                100.0 * done as f32 / total as f32,
                t0.elapsed().as_secs_f32()
            );
            last_log_frame = done;
        }
    });
    match result {
        Ok(()) => {
            eprintln!(
                "done in {:.2}s -> {}",
                t0.elapsed().as_secs_f32(),
                out.display()
            );
            ExitCode::SUCCESS
        }
        Err(err) => {
            eprintln!("error: render failed: {:#}", err);
            ExitCode::from(1)
        }
    }
}
