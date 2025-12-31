use crate::simulation::MaterialId;
use crate::world::chunk::{Chunk, CHUNK_SIZE};
use noise::{Fbm, MultiFractal, NoiseFn, Perlin};

/// World generator using multi-octave Perlin noise for cave generation
pub struct WorldGenerator {
    pub seed: u64,
    cave_noise: Fbm<Perlin>,
    // Separate noise layers for different ore types
    coal_noise: Perlin,
    iron_noise: Perlin,
    copper_noise: Perlin,
    gold_noise: Perlin,
}

impl WorldGenerator {
    pub fn new(seed: u64) -> Self {
        // Configure multi-octave noise for natural-looking caves
        let cave_noise = Fbm::<Perlin>::new(seed as u32)
            .set_octaves(4) // Detail levels
            .set_frequency(0.02) // Cave scale (~50 pixel wavelength)
            .set_lacunarity(2.0) // Octave frequency multiplier
            .set_persistence(0.5); // Octave amplitude multiplier

        // Separate noise layers for each ore type (different seeds for variety)
        let coal_noise = Perlin::new((seed + 1) as u32);
        let iron_noise = Perlin::new((seed + 2) as u32);
        let copper_noise = Perlin::new((seed + 3) as u32);
        let gold_noise = Perlin::new((seed + 4) as u32);

        Self {
            seed,
            cave_noise,
            coal_noise,
            iron_noise,
            copper_noise,
            gold_noise,
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
            }
        }

        // Fresh chunks start as not dirty
        chunk.dirty = false;
        chunk
    }

    /// Determine material at a world coordinate using noise sampling
    fn get_material_at(&self, world_x: i32, world_y: i32) -> u16 {
        // Vertical layer constants
        const SURFACE_LEVEL: i32 = 32; // y=32 is ground level
        const BEDROCK_LEVEL: i32 = -96; // y=-96 is indestructible floor

        // Bedrock layer (indestructible floor)
        if world_y <= BEDROCK_LEVEL {
            return MaterialId::BEDROCK;
        }

        // Air above surface
        if world_y > SURFACE_LEVEL {
            return MaterialId::AIR;
        }

        // Underground: Use cave noise to carve out caves
        let cave_value = self.cave_noise.get([world_x as f64, world_y as f64]);

        // Cave carving: threshold determines cave density
        // Higher threshold = more caves
        const CAVE_THRESHOLD: f64 = 0.2;
        if cave_value > CAVE_THRESHOLD {
            return MaterialId::AIR;
        }

        // Solid terrain: depth-based material selection
        let depth = SURFACE_LEVEL - world_y;

        // Top layer: dirt and sand (shallow)
        if depth < 4 {
            return MaterialId::DIRT;
        } else if depth < 8 {
            return MaterialId::SAND;
        }

        // Ore generation with depth stratification
        // Noise scale: 0.1 = ~10 pixel wavelength for vein patterns
        const NOISE_SCALE: f64 = 0.08;

        // Coal: shallow (depth 8-40, roughly y=24 to y=-8)
        if (8..=40).contains(&depth) {
            let coal_value = self
                .coal_noise
                .get([world_x as f64 * NOISE_SCALE, world_y as f64 * NOISE_SCALE]);
            // Threshold 0.75 = ~3% density
            if coal_value > 0.75 {
                return MaterialId::COAL_ORE;
            }
        }

        // Copper: medium depth (depth 20-60, roughly y=12 to y=-28)
        if (20..=60).contains(&depth) {
            let copper_value = self
                .copper_noise
                .get([world_x as f64 * NOISE_SCALE, world_y as f64 * NOISE_SCALE]);
            // Threshold 0.77 = ~2.5% density
            if copper_value > 0.77 {
                return MaterialId::COPPER_ORE;
            }
        }

        // Iron: medium-deep (depth 30-80, roughly y=2 to y=-48)
        if (30..=80).contains(&depth) {
            let iron_value = self
                .iron_noise
                .get([world_x as f64 * NOISE_SCALE, world_y as f64 * NOISE_SCALE]);
            // Threshold 0.76 = ~2.8% density
            if iron_value > 0.76 {
                return MaterialId::IRON_ORE;
            }
        }

        // Gold: deep (depth 60-120, roughly y=-28 to y=-88)
        if (60..=120).contains(&depth) {
            let gold_value = self
                .gold_noise
                .get([world_x as f64 * NOISE_SCALE, world_y as f64 * NOISE_SCALE]);
            // Threshold 0.80 = ~2% density (rarest)
            if gold_value > 0.80 {
                return MaterialId::GOLD_ORE;
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
        let gen = WorldGenerator::new(42);

        // Chunk at y=-96 should be all bedrock
        let chunk = gen.generate_chunk(0, -2); // chunk y=-2 * 64 = y=-128 to y=-65

        // Bottom rows should be bedrock
        for x in 0..CHUNK_SIZE {
            let material = chunk.get_material(x, 0);
            assert_eq!(
                material,
                MaterialId::BEDROCK,
                "Expected bedrock at bottom of chunk"
            );
        }
    }

    #[test]
    fn test_surface_layer() {
        let gen = WorldGenerator::new(42);

        // Chunk above surface should be mostly air
        let chunk = gen.generate_chunk(0, 1); // chunk y=1 * 64 = y=64 to y=127

        let mut air_count = 0;
        for y in 0..CHUNK_SIZE {
            for x in 0..CHUNK_SIZE {
                if chunk.get_material(x, y) == MaterialId::AIR {
                    air_count += 1;
                }
            }
        }

        // Most pixels should be air above surface
        assert!(
            air_count > CHUNK_SIZE * CHUNK_SIZE / 2,
            "Expected mostly air above surface, got {} air pixels",
            air_count
        );
    }
}
