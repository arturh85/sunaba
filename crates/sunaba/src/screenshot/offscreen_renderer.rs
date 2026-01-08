//! GPU offscreen rendering for UI screenshots
//!
//! Provides headless wgpu rendering without a window, enabling automated
//! UI panel screenshot capture. Uses the same egui_wgpu integration as the
//! main renderer but renders to an offscreen texture and copies pixels back
//! to CPU memory for PNG export.

use anyhow::{Context, Result};

/// GPU offscreen renderer for UI screenshots
pub struct OffscreenRenderer {
    _instance: wgpu::Instance, // Keep instance alive
    device: wgpu::Device,
    queue: wgpu::Queue,
    egui_renderer: egui_wgpu::Renderer,
    texture: wgpu::Texture,
    texture_view: wgpu::TextureView,
    buffer: wgpu::Buffer,
    width: u32,
    height: u32,
}

impl OffscreenRenderer {
    /// Create a new offscreen renderer without a window (headless)
    pub fn new(width: u32, height: u32) -> Result<Self> {
        log::info!("Initializing offscreen renderer ({}x{})", width, height);

        // Create wgpu instance
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            ..Default::default()
        });

        // Request adapter without a surface (headless mode)
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::default(),
            compatible_surface: None, // No window needed
            force_fallback_adapter: false,
        }))
        .context("Failed to find a suitable GPU adapter")?;

        // Request device and queue
        let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("offscreen_device"),
            required_features: wgpu::Features::empty(),
            required_limits: wgpu::Limits::default(),
            memory_hints: Default::default(),
            experimental_features: wgpu::ExperimentalFeatures::disabled(),
            trace: wgpu::Trace::Off,
        }))
        .context("Failed to create GPU device")?;

        // Create offscreen render target texture
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("offscreen_render_target"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });

        let texture_view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        // Create buffer for reading pixels back from GPU to CPU
        let buffer_size = (width * height * 4) as u64; // RGBA = 4 bytes per pixel
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("offscreen_readback_buffer"),
            size: buffer_size,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        // Initialize egui renderer (same as main renderer)
        let egui_renderer = egui_wgpu::Renderer::new(
            &device,
            wgpu::TextureFormat::Rgba8UnormSrgb,
            egui_wgpu::RendererOptions::default(),
        );

        log::info!("Offscreen renderer initialized successfully");

        Ok(Self {
            _instance: instance,
            device,
            queue,
            egui_renderer,
            texture,
            texture_view,
            buffer,
            width,
            height,
        })
    }

    /// Render UI to offscreen texture and return pixel data
    ///
    /// The `ui_fn` closure receives an egui::Context and should render the desired UI.
    /// Returns RGBA pixel data (width × height × 4 bytes).
    pub fn render_ui<F>(&mut self, ui_fn: F) -> Result<Vec<u8>>
    where
        F: FnMut(&egui::Context),
    {
        // Create egui context
        let egui_ctx = egui::Context::default();

        // Create input for egui (minimal - just screen size)
        let raw_input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(self.width as f32, self.height as f32),
            )),
            ..Default::default()
        };

        // Run egui rendering logic
        let full_output = egui_ctx.run(raw_input, ui_fn);

        // Extract shapes and texture delta
        let shapes = full_output.shapes;
        let textures_delta = full_output.textures_delta;

        // Update egui textures (fonts, images, etc.)
        for (id, image_delta) in &textures_delta.set {
            self.egui_renderer
                .update_texture(&self.device, &self.queue, *id, image_delta);
        }

        // Tessellate egui output into GPU primitives
        let primitives = egui_ctx.tessellate(shapes, 1.0);

        // Create screen descriptor for egui_wgpu
        let screen_descriptor = egui_wgpu::ScreenDescriptor {
            size_in_pixels: [self.width, self.height],
            pixels_per_point: 1.0,
        };

        // Create command encoder
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("offscreen_encoder"),
            });

        // Update egui vertex/index buffers
        self.egui_renderer.update_buffers(
            &self.device,
            &self.queue,
            &mut encoder,
            &primitives,
            &screen_descriptor,
        );

        // Render egui to offscreen texture
        {
            let render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("offscreen_egui_render_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.texture_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                ..Default::default()
            });

            self.egui_renderer.render(
                &mut render_pass.forget_lifetime(),
                &primitives,
                &screen_descriptor,
            );
        }

        // Copy texture to CPU-readable buffer
        encoder.copy_texture_to_buffer(
            self.texture.as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &self.buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(self.width * 4), // RGBA = 4 bytes per pixel
                    rows_per_image: None,
                },
            },
            wgpu::Extent3d {
                width: self.width,
                height: self.height,
                depth_or_array_layers: 1,
            },
        );

        // Submit commands to GPU
        self.queue.submit(std::iter::once(encoder.finish()));

        // Map buffer for reading
        let buffer_slice = self.buffer.slice(..);

        // Use a simple Arc<Mutex<Option>> to capture the result
        let mapping_result = std::sync::Arc::new(std::sync::Mutex::new(None));
        let mapping_result_clone = mapping_result.clone();

        buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
            *mapping_result_clone.lock().unwrap() = Some(result);
        });

        // Poll device to process all pending operations (including buffer mapping)
        // In wgpu 27, poll() returns Result<PollStatus, PollError>
        self.device
            .poll(wgpu::PollType::wait_indefinitely())
            .context("Device poll failed")?;

        // Check the mapping result (should be set by now)
        let result =
            mapping_result.lock().unwrap().take().ok_or_else(|| {
                anyhow::anyhow!("Buffer mapping callback was not called after poll")
            })?;
        result.context("Failed to map buffer")?;

        // Read pixel data from mapped buffer
        let data = buffer_slice.get_mapped_range();
        let pixels = data.to_vec();

        // Cleanup
        drop(data); // Release mapped range before unmapping
        self.buffer.unmap();

        // Free egui textures
        for id in &textures_delta.free {
            self.egui_renderer.free_texture(id);
        }

        Ok(pixels)
    }
}
