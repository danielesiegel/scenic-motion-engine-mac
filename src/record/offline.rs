//! Headless frame-by-frame renderer.
//!
//! Creates a standalone wgpu Device/Queue (no winit, no surface), renders every frame of a time
//! range into an offscreen `Rgba8UnormSrgb` texture, reads it back via a mapped buffer, and pipes
//! each frame to [`FfmpegRecorder`]. Deterministic, independent from the eframe redraw loop, and
//! suitable for long high-quality exports.
//!
//! The glyphon atlas is bound to the same color format as the render target so the
//! [`TextRenderer`] drawing primitives work unchanged against the offscreen target.

use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{anyhow, Context, Result};

use crate::engine::{evaluate, EngineInputs};
use crate::record::mp4::FfmpegRecorder;
use crate::render::gpu::GpuContext;
use crate::render::text_renderer::TextRenderer;
use crate::text::{build_unit_layout, FontLoader};
use crate::types::{Config, EventSource, KeyRowValue, RowsMode, UnitLayout};

/// Chosen color format for both the render target and the glyphon atlas.
/// Matches the live eframe-wgpu backend default so pixel semantics are identical.
const TARGET_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;

/// Options describing a single offline render job.
#[derive(Debug, Clone)]
pub struct OfflineRenderOptions {
    pub start_at: f32,
    pub duration: f32,
    pub fps: u32,
    pub width: u32,
    pub height: u32,
    pub output: PathBuf,
    pub audio_in: Option<PathBuf>,
}

/// Row-pitch padded to [`wgpu::COPY_BYTES_PER_ROW_ALIGNMENT`] (256 B).
fn padded_bytes_per_row(width: u32) -> u32 {
    let unpadded = width * 4;
    let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
    unpadded.div_ceil(align) * align
}

/// Holds the wgpu context + text renderer + per-frame readback state.
///
/// Lifetime: build once, call [`OfflineRenderer::render`] N times for different jobs. The render
/// target is sized for the first job and resized on subsequent jobs if dimensions differ.
pub struct OfflineRenderer {
    pub config: Config,
    pub source: Arc<dyn EventSource>,
    pub font_loader: FontLoader,
    pub layout: UnitLayout,

    device: Arc<wgpu::Device>,
    queue: Arc<wgpu::Queue>,

    text_renderer: TextRenderer,

    target: Option<RenderTarget>,
}

struct RenderTarget {
    width: u32,
    height: u32,
    texture: wgpu::Texture,
    view: wgpu::TextureView,
    readback: wgpu::Buffer,
    padded_row_bytes: u32,
}

impl OfflineRenderer {
    /// Build a standalone offline renderer. The provided `config` carries canvas, text, colors,
    /// camera, feel, and binding configuration. `source` is the audio event source (JSON analysis
    /// or synthetic). `font_dir` is scanned for fonts; falls back to system defaults if missing.
    pub fn new(
        config: Config,
        source: Arc<dyn EventSource>,
        font_dir: &std::path::Path,
    ) -> Result<Self> {
        let mut font_loader = if font_dir.is_dir() {
            FontLoader::new_with_dir(font_dir)
                .with_context(|| format!("loading fonts from {}", font_dir.display()))?
        } else {
            log::warn!(
                "[offline] font directory {} not found; using system defaults",
                font_dir.display()
            );
            FontLoader::new()
        };

        let layout = build_unit_layout(
            &mut font_loader,
            config.layout.font_size_px,
            &config.text.unit,
        );

        // --- wgpu Device / Queue (headless) ---
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::METAL | wgpu::Backends::VULKAN,
            ..Default::default()
        });
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: None,
            force_fallback_adapter: false,
        }))
        .ok_or_else(|| anyhow!("no wgpu adapter available for offline rendering"))?;

        let (device, queue) = pollster::block_on(adapter.request_device(
            &wgpu::DeviceDescriptor {
                label: Some("offline-renderer"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                memory_hints: wgpu::MemoryHints::default(),
            },
            None,
        ))
        .context("request_device failed")?;

        let device = Arc::new(device);
        let queue = Arc::new(queue);

        let gpu = GpuContext {
            device: device.clone(),
            queue: queue.clone(),
            surface_format: TARGET_FORMAT,
        };

        let mut text_renderer = TextRenderer::new(gpu, &mut font_loader.font_system)
            .context("TextRenderer init failed")?;
        text_renderer.set_font_family(Some(font_loader.primary_family.clone()));

        Ok(Self {
            config,
            source,
            font_loader,
            layout,
            device,
            queue,
            text_renderer,
            target: None,
        })
    }

    /// (Re)allocate the render target texture + readback buffer at `width x height`.
    fn ensure_target(&mut self, width: u32, height: u32) {
        if let Some(t) = &self.target {
            if t.width == width && t.height == height {
                return;
            }
        }
        let padded_row_bytes = padded_bytes_per_row(width);
        let readback_size = padded_row_bytes as u64 * height as u64;

        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("offline-target"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: TARGET_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        let readback = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("offline-readback"),
            size: readback_size,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        self.text_renderer.resize(width, height);

        self.target = Some(RenderTarget {
            width,
            height,
            texture,
            view,
            readback,
            padded_row_bytes,
        });
    }

    fn resolve_row_count_and_key(&self, width: u32, height: u32) -> (u32, u32) {
        // Mirror ScenicApp::resolve_row_count_and_key, but respect the requested output
        // dimensions rather than the config.canvas defaults so that `--width/--height`
        // CLI flags take precedence without mutating the user's Config.
        let row_count = match self.config.layout.rows {
            RowsMode::Count(n) => n.max(1),
            RowsMode::Auto(_) => {
                let h = (height as f32 - 2.0 * self.config.layout.margin_px).max(1.0);
                let lh = if self.config.layout.leading_px > 0.0 {
                    self.config.layout.leading_px
                } else {
                    self.layout.line_height.max(self.config.layout.font_size_px)
                };
                let base = (h / lh).ceil() as u32;
                let extra = self.config.camera.row_overshoot;
                base.max(1) + 2 * extra
            }
        };
        let key_row = match self.config.key.single_row {
            KeyRowValue::Middle(_) => row_count / 2,
            KeyRowValue::Index(i) => i.min(row_count.saturating_sub(1)),
        };
        let _ = width; // reserved for future horizontal-aware layouts
        (row_count, key_row)
    }

    /// Render the entire time range into `opts.output`.
    ///
    /// `on_progress(frame_index, total_frames)` fires after each successfully encoded frame.
    pub fn render(
        &mut self,
        opts: &OfflineRenderOptions,
        mut on_progress: impl FnMut(u32, u32),
    ) -> Result<()> {
        if opts.width == 0 || opts.height == 0 {
            anyhow::bail!(
                "width/height must be non-zero (got {}x{})",
                opts.width,
                opts.height
            );
        }
        if opts.fps == 0 {
            anyhow::bail!("fps must be non-zero");
        }
        if opts.duration <= 0.0 {
            anyhow::bail!("duration must be > 0 (got {})", opts.duration);
        }

        self.ensure_target(opts.width, opts.height);

        // Patch the config's canvas dimensions for this render pass. We clone so the caller's
        // Config stays untouched between runs.
        let mut config = self.config.clone();
        config.canvas.width = opts.width;
        config.canvas.height = opts.height;
        // Offline target maps 1:1 to pixels — no DPR or render-scale upsampling.
        config.canvas.dpr = 1.0;
        config.canvas.render_scale = 1.0;

        let (row_count, key_row) = self.resolve_row_count_and_key(opts.width, opts.height);

        let total_frames = (opts.duration * opts.fps as f32).round() as u32;
        if total_frames == 0 {
            anyhow::bail!(
                "computed zero frames (duration={}s fps={})",
                opts.duration,
                opts.fps
            );
        }

        let mut recorder = FfmpegRecorder::start(
            &opts.output,
            opts.width,
            opts.height,
            opts.fps,
            opts.audio_in.as_deref(),
            opts.start_at,
            opts.duration,
        )
        .context("starting ffmpeg")?;

        for frame_index in 0..total_frames {
            let t = opts.start_at + frame_index as f32 / opts.fps as f32;

            let state = evaluate(EngineInputs {
                t,
                config: &config,
                source: self.source.as_ref(),
                layout: &self.layout,
                row_count,
                key_row,
            });

            if let Err(err) = self.text_renderer.prepare(
                &config,
                &self.layout,
                &state,
                row_count,
                key_row,
                &mut self.font_loader.font_system,
            ) {
                log::warn!("[offline] prepare failed at t={:.3}: {}", t, err);
            }

            let rgba = self.render_one_frame(opts.width, opts.height)?;
            recorder
                .push_frame(&rgba)
                .with_context(|| format!("writing frame {} / {}", frame_index + 1, total_frames))?;

            on_progress(frame_index + 1, total_frames);
        }

        recorder.finish().context("ffmpeg finish")?;
        Ok(())
    }

    /// Render one prepared frame to the offscreen texture, copy it to the readback buffer,
    /// map/unmap it, and return the unpadded RGBA byte vector.
    fn render_one_frame(&mut self, width: u32, height: u32) -> Result<Vec<u8>> {
        let target = self.target.as_ref().expect("ensure_target called");
        let padded_row_bytes = target.padded_row_bytes;

        let bg_rgba = parse_bg_color(&self.config.colors.background);

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("offline-frame"),
            });

        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("offline-pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &target.view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(bg_rgba),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            if let Err(err) = self.text_renderer.render(&mut pass) {
                log::warn!("[offline] text render failed: {}", err);
            }
        }

        encoder.copy_texture_to_buffer(
            wgpu::ImageCopyTexture {
                texture: &target.texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::ImageCopyBuffer {
                buffer: &target.readback,
                layout: wgpu::ImageDataLayout {
                    offset: 0,
                    bytes_per_row: Some(padded_row_bytes),
                    rows_per_image: Some(height),
                },
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );

        self.queue.submit(std::iter::once(encoder.finish()));

        // Map readback. We use a flume-free synchronous barrier via a std channel.
        let (tx, rx) = std::sync::mpsc::channel();
        {
            let slice = target.readback.slice(..);
            slice.map_async(wgpu::MapMode::Read, move |res| {
                let _ = tx.send(res);
            });
        }
        self.device.poll(wgpu::Maintain::Wait);
        rx.recv()
            .context("map_async channel closed")?
            .context("map_async failed")?;

        let unpadded = {
            let slice = target.readback.slice(..);
            let data = slice.get_mapped_range();
            let unpadded_row = (width * 4) as usize;
            let padded = padded_row_bytes as usize;
            let mut out = Vec::with_capacity(unpadded_row * height as usize);
            for y in 0..height as usize {
                let start = y * padded;
                out.extend_from_slice(&data[start..start + unpadded_row]);
            }
            drop(data);
            out
        };
        target.readback.unmap();

        Ok(unpadded)
    }
}

/// Parse `#rrggbb[aa]` into a wgpu Color. Falls back to opaque black on error.
fn parse_bg_color(s: &str) -> wgpu::Color {
    let trimmed = s.trim().trim_start_matches('#');
    let bytes = trimmed.as_bytes();
    let hex2 = |a: u8, b: u8| -> Option<u8> {
        let hi = (a as char).to_digit(16)? as u8;
        let lo = (b as char).to_digit(16)? as u8;
        Some((hi << 4) | lo)
    };
    let (r, g, b, a) = match bytes.len() {
        6 => (
            hex2(bytes[0], bytes[1]).unwrap_or(0),
            hex2(bytes[2], bytes[3]).unwrap_or(0),
            hex2(bytes[4], bytes[5]).unwrap_or(0),
            255u8,
        ),
        8 => (
            hex2(bytes[0], bytes[1]).unwrap_or(0),
            hex2(bytes[2], bytes[3]).unwrap_or(0),
            hex2(bytes[4], bytes[5]).unwrap_or(0),
            hex2(bytes[6], bytes[7]).unwrap_or(255),
        ),
        _ => (0, 0, 0, 255),
    };
    // Convert sRGB u8 -> linear f64 for the Rgba8UnormSrgb clear value.
    fn srgb_to_linear(c: u8) -> f64 {
        let x = c as f64 / 255.0;
        if x <= 0.04045 {
            x / 12.92
        } else {
            ((x + 0.055) / 1.055).powf(2.4)
        }
    }
    wgpu::Color {
        r: srgb_to_linear(r),
        g: srgb_to_linear(g),
        b: srgb_to_linear(b),
        a: a as f64 / 255.0,
    }
}
