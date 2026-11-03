//! Mac-style egui theme. One-shot `install_mac_style(ctx)` sets fonts, spacing, visuals,
//! widget borders, selection, scrollbars, and shadows to match a Sonoma-era system vibe.
//!
//! Design notes:
//! - egui 0.29 bundles Hack/Ubuntu/NotoSans. We can't ship SF Pro, so we tweak sizes and
//!   letter rhythm instead of swapping families. The `FontDefinitions::default()` is kept.
//! - Palettes split dark/light. Hybrid pulls accent + text from dark, surfaces from light —
//!   not used by `install_mac_style` directly, but exposed for callers that want a mixed
//!   HUD look.
//! - Rounding is a flat 6 px on widgets, 10 px on windows, 8 px on menus — close to AppKit.
//! - Hairlines are 0.5 px so they're retina-crisp without egui's AA fuzzing them.
//! - Scrollbars route through `Spacing::scroll` (ScrollStyle) in egui 0.29 — NOT
//!   `Visuals::scroll`. The task spec said `visuals.scroll.*`; corrected here.

use egui::epaint::Shadow;
use egui::{
    vec2, Color32, Context, FontFamily, FontId, Rounding, Stroke, Style, TextStyle, Vec2, Visuals,
};

// ---------------------------------------------------------------------------
// Palettes
// ---------------------------------------------------------------------------

/// macOS Sonoma system vibrancy palette, dark mode.
#[derive(Clone, Copy, Debug)]
pub struct MacPalette {
    pub window_bg: Color32,
    pub panel_bg: Color32,
    pub hover: Color32,
    pub selected: Color32,
    pub border: Color32,
    pub text_primary: Color32,
    pub text_secondary: Color32,
}

/// Dark palette (SF Symbols / Sonoma dark surfaces).
pub const MAC_PALETTE_DARK: MacPalette = MacPalette {
    window_bg: Color32::from_rgb(0x1C, 0x1C, 0x1E),
    panel_bg: Color32::from_rgb(0x2C, 0x2C, 0x2E),
    hover: Color32::from_rgb(0x3A, 0x3A, 0x3C),
    selected: Color32::from_rgb(0x0A, 0x84, 0xFF),
    border: Color32::from_rgb(0x38, 0x38, 0x3A),
    text_primary: Color32::from_rgb(0xF2, 0xF2, 0xF7),
    text_secondary: Color32::from_rgb(0x8E, 0x8E, 0x93),
};

/// Light palette (Sonoma light surfaces).
pub const MAC_PALETTE_LIGHT: MacPalette = MacPalette {
    window_bg: Color32::from_rgb(0xF2, 0xF2, 0xF7),
    panel_bg: Color32::from_rgb(0xFF, 0xFF, 0xFF),
    hover: Color32::from_rgb(0xE5, 0xE5, 0xEA),
    selected: Color32::from_rgb(0x00, 0x7A, 0xFF),
    border: Color32::from_rgb(0xD1, 0xD1, 0xD6),
    text_primary: Color32::from_rgb(0x00, 0x00, 0x00),
    text_secondary: Color32::from_rgb(0x6E, 0x6E, 0x73),
};

/// Hybrid palette (dark accent + text, light surfaces) — exposed for custom HUDs.
pub const MAC_PALETTE_HYBRID: MacPalette = MacPalette {
    window_bg: MAC_PALETTE_LIGHT.window_bg,
    panel_bg: MAC_PALETTE_LIGHT.panel_bg,
    hover: MAC_PALETTE_LIGHT.hover,
    selected: MAC_PALETTE_DARK.selected,
    border: MAC_PALETTE_LIGHT.border,
    text_primary: MAC_PALETTE_DARK.text_primary,
    text_secondary: MAC_PALETTE_DARK.text_secondary,
};

/// Returns the palette matching the current egui dark/light mode.
pub fn mac_palette_for(ctx: &Context) -> MacPalette {
    if ctx.style().visuals.dark_mode {
        MAC_PALETTE_DARK
    } else {
        MAC_PALETTE_LIGHT
    }
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

/// Install the full Mac-style theme. Call once from `ScenicApp::new`.
pub fn install_mac_style(ctx: Context) {
    // Fonts: keep bundled families; we only tweak sizes via TextStyles below.
    ctx.set_fonts(egui::FontDefinitions::default());

    ctx.style_mut(|style| {
        apply_typography(style);
        apply_spacing(style);
        apply_visuals(style);
        apply_scroll(style);
        // Logic Pro–ish tooltip delay (default is already 0.5, but keep explicit).
        style.interaction.tooltip_delay = 0.5;
    });
}

/// Draw a 20%-opacity hairline divider spanning `ui.max_rect().x_range()` at the
/// current cursor y. Logic Pro–style soft separator, replaces loud `ui.separator()`.
pub fn soft_divider(ui: &mut egui::Ui) {
    let color = ui.visuals().widgets.noninteractive.bg_stroke.color;
    let faded = Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), 51);
    let y = ui.cursor().min.y;
    ui.painter()
        .hline(ui.max_rect().x_range(), y, Stroke::new(1.0_f32, faded));
    ui.add_space(4.0);
}

/// Secondary label color helper — SF-style grey (`#8E8E93`).
pub fn secondary_label(ui: &mut egui::Ui, text: &str) -> egui::Response {
    let p = if ui.visuals().dark_mode {
        MAC_PALETTE_DARK
    } else {
        MAC_PALETTE_LIGHT
    };
    ui.label(egui::RichText::new(text).color(p.text_secondary))
}

// ---------------------------------------------------------------------------
// Typography
// ---------------------------------------------------------------------------

fn apply_typography(style: &mut Style) {
    // Body / Button: 13 px proportional (SF-ish body size).
    style
        .text_styles
        .insert(TextStyle::Body, FontId::new(13.0, FontFamily::Proportional));
    style.text_styles.insert(
        TextStyle::Button,
        FontId::new(13.0, FontFamily::Proportional),
    );
    // Heading: 15 px (we can't bold the bundled font — size + spacing stand in).
    style.text_styles.insert(
        TextStyle::Heading,
        FontId::new(15.0, FontFamily::Proportional),
    );
    // Small (captions / secondary text): 11 px.
    style.text_styles.insert(
        TextStyle::Small,
        FontId::new(11.0, FontFamily::Proportional),
    );
    // Monospace (HUD): 12 px.
    style.text_styles.insert(
        TextStyle::Monospace,
        FontId::new(12.0, FontFamily::Monospace),
    );
}

// ---------------------------------------------------------------------------
// Spacing
// ---------------------------------------------------------------------------

fn apply_spacing(style: &mut Style) {
    let s = &mut style.spacing;
    // Tighter vertical rhythm, slightly wider horizontal — inspector feel.
    s.item_spacing = vec2(10.0, 4.0);
    s.indent = 14.0;
    s.button_padding = vec2(12.0, 4.0);
    s.icon_width = 14.0;
    s.slider_width = 180.0;
    s.combo_width = 160.0;
}

// ---------------------------------------------------------------------------
// Visuals (colors, borders, rounding, shadows, selection)
// ---------------------------------------------------------------------------

fn apply_visuals(style: &mut Style) {
    let p = if style.visuals.dark_mode {
        MAC_PALETTE_DARK
    } else {
        MAC_PALETTE_LIGHT
    };
    let v = &mut style.visuals;

    // Base text + surfaces.
    v.override_text_color = Some(p.text_primary);
    v.window_fill = p.window_bg;
    v.panel_fill = p.panel_bg;
    v.extreme_bg_color = p.window_bg;
    v.faint_bg_color = p.hover;

    // Window + menu rounding.
    v.window_rounding = Rounding::same(10.0);
    v.menu_rounding = Rounding::same(8.0);

    // Shadows: window shadow off (Mac side panels are flush); popup gets a soft lift.
    v.window_shadow = Shadow::NONE;
    v.popup_shadow = Shadow {
        offset: Vec2::new(0.0, 2.0),
        blur: 10.0,
        spread: 0.0,
        color: Color32::from_black_alpha(if v.dark_mode { 80 } else { 40 }),
    };

    // Window stroke (hairline border).
    v.window_stroke = Stroke::new(0.5_f32, p.border);

    // Logic Pro–style bare collapsing headers (no boxed frame, just an indent chevron).
    v.collapsing_header_frame = false;

    // Selection (text, highlighted items).
    v.selection.bg_fill = p.selected;
    v.selection.stroke = Stroke::new(1.0_f32, p.selected);
    v.hyperlink_color = p.selected;

    // Widget visuals: tighten rounding and move to Mac grays.
    apply_widget_visuals(v, &p);
}

fn apply_widget_visuals(v: &mut Visuals, p: &MacPalette) {
    let r = Rounding::same(6.0);
    let hairline = Stroke::new(0.5_f32, p.border);

    // Noninteractive = window chrome / dividers.
    let w = &mut v.widgets.noninteractive;
    w.rounding = r;
    w.bg_fill = p.panel_bg;
    w.weak_bg_fill = p.panel_bg;
    w.bg_stroke = hairline;
    w.fg_stroke = Stroke::new(1.0_f32, p.text_primary);

    // Inactive = buttons at rest, slider track.
    // `weak_bg_fill` is what Button uses for its background; keep it a shade above the
    // panel so buttons are visible without being loud (fixes buttons-blend-into-panel).
    let w = &mut v.widgets.inactive;
    w.rounding = r;
    w.bg_fill = p.hover;
    w.weak_bg_fill = p.hover;
    w.bg_stroke = hairline;
    w.fg_stroke = Stroke::new(1.0_f32, p.text_primary);

    // Hovered = highlight on mouseover.
    let w = &mut v.widgets.hovered;
    w.rounding = r;
    w.bg_fill = p.hover;
    w.weak_bg_fill = p.hover;
    w.bg_stroke = Stroke::new(0.5_f32, p.selected);
    w.fg_stroke = Stroke::new(1.0_f32, p.text_primary);

    // Active = pressed / focused.
    let w = &mut v.widgets.active;
    w.rounding = r;
    w.bg_fill = p.selected;
    w.weak_bg_fill = p.selected;
    w.bg_stroke = Stroke::new(1.0_f32, p.selected);
    w.fg_stroke = Stroke::new(1.0_f32, Color32::WHITE);

    // Open = combo-box with a dropdown out.
    let w = &mut v.widgets.open;
    w.rounding = r;
    w.bg_fill = p.hover;
    w.weak_bg_fill = p.hover;
    w.bg_stroke = hairline;
    w.fg_stroke = Stroke::new(1.0_f32, p.text_primary);
}

// ---------------------------------------------------------------------------
// Scrollbars (Mac-style thin, auto-hide)
// ---------------------------------------------------------------------------

fn apply_scroll(style: &mut Style) {
    // NOTE: In egui 0.29, `ScrollStyle` lives on `Spacing::scroll`, NOT `Visuals::scroll`.
    // The task spec said `visuals.scroll.*`; mapped to `spacing.scroll.*` here.
    // TODO: `foreground_color` on egui 0.29's ScrollStyle is a `bool` (contrast toggle),
    // not a `Color32`. We set it to `true` for high-contrast floating handles instead of
    // the requested rgba. If a future egui exposes a handle color field, wire it there.
    let s = &mut style.spacing.scroll;
    s.floating = true;
    s.bar_width = 8.0;
    s.handle_min_length = 24.0;
    s.foreground_color = true;
    s.bar_outer_margin = 2.0;
    s.bar_inner_margin = 2.0;
}
