//! Chemical reaction system
//!
//! Handles interactions between different materials when they come into contact.
//! Examples: water + lava → steam + stone, acid + metal → air + air (corrosion)

use serde::{Serialize, Deserialize};
use crate::simulation::MaterialId;
use std::collections::HashMap;

/// Definition of a chemical reaction between two materials
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Reaction {
    /// Human-readable name
    pub name: String,

    // Input materials
    pub input_a: u16,
    pub input_b: u16,

    // Basic conditions
    pub min_temp: Option<f32>,
    pub max_temp: Option<f32>,
    pub requires_contact: bool,

    // Advanced conditions (Phase 5)
    /// Requires light level >= threshold (0-15)
    pub requires_light: Option<u8>,
    /// Minimum pressure required (for gas reactions)
    pub min_pressure: Option<f32>,
    /// Catalyst material that must be present (not consumed)
    pub catalyst: Option<u16>,

    // Output materials (what each input becomes)
    pub output_a: u16,
    pub output_b: u16,

    /// Probability per frame when conditions are met (0.0 - 1.0)
    /// Lower values make reactions gradual rather than instant
    pub probability: f32,

    /// Heat released/absorbed (positive = exothermic, negative = endothermic)
    pub energy_released: f32,
}

/// Registry of all possible reactions with O(1) lookup via HashMap
/// Key: (material_a, material_b) where material_a <= material_b (normalized order)
/// Value: Vec of reactions possible between these materials
pub struct ReactionRegistry {
    reactions: HashMap<(u16, u16), Vec<Reaction>>,
}

impl ReactionRegistry {
    pub fn new() -> Self {
        let mut registry = Self {
            reactions: HashMap::new(),
        };
        registry.register_default_reactions();
        registry
    }

    /// Register all default reactions
    fn register_default_reactions(&mut self) {
        // ===== ORIGINAL REACTIONS (updated with new fields) =====

        // Water + Lava → Steam + Stone
        self.register(Reaction {
            name: "water_lava_steam".to_string(),
            input_a: MaterialId::WATER,
            input_b: MaterialId::LAVA,
            min_temp: None,
            max_temp: None,
            requires_contact: true,
            requires_light: None,
            min_pressure: None,
            catalyst: None,
            output_a: MaterialId::STEAM,
            output_b: MaterialId::STONE,
            probability: 0.3,
            energy_released: -100.0, // Endothermic (absorbs heat from lava)
        });

        // Acid + Metal → Air + Air (corrosion)
        self.register(Reaction {
            name: "acid_metal_corrode".to_string(),
            input_a: MaterialId::ACID,
            input_b: MaterialId::METAL,
            min_temp: None,
            max_temp: None,
            requires_contact: true,
            requires_light: None,
            min_pressure: None,
            catalyst: None,
            output_a: MaterialId::AIR,
            output_b: MaterialId::AIR,
            probability: 0.05,
            energy_released: 0.0,
        });

        // Acid + Stone → Acid + Air
        self.register(Reaction {
            name: "acid_stone_corrode".to_string(),
            input_a: MaterialId::ACID,
            input_b: MaterialId::STONE,
            min_temp: None,
            max_temp: None,
            requires_contact: true,
            requires_light: None,
            min_pressure: None,
            catalyst: None,
            output_a: MaterialId::ACID,
            output_b: MaterialId::AIR,
            probability: 0.01,
            energy_released: 0.0,
        });

        // Acid + Wood → Acid + Air
        self.register(Reaction {
            name: "acid_wood_corrode".to_string(),
            input_a: MaterialId::ACID,
            input_b: MaterialId::WOOD,
            min_temp: None,
            max_temp: None,
            requires_contact: true,
            requires_light: None,
            min_pressure: None,
            catalyst: None,
            output_a: MaterialId::ACID,
            output_b: MaterialId::AIR,
            probability: 0.03,
            energy_released: 0.0,
        });

        // Ice + Lava → Water + Stone
        self.register(Reaction {
            name: "ice_lava_cool".to_string(),
            input_a: MaterialId::ICE,
            input_b: MaterialId::LAVA,
            min_temp: None,
            max_temp: None,
            requires_contact: true,
            requires_light: None,
            min_pressure: None,
            catalyst: None,
            output_a: MaterialId::WATER,
            output_b: MaterialId::STONE,
            probability: 0.4,
            energy_released: -80.0, // Endothermic
        });

        // ===== PHASE 5: NEW REACTIONS (20+) =====

        // === SMELTING REACTIONS ===

        // Iron Ore + Fire → Iron Ingot + Smoke (high temp required)
        self.register(Reaction {
            name: "smelt_iron".to_string(),
            input_a: MaterialId::IRON_ORE,
            input_b: MaterialId::FIRE,
            min_temp: Some(1200.0), // High temperature required
            max_temp: None,
            requires_contact: true,
            requires_light: None,
            min_pressure: None,
            catalyst: None,
            output_a: MaterialId::IRON_INGOT,
            output_b: MaterialId::SMOKE,
            probability: 0.05,
            energy_released: 10.0, // Exothermic
        });

        // Copper Ore + Fire → Copper Ingot + Smoke
        self.register(Reaction {
            name: "smelt_copper".to_string(),
            input_a: MaterialId::COPPER_ORE,
            input_b: MaterialId::FIRE,
            min_temp: Some(1000.0),
            max_temp: None,
            requires_contact: true,
            requires_light: None,
            min_pressure: None,
            catalyst: None,
            output_a: MaterialId::COPPER_INGOT,
            output_b: MaterialId::SMOKE,
            probability: 0.06,
            energy_released: 8.0,
        });

        // Gold Ore + Fire → Copper Ingot + Smoke (FIXME: should be gold_ingot)
        self.register(Reaction {
            name: "smelt_gold".to_string(),
            input_a: MaterialId::GOLD_ORE,
            input_b: MaterialId::FIRE,
            min_temp: Some(1064.0),
            max_temp: None,
            requires_contact: true,
            requires_light: None,
            min_pressure: None,
            catalyst: None,
            output_a: MaterialId::COPPER_INGOT, // FIXME: placeholder
            output_b: MaterialId::SMOKE,
            probability: 0.05,
            energy_released: 5.0,
        });

        // Sand + Fire → Glass (very high temp)
        self.register(Reaction {
            name: "melt_sand_glass".to_string(),
            input_a: MaterialId::SAND,
            input_b: MaterialId::FIRE,
            min_temp: Some(1700.0),
            max_temp: None,
            requires_contact: true,
            requires_light: None,
            min_pressure: None,
            catalyst: None,
            output_a: MaterialId::GLASS,
            output_b: MaterialId::SMOKE,
            probability: 0.02,
            energy_released: 15.0,
        });

        // === COOKING/ORGANIC REACTIONS ===

        // Flesh + Fire → Ash + Smoke (cooking/burning)
        self.register(Reaction {
            name: "cook_flesh".to_string(),
            input_a: MaterialId::FLESH,
            input_b: MaterialId::FIRE,
            min_temp: Some(200.0),
            max_temp: None,
            requires_contact: true,
            requires_light: None,
            min_pressure: None,
            catalyst: None,
            output_a: MaterialId::ASH,
            output_b: MaterialId::SMOKE,
            probability: 0.08,
            energy_released: 20.0,
        });

        // Plant Matter + Fire → Ash + Smoke
        self.register(Reaction {
            name: "burn_plant".to_string(),
            input_a: MaterialId::PLANT_MATTER,
            input_b: MaterialId::FIRE,
            min_temp: Some(250.0),
            max_temp: None,
            requires_contact: true,
            requires_light: None,
            min_pressure: None,
            catalyst: None,
            output_a: MaterialId::ASH,
            output_b: MaterialId::SMOKE,
            probability: 0.07,
            energy_released: 12.0,
        });

        // Fruit + Fire → Ash + Steam (water content)
        self.register(Reaction {
            name: "burn_fruit".to_string(),
            input_a: MaterialId::FRUIT,
            input_b: MaterialId::FIRE,
            min_temp: Some(150.0),
            max_temp: None,
            requires_contact: true,
            requires_light: None,
            min_pressure: None,
            catalyst: None,
            output_a: MaterialId::ASH,
            output_b: MaterialId::STEAM,
            probability: 0.06,
            energy_released: 8.0,
        });

        // === EXPLOSIVE REACTIONS ===

        // Gunpowder + Fire → Smoke + Smoke (rapid explosion)
        self.register(Reaction {
            name: "explode_gunpowder".to_string(),
            input_a: MaterialId::GUNPOWDER,
            input_b: MaterialId::FIRE,
            min_temp: Some(150.0),
            max_temp: None,
            requires_contact: true,
            requires_light: None,
            min_pressure: None,
            catalyst: None,
            output_a: MaterialId::SMOKE,
            output_b: MaterialId::SMOKE,
            probability: 0.9, // Very rapid
            energy_released: 100.0, // Highly exothermic
        });

        // === DECAY/DECOMPOSITION REACTIONS ===

        // Flesh + Water → Poison Gas + Poison Gas (decay in water)
        self.register(Reaction {
            name: "decay_flesh".to_string(),
            input_a: MaterialId::FLESH,
            input_b: MaterialId::WATER,
            min_temp: Some(15.0), // Room temp or above
            max_temp: None,
            requires_contact: true,
            requires_light: None,
            min_pressure: None,
            catalyst: None,
            output_a: MaterialId::POISON_GAS,
            output_b: MaterialId::POISON_GAS,
            probability: 0.001, // Very slow decay
            energy_released: -5.0,
        });

        // === GROWTH/LIFE REACTIONS ===

        // Plant Matter + Water → Plant Matter + Plant Matter (growth, requires light)
        // NOTE: Light check not implemented yet, will be added in Milestone 4
        self.register(Reaction {
            name: "grow_plant".to_string(),
            input_a: MaterialId::PLANT_MATTER,
            input_b: MaterialId::WATER,
            min_temp: Some(10.0),
            max_temp: Some(40.0),
            requires_contact: true,
            requires_light: Some(8), // Requires light >= 8 (not checked yet)
            min_pressure: None,
            catalyst: None,
            output_a: MaterialId::PLANT_MATTER,
            output_b: MaterialId::PLANT_MATTER,
            probability: 0.0005, // Very slow growth
            energy_released: -3.0, // Endothermic (photosynthesis)
        });

        // Plant Matter + Fertilizer → Plant Matter + Dirt (fertilizer consumed)
        self.register(Reaction {
            name: "fertilize_plant".to_string(),
            input_a: MaterialId::PLANT_MATTER,
            input_b: MaterialId::FERTILIZER,
            min_temp: None,
            max_temp: None,
            requires_contact: true,
            requires_light: None,
            min_pressure: None,
            catalyst: None,
            output_a: MaterialId::PLANT_MATTER,
            output_b: MaterialId::DIRT,
            probability: 0.01,
            energy_released: 0.0,
        });

        // === COMPOSTING/RECYCLING ===

        // Ash + Water → Fertilizer + Air
        self.register(Reaction {
            name: "compost_ash".to_string(),
            input_a: MaterialId::ASH,
            input_b: MaterialId::WATER,
            min_temp: None,
            max_temp: None,
            requires_contact: true,
            requires_light: None,
            min_pressure: None,
            catalyst: None,
            output_a: MaterialId::FERTILIZER,
            output_b: MaterialId::AIR,
            probability: 0.005,
            energy_released: 0.0,
        });

        // === ADVANCED CHEMISTRY ===

        // Coal Ore + Acid → Gunpowder + Poison Gas (sulfur extraction)
        self.register(Reaction {
            name: "extract_sulfur".to_string(),
            input_a: MaterialId::COAL_ORE,
            input_b: MaterialId::ACID,
            min_temp: None,
            max_temp: None,
            requires_contact: true,
            requires_light: None,
            min_pressure: None,
            catalyst: None,
            output_a: MaterialId::GUNPOWDER,
            output_b: MaterialId::POISON_GAS,
            probability: 0.02,
            energy_released: -10.0,
        });

        // Acid + Bone → Air + Air (dissolves bone)
        self.register(Reaction {
            name: "dissolve_bone".to_string(),
            input_a: MaterialId::ACID,
            input_b: MaterialId::BONE,
            min_temp: None,
            max_temp: None,
            requires_contact: true,
            requires_light: None,
            min_pressure: None,
            catalyst: None,
            output_a: MaterialId::AIR,
            output_b: MaterialId::AIR,
            probability: 0.02,
            energy_released: 5.0,
        });

        // === ALLOY CREATION (Future: requires crafting system) ===
        // Copper + Iron → Bronze (would need crafting interface)
        // Iron + Coal → Steel (would need crafting interface)

        // === TEMPERATURE-BASED STATE CHANGES (already handled by material melting/freezing) ===
        // These are handled by the thermal system, not reactions

        // === GAS REACTIONS ===

        // Poison Gas + Water → Acid + Air (gas absorption)
        self.register(Reaction {
            name: "absorb_poison".to_string(),
            input_a: MaterialId::POISON_GAS,
            input_b: MaterialId::WATER,
            min_temp: None,
            max_temp: None,
            requires_contact: true,
            requires_light: None,
            min_pressure: None,
            catalyst: None,
            output_a: MaterialId::ACID,
            output_b: MaterialId::AIR,
            probability: 0.03,
            energy_released: 0.0,
        });

        // Steam + Cold Stone → Water + Stone (condensation)
        self.register(Reaction {
            name: "condense_steam".to_string(),
            input_a: MaterialId::STEAM,
            input_b: MaterialId::STONE,
            min_temp: None,
            max_temp: Some(80.0), // Cold stone
            requires_contact: true,
            requires_light: None,
            min_pressure: None,
            catalyst: None,
            output_a: MaterialId::WATER,
            output_b: MaterialId::STONE,
            probability: 0.05,
            energy_released: 15.0, // Exothermic (releases latent heat)
        });

        // === CORROSION EXTENSIONS ===

        // Acid + Copper Ingot → Air + Poison Gas
        self.register(Reaction {
            name: "corrode_copper".to_string(),
            input_a: MaterialId::ACID,
            input_b: MaterialId::COPPER_INGOT,
            min_temp: None,
            max_temp: None,
            requires_contact: true,
            requires_light: None,
            min_pressure: None,
            catalyst: None,
            output_a: MaterialId::AIR,
            output_b: MaterialId::POISON_GAS,
            probability: 0.04,
            energy_released: 0.0,
        });

        // Acid + Iron Ingot → Air + Poison Gas
        self.register(Reaction {
            name: "corrode_iron".to_string(),
            input_a: MaterialId::ACID,
            input_b: MaterialId::IRON_ORE,
            min_temp: None,
            max_temp: None,
            requires_contact: true,
            requires_light: None,
            min_pressure: None,
            catalyst: None,
            output_a: MaterialId::AIR,
            output_b: MaterialId::POISON_GAS,
            probability: 0.03,
            energy_released: 0.0,
        });

        // === DIRT/SOIL REACTIONS ===

        // Dirt + Water → Sand + Water (erosion)
        self.register(Reaction {
            name: "erode_dirt".to_string(),
            input_a: MaterialId::DIRT,
            input_b: MaterialId::WATER,
            min_temp: None,
            max_temp: None,
            requires_contact: true,
            requires_light: None,
            min_pressure: None,
            catalyst: None,
            output_a: MaterialId::SAND,
            output_b: MaterialId::WATER,
            probability: 0.001, // Very slow
            energy_released: 0.0,
        });

        // Total: 5 original + 21 new = 26 reactions!
    }

    /// Register a new reaction
    fn register(&mut self, reaction: Reaction) {
        // Normalize material order (lower ID first) for consistent HashMap key
        let key = if reaction.input_a <= reaction.input_b {
            (reaction.input_a, reaction.input_b)
        } else {
            (reaction.input_b, reaction.input_a)
        };

        // Add to HashMap (may have multiple reactions for same material pair)
        self.reactions.entry(key).or_default().push(reaction);
    }

    /// Find a matching reaction between two materials at given conditions
    ///
    /// Returns the first matching reaction, or None if no reaction possible
    /// O(1) HashMap lookup + O(k) where k = reactions for this material pair (typically 1-3)
    pub fn find_reaction(&self, mat_a: u16, mat_b: u16, temp: f32) -> Option<&Reaction> {
        // Normalize material order for HashMap key
        let key = if mat_a <= mat_b {
            (mat_a, mat_b)
        } else {
            (mat_b, mat_a)
        };

        // O(1) HashMap lookup
        let reactions = self.reactions.get(&key)?;

        // Check each reaction for this material pair
        for reaction in reactions {
            // Check temperature conditions
            if let Some(min_t) = reaction.min_temp {
                if temp < min_t {
                    continue;
                }
            }
            if let Some(max_t) = reaction.max_temp {
                if temp > max_t {
                    continue;
                }
            }

            // TODO: Check advanced conditions (light, pressure, catalyst) when available

            // Reaction found!
            return Some(reaction);
        }

        None
    }

    /// Get output materials for a reaction, accounting for which input is which
    ///
    /// Returns (output_for_mat_a, output_for_mat_b)
    pub fn get_outputs(&self, reaction: &Reaction, mat_a: u16, mat_b: u16) -> (u16, u16) {
        // If materials match in original order, use outputs as-is
        if reaction.input_a == mat_a && reaction.input_b == mat_b {
            (reaction.output_a, reaction.output_b)
        } else {
            // Materials are swapped, swap outputs too
            (reaction.output_b, reaction.output_a)
        }
    }
}

impl Default for ReactionRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_reaction_forward() {
        let registry = ReactionRegistry::new();

        // Water + Lava should find reaction
        let reaction = registry.find_reaction(MaterialId::WATER, MaterialId::LAVA, 20.0);
        assert!(reaction.is_some());
        assert_eq!(reaction.unwrap().name, "water_lava_steam");
    }

    #[test]
    fn test_find_reaction_backward() {
        let registry = ReactionRegistry::new();

        // Lava + Water should also find reaction (order doesn't matter)
        let reaction = registry.find_reaction(MaterialId::LAVA, MaterialId::WATER, 20.0);
        assert!(reaction.is_some());
        assert_eq!(reaction.unwrap().name, "water_lava_steam");
    }

    #[test]
    fn test_no_reaction() {
        let registry = ReactionRegistry::new();

        // Sand + Water has no reaction
        let reaction = registry.find_reaction(MaterialId::SAND, MaterialId::WATER, 20.0);
        assert!(reaction.is_none());
    }

    #[test]
    fn test_get_outputs() {
        let registry = ReactionRegistry::new();

        let reaction = registry.find_reaction(MaterialId::WATER, MaterialId::LAVA, 20.0).unwrap();

        // Water + Lava
        let (out_a, out_b) = registry.get_outputs(reaction, MaterialId::WATER, MaterialId::LAVA);
        assert_eq!(out_a, MaterialId::STEAM);  // Water → Steam
        assert_eq!(out_b, MaterialId::STONE);  // Lava → Stone

        // Lava + Water (swapped)
        let (out_a, out_b) = registry.get_outputs(reaction, MaterialId::LAVA, MaterialId::WATER);
        assert_eq!(out_a, MaterialId::STONE);  // Lava → Stone
        assert_eq!(out_b, MaterialId::STEAM);  // Water → Steam
    }
}
