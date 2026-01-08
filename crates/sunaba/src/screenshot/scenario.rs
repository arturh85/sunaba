//! Screenshot scenario types and parsing
//!
//! Defines different types of screenshot scenarios (levels, UI panels, interactive)
//! and provides parsing/listing utilities.

use anyhow::Result;

/// Screenshot scenario - what to capture
#[derive(Debug, Clone)]
pub enum ScreenshotScenario {
    /// Render a demo level (existing functionality)
    Level { id: usize, settle_frames: usize },

    /// Render a UI panel on solid background
    UiPanel {
        panel: super::UiPanel,
        background_color: [u8; 4],
        with_sample_data: bool,
    },

    /// Render world background with UI panels overlaid
    Composite {
        level_id: usize,
        panels: Vec<super::UiPanel>,
        settle_frames: usize,
    },

    /// Render with a predefined layout template
    Layout {
        layout_name: String,
        level_id: Option<usize>,
        settle_frames: usize,
    },

    /// Future: Interactive scenario with setup steps
    #[allow(dead_code)]
    Interactive {
        name: String,
        setup: Vec<ScenarioAction>,
    },
}

/// Actions to perform in interactive scenarios (future extensibility)
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum ScenarioAction {
    /// Open a UI panel
    OpenPanel(super::UiPanel),
    /// Close a UI panel
    ClosePanel(super::UiPanel),
    /// Simulate a key press
    SimulateKeyPress(String), // KeyCode as string for now
    /// Wait N frames before continuing
    WaitFrames(usize),
    /// Click a button by label
    ClickButton(String),
    /// Set player position
    SetPlayerPosition(f32, f32),
}

impl ScreenshotScenario {
    /// Parse a scenario string like "level:3", "ui:params", "composite:3:inventory", "scenario:name"
    pub fn parse(s: &str, settle_frames: usize) -> Result<Self> {
        if let Some(rest) = s.strip_prefix("level:") {
            let id = rest
                .parse::<usize>()
                .map_err(|_| anyhow::anyhow!("Invalid level ID: {}", rest))?;
            return Ok(ScreenshotScenario::Level { id, settle_frames });
        }

        if let Some(rest) = s.strip_prefix("ui:") {
            let panel = super::UiPanel::from_str(rest)
                .ok_or_else(|| anyhow::anyhow!("Unknown UI panel: {}", rest))?;
            return Ok(ScreenshotScenario::UiPanel {
                panel,
                background_color: [45, 45, 48, 255], // Dark gray background (VS Code dark theme)
                with_sample_data: true,
            });
        }

        if let Some(rest) = s.strip_prefix("composite:") {
            // Format: composite:LEVEL_ID:panel1,panel2,panel3
            // Example: composite:3:inventory,crafting
            let parts: Vec<&str> = rest.split(':').collect();
            if parts.len() != 2 {
                anyhow::bail!(
                    "Invalid composite format: '{}'. Expected 'composite:LEVEL_ID:panel1,panel2'",
                    s
                );
            }

            let level_id = parts[0]
                .parse::<usize>()
                .map_err(|_| anyhow::anyhow!("Invalid level ID: {}", parts[0]))?;

            let panel_names: Vec<&str> = parts[1].split(',').collect();
            let mut panels = Vec::new();
            for name in panel_names {
                let panel = super::UiPanel::from_str(name.trim())
                    .ok_or_else(|| anyhow::anyhow!("Unknown UI panel: {}", name))?;
                panels.push(panel);
            }

            if panels.is_empty() {
                anyhow::bail!("Composite screenshot requires at least one panel");
            }

            return Ok(ScreenshotScenario::Composite {
                level_id,
                panels,
                settle_frames,
            });
        }

        if let Some(rest) = s.strip_prefix("layout:") {
            // Format: layout:LAYOUT_NAME or layout:LAYOUT_NAME:LEVEL_ID
            // Example: layout:default or layout:inventory:3
            let parts: Vec<&str> = rest.split(':').collect();
            let layout_name = parts[0].to_string();
            let level_id = if parts.len() >= 2 {
                Some(
                    parts[1]
                        .parse::<usize>()
                        .map_err(|_| anyhow::anyhow!("Invalid level ID: {}", parts[1]))?,
                )
            } else {
                None
            };

            return Ok(ScreenshotScenario::Layout {
                layout_name,
                level_id,
                settle_frames,
            });
        }

        if s.strip_prefix("scenario:").is_some() {
            anyhow::bail!("Interactive scenarios not yet implemented");
        }

        // Try parsing as plain number (backward compatible: "3" -> "level:3")
        if let Ok(id) = s.parse::<usize>() {
            return Ok(ScreenshotScenario::Level { id, settle_frames });
        }

        anyhow::bail!(
            "Invalid scenario format: '{}'. Expected 'level:N', 'ui:panel', 'composite:N:panels', or just 'N'",
            s
        )
    }

    /// Get a descriptive name for this scenario
    pub fn name(&self) -> String {
        match self {
            ScreenshotScenario::Level { id, .. } => format!("level_{}", id),
            ScreenshotScenario::UiPanel { panel, .. } => format!("ui_{}", panel.as_str()),
            ScreenshotScenario::Composite {
                level_id, panels, ..
            } => {
                let panel_str = panels
                    .iter()
                    .map(|p| p.as_str())
                    .collect::<Vec<_>>()
                    .join("_");
                format!("composite_level{}_{}", level_id, panel_str)
            }
            ScreenshotScenario::Layout {
                layout_name,
                level_id,
                ..
            } => {
                if let Some(id) = level_id {
                    format!("layout_{}_level{}", layout_name, id)
                } else {
                    format!("layout_{}", layout_name)
                }
            }
            ScreenshotScenario::Interactive { name, .. } => format!("scenario_{}", name),
        }
    }
}

/// Extension trait for UiPanel to get string representation
pub trait UiPanelExt {
    fn as_str(&self) -> &'static str;
}

impl UiPanelExt for super::UiPanel {
    fn as_str(&self) -> &'static str {
        match self {
            super::UiPanel::Params => "params",
            #[cfg(feature = "multiplayer")]
            super::UiPanel::Multiplayer => "multiplayer",
            super::UiPanel::Inventory => "inventory",
            super::UiPanel::Crafting => "crafting",
            super::UiPanel::Logger => "logger",
            super::UiPanel::WorldGen => "worldgen",
            super::UiPanel::LevelSelector => "levels",
        }
    }
}

/// List all available scenarios
pub fn list_all_scenarios() {
    use crate::levels::LevelManager;

    println!("Available screenshot scenarios:");
    println!();

    // List demo levels
    println!("Levels:");
    let level_manager = LevelManager::new();
    for level in level_manager.levels() {
        println!("  level:{:<3} - {}", level.id, level.name);
    }
    println!();

    // List UI panels
    println!("UI Panels:");
    println!("  ui:params      - Parameters/Settings Panel");
    #[cfg(feature = "multiplayer")]
    println!("  ui:multiplayer - Multiplayer Status Panel");
    println!("  ui:inventory   - Player Inventory Panel");
    println!("  ui:crafting    - Crafting UI Panel");
    println!("  ui:logger      - Log Viewer Panel");
    println!("  ui:worldgen    - World Generation Editor");
    println!("  ui:levels      - Level Selector Panel");
    println!();

    println!("Usage:");
    println!("  just screenshot level:3");
    println!("  just screenshot ui:params");
    println!("  cargo run --release --features headless -- --screenshot ui:inventory");
}
