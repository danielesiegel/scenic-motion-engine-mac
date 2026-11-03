use scenic_motion_engine::app::ScenicApp;

/// Procedurally generate a 64x64 RGBA icon (small enough to inline cheaply).
/// macOS will upscale as needed. We use a 4-dot gradient suggestive of the four
/// audio bands (sub / low / mid / high) over a near-black background.
fn generate_app_icon() -> egui::IconData {
    const W: u32 = 64;
    const H: u32 = 64;
    let mut rgba = vec![0u8; (W * H * 4) as usize];
    // Palette per quadrant.
    let bands: [[u8; 3]; 4] = [
        [0x3a, 0x7a, 0xff], // sub   — blue
        [0x44, 0xcc, 0x88], // low   — green
        [0xff, 0xaa, 0x33], // mid   — orange
        [0xff, 0x55, 0x66], // high  — red
    ];
    for y in 0..H {
        for x in 0..W {
            let qx = (x >= W / 2) as usize;
            let qy = (y >= H / 2) as usize;
            let idx = qy * 2 + qx;
            let [r, g, b] = bands[idx];
            let i = ((y * W + x) * 4) as usize;
            rgba[i] = r;
            rgba[i + 1] = g;
            rgba[i + 2] = b;
            rgba[i + 3] = 0xff;
        }
    }
    egui::IconData {
        rgba,
        width: W,
        height: H,
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
        .with_icon(generate_app_icon());

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
