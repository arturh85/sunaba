use crate::simulation::MaterialId;
use crate::world::biome::{BiomeRegistry, BiomeType, select_biome};
use crate::world::chunk::{CHUNK_SIZE, Chunk};
use crate::world::worldgen_config::WorldGenConfig;
use fastnoise_lite::{FastNoiseLite, NoiseType};
use std::collections::HashMap;

// World dimension constants (defaults, can be overridden by config)
pub const SURFACE_Y: i32 = 0; // Sea level baseline
pub const SKY_HEIGHT: i32 = 1000; // Top of atmosphere
pub const BEDROCK_Y: i32 = -3500; // Bedrock layer starts here
pub const MAX_UNDERGROUND: i32 = -3500; // Bottom before bedrock

// Underground layer boundaries
pub const SHALLOW_UNDERGROUND: i32 = -500; // Common ores, shallow caves
pub const DEEP_UNDERGROUND: i32 = -1500; // Better ores, larger caves
pub const CAVERN_LAYER: i32 = -2500; // Rare ores, huge caverns, lava

/// World generator using multi-octave Perlin noise for biome-based generation
///
/// Can be created with default hardcoded parameters via `new(seed)` or
/// with a full configuration via `from_config(seed, config)`.
pub struct WorldGenerator {
    pub seed: u64,

    // Configuration (stored for update_config)
    config: WorldGenConfig,

    // Biome definitions (legacy, for backward compatibility)
    biome_registry: BiomeRegistry,

    // Large-scale (biome selection) - very low frequency
    temperature_noise: FastNoiseLite, // 2 octaves, freq=0.0003
    moisture_noise: FastNoiseLite,    // 2 octaves, freq=0.0003

    // Medium-scale (terrain features) - low frequency
    terrain_height_noise: FastNoiseLite, // 4 octaves, freq=0.001

    // Small-scale (cave details) - medium frequency
    cave_noise_large: FastNoiseLite, // 3 octaves, freq=0.005 (big caverns)
    cave_noise_small: FastNoiseLite, // 4 octaves, freq=0.008 (tunnels)

    // Per-ore noise layers (keyed by material ID for config-driven generation)
    ore_noises: HashMap<u16, FastNoiseLite>,

    // Legacy ore noise fields (for backward compatibility)
    coal_noise: FastNoiseLite,
    iron_noise: FastNoiseLite,
    copper_noise: FastNoiseLite,
    gold_noise: FastNoiseLite,

    // Vegetation placement
    tree_noise: FastNoiseLite,
    plant_noise: FastNoiseLite,
}

impl WorldGenerator {
    /// Create a new WorldGenerator with default configuration
    ///
    /// This maintains backward compatibility with existing code.
    pub fn new(seed: u64) -> Self {
        Self::from_config(seed, WorldGenConfig::default())
    }

    /// Create a WorldGenerator from a configuration
    ///
    /// This allows full control over generation parameters via the config.
    pub fn from_config(seed: u64, config: WorldGenConfig) -> Self {
        // Build all noise layers from config
        let temperature_noise = config.biomes.temperature_noise.to_fastnoise(seed);
        let moisture_noise = config.biomes.moisture_noise.to_fastnoise(seed);
        let terrain_height_noise = config.terrain.height_noise.to_fastnoise(seed);
        let cave_noise_large = config.caves.large_caves.to_fastnoise(seed);
        let cave_noise_small = config.caves.tunnels.to_fastnoise(seed);

        // Build ore noise map from config
        let mut ore_noises = HashMap::new();
        for ore_config in &config.ores {
            let noise = ore_config.noise.to_fastnoise(seed);
            ore_noises.insert(ore_config.material_id, noise);
        }

        // Helper to create ore noise from config or default
        let create_ore_noise = |material_id: u16, default_seed_offset: i32| -> FastNoiseLite {
            config
                .ores
                .iter()
                .find(|o| o.material_id == material_id)
                .map(|o| o.noise.to_fastnoise(seed))
                .unwrap_or_else(|| {
                    let mut noise = FastNoiseLite::with_seed((seed as i32) + default_seed_offset);
                    noise.set_noise_type(Some(NoiseType::OpenSimplex2));
                    noise
                })
        };

        // Legacy ore noise fields (for backward compatibility with existing code paths)
        let coal_noise = create_ore_noise(MaterialId::COAL_ORE, 5);
        let iron_noise = create_ore_noise(MaterialId::IRON_ORE, 6);
        let copper_noise = create_ore_noise(MaterialId::COPPER_ORE, 7);
        let gold_noise = create_ore_noise(MaterialId::GOLD_ORE, 8);

        // Vegetation noise
        let tree_noise = config.vegetation.tree_noise.to_fastnoise(seed);
        let plant_noise = config.vegetation.plant_noise.to_fastnoise(seed);

        Self {
            seed,
            config,
            biome_registry: BiomeRegistry::new(),
            temperature_noise,
            moisture_noise,
            terrain_height_noise,
            cave_noise_large,
            cave_noise_small,
            ore_noises,
            coal_noise,
            iron_noise,
            copper_noise,
            gold_noise,
            tree_noise,
            plant_noise,
        }
    }

    /// Update the configuration and rebuild noise layers
    ///
    /// Used for live preview in the editor. Maintains the same seed.
    pub fn update_config(&mut self, config: WorldGenConfig) {
        let seed = self.seed;
        *self = Self::from_config(seed, config);
    }

    /// Get the current configuration
    pub fn config(&self) -> &WorldGenConfig {
        &self.config
    }

    /// Get world parameters from config
    pub fn surface_y(&self) -> i32 {
        self.config.world.surface_y
    }

    /// Get bedrock depth from config
    pub fn bedrock_y(&self) -> i32 {
        self.config.world.bedrock_y
    }

    /// Get cavern layer depth from config
    pub fn cavern_layer(&self) -> i32 {
        self.config.world.underground_layers.cavern
    }

    /// Generate a complete chunk at the given chunk coordinates
    pub fn generate_chunk(&self, chunk_x: i32, chunk_y: i32) -> Chunk {
        let mut chunk = Chunk::new(chunk_x, chunk_y);

        for local_y in 0..CHUNK_SIZE {
            for local_x in 0..CHUNK_SIZE {
                let world_x = chunk_x * CHUNK_SIZE as i32 + local_x as i32;
                let world_y = chunk_y * CHUNK_SIZE as i32 + local_y as i32;

                let material = self.get_material_at(world_x, world_y);
                chunk.set_material(local_x, local_y, material);

                // Generate background layer - shows rock behind caves
                let background = self.get_background_at(world_x, world_y, material);
                chunk.set_background(local_x, local_y, background);
            }
        }

        // Mark fresh chunks as dirty so they get synced to database in multiplayer
        chunk.dirty = true;
        chunk
    }

    /// Determine background material at a world coordinate
    /// Background is decorative (no physics) - shows cave walls behind open spaces
    fn get_background_at(&self, world_x: i32, world_y: i32, foreground: u16) -> u16 {
        // Above ground: no background (sky)
        let terrain_height_value =
            self.terrain_height_noise.get_noise_2d(world_x as f32, 0.0) as f64;
        let height_variation = (terrain_height_value * 100.0) as i32;
        let terrain_y = SURFACE_Y + height_variation;

        if world_y > terrain_y - 5 {
            return MaterialId::AIR; // No background above/near surface
        }

        // Underground: if foreground is air (cave), show stone background
        if foreground == MaterialId::AIR {
            // Use offset noise to create variation in background
            let bg_noise = self.cave_noise_large.get_noise_2d(
                world_x as f32 + 1000.0, // Offset to get different pattern
                world_y as f32 + 1000.0,
            ) as f64;

            // Vary the background material for visual interest
            if bg_noise > 0.3 {
                return MaterialId::STONE;
            } else if bg_noise > 0.0 {
                // Darker stone variant (use dirt as placeholder for now)
                return MaterialId::DIRT;
            } else {
                return MaterialId::STONE;
            }
        }

        // Solid foreground: no visible background
        MaterialId::AIR
    }

    /// Determine material at a world coordinate using biome-based generation
    fn get_material_at(&self, world_x: i32, world_y: i32) -> u16 {
        // Bedrock layer (indestructible floor)
        if world_y <= BEDROCK_Y {
            return MaterialId::BEDROCK;
        }

        // Step 1: Sample biome noise to determine biome type
        let temperature = self.temperature_noise.get_noise_2d(world_x as f32, 0.0) as f64;
        let moisture = self.moisture_noise.get_noise_2d(world_x as f32, 0.0) as f64;

        let biome_type = select_biome(temperature, moisture);
        let biome = self.biome_registry.get(biome_type);

        // Step 2: Calculate terrain height using biome parameters
        let terrain_height_value =
            self.terrain_height_noise.get_noise_2d(world_x as f32, 0.0) as f64;
        // Apply biome-specific height variance and offset
        let height_variation = (terrain_height_value * 100.0 * biome.height_variance as f64) as i32;
        let terrain_y = SURFACE_Y + biome.height_offset + height_variation;

        // Step 3: Handle ocean biome specially
        if biome_type == BiomeType::Ocean {
            // Ocean surface is water
            if world_y > terrain_y && world_y <= SURFACE_Y - 20 {
                return MaterialId::WATER;
            }
            // Above water is air
            if world_y > SURFACE_Y - 20 {
                return MaterialId::AIR;
            }
            // Ocean floor is sand
            if world_y > terrain_y - 10 && world_y <= terrain_y {
                return MaterialId::SAND;
            }
        } else {
            // Non-ocean biomes: Air above terrain
            if world_y > terrain_y {
                return MaterialId::AIR;
            }
        }

        // Step 4: Surface vegetation
        let depth = terrain_y - world_y;
        if depth == 0 || depth == 1 {
            // Plant placement
            let plant_value = self.plant_noise.get_noise_2d(world_x as f32 * 0.05, 0.0) as f64;
            if plant_value > (1.0 - biome.plant_density as f64) {
                return MaterialId::PLANT_MATTER;
            }

            // Tree placement (will be expanded in Phase 2.5)
            let tree_value = self.tree_noise.get_noise_2d(world_x as f32 * 0.03, 0.0) as f64;
            if tree_value > (1.0 - biome.tree_density as f64) {
                // Simple tree: just wood for now
                if depth == 0 && world_y < SKY_HEIGHT {
                    return MaterialId::WOOD;
                }
            }
        }

        // Step 5: Underground caves (apply biome cave density multiplier)
        // Thresholds lowered to create larger, more open caves like Noita
        if world_y < terrain_y - 10 {
            let cave_large = self
                .cave_noise_large
                .get_noise_2d(world_x as f32, world_y as f32) as f64;
            let cave_small = self
                .cave_noise_small
                .get_noise_2d(world_x as f32, world_y as f32) as f64;

            // Apply biome cave density - lower thresholds = more cave space
            // Original: 0.3/0.4, now 0.15/0.25 for larger openings
            let cave_threshold_large = 0.15 / biome.cave_density_multiplier as f64;
            let cave_threshold_small = 0.25 / biome.cave_density_multiplier as f64;

            if cave_large > cave_threshold_large || cave_small > cave_threshold_small {
                return MaterialId::AIR;
            }
        }

        // Step 6: Material layers based on depth and biome
        if depth < 1 {
            // Surface material (grass, sand, stone, etc.)
            return biome.surface_material;
        } else if depth < biome.stone_depth {
            // Subsurface material (dirt, sandstone, etc.)
            return biome.subsurface_material;
        }

        // Step 7: Ore generation with biome multipliers
        const NOISE_SCALE: f32 = 0.08;

        // Coal: shallow underground (y=-50 to y=-500)
        if world_y > SHALLOW_UNDERGROUND && world_y < SURFACE_Y - 50 {
            let coal_value = self
                .coal_noise
                .get_noise_2d(world_x as f32 * NOISE_SCALE, world_y as f32 * NOISE_SCALE)
                as f64;
            let threshold = 0.75 / biome.get_ore_multiplier(MaterialId::COAL_ORE) as f64;
            if coal_value > threshold {
                return MaterialId::COAL_ORE;
            }
        }

        // Copper: medium depth (y=-200 to y=-1000)
        if world_y > -1000 && world_y < -200 {
            let copper_value = self
                .copper_noise
                .get_noise_2d(world_x as f32 * NOISE_SCALE, world_y as f32 * NOISE_SCALE)
                as f64;
            let threshold = 0.77 / biome.get_ore_multiplier(MaterialId::COPPER_ORE) as f64;
            if copper_value > threshold {
                return MaterialId::COPPER_ORE;
            }
        }

        // Iron: deep (y=-500 to y=-2000)
        if world_y > -2000 && world_y < -500 {
            let iron_value = self
                .iron_noise
                .get_noise_2d(world_x as f32 * NOISE_SCALE, world_y as f32 * NOISE_SCALE)
                as f64;
            let threshold = 0.76 / biome.get_ore_multiplier(MaterialId::IRON_ORE) as f64;
            if iron_value > threshold {
                return MaterialId::IRON_ORE;
            }
        }

        // Gold: very deep (y=-1500 to y=-3000)
        if world_y > -3000 && world_y < -1500 {
            let gold_value = self
                .gold_noise
                .get_noise_2d(world_x as f32 * NOISE_SCALE, world_y as f32 * NOISE_SCALE)
                as f64;
            let threshold = 0.80 / biome.get_ore_multiplier(MaterialId::GOLD_ORE) as f64;
            if gold_value > threshold {
                return MaterialId::GOLD_ORE;
            }
        }

        // Step 8: Lava layer at extreme depths (y < -2500)
        if world_y < CAVERN_LAYER {
            let lava_value = self
                .cave_noise_large
                .get_noise_2d(world_x as f32 * 0.05, world_y as f32 * 0.05)
                as f64;
            if lava_value > 0.6 {
                return MaterialId::LAVA;
            }
        }

        // Default: stone
        MaterialId::STONE
    }

    // Internal methods for ContextScanner access

    /// Get material at a world coordinate (public access for ContextScanner)
    pub(crate) fn get_material_at_internal(&self, world_x: i32, world_y: i32) -> u16 {
        self.get_material_at(world_x, world_y)
    }

    /// Get biome type at an X coordinate (public access for ContextScanner)
    pub(crate) fn get_biome_at_internal(&self, world_x: i32) -> BiomeType {
        let temperature = self.temperature_noise.get_noise_2d(world_x as f32, 0.0) as f64;
        let moisture = self.moisture_noise.get_noise_2d(world_x as f32, 0.0) as f64;
        select_biome(temperature, moisture)
    }

    /// Get terrain height at an X coordinate
    pub fn get_terrain_height(&self, world_x: i32) -> i32 {
        let temperature = self.temperature_noise.get_noise_2d(world_x as f32, 0.0) as f64;
        let moisture = self.moisture_noise.get_noise_2d(world_x as f32, 0.0) as f64;
        let biome_type = select_biome(temperature, moisture);
        let biome = self.biome_registry.get(biome_type);

        let terrain_height_value =
            self.terrain_height_noise.get_noise_2d(world_x as f32, 0.0) as f64;
        let height_variation = (terrain_height_value * 100.0 * biome.height_variance as f64) as i32;
        SURFACE_Y + biome.height_offset + height_variation
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deterministic_generation() {
        let gen1 = WorldGenerator::new(42);
        let gen2 = WorldGenerator::new(42);

        let chunk1 = gen1.generate_chunk(0, 0);
        let chunk2 = gen2.generate_chunk(0, 0);

        // Same seed should produce identical chunks
        for y in 0..CHUNK_SIZE {
            for x in 0..CHUNK_SIZE {
                assert_eq!(
                    chunk1.get_material(x, y),
                    chunk2.get_material(x, y),
                    "Mismatch at ({}, {})",
                    x,
                    y
                );
            }
        }
    }

    #[test]
    fn test_bedrock_layer() {
        let generator = WorldGenerator::new(42);

        // Chunk well below BEDROCK_Y should be all bedrock
        // chunk y=-60 * 64 = y=-3840 to y=-3777 (all below BEDROCK_Y = -3500)
        let chunk = generator.generate_chunk(0, -60);

        // All pixels should be bedrock (well below BEDROCK_Y = -3500)
        for y in 0..CHUNK_SIZE {
            for x in 0..CHUNK_SIZE {
                let material = chunk.get_material(x, y);
                assert_eq!(
                    material,
                    MaterialId::BEDROCK,
                    "Expected bedrock well below BEDROCK_Y"
                );
            }
        }
    }

    #[test]
    fn test_surface_layer() {
        let generator = WorldGenerator::new(42);

        // Chunk well above surface should be mostly air
        let chunk = generator.generate_chunk(0, 5); // chunk y=5 * 64 = y=320 to y=383

        let mut air_count = 0;
        for y in 0..CHUNK_SIZE {
            for x in 0..CHUNK_SIZE {
                if chunk.get_material(x, y) == MaterialId::AIR {
                    air_count += 1;
                }
            }
        }

        // Most pixels should be air well above surface (y=0)
        assert!(
            air_count > CHUNK_SIZE * CHUNK_SIZE / 2,
            "Expected mostly air above surface, got {} air pixels",
            air_count
        );
    }

    #[test]
    fn test_from_config_produces_same_as_default() {
        // WorldGenerator::new(seed) should produce same results as from_config(seed, default)
        let gen_new = WorldGenerator::new(42);
        let gen_config = WorldGenerator::from_config(42, WorldGenConfig::default());

        let chunk_new = gen_new.generate_chunk(0, 0);
        let chunk_config = gen_config.generate_chunk(0, 0);

        // Same config should produce identical chunks
        for y in 0..CHUNK_SIZE {
            for x in 0..CHUNK_SIZE {
                assert_eq!(
                    chunk_new.get_material(x, y),
                    chunk_config.get_material(x, y),
                    "Mismatch at ({}, {})",
                    x,
                    y
                );
            }
        }
    }

    #[test]
    fn test_update_config() {
        let mut generator = WorldGenerator::new(42);
        let _chunk_before = generator.generate_chunk(0, 0);

        // Update with different terrain height scale
        let mut config = WorldGenConfig::default();
        config.terrain.height_scale = 50.0; // Different from default 100.0
        generator.update_config(config);

        // Seed should be preserved
        assert_eq!(generator.seed, 42);

        // Config should be updated
        assert_eq!(generator.config().terrain.height_scale, 50.0);

        // New chunk should still be deterministic with same seed+config
        let chunk_after1 = generator.generate_chunk(0, 0);
        let chunk_after2 = generator.generate_chunk(0, 0);

        for y in 0..CHUNK_SIZE {
            for x in 0..CHUNK_SIZE {
                assert_eq!(
                    chunk_after1.get_material(x, y),
                    chunk_after2.get_material(x, y),
                    "Non-deterministic after config update at ({}, {})",
                    x,
                    y
                );
            }
        }
    }
}
