//! Training terrain configuration system
//!
//! Provides high-level difficulty parameters that map to WorldGenConfig for procedural terrain generation.

use serde::{Deserialize, Serialize};
use sunaba_core::world::worldgen_config::WorldGenConfig;

/// Configuration for procedural training terrain generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingTerrainConfig {
    /// Base seed for deterministic generation
    pub base_seed: u64,

    /// World dimensions (pixels)
    pub width: i32,
    pub height: i32,

    /// Difficulty parameters
    pub difficulty: DifficultyConfig,

    /// Underlying WorldGenConfig (can be modified by difficulty)
    pub worldgen_config: WorldGenConfig,
}

/// Parameterized difficulty settings (0.0 = easiest, 1.0 = hardest)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DifficultyConfig {
    /// Terrain roughness (0.0 = flat, 1.0 = max variance)
    /// Maps to WorldGenConfig.terrain.height_scale
    pub terrain_roughness: f32,

    /// Obstacle density (0.0 = none, 1.0 = frequent)
    /// Maps to structure placement density
    pub obstacle_density: f32,

    /// Hazard density (0.0 = none, 1.0 = dangerous)
    /// Maps to lava pool frequency
    pub hazard_density: f32,

    /// Cave density (0.0 = solid, 1.0 = max caves)
    /// Maps to cave noise thresholds
    pub cave_density: f32,

    /// Gap frequency (0.0 = continuous, 1.0 = frequent gaps)
    /// Controls how often to place air gaps in terrain
    pub gap_frequency: f32,
}

impl Default for DifficultyConfig {
    fn default() -> Self {
        Self::flat()
    }
}

impl DifficultyConfig {
    /// Stage 1: Completely flat terrain
    pub fn flat() -> Self {
        Self {
            terrain_roughness: 0.0,
            obstacle_density: 0.0,
            hazard_density: 0.0,
            cave_density: 0.0,
            gap_frequency: 0.0,
        }
    }

    /// Stage 2: Gentle hills with minimal obstacles
    pub fn gentle_hills() -> Self {
        Self {
            terrain_roughness: 0.3,
            obstacle_density: 0.1,
            hazard_density: 0.0,
            cave_density: 0.1,
            gap_frequency: 0.05,
        }
    }

    /// Stage 3: Moderate terrain with obstacles
    pub fn obstacles() -> Self {
        Self {
            terrain_roughness: 0.5,
            obstacle_density: 0.5,
            hazard_density: 0.1,
            cave_density: 0.3,
            gap_frequency: 0.2,
        }
    }

    /// Stage 4: Challenging terrain with hazards
    pub fn hazards() -> Self {
        Self {
            terrain_roughness: 0.7,
            obstacle_density: 0.4,
            hazard_density: 0.4,
            cave_density: 0.4,
            gap_frequency: 0.3,
        }
    }

    /// Stage 5: Full random terrain
    pub fn random() -> Self {
        Self {
            terrain_roughness: 1.0,
            obstacle_density: 0.6,
            hazard_density: 0.3,
            cave_density: 0.5,
            gap_frequency: 0.4,
        }
    }

    /// Interpolate between two difficulty configs
    pub fn lerp(&self, other: &Self, t: f32) -> Self {
        let t = t.clamp(0.0, 1.0);
        Self {
            terrain_roughness: self.terrain_roughness
                + (other.terrain_roughness - self.terrain_roughness) * t,
            obstacle_density: self.obstacle_density
                + (other.obstacle_density - self.obstacle_density) * t,
            hazard_density: self.hazard_density + (other.hazard_density - self.hazard_density) * t,
            cave_density: self.cave_density + (other.cave_density - self.cave_density) * t,
            gap_frequency: self.gap_frequency + (other.gap_frequency - self.gap_frequency) * t,
        }
    }
}

impl TrainingTerrainConfig {
    /// Create a flat training terrain (Stage 1: basic locomotion)
    pub fn flat(seed: u64, width: i32, height: i32) -> Self {
        Self {
            base_seed: seed,
            width,
            height,
            difficulty: DifficultyConfig::flat(),
            worldgen_config: WorldGenConfig::default(),
        }
    }

    /// Create gentle hills terrain (Stage 2: slope handling)
    pub fn gentle_hills(seed: u64, width: i32, height: i32) -> Self {
        Self {
            base_seed: seed,
            width,
            height,
            difficulty: DifficultyConfig::gentle_hills(),
            worldgen_config: WorldGenConfig::default(),
        }
    }

    /// Create obstacle course terrain (Stage 3: navigation)
    pub fn obstacles(seed: u64, width: i32, height: i32) -> Self {
        Self {
            base_seed: seed,
            width,
            height,
            difficulty: DifficultyConfig::obstacles(),
            worldgen_config: WorldGenConfig::default(),
        }
    }

    /// Create hazardous terrain (Stage 4: survival)
    pub fn hazards(seed: u64, width: i32, height: i32) -> Self {
        Self {
            base_seed: seed,
            width,
            height,
            difficulty: DifficultyConfig::hazards(),
            worldgen_config: WorldGenConfig::default(),
        }
    }

    /// Create fully random terrain (Stage 5: generalization)
    pub fn random(seed: u64, width: i32, height: i32) -> Self {
        Self {
            base_seed: seed,
            width,
            height,
            difficulty: DifficultyConfig::random(),
            worldgen_config: WorldGenConfig::default(),
        }
    }

    /// Apply difficulty parameters to create a WorldGenConfig
    ///
    /// Maps high-level difficulty settings to low-level worldgen parameters.
    /// This allows training scenarios to use intuitive difficulty knobs
    /// without needing to understand all the WorldGen parameters.
    pub fn apply_difficulty(&self) -> WorldGenConfig {
        let mut config = self.worldgen_config.clone();

        // Apply terrain roughness to height scale
        // flat (0.0) → height_scale = 0 (completely flat)
        // max (1.0) → height_scale = 100 (default varied terrain)
        config.terrain.height_scale = self.difficulty.terrain_roughness * 100.0;

        // Apply cave density to thresholds
        // 0.0 = no caves (threshold = 1.0, nothing passes)
        // 1.0 = max caves (threshold = 0.3, lots passes)
        let cave_threshold = 1.0 - (self.difficulty.cave_density * 0.7);
        config.caves.large_threshold = cave_threshold;
        config.caves.tunnel_threshold = cave_threshold + 0.1;

        // Apply hazard density to feature frequency
        // This will control lava pool generation (feature system)
        // For now, store in a comment - will be used when features are integrated
        // TODO: Add hazard_density to FeatureParams when implementing hazard generation

        // Apply obstacle density to structure placement
        // This will be used by the structure system
        // TODO: Add obstacle_density to StructureConfig when implementing obstacle placement

        // Gap frequency doesn't map directly to WorldGenConfig
        // It will be handled procedurally during world setup
        // by occasionally removing sections of terrain

        config
    }
}

impl Default for TrainingTerrainConfig {
    fn default() -> Self {
        Self::flat(42, 512, 128)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_difficulty_config_presets() {
        let flat = DifficultyConfig::flat();
        assert_eq!(flat.terrain_roughness, 0.0);
        assert_eq!(flat.obstacle_density, 0.0);

        let hills = DifficultyConfig::gentle_hills();
        assert!(hills.terrain_roughness > 0.0);
        assert!(hills.terrain_roughness < 0.5);

        let obstacles = DifficultyConfig::obstacles();
        assert!(obstacles.obstacle_density > 0.0);

        let hazards = DifficultyConfig::hazards();
        assert!(hazards.hazard_density > 0.0);

        let random = DifficultyConfig::random();
        assert!(random.terrain_roughness > 0.5);
    }

    #[test]
    fn test_difficulty_lerp() {
        let flat = DifficultyConfig::flat();
        let random = DifficultyConfig::random();

        let mid = flat.lerp(&random, 0.5);
        assert!(mid.terrain_roughness > flat.terrain_roughness);
        assert!(mid.terrain_roughness < random.terrain_roughness);

        let same = flat.lerp(&random, 0.0);
        assert_eq!(same.terrain_roughness, flat.terrain_roughness);

        let other = flat.lerp(&random, 1.0);
        assert_eq!(other.terrain_roughness, random.terrain_roughness);
    }

    #[test]
    fn test_training_terrain_config_presets() {
        let flat = TrainingTerrainConfig::flat(42, 512, 128);
        assert_eq!(flat.base_seed, 42);
        assert_eq!(flat.width, 512);
        assert_eq!(flat.height, 128);
        assert_eq!(flat.difficulty.terrain_roughness, 0.0);

        let hills = TrainingTerrainConfig::gentle_hills(100, 400, 100);
        assert_eq!(hills.base_seed, 100);
        assert!(hills.difficulty.terrain_roughness > 0.0);
    }

    #[test]
    fn test_apply_difficulty_flat() {
        let config = TrainingTerrainConfig::flat(42, 512, 128);
        let worldgen = config.apply_difficulty();

        // Flat terrain should have height_scale = 0.0
        assert_eq!(worldgen.terrain.height_scale, 0.0);

        // Should have minimal caves
        assert!(worldgen.caves.large_threshold > 0.9);
    }

    #[test]
    fn test_apply_difficulty_random() {
        let config = TrainingTerrainConfig::random(42, 512, 128);
        let worldgen = config.apply_difficulty();

        // Random terrain should have max height variance
        assert_eq!(worldgen.terrain.height_scale, 100.0);

        // Should have more caves than flat (cave_density=0.5 → threshold=0.65)
        assert!(worldgen.caves.large_threshold < 0.9);
        assert!(worldgen.caves.large_threshold > 0.0);
    }

    #[test]
    fn test_deterministic_config() {
        let config1 = TrainingTerrainConfig::flat(42, 512, 128);
        let config2 = TrainingTerrainConfig::flat(42, 512, 128);

        let worldgen1 = config1.apply_difficulty();
        let worldgen2 = config2.apply_difficulty();

        // Same input should produce same output
        assert_eq!(
            worldgen1.terrain.height_scale,
            worldgen2.terrain.height_scale
        );
        assert_eq!(
            worldgen1.caves.large_threshold,
            worldgen2.caves.large_threshold
        );
    }
}
