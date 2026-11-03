//! Font loading via `cosmic-text::FontSystem`. The text-agent fills in `FontLoader::new_with_dir`
//! to scan `assets/fonts/` and expose a `FontSystem` the layout module uses.
//!
//! Contract:
//! - `FontLoader::new_with_dir(dir)` initializes a FontSystem from a font database rooted at `dir`.
//! - `font_system()` returns a mutable reference for shaping.
//! - `primary_family` holds the ITC Avant Garde family name, or a sensible fallback.

use anyhow::Result;
use cosmic_text::{fontdb, Attrs, Family, FontSystem};
use std::path::Path;

/// Known Avant Garde family names across common vendor packagings. We match these
/// case-insensitively against the family names reported by fontdb after loading
/// the user's `assets/fonts/` directory.
pub const AVANT_GARDE_CANDIDATES: &[&str] = &[
    "ITC Avant Garde Gothic LT Demi",
    "ITC Avant Garde Gothic Lt Demi",
    "ITC Avant Garde Gothic Std Demi",
    "ITC Avant Garde Std Demi",
    "ITCAvantGardeStd-Demi",
    "ITCAvantGarde-Demi",
    "ITC Avant Garde Gothic",
    "AvantGarde-Demi",
    "Avant Garde",
    "AvantGarde",
];

/// Fallback families (visually similar open-source clones) if no Avant Garde is present.
pub const FALLBACK_CANDIDATES: &[&str] = &[
    "URW Gothic",
    "URW Gothic L",
    "URWGothic-Demi",
    "TeX Gyre Adventor",
    "TeXGyreAdventor",
];

pub struct FontLoader {
    pub font_system: FontSystem,
    pub primary_family: String,
}

impl FontLoader {
    /// Build a FontLoader using only system-installed fonts. The `primary_family` is set to the
    /// default Avant Garde name; callers are expected to drop a font file via `new_with_dir` for
    /// a reproducible build.
    pub fn new() -> Self {
        Self {
            font_system: FontSystem::new(),
            primary_family: "ITC Avant Garde Gothic LT Demi".to_string(),
        }
    }

    /// Build a FontLoader whose font database is populated from every .otf/.ttf under `dir`.
    /// Then scan the loaded families and resolve `primary_family` to:
    ///   1. the first family that case-insensitively matches an Avant Garde candidate,
    ///   2. otherwise the first family matching a known fallback (URW Gothic, TeX Gyre Adventor),
    ///   3. otherwise the first available family (with a `log::warn!`).
    pub fn new_with_dir<P: AsRef<Path>>(dir: P) -> Result<Self> {
        let dir = dir.as_ref();
        let mut db = fontdb::Database::new();
        // fontdb walks recursively and silently skips non-font files.
        db.load_fonts_dir(dir);

        // Collect the canonical (first-reported, English-US) family name per face.
        // fontdb stores families as Vec<(name, lang)>; the first entry is the preferred one.
        let mut family_names: Vec<String> = db
            .faces()
            .filter_map(|face| face.families.first().map(|(name, _)| name.clone()))
            .collect();
        family_names.sort();
        family_names.dedup();

        let find_ci = |cands: &[&str]| -> Option<String> {
            for cand in cands {
                let cand_lc = cand.to_lowercase();
                if let Some(found) = family_names.iter().find(|f| f.to_lowercase() == cand_lc) {
                    return Some(found.clone());
                }
            }
            // Also allow fuzzy substring match for Avant Garde (e.g. the actual family
            // name is "ITC Avant Garde Gothic LT" with "Demi" as the subfamily/style).
            for f in &family_names {
                let flc = f.to_lowercase();
                for cand in cands {
                    let clc = cand.to_lowercase();
                    if flc.contains(&clc) || clc.contains(&flc) {
                        return Some(f.clone());
                    }
                }
            }
            None
        };

        let primary_family = if let Some(name) = find_ci(AVANT_GARDE_CANDIDATES) {
            name
        } else if let Some(name) = find_ci(FALLBACK_CANDIDATES) {
            log::warn!(
                "[font] ITC Avant Garde Gothic LT Demi not found in {}. Falling back to '{}'. \
                 Drop the correct .otf/.ttf into assets/fonts/ for the real face.",
                dir.display(),
                name
            );
            name
        } else if let Some(name) = family_names.first().cloned() {
            log::warn!(
                "[font] No Avant Garde or known fallback found in {}. Using first available \
                 family '{}'.",
                dir.display(),
                name
            );
            name
        } else {
            log::warn!(
                "[font] No fonts loaded from {}. Falling back to default family name string.",
                dir.display()
            );
            "ITC Avant Garde Gothic LT Demi".to_string()
        };

        let font_system = FontSystem::new_with_locale_and_db("en-US".to_string(), db);

        Ok(Self {
            font_system,
            primary_family,
        })
    }

    pub fn font_system(&mut self) -> &mut FontSystem {
        &mut self.font_system
    }

    pub fn primary_attrs(&self) -> Attrs<'_> {
        Attrs::new().family(Family::Name(&self.primary_family))
    }
}

impl Default for FontLoader {
    fn default() -> Self {
        Self::new()
    }
}
