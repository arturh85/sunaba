//! Application state and main game loop

use anyhow::Result;
use glam::Vec2;
use instant::{Duration, Instant};
use rand::Rng;
use winit::{
    application::ApplicationHandler,
    event::{ElementState, MouseButton, WindowEvent},
    event_loop::{ActiveEventLoop, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    window::{Window, WindowAttributes},
};

#[cfg(not(target_arch = "wasm32"))]
use crate::config::GameConfig;
use crate::entity::InputState;
use crate::levels::LevelManager;
use crate::render::{ParticleSystem, Renderer};
use crate::simulation::MaterialType;
use crate::ui::UiState;
use crate::world::World;

/// Game mode: persistent world or demo level
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameMode {
    PersistentWorld,
    DemoLevel(usize),
}

// Zoom constants - fallback for WASM (native uses GameConfig)
#[cfg(target_arch = "wasm32")]
const MIN_ZOOM: f32 = 0.002;
#[cfg(target_arch = "wasm32")]
const MAX_ZOOM: f32 = 0.01;
#[cfg(target_arch = "wasm32")]
const DEBUG_PLACEMENT: bool = true;

/// Convert screen coordinates to world coordinates
fn screen_to_world(
    screen_x: f64,
    screen_y: f64,
    window_width: u32,
    window_height: u32,
    camera_pos: Vec2,
    camera_zoom: f32,
) -> (i32, i32) {
    // Convert to NDC (Normalized Device Coordinates)
    let ndc_x = (screen_x / window_width as f64) * 2.0 - 1.0;
    let ndc_y = 1.0 - (screen_y / window_height as f64) * 2.0; // Flip Y

    let aspect = window_width as f32 / window_height as f32;

    // Transform to world space
    let world_x = (ndc_x as f32 * aspect / camera_zoom) + camera_pos.x;
    let world_y = (ndc_y as f32 / camera_zoom) + camera_pos.y;

    log::trace!(
        "screen_to_world: screen({:.0},{:.0}) → ndc({:.2},{:.2}) → world({:.1},{:.1}) [aspect={:.2}, zoom={:.2}, cam={:?}]",
        screen_x,
        screen_y,
        ndc_x,
        ndc_y,
        world_x,
        world_y,
        aspect,
        camera_zoom,
        camera_pos
    );

    (world_x as i32, world_y as i32)
}

/// Convert world coordinates to screen coordinates
fn world_to_screen(
    world_x: f32,
    world_y: f32,
    window_width: u32,
    window_height: u32,
    camera_pos: Vec2,
    camera_zoom: f32,
) -> (f32, f32) {
    let aspect = window_width as f32 / window_height as f32;

    // Transform from world space to NDC
    let ndc_x = (world_x - camera_pos.x) * camera_zoom / aspect;
    let ndc_y = (world_y - camera_pos.y) * camera_zoom;

    // Convert from NDC to screen coordinates
    let screen_x = (ndc_x + 1.0) * window_width as f32 / 2.0;
    let screen_y = (1.0 - ndc_y) * window_height as f32 / 2.0; // Flip Y

    (screen_x, screen_y)
}

pub struct App {
    window: Window,
    renderer: Renderer,
    world: World,
    input_state: InputState,
    egui_ctx: egui::Context,
    egui_state: egui_winit::State,
    ui_state: UiState,
    level_manager: LevelManager,
    game_mode: GameMode,
    last_autosave: Instant,
    particle_system: ParticleSystem,
    #[cfg(not(target_arch = "wasm32"))]
    config: GameConfig,
    /// Hot reload manager for config and materials.
    /// On WASM, this is a no-op but we keep it for API consistency.
    #[allow(dead_code)]
    hot_reload: crate::hot_reload::HotReloadManager,
}

impl App {
    /// Check if debug placement is enabled (allows placing materials without consuming inventory)
    #[inline]
    fn debug_placement(&self) -> bool {
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.config.debug.debug_placement
        }
        #[cfg(target_arch = "wasm32")]
        {
            DEBUG_PLACEMENT
        }
    }

    /// Get the zoom speed multiplier
    #[inline]
    fn zoom_speed(&self) -> f32 {
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.config.camera.zoom_speed
        }
        #[cfg(target_arch = "wasm32")]
        {
            1.1 // Default zoom speed for WASM
        }
    }

    pub async fn new() -> Result<(Self, EventLoop<()>)> {
        let event_loop = EventLoop::new()?;

        // Platform-specific window creation using WindowAttributes
        #[cfg(target_arch = "wasm32")]
        let window_attrs = {
            use wasm_bindgen::JsCast;
            use winit::platform::web::WindowAttributesExtWebSys;

            // Get the canvas element from the DOM
            let web_window =
                web_sys::window().ok_or_else(|| anyhow::anyhow!("Failed to get web window"))?;
            let document = web_window
                .document()
                .ok_or_else(|| anyhow::anyhow!("Failed to get document"))?;
            let canvas = document
                .get_element_by_id("canvas")
                .ok_or_else(|| anyhow::anyhow!("Failed to find canvas element with id='canvas'"))?
                .dyn_into::<web_sys::HtmlCanvasElement>()
                .map_err(|_| anyhow::anyhow!("Element 'canvas' is not an HTMLCanvasElement"))?;

            log::info!("Found canvas element, binding to window");

            WindowAttributes::default()
                .with_title("Sunaba - 2D Physics Sandbox")
                .with_canvas(Some(canvas))
        };

        #[cfg(not(target_arch = "wasm32"))]
        let config = GameConfig::load()?;

        #[cfg(not(target_arch = "wasm32"))]
        let window_attrs = {
            WindowAttributes::default()
                .with_title("Sunaba - 2D Physics Sandbox")
                .with_inner_size(winit::dpi::LogicalSize::new(
                    config.ui.window_width,
                    config.ui.window_height,
                ))
        };

        // Use deprecated create_window to avoid async complexity for now
        #[allow(deprecated)]
        let window = event_loop.create_window(window_attrs)?;

        let renderer = Renderer::new(&window).await?;
        let mut world = World::new();

        // Initialize level manager (but don't load a level yet)
        let level_manager = LevelManager::new();

        // Load persistent world instead of demo level
        world.load_persistent_world();
        let game_mode = GameMode::PersistentWorld;

        // Initialize egui
        let egui_ctx = egui::Context::default();
        let egui_state = egui_winit::State::new(
            egui_ctx.clone(),
            egui::ViewportId::ROOT,
            &window,
            Some(window.scale_factor() as f32),
            None,
            None, // max_texture_side
        );

        log::info!("Loaded persistent world");

        #[cfg(not(target_arch = "wasm32"))]
        let ui_state = UiState::new(&config);

        #[cfg(target_arch = "wasm32")]
        let ui_state = UiState::default();

        let app = Self {
            window,
            renderer,
            world,
            input_state: InputState::default(),
            egui_ctx,
            egui_state,
            ui_state,
            level_manager,
            game_mode,
            last_autosave: Instant::now(),
            particle_system: ParticleSystem::new(),
            #[cfg(not(target_arch = "wasm32"))]
            config,
            hot_reload: crate::hot_reload::HotReloadManager::new(),
        };

        Ok((app, event_loop))
    }

    /// Switch to a demo level (disables persistence)
    #[allow(dead_code)]
    fn switch_to_demo_level(&mut self, level_id: usize) {
        // Save current world if in persistent mode
        if matches!(self.game_mode, GameMode::PersistentWorld) {
            self.world.save_all_dirty_chunks();
        }

        self.game_mode = GameMode::DemoLevel(level_id);
        self.level_manager.load_level(level_id, &mut self.world);
        log::info!(
            "Switched to demo level {}: {}",
            level_id,
            self.level_manager.current_level_name()
        );
    }

    /// Return to persistent world from demo level
    #[allow(dead_code)]
    fn return_to_persistent_world(&mut self) {
        self.game_mode = GameMode::PersistentWorld;
        self.world.load_persistent_world();
        log::info!("Returned to persistent world");
    }

    /// Get a description of the current game mode
    #[allow(dead_code)]
    fn game_mode_description(&self) -> String {
        match self.game_mode {
            GameMode::PersistentWorld => "Persistent World".to_string(),
            GameMode::DemoLevel(id) => format!(
                "Demo Level {}: {}",
                id + 1,
                self.level_manager.current_level_name()
            ),
        }
    }

    /// Select a hotbar slot and equip/unequip tools
    fn select_hotbar_slot(&mut self, slot: usize) {
        // Select the inventory slot
        self.world.player.select_slot(slot);

        // Check what's in the selected slot (extract value to avoid borrow issues)
        let slot_tool_id = self
            .world
            .player
            .inventory
            .get_slot(slot)
            .and_then(|s| s.as_ref())
            .and_then(|stack| stack.tool_id());

        let has_material = self
            .world
            .player
            .inventory
            .get_slot(slot)
            .and_then(|s| s.as_ref())
            .and_then(|stack| stack.material_id())
            .is_some();

        if let Some(tool_id) = slot_tool_id {
            // Equip the tool
            self.world.player.equip_tool(tool_id);
            log::debug!("Equipped tool {} from slot {}", tool_id, slot);
        } else if has_material {
            // Unequip any equipped tool (switching to material placement)
            if self.world.player.equipped_tool.is_some() {
                self.world.player.unequip_tool();
                log::debug!("Unequipped tool, slot {} has material", slot);
            }
        } else {
            // Empty slot - unequip any equipped tool
            if self.world.player.equipped_tool.is_some() {
                self.world.player.unequip_tool();
                log::debug!("Unequipped tool, slot {} is empty", slot);
            }
        }
    }

    /// Select a material directly by key (debug mode)
    /// Maps keys 0-9 to materials: AIR, STONE, SAND, WATER, WOOD, FIRE, SMOKE, STEAM, LAVA, OIL
    fn select_debug_material(&mut self, key: u8) {
        use crate::simulation::MaterialId;
        let material_id = match key {
            0 => MaterialId::AIR,
            1 => MaterialId::STONE,
            2 => MaterialId::SAND,
            3 => MaterialId::WATER,
            4 => MaterialId::WOOD,
            5 => MaterialId::FIRE,
            6 => MaterialId::SMOKE,
            7 => MaterialId::STEAM,
            8 => MaterialId::LAVA,
            9 => MaterialId::OIL,
            _ => MaterialId::SAND, // fallback
        };
        self.input_state.selected_material = material_id;
        log::debug!("Selected debug material: {} (id={})", key, material_id);
    }

    pub fn run(event_loop: EventLoop<()>, mut app: Self) -> Result<()> {
        event_loop.run_app(&mut app)?;
        Ok(())
    }

    fn handle_redraw(&mut self) {
        // Signal new frame to puffin profiler
        #[cfg(feature = "profiling")]
        puffin::GlobalProfiler::lock().new_frame();

        // Begin frame timing
        self.ui_state.stats.begin_frame();

        // Check for hot-reloadable file changes
        #[cfg(not(target_arch = "wasm32"))]
        {
            let flags = self.hot_reload.check_for_changes();
            if flags.config_changed {
                match GameConfig::load() {
                    Ok(new_config) => {
                        log::info!("Hot-reloaded config.ron");
                        self.config = new_config;
                    }
                    Err(e) => {
                        log::error!("Failed to hot-reload config: {}", e);
                    }
                }
            }
            // materials.ron hot reload would go here when implemented

            // Check for params changes (from dock parameters panel) and apply to game systems
            if self.ui_state.take_params_changed() {
                // Apply rendering params
                self.renderer.set_post_process_params(
                    self.config.rendering.scanline_intensity,
                    self.config.rendering.vignette_intensity,
                    self.config.rendering.bloom_intensity,
                );

                log::debug!("Applied params changes from dock");
            }
        }

        // Periodic auto-save in persistent world mode
        #[cfg(not(target_arch = "wasm32"))]
        let autosave_interval = Duration::from_secs(self.config.world.autosave_interval_secs);
        #[cfg(target_arch = "wasm32")]
        let autosave_interval = Duration::from_secs(60);

        if matches!(self.game_mode, GameMode::PersistentWorld)
            && self.last_autosave.elapsed() >= autosave_interval
        {
            self.world.save_all_dirty_chunks(); // Save chunks AND player data
            self.last_autosave = Instant::now();
            log::info!("Auto-saved world and player data");
        }

        // Update player from input
        self.world.update_player(&self.input_state, 1.0 / 60.0);

        // Spawn flight particles when flying (W pressed while airborne)
        if self.input_state.w_pressed && !self.world.player.grounded {
            self.particle_system.spawn_flight_burst(
                self.world.player.position,
                crate::entity::player::Player::HEIGHT,
            );
        }

        // Update visual particles
        self.particle_system.update(1.0 / 60.0);

        // Update camera zoom
        #[cfg(not(target_arch = "wasm32"))]
        let (min_zoom, max_zoom) = (self.config.camera.min_zoom, self.config.camera.max_zoom);
        #[cfg(target_arch = "wasm32")]
        let (min_zoom, max_zoom) = (MIN_ZOOM, MAX_ZOOM);
        self.renderer
            .update_zoom(self.input_state.zoom_delta, min_zoom, max_zoom);

        // Log camera state periodically
        use std::sync::atomic::{AtomicU32, Ordering};
        static FRAME_COUNT: AtomicU32 = AtomicU32::new(0);
        let frame = FRAME_COUNT.fetch_add(1, Ordering::Relaxed);
        if frame.is_multiple_of(600) {
            // Every 10 seconds at 60fps
            log::info!(
                "Frame {}: player_pos={:?}, zoom={:.2}, selected_material={}",
                frame,
                self.world.player.position,
                self.renderer.camera_zoom(),
                self.input_state.selected_material
            );
        }

        // DEBUG: Right-click instant mining circle (for exploration)
        // Continuously mines while button is held
        if self.input_state.right_mouse_pressed {
            let player_pos = self.world.player.position;
            let center_x = player_pos.x as i32;
            let center_y = player_pos.y as i32;
            self.world.debug_mine_circle(center_x, center_y, 16);

            // Spawn dust particles at mining location
            self.particle_system.spawn_dust_cloud(
                player_pos,
                [140, 130, 120, 255], // Generic dusty color
            );
        }

        // Placing material from inventory with left mouse button
        if self.input_state.left_mouse_pressed
            && let Some((wx, wy)) = self.input_state.mouse_world_pos
        {
            let material_id = self.input_state.selected_material;
            let material_def = self.world.materials.get(material_id);
            let color = material_def.color;
            let is_liquid = material_def.material_type == MaterialType::Liquid;

            if self.debug_placement() {
                self.world.place_material_debug(wx, wy, material_id);
            } else {
                self.world
                    .place_material_from_inventory(wx, wy, material_id);
            }

            // Spawn particles at placement location
            let pos = Vec2::new(wx as f32, wy as f32);
            if is_liquid {
                self.particle_system.spawn_liquid_splash(pos, color);
            } else {
                self.particle_system.spawn_impact_burst(pos, color);
            }
        }

        // Update simulation with timing
        #[cfg(feature = "profiling")]
        puffin::profile_scope!("simulation");
        self.ui_state.stats.begin_sim();
        self.world.update(1.0 / 60.0, &mut self.ui_state.stats, &mut rand::rng());
        self.ui_state.stats.end_sim();

        // Collect world stats
        self.ui_state.stats.collect_world_stats(&self.world);

        // Update tooltip with world data
        self.ui_state.update_tooltip(
            &self.world,
            self.world.materials(),
            self.input_state.mouse_world_pos,
            self.renderer.is_light_overlay_enabled(),
        );

        // Prepare egui frame
        let raw_input = self.egui_state.take_egui_input(&self.window);
        let egui_build_start = Instant::now();

        // Extract data for active chunks overlay before the closure to avoid borrow issues
        let show_active_chunks = self.renderer.is_active_chunks_overlay_enabled();
        let active_chunks_data = if show_active_chunks {
            Some((
                self.window.inner_size(),
                self.renderer.camera_position(),
                self.renderer.camera_zoom(),
                self.world.active_chunk_positions().to_vec(),
            ))
        } else {
            None
        };

        let full_output = self.egui_ctx.run(raw_input, |ctx| {
            // Get cursor position from egui context
            let cursor_pos = ctx.pointer_hover_pos().unwrap_or(egui::pos2(0.0, 0.0));

            // Get game mode description
            let game_mode_desc = match self.game_mode {
                GameMode::PersistentWorld => "Persistent World".to_string(),
                GameMode::DemoLevel(id) => format!(
                    "Demo Level {}: {}",
                    id + 1,
                    self.level_manager.current_level_name()
                ),
            };
            let in_persistent_world = matches!(self.game_mode, GameMode::PersistentWorld);

            #[cfg(not(target_arch = "wasm32"))]
            self.ui_state.render(
                ctx,
                cursor_pos,
                self.input_state.selected_material,
                self.world.materials(),
                &game_mode_desc,
                in_persistent_world,
                &self.level_manager,
                &self.world.player,
                self.world.tool_registry(),
                &self.world.recipe_registry,
                &mut self.config,
            );

            #[cfg(target_arch = "wasm32")]
            self.ui_state.render(
                ctx,
                cursor_pos,
                self.input_state.selected_material,
                self.world.materials(),
                &game_mode_desc,
                in_persistent_world,
                &self.level_manager,
                &self.world.player,
                self.world.tool_registry(),
                &self.world.recipe_registry,
            );

            // Draw active chunks overlay if enabled
            if let Some((window_size, camera_pos, camera_zoom, active_chunks)) = &active_chunks_data
            {
                draw_active_chunks_overlay(
                    ctx,
                    *window_size,
                    *camera_pos,
                    *camera_zoom,
                    active_chunks,
                );
            }
        });
        let egui_build_time = egui_build_start.elapsed().as_secs_f32() * 1000.0;

        // TODO: Level selector actions now handled via dock
        // The dock's LevelSelector tab currently just displays info
        // Interactive level switching could be added later via dock callback system

        // Handle egui output
        self.egui_state
            .handle_platform_output(&self.window, full_output.platform_output);

        // Update overlay textures
        let overlay_start = Instant::now();
        self.renderer.update_temperature_overlay(&self.world);
        self.renderer.update_light_overlay(&self.world);
        let overlay_time = overlay_start.elapsed().as_secs_f32() * 1000.0;

        // Set frame loop timing stats
        self.ui_state
            .stats
            .set_frame_loop_timing(egui_build_time, overlay_time);

        // Update player sprite animation
        self.renderer.update_player_sprite(
            self.world.player.velocity,
            self.world.player.mining_progress.is_mining(),
            1.0 / 60.0,
        );

        // Update camera to smoothly follow player
        self.renderer
            .update_camera_follow(self.world.player.position, 1.0 / 60.0);

        // Render world + UI
        match self.renderer.render(
            &mut self.world,
            &self.particle_system,
            &self.egui_ctx,
            full_output.textures_delta,
            full_output.shapes,
        ) {
            Ok(timing) => {
                // Collect render timing breakdown
                self.ui_state.stats.set_render_timing(
                    timing.pixel_buffer_ms,
                    timing.gpu_upload_ms,
                    timing.acquire_ms,
                    timing.egui_ms,
                    timing.present_ms,
                );
            }
            Err(e) => {
                log::error!("Render error: {e}");
            }
        }

        // Collect render stats for debugging
        let (dirty_chunks, rendered_total) = self.renderer.get_render_stats();
        self.ui_state
            .stats
            .set_render_stats(dirty_chunks, rendered_total);

        // Reset per-frame input state
        self.input_state.zoom_delta = 1.0;
        self.input_state.prev_right_mouse_pressed = self.input_state.right_mouse_pressed;
    }
}

/// Draw active chunks overlay using egui painter (free function to avoid borrow issues)
fn draw_active_chunks_overlay(
    ctx: &egui::Context,
    window_size: winit::dpi::PhysicalSize<u32>,
    camera_pos: Vec2,
    camera_zoom: f32,
    active_chunks: &[glam::IVec2],
) {
    use crate::world::CHUNK_SIZE;

    if active_chunks.is_empty() {
        return;
    }

    // Calculate bounding box of active chunks in world coordinates
    let mut min_chunk_x = i32::MAX;
    let mut min_chunk_y = i32::MAX;
    let mut max_chunk_x = i32::MIN;
    let mut max_chunk_y = i32::MIN;

    for chunk_pos in active_chunks {
        min_chunk_x = min_chunk_x.min(chunk_pos.x);
        min_chunk_y = min_chunk_y.min(chunk_pos.y);
        max_chunk_x = max_chunk_x.max(chunk_pos.x);
        max_chunk_y = max_chunk_y.max(chunk_pos.y);
    }

    // Draw on the background layer (behind UI windows)
    let painter = ctx.layer_painter(egui::LayerId::background());

    // Colors for the overlay
    let outer_color = egui::Color32::from_rgba_unmultiplied(0, 255, 0, 180);
    let grid_color = egui::Color32::from_rgba_unmultiplied(0, 200, 0, 100);

    // Draw outer rectangle around entire active region
    let outer_min_world = (
        (min_chunk_x * CHUNK_SIZE as i32) as f32,
        (min_chunk_y * CHUNK_SIZE as i32) as f32,
    );
    let outer_max_world = (
        ((max_chunk_x + 1) * CHUNK_SIZE as i32) as f32,
        ((max_chunk_y + 1) * CHUNK_SIZE as i32) as f32,
    );

    let outer_min_screen = world_to_screen(
        outer_min_world.0,
        outer_max_world.1, // Note: Y is flipped in screen space
        window_size.width,
        window_size.height,
        camera_pos,
        camera_zoom,
    );
    let outer_max_screen = world_to_screen(
        outer_max_world.0,
        outer_min_world.1,
        window_size.width,
        window_size.height,
        camera_pos,
        camera_zoom,
    );

    let outer_rect = egui::Rect::from_min_max(
        egui::pos2(outer_min_screen.0, outer_min_screen.1),
        egui::pos2(outer_max_screen.0, outer_max_screen.1),
    );
    painter.rect_stroke(
        outer_rect,
        0.0,
        egui::Stroke::new(2.0, outer_color),
        egui::StrokeKind::Inside,
    );

    // Draw grid lines for individual chunks
    for chunk_x in min_chunk_x..=max_chunk_x {
        for chunk_y in min_chunk_y..=max_chunk_y {
            let chunk_min_world = (
                (chunk_x * CHUNK_SIZE as i32) as f32,
                (chunk_y * CHUNK_SIZE as i32) as f32,
            );
            let chunk_max_world = (
                ((chunk_x + 1) * CHUNK_SIZE as i32) as f32,
                ((chunk_y + 1) * CHUNK_SIZE as i32) as f32,
            );

            let chunk_min_screen = world_to_screen(
                chunk_min_world.0,
                chunk_max_world.1,
                window_size.width,
                window_size.height,
                camera_pos,
                camera_zoom,
            );
            let chunk_max_screen = world_to_screen(
                chunk_max_world.0,
                chunk_min_world.1,
                window_size.width,
                window_size.height,
                camera_pos,
                camera_zoom,
            );

            let chunk_rect = egui::Rect::from_min_max(
                egui::pos2(chunk_min_screen.0, chunk_min_screen.1),
                egui::pos2(chunk_max_screen.0, chunk_max_screen.1),
            );
            painter.rect_stroke(
                chunk_rect,
                0.0,
                egui::Stroke::new(1.0, grid_color),
                egui::StrokeKind::Inside,
            );
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, _event_loop: &ActiveEventLoop) {
        // Window and renderer are already initialized in new()
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        // Let egui handle events first
        let _ = self.egui_state.on_window_event(&self.window, &event);

        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::Resized(size) => {
                self.renderer.resize(size.width, size.height);
            }
            WindowEvent::KeyboardInput {
                event: key_event, ..
            } => {
                // Skip input if egui wants it
                if self.egui_ctx.wants_keyboard_input() {
                    return;
                }
                if let PhysicalKey::Code(code) = key_event.physical_key {
                    let pressed = key_event.state == ElementState::Pressed;
                    log::debug!(
                        "Keyboard: {:?} {}",
                        code,
                        if pressed { "pressed" } else { "released" }
                    );

                    match code {
                        // Movement keys
                        KeyCode::KeyW => self.input_state.w_pressed = pressed,
                        KeyCode::KeyA => self.input_state.a_pressed = pressed,
                        KeyCode::KeyS => self.input_state.s_pressed = pressed,
                        KeyCode::KeyD => self.input_state.d_pressed = pressed,
                        KeyCode::Space => self.input_state.jump_pressed = pressed,

                        // Material/hotbar selection (0-9)
                        // In debug mode: select materials directly (AIR, STONE, SAND, etc.)
                        // In normal mode: select inventory slots
                        KeyCode::Digit0 => {
                            if pressed {
                                if self.debug_placement() {
                                    self.select_debug_material(0);
                                } else {
                                    self.select_hotbar_slot(9); // 0 key = slot 9
                                }
                            }
                        }
                        KeyCode::Digit1 => {
                            if pressed {
                                if self.debug_placement() {
                                    self.select_debug_material(1);
                                } else {
                                    self.select_hotbar_slot(0);
                                }
                            }
                        }
                        KeyCode::Digit2 => {
                            if pressed {
                                if self.debug_placement() {
                                    self.select_debug_material(2);
                                } else {
                                    self.select_hotbar_slot(1);
                                }
                            }
                        }
                        KeyCode::Digit3 => {
                            if pressed {
                                if self.debug_placement() {
                                    self.select_debug_material(3);
                                } else {
                                    self.select_hotbar_slot(2);
                                }
                            }
                        }
                        KeyCode::Digit4 => {
                            if pressed {
                                if self.debug_placement() {
                                    self.select_debug_material(4);
                                } else {
                                    self.select_hotbar_slot(3);
                                }
                            }
                        }
                        KeyCode::Digit5 => {
                            if pressed {
                                if self.debug_placement() {
                                    self.select_debug_material(5);
                                } else {
                                    self.select_hotbar_slot(4);
                                }
                            }
                        }
                        KeyCode::Digit6 => {
                            if pressed {
                                if self.debug_placement() {
                                    self.select_debug_material(6);
                                } else {
                                    self.select_hotbar_slot(5);
                                }
                            }
                        }
                        KeyCode::Digit7 => {
                            if pressed {
                                if self.debug_placement() {
                                    self.select_debug_material(7);
                                } else {
                                    self.select_hotbar_slot(6);
                                }
                            }
                        }
                        KeyCode::Digit8 => {
                            if pressed {
                                if self.debug_placement() {
                                    self.select_debug_material(8);
                                } else {
                                    self.select_hotbar_slot(7);
                                }
                            }
                        }
                        KeyCode::Digit9 => {
                            if pressed {
                                if self.debug_placement() {
                                    self.select_debug_material(9);
                                } else {
                                    self.select_hotbar_slot(8);
                                }
                            }
                        }

                        // UI toggles - all panels are now dock tabs
                        KeyCode::F1 => {
                            if pressed {
                                self.ui_state.toggle_tab(crate::ui::DockTab::Stats);
                            }
                        }
                        KeyCode::F2 => {
                            if pressed {
                                self.renderer.toggle_active_chunks_overlay();
                            }
                        }
                        #[cfg(feature = "profiling")]
                        KeyCode::F3 => {
                            if pressed {
                                self.ui_state.toggle_tab(crate::ui::DockTab::Profiler);
                            }
                        }
                        #[cfg(not(target_arch = "wasm32"))]
                        KeyCode::F4 => {
                            if pressed {
                                self.ui_state.toggle_tab(crate::ui::DockTab::Parameters);
                            }
                        }
                        #[cfg(not(target_arch = "wasm32"))]
                        KeyCode::F6 => {
                            if pressed {
                                self.ui_state.toggle_tab(crate::ui::DockTab::Logger);
                            }
                        }
                        KeyCode::KeyH => {
                            if pressed {
                                self.ui_state.toggle_tab(crate::ui::DockTab::Controls);
                            }
                        }
                        KeyCode::KeyT => {
                            if pressed {
                                self.renderer.toggle_temperature_overlay();
                            }
                        }
                        KeyCode::KeyV => {
                            if pressed {
                                self.renderer.toggle_light_overlay();
                            }
                        }
                        KeyCode::KeyL => {
                            if pressed {
                                self.ui_state.toggle_tab(crate::ui::DockTab::LevelSelector);
                            }
                        }
                        KeyCode::KeyI => {
                            if pressed {
                                self.ui_state.toggle_tab(crate::ui::DockTab::Inventory);
                            }
                        }
                        KeyCode::KeyC => {
                            if pressed {
                                self.ui_state.toggle_tab(crate::ui::DockTab::Crafting);
                            }
                        }
                        KeyCode::KeyG => {
                            if pressed {
                                use crate::creature::genome::CreatureGenome;

                                // Check population limit
                                if self.world.creature_manager.can_spawn() {
                                    // Randomly select genome
                                    let genome = match rand::random::<u8>() % 3 {
                                        0 => CreatureGenome::test_biped(),
                                        1 => CreatureGenome::test_quadruped(),
                                        _ => CreatureGenome::test_worm(),
                                    };

                                    let id = self.world.spawn_creature_at_player(genome);
                                    log::info!("Spawned creature {} at player position", id);
                                } else {
                                    log::warn!("Cannot spawn: population limit reached");
                                }
                            }
                        }

                        // Manual save (F5)
                        KeyCode::F5 => {
                            if pressed && matches!(self.game_mode, GameMode::PersistentWorld) {
                                self.world.save_all_dirty_chunks();
                                self.ui_state.show_toast("World saved!");
                                log::info!("Manual save completed");
                            }
                        }

                        // Zoom controls
                        KeyCode::Equal | KeyCode::NumpadAdd => {
                            if pressed {
                                self.input_state.zoom_delta *= self.zoom_speed();
                                log::debug!("Zoom in: delta={:.2}", self.input_state.zoom_delta);
                            }
                        }
                        KeyCode::Minus | KeyCode::NumpadSubtract => {
                            if pressed {
                                self.input_state.zoom_delta /= self.zoom_speed();
                                log::debug!("Zoom out: delta={:.2}", self.input_state.zoom_delta);
                            }
                        }

                        _ => {}
                    }
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                let (window_width, window_height) = self.renderer.window_size();
                let world_pos = screen_to_world(
                    position.x,
                    position.y,
                    window_width,
                    window_height,
                    self.world.player.position,
                    self.renderer.camera_zoom(),
                );
                self.input_state.mouse_world_pos = Some(world_pos);
                log::trace!(
                    "Mouse: screen({:.0}, {:.0}) → world({}, {})",
                    position.x,
                    position.y,
                    world_pos.0,
                    world_pos.1
                );
            }
            WindowEvent::MouseInput { state, button, .. } => match button {
                MouseButton::Left => {
                    self.input_state.left_mouse_pressed = state == ElementState::Pressed;
                    log::debug!(
                        "Left mouse: {}",
                        if state == ElementState::Pressed {
                            "pressed"
                        } else {
                            "released"
                        }
                    );
                }
                MouseButton::Right => {
                    self.input_state.right_mouse_pressed = state == ElementState::Pressed;
                    log::debug!(
                        "Right mouse: {}",
                        if state == ElementState::Pressed {
                            "pressed"
                        } else {
                            "released"
                        }
                    );
                }
                _ => {}
            },
            WindowEvent::MouseWheel { delta, .. } => {
                // Skip input if egui wants it
                if self.egui_ctx.wants_pointer_input() {
                    return;
                }

                let scroll_amount = match delta {
                    winit::event::MouseScrollDelta::LineDelta(_, y) => y,
                    winit::event::MouseScrollDelta::PixelDelta(pos) => {
                        (pos.y / 50.0) as f32 // Normalize pixel deltas
                    }
                };

                // Zoom in/out based on scroll direction
                let zoom_factor = 1.0 + (scroll_amount * 0.1);
                self.input_state.zoom_delta *= zoom_factor;
                log::debug!(
                    "Mouse wheel: scroll={:.2}, zoom_delta={:.2}",
                    scroll_amount,
                    self.input_state.zoom_delta
                );
            }
            WindowEvent::RedrawRequested => {
                self.handle_redraw();
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        self.window.request_redraw();
    }
}
