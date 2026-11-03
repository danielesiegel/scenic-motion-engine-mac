use scenic_motion_engine::app::ScenicApp;

fn main() -> Result<(), eframe::Error> {
    env_logger::init();
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("scenic-motion-engine")
            .with_inner_size([1680.0, 1050.0])
            .with_min_inner_size([960.0, 600.0]),
        renderer: eframe::Renderer::Wgpu,
        ..Default::default()
    };
    eframe::run_native(
        "scenic-motion-engine",
        native_options,
        Box::new(|cc| Ok(Box::new(ScenicApp::new(cc)))),
    )
}
