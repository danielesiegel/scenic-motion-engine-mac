//! Ticker helpers. Mirrors `scenic-motion-engine-web/src/motion/tickers.ts`.
//! Mostly used by engine.rs internally; kept here as a module for parity.

use crate::types::TickerConfig;
use std::collections::HashMap;

pub fn resolve_threshold(cfg: &TickerConfig, runtime_thresholds: &HashMap<String, f32>) -> f32 {
    let band_key = cfg.band.as_str();
    let base = cfg.strength_threshold.unwrap_or(0.0);
    let runtime = runtime_thresholds.get(band_key).copied().unwrap_or(0.0);
    base.max(runtime)
}
