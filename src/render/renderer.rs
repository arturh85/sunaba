//! wgpu-based renderer for pixel world

use std::iter;
use anyhow::Result;
use wgpu::util::DeviceExt;
use winit::window::Window;

use crate::world::{World, Chunk, CHUNK_SIZE};

/// Vertex for fullscreen quad
#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    position: [f32; 2],
    tex_coords: [f32; 2],
}

impl Vertex {
    const ATTRIBS: [wgpu::VertexAttribute; 2] = wgpu::vertex_attr_array![
        0 => Float32x2,
        1 => Float32x2,
    ];
    
    fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBS,
        }
    }
}

// Fullscreen quad vertices
const QUAD_VERTICES: &[Vertex] = &[
    Vertex { position: [-1.0, -1.0], tex_coords: [0.0, 1.0] },
    Vertex { position: [ 1.0, -1.0], tex_coords: [1.0, 1.0] },
    Vertex { position: [ 1.0,  1.0], tex_coords: [1.0, 0.0] },
    Vertex { position: [-1.0,  1.0], tex_coords: [0.0, 0.0] },
];

const QUAD_INDICES: &[u16] = &[0, 1, 2, 2, 3, 0];

/// Camera uniform data
#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct CameraUniform {
    /// Camera position in world pixels
    position: [f32; 2],
    /// Zoom level (pixels per screen unit)
    zoom: f32,
    /// Aspect ratio (width / height)
    aspect: f32,
}

pub struct Renderer {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    size: winit::dpi::PhysicalSize<u32>,

    render_pipeline: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,

    // World texture (stores all visible chunks)
    world_texture: wgpu::Texture,
    #[allow(dead_code)]
    world_texture_view: wgpu::TextureView,
    #[allow(dead_code)]
    world_sampler: wgpu::Sampler,
    world_bind_group: wgpu::BindGroup,

    // Camera
    camera_buffer: wgpu::Buffer,
    camera_bind_group: wgpu::BindGroup,
    camera: CameraUniform,

    // Pixel buffer for CPU-side rendering
    pixel_buffer: Vec<u8>,

    // UI rendering
    egui_renderer: egui_wgpu::Renderer,

    // Temperature overlay
    temp_texture: wgpu::Texture,
    #[allow(dead_code)]
    temp_texture_view: wgpu::TextureView,
    #[allow(dead_code)]
    temp_sampler: wgpu::Sampler,
    overlay_uniform_buffer: wgpu::Buffer,
    temp_bind_group: wgpu::BindGroup,
    overlay_enabled: bool,
}

impl Renderer {
    const WORLD_TEXTURE_SIZE: u32 = 512; // 8x8 chunks visible
    
    pub async fn new(window: &Window) -> Result<Self> {
        let size = window.inner_size();
        
        // Create instance
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });
        
        // Create surface
        // SAFETY: The window must outlive the surface. This is ensured by the App struct
        // owning both the window and the renderer, with the renderer field appearing after
        // the window field (drop order is reverse of declaration order in Rust).
        let surface = unsafe {
            let target = wgpu::SurfaceTargetUnsafe::from_window(&window)
                .map_err(|e| anyhow::anyhow!("Failed to create surface target: {:?}", e))?;
            instance.create_surface_unsafe(target)?
        };
        
        // Request adapter
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .ok_or_else(|| anyhow::anyhow!("Failed to find suitable GPU adapter"))?;
        
        // Create device and queue
        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::default(),
                    label: Some("device"),
                },
                None,
            )
            .await?;
        
        // Configure surface
        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps
            .formats
            .iter()
            .find(|f| f.is_srgb())
            .copied()
            .unwrap_or(surface_caps.formats[0]);
        
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::AutoVsync,
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);
        
        // Create world texture
        let world_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("world_texture"),
            size: wgpu::Extent3d {
                width: Self::WORLD_TEXTURE_SIZE,
                height: Self::WORLD_TEXTURE_SIZE,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        
        let world_texture_view = world_texture.create_view(&wgpu::TextureViewDescriptor::default());
        
        let world_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest, // Pixel-perfect rendering
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });
        
        // Texture bind group layout
        let texture_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("texture_bind_group_layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        
        let world_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("world_bind_group"),
            layout: &texture_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&world_texture_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&world_sampler),
                },
            ],
        });
        
        // Camera uniform
        let camera = CameraUniform {
            position: [0.0, 0.0],
            zoom: 0.015, // Lower = zoomed out more (world_width = 2 * aspect / zoom)
            aspect: size.width as f32 / size.height as f32,
        };
        
        let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("camera_buffer"),
            contents: bytemuck::cast_slice(&[camera]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        
        let camera_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("camera_bind_group_layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        
        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("camera_bind_group"),
            layout: &camera_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            }],
        });
        
        // Temperature overlay texture (40x40 for 5x5 chunks × 8x8 cells)
        const TEMP_TEXTURE_SIZE: u32 = 40;
        let temp_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("temp_texture"),
            size: wgpu::Extent3d {
                width: TEMP_TEXTURE_SIZE,
                height: TEMP_TEXTURE_SIZE,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R32Float,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        let temp_texture_view = temp_texture.create_view(&wgpu::TextureViewDescriptor::default());

        let temp_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest, // Nearest for R32Float (not filterable)
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        // Overlay uniform buffer (enabled flag)
        let overlay_uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("overlay_uniform_buffer"),
            size: 32, // u32 (4) + vec3<u32> padding (16 due to alignment) + struct padding (12) = 32 bytes
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Temperature overlay bind group layout
        let temp_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("temp_bind_group_layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: false },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::NonFiltering),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        let temp_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("temp_bind_group"),
            layout: &temp_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&temp_texture_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&temp_sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: overlay_uniform_buffer.as_entire_binding(),
                },
            ],
        });

        // Shader
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
        });

        // Pipeline layout
        let render_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("render_pipeline_layout"),
            bind_group_layouts: &[&texture_bind_group_layout, &camera_bind_group_layout, &temp_bind_group_layout],
            push_constant_ranges: &[],
        });
        
        // Render pipeline
        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("render_pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[Vertex::desc()],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
        });
        
        // Vertex and index buffers
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("vertex_buffer"),
            contents: bytemuck::cast_slice(QUAD_VERTICES),
            usage: wgpu::BufferUsages::VERTEX,
        });
        
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("index_buffer"),
            contents: bytemuck::cast_slice(QUAD_INDICES),
            usage: wgpu::BufferUsages::INDEX,
        });
        
        // Pixel buffer for CPU rendering
        let pixel_buffer = vec![0u8; (Self::WORLD_TEXTURE_SIZE * Self::WORLD_TEXTURE_SIZE * 4) as usize];

        // Initialize egui renderer
        let egui_renderer = egui_wgpu::Renderer::new(
            &device,
            config.format,
            None,
            1,
        );

        // Initialize overlay as disabled (32 bytes: enabled + padding)
        queue.write_buffer(&overlay_uniform_buffer, 0, bytemuck::cast_slice(&[0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32]));

        Ok(Self {
            surface,
            device,
            queue,
            config,
            size,
            render_pipeline,
            vertex_buffer,
            index_buffer,
            world_texture,
            world_texture_view,
            world_sampler,
            world_bind_group,
            camera_buffer,
            camera_bind_group,
            camera,
            pixel_buffer,
            egui_renderer,
            temp_texture,
            temp_texture_view,
            temp_sampler,
            overlay_uniform_buffer,
            temp_bind_group,
            overlay_enabled: false,
        })
    }
    
    pub fn resize(&mut self, width: u32, height: u32) {
        if width > 0 && height > 0 {
            self.size = winit::dpi::PhysicalSize::new(width, height);
            self.config.width = width;
            self.config.height = height;
            self.surface.configure(&self.device, &self.config);
            
            // Update camera aspect ratio
            self.camera.aspect = width as f32 / height as f32;
            self.queue.write_buffer(&self.camera_buffer, 0, bytemuck::cast_slice(&[self.camera]));
        }
    }
    
    pub fn render(
        &mut self,
        world: &World,
        egui_ctx: &egui::Context,
        textures_delta: egui::TexturesDelta,
        shapes: Vec<egui::epaint::ClippedShape>,
    ) -> Result<()> {
        log::trace!("Render frame: camera pos=({:.1}, {:.1}), zoom={:.2}",
                   self.camera.position[0], self.camera.position[1], self.camera.zoom);

        // Update pixel buffer from world chunks
        self.update_pixel_buffer(world);

        // Upload to GPU
        self.queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &self.world_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &self.pixel_buffer,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(Self::WORLD_TEXTURE_SIZE * 4),
                rows_per_image: Some(Self::WORLD_TEXTURE_SIZE),
            },
            wgpu::Extent3d {
                width: Self::WORLD_TEXTURE_SIZE,
                height: Self::WORLD_TEXTURE_SIZE,
                depth_or_array_layers: 1,
            },
        );

        // Update camera position to follow player
        self.camera.position = [world.player.position.x, world.player.position.y];
        self.queue.write_buffer(&self.camera_buffer, 0, bytemuck::cast_slice(&[self.camera]));

        // Get output texture
        let output = self.surface.get_current_texture()?;
        let view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("render_encoder"),
        });

        // Update egui textures
        for (id, image_delta) in &textures_delta.set {
            self.egui_renderer.update_texture(&self.device, &self.queue, *id, image_delta);
        }

        // Prepare egui primitives
        let screen_descriptor = egui_wgpu::ScreenDescriptor {
            size_in_pixels: [self.config.width, self.config.height],
            pixels_per_point: 1.0,
        };

        let primitives = egui_ctx.tessellate(shapes, 1.0);
        self.egui_renderer.update_buffers(&self.device, &self.queue, &mut encoder, &primitives, &screen_descriptor);

        // Render world
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("world_render_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.1,
                            g: 0.1,
                            b: 0.15,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                ..Default::default()
            });

            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_bind_group(0, &self.world_bind_group, &[]);
            render_pass.set_bind_group(1, &self.camera_bind_group, &[]);
            render_pass.set_bind_group(2, &self.temp_bind_group, &[]);
            render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
            render_pass.draw_indexed(0..6, 0, 0..1);
        }

        // Render egui UI
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("egui_render_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                ..Default::default()
            });

            self.egui_renderer.render(&mut render_pass, &primitives, &screen_descriptor);
        }

        // Free egui textures
        for id in &textures_delta.free {
            self.egui_renderer.free_texture(id);
        }

        self.queue.submit(iter::once(encoder.finish()));
        output.present();

        Ok(())
    }
    
    fn update_pixel_buffer(&mut self, world: &World) {
        // Clear buffer with background color
        for pixel in self.pixel_buffer.chunks_mut(4) {
            pixel[0] = 40;  // R
            pixel[1] = 44;  // G
            pixel[2] = 52;  // B
            pixel[3] = 255; // A
        }

        // Render each chunk
        for chunk in world.active_chunks() {
            self.render_chunk_to_buffer(chunk, world);
        }

        // Render active debris on top of chunks
        let debris_list = world.get_active_debris();
        if !debris_list.is_empty() {
            log::debug!("Rendering {} active debris", debris_list.len());
        }

        for debris_data in &debris_list {
            self.render_debris_to_buffer(debris_data, world.materials());
        }
    }
    
    fn render_chunk_to_buffer(&mut self, chunk: &Chunk, world: &World) {
        // Calculate chunk position in texture
        // Center of texture is world origin
        let tex_origin_x = (Self::WORLD_TEXTURE_SIZE / 2) as i32 + chunk.x * CHUNK_SIZE as i32;
        let tex_origin_y = (Self::WORLD_TEXTURE_SIZE / 2) as i32 + chunk.y * CHUNK_SIZE as i32;

        let mut pixels_written = 0;

        for y in 0..CHUNK_SIZE {
            for x in 0..CHUNK_SIZE {
                let pixel = chunk.get_pixel(x, y);
                if pixel.material_id == 0 {
                    continue; // Skip air
                }

                let color = world.materials.get_color(pixel.material_id);

                let tex_x = tex_origin_x + x as i32;
                let tex_y = tex_origin_y + y as i32;

                // Bounds check
                if tex_x >= 0 && tex_x < Self::WORLD_TEXTURE_SIZE as i32
                    && tex_y >= 0 && tex_y < Self::WORLD_TEXTURE_SIZE as i32
                {
                    // Don't flip here - shader handles Y-flip
                    let idx = ((tex_y as u32 * Self::WORLD_TEXTURE_SIZE + tex_x as u32) * 4) as usize;

                    self.pixel_buffer[idx] = color[0];
                    self.pixel_buffer[idx + 1] = color[1];
                    self.pixel_buffer[idx + 2] = color[2];
                    self.pixel_buffer[idx + 3] = color[3];
                    pixels_written += 1;
                }
            }
        }

        if pixels_written > 0 {
            log::trace!("Chunk ({:2}, {:2}): rendered {} pixels at tex ({}, {})",
                       chunk.x, chunk.y, pixels_written, tex_origin_x, tex_origin_y);
        }
    }

    /// Get current camera zoom level
    pub fn camera_zoom(&self) -> f32 {
        self.camera.zoom
    }

    /// Update camera zoom level with delta and clamp to min/max
    pub fn update_zoom(&mut self, zoom_delta: f32, min_zoom: f32, max_zoom: f32) {
        self.camera.zoom *= zoom_delta;
        self.camera.zoom = self.camera.zoom.clamp(min_zoom, max_zoom);
    }

    /// Get window size
    pub fn window_size(&self) -> (u32, u32) {
        (self.size.width, self.size.height)
    }

    /// Render a single debris to the pixel buffer
    fn render_debris_to_buffer(
        &mut self,
        debris: &crate::physics::DebrisRenderData,
        materials: &crate::simulation::Materials,
    ) {
        // For each pixel in the debris
        for (local_pos, material_id) in &debris.pixels {
            // Apply rotation
            let rotated = Self::rotate_point(*local_pos, debris.rotation);

            // Translate to world position
            let world_x = (debris.position.x + rotated.x as f32).round() as i32;
            let world_y = (debris.position.y + rotated.y as f32).round() as i32;

            // Convert world coordinates to texture coordinates
            let tex_x = (Self::WORLD_TEXTURE_SIZE / 2) as i32 + world_x;
            let tex_y = (Self::WORLD_TEXTURE_SIZE / 2) as i32 + world_y;

            // Bounds check
            if tex_x >= 0 && tex_x < Self::WORLD_TEXTURE_SIZE as i32 &&
               tex_y >= 0 && tex_y < Self::WORLD_TEXTURE_SIZE as i32 {
                // Write pixel to buffer
                let idx = ((tex_y as u32 * Self::WORLD_TEXTURE_SIZE + tex_x as u32) * 4) as usize;
                let color = materials.get_color(*material_id);
                self.pixel_buffer[idx] = color[0];
                self.pixel_buffer[idx + 1] = color[1];
                self.pixel_buffer[idx + 2] = color[2];
                self.pixel_buffer[idx + 3] = color[3];
            }
        }
    }

    /// Rotate a point around origin
    fn rotate_point(point: glam::IVec2, angle: f32) -> glam::IVec2 {
        let cos = angle.cos();
        let sin = angle.sin();
        let x = point.x as f32 * cos - point.y as f32 * sin;
        let y = point.x as f32 * sin + point.y as f32 * cos;
        glam::IVec2::new(x.round() as i32, y.round() as i32)
    }

    /// Update temperature overlay texture with data from world
    pub fn update_temperature_overlay(&mut self, world: &World) {
        const TEMP_TEXTURE_SIZE: u32 = 40;
        const CELLS_PER_CHUNK: usize = 8; // 8x8 temperature grid per chunk

        // Create temperature data buffer (40x40 = 5x5 chunks × 8x8 cells)
        let mut temp_data = vec![20.0f32; (TEMP_TEXTURE_SIZE * TEMP_TEXTURE_SIZE) as usize];

        // Sample temperature from 5x5 chunks around player
        let player_chunk_x = (world.player.position.x / CHUNK_SIZE as f32).floor() as i32;
        let player_chunk_y = (world.player.position.y / CHUNK_SIZE as f32).floor() as i32;

        for cy in -2..=2 {
            for cx in -2..=2 {
                let chunk_x = player_chunk_x + cx;
                let chunk_y = player_chunk_y + cy;

                // Get temperature data for this chunk
                for cell_y in 0..CELLS_PER_CHUNK {
                    for cell_x in 0..CELLS_PER_CHUNK {
                        // World coordinates of this cell
                        let world_x = chunk_x * CHUNK_SIZE as i32 + (cell_x * CHUNK_SIZE / CELLS_PER_CHUNK) as i32;
                        let world_y = chunk_y * CHUNK_SIZE as i32 + (cell_y * CHUNK_SIZE / CELLS_PER_CHUNK) as i32;

                        // Texture coordinates (40x40)
                        let tex_x = ((cx + 2) * CELLS_PER_CHUNK as i32 + cell_x as i32) as usize;
                        let tex_y = ((cy + 2) * CELLS_PER_CHUNK as i32 + cell_y as i32) as usize;

                        // Get temperature at this world position
                        let temp = world.get_temperature_at_pixel(world_x, world_y);

                        let idx = tex_y * TEMP_TEXTURE_SIZE as usize + tex_x;
                        if idx < temp_data.len() {
                            temp_data[idx] = temp;
                        }
                    }
                }
            }
        }

        // Upload to GPU
        self.queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &self.temp_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            bytemuck::cast_slice(&temp_data),
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(TEMP_TEXTURE_SIZE * 4), // 4 bytes per f32
                rows_per_image: Some(TEMP_TEXTURE_SIZE),
            },
            wgpu::Extent3d {
                width: TEMP_TEXTURE_SIZE,
                height: TEMP_TEXTURE_SIZE,
                depth_or_array_layers: 1,
            },
        );
    }

    /// Toggle temperature overlay on/off
    pub fn toggle_temperature_overlay(&mut self) {
        self.overlay_enabled = !self.overlay_enabled;
        let enabled_value = if self.overlay_enabled { 1u32 } else { 0u32 };
        self.queue.write_buffer(
            &self.overlay_uniform_buffer,
            0,
            bytemuck::cast_slice(&[enabled_value, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32]),
        );
        log::info!("Temperature overlay: {}", if self.overlay_enabled { "ON" } else { "OFF" });
    }

    /// Check if temperature overlay is enabled
    pub fn is_temperature_overlay_enabled(&self) -> bool {
        self.overlay_enabled
    }
}
