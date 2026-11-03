//! Config persistence — serialize the live Config to JSON on disk and reload it.
//!
//! Design notes:
//! - Writes are atomic (write to `<path>.tmp`, fsync, rename over target).
//! - Parent directories are created on demand.
//! - `load_or_default` never panics: on any error it logs a warning and returns
//!   `defaults::default_config()`, so corrupt files can't brick the app.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use log::warn;

use crate::config::defaults::default_config;
use crate::types::Config;

/// Application name used to scope the config directory.
const APP_DIR_NAME: &str = "scenic-motion-engine";
/// Default filename stored under the config dir.
const CONFIG_FILE_NAME: &str = "config.json";

/// Serialize `config` to pretty JSON and write atomically.
///
/// The write goes to `<path>.tmp` first, is flushed+synced, then renamed over
/// the final `path`. Parent dirs are created recursively if missing.
pub fn save_config(config: &Config, path: &Path) -> Result<()> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)
                .with_context(|| format!("creating parent dir for {}", path.display()))?;
        }
    }

    let json = serde_json::to_string_pretty(config).context("serializing Config to JSON")?;

    let tmp_path = path.with_extension("json.tmp");

    {
        let mut tmp = fs::File::create(&tmp_path)
            .with_context(|| format!("creating temp file {}", tmp_path.display()))?;
        tmp.write_all(json.as_bytes())
            .with_context(|| format!("writing temp file {}", tmp_path.display()))?;
        tmp.flush()
            .with_context(|| format!("flushing temp file {}", tmp_path.display()))?;
        tmp.sync_all()
            .with_context(|| format!("fsyncing temp file {}", tmp_path.display()))?;
    }

    fs::rename(&tmp_path, path)
        .with_context(|| format!("renaming {} -> {}", tmp_path.display(), path.display()))?;

    Ok(())
}

/// Read JSON at `path` and deserialize into `Config`.
pub fn load_config(path: &Path) -> Result<Config> {
    let bytes =
        fs::read(path).with_context(|| format!("reading config file {}", path.display()))?;
    let config: Config = serde_json::from_slice(&bytes)
        .with_context(|| format!("deserializing config file {}", path.display()))?;
    Ok(config)
}

/// Return the default on-disk location:
/// `~/Library/Application Support/scenic-motion-engine/config.json` on macOS.
pub fn default_config_path() -> Result<PathBuf> {
    let dir = dirs::config_dir()
        .context("locating platform config dir (dirs::config_dir returned None)")?;
    Ok(dir.join(APP_DIR_NAME).join(CONFIG_FILE_NAME))
}

/// Attempt to load the config from the default path. On any failure (missing
/// file, bad JSON, filesystem error) log a warning and return `default_config()`.
/// Never panics.
pub fn load_or_default() -> Config {
    let path = match default_config_path() {
        Ok(p) => p,
        Err(e) => {
            warn!("could not resolve default config path, using built-in defaults: {e:#}");
            return default_config();
        }
    };

    if !path.exists() {
        return default_config();
    }

    match load_config(&path) {
        Ok(cfg) => cfg,
        Err(e) => {
            warn!(
                "failed to load config from {}, falling back to defaults: {e:#}",
                path.display()
            );
            default_config()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_default_config() {
        let cfg = default_config();

        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("config.json");

        save_config(&cfg, &path).expect("save");
        assert!(path.exists(), "config file should exist after save");

        let loaded = load_config(&path).expect("load");

        // Scalar round-trip assertions on representative fields.
        assert_eq!(loaded.feel.duration_scale, cfg.feel.duration_scale);
        assert_eq!(loaded.canvas.width, cfg.canvas.width);
        assert_eq!(loaded.canvas.height, cfg.canvas.height);
        assert_eq!(loaded.tickers.len(), cfg.tickers.len());
        assert_eq!(loaded.bindings.len(), cfg.bindings.len());
    }

    #[test]
    fn load_or_default_never_panics_on_missing() {
        // We can't safely assert exact path behaviour across users, but
        // load_or_default must at minimum return a usable Config without
        // panicking in any environment.
        let cfg = load_or_default();
        assert!(cfg.canvas.width > 0);
    }
}
