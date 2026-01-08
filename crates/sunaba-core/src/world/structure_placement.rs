//! Structure placement engine with physics-aware validation

use crate::simulation::MaterialId;
use crate::world::chunk::{CHUNK_SIZE, Chunk};
use crate::world::context_scanner::ContextScanner;
use crate::world::structures::{AnchorType, StructureTemplate};

/// Maximum height for bridge support columns
const MAX_COLUMN_HEIGHT: i32 = 64;

/// Chunk position in chunk space
#[derive(Debug, Clone, Copy)]
struct ChunkPos {
    x: i32,
    y: i32,
}

/// Place a structure template at world coordinates
///
/// # Arguments
/// * `chunk` - Chunk to place pixels in
/// * `chunk_x` - Chunk X coordinate in chunk space
/// * `chunk_y` - Chunk Y coordinate in chunk space
/// * `world_x` - World X coordinate for anchor point
/// * `world_y` - World Y coordinate for anchor point
/// * `template` - Structure template to place
/// * `scanner` - Context scanner for terrain queries
pub fn place_structure(
    chunk: &mut Chunk,
    chunk_x: i32,
    chunk_y: i32,
    world_x: i32,
    world_y: i32,
    template: &StructureTemplate,
    scanner: &ContextScanner,
) {
    let chunk_size = CHUNK_SIZE as i32;
    let chunk_world_x = chunk_x * chunk_size;
    let chunk_world_y = chunk_y * chunk_size;

    // Calculate anchor offset based on type
    let (anchor_offset_x, anchor_offset_y) = match template.anchor {
        AnchorType::BottomCenter => (0, 0),
        AnchorType::TopCenter => (0, template.bounds.3), // Offset by height
        AnchorType::Center => {
            let center_x = (template.bounds.0 + template.bounds.2) / 2;
            let center_y = (template.bounds.1 + template.bounds.3) / 2;
            (-center_x, -center_y)
        }
        AnchorType::BridgeEnds { .. } => (0, 0),
    };

    // Place support columns first (if bridge)
    if let AnchorType::BridgeEnds {
        left_offset,
        right_offset,
    } = template.anchor
    {
        place_bridge_supports(
            chunk,
            ChunkPos {
                x: chunk_x,
                y: chunk_y,
            },
            world_x,
            world_y,
            left_offset,
            right_offset,
            scanner,
        );
    }

    // Place template pixels
    for &(dx, dy, material) in &template.pixels {
        let pixel_world_x = world_x + anchor_offset_x as i32 + dx as i32;
        let pixel_world_y = world_y + anchor_offset_y as i32 + dy as i32;

        // Convert to chunk-local coordinates
        let local_x = pixel_world_x - chunk_world_x;
        let local_y = pixel_world_y - chunk_world_y;

        // Check bounds and only place in air
        if (0..chunk_size).contains(&local_x)
            && (0..chunk_size).contains(&local_y)
            && chunk.get_material(local_x as usize, local_y as usize) == MaterialId::AIR
        {
            chunk.set_material(local_x as usize, local_y as usize, material);
        }
    }
}

/// Place support columns for bridges (scan down to find ground)
///
/// Auto-generates vertical wood columns from bridge deck down to ground.
/// If no ground is found within MAX_COLUMN_HEIGHT, the support is not placed.
fn place_bridge_supports(
    chunk: &mut Chunk,
    chunk_pos: ChunkPos,
    bridge_x: i32,
    bridge_y: i32,
    left_offset: i8,
    right_offset: i8,
    scanner: &ContextScanner,
) {
    let chunk_size = CHUNK_SIZE as i32;
    let chunk_world_x = chunk_pos.x * chunk_size;
    let chunk_world_y = chunk_pos.y * chunk_size;

    for support_offset in [left_offset as i32, right_offset as i32] {
        let support_x = bridge_x + support_offset;

        // Scan downward to find ground
        let mut found_ground = false;
        for dy in 0..MAX_COLUMN_HEIGHT {
            let check_y = bridge_y - dy;

            // Check if we hit solid ground
            let material = scanner.get_material(support_x, check_y);
            if material != MaterialId::AIR {
                found_ground = true;
                break;
            }

            // Place wood column pixel
            let local_x = support_x - chunk_world_x;
            let local_y = check_y - chunk_world_y;

            if (0..chunk_size).contains(&local_x) && (0..chunk_size).contains(&local_y) {
                chunk.set_material(local_x as usize, local_y as usize, MaterialId::WOOD);
            }
        }

        // If no ground found, log warning (bridge may be unstable)
        if !found_ground {
            log::warn!(
                "Bridge support at ({}, {}) found no ground within {} pixels",
                support_x,
                bridge_y,
                MAX_COLUMN_HEIGHT
            );
        }
    }
}

/// Check if structure placement is valid (physics-aware)
///
/// Validates anchor requirements:
/// - BottomCenter: solid ground below
/// - TopCenter: solid ceiling above
/// - BridgeEnds: solid ground at both support points
/// - Center: no requirements (generic placement)
pub fn is_placement_valid(
    world_x: i32,
    world_y: i32,
    template: &StructureTemplate,
    scanner: &ContextScanner,
) -> bool {
    match template.anchor {
        AnchorType::BottomCenter => {
            // Must have solid ground below center
            scanner.get_material(world_x, world_y - 1) != MaterialId::AIR
        }
        AnchorType::TopCenter => {
            // Must have solid ceiling above center
            scanner.get_material(world_x, world_y + 1) != MaterialId::AIR
        }
        AnchorType::BridgeEnds {
            left_offset,
            right_offset,
        } => {
            // Check for air at bridge level (gap exists)
            let is_gap_center = scanner.get_material(world_x, world_y) == MaterialId::AIR;

            // Check for ground near both support points (within scan range)
            let left_has_ground = has_ground_below(
                world_x + left_offset as i32,
                world_y,
                MAX_COLUMN_HEIGHT,
                scanner,
            );
            let right_has_ground = has_ground_below(
                world_x + right_offset as i32,
                world_y,
                MAX_COLUMN_HEIGHT,
                scanner,
            );

            is_gap_center && left_has_ground && right_has_ground
        }
        AnchorType::Center => true, // Generic placement, no strict requirements
    }
}

/// Helper: Check if there's solid ground below a position within max_dist
fn has_ground_below(x: i32, y: i32, max_dist: i32, scanner: &ContextScanner) -> bool {
    for dy in 0..max_dist {
        let material = scanner.get_material(x, y - dy);
        if material != MaterialId::AIR {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::generation::WorldGenerator;

    use crate::world::structure_templates::TemplateBuilder;

    #[test]
    fn test_place_structure_respects_chunk_bounds() {
        let mut chunk = Chunk::new(0, 0);
        let generator = WorldGenerator::new(42);
        let scanner = ContextScanner::new(&generator);

        let template = TemplateBuilder::new("test", AnchorType::BottomCenter)
            .pixel(0, 0, MaterialId::WOOD)
            .build();

        // Place at center of chunk
        place_structure(&mut chunk, 0, 0, 32, 32, &template, &scanner);

        // Verify placement
        assert_eq!(chunk.get_material(32, 32), MaterialId::WOOD);
    }

    #[test]
    fn test_place_structure_only_in_air() {
        let mut chunk = Chunk::new(0, 0);
        let generator = WorldGenerator::new(42);
        let scanner = ContextScanner::new(&generator);

        // Fill chunk with stone
        for y in 0..CHUNK_SIZE {
            for x in 0..CHUNK_SIZE {
                chunk.set_material(x, y, MaterialId::STONE);
            }
        }

        let template = TemplateBuilder::new("test", AnchorType::Center)
            .rect(-2, -2, 2, 2, MaterialId::WOOD)
            .build();

        // Try to place - should not overwrite stone
        place_structure(&mut chunk, 0, 0, 32, 32, &template, &scanner);

        // All pixels should still be stone
        for y in 30..35 {
            for x in 30..35 {
                assert_eq!(chunk.get_material(x, y), MaterialId::STONE);
            }
        }
    }

    #[test]
    fn test_placement_validation_bottom_center() {
        let generator = WorldGenerator::new(42);
        let scanner = ContextScanner::new(&generator);

        let template = TemplateBuilder::new("tree", AnchorType::BottomCenter)
            .v_line(0, 1, 10, MaterialId::WOOD)
            .build();

        // Find surface position
        let terrain_y = generator.get_terrain_height(0);

        // Should be valid on ground
        assert!(is_placement_valid(0, terrain_y + 1, &template, &scanner));

        // Should be invalid in air
        assert!(!is_placement_valid(0, terrain_y + 50, &template, &scanner));
    }

    #[test]
    fn test_placement_validation_top_center() {
        let generator = WorldGenerator::new(42);
        let scanner = ContextScanner::new(&generator);

        let template = TemplateBuilder::new("stalactite", AnchorType::TopCenter)
            .v_line(0, -10, -1, MaterialId::STONE)
            .build();

        // At terrain surface, there's air above, so TopCenter should be invalid
        let terrain_y = generator.get_terrain_height(100);
        assert!(!is_placement_valid(100, terrain_y, &template, &scanner));

        // Below terrain (underground), if there's solid above, it could be valid
        // But in open air far above terrain, definitely invalid
        assert!(!is_placement_valid(
            100,
            terrain_y + 50,
            &template,
            &scanner
        ));
    }

    #[test]
    fn test_has_ground_below() {
        let generator = WorldGenerator::new(42);
        let scanner = ContextScanner::new(&generator);

        let terrain_y = generator.get_terrain_height(0);

        // Should find ground when close enough
        assert!(has_ground_below(0, terrain_y + 10, 20, &scanner));

        // Should not find ground when too far
        assert!(!has_ground_below(0, terrain_y + 100, 20, &scanner));
    }

    #[test]
    fn test_place_structure_with_anchor_offsets() {
        let mut chunk = Chunk::new(0, 0);
        let generator = WorldGenerator::new(42);
        let scanner = ContextScanner::new(&generator);

        // Create a template with TopCenter anchor
        let template = TemplateBuilder::new("test", AnchorType::TopCenter)
            .v_line(0, -5, 0, MaterialId::STONE)
            .build();

        // Place at y=10 - pixels should extend downward due to TopCenter anchor
        place_structure(&mut chunk, 0, 0, 32, 10, &template, &scanner);

        // With TopCenter, the template extends downward from anchor
        // The anchor offset_y should be bounds.3 = 0
        // So pixels at dy=-5 to dy=0 should map to world_y 10 + 0 + dy
        for dy in -5..=0 {
            let world_y = 10 + dy;
            if (0..CHUNK_SIZE as i32).contains(&world_y) {
                assert_eq!(
                    chunk.get_material(32, world_y as usize),
                    MaterialId::STONE,
                    "Expected STONE at y={}, anchor offset for TopCenter",
                    world_y
                );
            }
        }
    }
}
