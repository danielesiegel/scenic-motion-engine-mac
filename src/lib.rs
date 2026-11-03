//! scenic-motion-engine — native Mac port of the TypeScript prototype.
//!
//! The engine is a **pure function of time**: given a Config, an EventSource, and a UnitLayout,
//! `engine::evaluate(t, …)` returns a FrameState with no mutable state. Scrubbing, looping, and
//! offline rendering are frame-identical to live playback.
//!
//! Module boundaries:
//! - `types`   — all public shared types; the contract between modules
//! - `config`  — default Config construction
//! - `audio`   — EventSource trait + JSON loader + cpal/symphonia playback
//! - `text`    — font loading + scenicsynth unit layout (pre-shaped glyph stride)
//! - `motion`  — easings, path registry, ticker evaluation, binding resolution
//! - `engine`  — the pure `evaluate(t)` composer
//! - `render`  — wgpu text renderer (glyphon-backed) that consumes FrameState
//! - `gui`     — egui control panel (one-to-one with the TS Tweakpane)
//! - `record`  — frame-by-frame MP4 export via ffmpeg subprocess
//! - `app`     — top-level App struct implementing eframe::App

pub mod types;
pub mod config;
pub mod audio;
pub mod text;
pub mod motion;
pub mod engine;
pub mod render;
pub mod gui;
pub mod record;
pub mod app;
