//! wgpu context. In the eframe-wgpu backend the Device/Queue/Surface come from eframe::Frame.
//! The render-agent fills in the glue code that extracts them in App::update and hands them to
//! TextRenderer.

use std::sync::Arc;

pub struct GpuContext {
    pub device: Arc<wgpu::Device>,
    pub queue: Arc<wgpu::Queue>,
    pub surface_format: wgpu::TextureFormat,
}

impl GpuContext {
    pub fn from_render_state(rs: &egui_wgpu::RenderState) -> Self {
        Self {
            device: rs.device.clone(),
            queue: rs.queue.clone(),
            surface_format: rs.target_format,
        }
    }
}
