//! Tool system for mining and crafting

use crate::simulation::{MaterialDef, MaterialTag};
use serde::{Deserialize, Serialize};

/// Tool types with different use cases
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ToolType {
    /// Mining solid blocks and ores
    Pickaxe,
    /// Harvesting wood/plants (future - Phase 8)
    Axe,
    /// Combat (future - Phase 8)
    Sword,
    /// Digging powder materials (future - Phase 8)
    Shovel,
}

/// Tool tier affects speed and durability
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ToolTier {
    /// 1.0x speed, 50 uses
    Wood,
    /// 1.5x speed, 100 uses
    Stone,
    /// 2.0x speed, 200 uses
    Copper,
    /// 2.5x speed, 300 uses
    Bronze,
    /// 3.0x speed, 400 uses
    Iron,
    /// 4.0x speed, 600 uses
    Steel,
}

impl ToolTier {
    /// Get the speed multiplier for this tier
    pub fn speed_multiplier(&self) -> f32 {
        match self {
            ToolTier::Wood => 1.0,
            ToolTier::Stone => 1.5,
            ToolTier::Copper => 2.0,
            ToolTier::Bronze => 2.5,
            ToolTier::Iron => 3.0,
            ToolTier::Steel => 4.0,
        }
    }

    /// Get the max durability for this tier
    pub fn max_durability(&self) -> u32 {
        match self {
            ToolTier::Wood => 50,
            ToolTier::Stone => 100,
            ToolTier::Copper => 200,
            ToolTier::Bronze => 300,
            ToolTier::Iron => 400,
            ToolTier::Steel => 600,
        }
    }
}

/// Tool definition (data-driven, registered in ToolRegistry)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDef {
    pub id: u16,
    pub name: String,
    pub tool_type: ToolType,
    pub tier: ToolTier,
    pub can_harvest: Vec<MaterialTag>,
}

impl ToolDef {
    /// Calculate mining speed for a given material
    /// Returns multiplier (1.0 = base, 2.0 = 2x faster, 0.1 = wrong tool penalty)
    pub fn get_mining_speed(&self, material: &MaterialDef) -> f32 {
        // Check if tool can harvest this material type
        let can_harvest = material
            .tags
            .iter()
            .any(|tag| self.can_harvest.contains(tag));

        if !can_harvest {
            return 0.1; // 10x slower penalty for wrong tool
        }

        // Speed = tool_speed / material_hardness_multiplier
        self.tier.speed_multiplier() / material.hardness_multiplier
    }

    /// Get max durability for this tool
    pub fn max_durability(&self) -> u32 {
        self.tier.max_durability()
    }
}

/// Tool registry (singleton, loaded once)
#[derive(Debug, Clone)]
pub struct ToolRegistry {
    tools: Vec<ToolDef>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        let mut registry = Self { tools: Vec::new() };
        registry.register_default_tools();
        registry
    }

    fn register_default_tools(&mut self) {
        // Wood Pickaxe - Basic mining
        self.register(ToolDef {
            id: 1000,
            name: "Wood Pickaxe".to_string(),
            tool_type: ToolType::Pickaxe,
            tier: ToolTier::Wood,
            can_harvest: vec![MaterialTag::Mineral, MaterialTag::Ore],
        });

        // Stone Pickaxe - Better mining
        self.register(ToolDef {
            id: 1001,
            name: "Stone Pickaxe".to_string(),
            tool_type: ToolType::Pickaxe,
            tier: ToolTier::Stone,
            can_harvest: vec![MaterialTag::Mineral, MaterialTag::Ore],
        });

        // Iron Pickaxe - Fast mining
        self.register(ToolDef {
            id: 1002,
            name: "Iron Pickaxe".to_string(),
            tool_type: ToolType::Pickaxe,
            tier: ToolTier::Iron,
            can_harvest: vec![MaterialTag::Mineral, MaterialTag::Ore],
        });

        // Future (Phase 8): Add axes, shovels, swords for expanded gameplay
    }

    fn register(&mut self, tool: ToolDef) {
        // Tools use IDs 1000+, so we need to ensure the vec is large enough
        let index = (tool.id - 1000) as usize;
        if self.tools.len() <= index {
            self.tools.resize(index + 1, self.dummy_tool());
        }
        self.tools[index] = tool;
    }

    /// Create a dummy tool for vec resizing
    fn dummy_tool(&self) -> ToolDef {
        ToolDef {
            id: 0,
            name: "INVALID".to_string(),
            tool_type: ToolType::Pickaxe,
            tier: ToolTier::Wood,
            can_harvest: vec![],
        }
    }

    /// Get a tool definition by ID
    pub fn get(&self, id: u16) -> Option<&ToolDef> {
        if id < 1000 {
            return None;
        }
        let index = (id - 1000) as usize;
        self.tools.get(index).filter(|t| t.id != 0)
    }

    /// Get all registered tools
    pub fn all_tools(&self) -> impl Iterator<Item = &ToolDef> {
        self.tools.iter().filter(|t| t.id != 0)
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::simulation::{MaterialDef, MaterialTag, MaterialType};

    fn make_test_material(hardness_mult: f32, tags: Vec<MaterialTag>) -> MaterialDef {
        MaterialDef {
            id: 1,
            name: "Test".to_string(),
            material_type: MaterialType::Solid,
            color: [0, 0, 0, 255],
            density: 1.0,
            hardness: Some(5),
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

    #[test]
    fn test_tool_tier_speed() {
        assert_eq!(ToolTier::Wood.speed_multiplier(), 1.0);
        assert_eq!(ToolTier::Stone.speed_multiplier(), 1.5);
        assert_eq!(ToolTier::Iron.speed_multiplier(), 3.0);
    }

    #[test]
    fn test_tool_tier_durability() {
        assert_eq!(ToolTier::Wood.max_durability(), 50);
        assert_eq!(ToolTier::Stone.max_durability(), 100);
        assert_eq!(ToolTier::Iron.max_durability(), 400);
    }

    #[test]
    fn test_tool_speed_calculations() {
        let wood_pick = ToolDef {
            id: 1000,
            name: "Wood Pickaxe".to_string(),
            tool_type: ToolType::Pickaxe,
            tier: ToolTier::Wood,
            can_harvest: vec![MaterialTag::Mineral, MaterialTag::Ore],
        };

        let iron_pick = ToolDef {
            id: 1002,
            name: "Iron Pickaxe".to_string(),
            tool_type: ToolType::Pickaxe,
            tier: ToolTier::Iron,
            can_harvest: vec![MaterialTag::Mineral, MaterialTag::Ore],
        };

        let stone = make_test_material(1.0, vec![MaterialTag::Mineral]);
        let iron_ore = make_test_material(2.0, vec![MaterialTag::Ore]);
        let wood = make_test_material(0.8, vec![MaterialTag::Organic]);

        // Wood pickaxe on stone (1.0 speed / 1.0 hardness = 1.0)
        assert_eq!(wood_pick.get_mining_speed(&stone), 1.0);

        // Wood pickaxe on iron ore (1.0 speed / 2.0 hardness = 0.5)
        assert_eq!(wood_pick.get_mining_speed(&iron_ore), 0.5);

        // Iron pickaxe on iron ore (3.0 speed / 2.0 hardness = 1.5)
        assert_eq!(iron_pick.get_mining_speed(&iron_ore), 1.5);

        // Wrong tool (pickaxe on wood, no Mineral/Ore tag)
        assert_eq!(wood_pick.get_mining_speed(&wood), 0.1);
    }

    #[test]
    fn test_tool_registry() {
        let registry = ToolRegistry::new();

        // Should have 3 pickaxes registered
        assert_eq!(registry.all_tools().count(), 3);

        // Get wood pickaxe
        let wood_pick = registry.get(1000);
        assert!(wood_pick.is_some());
        assert_eq!(wood_pick.unwrap().name, "Wood Pickaxe");

        // Get stone pickaxe
        let stone_pick = registry.get(1001);
        assert!(stone_pick.is_some());
        assert_eq!(stone_pick.unwrap().tier, ToolTier::Stone);

        // Get iron pickaxe
        let iron_pick = registry.get(1002);
        assert!(iron_pick.is_some());
        assert_eq!(iron_pick.unwrap().tier, ToolTier::Iron);

        // Invalid ID
        assert!(registry.get(999).is_none());
        assert!(registry.get(2000).is_none());
    }
}
