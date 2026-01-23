//! Post-generation feature placement
//!
//! Features are placed after base terrain/caves are generated, using
//! ContextScanner for context-aware placement decisions.

use crate::simulation::MaterialId;
use crate::world::biome_zones::UndergroundZone;
use crate::world::chunk::Chunk;
use crate::world::context_scanner::{ContextScanner, PlacementPredicate};
use crate::world::generation::WorldGenerator;
use crate::world::worldgen_config::{
    StalactiteConfig, ThunderZoneConfig, ToxicVentConfig, VolatilePoolConfig, WireNetworkConfig,
};
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

    // Structure generation - load templates once
    let templates = crate::world::structure_templates::create_builtin_templates();

    if config.features.structures.bridges.enabled {
        generate_bridges(
            chunk,
            chunk_x,
            chunk_y,
            generator,
            &config.features.structures.bridges,
            &templates,
        );
    }

    if config.features.structures.trees.enabled {
        generate_trees(
            chunk,
            chunk_x,
            chunk_y,
            generator,
            &config.features.structures.trees,
            &templates,
        );
    }

    if config.features.structures.ruins.enabled {
        generate_ruins(
            chunk,
            chunk_x,
            chunk_y,
            generator,
            &config.features.structures.ruins,
            &templates,
        );
    }

    // Zone-specific features for ML creatures
    if config.features.wire_networks.enabled {
        generate_wire_networks(
            chunk,
            chunk_x,
            chunk_y,
            generator,
            &config.features.wire_networks,
        );
    }

    if config.features.thunder_zones.enabled {
        generate_thunder_zones(
            chunk,
            chunk_x,
            chunk_y,
            generator,
            &config.features.thunder_zones,
        );
    }

    if config.features.volatile_pools.enabled {
        generate_volatile_pools(
            chunk,
            chunk_x,
            chunk_y,
            generator,
            &config.features.volatile_pools,
        );
    }

    if config.features.toxic_vents.enabled {
        generate_toxic_vents(
            chunk,
            chunk_x,
            chunk_y,
            generator,
            &config.features.toxic_vents,
        );
    }
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

/// Generate wooden bridges over gaps
fn generate_bridges(
    chunk: &mut Chunk,
    chunk_x: i32,
    chunk_y: i32,
    generator: &WorldGenerator,
    config: &crate::world::worldgen_config::BridgeConfig,
    templates: &std::collections::HashMap<&str, crate::world::structures::StructureVariants>,
) {
    const CHUNK_SIZE: i32 = 64;
    let chunk_world_y = chunk_y * CHUNK_SIZE;

    // Skip chunks above min depth
    if chunk_world_y > config.min_depth {
        return;
    }

    let scanner = ContextScanner::new(generator);
    let mut placement_noise =
        FastNoiseLite::with_seed((generator.seed as i32) + config.seed_offset);
    placement_noise.set_noise_type(Some(NoiseType::OpenSimplex2));

    let bridge_variants = templates.get("wooden_bridge").unwrap();
    let chunk_world_x = chunk_x * CHUNK_SIZE;

    for local_y in (0..CHUNK_SIZE).step_by(config.spacing as usize) {
        for local_x in (0..CHUNK_SIZE).step_by(config.spacing as usize) {
            let world_x = chunk_world_x + local_x;
            let world_y = chunk_world_y + local_y;

            // Check gap width at this position
            let gap_width = scanner.scan_gap_width(world_x, world_y);

            if gap_width < config.min_gap_width || gap_width > config.max_gap_width {
                continue;
            }

            // Check placement probability
            let placement_value =
                placement_noise.get_noise_2d(world_x as f32 * 0.1, world_y as f32 * 0.1) as f64;

            if placement_value < (1.0 - config.placement_chance as f64) {
                continue;
            }

            // Select appropriate bridge variant based on gap width
            let variant_noise =
                placement_noise.get_noise_2d(world_x as f32 * 0.05, world_y as f32 * 0.05) as f64;

            let template = bridge_variants.select_variant(variant_noise);

            // Validate placement
            if !crate::world::structure_placement::is_placement_valid(
                world_x, world_y, template, &scanner,
            ) {
                continue;
            }

            // Place the bridge
            crate::world::structure_placement::place_structure(
                chunk, chunk_x, chunk_y, world_x, world_y, template, &scanner,
            );
        }
    }
}

/// Generate surface trees (normal or marker based on cave detection)
fn generate_trees(
    chunk: &mut Chunk,
    chunk_x: i32,
    chunk_y: i32,
    generator: &WorldGenerator,
    config: &crate::world::worldgen_config::TreeConfig,
    templates: &std::collections::HashMap<&str, crate::world::structures::StructureVariants>,
) {
    const CHUNK_SIZE: i32 = 64;

    let scanner = ContextScanner::new(generator);
    let mut placement_noise =
        FastNoiseLite::with_seed((generator.seed as i32) + config.seed_offset);
    placement_noise.set_noise_type(Some(NoiseType::OpenSimplex2));

    let normal_variants = templates.get("tree_normal").unwrap();
    let marker_variants = templates.get("tree_marker").unwrap();

    let predicate = PlacementPredicate::All(vec![
        PlacementPredicate::IsSurface,
        PlacementPredicate::OnGround,
        PlacementPredicate::MinAirAbove(config.min_air_above),
    ]);

    let chunk_world_x = chunk_x * CHUNK_SIZE;
    let chunk_world_y = chunk_y * CHUNK_SIZE;

    for local_y in (0..CHUNK_SIZE).step_by(config.spacing as usize) {
        for local_x in (0..CHUNK_SIZE).step_by(config.spacing as usize) {
            let world_x = chunk_world_x + local_x;
            let world_y = chunk_world_y + local_y;

            // Check if position matches surface tree predicate
            if !scanner.matches(world_x, world_y, &predicate) {
                continue;
            }

            // Check placement probability
            let placement_value =
                placement_noise.get_noise_2d(world_x as f32 * 0.1, world_y as f32 * 0.1) as f64;

            if placement_value < (1.0 - config.placement_chance as f64) {
                continue;
            }

            // Detect cave below by scanning downward
            let has_cave_below =
                detect_cave_below(world_x, world_y, config.cave_scan_depth, &scanner);

            // Select tree type based on cave detection
            let tree_variants = if has_cave_below {
                let marker_chance = placement_noise
                    .get_noise_2d(world_x as f32 * 0.07, world_y as f32 * 0.07)
                    as f64;

                if marker_chance > (1.0 - config.marker_tree_chance as f64) {
                    marker_variants
                } else {
                    normal_variants
                }
            } else {
                normal_variants
            };

            let variant_noise =
                placement_noise.get_noise_2d(world_x as f32 * 0.05, world_y as f32 * 0.05) as f64;

            let template = tree_variants.select_variant(variant_noise);

            crate::world::structure_placement::place_structure(
                chunk, chunk_x, chunk_y, world_x, world_y, template, &scanner,
            );
        }
    }
}

/// Detect if there's a cave within scan depth below position
fn detect_cave_below(
    world_x: i32,
    world_y: i32,
    scan_depth: i32,
    scanner: &ContextScanner,
) -> bool {
    for dy in 1..=scan_depth {
        let check_y = world_y - dy;

        // Check for cave (air surrounded by solid)
        let is_air = scanner.get_material(world_x, check_y) == MaterialId::AIR;
        let has_ceiling = scanner.get_material(world_x, check_y + 1) != MaterialId::AIR;
        let has_floor = scanner.get_material(world_x, check_y - 1) != MaterialId::AIR;

        if is_air && has_ceiling && has_floor {
            return true; // Found enclosed cave space
        }
    }
    false
}

/// Generate underground ruins
fn generate_ruins(
    chunk: &mut Chunk,
    chunk_x: i32,
    chunk_y: i32,
    generator: &WorldGenerator,
    config: &crate::world::worldgen_config::RuinConfig,
    templates: &std::collections::HashMap<&str, crate::world::structures::StructureVariants>,
) {
    const CHUNK_SIZE: i32 = 64;
    let chunk_world_y = chunk_y * CHUNK_SIZE;

    // Skip chunks outside depth range
    if chunk_world_y > config.max_depth || chunk_world_y < config.min_depth {
        return;
    }

    let scanner = ContextScanner::new(generator);
    let mut placement_noise =
        FastNoiseLite::with_seed((generator.seed as i32) + config.seed_offset);
    placement_noise.set_noise_type(Some(NoiseType::OpenSimplex2));

    let predicate = PlacementPredicate::All(vec![
        PlacementPredicate::IsCaveInterior,
        PlacementPredicate::OnGround,
        PlacementPredicate::MinAirAbove(8),
    ]);

    let chunk_world_x = chunk_x * CHUNK_SIZE;

    for local_y in (0..CHUNK_SIZE).step_by(config.spacing as usize) {
        for local_x in (0..CHUNK_SIZE).step_by(config.spacing as usize) {
            let world_x = chunk_world_x + local_x;
            let world_y = chunk_world_y + local_y;

            if !scanner.matches(world_x, world_y, &predicate) {
                continue;
            }

            let placement_value =
                placement_noise.get_noise_2d(world_x as f32 * 0.1, world_y as f32 * 0.1) as f64;

            if placement_value < (1.0 - config.placement_chance as f64) {
                continue;
            }

            // Randomly select wall or pillar
            let type_noise =
                placement_noise.get_noise_2d(world_x as f32 * 0.03, world_y as f32 * 0.03) as f64;

            let variants = if type_noise > 0.0 {
                templates.get("ruin_wall").unwrap()
            } else {
                templates.get("ruin_pillar").unwrap()
            };

            let variant_noise =
                placement_noise.get_noise_2d(world_x as f32 * 0.05, world_y as f32 * 0.05) as f64;

            let template = variants.select_variant(variant_noise);

            crate::world::structure_placement::place_structure(
                chunk, chunk_x, chunk_y, world_x, world_y, template, &scanner,
            );
        }
    }
}

/// Generate wire networks in Circuit Ruins zone
fn generate_wire_networks(
    chunk: &mut Chunk,
    chunk_x: i32,
    chunk_y: i32,
    generator: &WorldGenerator,
    config: &WireNetworkConfig,
) {
    const CHUNK_SIZE: i32 = 64;

    let chunk_world_y = chunk_y * CHUNK_SIZE;
    let chunk_world_x = chunk_x * CHUNK_SIZE;

    // Skip chunks outside the Circuit Ruins zone
    if chunk_world_y > config.min_depth || chunk_world_y < config.max_depth {
        return;
    }

    // Verify we're in Circuit Ruins zone
    let zone = generator.zone_registry().get_zone_at(chunk_world_y);
    if !matches!(
        zone.map(|z| z.zone_type),
        Some(UndergroundZone::CircuitRuins)
    ) {
        return;
    }

    let mut wire_noise_h = FastNoiseLite::with_seed((generator.seed as i32) + config.seed_offset);
    wire_noise_h.set_noise_type(Some(NoiseType::OpenSimplex2));

    let mut wire_noise_v =
        FastNoiseLite::with_seed((generator.seed as i32) + config.seed_offset + 1);
    wire_noise_v.set_noise_type(Some(NoiseType::OpenSimplex2));

    let mut battery_noise =
        FastNoiseLite::with_seed((generator.seed as i32) + config.seed_offset + 2);
    battery_noise.set_noise_type(Some(NoiseType::OpenSimplex2));

    for local_y in 0..CHUNK_SIZE {
        for local_x in 0..CHUNK_SIZE {
            let world_x = chunk_world_x + local_x;
            let world_y = chunk_world_y + local_y;

            // Only place in solid stone
            let current = chunk.get_material(local_x as usize, local_y as usize);
            if current != MaterialId::STONE {
                continue;
            }

            // Horizontal wire traces
            let h_noise = wire_noise_h.get_noise_2d(
                world_x as f32 * 0.005,
                world_y as f32 * 0.02, // More variation in Y for horizontal lines
            ) as f64;

            // Vertical wire traces
            let v_noise = wire_noise_v.get_noise_2d(
                world_x as f32 * 0.02, // More variation in X for vertical lines
                world_y as f32 * 0.005,
            ) as f64;

            let is_h_wire = h_noise > config.h_wire_threshold as f64;
            let is_v_wire = v_noise > config.v_wire_threshold as f64;

            if is_h_wire || is_v_wire {
                // Check for intersection (potential battery placement)
                if is_h_wire && is_v_wire {
                    let battery_value = battery_noise
                        .get_noise_2d(world_x as f32 * 0.1, world_y as f32 * 0.1)
                        as f64;
                    if battery_value > (1.0 - config.battery_chance as f64 * 2.0) {
                        chunk.set_material(local_x as usize, local_y as usize, MaterialId::BATTERY);
                        continue;
                    }
                }
                chunk.set_material(local_x as usize, local_y as usize, MaterialId::WIRE);
            }
        }
    }

    // Place sparks near batteries in air spaces
    let scanner = ContextScanner::new(generator);
    let mut spark_noise =
        FastNoiseLite::with_seed((generator.seed as i32) + config.seed_offset + 3);
    spark_noise.set_noise_type(Some(NoiseType::OpenSimplex2));

    for local_y in 0..CHUNK_SIZE {
        for local_x in 0..CHUNK_SIZE {
            let world_x = chunk_world_x + local_x;
            let world_y = chunk_world_y + local_y;

            // Only place sparks in air
            let current = chunk.get_material(local_x as usize, local_y as usize);
            if current != MaterialId::AIR {
                continue;
            }

            // Check if near a battery (scan 16px radius)
            let near_battery = (-8..=8).any(|dy| {
                (-8..=8).any(|dx| {
                    let check_x = local_x + dx;
                    let check_y = local_y + dy;
                    if (0..CHUNK_SIZE).contains(&check_x) && (0..CHUNK_SIZE).contains(&check_y) {
                        chunk.get_material(check_x as usize, check_y as usize)
                            == MaterialId::BATTERY
                    } else {
                        // Out of chunk - check via scanner
                        scanner.get_material(world_x + dx, world_y + dy) == MaterialId::BATTERY
                    }
                })
            });

            if near_battery {
                let spark_value =
                    spark_noise.get_noise_2d(world_x as f32 * 0.2, world_y as f32 * 0.2) as f64;
                if spark_value > (1.0 - config.spark_chance as f64) {
                    chunk.set_material(local_x as usize, local_y as usize, MaterialId::SPARK);
                }
            }
        }
    }
}

/// Generate thunder zones in Thunder Caverns
fn generate_thunder_zones(
    chunk: &mut Chunk,
    chunk_x: i32,
    chunk_y: i32,
    generator: &WorldGenerator,
    config: &ThunderZoneConfig,
) {
    const CHUNK_SIZE: i32 = 64;

    let chunk_world_y = chunk_y * CHUNK_SIZE;
    let chunk_world_x = chunk_x * CHUNK_SIZE;

    // Skip chunks outside the Thunder Caverns zone
    if chunk_world_y > config.min_depth || chunk_world_y < config.max_depth {
        return;
    }

    // Verify we're in Thunder Caverns zone
    let zone = generator.zone_registry().get_zone_at(chunk_world_y);
    if !matches!(
        zone.map(|z| z.zone_type),
        Some(UndergroundZone::ThunderCaverns)
    ) {
        return;
    }

    let scanner = ContextScanner::new(generator);

    let mut thunder_noise = FastNoiseLite::with_seed((generator.seed as i32) + config.seed_offset);
    thunder_noise.set_noise_type(Some(NoiseType::OpenSimplex2));

    let mut spark_noise =
        FastNoiseLite::with_seed((generator.seed as i32) + config.seed_offset + 1);
    spark_noise.set_noise_type(Some(NoiseType::OpenSimplex2));

    // Pass 1: Place thunder clusters at cave ceilings
    for local_y in 0..CHUNK_SIZE {
        for local_x in 0..CHUNK_SIZE {
            let world_x = chunk_world_x + local_x;
            let world_y = chunk_world_y + local_y;

            let current = chunk.get_material(local_x as usize, local_y as usize);

            // Place thunder in basalt near cave ceilings
            if current == MaterialId::BASALT {
                // Check if this is a ceiling (air below)
                let below_y = local_y - 1;
                let is_ceiling = if below_y >= 0 {
                    chunk.get_material(local_x as usize, below_y as usize) == MaterialId::AIR
                } else {
                    scanner.get_material(world_x, world_y - 1) == MaterialId::AIR
                };

                if is_ceiling {
                    let thunder_value = thunder_noise
                        .get_noise_2d(world_x as f32 * 0.02, world_y as f32 * 0.02)
                        as f64;
                    if thunder_value > config.thunder_threshold as f64 {
                        chunk.set_material(local_x as usize, local_y as usize, MaterialId::THUNDER);
                    }
                }
            }

            // Place sparks in cave air
            if current == MaterialId::AIR {
                let spark_value =
                    spark_noise.get_noise_2d(world_x as f32 * 0.05, world_y as f32 * 0.05) as f64;
                if spark_value > (1.0 - config.spark_chance as f64) {
                    chunk.set_material(local_x as usize, local_y as usize, MaterialId::SPARK);
                }
            }
        }
    }

    // Pass 2: Place wire lightning rods below thunder
    let mut rod_noise = FastNoiseLite::with_seed((generator.seed as i32) + config.seed_offset + 2);
    rod_noise.set_noise_type(Some(NoiseType::OpenSimplex2));

    for local_x in 0..CHUNK_SIZE {
        for local_y in (0..CHUNK_SIZE).rev() {
            // Start from top
            let world_x = chunk_world_x + local_x;
            let world_y = chunk_world_y + local_y;

            // Check if there's thunder above
            let above_y = local_y + 1;
            let has_thunder_above = if above_y < CHUNK_SIZE {
                chunk.get_material(local_x as usize, above_y as usize) == MaterialId::THUNDER
            } else {
                scanner.get_material(world_x, world_y + 1) == MaterialId::THUNDER
            };

            if has_thunder_above {
                let current = chunk.get_material(local_x as usize, local_y as usize);
                if current == MaterialId::AIR {
                    let rod_value =
                        rod_noise.get_noise_2d(world_x as f32 * 0.1, world_y as f32 * 0.1) as f64;
                    if rod_value > (1.0 - config.wire_rod_chance as f64) {
                        chunk.set_material(local_x as usize, local_y as usize, MaterialId::WIRE);
                    }
                }
            }
        }
    }
}

/// Generate volatile pools in Volatile Lakes zone
fn generate_volatile_pools(
    chunk: &mut Chunk,
    chunk_x: i32,
    chunk_y: i32,
    generator: &WorldGenerator,
    config: &VolatilePoolConfig,
) {
    const CHUNK_SIZE: i32 = 64;

    let chunk_world_y = chunk_y * CHUNK_SIZE;
    let chunk_world_x = chunk_x * CHUNK_SIZE;

    // Skip chunks outside the Volatile Lakes zone
    if chunk_world_y > config.min_depth || chunk_world_y < config.max_depth {
        return;
    }

    // Verify we're in Volatile Lakes zone
    let zone = generator.zone_registry().get_zone_at(chunk_world_y);
    if !matches!(
        zone.map(|z| z.zone_type),
        Some(UndergroundZone::VolatileLakes)
    ) {
        return;
    }

    let scanner = ContextScanner::new(generator);

    let mut pool_noise = FastNoiseLite::with_seed((generator.seed as i32) + config.seed_offset);
    pool_noise.set_noise_type(Some(NoiseType::OpenSimplex2));

    let mut material_noise =
        FastNoiseLite::with_seed((generator.seed as i32) + config.seed_offset + 1);
    material_noise.set_noise_type(Some(NoiseType::OpenSimplex2));

    let mut gunpowder_noise =
        FastNoiseLite::with_seed((generator.seed as i32) + config.seed_offset + 2);
    gunpowder_noise.set_noise_type(Some(NoiseType::OpenSimplex2));

    let mut lava_noise = FastNoiseLite::with_seed((generator.seed as i32) + config.seed_offset + 3);
    lava_noise.set_noise_type(Some(NoiseType::OpenSimplex2));

    for local_y in 0..CHUNK_SIZE {
        for local_x in 0..CHUNK_SIZE {
            let world_x = chunk_world_x + local_x;
            let world_y = chunk_world_y + local_y;

            let current = chunk.get_material(local_x as usize, local_y as usize);

            // Place pools in cave floor air
            if current == MaterialId::AIR {
                // Check if this is on the floor (solid below)
                let below_y = local_y - 1;
                let is_floor = if below_y >= 0 {
                    let below_mat = chunk.get_material(local_x as usize, below_y as usize);
                    below_mat != MaterialId::AIR && below_mat != MaterialId::WATER
                } else {
                    let below_mat = scanner.get_material(world_x, world_y - 1);
                    below_mat != MaterialId::AIR && below_mat != MaterialId::WATER
                };

                if is_floor {
                    let pool_value = pool_noise
                        .get_noise_2d(world_x as f32 * 0.008, world_y as f32 * 0.008)
                        as f64;

                    if pool_value > config.pool_threshold as f64 {
                        // Select pool material
                        let mat_value = material_noise
                            .get_noise_2d(world_x as f32 * 0.05, world_y as f32 * 0.05)
                            as f64;
                        let mat_normalized = (mat_value + 1.0) / 2.0; // 0.0 to 1.0

                        let total_weight = config.nitro_weight + config.oil_weight + 0.1; // 0.1 for water
                        let nitro_threshold = config.nitro_weight / total_weight;
                        let oil_threshold =
                            (config.nitro_weight + config.oil_weight) / total_weight;

                        let pool_mat = if mat_normalized < nitro_threshold as f64 {
                            MaterialId::NITRO
                        } else if mat_normalized < oil_threshold as f64 {
                            MaterialId::OIL
                        } else {
                            MaterialId::WATER
                        };

                        chunk.set_material(local_x as usize, local_y as usize, pool_mat);
                    }
                }
            }

            // Place gunpowder near pool edges (in stone)
            if current == MaterialId::STONE {
                // Check if near a pool
                let near_pool = (-3..=3).any(|dy| {
                    (-3..=3).any(|dx| {
                        if dx == 0 && dy == 0 {
                            return false;
                        }
                        let check_x = local_x + dx;
                        let check_y = local_y + dy;
                        if (0..CHUNK_SIZE).contains(&check_x) && (0..CHUNK_SIZE).contains(&check_y)
                        {
                            let mat = chunk.get_material(check_x as usize, check_y as usize);
                            mat == MaterialId::NITRO || mat == MaterialId::OIL
                        } else {
                            false
                        }
                    })
                });

                if near_pool {
                    let gp_value = gunpowder_noise
                        .get_noise_2d(world_x as f32 * 0.1, world_y as f32 * 0.1)
                        as f64;
                    if gp_value > (1.0 - config.gunpowder_chance as f64) {
                        chunk.set_material(
                            local_x as usize,
                            local_y as usize,
                            MaterialId::GUNPOWDER,
                        );
                    }
                }
            }

            // Place lava veins (dangerous ignition source!)
            if current == MaterialId::STONE {
                let lava_value =
                    lava_noise.get_noise_2d(world_x as f32 * 0.015, world_y as f32 * 0.015) as f64;
                if lava_value > config.lava_vein_threshold as f64 {
                    chunk.set_material(local_x as usize, local_y as usize, MaterialId::LAVA);
                }
            }
        }
    }
}

/// Generate toxic vents in Toxic Depths zone
fn generate_toxic_vents(
    chunk: &mut Chunk,
    chunk_x: i32,
    chunk_y: i32,
    generator: &WorldGenerator,
    config: &ToxicVentConfig,
) {
    const CHUNK_SIZE: i32 = 64;

    let chunk_world_y = chunk_y * CHUNK_SIZE;
    let chunk_world_x = chunk_x * CHUNK_SIZE;

    // Skip chunks outside the Toxic Depths zone
    if chunk_world_y > config.min_depth || chunk_world_y < config.max_depth {
        return;
    }

    // Verify we're in Toxic Depths zone
    let zone = generator.zone_registry().get_zone_at(chunk_world_y);
    if !matches!(
        zone.map(|z| z.zone_type),
        Some(UndergroundZone::ToxicDepths)
    ) {
        return;
    }

    let scanner = ContextScanner::new(generator);

    let mut vent_noise = FastNoiseLite::with_seed((generator.seed as i32) + config.seed_offset);
    vent_noise.set_noise_type(Some(NoiseType::OpenSimplex2));

    let mut virus_noise =
        FastNoiseLite::with_seed((generator.seed as i32) + config.seed_offset + 1);
    virus_noise.set_noise_type(Some(NoiseType::OpenSimplex2));

    let mut mercury_noise =
        FastNoiseLite::with_seed((generator.seed as i32) + config.seed_offset + 2);
    mercury_noise.set_noise_type(Some(NoiseType::OpenSimplex2));

    // Pass 1: Find vent locations and place gas columns
    for local_x in 0..CHUNK_SIZE {
        // Check for vent at this X position
        let world_x = chunk_world_x + local_x;
        let vent_value = vent_noise.get_noise_2d(world_x as f32 * 0.02, 0.0) as f64; // X-based only for consistent columns

        if vent_value > config.vent_threshold as f64 {
            // Find the floor level for this column
            for local_y in 0..CHUNK_SIZE {
                let world_y = chunk_world_y + local_y;

                let current = chunk.get_material(local_x as usize, local_y as usize);
                if current != MaterialId::AIR {
                    continue;
                }

                // Check if this is floor level
                let below_y = local_y - 1;
                let is_floor = if below_y >= 0 {
                    chunk.get_material(local_x as usize, below_y as usize) != MaterialId::AIR
                } else {
                    scanner.get_material(world_x, world_y - 1) != MaterialId::AIR
                };

                if is_floor {
                    // Place gas column rising from this point
                    let gas_height = (config.max_gas_height as f32
                        * ((vent_value - config.vent_threshold as f64) * 3.0 + 0.5) as f32)
                        .min(config.max_gas_height as f32)
                        as i32;

                    for dy in 0..gas_height {
                        let gas_y = local_y + dy;
                        if gas_y >= CHUNK_SIZE {
                            break;
                        }
                        if chunk.get_material(local_x as usize, gas_y as usize) == MaterialId::AIR {
                            chunk.set_material(
                                local_x as usize,
                                gas_y as usize,
                                MaterialId::POISON_GAS,
                            );
                        }
                    }
                }
            }
        }
    }

    // Pass 2: Virus patches on walls
    for local_y in 0..CHUNK_SIZE {
        for local_x in 0..CHUNK_SIZE {
            let world_x = chunk_world_x + local_x;
            let world_y = chunk_world_y + local_y;

            let current = chunk.get_material(local_x as usize, local_y as usize);

            // Place virus on basalt walls (adjacent to air)
            if current == MaterialId::BASALT {
                let has_adjacent_air =
                    [(-1, 0), (1, 0), (0, -1), (0, 1)].iter().any(|&(dx, dy)| {
                        let check_x = local_x + dx;
                        let check_y = local_y + dy;
                        if (0..CHUNK_SIZE).contains(&check_x) && (0..CHUNK_SIZE).contains(&check_y)
                        {
                            chunk.get_material(check_x as usize, check_y as usize)
                                == MaterialId::AIR
                        } else {
                            scanner.get_material(world_x + dx, world_y + dy) == MaterialId::AIR
                        }
                    });

                if has_adjacent_air {
                    let virus_value = virus_noise
                        .get_noise_2d(world_x as f32 * 0.03, world_y as f32 * 0.03)
                        as f64;
                    if virus_value > config.virus_threshold as f64 {
                        chunk.set_material(local_x as usize, local_y as usize, MaterialId::VIRUS);
                    }
                }
            }

            // Place mercury pools at low points
            if current == MaterialId::AIR {
                let below_y = local_y - 1;
                let is_floor = if below_y >= 0 {
                    chunk.get_material(local_x as usize, below_y as usize) != MaterialId::AIR
                } else {
                    scanner.get_material(world_x, world_y - 1) != MaterialId::AIR
                };

                if is_floor {
                    let mercury_value = mercury_noise
                        .get_noise_2d(world_x as f32 * 0.01, world_y as f32 * 0.01)
                        as f64;
                    if mercury_value > config.mercury_threshold as f64 {
                        chunk.set_material(local_x as usize, local_y as usize, MaterialId::MERCURY);
                    }
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
        // Scaled 3× for larger world (was 3, 12, 16, 3, 5)
        assert_eq!(config.min_length, 9);
        assert_eq!(config.max_length, 36);
        assert_eq!(config.spacing, 48);
        assert_eq!(config.base_width, 5);
        assert_eq!(config.min_air_below, 15);
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

    #[test]
    fn test_zone_feature_configs_default() {
        // Verify all new zone configs have sensible defaults
        let wire_config = WireNetworkConfig::default();
        assert!(wire_config.enabled);
        assert_eq!(wire_config.min_depth, -15000);
        assert_eq!(wire_config.max_depth, -22000);

        let thunder_config = ThunderZoneConfig::default();
        assert!(thunder_config.enabled);
        assert_eq!(thunder_config.min_depth, -45000);
        assert_eq!(thunder_config.max_depth, -52000);

        let volatile_config = VolatilePoolConfig::default();
        assert!(volatile_config.enabled);
        assert_eq!(volatile_config.min_depth, -30000);
        assert_eq!(volatile_config.max_depth, -38000);

        let toxic_config = ToxicVentConfig::default();
        assert!(toxic_config.enabled);
        assert_eq!(toxic_config.min_depth, -58000);
        assert_eq!(toxic_config.max_depth, -65000);
    }

    #[test]
    fn test_circuit_ruins_zone_generation() {
        let generator = WorldGenerator::new(42);

        // Generate chunk in Circuit Ruins zone (-15000 to -22000)
        // chunk_y = -280 means world_y = -280 * 64 = -17920 (in zone)
        let chunk = generator.generate_chunk(0, -280);

        // Check zone is correctly identified
        let zone = generator.zone_registry().get_zone_at(-17920);
        assert!(zone.is_some());
        assert_eq!(
            zone.unwrap().zone_type,
            crate::world::biome_zones::UndergroundZone::CircuitRuins
        );

        // Verify determinism (wire/battery/spark counts may vary based on terrain)
        let chunk2 = generator.generate_chunk(0, -280);
        for y in 0..64 {
            for x in 0..64 {
                assert_eq!(
                    chunk.get_material(x, y),
                    chunk2.get_material(x, y),
                    "Circuit ruins generation should be deterministic"
                );
            }
        }
    }

    #[test]
    fn test_volatile_lakes_zone_generation() {
        let generator = WorldGenerator::new(42);

        // Generate chunk in Volatile Lakes zone (-30000 to -38000)
        // chunk_y = -530 means world_y = -530 * 64 = -33920 (in zone)
        let chunk = generator.generate_chunk(0, -530);

        // Check zone is correctly identified
        let zone = generator.zone_registry().get_zone_at(-33920);
        assert!(zone.is_some());
        assert_eq!(
            zone.unwrap().zone_type,
            crate::world::biome_zones::UndergroundZone::VolatileLakes
        );

        // Verify determinism
        let chunk2 = generator.generate_chunk(0, -530);
        for y in 0..64 {
            for x in 0..64 {
                assert_eq!(
                    chunk.get_material(x, y),
                    chunk2.get_material(x, y),
                    "Volatile lakes generation should be deterministic"
                );
            }
        }
    }

    #[test]
    fn test_thunder_caverns_zone_generation() {
        let generator = WorldGenerator::new(42);

        // Generate chunk in Thunder Caverns zone (-45000 to -52000)
        // chunk_y = -750 means world_y = -750 * 64 = -48000 (in zone)
        let chunk = generator.generate_chunk(0, -750);

        // Check zone is correctly identified
        let zone = generator.zone_registry().get_zone_at(-48000);
        assert!(zone.is_some());
        assert_eq!(
            zone.unwrap().zone_type,
            crate::world::biome_zones::UndergroundZone::ThunderCaverns
        );

        // Verify determinism
        let chunk2 = generator.generate_chunk(0, -750);
        for y in 0..64 {
            for x in 0..64 {
                assert_eq!(
                    chunk.get_material(x, y),
                    chunk2.get_material(x, y),
                    "Thunder caverns generation should be deterministic"
                );
            }
        }
    }

    #[test]
    fn test_toxic_depths_zone_generation() {
        let generator = WorldGenerator::new(42);

        // Generate chunk in Toxic Depths zone (-58000 to -65000)
        // chunk_y = -970 means world_y = -970 * 64 = -62080 (in zone)
        let chunk = generator.generate_chunk(0, -970);

        // Check zone is correctly identified
        let zone = generator.zone_registry().get_zone_at(-62080);
        assert!(zone.is_some());
        assert_eq!(
            zone.unwrap().zone_type,
            crate::world::biome_zones::UndergroundZone::ToxicDepths
        );

        // Verify determinism
        let chunk2 = generator.generate_chunk(0, -970);
        for y in 0..64 {
            for x in 0..64 {
                assert_eq!(
                    chunk.get_material(x, y),
                    chunk2.get_material(x, y),
                    "Toxic depths generation should be deterministic"
                );
            }
        }
    }
}
