//! Video scenario definitions for documentation and showcases
//!
//! Defines animated scenarios that demonstrate game features through MP4 videos.

use serde::{Deserialize, Serialize};

/// A video scenario that captures game simulation as MP4
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoScenario {
    /// Unique identifier for the scenario
    pub id: &'static str,

    /// Human-readable name
    pub name: &'static str,

    /// Description of what the scenario demonstrates
    pub description: &'static str,

    /// Optional demo level to load (None for empty world)
    pub level_id: Option<usize>,

    /// Duration of the video in seconds
    pub duration_seconds: f32,

    /// Target frames per second for the output video
    pub fps: u32,

    /// Video width in pixels
    pub width: u32,

    /// Video height in pixels
    pub height: u32,

    /// Actions to perform during the scenario
    pub actions: Vec<ScenarioAction>,
}

/// Actions that can be performed during a video scenario
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ScenarioAction {
    /// Wait for a number of frames (does nothing, just advances simulation)
    Wait {
        /// Number of frames to wait
        frames: usize,
    },

    /// Mine a circular area
    MineCircle {
        /// Center X coordinate (world space)
        x: i32,
        /// Center Y coordinate (world space)
        y: i32,
        /// Radius in pixels
        radius: i32,
    },

    /// Place material in a circular area
    PlaceMaterial {
        /// Center X coordinate (world space)
        x: i32,
        /// Center Y coordinate (world space)
        y: i32,
        /// Material ID (u16)
        material: u16,
        /// Radius in pixels
        radius: i32,
    },

    /// Remove support structure (for collapse scenarios)
    RemoveSupport {
        /// Min X coordinate (world space)
        x: i32,
        /// Min Y coordinate (world space)
        y: i32,
        /// Width in pixels
        width: i32,
        /// Height in pixels
        height: i32,
    },

    /// Teleport player to position
    TeleportPlayer {
        /// X coordinate (world space)
        x: f32,
        /// Y coordinate (world space)
        y: f32,
    },

    /// Simulate player mining toward a target
    SimulatePlayerMining {
        /// Target X coordinate (world space)
        target_x: f32,
        /// Target Y coordinate (world space)
        target_y: f32,
        /// Duration in frames
        duration_frames: usize,
    },
}

/// Get all predefined video scenarios
pub fn get_all_scenarios() -> Vec<VideoScenario> {
    vec![
        // Emergent Physics
        create_fire_spread_scenario(),
        create_water_flow_scenario(),
        create_bridge_collapse_scenario(),
        create_lava_water_reaction_scenario(),
        // Gameplay
        create_player_mining_scenario(),
        // Chemistry/Crafting
        create_smelting_demo_scenario(),
        // Advanced Systems
        create_plant_growth_scenario(),
        create_material_reactions_scenario(),
    ]
}

/// Get a scenario by ID
pub fn get_scenario_by_id(id: &str) -> Option<VideoScenario> {
    get_all_scenarios().into_iter().find(|s| s.id == id)
}

/// Fire spreading through wooden structures (Emergent Physics)
fn create_fire_spread_scenario() -> VideoScenario {
    VideoScenario {
        id: "fire_spread",
        name: "Fire Spreading",
        description: "Fire propagates through wooden structures, demonstrating cellular automata and emergent behavior",
        level_id: Some(1), // Inferno level (wood columns with fire at base)
        duration_seconds: 5.0,
        fps: 20,
        width: 1280,
        height: 720,
        actions: vec![
            // Let fire naturally spread for 5 seconds (no additional actions needed)
            // The Inferno level already has fire at the base of wood columns
            ScenarioAction::Wait { frames: 300 }, // 5 seconds @ 60fps physics
        ],
    }
}

/// Water flowing down stepped platforms (Powder/Liquid Physics)
fn create_water_flow_scenario() -> VideoScenario {
    VideoScenario {
        id: "water_flow",
        name: "Water Flow",
        description: "Water flows down stepped platforms, showcasing liquid physics, viscosity, and gravity",
        level_id: Some(5), // Liquid Lab level (water and oil on stepped platforms)
        duration_seconds: 5.0,
        fps: 20,
        width: 1280,
        height: 720,
        actions: vec![
            // Let water flow naturally for 5 seconds
            ScenarioAction::Wait { frames: 300 }, // 5 seconds @ 60fps physics
        ],
    }
}

/// Bridge collapse with cascade failure (Structural Integrity)
fn create_bridge_collapse_scenario() -> VideoScenario {
    VideoScenario {
        id: "bridge_collapse",
        name: "Bridge Collapse",
        description: "Bridge collapses after pillar removal, demonstrating structural dependency and debris physics",
        level_id: Some(8), // Bridge Demolition level
        duration_seconds: 6.0,
        fps: 20,
        width: 1280,
        height: 720,
        actions: vec![
            ScenarioAction::Wait { frames: 60 }, // Wait 1 second
            ScenarioAction::RemoveSupport {
                x: -5,
                y: 40,
                width: 10,
                height: 50,
            }, // Remove pillar
            ScenarioAction::Wait { frames: 300 }, // Watch collapse (5 seconds)
        ],
    }
}

/// Lava meets water reaction (Chemistry)
fn create_lava_water_reaction_scenario() -> VideoScenario {
    VideoScenario {
        id: "lava_water_reaction",
        name: "Lava-Water Reaction",
        description: "Lava meets water, creating steam and demonstrating temperature-based state changes",
        level_id: Some(2), // Lava Meets Water level
        duration_seconds: 5.0,
        fps: 20,
        width: 1280,
        height: 720,
        actions: vec![
            ScenarioAction::Wait { frames: 30 }, // Wait 0.5 seconds
            ScenarioAction::RemoveSupport {
                x: -10,
                y: 50,
                width: 20,
                height: 80,
            }, // Remove separator wall
            ScenarioAction::Wait { frames: 270 }, // Watch reaction (4.5 seconds)
        ],
    }
}

/// Player mining demonstration (Gameplay)
fn create_player_mining_scenario() -> VideoScenario {
    VideoScenario {
        id: "player_mining",
        name: "Player Mining",
        description: "Player mines stone and collects materials, showcasing core gameplay loop",
        level_id: Some(16), // Survival Tutorial level
        duration_seconds: 8.0,
        fps: 20,
        width: 1280,
        height: 720,
        actions: vec![
            ScenarioAction::TeleportPlayer { x: 0.0, y: 100.0 },
            ScenarioAction::Wait { frames: 30 }, // Brief pause
            ScenarioAction::SimulatePlayerMining {
                target_x: 20.0,
                target_y: 80.0,
                duration_frames: 180,
            }, // Mine for 3 seconds
            ScenarioAction::Wait { frames: 270 }, // Show result (4.5 seconds)
        ],
    }
}

/// Iron smelting demonstration (Chemistry/Crafting)
fn create_smelting_demo_scenario() -> VideoScenario {
    VideoScenario {
        id: "smelting_demo",
        name: "Smelting Demo",
        description: "Iron ore transforms into iron ingot through smelting, demonstrating crafting system",
        level_id: Some(18), // Alchemy Lab level
        duration_seconds: 10.0,
        fps: 15,
        width: 1280,
        height: 720,
        actions: vec![
            // Let the existing smelting chambers demonstrate the process
            ScenarioAction::Wait { frames: 600 }, // 10 seconds to show reactions
        ],
    }
}

/// Plant growth under light (Advanced Systems)
fn create_plant_growth_scenario() -> VideoScenario {
    VideoScenario {
        id: "plant_growth",
        name: "Plant Growth",
        description: "Plants grow under light, demonstrating light propagation and organic growth systems",
        level_id: Some(20), // Day/Night Cycle level
        duration_seconds: 8.0,
        fps: 15,
        width: 1280,
        height: 720,
        actions: vec![
            // Show plant growth over time
            ScenarioAction::Wait { frames: 480 }, // 8 seconds for growth
        ],
    }
}

/// Multiple material reactions showcase (Chemistry)
fn create_material_reactions_scenario() -> VideoScenario {
    VideoScenario {
        id: "material_reactions",
        name: "Material Reactions",
        description: "Comprehensive showcase of material transformations: gunpowder explosion, acid corrosion, flesh cooking",
        level_id: Some(18), // Alchemy Lab level
        duration_seconds: 10.0,
        fps: 15,
        width: 1280,
        height: 720,
        actions: vec![
            ScenarioAction::Wait { frames: 120 }, // Let initial reactions settle
            // Trigger gunpowder explosion (if present in level)
            ScenarioAction::PlaceMaterial {
                x: -50,
                y: 80,
                material: 22,
                radius: 3,
            }, // Gunpowder (material ID 22)
            ScenarioAction::PlaceMaterial {
                x: -50,
                y: 83,
                material: 5,
                radius: 1,
            }, // Fire on top
            ScenarioAction::Wait { frames: 480 }, // Watch reactions (8 seconds)
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_all_scenarios() {
        let scenarios = get_all_scenarios();
        assert_eq!(scenarios.len(), 8, "Should have 8 scenarios in Phase 2");

        // Verify fire_spread scenario exists
        let fire_spread = scenarios.iter().find(|s| s.id == "fire_spread");
        assert!(fire_spread.is_some(), "fire_spread scenario should exist");

        let fire_spread = fire_spread.unwrap();
        assert_eq!(fire_spread.name, "Fire Spreading");
        assert_eq!(fire_spread.level_id, Some(1));
        assert_eq!(fire_spread.fps, 20);
        assert_eq!(fire_spread.width, 1280);
        assert_eq!(fire_spread.height, 720);
    }

    #[test]
    fn test_get_scenario_by_id() {
        let fire_spread = get_scenario_by_id("fire_spread");
        assert!(fire_spread.is_some());
        assert_eq!(fire_spread.unwrap().id, "fire_spread");

        let nonexistent = get_scenario_by_id("nonexistent");
        assert!(nonexistent.is_none());
    }

    #[test]
    fn test_water_flow_scenario() {
        let water_flow = get_scenario_by_id("water_flow");
        assert!(water_flow.is_some());

        let water_flow = water_flow.unwrap();
        assert_eq!(water_flow.id, "water_flow");
        assert_eq!(water_flow.level_id, Some(5));
        assert_eq!(water_flow.duration_seconds, 5.0);
        assert_eq!(water_flow.actions.len(), 1); // Just Wait action
    }
}
