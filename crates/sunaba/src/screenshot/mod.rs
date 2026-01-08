//! Screenshot capture for visual iteration
//!
//! Allows capturing game state in specific scenarios for visual verification.
//!
//! ## Architecture
//!
//! - `scenario` - Scenario types and parsing (Level, UiPanel, Interactive)
//! - `offscreen_renderer` - GPU offscreen rendering for UI screenshots (TODO)
//! - `sample_data` - Test data generators for UI panels (TODO)
//!
//! ## Usage
//!
//! ```bash
//! # Capture level screenshot
//! just screenshot level:3
//! just screenshot 3  # Backward compatible
//!
//! # Capture UI panel screenshot (coming soon)
//! just screenshot ui:params
//!
//! # List available scenarios
//! just list-scenarios
//! ```

mod layouts;
mod offscreen_renderer;
mod sample_data;
pub mod scenario;
pub mod video_capture;
pub mod video_scenarios;

pub use layouts::ScreenshotLayout;

use anyhow::Result;
use glam::Vec2;
use std::path::Path;

use crate::headless::PixelRenderer;
use crate::levels::LevelManager;
use crate::simulation::Materials;
use crate::world::{NoopStats, World};
use rand::thread_rng;

use offscreen_renderer::OffscreenRenderer;

// Imports for UI screenshot capture
use crate::entity::{
    crafting::RecipeRegistry, inventory::ItemStack, player::Player, tools::ToolRegistry,
};

pub use scenario::{ScreenshotScenario, list_all_scenarios};
pub use video_capture::VideoCapture;
pub use video_scenarios::{
    ScenarioAction, VideoScenario, get_all_scenarios as get_all_video_scenarios,
    get_scenario_by_id as get_video_scenario_by_id,
};

/// Screenshot configuration
pub struct ScreenshotConfig {
    /// Width of the screenshot in pixels
    pub width: usize,
    /// Height of the screenshot in pixels
    pub height: usize,
    /// Number of frames to simulate before capturing (let physics settle)
    pub settle_frames: usize,
    /// Camera center position (None = auto-center on level)
    pub camera_center: Option<Vec2>,
}

/// UI panels that can be shown in screenshots
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UiPanel {
    /// Parameters/settings panel
    Params,
    /// Multiplayer status panel
    #[cfg(feature = "multiplayer")]
    Multiplayer,
    /// Inventory panel
    Inventory,
    /// Crafting panel
    Crafting,
    /// Logger panel
    Logger,
    /// World generation editor
    WorldGen,
    /// Level selector
    LevelSelector,
}

impl Default for ScreenshotConfig {
    fn default() -> Self {
        Self {
            width: 1920,
            height: 1080,
            settle_frames: 60, // 1 second at 60fps
            camera_center: None,
        }
    }
}

/// Capture a screenshot of a specific level
pub fn capture_level_screenshot(
    level_id: usize,
    output_path: impl AsRef<Path>,
    config: ScreenshotConfig,
) -> Result<()> {
    log::info!("Capturing screenshot of level {}", level_id);
    log::info!("  Resolution: {}x{}", config.width, config.height);
    log::info!("  Settle frames: {}", config.settle_frames);

    // Initialize world (skip initial creatures for clean screenshot)
    let mut world = World::new(true);
    let materials = Materials::new();

    // Load the specified level
    let mut level_manager = LevelManager::new();
    if level_id >= level_manager.levels().len() {
        anyhow::bail!(
            "Invalid level ID: {} (max: {})",
            level_id,
            level_manager.levels().len() - 1
        );
    }
    level_manager.load_level(level_id, &mut world);

    let level_name = level_manager.levels()[level_id].name;
    log::info!("  Level: {}", level_name);

    // Determine camera center (default to world origin)
    let camera_center = config.camera_center.unwrap_or(Vec2::new(0.0, 32.0));

    // Let physics settle
    log::info!(
        "Simulating {} frames to settle physics...",
        config.settle_frames
    );
    let mut stats = NoopStats;
    let mut rng = thread_rng();
    for _ in 0..config.settle_frames {
        world.update(1.0 / 60.0, &mut stats, &mut rng, false);
    }

    // Render to pixel buffer
    log::info!("Rendering scene...");
    let mut renderer = PixelRenderer::new(config.width, config.height);
    renderer.render(&world, &materials, camera_center, &[]);

    // Save to PNG
    log::info!("Saving to {:?}...", output_path.as_ref());
    save_buffer_as_png(&renderer.buffer, config.width, config.height, output_path)?;

    log::info!("Screenshot saved successfully!");
    Ok(())
}

// TODO: Add world screenshot support once world loading API is finalized
// /// Capture a screenshot of a saved world
// pub fn capture_world_screenshot(
//     world_path: impl AsRef<Path>,
//     output_path: impl AsRef<Path>,
//     config: ScreenshotConfig,
// ) -> Result<()> {
//     // Load world from disk
//     let mut world = World::new(false); // Load from persistence
//     world.load_persistent_world();
//     let materials = Materials::new();
//
//     // ... rest of implementation
//     Ok(())
// }

/// Parse UI panel from string
impl UiPanel {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "params" | "parameters" | "settings" => Some(UiPanel::Params),
            #[cfg(feature = "multiplayer")]
            "multiplayer" | "mp" | "multi" => Some(UiPanel::Multiplayer),
            "inventory" | "inv" => Some(UiPanel::Inventory),
            "crafting" | "craft" => Some(UiPanel::Crafting),
            "logger" | "log" | "logs" => Some(UiPanel::Logger),
            "worldgen" | "world-gen" | "wg" => Some(UiPanel::WorldGen),
            "levels" | "level-selector" => Some(UiPanel::LevelSelector),
            _ => None,
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            UiPanel::Params => "Parameters",
            #[cfg(feature = "multiplayer")]
            UiPanel::Multiplayer => "Multiplayer",
            UiPanel::Inventory => "Inventory",
            UiPanel::Crafting => "Crafting",
            UiPanel::Logger => "Logger",
            UiPanel::WorldGen => "World Generation",
            UiPanel::LevelSelector => "Level Selector",
        }
    }
}

/// List all available demo levels
pub fn list_levels() {
    let level_manager = LevelManager::new();
    println!("Available demo levels:");
    println!();
    for level in level_manager.levels() {
        println!("  [{}] {}", level.id, level.name);
        println!("      {}", level.description);
    }
    println!();
    println!("Total: {} levels", level_manager.levels().len());
}

/// List all available UI panels
pub fn list_ui_panels() {
    println!("Available UI panels:");
    println!();
    println!("  params          - Parameters/settings panel");
    #[cfg(feature = "multiplayer")]
    println!("  multiplayer     - Multiplayer status panel");
    println!("  inventory       - Inventory panel");
    println!("  crafting        - Crafting panel");
    println!("  logger          - Log viewer panel");
    println!("  worldgen        - World generation editor");
    println!("  levels          - Level selector");
    println!();
    println!("Note: UI screenshots require running the full game with GPU rendering");
    println!("Use: just screenshot-ui <level_id> <panel_name>");
}

/// Capture a composite screenshot (world + UI)
#[cfg(not(target_arch = "wasm32"))]
fn capture_composite_screenshot(
    level_id: usize,
    panels: Vec<UiPanel>,
    output_path: impl AsRef<Path>,
    width: usize,
    height: usize,
    settle_frames: usize,
) -> Result<()> {
    log::info!(
        "Capturing composite screenshot: level {} with {:?}",
        level_id,
        panels
    );
    log::info!("  Resolution: {}x{}", width, height);

    // 1. Render world background using CPU PixelRenderer
    let mut world = World::new(true);
    let materials = Materials::new();
    let mut level_manager = LevelManager::new();

    if level_id >= level_manager.levels().len() {
        anyhow::bail!(
            "Invalid level ID: {} (max: {})",
            level_id,
            level_manager.levels().len() - 1
        );
    }

    level_manager.load_level(level_id, &mut world);

    // Let physics settle
    log::info!("Simulating {} frames...", settle_frames);
    let mut stats = NoopStats;
    let mut rng = thread_rng();
    for _ in 0..settle_frames {
        world.update(1.0 / 60.0, &mut stats, &mut rng, false);
    }

    // Render world to pixel buffer
    let camera_center = Vec2::new(0.0, 32.0);
    let mut pixel_renderer = PixelRenderer::new(width, height);
    pixel_renderer.render(&world, &materials, camera_center, &[]);
    let world_pixels = &pixel_renderer.buffer;

    // 2. Create sample data for UI
    let player = sample_data::create_sample_player_with_inventory();
    let tool_registry = ToolRegistry::default();
    let recipe_registry = RecipeRegistry::new();

    // 3. Render UI on top of world using OffscreenRenderer
    let mut renderer = OffscreenRenderer::new(width as u32, height as u32)?;

    let pixels = renderer.render_ui_with_background(Some(world_pixels), |ctx| {
        // Apply theme
        ctx.style_mut(|style| {
            style.visuals.window_fill = egui::Color32::from_rgba_unmultiplied(45, 45, 48, 240); // Semi-transparent
        });

        // Render each panel in sequence (for now, just render all in right side panel)
        // TODO: Support multiple panels with proper layout
        if let Some(panel) = panels.first() {
            render_panel_fullscreen(
                *panel,
                ctx,
                &materials,
                &player,
                &tool_registry,
                &recipe_registry,
                &level_manager,
                &format!("Level {}", level_id),
            );
        }
    })?;

    // 4. Save as PNG
    log::info!("Saving to {:?}...", output_path.as_ref());
    save_buffer_as_png(&pixels, width, height, output_path)?;

    log::info!("Composite screenshot saved successfully!");
    Ok(())
}

/// Capture a screenshot of a UI panel
#[cfg(not(target_arch = "wasm32"))]
fn capture_ui_panel_screenshot(
    panel: UiPanel,
    output_path: impl AsRef<Path>,
    width: usize,
    height: usize,
    background_color: [u8; 4],
    with_sample_data: bool,
) -> Result<()> {
    log::info!("Capturing UI screenshot: {}", panel.name());
    log::info!("  Resolution: {}x{}", width, height);
    log::info!("  Sample data: {}", with_sample_data);

    // 1. Initialize offscreen GPU renderer
    let mut renderer = OffscreenRenderer::new(width as u32, height as u32)?;

    // 2. Create sample data
    let materials = Materials::new();
    let player = if with_sample_data {
        sample_data::create_sample_player_with_inventory()
    } else {
        Player::new(Vec2::ZERO)
    };
    let tool_registry = ToolRegistry::default();
    let recipe_registry = RecipeRegistry::new();
    let level_manager = LevelManager::new();

    // 3. Render UI to offscreen texture (fullscreen, bypassing dock system)
    let pixels = renderer.render_ui(|ctx| {
        // Set background color
        let bg_color = egui::Color32::from_rgba_unmultiplied(
            background_color[0],
            background_color[1],
            background_color[2],
            background_color[3],
        );
        ctx.style_mut(|style| {
            style.visuals.panel_fill = bg_color;
            style.visuals.window_fill = bg_color;
        });

        // Render panel fullscreen without dock wrapper
        render_panel_fullscreen(
            panel,
            ctx,
            &materials,
            &player,
            &tool_registry,
            &recipe_registry,
            &level_manager,
            "Screenshot Mode",
        );
    })?;

    // 4. Save as PNG
    log::info!("Saving to {:?}...", output_path.as_ref());
    save_buffer_as_png(&pixels, width, height, output_path)?;

    log::info!("Screenshot saved successfully!");
    Ok(())
}

/// Render a single panel in a dock-like side panel
///
/// Renders panels in a right-side panel (400px wide) to match the actual in-game dock appearance.
fn render_panel_fullscreen(
    panel: UiPanel,
    ctx: &egui::Context,
    materials: &Materials,
    player: &Player,
    tool_registry: &ToolRegistry,
    recipe_registry: &RecipeRegistry,
    level_manager: &LevelManager,
    game_mode_desc: &str,
) {
    // Render panel in a right-side panel (matching actual dock width)
    egui::SidePanel::right("screenshot_dock")
        .default_width(400.0)
        .resizable(false)
        .show(ctx, |ui| match panel {
            UiPanel::Params => render_params_fullscreen(ui),
            UiPanel::Inventory => render_inventory_fullscreen(ui, player, materials, tool_registry),
            UiPanel::Crafting => render_crafting_fullscreen(ui, recipe_registry, materials),
            UiPanel::Logger => render_logger_fullscreen(ui),
            UiPanel::LevelSelector => {
                render_level_selector_fullscreen(ui, level_manager, game_mode_desc)
            }
            #[cfg(feature = "multiplayer")]
            UiPanel::Multiplayer => render_multiplayer_fullscreen(ui),
            UiPanel::WorldGen => {
                ui.heading("WorldGen Editor");
                ui.label("WorldGen screenshot not yet implemented");
                ui.label("(WorldGen is a separate window, not a dock panel)");
            }
        });

    // Fill remaining space with background (left side of screen)
    egui::CentralPanel::default().show(ctx, |_ui| {
        // Empty background panel
    });
}

fn render_params_fullscreen(ui: &mut egui::Ui) {
    ui.heading("Parameters");
    ui.separator();

    ui.collapsing("Graphics", |ui| {
        ui.checkbox(&mut true.clone(), "VSync");
        ui.checkbox(&mut false.clone(), "Fullscreen");
        ui.horizontal(|ui| {
            ui.label("Resolution:");
            ui.label("1920x1080");
        });
    });

    ui.collapsing("Physics", |ui| {
        ui.horizontal(|ui| {
            ui.label("Update Rate:");
            ui.label("60 Hz");
        });
        ui.horizontal(|ui| {
            ui.label("Chunk Update Radius:");
            ui.label("5");
        });
    });

    ui.collapsing("World", |ui| {
        ui.horizontal(|ui| {
            ui.label("Seed:");
            ui.label("12345678");
        });
        ui.checkbox(&mut true.clone(), "Auto-save");
    });

    ui.collapsing("Debug", |ui| {
        ui.checkbox(&mut false.clone(), "Show FPS");
        ui.checkbox(&mut false.clone(), "Show Active Chunks");
        ui.checkbox(&mut false.clone(), "Show Player Stats");
    });
}

fn render_inventory_fullscreen(
    ui: &mut egui::Ui,
    player: &Player,
    materials: &Materials,
    tool_registry: &ToolRegistry,
) {
    // Copy from dock.rs::render_inventory() (lines 252-292)
    ui.heading("Inventory");

    let inventory = &player.inventory;
    ui.label(format!(
        "Using {}/{} slots",
        inventory.used_slot_count(),
        inventory.max_slots
    ));

    ui.separator();

    // Show all slots (not just first 10 like dock panel)
    for i in 0..inventory.max_slots {
        if let Some(Some(stack)) = inventory.get_slot(i) {
            match stack {
                ItemStack::Material { material_id, count } => {
                    let mat = materials.get(*material_id);
                    ui.label(format!("[{}] {} x{}", i, mat.name, count));
                }
                ItemStack::Tool {
                    tool_id,
                    durability,
                } => {
                    if let Some(tool_def) = tool_registry.get(*tool_id) {
                        ui.label(format!("[{}] {} ({})", i, tool_def.name, durability));
                    }
                }
            }
        }
    }

    // Show equipped tool
    if let Some(tool_id) = player.equipped_tool {
        ui.separator();
        if let Some(tool_def) = tool_registry.get(tool_id) {
            ui.label(format!("Tool: {}", tool_def.name));
        }
    }
}

fn render_crafting_fullscreen(
    ui: &mut egui::Ui,
    recipe_registry: &RecipeRegistry,
    materials: &Materials,
) {
    // Copy from dock.rs::render_crafting() (lines 294-309)
    ui.heading("Crafting");
    ui.label("Available recipes:");

    for recipe in recipe_registry.all_recipes() {
        ui.horizontal(|ui| {
            ui.label(&recipe.name);
            ui.label("-");
            if let Some((mat_id, count)) = recipe.inputs.first() {
                let mat = materials.get(*mat_id);
                ui.label(format!("{} x{}", mat.name, count));
            }
        });
    }
}

fn render_logger_fullscreen(ui: &mut egui::Ui) {
    // Copy from dock.rs::render_logger() (lines 312-319)
    #[cfg(not(target_arch = "wasm32"))]
    egui_logger::logger_ui().show(ui);

    #[cfg(target_arch = "wasm32")]
    ui.label("Logger panel (native only)");
}

fn render_level_selector_fullscreen(
    ui: &mut egui::Ui,
    level_manager: &LevelManager,
    game_mode_desc: &str,
) {
    ui.heading("Levels");
    ui.label(format!("Current: {}", game_mode_desc));
    ui.separator();

    egui::ScrollArea::vertical().show(ui, |ui| {
        for level in level_manager.levels() {
            ui.horizontal(|ui| {
                if ui.button(format!("[{}]", level.id)).clicked() {
                    // No-op in screenshot mode
                }
                ui.label(level.name);
            });
            ui.label(format!("  {}", level.description));
            ui.add_space(4.0);
        }
    });
}

#[cfg(feature = "multiplayer")]
fn render_multiplayer_fullscreen(ui: &mut egui::Ui) {
    ui.heading("Multiplayer");
    ui.separator();

    ui.horizontal(|ui| {
        ui.label("Status:");
        ui.colored_label(egui::Color32::GREEN, "Connected");
    });

    ui.horizontal(|ui| {
        ui.label("Server:");
        ui.label("localhost:3000");
    });

    ui.separator();
    ui.label("Players (4):");

    egui::ScrollArea::vertical().show(ui, |ui| {
        for i in 0..4 {
            ui.horizontal(|ui| {
                let name = match i {
                    0 => "Alice",
                    1 => "Bob",
                    2 => "Charlie",
                    3 => "Dave",
                    _ => "Player",
                };
                ui.label(format!("👤 {}", name));
                ui.label(format!("({}, {})", i * 100, i * 50));
            });
        }
    });
}

/// Capture a screenshot of a UI panel (WASM stub - not supported)
#[cfg(target_arch = "wasm32")]
fn capture_ui_panel_screenshot(
    _panel: UiPanel,
    _output_path: impl AsRef<Path>,
    _width: usize,
    _height: usize,
    _background_color: [u8; 4],
    _with_sample_data: bool,
) -> Result<()> {
    anyhow::bail!("UI screenshot capture is not supported on WASM")
}

/// Capture a screenshot based on a scenario
pub fn capture_scenario(
    scenario: ScreenshotScenario,
    output_path: impl AsRef<Path>,
    width: usize,
    height: usize,
) -> Result<()> {
    match scenario {
        ScreenshotScenario::Level { id, settle_frames } => {
            let config = ScreenshotConfig {
                width,
                height,
                settle_frames,
                camera_center: None,
            };
            capture_level_screenshot(id, output_path, config)
        }
        ScreenshotScenario::UiPanel {
            panel,
            background_color,
            with_sample_data,
        } => capture_ui_panel_screenshot(
            panel,
            output_path,
            width,
            height,
            background_color,
            with_sample_data,
        ),
        ScreenshotScenario::Composite {
            level_id,
            panels,
            settle_frames,
        } => capture_composite_screenshot(
            level_id,
            panels,
            output_path,
            width,
            height,
            settle_frames,
        ),
        ScreenshotScenario::Layout {
            layout_name,
            level_id,
            settle_frames,
        } => {
            // Get layout template
            let layout = ScreenshotLayout::by_name(&layout_name)
                .ok_or_else(|| anyhow::anyhow!("Unknown layout: {}", layout_name))?;

            // For now, layout screenshots just render the active panel from the layout
            // Future: Render full dock with multiple tabs + overlays
            if let Some(panel) = layout.active_panel {
                if let Some(level) = level_id {
                    // Layout with world background
                    capture_composite_screenshot(
                        level,
                        vec![panel],
                        output_path,
                        width,
                        height,
                        settle_frames,
                    )
                } else {
                    // Layout without world (just UI panel)
                    capture_ui_panel_screenshot(
                        panel,
                        output_path,
                        width,
                        height,
                        [45, 45, 48, 255],
                        true,
                    )
                }
            } else {
                // Minimal layout (no panels) - just world or empty
                if let Some(level) = level_id {
                    let config = ScreenshotConfig {
                        width,
                        height,
                        settle_frames,
                        camera_center: None,
                    };
                    capture_level_screenshot(level, output_path, config)
                } else {
                    anyhow::bail!("Minimal layout requires a level ID")
                }
            }
        }
        ScreenshotScenario::Interactive { ref name, .. } => {
            anyhow::bail!("Interactive scenarios not yet implemented: {}", name)
        }
    }
}

/// Capture a video scenario and encode to MP4
///
/// # Arguments
/// * `scenario` - The video scenario to capture
/// * `output_path` - Output MP4 file path (relative to current directory)
///
/// # Returns
/// Ok(()) on success, or an error if capturing or encoding fails
pub fn capture_video_scenario(
    scenario: &VideoScenario,
    output_path: impl AsRef<Path>,
) -> Result<()> {
    log::info!("Capturing video scenario: {}", scenario.name);
    log::info!("  Description: {}", scenario.description);
    log::info!("  Resolution: {}x{}", scenario.width, scenario.height);
    log::info!(
        "  Duration: {}s @ {}fps",
        scenario.duration_seconds,
        scenario.fps
    );
    log::info!("  Actions: {}", scenario.actions.len());

    // Initialize video capture
    let mut video = VideoCapture::new(scenario.width, scenario.height, scenario.fps)?;

    // Initialize world (skip initial creatures for clean video)
    let mut world = World::new(true);
    let materials = Materials::new();

    // Load level if specified
    if let Some(level_id) = scenario.level_id {
        let mut level_manager = LevelManager::new();
        if level_id >= level_manager.levels().len() {
            anyhow::bail!(
                "Invalid level ID: {} (max: {})",
                level_id,
                level_manager.levels().len() - 1
            );
        }
        level_manager.load_level(level_id, &mut world);
        let level_name = level_manager.levels()[level_id].name;
        log::info!("  Level: {}", level_name);
    } else {
        log::info!("  Level: (empty world)");
    }

    // Determine camera center (default to world origin)
    let camera_center = Vec2::new(0.0, 32.0);

    // Create renderer
    let mut renderer = PixelRenderer::new(scenario.width as usize, scenario.height as usize);

    // Physics simulation parameters
    let dt = 1.0 / 60.0; // 60 FPS physics
    let capture_interval = 60 / scenario.fps as usize; // Capture every N frames (e.g., every 3 frames for 20fps)
    let total_frames = (scenario.duration_seconds * 60.0) as usize;

    let mut stats = NoopStats;
    let mut rng = thread_rng();

    log::info!(
        "Simulating {} frames ({} captures)...",
        total_frames,
        total_frames / capture_interval
    );

    // Build action timeline (frame number -> actions to execute)
    let action_timeline = build_action_timeline(&scenario.actions);

    // Simulation and capture loop
    for frame in 0..total_frames {
        // Execute scenario actions scheduled for this frame
        if let Some(actions) = action_timeline.get(&frame) {
            for action in actions {
                execute_video_action(action, &mut world, frame);
            }
        }

        // Update world physics
        world.update(dt, &mut stats, &mut rng, false);

        // Capture frame at intervals
        if frame % capture_interval == 0 {
            renderer.render(&world, &materials, camera_center, &[]);
            video.capture_frame(&renderer)?;
        }

        // Progress logging every second
        if frame % 60 == 0 {
            log::debug!(
                "  Frame {}/{} ({:.1}s)",
                frame,
                total_frames,
                frame as f32 / 60.0
            );
        }
    }

    log::info!("Encoding {} frames to MP4...", video.frame_count());
    video.encode_to_mp4(&output_path)?;

    log::info!("Video saved successfully: {:?}", output_path.as_ref());
    Ok(())
}

/// Build action timeline from scenario actions
///
/// Converts a sequence of actions (with Wait actions) into a frame-based timeline.
/// Returns a map of frame number -> actions to execute at that frame.
fn build_action_timeline(
    actions: &[ScenarioAction],
) -> std::collections::HashMap<usize, Vec<ScenarioAction>> {
    use std::collections::HashMap;

    let mut timeline: HashMap<usize, Vec<ScenarioAction>> = HashMap::new();
    let mut current_frame = 0;

    for action in actions {
        match action {
            ScenarioAction::Wait { frames } => {
                // Advance frame counter (don't schedule any action)
                current_frame += frames;
            }
            other => {
                // Schedule action at current frame
                timeline
                    .entry(current_frame)
                    .or_insert_with(Vec::new)
                    .push(other.clone());
            }
        }
    }

    if !timeline.is_empty() {
        log::debug!("Action timeline: {} trigger frames", timeline.len());
        for (frame, actions) in &timeline {
            log::debug!("  Frame {}: {} action(s)", frame, actions.len());
        }
    }

    timeline
}

/// Execute a scenario action
fn execute_video_action(action: &ScenarioAction, world: &mut World, frame: usize) {
    match action {
        ScenarioAction::Wait { .. } => {
            // Does nothing - just advance simulation
        }

        ScenarioAction::MineCircle { x, y, radius } => {
            world.debug_mine_circle(*x, *y, *radius);
        }

        ScenarioAction::PlaceMaterial {
            x,
            y,
            material,
            radius,
        } => {
            world.place_material_debug(*x, *y, *material, *radius as u32);
        }

        ScenarioAction::RemoveSupport {
            x,
            y,
            width,
            height,
        } => {
            // Remove all materials in the rectangular area (set to AIR)
            const AIR: u16 = 0;
            for dy in 0..*height {
                for dx in 0..*width {
                    let world_x = x + dx;
                    let world_y = y + dy;
                    world.set_pixel(world_x, world_y, AIR);
                }
            }
        }

        ScenarioAction::TeleportPlayer { .. } => {
            // TODO: Implement player teleport in headless mode
            // For now, skip this action since headless mode doesn't have a player entity
            log::warn!(
                "TeleportPlayer action not yet implemented in headless video capture (frame {})",
                frame
            );
        }

        ScenarioAction::SimulatePlayerMining { .. } => {
            // TODO: Implement player mining simulation in headless mode
            // For now, skip this action
            log::warn!(
                "SimulatePlayerMining action not yet implemented in headless video capture (frame {})",
                frame
            );
        }
    }
}

/// List all available video scenarios
pub fn list_video_scenarios() {
    let scenarios = get_all_video_scenarios();
    println!("Available video scenarios:");
    println!();

    for scenario in scenarios {
        println!(
            "  {} ({}) - {}s @ {}fps",
            scenario.id, scenario.name, scenario.duration_seconds, scenario.fps
        );
        println!("    {}", scenario.description);
        if let Some(level_id) = scenario.level_id {
            println!("    Level: {}", level_id);
        }
        println!();
    }

    println!("Total: {} scenarios", get_all_video_scenarios().len());
    println!();
    println!("Usage:");
    println!("  cargo run --release --features headless -- --video-scenario <id>");
    println!("  cargo run --release --features headless -- --generate-all-videos");
}

/// Save RGBA buffer as PNG
fn save_buffer_as_png(
    buffer: &[u8],
    width: usize,
    height: usize,
    path: impl AsRef<Path>,
) -> Result<()> {
    use image::{ImageBuffer, Rgba};

    let img: ImageBuffer<Rgba<u8>, _> =
        ImageBuffer::from_raw(width as u32, height as u32, buffer.to_vec())
            .ok_or_else(|| anyhow::anyhow!("Failed to create image buffer"))?;

    img.save(path)?;
    Ok(())
}
