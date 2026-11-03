//! Pre-shape one `scenicsynth` unit at a given font size with inter-unit kerning so an infinite
//! stream of units can be positioned as `unit_pos[g % 11] + floor(g / 11) * stride`.
//!
//! Reference: ../../scenic-motion-engine-web/src/text/layout.ts
//!
//! Contract:
//! - `build_unit_layout(font_loader, size, unit="scenicsynth")` returns a `UnitLayout`.
//! - `stride` is the horizontal distance from glyph 0 to glyph `unit.len()` (first glyph of the next
//!   unit), i.e. bakes in the kern between 'h' and 's' at the unit boundary.
//! - `unit_pos.len() == unit.chars().count()`.
//! - `ascent + descent == line_height` approximately (we normalise so this invariant holds).

use crate::text::font::FontLoader;
use crate::types::UnitLayout;
use cosmic_text::{Attrs, Buffer, Family, Metrics, Shaping};

/// Build the pre-shaped unit layout. We shape `unit + unit[0]` so the position of the trailing
/// repeated glyph gives us `stride` with the inter-unit kern baked in.
pub fn build_unit_layout(loader: &mut FontLoader, size: f32, unit: &str) -> UnitLayout {
    let n = unit.chars().count();
    assert!(n >= 2, "unit must have at least 2 chars (got {:?})", unit);

    // Build "scenicsynth" + "s" => "scenicsynths" (12 glyphs for the canonical unit).
    let first_char = unit.chars().next().expect("unit non-empty (checked above)");
    let mut shaped_string = String::with_capacity(unit.len() + first_char.len_utf8());
    shaped_string.push_str(unit);
    shaped_string.push(first_char);
    let expected_glyphs = n + 1;

    let line_height = size * 1.2;
    let metrics = Metrics::new(size, line_height);

    // We need `primary_family` as an owned string so we don't collide with the borrow of
    // `loader.font_system` below.
    let primary_family = loader.primary_family.clone();
    let fs = loader.font_system();

    let mut buffer = Buffer::new(fs, metrics);
    let attrs = Attrs::new().family(Family::Name(&primary_family));
    buffer.set_text(fs, &shaped_string, attrs, Shaping::Advanced);
    // No wrap: we want everything on one line so stride is a single-run distance.
    buffer.set_wrap(fs, cosmic_text::Wrap::None);

    // Pull positions from the first layout run.
    let mut positions_opt: Option<Vec<f32>> = None;
    let mut ascent_from_run: Option<f32> = None;
    let mut descent_from_run: Option<f32> = None;
    let mut line_height_from_run: Option<f32> = None;

    for run in buffer.layout_runs() {
        if run.glyphs.is_empty() {
            continue;
        }
        let mut xs: Vec<f32> = run.glyphs.iter().map(|g| g.x).collect();
        // Guarantee monotonic ordering just in case BiDi reordering mangles things
        // (shouldn't for Latin text, but belt and braces).
        if xs.windows(2).any(|w| w[1] < w[0]) {
            log::warn!(
                "[layout] shaped glyph x positions were not monotonically increasing; sorting."
            );
            xs.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        }
        positions_opt = Some(xs);
        line_height_from_run = Some(run.line_height);
        break;
    }

    // ascent/descent live on LayoutLine, not LayoutRun. Dip back into line_layout() to grab them.
    if let Some(layout) = buffer.line_layout(fs, 0) {
        if let Some(first) = layout.first() {
            ascent_from_run = Some(first.max_ascent);
            descent_from_run = Some(first.max_descent);
        }
    }

    let mut positions = match positions_opt {
        Some(p) => p,
        None => {
            log::error!(
                "[layout] cosmic-text produced no runs for '{}' with family '{}'. Font missing? \
                 Falling back to equal-width stub layout.",
                shaped_string,
                primary_family
            );
            return stub_layout(size, unit);
        }
    };

    // Handle shaping that collapsed glyphs (ligatures) or produced extras.
    if positions.len() != expected_glyphs {
        log::warn!(
            "[layout] shaped glyph count {} != expected {} for '{}'. Padding/truncating with \
             equal-width fallbacks. (Avant Garde has no ligatures affecting 'scenicsynth'.)",
            positions.len(),
            expected_glyphs,
            shaped_string
        );
        let advance = size * 0.55;
        if positions.len() < expected_glyphs {
            let last = positions.last().copied().unwrap_or(0.0);
            while positions.len() < expected_glyphs {
                let next = positions.last().copied().unwrap_or(last) + advance;
                positions.push(next);
            }
        } else {
            positions.truncate(expected_glyphs);
        }
    }

    let unit_pos: Vec<f32> = positions[..n].to_vec();
    let stride = positions[n];

    // Prefer real metrics from the laid-out line; otherwise synthesise.
    // TODO: real metrics — cosmic-text 0.12 exposes max_ascent/max_descent per layout line, which
    // are driven by the actual face. If a font loads cleanly these will be non-zero; else we fall
    // back to the font-agnostic 0.78/0.22 split used in the TS reference.
    let (ascent, descent, line_height) = match (ascent_from_run, descent_from_run) {
        (Some(a), Some(d)) if a > 0.0 && d >= 0.0 => {
            let lh = line_height_from_run.unwrap_or(line_height).max(size * 1.2);
            (a, d, lh)
        }
        _ => {
            let a = size * 0.78;
            let d = size * 0.22;
            let lh = line_height_from_run.unwrap_or(line_height).max(size * 1.2);
            (a, d, lh)
        }
    };

    debug_assert_eq!(
        unit_pos.len(),
        n,
        "unit_pos length must equal unit.chars().count()"
    );

    UnitLayout {
        unit: unit.to_string(),
        unit_length: n as u32,
        stride,
        unit_pos,
        ascent,
        descent,
        line_height,
        size,
    }
}

/// Equal-width fallback used when shaping completely fails. Keeps the engine running so the
/// user sees something on screen rather than a crash.
fn stub_layout(size: f32, unit: &str) -> UnitLayout {
    let n = unit.chars().count() as u32;
    let advance = size * 0.55;
    let unit_pos: Vec<f32> = (0..n).map(|i| i as f32 * advance).collect();
    UnitLayout {
        unit: unit.to_string(),
        unit_length: n,
        stride: n as f32 * advance,
        unit_pos,
        ascent: size * 0.78,
        descent: size * 0.22,
        line_height: size * 1.2,
        size,
    }
}
