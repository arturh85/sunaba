//! Application state and main game loop

use anyhow::Result;
use glam::Vec2;
use instant::{Duration, Instant};
use winit::{
    application::ApplicationHandler,
    event::{ElementState, MouseButton, WindowEvent},
    event_loop::{ActiveEventLoop, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    window::{Window, WindowAttributes},
};

use crate::levels::LevelManager;
use crate::render::Renderer;
use crate::simulation::MaterialId;
use crate::ui::UiState;
use crate::world::World;

/// Game mode: persistent world or demo level
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameMode {
    PersistentWorld,
    DemoLevel(usize),
}

// Zoom constants
const ZOOM_SPEED: f32 = 1.1; // Multiplicative zoom factor per keypress
const MIN_ZOOM: f32 = 0.001; // Max zoom out (see more of the world)
const MAX_ZOOM: f32 = 0.5; // Max zoom in (closer view)

/// Print controls to console
fn print_controls() {
    println!("=== Sunaba Controls ===");
    println!("Movement: WASD");
    println!("Jump: Space");
    println!("Zoom: +/- or Mouse Wheel");
    println!("Materials: 1-9 (Stone, Sand, Water, Wood, Fire, Smoke, Steam, Lava, Oil)");
    println!("Spawn: Left Click");
    println!("Spawn Creature: G");
    println!("Toggle Temperature Overlay: T");
    println!("Toggle Stats: F1");
    println!("Toggle Help: H");
    println!("Level Selector: L");
    println!("Manual Save: F5");
    println!("======================");
}

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

    log::trace!("screen_to_world: screen({:.0},{:.0}) → ndc({:.2},{:.2}) → world({:.1},{:.1}) [aspect={:.2}, zoom={:.2}, cam={:?}]",
               screen_x, screen_y, ndc_x, ndc_y, world_x, world_y, aspect, camera_zoom, camera_pos);

    (world_x as i32, world_y as i32)
}

/// Tracks current input state
pub struct InputState {
    // Movement keys
    pub w_pressed: bool,
    pub a_pressed: bool,
    pub s_pressed: bool,
    pub d_pressed: bool,
    pub jump_pressed: bool, // Space bar for jumping

    // Material selection (1-9 map to material IDs)
    pub selected_material: u16,

    // Mouse state
    pub mouse_world_pos: Option<(i32, i32)>, // Converted to world coords
    pub left_mouse_pressed: bool,
    pub right_mouse_pressed: bool,
    pub prev_right_mouse_pressed: bool, // Previous frame's right mouse state

    // Zoom control
    pub zoom_delta: f32, // Zoom change this frame (1.0 = no change)
}

impl InputState {
    pub fn new() -> Self {
        Self {
            w_pressed: false,
            a_pressed: false,
            s_pressed: false,
            d_pressed: false,
            jump_pressed: false,
            selected_material: MaterialId::SAND, // Start with sand
            mouse_world_pos: None,
            left_mouse_pressed: false,
            right_mouse_pressed: false,
            prev_right_mouse_pressed: false,
            zoom_delta: 1.0, // No change by default
        }
    }
}

impl Default for InputState {
    fn default() -> Self {
        Self::new()
    }
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
}

impl App {
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
        let window_attrs = {
            WindowAttributes::default()
                .with_title("Sunaba - 2D Physics Sandbox")
                .with_inner_size(winit::dpi::LogicalSize::new(1280, 720))
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

        // Print controls to console
        print_controls();
        log::info!("Loaded persistent world");

        let app = Self {
            window,
            renderer,
            world,
            input_state: InputState::default(),
            egui_ctx,
            egui_state,
            ui_state: UiState::new(),
            level_manager,
            game_mode,
            last_autosave: Instant::now(),
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

    pub fn run(event_loop: EventLoop<()>, mut app: Self) -> Result<()> {
        event_loop.run_app(&mut app)?;
        Ok(())
    }

    fn handle_redraw(&mut self) {
        // Begin frame timing
        self.ui_state.stats.begin_frame();

        // Periodic auto-save (every 60 seconds in persistent world mode)
        const AUTOSAVE_INTERVAL: Duration = Duration::from_secs(60);
        if matches!(self.game_mode, GameMode::PersistentWorld)
            && self.last_autosave.elapsed() >= AUTOSAVE_INTERVAL
        {
            self.world.save_all_dirty_chunks(); // Save chunks AND player data
            self.last_autosave = Instant::now();
            log::info!("Auto-saved world and player data");
        }

        // Update player from input
        self.world.update_player(&self.input_state, 1.0 / 60.0);

        // Update camera zoom
        self.renderer
            .update_zoom(self.input_state.zoom_delta, MIN_ZOOM, MAX_ZOOM);

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
        // Only trigger on initial click, not hold
        if self.input_state.right_mouse_pressed && !self.input_state.prev_right_mouse_pressed {
            let player_pos = self.world.player.position;
            let center_x = player_pos.x as i32;
            let center_y = player_pos.y as i32;
            self.world.debug_mine_circle(center_x, center_y, 13);
        }

        // Placing material from inventory with left mouse button
        if self.input_state.left_mouse_pressed {
            if let Some((wx, wy)) = self.input_state.mouse_world_pos {
                self.world.place_material_from_inventory(
                    wx,
                    wy,
                    self.input_state.selected_material,
                );
            }
        }

        // Update simulation with timing
        self.ui_state.stats.begin_sim();
        self.world.update(1.0 / 60.0, &mut self.ui_state.stats);
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
            );

            // Render crafting UI and handle crafting output
            if let Some(output) = self.ui_state.crafting_ui.render(
                ctx,
                &mut self.world.player.inventory,
                &self.world.recipe_registry,
                &self.world.materials,
            ) {
                // Handle crafted output
                use crate::entity::crafting::RecipeOutput;
                match output {
                    RecipeOutput::Material { id, count } => {
                        let added = self.world.player.inventory.add_item(id, count);
                        if added == count {
                            log::info!(
                                "[CRAFTING] Added {} x{} to inventory",
                                self.world.materials.get(id).name,
                                count
                            );
                        } else {
                            log::warn!(
                                "[CRAFTING] Inventory full, only added {} of {} items",
                                added,
                                count
                            );
                        }
                    }
                    RecipeOutput::Tool {
                        tool_id,
                        durability,
                    } => {
                        if self.world.player.inventory.add_tool(tool_id, durability) {
                            log::info!("[CRAFTING] Added tool {} to inventory", tool_id);
                        } else {
                            log::warn!("[CRAFTING] Inventory full, could not add crafted tool");
                        }
                    }
                }
            }
        });
        let egui_build_time = egui_build_start.elapsed().as_secs_f32() * 1000.0;

        // Handle level selector actions
        if self.ui_state.level_selector.return_to_world {
            self.game_mode = GameMode::PersistentWorld;
            self.world.load_persistent_world();
            self.ui_state.level_selector.reset_flags(); // Reset flag after processing
            log::info!("Returned to persistent world");
        } else if let Some(level_id) = self.ui_state.level_selector.selected_level {
            // Save current world if in persistent mode
            if matches!(self.game_mode, GameMode::PersistentWorld) {
                self.world.save_all_dirty_chunks();
            }

            self.game_mode = GameMode::DemoLevel(level_id);
            self.level_manager.load_level(level_id, &mut self.world);
            self.ui_state.level_selector.reset_flags(); // Reset flag after processing
            log::info!(
                "Switched to demo level {}: {}",
                level_id,
                self.level_manager.current_level_name()
            );
        }

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

        // Render world + UI
        match self.renderer.render(
            &mut self.world,
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

                        // Hotbar selection (0-9) - select inventory slot and equip/unequip tools
                        KeyCode::Digit0 => {
                            if pressed {
                                self.select_hotbar_slot(9); // 0 key = slot 9
                            }
                        }
                        KeyCode::Digit1 => {
                            if pressed {
                                self.select_hotbar_slot(0);
                            }
                        }
                        KeyCode::Digit2 => {
                            if pressed {
                                self.select_hotbar_slot(1);
                            }
                        }
                        KeyCode::Digit3 => {
                            if pressed {
                                self.select_hotbar_slot(2);
                            }
                        }
                        KeyCode::Digit4 => {
                            if pressed {
                                self.select_hotbar_slot(3);
                            }
                        }
                        KeyCode::Digit5 => {
                            if pressed {
                                self.select_hotbar_slot(4);
                            }
                        }
                        KeyCode::Digit6 => {
                            if pressed {
                                self.select_hotbar_slot(5);
                            }
                        }
                        KeyCode::Digit7 => {
                            if pressed {
                                self.select_hotbar_slot(6);
                            }
                        }
                        KeyCode::Digit8 => {
                            if pressed {
                                self.select_hotbar_slot(7);
                            }
                        }
                        KeyCode::Digit9 => {
                            if pressed {
                                self.select_hotbar_slot(8);
                            }
                        }

                        // UI toggles
                        KeyCode::F1 => {
                            if pressed {
                                self.ui_state.toggle_stats();
                            }
                        }
                        KeyCode::KeyH => {
                            if pressed {
                                self.ui_state.toggle_help();
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
                                self.ui_state.toggle_level_selector();
                            }
                        }
                        KeyCode::KeyI => {
                            if pressed {
                                self.ui_state.toggle_inventory();
                            }
                        }
                        KeyCode::KeyC => {
                            if pressed {
                                self.ui_state.toggle_crafting();
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
                                self.input_state.zoom_delta *= ZOOM_SPEED;
                                log::debug!("Zoom in: delta={:.2}", self.input_state.zoom_delta);
                            }
                        }
                        KeyCode::Minus | KeyCode::NumpadSubtract => {
                            if pressed {
                                self.input_state.zoom_delta /= ZOOM_SPEED;
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
