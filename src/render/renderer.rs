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
    world_texture_view: wgpu::TextureView,
    world_sampler: wgpu::Sampler,
    world_bind_group: wgpu::BindGroup,
    
    // Camera
    camera_buffer: wgpu::Buffer,
    camera_bind_group: wgpu::BindGroup,
    camera: CameraUniform,
    
    // Pixel buffer for CPU-side rendering
    pixel_buffer: Vec<u8>,
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
        
        // Shader
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
        });
        
        // Pipeline layout
        let render_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("render_pipeline_layout"),
            bind_group_layouts: &[&texture_bind_group_layout, &camera_bind_group_layout],
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
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
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
    
    pub fn render(&mut self, world: &World) -> Result<()> {
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
        self.camera.position = [world.player_pos.x, world.player_pos.y];
        self.queue.write_buffer(&self.camera_buffer, 0, bytemuck::cast_slice(&[self.camera]));
        
        // Get output texture
        let output = self.surface.get_current_texture()?;
        let view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());
        
        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("render_encoder"),
        });
        
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("render_pass"),
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
            render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
            render_pass.draw_indexed(0..6, 0, 0..1);
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

    /// Get window size
    pub fn window_size(&self) -> (u32, u32) {
        (self.size.width, self.size.height)
    }
}
