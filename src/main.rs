use scenic_motion_engine::app::ScenicApp;

/// Decode the real AppIcon PNG (produced by the icon pipeline from AppIcon.svg)
/// into an `egui::IconData`. The PNG is embedded at compile time via
/// `include_bytes!`, so the running binary does not need the asset on disk.
fn load_app_icon() -> Option<egui::IconData> {
    let bytes = include_bytes!("../assets/icon/AppIcon.png");
    let img = image::load_from_memory(bytes).ok()?.to_rgba8();
    let (width, height) = img.dimensions();
    Some(egui::IconData {
        rgba: img.into_raw(),
        width,
        height,
    })
}

/// Minimal 1x1 transparent fallback. Only used if the embedded PNG ever fails
/// to decode (which would indicate a build-time asset problem).
fn fallback_1x1_icon() -> egui::IconData {
    egui::IconData {
        rgba: vec![0, 0, 0, 0],
        width: 1,
        height: 1,
    }
}

fn main() -> Result<(), eframe::Error> {
    env_logger::init();

    let viewport = egui::ViewportBuilder::default()
        .with_title("scenic motion engine")
        .with_app_id("com.integralmimetics.scenic-motion-engine")
        .with_inner_size([1680.0, 1050.0])
        .with_min_inner_size([1200.0, 720.0])
        .with_decorations(true)
        .with_transparent(false)
        .with_titlebar_shown(true)
        .with_titlebar_buttons_shown(true)
        // Modern macOS transparent title-bar overlay. If the traffic-light
        // buttons disappear on some systems, flip this to `false`.
        .with_fullsize_content_view(true)
        .with_title_shown(false)
        .with_icon(load_app_icon().unwrap_or_else(fallback_1x1_icon));

    let native_options = eframe::NativeOptions {
        viewport,
        renderer: eframe::Renderer::Wgpu,
        // Persist window position + size between runs (requires the
        // "persistence" eframe feature, enabled in Cargo.toml).
        persist_window: true,
        ..Default::default()
    };

    eframe::run_native(
        "scenic motion engine",
        native_options,
        Box::new(|cc| Ok(Box::new(ScenicApp::new(cc)))),
    )
}
