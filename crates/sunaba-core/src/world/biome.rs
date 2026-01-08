//! Biome system for diverse world generation

use crate::simulation::MaterialId;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Types of biomes in the world
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BiomeType {
    // Surface biomes (y > -100)
    Desert,
    Plains,
    Forest,
    Mountains,
    Ocean,
    // Future biomes (for Phase 4)
    // Tundra,
    // Jungle,
    // Volcanic,
    // Beach,
}

/// Defines the characteristics of a biome
#[derive(Debug, Clone)]
pub struct BiomeDefinition {
    pub name: &'static str,
    pub biome_type: BiomeType,

    // Surface generation
    pub surface_material: u16,    // Top layer (grass, sand, snow)
    pub subsurface_material: u16, // Below surface (dirt, sandstone, ice)
    pub stone_depth: i32,         // How many blocks deep before hitting stone

    // Terrain shape
    pub height_variance: f32, // Hilliness (0.0-1.0), multiplier for terrain height
    pub height_offset: i32,   // Base elevation offset from sea level

    // Vegetation
    pub tree_density: f32,  // 0.0-1.0 probability of tree generation
    pub plant_density: f32, // 0.0-1.0 probability of plant generation

    // Underground features
    pub cave_density_multiplier: f32, // Multiplier for cave frequency

    // Ore abundance multipliers (1.0 = normal)
    pub ore_multipliers: HashMap<u16, f32>,
}

impl BiomeDefinition {
    /// Create a Desert biome definition
    pub fn desert() -> Self {
        let mut ore_multipliers = HashMap::new();
        ore_multipliers.insert(MaterialId::GOLD_ORE, 1.5); // More gold in deserts
        ore_multipliers.insert(MaterialId::COAL_ORE, 0.5); // Less coal

        Self {
            name: "Desert",
            biome_type: BiomeType::Desert,
            surface_material: MaterialId::SAND,
            subsurface_material: MaterialId::SAND, // Sandy all the way down
            stone_depth: 15,                       // Relatively shallow stone
            height_variance: 0.3,                  // Gentle dunes
            height_offset: 5,                      // Slightly elevated
            tree_density: 0.0,                     // No trees
            plant_density: 0.05,                   // Sparse vegetation
            cave_density_multiplier: 1.2,          // More caves (dry climate)
            ore_multipliers,
        }
    }

    /// Create a Forest biome definition
    pub fn forest() -> Self {
        Self {
            name: "Forest",
            biome_type: BiomeType::Forest,
            surface_material: MaterialId::DIRT,
            subsurface_material: MaterialId::DIRT,
            stone_depth: 25,                 // Deep soil layer
            height_variance: 0.6,            // Rolling hills
            height_offset: 0,                // Sea level
            tree_density: 0.15,              // Dense trees
            plant_density: 0.4,              // Lots of plants
            cave_density_multiplier: 1.0,    // Normal caves
            ore_multipliers: HashMap::new(), // Normal ore distribution
        }
    }

    /// Create a Plains biome definition
    pub fn plains() -> Self {
        let mut ore_multipliers = HashMap::new();
        ore_multipliers.insert(MaterialId::COAL_ORE, 1.2); // Slightly more coal

        Self {
            name: "Plains",
            biome_type: BiomeType::Plains,
            surface_material: MaterialId::DIRT,
            subsurface_material: MaterialId::DIRT,
            stone_depth: 20,
            height_variance: 0.4, // Gentle rolling
            height_offset: 0,
            tree_density: 0.05, // Scattered trees
            plant_density: 0.3, // Moderate vegetation
            cave_density_multiplier: 1.0,
            ore_multipliers,
        }
    }

    /// Create a Mountains biome definition
    pub fn mountains() -> Self {
        let mut ore_multipliers = HashMap::new();
        ore_multipliers.insert(MaterialId::IRON_ORE, 1.5); // More iron
        ore_multipliers.insert(MaterialId::COPPER_ORE, 1.3); // More copper

        Self {
            name: "Mountains",
            biome_type: BiomeType::Mountains,
            surface_material: MaterialId::STONE, // Rocky surface
            subsurface_material: MaterialId::STONE,
            stone_depth: 5,       // Stone immediately
            height_variance: 1.2, // Very tall peaks
            height_offset: 30,    // Elevated baseline
            tree_density: 0.02,   // Sparse high-altitude vegetation
            plant_density: 0.1,
            cave_density_multiplier: 0.8, // Fewer caves (solid rock)
            ore_multipliers,
        }
    }

    /// Create an Ocean biome definition
    pub fn ocean() -> Self {
        Self {
            name: "Ocean",
            biome_type: BiomeType::Ocean,
            surface_material: MaterialId::WATER, // Water surface
            subsurface_material: MaterialId::SAND, // Sandy ocean floor
            stone_depth: 30,
            height_variance: 0.2,         // Gentle ocean floor
            height_offset: -40,           // Below sea level
            tree_density: 0.0,            // No trees underwater
            plant_density: 0.0,           // No plants (for now)
            cave_density_multiplier: 0.5, // Fewer caves underwater
            ore_multipliers: HashMap::new(),
        }
    }

    /// Get ore multiplier for a specific ore type (1.0 if not specified)
    pub fn get_ore_multiplier(&self, ore_material: u16) -> f32 {
        *self.ore_multipliers.get(&ore_material).unwrap_or(&1.0)
    }
}

/// Registry of all biome definitions
pub struct BiomeRegistry {
    biomes: HashMap<BiomeType, BiomeDefinition>,
}

impl Default for BiomeRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl BiomeRegistry {
    pub fn new() -> Self {
        let mut biomes = HashMap::new();
        biomes.insert(BiomeType::Desert, BiomeDefinition::desert());
        biomes.insert(BiomeType::Forest, BiomeDefinition::forest());
        biomes.insert(BiomeType::Plains, BiomeDefinition::plains());
        biomes.insert(BiomeType::Mountains, BiomeDefinition::mountains());
        biomes.insert(BiomeType::Ocean, BiomeDefinition::ocean());

        Self { biomes }
    }

    pub fn get(&self, biome_type: BiomeType) -> &BiomeDefinition {
        self.biomes
            .get(&biome_type)
            .expect("Biome definition not found")
    }
}

/// Select biome based on temperature and moisture values (-1.0 to 1.0)
pub fn select_biome(temperature: f64, moisture: f64) -> BiomeType {
    // Temperature ranges: -1.0 to 1.0
    // Moisture ranges: -1.0 to 1.0

    // Ocean: very low temperature and moisture combination indicates deep water
    if temperature < -0.3 && moisture < -0.3 {
        return BiomeType::Ocean;
    }

    // Mountains: high temperature (represents elevation, not heat)
    if temperature > 0.5 {
        return BiomeType::Mountains;
    }

    // Desert: hot and dry
    if temperature > 0.0 && moisture < -0.2 {
        return BiomeType::Desert;
    }

    // Forest: moderate temp, high moisture
    if moisture > 0.3 {
        return BiomeType::Forest;
    }

    // Default: Plains (moderate conditions)
    BiomeType::Plains
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_biome_selection() {
        // Test extreme cases
        assert_eq!(
            select_biome(0.8, 0.0),
            BiomeType::Mountains,
            "High temperature should give mountains"
        );
        assert_eq!(
            select_biome(-0.5, -0.5),
            BiomeType::Ocean,
            "Low temp + moisture should give ocean"
        );
        assert_eq!(
            select_biome(0.3, -0.5),
            BiomeType::Desert,
            "Hot and dry should give desert"
        );
        assert_eq!(
            select_biome(0.0, 0.8),
            BiomeType::Forest,
            "Moderate temp + high moisture should give forest"
        );
        assert_eq!(
            select_biome(0.0, 0.0),
            BiomeType::Plains,
            "Moderate conditions should give plains"
        );
    }

    #[test]
    fn test_ore_multipliers() {
        let desert = BiomeDefinition::desert();
        assert_eq!(desert.get_ore_multiplier(MaterialId::GOLD_ORE), 1.5);
        assert_eq!(desert.get_ore_multiplier(MaterialId::IRON_ORE), 1.0); // Default

        let mountains = BiomeDefinition::mountains();
        assert_eq!(mountains.get_ore_multiplier(MaterialId::IRON_ORE), 1.5);
    }

    #[test]
    fn test_biome_registry() {
        let registry = BiomeRegistry::new();
        let forest = registry.get(BiomeType::Forest);
        assert_eq!(forest.name, "Forest");
        assert_eq!(forest.surface_material, MaterialId::DIRT);
    }
}
