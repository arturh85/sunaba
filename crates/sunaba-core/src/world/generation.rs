use crate::simulation::MaterialId;
use crate::world::biome::{BiomeRegistry, BiomeType, select_biome};
use crate::world::chunk::{CHUNK_SIZE, Chunk};
use noise::{Fbm, MultiFractal, NoiseFn, Perlin};

// World dimension constants
pub const SURFACE_Y: i32 = 0; // Sea level baseline
pub const SKY_HEIGHT: i32 = 1000; // Top of atmosphere
pub const BEDROCK_Y: i32 = -3500; // Bedrock layer starts here
pub const MAX_UNDERGROUND: i32 = -3500; // Bottom before bedrock

// Underground layer boundaries
pub const SHALLOW_UNDERGROUND: i32 = -500; // Common ores, shallow caves
pub const DEEP_UNDERGROUND: i32 = -1500; // Better ores, larger caves
pub const CAVERN_LAYER: i32 = -2500; // Rare ores, huge caverns, lava

/// World generator using multi-octave Perlin noise for biome-based generation
pub struct WorldGenerator {
    pub seed: u64,

    // Biome definitions
    biome_registry: BiomeRegistry,

    // Large-scale (biome selection) - very low frequency
    temperature_noise: Fbm<Perlin>, // 2 octaves, freq=0.0003
    moisture_noise: Fbm<Perlin>,    // 2 octaves, freq=0.0003

    // Medium-scale (terrain features) - low frequency
    terrain_height_noise: Fbm<Perlin>, // 4 octaves, freq=0.001

    // Small-scale (cave details) - medium frequency
    cave_noise_large: Fbm<Perlin>, // 3 octaves, freq=0.01 (big caverns)
    cave_noise_small: Fbm<Perlin>, // 4 octaves, freq=0.02 (tunnels)

    // Per-ore noise layers
    coal_noise: Perlin,
    iron_noise: Perlin,
    copper_noise: Perlin,
    gold_noise: Perlin,

    // Vegetation placement
    tree_noise: Perlin,
    plant_noise: Perlin,
}

impl WorldGenerator {
    pub fn new(seed: u64) -> Self {
        // Large-scale biome selection noise (very low frequency for large regions)
        let temperature_noise = Fbm::<Perlin>::new(seed as u32)
            .set_octaves(2)
            .set_frequency(0.0003) // Very large biome regions (~3000 pixel wavelength)
            .set_lacunarity(2.0)
            .set_persistence(0.5);

        let moisture_noise = Fbm::<Perlin>::new((seed + 1) as u32)
            .set_octaves(2)
            .set_frequency(0.0003)
            .set_lacunarity(2.0)
            .set_persistence(0.5);

        // Medium-scale terrain height variation (hills and valleys)
        let terrain_height_noise = Fbm::<Perlin>::new((seed + 2) as u32)
            .set_octaves(4)
            .set_frequency(0.001) // ~1000 pixel wavelength for rolling hills
            .set_lacunarity(2.0)
            .set_persistence(0.5);

        // Large cave systems (big caverns) - reduced frequency for Noita-like scale
        // Target: 60-80px tall caverns (5-6x player height of 12px)
        let cave_noise_large = Fbm::<Perlin>::new((seed + 3) as u32)
            .set_octaves(3)
            .set_frequency(0.005) // ~200 pixel wavelength (was 0.01)
            .set_lacunarity(2.0)
            .set_persistence(0.5);

        // Small cave tunnels - reduced frequency for wider passages
        // Target: 36-48px wide tunnels (3-4x player height)
        let cave_noise_small = Fbm::<Perlin>::new((seed + 4) as u32)
            .set_octaves(4)
            .set_frequency(0.008) // ~125 pixel wavelength (was 0.02)
            .set_lacunarity(2.0)
            .set_persistence(0.5);

        // Ore placement noise (different seeds for variety)
        let coal_noise = Perlin::new((seed + 5) as u32);
        let iron_noise = Perlin::new((seed + 6) as u32);
        let copper_noise = Perlin::new((seed + 7) as u32);
        let gold_noise = Perlin::new((seed + 8) as u32);

        // Vegetation placement
        let tree_noise = Perlin::new((seed + 9) as u32);
        let plant_noise = Perlin::new((seed + 10) as u32);

        Self {
            seed,
            biome_registry: BiomeRegistry::new(),
            temperature_noise,
            moisture_noise,
            terrain_height_noise,
            cave_noise_large,
            cave_noise_small,
            coal_noise,
            iron_noise,
            copper_noise,
            gold_noise,
            tree_noise,
            plant_noise,
        }
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
        let terrain_height_value = self.terrain_height_noise.get([world_x as f64, 0.0]);
        let height_variation = (terrain_height_value * 100.0) as i32;
        let terrain_y = SURFACE_Y + height_variation;

        if world_y > terrain_y - 5 {
            return MaterialId::AIR; // No background above/near surface
        }

        // Underground: if foreground is air (cave), show stone background
        if foreground == MaterialId::AIR {
            // Use offset noise to create variation in background
            let bg_noise = self.cave_noise_large.get([
                world_x as f64 + 1000.0, // Offset to get different pattern
                world_y as f64 + 1000.0,
            ]);

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
        let temperature = self.temperature_noise.get([world_x as f64, 0.0]);
        let moisture = self.moisture_noise.get([world_x as f64, 0.0]);

        let biome_type = select_biome(temperature, moisture);
        let biome = self.biome_registry.get(biome_type);

        // Step 2: Calculate terrain height using biome parameters
        let terrain_height_value = self.terrain_height_noise.get([world_x as f64, 0.0]);
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
            let plant_value = self.plant_noise.get([world_x as f64 * 0.05, 0.0]);
            if plant_value > (1.0 - biome.plant_density as f64) {
                return MaterialId::PLANT_MATTER;
            }

            // Tree placement (will be expanded in Phase 2.5)
            let tree_value = self.tree_noise.get([world_x as f64 * 0.03, 0.0]);
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
            let cave_large = self.cave_noise_large.get([world_x as f64, world_y as f64]);
            let cave_small = self.cave_noise_small.get([world_x as f64, world_y as f64]);

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
        const NOISE_SCALE: f64 = 0.08;

        // Coal: shallow underground (y=-50 to y=-500)
        if world_y > SHALLOW_UNDERGROUND && world_y < SURFACE_Y - 50 {
            let coal_value = self
                .coal_noise
                .get([world_x as f64 * NOISE_SCALE, world_y as f64 * NOISE_SCALE]);
            let threshold = 0.75 / biome.get_ore_multiplier(MaterialId::COAL_ORE) as f64;
            if coal_value > threshold {
                return MaterialId::COAL_ORE;
            }
        }

        // Copper: medium depth (y=-200 to y=-1000)
        if world_y > -1000 && world_y < -200 {
            let copper_value = self
                .copper_noise
                .get([world_x as f64 * NOISE_SCALE, world_y as f64 * NOISE_SCALE]);
            let threshold = 0.77 / biome.get_ore_multiplier(MaterialId::COPPER_ORE) as f64;
            if copper_value > threshold {
                return MaterialId::COPPER_ORE;
            }
        }

        // Iron: deep (y=-500 to y=-2000)
        if world_y > -2000 && world_y < -500 {
            let iron_value = self
                .iron_noise
                .get([world_x as f64 * NOISE_SCALE, world_y as f64 * NOISE_SCALE]);
            let threshold = 0.76 / biome.get_ore_multiplier(MaterialId::IRON_ORE) as f64;
            if iron_value > threshold {
                return MaterialId::IRON_ORE;
            }
        }

        // Gold: very deep (y=-1500 to y=-3000)
        if world_y > -3000 && world_y < -1500 {
            let gold_value = self
                .gold_noise
                .get([world_x as f64 * NOISE_SCALE, world_y as f64 * NOISE_SCALE]);
            let threshold = 0.80 / biome.get_ore_multiplier(MaterialId::GOLD_ORE) as f64;
            if gold_value > threshold {
                return MaterialId::GOLD_ORE;
            }
        }

        // Step 8: Lava layer at extreme depths (y < -2500)
        if world_y < CAVERN_LAYER {
            let lava_value = self
                .cave_noise_large
                .get([world_x as f64 * 0.05, world_y as f64 * 0.05]);
            if lava_value > 0.6 {
                return MaterialId::LAVA;
            }
        }

        // Default: stone
        MaterialId::STONE
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
}
