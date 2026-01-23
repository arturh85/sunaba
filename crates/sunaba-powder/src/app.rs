//! Application state and event loop for Powder Game demo

use anyhow::Result;
use std::sync::Arc;
use web_time::Instant;
use winit::{
    application::ApplicationHandler,
    dpi::LogicalSize,
    event::{ElementState, MouseButton, WindowEvent},
    event_loop::{ActiveEventLoop, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    window::{Window, WindowAttributes, WindowId},
};

use sunaba_core::simulation::Materials;
use sunaba_core::world::{NoopStats, World};

use crate::config::PowderConfig;
use crate::render::Renderer;
use crate::tools::{DragTool, EraseTool, PenTool, Tool, WindTool};
use crate::ui::{ActiveTool, MaterialToolbar, PowderStats, ToolbarState, show_hud};

/// Main application state
pub struct App {
    // Window and rendering
    window: Arc<Window>,
    renderer: Renderer,
    egui_ctx: egui::Context,
    egui_state: egui_winit::State,
    egui_renderer: egui_wgpu::Renderer,

    // Simulation
    world: World,
    materials: Materials,
    config: PowderConfig,

    // UI state
    toolbar: MaterialToolbar,
    toolbar_state: ToolbarState,

    // Tools
    left_pen: PenTool,
    right_pen: PenTool,
    eraser: EraseTool,
    wind_tool: WindTool,
    drag_tool: DragTool,

    // Input state
    mouse_pos: Option<(f32, f32)>,
    left_pressed: bool,
    right_pressed: bool,
    last_draw_pos: Option<(i32, i32)>,

    // Timing
    last_update: Instant,
    frame_count: u64,
    fps_update_time: Instant,
    fps: f32,

    // Single step mode
    should_step: bool,
}

impl App {
    /// Create a new app
    pub async fn new() -> Result<(Self, EventLoop<()>)> {
        let config = PowderConfig::load();

        // Create event loop
        let event_loop = EventLoop::new()?;

        // Create window
        let window_attrs = WindowAttributes::default()
            .with_title("Sunaba Powder")
            .with_inner_size(LogicalSize::new(config.window_width, config.window_height));

        let window = Arc::new(event_loop.create_window(window_attrs)?);

        // Create renderer
        let renderer = Renderer::new(&window, config.world_size).await?;

        // Create materials
        let materials = Materials::new();

        // Create world
        let mut world = World::new(false);

        // Set active chunk radius to cover entire 1024px world (17×17 chunks)
        world.set_active_chunk_radius(8);

        // Ensure chunks around origin are loaded
        let half_size = (config.world_size as i32) / 2;
        world.ensure_chunks_for_area(-half_size, -half_size, half_size, half_size);

        // Create UI
        let toolbar = MaterialToolbar::new(&materials);
        let toolbar_state = ToolbarState::default();

        // Create tools
        let left_pen = PenTool::new(toolbar_state.left_material);
        let right_pen = PenTool::new(toolbar_state.right_material);
        let eraser = EraseTool;
        let wind_tool = WindTool::new();
        let drag_tool = DragTool::new();

        // Setup egui
        let egui_ctx = egui::Context::default();
        let egui_state = egui_winit::State::new(
            egui_ctx.clone(),
            egui::ViewportId::ROOT,
            &window,
            Some(window.scale_factor() as f32),
            None,
            None,
        );
        let egui_renderer = egui_wgpu::Renderer::new(
            &renderer.device,
            renderer.surface_format(),
            egui_wgpu::RendererOptions::default(),
        );

        Ok((
            Self {
                window,
                renderer,
                egui_ctx,
                egui_state,
                egui_renderer,
                world,
                materials,
                config,
                toolbar,
                toolbar_state,
                left_pen,
                right_pen,
                eraser,
                wind_tool,
                drag_tool,
                mouse_pos: None,
                left_pressed: false,
                right_pressed: false,
                last_draw_pos: None,
                last_update: Instant::now(),
                frame_count: 0,
                fps_update_time: Instant::now(),
                fps: 0.0,
                should_step: false,
            },
            event_loop,
        ))
    }

    /// Run the event loop
    pub fn run(event_loop: EventLoop<()>, mut app: Self) -> Result<()> {
        event_loop.run_app(&mut app)?;
        Ok(())
    }

    /// Update simulation
    fn update(&mut self) {
        let now = Instant::now();

        // Update FPS
        self.frame_count += 1;
        if now.duration_since(self.fps_update_time).as_secs_f32() >= 1.0 {
            self.fps = self.frame_count as f32;
            self.frame_count = 0;
            self.fps_update_time = now;
        }

        // Handle drawing based on active tool
        if let Some((screen_x, screen_y)) = self.mouse_pos {
            let (world_x, world_y) = self.renderer.screen_to_world(screen_x, screen_y);
            let brush_size = self.toolbar_state.brush_size;

            if self.left_pressed || self.right_pressed {
                match self.toolbar_state.active_tool {
                    ActiveTool::Pen => {
                        if self.left_pressed {
                            self.left_pen
                                .apply(&mut self.world, world_x, world_y, brush_size);
                        }
                        if self.right_pressed {
                            self.right_pen
                                .apply(&mut self.world, world_x, world_y, brush_size);
                        }
                    }
                    ActiveTool::Eraser => {
                        self.eraser
                            .apply(&mut self.world, world_x, world_y, brush_size);
                    }
                    ActiveTool::Wind => {
                        self.wind_tool
                            .apply(&mut self.world, world_x, world_y, brush_size);
                    }
                    ActiveTool::Drag => {
                        self.drag_tool
                            .apply_drag(&mut self.world, world_x, world_y, brush_size);
                    }
                }
            } else {
                // End drag when mouse released
                if self.toolbar_state.active_tool == ActiveTool::Drag {
                    self.drag_tool.end_drag();
                }
            }
        } else {
            // End drag when mouse leaves window
            if self.toolbar_state.active_tool == ActiveTool::Drag {
                self.drag_tool.end_drag();
            }
        }

        // Update simulation (unless paused)
        if !self.toolbar_state.paused || self.should_step {
            // Calculate how many steps to run based on speed
            let steps = (self.toolbar_state.sim_speed.max(0.25)) as usize;
            let mut stats = NoopStats;
            let mut rng = rand::thread_rng();
            for _ in 0..steps.max(1) {
                self.world.update(1.0 / 60.0, &mut stats, &mut rng, false);
            }
            self.should_step = false;
        }

        self.last_update = now;
    }

    /// Render frame
    fn render(&mut self) -> Result<()> {
        // Update tool materials from toolbar state
        self.left_pen.set_material(self.toolbar_state.left_material);
        self.right_pen
            .set_material(self.toolbar_state.right_material);

        // Update visualization mode
        self.renderer
            .set_visualization_mode(self.toolbar_state.visualization_mode);

        // Update world texture
        self.renderer
            .update_world_texture(&self.world, &self.materials);

        // Collect data for egui closure to avoid borrow checker issues
        let particle_count = self.count_particles();
        let fps = self.fps;
        let brush_size = self.toolbar_state.brush_size;
        let paused = self.toolbar_state.paused;

        // Begin frame
        let output = self.renderer.begin_frame()?;
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder =
            self.renderer
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("render_encoder"),
                });

        // Render world
        self.renderer.render_world(&mut encoder, &view)?;

        // Run egui
        let raw_input = self.egui_state.take_egui_input(&self.window);
        let full_output = self.egui_ctx.run(raw_input, |ctx| {
            // Show toolbar
            self.toolbar.show(ctx, &mut self.toolbar_state);

            // Show HUD
            let stats = PowderStats {
                fps,
                particle_count,
                brush_size,
                paused,
            };
            show_hud(ctx, &stats);
        });

        // Handle egui platform output
        self.egui_state
            .handle_platform_output(&self.window, full_output.platform_output);

        // Tessellate egui shapes
        let paint_jobs = self
            .egui_ctx
            .tessellate(full_output.shapes, full_output.pixels_per_point);

        // Update egui textures
        for (id, delta) in &full_output.textures_delta.set {
            self.egui_renderer.update_texture(
                &self.renderer.device,
                &self.renderer.queue,
                *id,
                delta,
            );
        }

        // Create screen descriptor
        let screen_descriptor = egui_wgpu::ScreenDescriptor {
            size_in_pixels: [self.renderer.size().width, self.renderer.size().height],
            pixels_per_point: full_output.pixels_per_point,
        };

        // Update egui buffers
        self.egui_renderer.update_buffers(
            &self.renderer.device,
            &self.renderer.queue,
            &mut encoder,
            &paint_jobs,
            &screen_descriptor,
        );

        // Render egui
        {
            let render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("egui_render_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });

            self.egui_renderer.render(
                &mut render_pass.forget_lifetime(),
                &paint_jobs,
                &screen_descriptor,
            );
        }

        // Free egui textures
        for id in &full_output.textures_delta.free {
            self.egui_renderer.free_texture(id);
        }

        // Submit and present
        self.renderer
            .queue
            .submit(std::iter::once(encoder.finish()));
        self.renderer.end_frame(output);

        Ok(())
    }

    /// Count non-air particles in the world
    fn count_particles(&self) -> usize {
        let mut count = 0;
        let half_size = (self.config.world_size / 64) as i32 / 2;

        for cy in -half_size..half_size {
            for cx in -half_size..half_size {
                if let Some(chunk) = self.world.get_chunk(cx, cy) {
                    for y in 0..64 {
                        for x in 0..64 {
                            let pixel = chunk.get_pixel(x, y);
                            if pixel.material_id != 0 {
                                count += 1;
                            }
                        }
                    }
                }
            }
        }
        count
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, _event_loop: &ActiveEventLoop) {
        // Nothing to do on resume for now
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        // Let egui handle events first
        let egui_response = self.egui_state.on_window_event(&self.window, &event);
        if egui_response.consumed {
            return;
        }

        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::Resized(size) => {
                self.renderer.resize(size);
            }
            WindowEvent::RedrawRequested => {
                self.update();
                if let Err(e) = self.render() {
                    log::error!("Render error: {}", e);
                }
                self.window.request_redraw();
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.mouse_pos = Some((position.x as f32, position.y as f32));
            }
            WindowEvent::CursorLeft { .. } => {
                self.mouse_pos = None;
                self.left_pressed = false;
                self.right_pressed = false;
            }
            WindowEvent::MouseInput { state, button, .. } => {
                let pressed = state == ElementState::Pressed;
                match button {
                    MouseButton::Left => self.left_pressed = pressed,
                    MouseButton::Right => self.right_pressed = pressed,
                    _ => {}
                }
                if !pressed {
                    self.last_draw_pos = None;
                }
            }
            WindowEvent::MouseWheel { delta, .. } => {
                // Zoom with scroll wheel
                let scroll = match delta {
                    winit::event::MouseScrollDelta::LineDelta(_, y) => y,
                    winit::event::MouseScrollDelta::PixelDelta(pos) => pos.y as f32 / 100.0,
                };
                let current_zoom = self.renderer.camera_zoom();
                let new_zoom = (current_zoom * (1.0 + scroll * 0.1)).clamp(0.001, 0.1);
                self.renderer.set_camera_zoom(new_zoom);
            }
            WindowEvent::KeyboardInput { event, .. } => {
                if event.state == ElementState::Pressed {
                    match event.physical_key {
                        PhysicalKey::Code(KeyCode::Space) => {
                            self.toolbar_state.paused = !self.toolbar_state.paused;
                        }
                        PhysicalKey::Code(KeyCode::KeyS) => {
                            if self.toolbar_state.paused {
                                self.should_step = true;
                            }
                        }
                        PhysicalKey::Code(KeyCode::KeyC) => {
                            // Clear world
                            let half_size = (self.config.world_size / 64) as i32 / 2;
                            for cy in -half_size..half_size {
                                for cx in -half_size..half_size {
                                    for y in 0..64 {
                                        for x in 0..64 {
                                            let world_x = cx * 64 + x;
                                            let world_y = cy * 64 + y;
                                            self.world.set_pixel(world_x, world_y, 0);
                                        }
                                    }
                                }
                            }
                        }
                        PhysicalKey::Code(KeyCode::BracketLeft) => {
                            if self.toolbar_state.brush_size > 1 {
                                self.toolbar_state.brush_size -= 1;
                            }
                        }
                        PhysicalKey::Code(KeyCode::BracketRight) => {
                            if self.toolbar_state.brush_size < 10 {
                                self.toolbar_state.brush_size += 1;
                            }
                        }
                        PhysicalKey::Code(KeyCode::Escape) => {
                            event_loop.exit();
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }
}
