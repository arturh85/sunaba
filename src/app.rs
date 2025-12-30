//! Application state and main game loop

use winit::{
    event::{Event, WindowEvent, ElementState, MouseButton},
    event_loop::EventLoop,
    window::{Window, WindowBuilder},
    keyboard::{KeyCode, PhysicalKey},
};
use anyhow::Result;
use glam::Vec2;

use crate::world::World;
use crate::render::Renderer;
use crate::simulation::MaterialId;

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
}

impl App {
    pub async fn new() -> Result<Self> {
        let event_loop = EventLoop::new()?;
        let window = WindowBuilder::new()
            .with_title("Sunaba")
            .with_inner_size(winit::dpi::LogicalSize::new(1280, 720))
            .build(&event_loop)?;
        
        let renderer = Renderer::new(&window).await?;
        let world = World::new();
        
        Ok(Self {
            window,
            event_loop,
            renderer,
            world,
            input_state: InputState::default(),
        })
    }
    
    pub fn run(self) -> Result<()> {
        let Self { window, event_loop, mut renderer, mut world, mut input_state } = self;
        
        event_loop.run(move |event, elwt| {
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
                    if let PhysicalKey::Code(code) = key_event.physical_key {
                        let pressed = key_event.state == ElementState::Pressed;
                        log::debug!("Keyboard: {:?} {}", code, if pressed { "pressed" } else { "released" });

                        match code {
                            // Movement keys
                            KeyCode::KeyW => input_state.w_pressed = pressed,
                            KeyCode::KeyA => input_state.a_pressed = pressed,
                            KeyCode::KeyS => input_state.s_pressed = pressed,
                            KeyCode::KeyD => input_state.d_pressed = pressed,

                            // Material selection (1-9)
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
                        world.player_pos,
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
                    if button == MouseButton::Left {
                        input_state.left_mouse_pressed = state == ElementState::Pressed;
                        log::info!("Mouse: {}", if state == ElementState::Pressed { "clicked" } else { "released" });
                    }
                }
                Event::WindowEvent { event: WindowEvent::RedrawRequested, .. } => {
                    // Update player from input
                    world.update_player(&input_state, 1.0 / 60.0);

                    // Log camera state periodically
                    static mut FRAME_COUNT: u32 = 0;
                    unsafe {
                        FRAME_COUNT += 1;
                        if FRAME_COUNT % 120 == 0 {  // Every 2 seconds at 60fps
                            log::info!("Frame {}: player_pos={:?}, zoom={:.2}, selected_material={}",
                                       FRAME_COUNT, world.player_pos, renderer.camera_zoom(), input_state.selected_material);
                        }
                    }

                    // Spawn material on mouse click
                    if input_state.left_mouse_pressed {
                        if let Some((wx, wy)) = input_state.mouse_world_pos {
                            world.spawn_material(wx, wy, input_state.selected_material);
                        }
                    }

                    // Update simulation
                    world.update(1.0 / 60.0);

                    // Render
                    if let Err(e) = renderer.render(&world) {
                        log::error!("Render error: {e}");
                    }
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
