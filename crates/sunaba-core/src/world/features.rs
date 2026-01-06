//! Post-generation feature placement
//!
//! Features are placed after base terrain/caves are generated, using
//! ContextScanner for context-aware placement decisions.

use crate::simulation::MaterialId;
use crate::world::chunk::Chunk;
use crate::world::context_scanner::{ContextScanner, PlacementPredicate};
use crate::world::generation::WorldGenerator;
use crate::world::worldgen_config::StalactiteConfig;
use fastnoise_lite::{FastNoiseLite, NoiseType};

/// Apply all enabled features to a freshly generated chunk
pub fn apply_features(chunk: &mut Chunk, chunk_x: i32, chunk_y: i32, generator: &WorldGenerator) {
    let config = generator.config();

    if config.features.stalactites.enabled {
        generate_stalactites(
            chunk,
            chunk_x,
            chunk_y,
            generator,
            &config.features.stalactites,
        );
    }

    // Future: stalagmites, crystals, mushrooms, etc.
}

/// Generate stalactites in a chunk
fn generate_stalactites(
    chunk: &mut Chunk,
    chunk_x: i32,
    chunk_y: i32,
    generator: &WorldGenerator,
    config: &StalactiteConfig,
) {
    const CHUNK_SIZE: i32 = 64;

    // Skip chunks above minimum depth
    let chunk_world_y = chunk_y * CHUNK_SIZE;
    if chunk_world_y > config.min_depth {
        return;
    }

    // Create scanner and noise generators
    let scanner = ContextScanner::new(generator);
    let mut placement_noise =
        FastNoiseLite::with_seed((generator.seed as i32) + config.seed_offset);
    placement_noise.set_noise_type(Some(NoiseType::OpenSimplex2));

    let mut length_noise =
        FastNoiseLite::with_seed((generator.seed as i32) + config.seed_offset + 1);
    length_noise.set_noise_type(Some(NoiseType::OpenSimplex2));

    // Build predicate for stalactite placement
    let predicate = PlacementPredicate::All(vec![
        PlacementPredicate::IsCaveInterior,
        PlacementPredicate::AtCeiling,
        PlacementPredicate::MinAirBelow(config.min_air_below),
    ]);

    // Sample on grid with spacing
    let chunk_world_x = chunk_x * CHUNK_SIZE;

    for local_y in (0..CHUNK_SIZE).step_by(config.spacing as usize) {
        for local_x in (0..CHUNK_SIZE).step_by(config.spacing as usize) {
            let world_x = chunk_world_x + local_x;
            let world_y = chunk_world_y + local_y;

            // Check if position matches predicate
            if !scanner.matches(world_x, world_y, &predicate) {
                continue;
            }

            // Use noise to randomize placement
            let placement_value =
                placement_noise.get_noise_2d(world_x as f32 * 0.1, world_y as f32 * 0.1) as f64;

            if placement_value < (1.0 - config.placement_chance as f64) {
                continue;
            }

            // Determine stalactite length using noise
            let length_value =
                length_noise.get_noise_2d(world_x as f32 * 0.05, world_y as f32 * 0.05) as f64;

            // Map noise [-1, 1] to [min_length, max_length]
            let length_range = (config.max_length - config.min_length) as f64;
            let length = config.min_length + ((length_value + 1.0) * 0.5 * length_range) as i32;

            // Draw the stalactite
            draw_stalactite(chunk, chunk_x, chunk_y, world_x, world_y, length, config);
        }
    }
}

/// Draw a single stalactite starting at (x, y)
fn draw_stalactite(
    chunk: &mut Chunk,
    chunk_x: i32,
    chunk_y: i32,
    world_x: i32,
    world_y: i32,
    length: i32,
    config: &StalactiteConfig,
) {
    const CHUNK_SIZE: i32 = 64;
    let chunk_world_x = chunk_x * CHUNK_SIZE;
    let chunk_world_y = chunk_y * CHUNK_SIZE;

    for dy in 0..length {
        // Calculate width at this depth
        let width = if config.taper {
            // Linear taper: base_width at top, 1 at bottom
            let taper_factor = 1.0 - (dy as f32 / length as f32);
            (config.base_width as f32 * taper_factor).max(1.0) as i32
        } else {
            config.base_width
        };

        // Draw horizontal line at this depth
        for dx in -(width / 2)..=(width / 2) {
            let pixel_world_x = world_x + dx;
            let pixel_world_y = world_y - dy; // Grow downward

            // Convert to chunk-local coordinates
            let local_x = pixel_world_x - chunk_world_x;
            let local_y = pixel_world_y - chunk_world_y;

            // Check bounds
            if (0..CHUNK_SIZE).contains(&local_x) && (0..CHUNK_SIZE).contains(&local_y) {
                // Only place if currently air (don't overwrite solid material)
                if chunk.get_material(local_x as usize, local_y as usize) == MaterialId::AIR {
                    chunk.set_material(local_x as usize, local_y as usize, MaterialId::STONE);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::generation::WorldGenerator;
    use crate::world::worldgen_config::WorldGenConfig;

    #[test]
    fn test_stalactite_config_default() {
        let config = StalactiteConfig::default();
        assert!(config.enabled);
        assert_eq!(config.min_length, 3);
        assert_eq!(config.max_length, 12);
        assert_eq!(config.spacing, 16);
        assert_eq!(config.base_width, 3);
        assert_eq!(config.min_air_below, 5);
        assert_eq!(config.placement_chance, 0.5);
        assert!(config.taper);
    }

    #[test]
    fn test_stalactites_disabled() {
        let mut config = WorldGenConfig::default();
        config.features.stalactites.enabled = false;

        let generator = WorldGenerator::from_config(42, config);
        let chunk = generator.generate_chunk(0, -10); // Deep underground

        // With stalactites disabled, should be deterministic
        let chunk2 = generator.generate_chunk(0, -10);

        for y in 0..64 {
            for x in 0..64 {
                assert_eq!(
                    chunk.get_material(x, y),
                    chunk2.get_material(x, y),
                    "Chunk should be deterministic with features disabled"
                );
            }
        }
    }

    #[test]
    fn test_stalactites_deterministic() {
        let generator = WorldGenerator::new(42);

        let chunk1 = generator.generate_chunk(0, -20);
        let chunk2 = generator.generate_chunk(0, -20);

        // Same seed should produce identical stalactites
        for y in 0..64 {
            for x in 0..64 {
                assert_eq!(
                    chunk1.get_material(x, y),
                    chunk2.get_material(x, y),
                    "Stalactites should be deterministic"
                );
            }
        }
    }

    #[test]
    fn test_draw_stalactite_bounds_checking() {
        // Test that drawing near chunk edges doesn't panic
        let mut chunk = Chunk::new(0, 0);
        let config = StalactiteConfig::default();

        // Draw at edge of chunk
        draw_stalactite(&mut chunk, 0, 0, 0, 0, 10, &config);
        draw_stalactite(&mut chunk, 0, 0, 63, 63, 10, &config);

        // Should not panic
    }

    #[test]
    fn test_stalactite_only_places_in_air() {
        let mut chunk = Chunk::new(0, 0);
        let config = StalactiteConfig::default();

        // Fill chunk with stone
        for y in 0..64 {
            for x in 0..64 {
                chunk.set_material(x, y, MaterialId::STONE);
            }
        }

        // Try to draw stalactite - should not overwrite existing stone
        draw_stalactite(&mut chunk, 0, 0, 32, 32, 10, &config);

        // All pixels should still be stone
        for y in 0..64 {
            for x in 0..64 {
                assert_eq!(chunk.get_material(x, y), MaterialId::STONE);
            }
        }
    }
}
