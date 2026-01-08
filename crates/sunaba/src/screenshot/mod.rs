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

mod offscreen_renderer;
mod sample_data;
pub mod scenario;

use anyhow::Result;
use glam::Vec2;
use std::path::Path;

use crate::headless::PixelRenderer;
use crate::levels::LevelManager;
use crate::simulation::{MaterialId, Materials};
use crate::world::{NoopStats, World};
use rand::thread_rng;

use offscreen_renderer::OffscreenRenderer;

// Imports for UI screenshot capture
#[cfg(not(target_arch = "wasm32"))]
use crate::config::GameConfig;
use crate::entity::{crafting::RecipeRegistry, player::Player, tools::ToolRegistry};
use crate::ui::{dock::DockTab, ui_state::UiState};

pub use scenario::{ScreenshotScenario, list_all_scenarios};

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
    let mut config = GameConfig::default();

    // 3. Initialize UI state
    let mut ui_state = UiState::new(&config);

    // 4. Open the target panel
    match panel {
        UiPanel::Params => ui_state.dock.open_tab(DockTab::Parameters),
        UiPanel::Inventory => ui_state.dock.open_tab(DockTab::Inventory),
        UiPanel::Crafting => ui_state.dock.open_tab(DockTab::Crafting),
        UiPanel::Logger => ui_state.dock.open_tab(DockTab::Logger),
        UiPanel::WorldGen => {
            // WorldGen editor is not in dock, it's a separate window
            // We'll need to render it differently
            anyhow::bail!("WorldGen screenshot not yet implemented")
        }
        UiPanel::LevelSelector => ui_state.dock.open_tab(DockTab::LevelSelector),
        #[cfg(feature = "multiplayer")]
        UiPanel::Multiplayer => ui_state.dock.open_tab(DockTab::MultiplayerStats),
    }

    // 5. Render UI to offscreen texture
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

        // Render the UI state (only the open panel will render)
        ui_state.render(
            ctx,
            egui::pos2(0.0, 0.0), // cursor pos (not used)
            MaterialId::AIR,
            &materials,
            "Screenshot Mode",
            false,
            &level_manager,
            &player,
            &tool_registry,
            &recipe_registry,
            &mut config,
            false, // no game over
            #[cfg(feature = "multiplayer")]
            None, // no multiplayer manager
        );
    })?;

    // 6. Save as PNG
    log::info!("Saving to {:?}...", output_path.as_ref());
    save_buffer_as_png(&pixels, width, height, output_path)?;

    log::info!("Screenshot saved successfully!");
    Ok(())
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
        ScreenshotScenario::Interactive { ref name, .. } => {
            anyhow::bail!("Interactive scenarios not yet implemented: {}", name)
        }
    }
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
