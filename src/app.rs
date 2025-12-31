//! Application state and main game loop

use winit::{
    event::{Event, WindowEvent, ElementState, MouseButton},
    event_loop::EventLoop,
    window::{Window, WindowBuilder},
    keyboard::{KeyCode, PhysicalKey},
};
use anyhow::Result;
use glam::Vec2;
use std::time::{Duration, Instant};

use crate::world::World;
use crate::render::Renderer;
use crate::simulation::MaterialId;
use crate::ui::UiState;
use crate::levels::LevelManager;

/// Game mode: persistent world or demo level
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameMode {
    PersistentWorld,
    DemoLevel(usize),
}

// Zoom constants
const ZOOM_SPEED: f32 = 1.1; // Multiplicative zoom factor per keypress
const MIN_ZOOM: f32 = 0.001; // Max zoom out (see more of the world)
const MAX_ZOOM: f32 = 0.5;   // Max zoom in (closer view)

/// Print controls to console
fn print_controls() {
    println!("=== Sunaba Controls ===");
    println!("Movement: WASD");
    println!("Zoom: +/- or Mouse Wheel");
    println!("Materials: 1-9 (Stone, Sand, Water, Wood, Fire, Smoke, Steam, Lava, Oil)");
    println!("Spawn: Left Click");
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

    // Material selection (1-9 map to material IDs)
    pub selected_material: u16,

    // Mouse state
    pub mouse_world_pos: Option<(i32, i32)>, // Converted to world coords
    pub left_mouse_pressed: bool,
    pub right_mouse_pressed: bool,

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
            selected_material: MaterialId::SAND, // Start with sand
            mouse_world_pos: None,
            left_mouse_pressed: false,
            right_mouse_pressed: false,
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
    event_loop: EventLoop<()>,
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
    pub async fn new() -> Result<Self> {
        let event_loop = EventLoop::new()?;
        let window = WindowBuilder::new()
            .with_title("Sunaba - 2D Physics Sandbox")
            .with_inner_size(winit::dpi::LogicalSize::new(1280, 720))
            .build(&event_loop)?;

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
        );

        // Print controls to console
        print_controls();
        log::info!("Loaded persistent world");

        Ok(Self {
            window,
            event_loop,
            renderer,
            world,
            input_state: InputState::default(),
            egui_ctx,
            egui_state,
            ui_state: UiState::new(),
            level_manager,
            game_mode,
            last_autosave: Instant::now(),
        })
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
        log::info!("Switched to demo level {}: {}", level_id, self.level_manager.current_level_name());
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
            GameMode::DemoLevel(id) => format!("Demo Level {}: {}", id + 1, self.level_manager.current_level_name()),
        }
    }
    
    pub fn run(self) -> Result<()> {
        let Self { window, event_loop, mut renderer, mut world, mut input_state, egui_ctx, mut egui_state, mut ui_state, mut level_manager, mut game_mode, mut last_autosave } = self;
        
        event_loop.run(move |event, elwt| {
            // Let egui handle events first
            if let Event::WindowEvent { ref event, .. } = event {
                let _ = egui_state.on_window_event(&window, event);
            }

            match event {
                Event::WindowEvent { event: WindowEvent::CloseRequested, .. } => {
                    elwt.exit();
                }
                Event::WindowEvent { event: WindowEvent::Resized(size), .. } => {
                    renderer.resize(size.width, size.height);
                }
                Event::WindowEvent {
                    event: WindowEvent::KeyboardInput { event: key_event, .. },
                    ..
                } => {
                    // Skip input if egui wants it
                    if egui_ctx.wants_keyboard_input() {
                        return;
                    }
                    if let PhysicalKey::Code(code) = key_event.physical_key {
                        let pressed = key_event.state == ElementState::Pressed;
                        log::debug!("Keyboard: {:?} {}", code, if pressed { "pressed" } else { "released" });

                        match code {
                            // Movement keys
                            KeyCode::KeyW => input_state.w_pressed = pressed,
                            KeyCode::KeyA => input_state.a_pressed = pressed,
                            KeyCode::KeyS => input_state.s_pressed = pressed,
                            KeyCode::KeyD => input_state.d_pressed = pressed,

                            // Material selection (0-9)
                            KeyCode::Digit0 => if pressed {
                                input_state.selected_material = MaterialId::AIR;
                            },
                            KeyCode::Digit1 => if pressed {
                                input_state.selected_material = MaterialId::STONE;
                            },
                            KeyCode::Digit2 => if pressed {
                                input_state.selected_material = MaterialId::SAND;
                            },
                            KeyCode::Digit3 => if pressed {
                                input_state.selected_material = MaterialId::WATER;
                            },
                            KeyCode::Digit4 => if pressed {
                                input_state.selected_material = MaterialId::WOOD;
                            },
                            KeyCode::Digit5 => if pressed {
                                input_state.selected_material = MaterialId::FIRE;
                            },
                            KeyCode::Digit6 => if pressed {
                                input_state.selected_material = MaterialId::SMOKE;
                            },
                            KeyCode::Digit7 => if pressed {
                                input_state.selected_material = MaterialId::STEAM;
                            },
                            KeyCode::Digit8 => if pressed {
                                input_state.selected_material = MaterialId::LAVA;
                            },
                            KeyCode::Digit9 => if pressed {
                                input_state.selected_material = MaterialId::OIL;
                            },

                            // UI toggles
                            KeyCode::F1 => if pressed {
                                ui_state.toggle_stats();
                            },
                            KeyCode::KeyH => if pressed {
                                ui_state.toggle_help();
                            },
                            KeyCode::KeyT => if pressed {
                                renderer.toggle_temperature_overlay();
                            },
                            KeyCode::KeyL => if pressed {
                                ui_state.toggle_level_selector();
                            },
                            KeyCode::KeyI => if pressed {
                                ui_state.toggle_inventory();
                            },

                            // Manual save (F5)
                            KeyCode::F5 => if pressed
                                && matches!(game_mode, GameMode::PersistentWorld) {
                                    world.save_all_dirty_chunks();
                                    ui_state.show_toast("World saved!");
                                    log::info!("Manual save completed");
                                },

                            // Zoom controls
                            KeyCode::Equal | KeyCode::NumpadAdd => if pressed {
                                input_state.zoom_delta *= ZOOM_SPEED;
                                log::debug!("Zoom in: delta={:.2}", input_state.zoom_delta);
                            },
                            KeyCode::Minus | KeyCode::NumpadSubtract => if pressed {
                                input_state.zoom_delta /= ZOOM_SPEED;
                                log::debug!("Zoom out: delta={:.2}", input_state.zoom_delta);
                            },

                            _ => {}
                        }
                    }
                }
                Event::WindowEvent {
                    event: WindowEvent::CursorMoved { position, .. },
                    ..
                } => {
                    let (window_width, window_height) = renderer.window_size();
                    let world_pos = screen_to_world(
                        position.x,
                        position.y,
                        window_width,
                        window_height,
                        world.player.position,
                        renderer.camera_zoom(),
                    );
                    input_state.mouse_world_pos = Some(world_pos);
                    log::trace!("Mouse: screen({:.0}, {:.0}) → world({}, {})",
                               position.x, position.y, world_pos.0, world_pos.1);
                }
                Event::WindowEvent {
                    event: WindowEvent::MouseInput { state, button, .. },
                    ..
                } => {
                    match button {
                        MouseButton::Left => {
                            input_state.left_mouse_pressed = state == ElementState::Pressed;
                            log::debug!("Left mouse: {}", if state == ElementState::Pressed { "pressed" } else { "released" });
                        }
                        MouseButton::Right => {
                            input_state.right_mouse_pressed = state == ElementState::Pressed;
                            log::debug!("Right mouse: {}", if state == ElementState::Pressed { "pressed" } else { "released" });
                        }
                        _ => {}
                    }
                }
                Event::WindowEvent {
                    event: WindowEvent::MouseWheel { delta, .. },
                    ..
                } => {
                    // Skip input if egui wants it
                    if egui_ctx.wants_pointer_input() {
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
                    input_state.zoom_delta *= zoom_factor;
                    log::debug!("Mouse wheel: scroll={:.2}, zoom_delta={:.2}", scroll_amount, input_state.zoom_delta);
                }
                Event::WindowEvent { event: WindowEvent::RedrawRequested, .. } => {
                    // Begin frame timing
                    ui_state.stats.begin_frame();

                    // Periodic auto-save (every 60 seconds in persistent world mode)
                    const AUTOSAVE_INTERVAL: Duration = Duration::from_secs(60);
                    if matches!(game_mode, GameMode::PersistentWorld)
                        && last_autosave.elapsed() >= AUTOSAVE_INTERVAL {
                            world.save_all_dirty_chunks(); // Save chunks AND player data
                            last_autosave = Instant::now();
                            log::info!("Auto-saved world and player data");
                        }

                    // Update player from input
                    world.update_player(&input_state, 1.0 / 60.0);

                    // Update camera zoom
                    renderer.update_zoom(input_state.zoom_delta, MIN_ZOOM, MAX_ZOOM);

                    // Log camera state periodically
                    use std::sync::atomic::{AtomicU32, Ordering};
                    static FRAME_COUNT: AtomicU32 = AtomicU32::new(0);
                    let frame = FRAME_COUNT.fetch_add(1, Ordering::Relaxed);
                    if frame.is_multiple_of(120) {  // Every 2 seconds at 60fps
                        log::info!("Frame {}: player_pos={:?}, zoom={:.2}, selected_material={}",
                                   frame, world.player.position, renderer.camera_zoom(), input_state.selected_material);
                    }

                    // Mining with right mouse button
                    if input_state.right_mouse_pressed {
                        if let Some((wx, wy)) = input_state.mouse_world_pos {
                            world.mine_pixel(wx, wy);
                        }
                    }

                    // Placing material from inventory with left mouse button
                    if input_state.left_mouse_pressed {
                        if let Some((wx, wy)) = input_state.mouse_world_pos {
                            world.place_material_from_inventory(wx, wy, input_state.selected_material);
                        }
                    }

                    // Update simulation with timing
                    ui_state.stats.begin_sim();
                    world.update(1.0 / 60.0, &mut ui_state.stats);
                    ui_state.stats.end_sim();

                    // Collect world stats
                    ui_state.stats.collect_world_stats(&world);

                    // Update tooltip with world data
                    ui_state.update_tooltip(&world, world.materials(), input_state.mouse_world_pos);

                    // Prepare egui frame
                    let raw_input = egui_state.take_egui_input(&window);
                    let full_output = egui_ctx.run(raw_input, |ctx| {
                        // Get cursor position from egui context
                        let cursor_pos = ctx.pointer_hover_pos().unwrap_or(egui::pos2(0.0, 0.0));

                        // Get game mode description
                        let game_mode_desc = match game_mode {
                            GameMode::PersistentWorld => "Persistent World".to_string(),
                            GameMode::DemoLevel(id) => format!("Demo Level {}: {}", id + 1, level_manager.current_level_name()),
                        };
                        let in_persistent_world = matches!(game_mode, GameMode::PersistentWorld);

                        ui_state.render(ctx, cursor_pos, input_state.selected_material, world.materials(), &game_mode_desc, in_persistent_world, &level_manager, &world.player);
                    });

                    // Handle level selector actions
                    if ui_state.level_selector.return_to_world {
                        game_mode = GameMode::PersistentWorld;
                        world.load_persistent_world();
                        log::info!("Returned to persistent world");
                    } else if let Some(level_id) = ui_state.level_selector.selected_level {
                        // Save current world if in persistent mode
                        if matches!(game_mode, GameMode::PersistentWorld) {
                            world.save_all_dirty_chunks();
                        }

                        game_mode = GameMode::DemoLevel(level_id);
                        level_manager.load_level(level_id, &mut world);
                        log::info!("Switched to demo level {}: {}", level_id, level_manager.current_level_name());
                    }

                    // Handle egui output
                    egui_state.handle_platform_output(&window, full_output.platform_output);

                    // Update temperature overlay texture
                    renderer.update_temperature_overlay(&world);

                    // Render world + UI
                    if let Err(e) = renderer.render(&world, &egui_ctx, full_output.textures_delta, full_output.shapes) {
                        log::error!("Render error: {e}");
                    }

                    // Reset per-frame input state
                    input_state.zoom_delta = 1.0;
                }
                Event::AboutToWait => {
                    window.request_redraw();
                }
                _ => {}
            }
        })?;
        
        Ok(())
    }
}
