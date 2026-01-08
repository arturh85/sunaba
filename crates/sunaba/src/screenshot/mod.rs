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

use anyhow::{Context, Result};
use glam::Vec2;
use std::path::Path;

use crate::headless::PixelRenderer;
use crate::levels::LevelManager;
use crate::simulation::Materials;
use crate::world::{NoopStats, World, CHUNK_SIZE};
use rand::thread_rng;

use offscreen_renderer::OffscreenRenderer;

// Imports for UI screenshot capture
use crate::entity::{
    crafting::RecipeRegistry, inventory::ItemStack, player::Player, tools::ToolRegistry,
};

pub use scenario::{ScreenshotScenario, list_all_scenarios};
pub use video_capture::VideoCapture;
pub use video_scenarios::{
    CameraParams, CameraSpec, MaterialFilter, ScenarioAction, VideoScenario,
    get_all_scenarios as get_all_video_scenarios, get_scenario_by_id as get_video_scenario_by_id,
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
    renderer.render(&world, &materials, camera_center, &[], 1.0); // 1x zoom (1:1 pixel mapping)

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
    pixel_renderer.render(&world, &materials, camera_center, &[], 1.0); // 1x zoom
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

/// Calculate camera parameters to frame a rectangular region within viewport
///
/// # Arguments
/// * `bounds` - World space bounding rectangle (min_x, min_y, max_x, max_y)
/// * `padding` - Extra world pixels to include around bounds
/// * `viewport_width` - Target viewport width in pixels
/// * `viewport_height` - Target viewport height in pixels
///
/// # Returns
/// CameraParams with center and zoom to frame the bounds
fn calculate_camera_from_bounds(
    bounds: (i32, i32, i32, i32),
    padding: i32,
    viewport_width: u32,
    viewport_height: u32,
) -> Result<CameraParams> {
    let (min_x, min_y, max_x, max_y) = bounds;

    // Edge case: empty bounds
    if max_x < min_x || max_y < min_y {
        anyhow::bail!(
            "Invalid bounds: max < min (got bounds ({}, {}, {}, {}))",
            min_x,
            min_y,
            max_x,
            max_y
        );
    }

    // Calculate content dimensions (inclusive of endpoints)
    let content_width = ((max_x - min_x + 1) as f32).max(10.0);
    let content_height = ((max_y - min_y + 1) as f32).max(10.0);

    // Apply padding (convert to world space)
    let padded_width = content_width + (2 * padding) as f32;
    let padded_height = content_height + (2 * padding) as f32;

    // Calculate center of bounds (world space)
    let center_x = (min_x + max_x) as f32 / 2.0;
    let center_y = (min_y + max_y) as f32 / 2.0;

    // Calculate zoom to fit content in viewport
    let zoom_x = viewport_width as f32 / padded_width;
    let zoom_y = viewport_height as f32 / padded_height;

    // Use the smaller zoom to ensure content fits in BOTH dimensions
    let zoom = zoom_x.min(zoom_y).clamp(0.1, 20.0);

    Ok(CameraParams {
        center: Vec2::new(center_x, center_y),
        zoom,
    })
}

/// Scan world chunks to detect content bounds based on material filter
///
/// # Arguments
/// * `world` - World to scan
/// * `materials` - Material definitions for type checking
/// * `filter` - Material filter determining what counts as content
///
/// # Returns
/// Bounding rectangle (min_x, min_y, max_x, max_y) or None if no content found
fn detect_content_bounds(
    world: &World,
    materials: &Materials,
    filter: &MaterialFilter,
) -> Option<(i32, i32, i32, i32)> {
    let mut min_x = i32::MAX;
    let mut min_y = i32::MAX;
    let mut max_x = i32::MIN;
    let mut max_y = i32::MIN;
    let mut found_any = false;

    // Iterate over all loaded chunks
    for (chunk_coord, chunk) in world.chunks() {
        // Chunk world origin (chunks are 64x64)
        let chunk_world_x = chunk_coord.x * CHUNK_SIZE as i32;
        let chunk_world_y = chunk_coord.y * CHUNK_SIZE as i32;

        // Scan all pixels in chunk
        for local_y in 0..CHUNK_SIZE {
            for local_x in 0..CHUNK_SIZE {
                let pixel = chunk.get_pixel(local_x, local_y);

                // Skip if pixel doesn't match filter
                if !pixel_matches_filter(pixel.material_id, materials, filter) {
                    continue;
                }

                // Update bounds
                let world_x = chunk_world_x + local_x as i32;
                let world_y = chunk_world_y + local_y as i32;

                min_x = min_x.min(world_x);
                min_y = min_y.min(world_y);
                max_x = max_x.max(world_x);
                max_y = max_y.max(world_y);
                found_any = true;
            }
        }
    }

    if found_any {
        Some((min_x, min_y, max_x, max_y))
    } else {
        None
    }
}

/// Check if a material ID matches the filter criteria
fn pixel_matches_filter(material_id: u16, materials: &Materials, filter: &MaterialFilter) -> bool {
    match filter {
        MaterialFilter::NonAir => material_id != 0,

        MaterialFilter::Types(types) => {
            if material_id == 0 {
                return false;
            }
            let material = materials.get(material_id);
            types.contains(&material.material_type)
        }

        MaterialFilter::ExcludeTypes(types) => {
            if material_id == 0 {
                return false;
            }
            let material = materials.get(material_id);
            !types.contains(&material.material_type)
        }

        MaterialFilter::Ids(ids) => ids.contains(&material_id),

        MaterialFilter::ExcludeIds(ids) => material_id != 0 && !ids.contains(&material_id),
    }
}

/// Resolve a CameraSpec to CameraParams
fn resolve_camera_spec(
    spec: &CameraSpec,
    world: &World,
    materials: &Materials,
    viewport_width: u32,
    viewport_height: u32,
) -> Result<CameraParams> {
    match spec {
        CameraSpec::Bounds {
            min_x,
            min_y,
            max_x,
            max_y,
            padding,
        } => calculate_camera_from_bounds(
            (*min_x, *min_y, *max_x, *max_y),
            *padding,
            viewport_width,
            viewport_height,
        ),

        CameraSpec::AutoDetect { filter, padding } => {
            let bounds = detect_content_bounds(world, materials, filter)
                .context("No content found for camera auto-detection")?;

            calculate_camera_from_bounds(bounds, *padding, viewport_width, viewport_height)
        }

        CameraSpec::Manual { center, zoom } => Ok(CameraParams {
            center: *center,
            zoom: *zoom,
        }),
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

    // Initialize world without random generation (skip initial creatures for clean video)
    let mut world = World::new(false);
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

        // CRITICAL: Disable persistence to prevent procedural chunk generation
        // This ensures only the demo level chunks are present (no random terrain)
        world.disable_persistence();

        let level_name = level_manager.levels()[level_id].name;
        log::info!("  Level: {}", level_name);
    } else {
        log::info!("  Level: (empty world)");
    }

    // Determine initial camera using CameraSpec resolution
    let initial_camera = if let Some(camera_spec) = &scenario.camera {
        // User-specified camera spec
        resolve_camera_spec(
            camera_spec,
            &world,
            &materials,
            scenario.width,
            scenario.height,
        )?
    } else {
        // Default: AutoDetect with NonAir filter, 20px padding
        let bounds = detect_content_bounds(&world, &materials, &MaterialFilter::NonAir)
            .context("No content found for camera auto-detection")?;
        calculate_camera_from_bounds(bounds, 20, scenario.width, scenario.height)?
    };

    log::info!(
        "  Camera: center=({:.1}, {:.1}), zoom={:.2}",
        initial_camera.center.x,
        initial_camera.center.y,
        initial_camera.zoom
    );

    // Make camera mutable for dynamic adjustments via scenario actions
    let mut current_camera = initial_camera;

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
        // Execute scenario actions scheduled for this frame (including camera actions)
        if let Some(actions) = action_timeline.get(&frame) {
            for action in actions {
                execute_video_action(
                    action,
                    &mut world,
                    &mut current_camera,
                    &materials,
                    scenario.width,
                    scenario.height,
                    frame,
                );
            }
        }

        // Update world physics
        world.update(dt, &mut stats, &mut rng, false);

        // Capture frame at intervals using dynamic camera
        if frame % capture_interval == 0 {
            renderer.render(
                &world,
                &materials,
                current_camera.center,
                &[],
                current_camera.zoom,
            );
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
fn execute_video_action(
    action: &ScenarioAction,
    world: &mut World,
    camera: &mut CameraParams,
    materials: &Materials,
    viewport_width: u32,
    viewport_height: u32,
    frame: usize,
) {
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

        // Camera control actions
        ScenarioAction::SetCameraBounds {
            min_x,
            min_y,
            max_x,
            max_y,
            padding,
        } => {
            match calculate_camera_from_bounds(
                (*min_x, *min_y, *max_x, *max_y),
                *padding,
                viewport_width,
                viewport_height,
            ) {
                Ok(new_camera) => {
                    *camera = new_camera;
                    log::info!(
                        "Frame {}: Camera set to bounds ({}, {}) - ({}, {}) with padding {} -> center=({:.1}, {:.1}), zoom={:.2}",
                        frame,
                        min_x,
                        min_y,
                        max_x,
                        max_y,
                        padding,
                        camera.center.x,
                        camera.center.y,
                        camera.zoom
                    );
                }
                Err(e) => {
                    log::error!("Frame {}: Failed to set camera bounds: {}", frame, e);
                }
            }
        }

        ScenarioAction::SetCameraCenter { x, y } => {
            camera.center = Vec2::new(*x, *y);
            log::info!(
                "Frame {}: Camera center set to ({:.1}, {:.1})",
                frame,
                camera.center.x,
                camera.center.y
            );
        }

        ScenarioAction::SetCameraZoom { zoom } => {
            camera.zoom = *zoom;
            log::info!("Frame {}: Camera zoom set to {:.2}", frame, camera.zoom);
        }

        ScenarioAction::AutoFrameContent { filter, padding } => {
            match detect_content_bounds(world, materials, filter) {
                Some(bounds) => {
                    match calculate_camera_from_bounds(
                        bounds,
                        *padding,
                        viewport_width,
                        viewport_height,
                    ) {
                        Ok(new_camera) => {
                            *camera = new_camera;
                            log::info!(
                                "Frame {}: Auto-framed content -> center=({:.1}, {:.1}), zoom={:.2}",
                                frame,
                                camera.center.x,
                                camera.center.y,
                                camera.zoom
                            );
                        }
                        Err(e) => {
                            log::error!(
                                "Frame {}: Failed to calculate camera from bounds: {}",
                                frame,
                                e
                            );
                        }
                    }
                }
                None => {
                    log::warn!("Frame {}: No content found for auto-framing", frame);
                }
            }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::simulation::{MaterialId, MaterialType, Materials};

    /// Test camera calculation with square content (equal width/height)
    #[test]
    fn test_calculate_camera_square_content() {
        let bounds = (0, 0, 63, 63); // 64x64 square
        let padding = 20;
        let viewport = (1280, 720);

        let result = calculate_camera_from_bounds(bounds, padding, viewport.0, viewport.1);
        assert!(result.is_ok());

        let camera = result.unwrap();
        // Center should be at midpoint
        assert_eq!(camera.center.x, 31.5);
        assert_eq!(camera.center.y, 31.5);
        // Zoom should fit 64px + 40px padding = 104px into 720px height (limiting dimension)
        // zoom = 720 / 104 = ~6.92
        assert!((camera.zoom - 6.92).abs() < 0.1);
    }

    /// Test camera calculation with wide content (wider than tall)
    #[test]
    fn test_calculate_camera_wide_content() {
        let bounds = (0, 0, 199, 49); // 200x50 wide rectangle
        let padding = 10;
        let viewport = (1280, 720);

        let result = calculate_camera_from_bounds(bounds, padding, viewport.0, viewport.1);
        assert!(result.is_ok());

        let camera = result.unwrap();
        // Center should be at midpoint
        assert_eq!(camera.center.x, 99.5);
        assert_eq!(camera.center.y, 24.5);
        // Zoom limited by width: 1280 / (200 + 20) = ~5.82
        assert!((camera.zoom - 5.82).abs() < 0.1);
    }

    /// Test camera calculation with tall content (taller than wide)
    #[test]
    fn test_calculate_camera_tall_content() {
        let bounds = (0, 0, 49, 199); // 50x200 tall rectangle
        let padding = 10;
        let viewport = (1280, 720);

        let result = calculate_camera_from_bounds(bounds, padding, viewport.0, viewport.1);
        assert!(result.is_ok());

        let camera = result.unwrap();
        // Center should be at midpoint
        assert_eq!(camera.center.x, 24.5);
        assert_eq!(camera.center.y, 99.5);
        // Zoom limited by height: 720 / (200 + 20) = ~3.27
        assert!((camera.zoom - 3.27).abs() < 0.1);
    }

    /// Test camera calculation with single pixel content
    #[test]
    fn test_calculate_camera_single_pixel() {
        let bounds = (50, 50, 50, 50); // Single pixel at (50, 50)
        let padding = 20;
        let viewport = (1280, 720);

        let result = calculate_camera_from_bounds(bounds, padding, viewport.0, viewport.1);
        assert!(result.is_ok());

        let camera = result.unwrap();
        // Center should be at the pixel
        assert_eq!(camera.center.x, 50.0);
        assert_eq!(camera.center.y, 50.0);
        // Zoom should use minimum content size (10px) + padding
        // zoom = min(1280/50, 720/50) = min(25.6, 14.4) = 14.4
        assert!((camera.zoom - 14.4).abs() < 0.1);
    }

    /// Test camera calculation with very small content (uses minimum 10px)
    #[test]
    fn test_calculate_camera_tiny_content() {
        let bounds = (100, 100, 102, 101); // 3x2 content
        let padding = 5;
        let viewport = (800, 600);

        let result = calculate_camera_from_bounds(bounds, padding, viewport.0, viewport.1);
        assert!(result.is_ok());

        let camera = result.unwrap();
        // Should use minimum 10px content size
        // padded = 10 + 10 = 20px
        // zoom = min(800/20, 600/20) = min(40, 30) = 20 (clamped to max)
        assert_eq!(camera.zoom, 20.0); // Max zoom clamp
    }

    /// Test camera calculation with invalid bounds (max < min)
    #[test]
    fn test_calculate_camera_invalid_bounds() {
        let bounds = (100, 100, 50, 50); // max_x < min_x, max_y < min_y
        let padding = 20;
        let viewport = (1280, 720);

        let result = calculate_camera_from_bounds(bounds, padding, viewport.0, viewport.1);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Invalid bounds"));
    }

    /// Test camera calculation with negative coordinates
    #[test]
    fn test_calculate_camera_negative_coords() {
        let bounds = (-50, -30, 50, 30); // 101x61 centered at origin
        let padding = 20;
        let viewport = (1280, 720);

        let result = calculate_camera_from_bounds(bounds, padding, viewport.0, viewport.1);
        assert!(result.is_ok());

        let camera = result.unwrap();
        // Center should be at (0, 0)
        assert_eq!(camera.center.x, 0.0);
        assert_eq!(camera.center.y, 0.0);
        // Zoom limited by height: 720 / (61 + 40) = ~7.13
        assert!((camera.zoom - 7.13).abs() < 0.1);
    }

    /// Test camera calculation with zero padding
    #[test]
    fn test_calculate_camera_zero_padding() {
        let bounds = (0, 0, 99, 99); // 100x100 square
        let padding = 0;
        let viewport = (1000, 1000);

        let result = calculate_camera_from_bounds(bounds, padding, viewport.0, viewport.1);
        assert!(result.is_ok());

        let camera = result.unwrap();
        assert_eq!(camera.center.x, 49.5);
        assert_eq!(camera.center.y, 49.5);
        // Zoom should be exactly 10.0 (1000 / 100)
        assert_eq!(camera.zoom, 10.0);
    }

    /// Test camera calculation with large padding
    #[test]
    fn test_calculate_camera_large_padding() {
        let bounds = (0, 0, 9, 9); // 10x10 square
        let padding = 100;
        let viewport = (1280, 720);

        let result = calculate_camera_from_bounds(bounds, padding, viewport.0, viewport.1);
        assert!(result.is_ok());

        let camera = result.unwrap();
        // Padded size = 10 + 200 = 210px
        // zoom = min(1280/210, 720/210) = min(6.09, 3.43) = 3.43
        assert!((camera.zoom - 3.43).abs() < 0.1);
    }

    /// Helper to create a minimal Materials registry for testing
    /// Uses the default materials which include:
    /// - MaterialId::AIR = 0 (Gas)
    /// - MaterialId::STONE = 1 (Solid)
    /// - MaterialId::SAND = 2 (Powder)
    /// - MaterialId::WATER = 3 (Liquid)
    /// - MaterialId::STEAM = 7 (Gas)
    fn create_test_materials() -> Materials {
        Materials::new()
    }

    /// Test pixel_matches_filter with NonAir filter
    #[test]
    fn test_pixel_matches_filter_non_air() {
        let materials = create_test_materials();
        let filter = MaterialFilter::NonAir;

        assert!(!pixel_matches_filter(MaterialId::AIR, &materials, &filter));
        assert!(pixel_matches_filter(MaterialId::STONE, &materials, &filter));
        assert!(pixel_matches_filter(MaterialId::SAND, &materials, &filter));
        assert!(pixel_matches_filter(MaterialId::WATER, &materials, &filter));
        assert!(pixel_matches_filter(MaterialId::STEAM, &materials, &filter));
    }

    /// Test pixel_matches_filter with Types filter (specific types)
    #[test]
    fn test_pixel_matches_filter_types() {
        let materials = create_test_materials();
        let filter = MaterialFilter::Types(vec![MaterialType::Solid, MaterialType::Powder]);

        assert!(!pixel_matches_filter(MaterialId::AIR, &materials, &filter));
        assert!(pixel_matches_filter(MaterialId::STONE, &materials, &filter)); // Solid
        assert!(pixel_matches_filter(MaterialId::SAND, &materials, &filter)); // Powder
        assert!(!pixel_matches_filter(MaterialId::WATER, &materials, &filter)); // Liquid
        assert!(!pixel_matches_filter(MaterialId::STEAM, &materials, &filter)); // Gas
    }

    /// Test pixel_matches_filter with ExcludeTypes filter (exclude gases)
    #[test]
    fn test_pixel_matches_filter_exclude_types() {
        let materials = create_test_materials();
        let filter = MaterialFilter::ExcludeTypes(vec![MaterialType::Gas]);

        assert!(!pixel_matches_filter(MaterialId::AIR, &materials, &filter)); // Air (always excluded)
        assert!(pixel_matches_filter(MaterialId::STONE, &materials, &filter));
        assert!(pixel_matches_filter(MaterialId::SAND, &materials, &filter));
        assert!(pixel_matches_filter(MaterialId::WATER, &materials, &filter));
        assert!(!pixel_matches_filter(MaterialId::STEAM, &materials, &filter)); // Gas - excluded
    }

    /// Test pixel_matches_filter with Ids filter (specific material IDs)
    #[test]
    fn test_pixel_matches_filter_ids() {
        let materials = create_test_materials();
        let filter = MaterialFilter::Ids(vec![MaterialId::STONE, MaterialId::WATER]);

        assert!(!pixel_matches_filter(MaterialId::AIR, &materials, &filter));
        assert!(pixel_matches_filter(MaterialId::STONE, &materials, &filter)); // In list
        assert!(!pixel_matches_filter(MaterialId::SAND, &materials, &filter));
        assert!(pixel_matches_filter(MaterialId::WATER, &materials, &filter)); // In list
        assert!(!pixel_matches_filter(MaterialId::STEAM, &materials, &filter));
    }

    /// Test pixel_matches_filter with ExcludeIds filter
    #[test]
    fn test_pixel_matches_filter_exclude_ids() {
        let materials = create_test_materials();
        let filter = MaterialFilter::ExcludeIds(vec![MaterialId::SAND, MaterialId::STEAM]);

        assert!(!pixel_matches_filter(MaterialId::AIR, &materials, &filter)); // Air (always excluded)
        assert!(pixel_matches_filter(MaterialId::STONE, &materials, &filter));
        assert!(!pixel_matches_filter(MaterialId::SAND, &materials, &filter)); // Excluded
        assert!(pixel_matches_filter(MaterialId::WATER, &materials, &filter));
        assert!(!pixel_matches_filter(MaterialId::STEAM, &materials, &filter)); // Excluded
    }

    /// Test CameraSpec serialization/deserialization (Bounds variant)
    #[test]
    fn test_camera_spec_bounds_serde() {
        let spec = CameraSpec::Bounds {
            min_x: -50,
            min_y: -30,
            max_x: 100,
            max_y: 80,
            padding: 20,
        };

        let ron_str = ron::to_string(&spec).unwrap();
        let deserialized: CameraSpec = ron::from_str(&ron_str).unwrap();

        match deserialized {
            CameraSpec::Bounds { min_x, min_y, max_x, max_y, padding } => {
                assert_eq!(min_x, -50);
                assert_eq!(min_y, -30);
                assert_eq!(max_x, 100);
                assert_eq!(max_y, 80);
                assert_eq!(padding, 20);
            }
            _ => panic!("Unexpected variant"),
        }
    }

    /// Test CameraSpec serialization/deserialization (AutoDetect variant)
    #[test]
    fn test_camera_spec_auto_detect_serde() {
        let spec = CameraSpec::AutoDetect {
            filter: MaterialFilter::ExcludeTypes(vec![MaterialType::Gas]),
            padding: 15,
        };

        let ron_str = ron::to_string(&spec).unwrap();
        let deserialized: CameraSpec = ron::from_str(&ron_str).unwrap();

        match deserialized {
            CameraSpec::AutoDetect { filter, padding } => {
                assert_eq!(padding, 15);
                match filter {
                    MaterialFilter::ExcludeTypes(types) => {
                        assert_eq!(types.len(), 1);
                        assert_eq!(types[0], MaterialType::Gas);
                    }
                    _ => panic!("Unexpected filter variant"),
                }
            }
            _ => panic!("Unexpected variant"),
        }
    }

    /// Test CameraSpec serialization/deserialization (Manual variant)
    #[test]
    fn test_camera_spec_manual_serde() {
        let spec = CameraSpec::Manual {
            center: Vec2::new(123.5, 456.7),
            zoom: 8.5,
        };

        let ron_str = ron::to_string(&spec).unwrap();
        let deserialized: CameraSpec = ron::from_str(&ron_str).unwrap();

        match deserialized {
            CameraSpec::Manual { center, zoom } => {
                assert!((center.x - 123.5).abs() < 0.001);
                assert!((center.y - 456.7).abs() < 0.001);
                assert!((zoom - 8.5).abs() < 0.001);
            }
            _ => panic!("Unexpected variant"),
        }
    }

    /// Test MaterialFilter default implementation
    #[test]
    fn test_material_filter_default() {
        let filter = MaterialFilter::default();
        match filter {
            MaterialFilter::NonAir => {}
            _ => panic!("Default should be NonAir"),
        }
    }

    /// Test zoom clamping to maximum value
    #[test]
    fn test_calculate_camera_zoom_clamp_max() {
        let bounds = (0, 0, 1, 1); // 2x2 tiny content
        let padding = 1;
        let viewport = (1280, 720);

        let result = calculate_camera_from_bounds(bounds, padding, viewport.0, viewport.1);
        assert!(result.is_ok());

        let camera = result.unwrap();
        // Without clamp, zoom would be 720 / (2 + 2) = 180
        // Should be clamped to 20.0
        assert_eq!(camera.zoom, 20.0);
    }

    /// Test zoom clamping to minimum value
    #[test]
    fn test_calculate_camera_zoom_clamp_min() {
        let bounds = (0, 0, 10000, 10000); // Huge content
        let padding = 1000;
        let viewport = (100, 100);

        let result = calculate_camera_from_bounds(bounds, padding, viewport.0, viewport.1);
        assert!(result.is_ok());

        let camera = result.unwrap();
        // Without clamp, zoom would be 100 / (10001 + 2000) = 0.00833
        // Should be clamped to 0.1
        assert_eq!(camera.zoom, 0.1);
    }
}
