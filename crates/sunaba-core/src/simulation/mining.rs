//! Mining system with hardness-based time calculations

use crate::entity::tools::ToolDef;
use crate::simulation::MaterialDef;
use serde::{Deserialize, Serialize};

/// Mining progress tracker (one per player)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MiningProgress {
    pub target_pixel: Option<(i32, i32)>, // World coordinates
    pub progress: f32,                    // 0.0 - 1.0
    pub required_time: f32,               // Seconds to complete
}

impl MiningProgress {
    pub fn new() -> Self {
        Self {
            target_pixel: None,
            progress: 0.0,
            required_time: 0.0,
        }
    }

    /// Start mining a new pixel
    pub fn start(&mut self, pos: (i32, i32), required_time: f32) {
        self.target_pixel = Some(pos);
        self.progress = 0.0;
        self.required_time = required_time;
    }

    /// Update mining progress
    /// Returns true if mining completed
    pub fn update(&mut self, delta_time: f32) -> bool {
        if self.target_pixel.is_none() {
            return false;
        }

        self.progress += delta_time / self.required_time;

        if self.progress >= 1.0 {
            self.reset();
            true
        } else {
            false
        }
    }

    /// Cancel current mining
    pub fn reset(&mut self) {
        self.target_pixel = None;
        self.progress = 0.0;
        self.required_time = 0.0;
    }

    /// Get progress as percentage (0-100)
    pub fn get_percentage(&self) -> f32 {
        (self.progress * 100.0).min(100.0)
    }

    /// Check if currently mining
    pub fn is_mining(&self) -> bool {
        self.target_pixel.is_some()
    }
}

impl Default for MiningProgress {
    fn default() -> Self {
        Self::new()
    }
}

/// Calculate time required to mine a material
///
/// Formula: time = base_time * hardness * hardness_mult / tool_speed
///
/// Examples:
/// - Stone (hardness=5, mult=1.0) with wood pickaxe (speed=1.0): 5.0 seconds
/// - Iron ore (hardness=5, mult=2.0) with wood pickaxe (speed=0.5): 20.0 seconds
/// - Iron ore with iron pickaxe (speed=1.5): 6.7 seconds
pub fn calculate_mining_time(
    base_time: f32,
    material: &MaterialDef,
    tool: Option<&ToolDef>,
) -> f32 {
    // Bedrock/Air can't be mined (no hardness)
    if material.hardness.is_none() {
        return f32::INFINITY;
    }

    let hardness = material.hardness.unwrap() as f32;
    let hardness_mult = material.hardness_multiplier;

    let tool_speed = if let Some(t) = tool {
        t.get_mining_speed(material)
    } else {
        0.5 // No tool = 50% base speed
    };

    // Formula: time = base * hardness * hardness_mult / tool_speed
    base_time * hardness * hardness_mult / tool_speed
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entity::tools::{ToolDef, ToolTier, ToolType};
    use crate::simulation::{MaterialDef, MaterialTag, MaterialType};

    fn make_test_material(hardness: u8, hardness_mult: f32, tags: Vec<MaterialTag>) -> MaterialDef {
        MaterialDef {
            id: 1,
            name: "Test".to_string(),
            material_type: MaterialType::Solid,
            color: [0, 0, 0, 255],
            density: 1.0,
            hardness: Some(hardness),
            hardness_multiplier: hardness_mult,
            friction: 0.5,
            viscosity: 0.5,
            melting_point: None,
            boiling_point: None,
            freezing_point: None,
            ignition_temp: None,
            heat_conductivity: 0.0,
            melts_to: None,
            boils_to: None,
            freezes_to: None,
            burns_to: None,
            burn_rate: 0.0,
            flammable: false,
            structural: false,
            conducts_electricity: false,
            electrical_conductivity: 0.0,
            electrical_resistance: 0.0,
            spark_threshold: 0.0,
            power_generation: 0.0,
            power_decay_rate: 0.0,
            nutritional_value: None,
            toxicity: None,
            structural_strength: None,
            fuel_value: None,
            tags,
        }
    }

    fn make_test_tool(tier: ToolTier) -> ToolDef {
        ToolDef {
            id: 1000,
            name: "Test Tool".to_string(),
            tool_type: ToolType::Pickaxe,
            tier,
            can_harvest: vec![MaterialTag::Mineral, MaterialTag::Ore],
        }
    }

    #[test]
    fn test_mining_progress_basic() {
        let mut progress = MiningProgress::new();
        assert!(!progress.is_mining());
        assert_eq!(progress.get_percentage(), 0.0);

        // Start mining
        progress.start((10, 20), 5.0);
        assert!(progress.is_mining());
        assert_eq!(progress.target_pixel, Some((10, 20)));
        assert_eq!(progress.required_time, 5.0);

        // Update progress (1 second passed)
        let completed = progress.update(1.0);
        assert!(!completed);
        assert_eq!(progress.get_percentage(), 20.0);

        // Update progress (4 more seconds - total 5 seconds)
        let completed = progress.update(4.0);
        assert!(completed);
        assert!(!progress.is_mining()); // Should be reset after completion
    }

    #[test]
    fn test_mining_progress_reset() {
        let mut progress = MiningProgress::new();
        progress.start((5, 10), 10.0);
        progress.update(3.0); // 30% progress

        progress.reset();
        assert!(!progress.is_mining());
        assert_eq!(progress.get_percentage(), 0.0);
        assert_eq!(progress.target_pixel, None);
    }

    #[test]
    fn test_calculate_mining_time() {
        let base_time = 1.0;

        // Stone: hardness=5, mult=1.0
        let stone = make_test_material(5, 1.0, vec![MaterialTag::Mineral]);

        // Iron ore: hardness=5, mult=2.0
        let iron_ore = make_test_material(5, 2.0, vec![MaterialTag::Ore]);

        let wood_pick = make_test_tool(ToolTier::Wood); // 1.0x speed
        let iron_pick = make_test_tool(ToolTier::Iron); // 3.0x speed

        // Stone with wood pickaxe: 1.0 * 5 * 1.0 / 1.0 = 5.0s
        let time = calculate_mining_time(base_time, &stone, Some(&wood_pick));
        assert_eq!(time, 5.0);

        // Iron ore with wood pickaxe: 1.0 * 5 * 2.0 / 0.5 = 20.0s
        // (wood pickaxe gets 0.5 speed on 2.0x hardness material)
        let time = calculate_mining_time(base_time, &iron_ore, Some(&wood_pick));
        assert_eq!(time, 20.0);

        // Iron ore with iron pickaxe: 1.0 * 5 * 2.0 / 1.5 ≈ 6.67s
        // (iron pickaxe gets 1.5 speed on 2.0x hardness material)
        let time = calculate_mining_time(base_time, &iron_ore, Some(&iron_pick));
        assert!((time - 6.67).abs() < 0.01);

        // No tool (50% speed): 1.0 * 5 * 1.0 / 0.5 = 10.0s
        let time = calculate_mining_time(base_time, &stone, None);
        assert_eq!(time, 10.0);
    }

    #[test]
    fn test_unmineable_materials() {
        let bedrock = MaterialDef {
            hardness: None, // Unbreakable
            ..make_test_material(0, 1.0, vec![])
        };

        let wood_pick = make_test_tool(ToolTier::Wood);

        let time = calculate_mining_time(1.0, &bedrock, Some(&wood_pick));
        assert!(time.is_infinite());
    }
}
