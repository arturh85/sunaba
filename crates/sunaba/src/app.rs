//! Application state and main game loop

use anyhow::Result;
use glam::Vec2;
use web_time::{Duration, Instant};
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

#[cfg(feature = "multiplayer")]
use crate::multiplayer::client::{
    DbContextTrait as _, PlayerTableAccessTrait as _, TableTrait as _,
};

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

/// Remote player data for rendering
/// Available in all builds for API consistency, but only populated when multiplayer is enabled
#[derive(Clone, Debug)]
pub struct RemotePlayerRenderData {
    pub x: f32,
    pub y: f32,
    pub vel_x: f32,
    pub vel_y: f32,
    pub name: Option<String>,
    pub identity: String,
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

    /// Multiplayer connection manager (handles state, client, reconnection)
    #[cfg(feature = "multiplayer")]
    multiplayer_manager: Option<crate::multiplayer::MultiplayerManager>,

    /// Track whether initial spawn chunks have loaded (multiplayer only)
    #[cfg(feature = "multiplayer")]
    multiplayer_initial_chunks_loaded: bool,

    /// Track last time we logged about waiting for chunks (throttle to 10s)
    #[cfg(feature = "multiplayer")]
    last_chunk_wait_log: Option<Instant>,

    /// Track when chunk loading started (for timeout detection)
    #[cfg(feature = "multiplayer")]
    chunk_loading_started_at: Option<Instant>,
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
        #[allow(unused_mut)] // mut only needed in singleplayer mode
        let mut world = World::new(false); // Spawn creatures in singleplayer (will be gated when connected to multiplayer)

        // Initialize level manager (but don't load a level yet)
        let level_manager = LevelManager::new();

        // In multiplayer mode, don't load local world - we'll receive state from server
        #[cfg(not(feature = "multiplayer"))]
        {
            world.load_persistent_world();
            log::info!("Loaded persistent world (singleplayer mode)");
        }

        #[cfg(feature = "multiplayer")]
        log::info!("Multiplayer mode - waiting for world state from server");

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
        #[allow(unused_mut)] // Mut needed for multiplayer feature
        let mut ui_state = UiState::new(&config);

        #[cfg(target_arch = "wasm32")]
        #[allow(unused_mut)] // Mut needed for multiplayer feature
        let mut ui_state = UiState::default();

        // Initialize multiplayer manager (starts disconnected)
        #[cfg(feature = "multiplayer")]
        let multiplayer_manager = {
            #[cfg(not(target_arch = "wasm32"))]
            {
                Some(crate::multiplayer::MultiplayerManager::new(
                    config.multiplayer.clone(),
                ))
            }
            #[cfg(target_arch = "wasm32")]
            {
                // WASM: use default config (no access to config file)
                Some(crate::multiplayer::MultiplayerManager::new(
                    crate::config::MultiplayerConfig::default(),
                ))
            }
        };

        #[cfg(feature = "multiplayer")]
        log::info!(
            "Multiplayer manager initialized (disconnected - use UI or --server arg to connect)"
        );

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
            #[cfg(feature = "multiplayer")]
            multiplayer_manager,
            #[cfg(feature = "multiplayer")]
            multiplayer_initial_chunks_loaded: false,
            #[cfg(feature = "multiplayer")]
            last_chunk_wait_log: None,
            #[cfg(feature = "multiplayer")]
            chunk_loading_started_at: None,
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

    /// Connect to a multiplayer server
    #[cfg(feature = "multiplayer")]
    pub async fn connect_to_server(&mut self, server_url: String) -> anyhow::Result<()> {
        let manager = self
            .multiplayer_manager
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("Multiplayer manager not initialized"))?;

        // Check if already connected
        if manager.state.is_connected() {
            log::warn!("Already connected to server");
            return Ok(());
        }

        log::info!("Connecting to server: {}", server_url);
        manager.start_connecting(server_url.clone());

        // Reset chunk loading state
        self.multiplayer_initial_chunks_loaded = false;
        self.chunk_loading_started_at = Some(Instant::now());
        self.last_chunk_wait_log = None;

        // Save singleplayer world before connecting
        log::info!("Saving singleplayer world...");
        self.world.save_all_dirty_chunks();
        manager.set_singleplayer_saved(true);

        // Attempt connection
        match manager.client.connect(&server_url, "sunaba").await {
            Ok(_) => {
                log::info!("Connected successfully");

                // Subscribe to world data
                if let Err(e) = manager.client.subscribe_world().await {
                    let error_msg = format!("Failed to subscribe to world: {}", e);
                    log::error!("{}", error_msg);
                    manager.mark_error(error_msg, server_url);
                    return Err(anyhow::anyhow!("Subscription failed"));
                }

                // Disable persistence for multiplayer world (server is authoritative)
                self.world.disable_persistence();

                // Mark as connected
                manager.mark_connected(server_url);

                // Initialize progressive chunk loading queue (3 chunks radius initially)
                manager.chunk_load_queue =
                    Some(crate::multiplayer::chunk_loader::ChunkLoadQueue::new(
                        glam::IVec2::ZERO, // Initial center at spawn
                        10,                // Max radius (will expand after spawn loads)
                        2,                 // Batch size (2 chunks per frame)
                    ));
                manager.subscription_center = glam::IVec2::ZERO;
                log::info!("Initialized progressive chunk loading (radius 10, batch size 2)");

                // Initialize metrics collector
                self.ui_state.metrics_collector =
                    Some(crate::multiplayer::metrics::MetricsCollector::new());

                log::info!("Multiplayer connection established");
                Ok(())
            }
            Err(e) => {
                let error_msg = format!("Connection failed: {}", e);
                log::error!("{}", error_msg);
                manager.mark_error(error_msg.clone(), server_url);
                Err(anyhow::anyhow!(error_msg))
            }
        }
    }

    /// Disconnect from multiplayer server and restore singleplayer world
    #[cfg(feature = "multiplayer")]
    pub async fn disconnect_from_server(&mut self) -> anyhow::Result<()> {
        let manager = self
            .multiplayer_manager
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("Multiplayer manager not initialized"))?;

        if manager.state.is_disconnected() {
            log::warn!("Already disconnected");
            return Ok(());
        }

        log::info!("Disconnecting from server...");

        // Disconnect client
        manager.client.disconnect().await?;

        // Clear multiplayer world chunks
        // Note: World has no clear_all method, but we'll reload singleplayer which replaces chunks
        log::info!("Restoring singleplayer world...");

        // Restore singleplayer world from disk
        self.world.load_persistent_world();

        // Mark as disconnected
        manager.mark_disconnected();

        // Clear metrics collector
        self.ui_state.metrics_collector = None;

        // Reset chunk loading state
        self.multiplayer_initial_chunks_loaded = false;
        self.chunk_loading_started_at = None;
        self.last_chunk_wait_log = None;

        log::info!("Disconnected - singleplayer world restored");
        Ok(())
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
                self.renderer.set_water_noise_params(
                    self.config.rendering.water_noise_frequency,
                    self.config.rendering.water_noise_speed,
                    self.config.rendering.water_noise_amplitude,
                );
                self.renderer.set_lava_noise_params(
                    self.config.rendering.lava_noise_frequency,
                    self.config.rendering.lava_noise_speed,
                    self.config.rendering.lava_noise_amplitude,
                );

                // Apply bloom settings
                if self.config.rendering.bloom_enabled {
                    self.renderer
                        .enable_bloom(self.config.rendering.bloom_quality);
                } else {
                    self.renderer.disable_bloom();
                }

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

        // Process SpacetimeDB messages (native multiplayer only)
        #[cfg(all(not(target_arch = "wasm32"), feature = "multiplayer"))]
        {
            if let Some(manager) = self.multiplayer_manager.as_mut() {
                manager.client.frame_tick();

                // Progressive chunk sync with rate limiting (2-3 chunks per frame)
                if let Some(ref mut queue) = manager.chunk_load_queue {
                    if let Ok(synced) = manager
                        .client
                        .sync_chunks_progressive(&mut self.world, queue)
                    {
                        if synced > 0 {
                            log::debug!("Loaded {} chunks (progressive)", synced);
                        }
                    } else {
                        log::error!("Failed to sync chunks progressively");
                    }
                }

                // Evict chunks >10 from player (multiplayer mode)
                if manager.state.is_connected() {
                    self.world.evict_distant_chunks(self.world.player.position);
                }
            }

            // Wait for initial spawn chunks before rendering player (check after sync)
            if !self.multiplayer_initial_chunks_loaded {
                use glam::IVec2;

                let spawn_chunk_positions = vec![
                    IVec2::new(0, 0), // Center
                    IVec2::new(-1, 0),
                    IVec2::new(1, 0), // Sides
                    IVec2::new(0, -1),
                    IVec2::new(0, 1), // Top/bottom
                ];

                let all_loaded = spawn_chunk_positions
                    .iter()
                    .all(|pos| self.world.has_chunk(*pos));

                if all_loaded {
                    self.multiplayer_initial_chunks_loaded = true;
                    self.chunk_loading_started_at = None; // Clear timer
                    log::info!(
                        "Initial spawn chunks loaded ({}), enabling player rendering",
                        spawn_chunk_positions.len()
                    );

                    // Expand subscription to larger radius now that spawn is loaded
                    if let Some(manager) = self.multiplayer_manager.as_mut() {
                        if manager.state.is_connected() {
                            if let Err(e) =
                                manager.client.expand_chunk_subscription(IVec2::ZERO, 10)
                            {
                                log::error!("Failed to expand chunk subscription: {}", e);
                            } else {
                                log::info!("Expanded chunk subscription to radius 10");
                            }
                        }
                    }
                } else {
                    // Chunks still loading - throttle logging and check for timeout
                    let now = Instant::now();
                    let elapsed = self
                        .chunk_loading_started_at
                        .map(|start| now.duration_since(start))
                        .unwrap_or(std::time::Duration::ZERO);

                    // Check for timeout (60 seconds)
                    if elapsed.as_secs() >= 60 {
                        log::error!(
                            "Chunk loading timeout after {}s - disconnecting",
                            elapsed.as_secs()
                        );
                        // Trigger error state (need mutable access)
                        if let Some(manager) = self.multiplayer_manager.as_mut() {
                            manager.mark_error(
                                format!("Failed to load chunks after {}s", elapsed.as_secs()),
                                manager.state.server_url().unwrap_or("unknown").to_string(),
                            );
                        }
                        self.chunk_loading_started_at = None;
                    } else {
                        // Throttled logging - only log every 10 seconds
                        let should_log = self
                            .last_chunk_wait_log
                            .map(|last_log| now.duration_since(last_log).as_secs() >= 10)
                            .unwrap_or(true); // Log on first check

                        if should_log {
                            log::info!(
                                "Waiting for spawn chunks... (waited {}s, have {}/5 chunks)",
                                elapsed.as_secs(),
                                spawn_chunk_positions
                                    .iter()
                                    .filter(|pos| self.world.has_chunk(**pos))
                                    .count()
                            );
                            self.last_chunk_wait_log = Some(now);
                        }
                    }
                }
            }

            // Check for re-subscription every 60 frames (~1 second at 60fps)
            if let Some(manager) = self.multiplayer_manager.as_mut() {
                use std::sync::atomic::{AtomicU32, Ordering};
                static RESUB_CHECK_FRAME: AtomicU32 = AtomicU32::new(0);
                let frame = RESUB_CHECK_FRAME.fetch_add(1, Ordering::Relaxed);

                if frame % 60 == 0 && manager.state.is_connected() {
                    use glam::IVec2;
                    let player_chunk = IVec2::new(
                        (self.world.player.position.x as i32).div_euclid(64),
                        (self.world.player.position.y as i32).div_euclid(64),
                    );

                    let distance = (player_chunk - manager.subscription_center)
                        .abs()
                        .max_element();

                    if distance > 8 {
                        log::info!(
                            "Player moved >8 chunks from subscription center ({:?} -> {:?}, distance {}), re-subscribing",
                            manager.subscription_center,
                            player_chunk,
                            distance
                        );

                        // Re-subscribe with new center (unsubscribe → subscribe, no reconnection)
                        if let Err(e) = manager.client.resubscribe_chunks(player_chunk, 10) {
                            log::error!("Failed to re-subscribe chunks: {}", e);
                        } else {
                            // Reset chunk queue with new center
                            if let Some(ref mut queue) = manager.chunk_load_queue {
                                queue.reset_center(player_chunk, 10);
                            }
                            manager.subscription_center = player_chunk;
                            log::info!("Re-subscribed to chunks around {:?}", player_chunk);
                        }
                    }
                }
            }

            // Update metrics collector
            if let Some(manager) = self.multiplayer_manager.as_ref() {
                // Update metrics collector
                if let Some(ref mut collector) = self.ui_state.metrics_collector {
                    // Record this update
                    collector.record_update();

                    // Send ping periodically (only when connected)
                    if manager.state.is_connected() {
                        collector.send_ping(&manager.client);
                    }

                    // Update server metrics from latest data
                    if let Some(server_metrics) = manager.client.get_latest_server_metrics() {
                        collector.update_server_metrics(&server_metrics);
                    }
                }
            }
        }

        // Update metrics collector (WASM multiplayer only)
        #[cfg(all(target_arch = "wasm32", feature = "multiplayer"))]
        {
            if let Some(manager) = self.multiplayer_manager.as_ref() {
                if let Some(ref mut collector) = self.ui_state.metrics_collector {
                    // Record this update
                    collector.record_update();

                    // Send ping periodically (only when connected)
                    if manager.state.is_connected() {
                        collector.send_ping(&manager.client);
                    }

                    // Update server metrics from latest data
                    if let Some(server_metrics) = manager.client.get_latest_server_metrics() {
                        collector.update_server_metrics(&server_metrics);
                    }
                }
            }
        }

        // CRITICAL: Skip ALL game simulation when loading multiplayer chunks
        #[cfg(feature = "multiplayer")]
        let is_loading_chunks = self
            .multiplayer_manager
            .as_ref()
            .map(|m| m.state.is_connected() && !self.multiplayer_initial_chunks_loaded)
            .unwrap_or(false);
        #[cfg(not(feature = "multiplayer"))]
        let is_loading_chunks = false;

        if is_loading_chunks {
            // Skip player update, world update, input processing
            // Jump directly to rendering (after world update section)
        } else {
            // Normal game loop - update player from input
            self.world.update_player(&self.input_state, 1.0 / 60.0);

            // Send player position to server (multiplayer only)
            #[cfg(feature = "multiplayer")]
            {
                if let Some(manager) = self.multiplayer_manager.as_ref() {
                    if manager.state.is_connected() {
                        let pos = self.world.player.position;
                        let vel = self.world.player.velocity;
                        if let Err(e) = manager
                            .client
                            .update_player_position(pos.x, pos.y, vel.x, vel.y)
                        {
                            log::warn!("Failed to send player position to server: {}", e);
                        }
                    }
                }
            }

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

                // Send mining action to server (multiplayer only)
                #[cfg(feature = "multiplayer")]
                {
                    if let Some(manager) = self.multiplayer_manager.as_ref() {
                        if manager.state.is_connected() {
                            if let Err(e) = manager.client.mine(center_x, center_y) {
                                log::warn!("Failed to send mining action to server: {}", e);
                            }
                        }
                    }
                }

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

                #[cfg(not(target_arch = "wasm32"))]
                let brush_size = self.config.debug.brush_size;
                #[cfg(target_arch = "wasm32")]
                let brush_size = 1; // Default brush size for WASM builds

                if self.debug_placement() {
                    self.world
                        .place_material_debug(wx, wy, material_id, brush_size);
                } else {
                    self.world
                        .place_material_from_inventory(wx, wy, material_id, brush_size);
                }

                // Send material placement to server (multiplayer only)
                #[cfg(feature = "multiplayer")]
                {
                    if let Some(manager) = self.multiplayer_manager.as_ref() {
                        if manager.state.is_connected() {
                            if let Err(e) = manager.client.place_material(wx, wy, material_id) {
                                log::warn!("Failed to send material placement to server: {}", e);
                            }
                        }
                    }
                }

                // Spawn particles at placement location
                let pos = Vec2::new(wx as f32, wy as f32);
                if is_liquid {
                    self.particle_system.spawn_liquid_splash(pos, color);
                } else {
                    self.particle_system.spawn_impact_burst(pos, color);
                }
            }

            // Update simulation with timing (disabled when connected to multiplayer - server is authoritative)
            // Run simulation if: (1) multiplayer disabled, OR (2) multiplayer enabled but disconnected
            let should_simulate = {
                #[cfg(feature = "multiplayer")]
                {
                    // Only simulate if not connected
                    self.multiplayer_manager
                        .as_ref()
                        .map(|m| !m.state.is_connected())
                        .unwrap_or(true)
                }
                #[cfg(not(feature = "multiplayer"))]
                {
                    true
                }
            };

            if should_simulate {
                #[cfg(feature = "profiling")]
                puffin::profile_scope!("simulation");
                self.ui_state.stats.begin_sim();

                // Check if connected to multiplayer (redundant check, but kept for clarity)
                #[cfg(feature = "multiplayer")]
                let is_multiplayer_connected = self
                    .multiplayer_manager
                    .as_ref()
                    .map(|m| m.state.is_connected())
                    .unwrap_or(false);
                #[cfg(not(feature = "multiplayer"))]
                let is_multiplayer_connected = false;

                self.world.update(
                    1.0 / 60.0,
                    &mut self.ui_state.stats,
                    &mut rand::thread_rng(),
                    is_multiplayer_connected,
                );
                self.ui_state.stats.end_sim();
            }
        } // End of game loop - skip when loading chunks

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

        // Collect multiplayer data before egui closure to avoid borrow checker issues
        #[cfg(feature = "multiplayer")]
        let multiplayer_overlay_data = {
            let remote_players = self.collect_remote_players();
            let local_player_name = if let Some(ref manager) = self.multiplayer_manager {
                if let Some(ref conn_arc) = manager.client.get_connection() {
                    if let Ok(conn_guard) = conn_arc.lock() {
                        let identity = conn_guard.identity();
                        conn_guard
                            .db
                            .player()
                            .identity()
                            .find(&identity)
                            .and_then(|p| p.name.clone())
                    } else {
                        None
                    }
                } else {
                    None
                }
            } else {
                None
            };
            let window_size = self.window.inner_size();
            let camera_pos = self.renderer.camera_position();
            let camera_zoom = self.renderer.camera_zoom();
            let player_pos = self.world.player.position;
            (
                remote_players,
                local_player_name,
                window_size,
                camera_pos,
                camera_zoom,
                player_pos,
            )
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
                self.world.is_player_dead(),
                #[cfg(feature = "multiplayer")]
                self.multiplayer_manager.as_ref(),
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
                self.world.is_player_dead(),
                #[cfg(feature = "multiplayer")]
                self.multiplayer_manager.as_ref(),
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

            // Draw player nicknames overlay (multiplayer only)
            #[cfg(feature = "multiplayer")]
            {
                let (
                    remote_players,
                    local_player_name,
                    window_size,
                    camera_pos,
                    camera_zoom,
                    player_pos,
                ) = &multiplayer_overlay_data;
                draw_player_nicknames_overlay(
                    ctx,
                    *window_size,
                    *camera_pos,
                    *camera_zoom,
                    *player_pos,
                    local_player_name.as_deref(),
                    remote_players,
                );
            }

            // Render fullscreen loading overlay when waiting for multiplayer chunks
            #[cfg(feature = "multiplayer")]
            if let Some(manager) = self.multiplayer_manager.as_ref() {
                if manager.state.is_connected() && !self.multiplayer_initial_chunks_loaded {
                    use glam::IVec2;

                    let spawn_chunk_positions = vec![
                        IVec2::new(0, 0),
                        IVec2::new(-1, 0),
                        IVec2::new(1, 0),
                        IVec2::new(0, -1),
                        IVec2::new(0, 1),
                    ];
                    let chunks_loaded = spawn_chunk_positions
                        .iter()
                        .filter(|pos| self.world.has_chunk(**pos))
                        .count();

                    let elapsed = self
                        .chunk_loading_started_at
                        .map(|start| web_time::Instant::now().duration_since(start))
                        .unwrap_or(std::time::Duration::ZERO);

                    let timed_out = matches!(
                        manager.state,
                        crate::multiplayer::MultiplayerState::Error { .. }
                    );

                    let action = crate::ui::ui_state::UiState::render_loading_overlay(
                        ctx,
                        chunks_loaded,
                        spawn_chunk_positions.len(),
                        elapsed,
                        timed_out,
                    );

                    // Store action in panel state for processing after egui::run()
                    match action {
                        crate::ui::ui_state::LoadingAction::ReturnToLocal => {
                            self.ui_state.multiplayer_panel.disconnect_requested = true;
                        }
                        crate::ui::ui_state::LoadingAction::Retry => {
                            // Retry by reconnecting to the same server
                            if let Some(url) = manager.state.server_url() {
                                self.ui_state.multiplayer_panel.connect_requested =
                                    Some(url.to_string());
                            }
                        }
                        crate::ui::ui_state::LoadingAction::None => {}
                    }
                }
            }
        });
        let egui_build_time = egui_build_start.elapsed().as_secs_f32() * 1000.0;

        // Handle multiplayer panel actions (connect/disconnect/cancel)
        #[cfg(feature = "multiplayer")]
        {
            if let Some(_manager) = self.multiplayer_manager.as_ref() {
                // Extract action flags from panel state (avoids borrow conflicts)
                let connect_url = self.ui_state.multiplayer_panel.connect_requested.take();
                let disconnect_req = self.ui_state.multiplayer_panel.disconnect_requested;
                let cancel_req = self.ui_state.multiplayer_panel.cancel_requested;

                // Handle connect request
                if let Some(url) = connect_url {
                    log::info!("Connecting to: {}", url);
                    // Block on async connection (brief freeze during handshake is acceptable)
                    if let Err(e) = pollster::block_on(self.connect_to_server(url.clone())) {
                        log::error!("Failed to connect: {}", e);
                        if let Some(manager) = self.multiplayer_manager.as_mut() {
                            manager.mark_error(e.to_string(), url);
                        }
                        self.ui_state
                            .show_toast_error(&format!("Connection failed: {}", e));
                    } else {
                        log::info!("Successfully connected");
                        self.ui_state.show_toast("Connected to server!");

                        // Set pre-entered nickname if provided
                        if !self
                            .ui_state
                            .multiplayer_panel
                            .nickname_input
                            .trim()
                            .is_empty()
                        {
                            let nickname = self
                                .ui_state
                                .multiplayer_panel
                                .nickname_input
                                .trim()
                                .to_string();
                            if let Some(ref manager) = self.multiplayer_manager {
                                if manager.state.is_connected() {
                                    if let Err(e) = manager.client.set_nickname(nickname.clone()) {
                                        log::warn!("Failed to set pre-entered nickname: {}", e);
                                    } else {
                                        log::info!("Set pre-entered nickname: {}", nickname);
                                    }
                                }
                            }
                        }
                    }
                }

                // Handle disconnect request
                if disconnect_req {
                    log::info!("Disconnecting from server");
                    if let Err(e) = pollster::block_on(self.disconnect_from_server()) {
                        log::error!("Error during disconnect: {}", e);
                        self.ui_state
                            .show_toast_warning(&format!("Disconnect error: {}", e));
                    } else {
                        log::info!("Successfully disconnected");
                        self.ui_state
                            .show_toast("Disconnected - returned to singleplayer");
                    }
                }

                // Handle cancel request
                if cancel_req {
                    log::info!("Cancelling connection - returning to singleplayer");
                    if let Err(e) = pollster::block_on(self.disconnect_from_server()) {
                        log::error!("Error during cancel: {}", e);
                    } else {
                        self.ui_state.show_toast("Connection cancelled");
                    }
                }

                // Handle OAuth login (both native and WASM)
                #[cfg(feature = "multiplayer")]
                if self.ui_state.multiplayer_panel.oauth_login_requested {
                    #[cfg_attr(not(target_arch = "wasm32"), allow(unused_variables))]
                    if let Some(ref mp_manager) = self.multiplayer_manager {
                        // WASM: async OAuth via JavaScript
                        #[cfg(target_arch = "wasm32")]
                        {
                            let client = mp_manager.client.clone();
                            wasm_bindgen_futures::spawn_local(async move {
                                if let Err(e) = client.oauth_login().await {
                                    log::error!("OAuth login failed: {}", e);
                                }
                            });
                        }

                        // Native: blocking OAuth in background thread
                        #[cfg(not(target_arch = "wasm32"))]
                        {
                            let client = mp_manager.client.clone();
                            std::thread::spawn(move || match client.oauth_login() {
                                Ok(token) => {
                                    if let Err(e) = client.save_oauth_token(&token) {
                                        log::error!("Failed to save OAuth token: {}", e);
                                    } else {
                                        log::info!("OAuth login successful");
                                    }
                                }
                                Err(e) => log::error!("OAuth login failed: {}", e),
                            });
                            self.ui_state
                                .show_toast("Opening browser for Google login...");
                        }
                    }
                }

                // Handle OAuth logout (both native and WASM)
                #[cfg(feature = "multiplayer")]
                if self.ui_state.multiplayer_panel.oauth_logout_requested {
                    if let Some(ref mp_manager) = self.multiplayer_manager {
                        mp_manager.client.oauth_logout();
                        self.ui_state.multiplayer_panel.oauth_claims = None;
                        self.ui_state.show_toast("Logged out");
                    }
                }

                // Handle nickname change request
                #[cfg(feature = "multiplayer")]
                if let Some(nickname) = self
                    .ui_state
                    .multiplayer_panel
                    .set_nickname_requested
                    .take()
                {
                    if let Some(ref manager) = self.multiplayer_manager {
                        if manager.state.is_connected() {
                            if let Err(e) = manager.client.set_nickname(nickname.clone()) {
                                log::error!("Failed to set nickname: {}", e);
                                self.ui_state
                                    .show_toast_error(&format!("Failed to set nickname: {}", e));
                            } else {
                                log::info!("Nickname set to: {}", nickname);
                                self.ui_state
                                    .show_toast(&format!("Nickname changed to: {}", nickname));
                            }
                        } else {
                            self.ui_state
                                .show_toast_error("Cannot set nickname: not connected to server");
                        }
                    }
                }

                // Update cached OAuth claims and claim admin status (both native and WASM)
                #[cfg(feature = "multiplayer")]
                if let Some(ref mp_manager) = self.multiplayer_manager {
                    if let Some(platform_claims) = mp_manager.client.get_oauth_claims() {
                        // Convert platform-specific claims to shared type
                        let claims = crate::multiplayer::OAuthClaims::from(platform_claims);

                        // Check if this is a new login (claims changed)
                        let is_new_login = self
                            .ui_state
                            .multiplayer_panel
                            .oauth_claims
                            .as_ref()
                            .map(|old| old.email != claims.email)
                            .unwrap_or(true);

                        self.ui_state.multiplayer_panel.oauth_claims = Some(claims.clone());

                        // If new login with email, claim admin status on server
                        if is_new_login && mp_manager.state.is_connected() {
                            #[cfg_attr(not(target_arch = "wasm32"), allow(unused_variables))]
                            if let Some(ref email) = claims.email {
                                // WASM: async claim_admin call
                                #[cfg(target_arch = "wasm32")]
                                {
                                    let client = mp_manager.client.clone();
                                    let email = email.clone();
                                    wasm_bindgen_futures::spawn_local(async move {
                                        if let Err(e) = client.claim_admin(email).await {
                                            log::error!("Failed to claim admin: {}", e);
                                        } else {
                                            log::info!("Admin claim request sent to server");
                                        }
                                    });
                                }

                                // Native: async claim_admin call via blocking
                                #[cfg(not(target_arch = "wasm32"))]
                                {
                                    let client = mp_manager.client.clone();
                                    let email = email.clone();
                                    std::thread::spawn(move || {
                                        if let Err(e) =
                                            pollster::block_on(client.claim_admin(email))
                                        {
                                            log::error!("Failed to claim admin: {}", e);
                                        } else {
                                            log::info!("Admin claim request sent to server");
                                        }
                                    });
                                }
                            }
                        }
                    }
                }

                // Handle admin rebuild world (both native and WASM)
                #[cfg(feature = "multiplayer")]
                if self.ui_state.multiplayer_panel.rebuild_world_requested {
                    if let Some(ref mp_manager) = self.multiplayer_manager {
                        if mp_manager.state.is_connected() {
                            // WASM: async rebuild_world call
                            #[cfg(target_arch = "wasm32")]
                            {
                                let client = mp_manager.client.clone();
                                wasm_bindgen_futures::spawn_local(async move {
                                    if let Err(e) = client.rebuild_world().await {
                                        log::error!("Failed to rebuild world: {}", e);
                                    } else {
                                        log::info!("World rebuild requested");
                                    }
                                });
                            }

                            // Native: async rebuild_world call via blocking
                            #[cfg(not(target_arch = "wasm32"))]
                            {
                                let client = mp_manager.client.clone();
                                std::thread::spawn(move || {
                                    if let Err(e) = pollster::block_on(client.rebuild_world()) {
                                        log::error!("Failed to rebuild world: {}", e);
                                    } else {
                                        log::info!("World rebuild requested");
                                    }
                                });
                            }

                            self.ui_state.show_toast("Rebuilding world...");
                        } else {
                            self.ui_state.show_toast("Not connected to server");
                        }
                    }
                }

                // Reset action flags after processing
                self.ui_state.multiplayer_panel.reset_flags();
            }

            // Handle game over panel actions
            if self.ui_state.game_over_panel.respawn_requested {
                log::info!("Player requested respawn");

                #[cfg(feature = "multiplayer")]
                {
                    if let Some(manager) = self.multiplayer_manager.as_ref() {
                        if manager.state.is_connected() {
                            // Multiplayer: request respawn from server
                            if let Err(e) = manager.client.request_respawn() {
                                log::error!("Failed to request respawn: {}", e);
                            }
                        } else {
                            // Not connected: respawn locally
                            self.world.respawn_player();
                        }
                    } else {
                        // Singleplayer: respawn locally
                        self.world.respawn_player();
                    }
                }

                #[cfg(not(feature = "multiplayer"))]
                {
                    // Singleplayer only: respawn locally
                    self.world.respawn_player();
                }

                self.ui_state.game_over_panel.reset_flags();
            }
        }

        // Future: Level selector in dock is currently read-only
        // Interactive level switching could be added via dock callback system
        // (would require architecture for dock -> app communication)

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

        // Extract remote players for renderer (already collected before egui closure)
        #[cfg(feature = "multiplayer")]
        let remote_players = &multiplayer_overlay_data.0;
        #[cfg(not(feature = "multiplayer"))]
        let remote_players: &[RemotePlayerRenderData] = &[];

        // Render world + UI
        match self.renderer.render(
            &mut self.world,
            &self.particle_system,
            remote_players,
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

    /// Collect remote player data from SpacetimeDB subscription (multiplayer only)
    #[cfg(feature = "multiplayer")]
    fn collect_remote_players(&self) -> Vec<RemotePlayerRenderData> {
        let Some(ref manager) = self.multiplayer_manager else {
            return Vec::new();
        };

        let Some(ref conn_arc) = manager.client.get_connection() else {
            return Vec::new();
        };

        let conn = match conn_arc.lock() {
            Ok(guard) => guard,
            Err(_) => return Vec::new(),
        };

        // Get local identity to filter out
        let local_identity = conn.identity();

        conn.db
            .player()
            .iter()
            .filter(|p| {
                // Filter out local player
                p.identity != local_identity
            })
            .map(|p| RemotePlayerRenderData {
                x: p.x,
                y: p.y,
                vel_x: p.vel_x,
                vel_y: p.vel_y,
                name: p.name.clone(),
                identity: p.identity.to_string(),
            })
            .collect()
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

/// Draw player nicknames overlay using egui painter
#[cfg(feature = "multiplayer")]
fn draw_player_nicknames_overlay(
    ctx: &egui::Context,
    window_size: winit::dpi::PhysicalSize<u32>,
    camera_pos: glam::Vec2,
    camera_zoom: f32,
    local_player_pos: glam::Vec2,
    local_player_name: Option<&str>,
    remote_players: &[RemotePlayerRenderData],
) {
    let painter = ctx.layer_painter(egui::LayerId::background());

    // Font for nicknames
    let font = egui::FontId::proportional(14.0);

    // Local player nickname (yellow, 20px above sprite)
    let local_name = local_player_name.unwrap_or("Player");
    let (screen_x, screen_y) = world_to_screen(
        local_player_pos.x,
        local_player_pos.y - 20.0, // 20 pixels above player sprite
        window_size.width,
        window_size.height,
        camera_pos,
        camera_zoom,
    );
    painter.text(
        egui::pos2(screen_x, screen_y),
        egui::Align2::CENTER_CENTER,
        local_name,
        font.clone(),
        egui::Color32::YELLOW,
    );

    // Remote player nicknames (white)
    for player in remote_players {
        let display_name = if let Some(ref name) = player.name {
            name.as_str()
        } else {
            // Fallback to shortened identity (last 6 chars)
            if player.identity.len() >= 6 {
                &player.identity[player.identity.len() - 6..]
            } else {
                &player.identity
            }
        };

        let (screen_x, screen_y) = world_to_screen(
            player.x,
            player.y - 20.0, // 20 pixels above sprite
            window_size.width,
            window_size.height,
            camera_pos,
            camera_zoom,
        );
        painter.text(
            egui::pos2(screen_x, screen_y),
            egui::Align2::CENTER_CENTER,
            display_name,
            font.clone(),
            egui::Color32::WHITE,
        );
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
                        #[cfg(feature = "multiplayer")]
                        KeyCode::KeyM => {
                            if pressed {
                                self.ui_state
                                    .toggle_tab(crate::ui::DockTab::MultiplayerStats);
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
                                // Check if connected to multiplayer - don't spawn creatures locally
                                #[cfg(feature = "multiplayer")]
                                let is_multiplayer = self
                                    .multiplayer_manager
                                    .as_ref()
                                    .map(|m| m.state.is_connected())
                                    .unwrap_or(false);
                                #[cfg(not(feature = "multiplayer"))]
                                let is_multiplayer = false;

                                if is_multiplayer {
                                    log::warn!("Cannot spawn creatures in multiplayer mode");
                                    self.ui_state
                                        .toasts
                                        .info("Cannot spawn creatures while connected to server");
                                } else {
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
