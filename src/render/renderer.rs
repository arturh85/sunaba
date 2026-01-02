//! wgpu-based renderer for pixel world

use anyhow::Result;
use instant::Instant;
use std::iter;
use wgpu::util::DeviceExt;
use winit::window::Window;

use crate::render::sprite::PlayerSprite;
use crate::world::{Chunk, World, CHUNK_SIZE};

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

/// Timing breakdown for render phases (in milliseconds)
#[derive(Clone, Copy, Debug, Default)]
pub struct RenderTiming {
    pub pixel_buffer_ms: f32,
    pub gpu_upload_ms: f32,
    pub acquire_ms: f32,
    pub egui_ms: f32,
    pub present_ms: f32,
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

    // Debug overlays (temperature and light)
    temp_texture: wgpu::Texture,
    #[allow(dead_code)]
    temp_texture_view: wgpu::TextureView,
    #[allow(dead_code)]
    temp_sampler: wgpu::Sampler,
    light_texture: wgpu::Texture,
    #[allow(dead_code)]
    light_texture_view: wgpu::TextureView,
    #[allow(dead_code)]
    light_sampler: wgpu::Sampler,
    overlay_uniform_buffer: wgpu::Buffer,
    overlay_bind_group: wgpu::BindGroup,
    overlay_type: u32, // 0=none, 1=temperature, 2=light

    // Active chunks debug overlay
    show_active_chunks: bool,

    // Texture origin tracking for dynamic camera-centered rendering
    /// World coordinate at texture pixel [0, 0]
    texture_origin: glam::Vec2,
    /// GPU buffer for texture_origin uniform (passed to shader via camera bind group)
    texture_origin_buffer: wgpu::Buffer,
    /// Track which chunks are currently rendered in the texture
    rendered_chunks: std::collections::HashSet<glam::IVec2>,
    /// Flag indicating texture needs full rebuffer
    needs_full_rebuffer: bool,

    // Render dirty tracking for incremental updates
    /// Chunks that need re-rendering this frame
    render_dirty_chunks: std::collections::HashSet<glam::IVec2>,
    /// Previous player position (to clear old sprite location)
    prev_player_pos: glam::Vec2,

    /// Player sprite for animated rendering
    player_sprite: PlayerSprite,
}

impl Renderer {
    const WORLD_TEXTURE_SIZE: u32 = 2048; // 32x32 chunks visible (~16 MB texture)

    pub async fn new(window: &Window) -> Result<Self> {
        let size = window.inner_size();

        // Create instance
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
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
        // wgpu 27+ returns Result instead of Option
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await?;

        // Create device and queue
        // In wgpu 27+, max_inter_stage_shader_components was removed (fixed in v23)
        let limits = wgpu::Limits::default();

        // wgpu 27+ request_device takes only descriptor, no separate trace parameter
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                required_features: wgpu::Features::empty(),
                required_limits: limits,
                label: Some("device"),
                memory_hints: wgpu::MemoryHints::default(),
                experimental_features: wgpu::ExperimentalFeatures::disabled(), // New in wgpu 27
                trace: wgpu::Trace::Off,                                       // New in wgpu 27
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

        // Texture origin tracking for dynamic camera-centered rendering
        // Start with texture centered on world origin
        let texture_origin = glam::Vec2::new(
            -(Self::WORLD_TEXTURE_SIZE as f32 / 2.0),
            -(Self::WORLD_TEXTURE_SIZE as f32 / 2.0),
        );

        let texture_origin_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("texture_origin_buffer"),
            contents: bytemuck::cast_slice(&[texture_origin.x, texture_origin.y]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let camera_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("camera_bind_group_layout"),
                entries: &[
                    // Camera uniform (binding 0)
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    // Texture origin uniform (binding 1)
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
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

        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("camera_bind_group"),
            layout: &camera_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: camera_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: texture_origin_buffer.as_entire_binding(),
                },
            ],
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

        // Light overlay texture (40x40 for 5x5 chunks × 8x8 downsampled light grid)
        const LIGHT_TEXTURE_SIZE: u32 = 40;
        let light_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("light_texture"),
            size: wgpu::Extent3d {
                width: LIGHT_TEXTURE_SIZE,
                height: LIGHT_TEXTURE_SIZE,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R32Float,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        let light_texture_view = light_texture.create_view(&wgpu::TextureViewDescriptor::default());

        let light_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        // Overlay uniform buffer (overlay type: 0=none, 1=temperature, 2=light)
        let overlay_uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("overlay_uniform_buffer"),
            size: 32, // u32 (4) + vec3<u32> padding (16 due to alignment) + struct padding (12) = 32 bytes
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Debug overlay bind group layout (temperature and light)
        let overlay_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("overlay_bind_group_layout"),
                entries: &[
                    // Binding 0: Temperature texture
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
                    // Binding 1: Temperature sampler
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::NonFiltering),
                        count: None,
                    },
                    // Binding 2: Light texture
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: false },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    // Binding 3: Light sampler
                    wgpu::BindGroupLayoutEntry {
                        binding: 3,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::NonFiltering),
                        count: None,
                    },
                    // Binding 4: Overlay uniform (overlay_type)
                    wgpu::BindGroupLayoutEntry {
                        binding: 4,
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

        let overlay_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("overlay_bind_group"),
            layout: &overlay_bind_group_layout,
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
                    resource: wgpu::BindingResource::TextureView(&light_texture_view),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::Sampler(&light_sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
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
        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("render_pipeline_layout"),
                bind_group_layouts: &[
                    &texture_bind_group_layout,
                    &camera_bind_group_layout,
                    &overlay_bind_group_layout,
                ],
                push_constant_ranges: &[],
            });

        // Render pipeline
        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("render_pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"), // wgpu 27+ expects Option<&str>
                buffers: &[Vertex::desc()],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"), // wgpu 27+ expects Option<&str>
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
            cache: None, // New field in wgpu 22+
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
        let pixel_buffer =
            vec![0u8; (Self::WORLD_TEXTURE_SIZE * Self::WORLD_TEXTURE_SIZE * 4) as usize];

        // Initialize egui renderer
        // egui-wgpu 0.33+ takes 3 arguments: device, format, RendererOptions
        let egui_renderer = egui_wgpu::Renderer::new(
            &device,
            config.format,
            egui_wgpu::RendererOptions::default(),
        );

        // Initialize overlay as disabled (32 bytes: enabled + padding)
        queue.write_buffer(
            &overlay_uniform_buffer,
            0,
            bytemuck::cast_slice(&[0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32]),
        );

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
            light_texture,
            light_texture_view,
            light_sampler,
            overlay_uniform_buffer,
            overlay_bind_group,
            overlay_type: 0, // 0 = no overlay
            show_active_chunks: false,
            texture_origin,
            texture_origin_buffer,
            rendered_chunks: std::collections::HashSet::new(),
            needs_full_rebuffer: true, // Start with full rebuffer
            render_dirty_chunks: std::collections::HashSet::new(),
            prev_player_pos: glam::Vec2::ZERO,
            player_sprite: PlayerSprite::new(include_bytes!("../entity/player_sprite.png"))
                .expect("Failed to load player sprite"),
        })
    }

    /// Get render stats for debugging (dirty_chunks, rendered_total)
    pub fn get_render_stats(&self) -> (usize, usize) {
        (self.render_dirty_chunks.len(), self.rendered_chunks.len())
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        if width > 0 && height > 0 {
            self.size = winit::dpi::PhysicalSize::new(width, height);
            self.config.width = width;
            self.config.height = height;
            self.surface.configure(&self.device, &self.config);

            // Update camera aspect ratio
            self.camera.aspect = width as f32 / height as f32;
            self.queue
                .write_buffer(&self.camera_buffer, 0, bytemuck::cast_slice(&[self.camera]));
        }
    }

    /// Update player sprite animation state and timing
    pub fn update_player_sprite(&mut self, velocity: glam::Vec2, is_mining: bool, delta_time: f32) {
        self.player_sprite
            .update_state(velocity.x, velocity.y, is_mining);
        self.player_sprite.update(delta_time);
    }

    pub fn render(
        &mut self,
        world: &mut World,
        egui_ctx: &egui::Context,
        textures_delta: egui::TexturesDelta,
        shapes: Vec<egui::epaint::ClippedShape>,
    ) -> Result<RenderTiming> {
        let mut timing = RenderTiming::default();

        log::trace!(
            "Render frame: camera pos=({:.1}, {:.1}), zoom={:.2}",
            self.camera.position[0],
            self.camera.position[1],
            self.camera.zoom
        );

        // Update texture origin to follow camera (for infinite world rendering)
        let camera_pos = glam::Vec2::new(self.camera.position[0], self.camera.position[1]);
        self.update_texture_origin(camera_pos);

        // Track if we need full upload (before update clears the flag)
        let needs_full_upload = self.needs_full_rebuffer;

        // Time: Update pixel buffer from world chunks
        let t0 = Instant::now();
        self.update_pixel_buffer(world);
        timing.pixel_buffer_ms = t0.elapsed().as_secs_f32() * 1000.0;

        // Time: Upload to GPU
        let t1 = Instant::now();
        if needs_full_upload {
            // Full texture upload after rebuffer
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
                    bytes_per_row: Some(Self::WORLD_TEXTURE_SIZE * 4),
                    rows_per_image: Some(Self::WORLD_TEXTURE_SIZE),
                },
                wgpu::Extent3d {
                    width: Self::WORLD_TEXTURE_SIZE,
                    height: Self::WORLD_TEXTURE_SIZE,
                    depth_or_array_layers: 1,
                },
            );
        } else {
            // Incremental upload: only upload dirty chunks
            self.upload_dirty_chunks();
        }
        timing.gpu_upload_ms = t1.elapsed().as_secs_f32() * 1000.0;

        // Update camera position to follow player
        self.camera.position = [world.player.position.x, world.player.position.y];
        self.queue
            .write_buffer(&self.camera_buffer, 0, bytemuck::cast_slice(&[self.camera]));

        // Get output texture (this can block waiting for GPU)
        let t_swap = Instant::now();
        let output = self.surface.get_current_texture()?;
        timing.acquire_ms = t_swap.elapsed().as_secs_f32() * 1000.0;

        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("render_encoder"),
            });

        // Time: egui preparation and rendering
        let t2 = Instant::now();

        // Update egui textures
        for (id, image_delta) in &textures_delta.set {
            self.egui_renderer
                .update_texture(&self.device, &self.queue, *id, image_delta);
        }

        // Prepare egui primitives
        let screen_descriptor = egui_wgpu::ScreenDescriptor {
            size_in_pixels: [self.config.width, self.config.height],
            pixels_per_point: 1.0,
        };

        let primitives = egui_ctx.tessellate(shapes, 1.0);
        self.egui_renderer.update_buffers(
            &self.device,
            &self.queue,
            &mut encoder,
            &primitives,
            &screen_descriptor,
        );

        // Render world
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("world_render_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    depth_slice: None, // New in wgpu 27
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
            render_pass.set_bind_group(2, &self.overlay_bind_group, &[]);
            render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
            render_pass.draw_indexed(0..6, 0, 0..1);
        }

        // Render egui UI
        {
            let render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("egui_render_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    depth_slice: None, // New in wgpu 27
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
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

        // Free egui textures
        for id in &textures_delta.free {
            self.egui_renderer.free_texture(id);
        }
        timing.egui_ms = t2.elapsed().as_secs_f32() * 1000.0;

        // Time: submit and present
        let t3 = Instant::now();
        self.queue.submit(iter::once(encoder.finish()));
        output.present();
        timing.present_ms = t3.elapsed().as_secs_f32() * 1000.0;

        Ok(timing)
    }

    fn update_pixel_buffer(&mut self, world: &mut World) {
        // If texture origin changed, need to fully rebuffer
        if self.needs_full_rebuffer {
            // Clear entire buffer to background
            for pixel in self.pixel_buffer.chunks_mut(4) {
                pixel[0] = 40; // R
                pixel[1] = 44; // G
                pixel[2] = 52; // B
                pixel[3] = 255; // A
            }

            // Get visible chunk range for new texture origin
            let (min_chunk, max_chunk) = self.get_visible_chunk_range();

            log::debug!(
                "Full rebuffer: rendering chunks from {:?} to {:?}",
                min_chunk,
                max_chunk
            );

            // Render all visible chunks
            for cy in min_chunk.y..=max_chunk.y {
                for cx in min_chunk.x..=max_chunk.x {
                    let chunk_pos = glam::IVec2::new(cx, cy);
                    if let Some(chunk) = world.chunks.get(&chunk_pos) {
                        self.render_chunk_to_buffer(chunk, world);
                    }
                }
            }

            self.needs_full_rebuffer = false;
            self.rendered_chunks.clear();

            // Track newly rendered chunks and clear their dirty_rect
            for cy in min_chunk.y..=max_chunk.y {
                for cx in min_chunk.x..=max_chunk.x {
                    let chunk_pos = glam::IVec2::new(cx, cy);
                    self.rendered_chunks.insert(chunk_pos);
                    // Clear dirty_rect after rendering
                    if let Some(chunk) = world.chunks.get_mut(&chunk_pos) {
                        chunk.clear_dirty_rect();
                    }
                }
            }
        } else {
            // Incremental update: only render dirty chunks
            self.collect_render_dirty_chunks(world);

            // Clear only dirty chunk regions (not entire buffer)
            self.clear_dirty_regions();

            // Render only dirty chunks
            let dirty_chunks: Vec<glam::IVec2> = self.render_dirty_chunks.iter().copied().collect();
            for chunk_pos in &dirty_chunks {
                if let Some(chunk) = world.chunks.get(chunk_pos) {
                    self.render_chunk_to_buffer(chunk, world);
                }
                // Mark chunk as rendered
                self.rendered_chunks.insert(*chunk_pos);
            }

            // Clear dirty_rect for rendered chunks - this is the key optimization!
            // Next frame, only chunks that changed AFTER rendering will be dirty
            for chunk_pos in &dirty_chunks {
                if let Some(chunk) = world.chunks.get_mut(chunk_pos) {
                    chunk.clear_dirty_rect();
                }
            }
        }

        // Render active debris on top of chunks
        let debris_list = world.get_active_debris();
        if !debris_list.is_empty() {
            log::debug!("Rendering {} active debris", debris_list.len());
        }

        for debris_data in &debris_list {
            self.render_debris_to_buffer(debris_data, world.materials());
        }

        // Render creatures on top of debris
        let creature_data = world.get_creature_render_data();
        for creature in &creature_data {
            self.render_creature_to_buffer(creature);
        }

        // Draw player sprite
        // Convert world coordinates to texture coordinates using dynamic texture origin
        let player_world_x = world.player.position.x as i32;
        let player_world_y = world.player.position.y as i32;

        let player_x = player_world_x - self.texture_origin.x as i32;
        let player_y = player_world_y - self.texture_origin.y as i32;

        let half_width = (self.player_sprite.display_width / 2) as i32;
        let half_height = (self.player_sprite.display_height / 2) as i32;

        // Determine if we need to flip horizontally (for left-facing animations)
        let flip_h = !self.player_sprite.facing_right;

        // Draw sprite centered on player position
        for local_y in 0..self.player_sprite.display_height as i32 {
            for local_x in 0..self.player_sprite.display_width as i32 {
                // Sample from sprite (handles scaling and flipping)
                if let Some(pixel) = self.player_sprite.sample_pixel(local_x, local_y, flip_h) {
                    // Calculate destination in texture buffer
                    let px = player_x + local_x - half_width;
                    let py = player_y + local_y - half_height;

                    // Bounds check
                    if px >= 0
                        && px < Self::WORLD_TEXTURE_SIZE as i32
                        && py >= 0
                        && py < Self::WORLD_TEXTURE_SIZE as i32
                    {
                        let idx = ((py as u32 * Self::WORLD_TEXTURE_SIZE + px as u32) * 4) as usize;
                        if idx + 3 < self.pixel_buffer.len() {
                            self.pixel_buffer[idx..idx + 4].copy_from_slice(&pixel);
                        }
                    }
                }
            }
        }
    }

    /// Calculate which chunks should be visible in current texture window
    fn get_visible_chunk_range(&self) -> (glam::IVec2, glam::IVec2) {
        let min_chunk_x = (self.texture_origin.x / CHUNK_SIZE as f32).floor() as i32;
        let min_chunk_y = (self.texture_origin.y / CHUNK_SIZE as f32).floor() as i32;

        let chunks_wide = (Self::WORLD_TEXTURE_SIZE / CHUNK_SIZE as u32) as i32;
        let chunks_tall = (Self::WORLD_TEXTURE_SIZE / CHUNK_SIZE as u32) as i32;

        let max_chunk_x = min_chunk_x + chunks_wide;
        let max_chunk_y = min_chunk_y + chunks_tall;

        (
            glam::IVec2::new(min_chunk_x, min_chunk_y),
            glam::IVec2::new(max_chunk_x, max_chunk_y),
        )
    }

    /// Collect chunks that need re-rendering this frame
    fn collect_render_dirty_chunks(&mut self, world: &World) {
        self.render_dirty_chunks.clear();

        let (min_chunk, max_chunk) = self.get_visible_chunk_range();

        for cy in min_chunk.y..=max_chunk.y {
            for cx in min_chunk.x..=max_chunk.x {
                let pos = glam::IVec2::new(cx, cy);

                if let Some(chunk) = world.chunks.get(&pos) {
                    // Re-render if: newly visible OR dirty from simulation
                    if !self.rendered_chunks.contains(&pos) || chunk.dirty_rect.is_some() {
                        self.render_dirty_chunks.insert(pos);
                    }
                }
            }
        }

        // Add chunks affected by player movement
        self.add_player_affected_chunks(world);
    }

    /// Add chunks that need re-rendering due to player movement
    fn add_player_affected_chunks(&mut self, world: &World) {
        // Use sprite display size (larger than collision box)
        let sprite_width = self.player_sprite.display_width as f32;
        let sprite_height = self.player_sprite.display_height as f32;

        // Current player position - get affected chunks
        let player_pos = world.player.position;
        let curr_chunks = self.get_chunks_for_rect(player_pos, sprite_width, sprite_height);

        // Previous player position - need to clear old sprite
        let prev_chunks =
            self.get_chunks_for_rect(self.prev_player_pos, sprite_width, sprite_height);

        for chunk_pos in curr_chunks.into_iter().chain(prev_chunks.into_iter()) {
            self.render_dirty_chunks.insert(chunk_pos);
        }

        // Update prev position for next frame
        self.prev_player_pos = player_pos;
    }

    /// Get chunk positions that overlap with a rectangle
    fn get_chunks_for_rect(&self, center: glam::Vec2, width: f32, height: f32) -> Vec<glam::IVec2> {
        let half_w = width / 2.0;
        let half_h = height / 2.0;

        let min_x = ((center.x - half_w) / CHUNK_SIZE as f32).floor() as i32;
        let max_x = ((center.x + half_w) / CHUNK_SIZE as f32).floor() as i32;
        let min_y = ((center.y - half_h) / CHUNK_SIZE as f32).floor() as i32;
        let max_y = ((center.y + half_h) / CHUNK_SIZE as f32).floor() as i32;

        let mut chunks = Vec::new();
        for cy in min_y..=max_y {
            for cx in min_x..=max_x {
                chunks.push(glam::IVec2::new(cx, cy));
            }
        }
        chunks
    }

    /// Clear only the regions in pixel buffer for dirty chunks
    fn clear_dirty_regions(&mut self) {
        const BG: [u8; 4] = [40, 44, 52, 255];

        for &chunk_pos in &self.render_dirty_chunks {
            // Calculate chunk's texture coordinates
            let world_x = chunk_pos.x * CHUNK_SIZE as i32;
            let world_y = chunk_pos.y * CHUNK_SIZE as i32;
            let tx = world_x - self.texture_origin.x as i32;
            let ty = world_y - self.texture_origin.y as i32;

            // Skip if outside texture bounds
            if tx < 0
                || ty < 0
                || tx >= Self::WORLD_TEXTURE_SIZE as i32
                || ty >= Self::WORLD_TEXTURE_SIZE as i32
            {
                continue;
            }

            // Clear this chunk's region
            for y in 0..CHUNK_SIZE {
                let py = ty + y as i32;
                if py < 0 || py >= Self::WORLD_TEXTURE_SIZE as i32 {
                    continue;
                }

                for x in 0..CHUNK_SIZE {
                    let px = tx + x as i32;
                    if px < 0 || px >= Self::WORLD_TEXTURE_SIZE as i32 {
                        continue;
                    }

                    let idx = ((py as u32 * Self::WORLD_TEXTURE_SIZE + px as u32) * 4) as usize;
                    if idx + 3 < self.pixel_buffer.len() {
                        self.pixel_buffer[idx..idx + 4].copy_from_slice(&BG);
                    }
                }
            }
        }
    }

    /// Convert chunk position to texture pixel coordinates
    fn chunk_to_texture_pixel(&self, chunk_pos: glam::IVec2) -> (i32, i32) {
        let world_x = chunk_pos.x * CHUNK_SIZE as i32;
        let world_y = chunk_pos.y * CHUNK_SIZE as i32;
        let tx = world_x - self.texture_origin.x as i32;
        let ty = world_y - self.texture_origin.y as i32;
        (tx, ty)
    }

    /// Extract a chunk's pixel data from the main buffer for GPU upload
    fn extract_chunk_pixels(&self, tx: i32, ty: i32) -> Vec<u8> {
        let mut chunk_data = vec![0u8; CHUNK_SIZE * CHUNK_SIZE * 4];

        for y in 0..CHUNK_SIZE {
            let src_y = ty + y as i32;
            if src_y < 0 || src_y >= Self::WORLD_TEXTURE_SIZE as i32 {
                continue;
            }

            for x in 0..CHUNK_SIZE {
                let src_x = tx + x as i32;
                if src_x < 0 || src_x >= Self::WORLD_TEXTURE_SIZE as i32 {
                    continue;
                }

                let src_idx =
                    ((src_y as u32 * Self::WORLD_TEXTURE_SIZE + src_x as u32) * 4) as usize;
                let dst_idx = (y * CHUNK_SIZE + x) * 4;

                if src_idx + 3 < self.pixel_buffer.len() {
                    chunk_data[dst_idx..dst_idx + 4]
                        .copy_from_slice(&self.pixel_buffer[src_idx..src_idx + 4]);
                }
            }
        }

        chunk_data
    }

    /// Upload only dirty chunks to GPU texture
    fn upload_dirty_chunks(&self) {
        for &chunk_pos in &self.render_dirty_chunks {
            let (tx, ty) = self.chunk_to_texture_pixel(chunk_pos);

            // Skip if outside texture bounds
            if tx < 0
                || ty < 0
                || tx + CHUNK_SIZE as i32 > Self::WORLD_TEXTURE_SIZE as i32
                || ty + CHUNK_SIZE as i32 > Self::WORLD_TEXTURE_SIZE as i32
            {
                continue;
            }

            // Extract chunk pixels and upload to GPU
            let chunk_data = self.extract_chunk_pixels(tx, ty);

            self.queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: &self.world_texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d {
                        x: tx as u32,
                        y: ty as u32,
                        z: 0,
                    },
                    aspect: wgpu::TextureAspect::All,
                },
                &chunk_data,
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(CHUNK_SIZE as u32 * 4),
                    rows_per_image: Some(CHUNK_SIZE as u32),
                },
                wgpu::Extent3d {
                    width: CHUNK_SIZE as u32,
                    height: CHUNK_SIZE as u32,
                    depth_or_array_layers: 1,
                },
            );
        }
    }

    /// Update texture origin to follow camera position
    /// This moves the texture window to keep the camera centered
    pub fn update_texture_origin(&mut self, camera_pos: glam::Vec2) {
        // Calculate desired texture origin to keep camera centered
        let desired_origin = glam::Vec2::new(
            camera_pos.x - (Self::WORLD_TEXTURE_SIZE as f32 / 2.0),
            camera_pos.y - (Self::WORLD_TEXTURE_SIZE as f32 / 2.0),
        );

        // Only update if moved significantly (one chunk = 64 pixels)
        const UPDATE_THRESHOLD: f32 = CHUNK_SIZE as f32;
        let delta = (desired_origin - self.texture_origin).length();

        if delta > UPDATE_THRESHOLD {
            // Texture origin changed - need to rebuffer
            let old_origin = self.texture_origin;
            self.texture_origin = desired_origin;

            // Update GPU buffer with new origin
            self.queue.write_buffer(
                &self.texture_origin_buffer,
                0,
                bytemuck::cast_slice(&[desired_origin.x, desired_origin.y]),
            );

            // Mark texture as needing full rebuffer
            self.needs_full_rebuffer = true;

            log::debug!(
                "Texture origin updated: {:?} -> {:?} (delta: {:.1})",
                old_origin,
                desired_origin,
                delta
            );
        }
    }

    fn render_chunk_to_buffer(&mut self, chunk: &Chunk, world: &World) {
        // Calculate chunk position in texture using dynamic texture origin
        let world_x = chunk.x * CHUNK_SIZE as i32;
        let world_y = chunk.y * CHUNK_SIZE as i32;

        // Use dynamic texture origin
        let tex_origin_x = world_x - self.texture_origin.x as i32;
        let tex_origin_y = world_y - self.texture_origin.y as i32;

        // Skip chunk if completely outside texture bounds (early return optimization)
        if tex_origin_x + (CHUNK_SIZE as i32) < 0
            || tex_origin_y + (CHUNK_SIZE as i32) < 0
            || tex_origin_x >= (Self::WORLD_TEXTURE_SIZE as i32)
            || tex_origin_y >= (Self::WORLD_TEXTURE_SIZE as i32)
        {
            return; // Chunk not visible in current texture window
        }

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
                if tex_x >= 0
                    && tex_x < Self::WORLD_TEXTURE_SIZE as i32
                    && tex_y >= 0
                    && tex_y < Self::WORLD_TEXTURE_SIZE as i32
                {
                    // Don't flip here - shader handles Y-flip
                    let idx =
                        ((tex_y as u32 * Self::WORLD_TEXTURE_SIZE + tex_x as u32) * 4) as usize;

                    self.pixel_buffer[idx] = color[0];
                    self.pixel_buffer[idx + 1] = color[1];
                    self.pixel_buffer[idx + 2] = color[2];
                    self.pixel_buffer[idx + 3] = color[3];
                    pixels_written += 1;
                }
            }
        }

        if pixels_written > 0 {
            log::trace!(
                "Chunk ({:2}, {:2}): rendered {} pixels at tex ({}, {})",
                chunk.x,
                chunk.y,
                pixels_written,
                tex_origin_x,
                tex_origin_y
            );
        }
    }

    /// Get current camera zoom level
    pub fn camera_zoom(&self) -> f32 {
        self.camera.zoom
    }

    /// Get current camera position
    pub fn camera_position(&self) -> glam::Vec2 {
        glam::Vec2::new(self.camera.position[0], self.camera.position[1])
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

            // Convert world coordinates to texture coordinates using dynamic texture origin
            let tex_x = world_x - self.texture_origin.x as i32;
            let tex_y = world_y - self.texture_origin.y as i32;

            // Bounds check
            if tex_x >= 0
                && tex_x < Self::WORLD_TEXTURE_SIZE as i32
                && tex_y >= 0
                && tex_y < Self::WORLD_TEXTURE_SIZE as i32
            {
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

    /// Render a single creature to the pixel buffer
    fn render_creature_to_buffer(&mut self, creature: &crate::creature::CreatureRenderData) {
        for body_part in &creature.body_parts {
            self.render_filled_circle(body_part.position, body_part.radius, body_part.color);
        }
    }

    /// Render a filled circle at world position
    fn render_filled_circle(&mut self, center: glam::Vec2, radius: f32, color: [u8; 4]) {
        let radius_i = radius.ceil() as i32;
        let radius_sq = radius * radius;

        // Convert center to texture coordinates
        let center_tex_x = center.x as i32 - self.texture_origin.x as i32;
        let center_tex_y = center.y as i32 - self.texture_origin.y as i32;

        // Iterate over bounding box of circle
        for dy in -radius_i..=radius_i {
            for dx in -radius_i..=radius_i {
                // Check if pixel is within circle
                let dist_sq = (dx * dx + dy * dy) as f32;
                if dist_sq <= radius_sq {
                    let tex_x = center_tex_x + dx;
                    let tex_y = center_tex_y + dy;

                    // Bounds check
                    if tex_x >= 0
                        && tex_x < Self::WORLD_TEXTURE_SIZE as i32
                        && tex_y >= 0
                        && tex_y < Self::WORLD_TEXTURE_SIZE as i32
                    {
                        let idx =
                            ((tex_y as u32 * Self::WORLD_TEXTURE_SIZE + tex_x as u32) * 4) as usize;
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
                        let world_x = chunk_x * CHUNK_SIZE as i32
                            + (cell_x * CHUNK_SIZE / CELLS_PER_CHUNK) as i32;
                        let world_y = chunk_y * CHUNK_SIZE as i32
                            + (cell_y * CHUNK_SIZE / CELLS_PER_CHUNK) as i32;

                        // Texture coordinates (40x40)
                        // Linear mapping to match shader (no Y-flip needed)
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
            wgpu::TexelCopyTextureInfo {
                texture: &self.temp_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            bytemuck::cast_slice(&temp_data),
            wgpu::TexelCopyBufferLayout {
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

    /// Cycle debug overlay: none → temperature → light → none
    /// T key cycles temperature, V key cycles light
    pub fn cycle_overlay(&mut self) {
        self.overlay_type = (self.overlay_type + 1) % 3;
        self.queue.write_buffer(
            &self.overlay_uniform_buffer,
            0,
            bytemuck::cast_slice(&[self.overlay_type, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32]),
        );
        let mode = match self.overlay_type {
            0 => "OFF",
            1 => "TEMPERATURE",
            2 => "LIGHT",
            _ => "UNKNOWN",
        };
        log::info!("Debug overlay: {}", mode);
    }

    /// Toggle temperature overlay (for T key)
    pub fn toggle_temperature_overlay(&mut self) {
        self.overlay_type = if self.overlay_type == 1 { 0 } else { 1 };
        self.queue.write_buffer(
            &self.overlay_uniform_buffer,
            0,
            bytemuck::cast_slice(&[self.overlay_type, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32]),
        );
        log::info!(
            "Temperature overlay: {}",
            if self.overlay_type == 1 { "ON" } else { "OFF" }
        );
    }

    /// Toggle light overlay (for V key)
    pub fn toggle_light_overlay(&mut self) {
        self.overlay_type = if self.overlay_type == 2 { 0 } else { 2 };
        self.queue.write_buffer(
            &self.overlay_uniform_buffer,
            0,
            bytemuck::cast_slice(&[self.overlay_type, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32]),
        );
        log::info!(
            "Light overlay: {}",
            if self.overlay_type == 2 { "ON" } else { "OFF" }
        );
    }

    /// Check if temperature overlay is enabled
    pub fn is_temperature_overlay_enabled(&self) -> bool {
        self.overlay_type == 1
    }

    /// Check if light overlay is enabled
    pub fn is_light_overlay_enabled(&self) -> bool {
        self.overlay_type == 2
    }

    /// Toggle active chunks debug overlay (F2)
    pub fn toggle_active_chunks_overlay(&mut self) {
        self.show_active_chunks = !self.show_active_chunks;
        log::info!(
            "Active chunks overlay: {}",
            if self.show_active_chunks { "ON" } else { "OFF" }
        );
    }

    /// Check if active chunks overlay is enabled
    pub fn is_active_chunks_overlay_enabled(&self) -> bool {
        self.show_active_chunks
    }

    /// Update light overlay texture with data from world
    pub fn update_light_overlay(&mut self, world: &World) {
        const LIGHT_TEXTURE_SIZE: u32 = 40;
        const SAMPLES_PER_CHUNK: usize = 8; // 8x8 downsampled light grid per chunk

        // Create light data buffer (40x40 = 5x5 chunks × 8x8 samples)
        let mut light_data = vec![0.0f32; (LIGHT_TEXTURE_SIZE * LIGHT_TEXTURE_SIZE) as usize];

        // Sample light from 5x5 chunks around player
        let player_chunk_x = (world.player.position.x / CHUNK_SIZE as f32).floor() as i32;
        let player_chunk_y = (world.player.position.y / CHUNK_SIZE as f32).floor() as i32;

        for cy in -2..=2 {
            for cx in -2..=2 {
                let chunk_x = player_chunk_x + cx;
                let chunk_y = player_chunk_y + cy;

                // Get light data for this chunk (8x8 downsampled)
                for sample_y in 0..SAMPLES_PER_CHUNK {
                    for sample_x in 0..SAMPLES_PER_CHUNK {
                        // World coordinates of this sample (center of 8x8 pixel block)
                        let world_x = chunk_x * CHUNK_SIZE as i32
                            + (sample_x * CHUNK_SIZE / SAMPLES_PER_CHUNK
                                + CHUNK_SIZE / (SAMPLES_PER_CHUNK * 2))
                                as i32;
                        let world_y = chunk_y * CHUNK_SIZE as i32
                            + (sample_y * CHUNK_SIZE / SAMPLES_PER_CHUNK
                                + CHUNK_SIZE / (SAMPLES_PER_CHUNK * 2))
                                as i32;

                        // Texture coordinates (40x40)
                        // Linear mapping to match shader (no Y-flip needed)
                        let tex_x =
                            ((cx + 2) * SAMPLES_PER_CHUNK as i32 + sample_x as i32) as usize;
                        let tex_y =
                            ((cy + 2) * SAMPLES_PER_CHUNK as i32 + sample_y as i32) as usize;

                        // Get light level at this world position
                        let light = world.get_light_at(world_x, world_y).unwrap_or(0) as f32;

                        let idx = tex_y * LIGHT_TEXTURE_SIZE as usize + tex_x;
                        if idx < light_data.len() {
                            light_data[idx] = light;
                        }
                    }
                }
            }
        }

        // Upload to GPU
        self.queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &self.light_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            bytemuck::cast_slice(&light_data),
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(LIGHT_TEXTURE_SIZE * 4), // 4 bytes per f32
                rows_per_image: Some(LIGHT_TEXTURE_SIZE),
            },
            wgpu::Extent3d {
                width: LIGHT_TEXTURE_SIZE,
                height: LIGHT_TEXTURE_SIZE,
                depth_or_array_layers: 1,
            },
        );
    }
}
