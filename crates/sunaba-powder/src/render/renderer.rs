//! Simplified wgpu renderer for Powder Game demo
//!
//! A stripped-down renderer focused on pixel-perfect material display
//! without post-processing effects or complex overlays.

use anyhow::Result;
use wgpu::util::DeviceExt;
use winit::window::Window;

use sunaba_core::simulation::Materials;
use sunaba_core::world::{CHUNK_SIZE, Chunk, World};

use crate::ui::VisualizationMode;

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
    Vertex {
        position: [-1.0, -1.0],
        tex_coords: [0.0, 1.0],
    },
    Vertex {
        position: [1.0, -1.0],
        tex_coords: [1.0, 1.0],
    },
    Vertex {
        position: [1.0, 1.0],
        tex_coords: [1.0, 0.0],
    },
    Vertex {
        position: [-1.0, 1.0],
        tex_coords: [0.0, 0.0],
    },
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

/// Simplified renderer for Powder Game demo
pub struct Renderer {
    surface: wgpu::Surface<'static>,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    size: winit::dpi::PhysicalSize<u32>,

    render_pipeline: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,

    // World texture (stores all visible pixels)
    world_texture: wgpu::Texture,
    world_bind_group: wgpu::BindGroup,

    // Camera
    camera_buffer: wgpu::Buffer,
    camera_bind_group: wgpu::BindGroup,
    camera: CameraUniform,

    // Pixel buffer for CPU-side rendering
    pixel_buffer: Vec<u8>,

    /// World texture size in pixels
    world_texture_size: u32,

    /// Current visualization mode
    visualization_mode: VisualizationMode,
}

impl Renderer {
    pub async fn new(window: &Window, world_size: u32) -> Result<Self> {
        let size = window.inner_size();

        // Create instance
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });

        // Create surface
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
            .await?;

        // Create device and queue
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                label: Some("device"),
                memory_hints: wgpu::MemoryHints::default(),
                experimental_features: wgpu::ExperimentalFeatures::disabled(),
                trace: wgpu::Trace::Off,
            })
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
                width: world_size,
                height: world_size,
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
        let texture_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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

        // Camera uniform - center on world origin with zoom to fit
        let camera = CameraUniform {
            position: [0.0, 0.0],          // Center on world origin
            zoom: 2.0 / world_size as f32, // Fit entire world in view
            aspect: size.width as f32 / size.height as f32,
        };

        let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("camera_buffer"),
            contents: bytemuck::cast_slice(&[camera]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let camera_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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

        // Create shader
        let shader_source = include_str!("../../assets/shaders/powder.wgsl");
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("powder_shader"),
            source: wgpu::ShaderSource::Wgsl(shader_source.into()),
        });

        // Create pipeline
        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("render_pipeline_layout"),
                bind_group_layouts: &[&texture_bind_group_layout, &camera_bind_group_layout],
                push_constant_ranges: &[],
            });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("render_pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[Vertex::desc()],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
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
            cache: None,
        });

        // Create vertex and index buffers
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

        // Create pixel buffer
        let pixel_buffer = vec![0u8; (world_size * world_size * 4) as usize];

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
            world_bind_group,
            camera_buffer,
            camera_bind_group,
            camera,
            pixel_buffer,
            world_texture_size: world_size,
            visualization_mode: VisualizationMode::None,
        })
    }

    /// Resize the renderer
    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);

            // Update camera aspect ratio
            self.camera.aspect = new_size.width as f32 / new_size.height as f32;
            self.queue
                .write_buffer(&self.camera_buffer, 0, bytemuck::cast_slice(&[self.camera]));
        }
    }

    /// Get surface format
    pub fn surface_format(&self) -> wgpu::TextureFormat {
        self.config.format
    }

    /// Get window size
    pub fn size(&self) -> winit::dpi::PhysicalSize<u32> {
        self.size
    }

    /// Update camera position
    pub fn set_camera_position(&mut self, x: f32, y: f32) {
        self.camera.position = [x, y];
        self.queue
            .write_buffer(&self.camera_buffer, 0, bytemuck::cast_slice(&[self.camera]));
    }

    /// Update camera zoom
    pub fn set_camera_zoom(&mut self, zoom: f32) {
        self.camera.zoom = zoom;
        self.queue
            .write_buffer(&self.camera_buffer, 0, bytemuck::cast_slice(&[self.camera]));
    }

    /// Get camera position
    pub fn camera_position(&self) -> (f32, f32) {
        (self.camera.position[0], self.camera.position[1])
    }

    /// Get camera zoom
    pub fn camera_zoom(&self) -> f32 {
        self.camera.zoom
    }

    /// Set visualization mode
    pub fn set_visualization_mode(&mut self, mode: VisualizationMode) {
        self.visualization_mode = mode;
    }

    /// Get current visualization mode
    pub fn visualization_mode(&self) -> VisualizationMode {
        self.visualization_mode
    }

    /// Convert screen coordinates to world coordinates
    pub fn screen_to_world(&self, screen_x: f32, screen_y: f32) -> (i32, i32) {
        // Normalize to -1..1
        let ndc_x = (screen_x / self.size.width as f32) * 2.0 - 1.0;
        let ndc_y = -((screen_y / self.size.height as f32) * 2.0 - 1.0); // Flip Y

        // Apply camera transform
        let world_x = (ndc_x * self.camera.aspect / self.camera.zoom) + self.camera.position[0];
        let world_y = (ndc_y / self.camera.zoom) + self.camera.position[1];

        (world_x as i32, world_y as i32)
    }

    /// Update world texture from World state
    pub fn update_world_texture(&mut self, world: &World, materials: &Materials) {
        let world_size = self.world_texture_size as i32;
        let half_size = world_size / 2;

        // Clear pixel buffer with background color
        for pixel in self.pixel_buffer.chunks_exact_mut(4) {
            pixel[0] = 26; // Dark background
            pixel[1] = 26;
            pixel[2] = 38;
            pixel[3] = 255;
        }

        // Render chunks that are within our world bounds
        let chunks_per_side = world_size / CHUNK_SIZE as i32;
        let min_chunk = -chunks_per_side / 2;
        let max_chunk = chunks_per_side / 2;

        for chunk_y in min_chunk..max_chunk {
            for chunk_x in min_chunk..max_chunk {
                if let Some(chunk) = world.get_chunk(chunk_x, chunk_y) {
                    self.render_chunk_to_buffer(chunk, chunk_x, chunk_y, half_size, materials);
                }
            }
        }

        // Upload to GPU
        self.queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &self.world_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &self.pixel_buffer,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(self.world_texture_size * 4),
                rows_per_image: Some(self.world_texture_size),
            },
            wgpu::Extent3d {
                width: self.world_texture_size,
                height: self.world_texture_size,
                depth_or_array_layers: 1,
            },
        );
    }

    /// Render a single chunk to the pixel buffer
    fn render_chunk_to_buffer(
        &mut self,
        chunk: &Chunk,
        chunk_x: i32,
        chunk_y: i32,
        half_size: i32,
        materials: &Materials,
    ) {
        use super::visualization::get_visualization_overlay;

        let chunk_size = CHUNK_SIZE as i32;
        let world_size = self.world_texture_size as i32;
        let vis_mode = self.visualization_mode;

        for local_y in 0..chunk_size {
            for local_x in 0..chunk_size {
                let pixel = chunk.get_pixel(local_x as usize, local_y as usize);
                let material_id = pixel.material_id;

                // Get material color
                let mut color = materials.get(material_id).color;

                // Apply visualization overlay if active
                if vis_mode != VisualizationMode::None {
                    let pressure = chunk.get_pressure_at(local_x as usize, local_y as usize);
                    let temperature =
                        chunk.temperature[(local_y as usize / 8) * 8 + (local_x as usize / 8)];
                    let light_level =
                        chunk.light_levels[(local_y as usize) * CHUNK_SIZE + (local_x as usize)];

                    if let Some(overlay) = get_visualization_overlay(
                        vis_mode,
                        material_id,
                        pressure,
                        temperature,
                        light_level,
                    ) {
                        // Blend overlay with material color using alpha
                        let alpha = overlay[3] as f32 / 255.0;
                        color[0] =
                            ((overlay[0] as f32 * alpha) + (color[0] as f32 * (1.0 - alpha))) as u8;
                        color[1] =
                            ((overlay[1] as f32 * alpha) + (color[1] as f32 * (1.0 - alpha))) as u8;
                        color[2] =
                            ((overlay[2] as f32 * alpha) + (color[2] as f32 * (1.0 - alpha))) as u8;
                    }
                }

                // Convert to texture coordinates (centered at half_size)
                let world_x = chunk_x * chunk_size + local_x;
                let world_y = chunk_y * chunk_size + local_y;

                let tex_x = world_x + half_size;
                let tex_y = world_y + half_size;

                // Bounds check
                if tex_x >= 0 && tex_x < world_size && tex_y >= 0 && tex_y < world_size {
                    let idx = ((tex_y * world_size + tex_x) * 4) as usize;
                    if idx + 3 < self.pixel_buffer.len() {
                        self.pixel_buffer[idx] = color[0];
                        self.pixel_buffer[idx + 1] = color[1];
                        self.pixel_buffer[idx + 2] = color[2];
                        self.pixel_buffer[idx + 3] = color[3];
                    }
                }
            }
        }
    }

    /// Begin frame rendering, returns surface texture
    pub fn begin_frame(&mut self) -> Result<wgpu::SurfaceTexture> {
        let output = self.surface.get_current_texture()?;
        Ok(output)
    }

    /// Render the world (call after begin_frame, before egui)
    pub fn render_world(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
    ) -> Result<()> {
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("world_render_pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
                resolve_target: None,
                depth_slice: None,
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
            occlusion_query_set: None,
            timestamp_writes: None,
        });

        render_pass.set_pipeline(&self.render_pipeline);
        render_pass.set_bind_group(0, &self.world_bind_group, &[]);
        render_pass.set_bind_group(1, &self.camera_bind_group, &[]);
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
        render_pass.draw_indexed(0..QUAD_INDICES.len() as u32, 0, 0..1);

        Ok(())
    }

    /// End frame and present
    pub fn end_frame(&self, output: wgpu::SurfaceTexture) {
        output.present();
    }

    /// Submit command buffer
    pub fn submit(&self, encoder: wgpu::CommandEncoder) {
        self.queue.submit(std::iter::once(encoder.finish()));
    }
}
