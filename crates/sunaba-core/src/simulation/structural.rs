//! Structural integrity checking and falling debris conversion
//!
//! Only player-placed structures are subject to structural integrity checks.
//! Natural terrain is never converted to debris. Player-built structures are
//! anchored to natural terrain (touching natural stone/dirt = stable).

use crate::simulation::{MaterialId, MaterialType};
use crate::world::{World, pixel_flags};
use glam::IVec2;
use std::collections::{HashSet, VecDeque};

/// Maximum distance to check for structural support
const MAX_FLOOD_FILL_RADIUS: i32 = 64;

/// Threshold for small vs large debris
const SMALL_DEBRIS_THRESHOLD: usize = 50;

/// System for tracking and processing structural integrity checks
pub struct StructuralIntegritySystem {
    /// Queue of positions that need structural checks (world coordinates)
    check_queue: HashSet<IVec2>,
}

impl StructuralIntegritySystem {
    pub fn new() -> Self {
        Self {
            check_queue: HashSet::new(),
        }
    }

    /// Schedule a structural check at the given world position
    /// This should be called when a structural material is removed
    pub fn schedule_check(&mut self, world_x: i32, world_y: i32) {
        self.check_queue.insert(IVec2::new(world_x, world_y));
    }

    /// Drain the check queue and return all positions
    /// Returns vector of positions that need checking
    pub fn drain_queue(&mut self) -> Vec<IVec2> {
        self.check_queue.drain().collect()
    }

    /// Process queued structural checks for a list of positions
    /// This is a static method to avoid borrow checker issues
    pub fn process_checks(world: &mut World, positions: Vec<IVec2>) -> usize {
        if positions.is_empty() {
            return 0;
        }

        let count = positions.len();
        log::debug!("Processing {} structural checks", count);

        for pos in positions {
            Self::check_position(world, pos.x, pos.y);
        }

        count
    }

    /// Check structural integrity at a specific position
    /// Only checks player-placed structural materials (not natural terrain)
    fn check_position(world: &mut World, world_x: i32, world_y: i32) {
        log::debug!("Structural: Checking position ({}, {})", world_x, world_y);

        // Get the pixel that was removed - check all 4 neighbors
        let neighbors = [
            (world_x, world_y + 1), // Above
            (world_x + 1, world_y), // Right
            (world_x, world_y - 1), // Below
            (world_x - 1, world_y), // Left
        ];

        for (nx, ny) in neighbors {
            if let Some(pixel) = world.get_pixel(nx, ny) {
                if pixel.is_empty() {
                    continue;
                }

                // Only check player-placed structural solids
                let material = world.materials().get(pixel.material_id);
                if !material.structural || material.material_type != MaterialType::Solid {
                    continue;
                }

                // Skip natural terrain (non-player-placed)
                if (pixel.flags & pixel_flags::PLAYER_PLACED) == 0 {
                    continue;
                }

                // Perform flood fill to find connected player-placed region
                let region = Self::flood_fill_structural(world, nx, ny);
                log::debug!(
                    "Structural: Flood fill from ({}, {}): found {} player-placed pixels",
                    nx,
                    ny,
                    region.len()
                );

                if region.is_empty() {
                    continue;
                }

                // Check if region is anchored (connected to natural terrain or bedrock)
                let is_anchored = Self::is_region_anchored(world, &region);
                log::debug!("Structural: Region anchored={}", is_anchored);

                if !is_anchored {
                    // Convert based on size
                    if region.len() < SMALL_DEBRIS_THRESHOLD {
                        log::info!(
                            "Structural: Converting {} pixels to sand particles",
                            region.len()
                        );
                        Self::convert_to_particles(world, region);
                    } else {
                        // Large debris - create rigid body
                        log::info!(
                            "Structural: Converting {} pixels to rigid body",
                            region.len()
                        );
                        Self::convert_to_rigid_body(world, region);
                    }
                }
            }
        }
    }

    /// Flood fill to find all connected player-placed structural solids
    /// Only traverses pixels with PLAYER_PLACED flag set
    /// Returns set of world coordinates
    fn flood_fill_structural(world: &World, start_x: i32, start_y: i32) -> HashSet<IVec2> {
        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();

        let start_pixel = match world.get_pixel(start_x, start_y) {
            Some(p) if !p.is_empty() => p,
            _ => return visited,
        };

        let start_material = world.materials().get(start_pixel.material_id);
        // Only process player-placed structural solids
        if !start_material.structural
            || start_material.material_type != MaterialType::Solid
            || (start_pixel.flags & pixel_flags::PLAYER_PLACED) == 0
        {
            return visited;
        }

        queue.push_back(IVec2::new(start_x, start_y));
        visited.insert(IVec2::new(start_x, start_y));

        let origin = IVec2::new(start_x, start_y);

        while let Some(pos) = queue.pop_front() {
            // Distance limit to prevent runaway flood fills
            if (pos - origin).abs().max_element() > MAX_FLOOD_FILL_RADIUS {
                continue;
            }

            // Check 4-connected neighbors
            let neighbors = [
                IVec2::new(pos.x, pos.y + 1),
                IVec2::new(pos.x + 1, pos.y),
                IVec2::new(pos.x, pos.y - 1),
                IVec2::new(pos.x - 1, pos.y),
            ];

            for neighbor in neighbors {
                if visited.contains(&neighbor) {
                    continue;
                }

                if let Some(pixel) = world.get_pixel(neighbor.x, neighbor.y) {
                    if pixel.is_empty() {
                        continue;
                    }

                    let material = world.materials().get(pixel.material_id);

                    // Only traverse player-placed structural solids
                    if material.structural
                        && material.material_type == MaterialType::Solid
                        && (pixel.flags & pixel_flags::PLAYER_PLACED) != 0
                    {
                        visited.insert(neighbor);
                        queue.push_back(neighbor);
                    }
                }
            }
        }

        visited
    }

    /// Check if any pixel in the region is anchored
    /// A region is anchored if it:
    /// - Contains bedrock, OR
    /// - Is adjacent to natural terrain (non-player-placed structural solid)
    fn is_region_anchored(world: &World, region: &HashSet<IVec2>) -> bool {
        for pos in region {
            // Check if this pixel is bedrock
            if let Some(pixel) = world.get_pixel(pos.x, pos.y)
                && pixel.material_id == MaterialId::BEDROCK
            {
                return true;
            }

            // Check 4-connected neighbors for natural terrain anchoring
            let neighbors = [
                IVec2::new(pos.x, pos.y + 1),
                IVec2::new(pos.x + 1, pos.y),
                IVec2::new(pos.x, pos.y - 1),
                IVec2::new(pos.x - 1, pos.y),
            ];

            for neighbor in neighbors {
                // Skip neighbors that are part of this region (player-placed)
                if region.contains(&neighbor) {
                    continue;
                }

                if let Some(pixel) = world.get_pixel(neighbor.x, neighbor.y) {
                    if pixel.is_empty() {
                        continue;
                    }

                    let material = world.materials().get(pixel.material_id);

                    // Anchored if adjacent to natural (non-player-placed) structural solid
                    if material.structural
                        && material.material_type == MaterialType::Solid
                        && (pixel.flags & pixel_flags::PLAYER_PLACED) == 0
                    {
                        return true;
                    }
                }
            }
        }
        false
    }

    /// Convert small debris to powder particles (sand)
    fn convert_to_particles(world: &mut World, region: HashSet<IVec2>) {
        log::info!("Converting {} pixels to particles", region.len());

        for pos in region {
            // Convert to sand (powder that will fall naturally)
            world.set_pixel(pos.x, pos.y, MaterialId::SAND);
        }
    }

    /// Convert large debris to a falling rigid body
    fn convert_to_rigid_body(world: &mut World, region: HashSet<IVec2>) {
        log::info!("Converting {} pixels to rigid body", region.len());
        world.create_debris(region);
    }
}

impl Default for StructuralIntegritySystem {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::{Pixel, World, pixel_flags};

    /// Helper to create a world with some terrain
    fn make_test_world() -> World {
        let mut world = World::new(false);
        world.ensure_chunks_for_area(-100, -100, 100, 100);
        world
    }

    /// Helper to place player-placed structural material
    fn place_player_structure(world: &mut World, x: i32, y: i32, material_id: u16) {
        let mut pixel = Pixel::new(material_id);
        pixel.flags |= pixel_flags::PLAYER_PLACED;
        world.set_pixel_full(x, y, pixel);
    }

    #[test]
    fn test_new_system_empty_queue() {
        let system = StructuralIntegritySystem::new();
        assert_eq!(system.check_queue.len(), 0);
    }

    #[test]
    fn test_schedule_check() {
        let mut system = StructuralIntegritySystem::new();
        system.schedule_check(10, 20);
        assert_eq!(system.check_queue.len(), 1);
        assert!(system.check_queue.contains(&IVec2::new(10, 20)));
    }

    #[test]
    fn test_schedule_check_deduplicates() {
        let mut system = StructuralIntegritySystem::new();
        system.schedule_check(10, 20);
        system.schedule_check(10, 20);
        system.schedule_check(10, 20);
        assert_eq!(system.check_queue.len(), 1);
    }

    #[test]
    fn test_drain_queue() {
        let mut system = StructuralIntegritySystem::new();
        system.schedule_check(10, 20);
        system.schedule_check(30, 40);

        let positions = system.drain_queue();
        assert_eq!(positions.len(), 2);
        assert_eq!(system.check_queue.len(), 0);
    }

    #[test]
    fn test_flood_fill_single_pixel() {
        let mut world = make_test_world();
        place_player_structure(&mut world, 50, 50, MaterialId::STONE);

        let region = StructuralIntegritySystem::flood_fill_structural(&world, 50, 50);
        assert_eq!(region.len(), 1);
        assert!(region.contains(&IVec2::new(50, 50)));
    }

    #[test]
    fn test_flood_fill_connected_structure() {
        let mut world = make_test_world();

        // Create a 3x3 player-placed structure
        for y in 50..53 {
            for x in 50..53 {
                place_player_structure(&mut world, x, y, MaterialId::STONE);
            }
        }

        let region = StructuralIntegritySystem::flood_fill_structural(&world, 51, 51);
        assert_eq!(region.len(), 9); // 3x3 = 9 pixels
    }

    #[test]
    fn test_flood_fill_ignores_natural_terrain() {
        let mut world = make_test_world();

        // Place player-placed stone
        place_player_structure(&mut world, 50, 50, MaterialId::STONE);

        // Place natural stone (no PLAYER_PLACED flag) next to it
        world.set_pixel(51, 50, MaterialId::STONE);
        // Don't set PLAYER_PLACED flag

        let region = StructuralIntegritySystem::flood_fill_structural(&world, 50, 50);
        // Should only include the player-placed pixel
        assert_eq!(region.len(), 1);
        assert!(region.contains(&IVec2::new(50, 50)));
        assert!(!region.contains(&IVec2::new(51, 50)));
    }

    #[test]
    fn test_flood_fill_respects_max_radius() {
        let mut world = make_test_world();

        // Create a long line of player-placed structures (> MAX_FLOOD_FILL_RADIUS)
        for x in 0..100 {
            place_player_structure(&mut world, x, 50, MaterialId::STONE);
        }

        let region = StructuralIntegritySystem::flood_fill_structural(&world, 0, 50);
        // Should stop at MAX_FLOOD_FILL_RADIUS (64)
        assert!(region.len() <= (MAX_FLOOD_FILL_RADIUS * 2 + 1) as usize);
    }

    #[test]
    fn test_flood_fill_empty_pixel_returns_empty() {
        let world = make_test_world();
        let region = StructuralIntegritySystem::flood_fill_structural(&world, 50, 50);
        assert_eq!(region.len(), 0);
    }

    #[test]
    fn test_flood_fill_non_structural_returns_empty() {
        let mut world = make_test_world();
        // Sand is not structural
        place_player_structure(&mut world, 50, 50, MaterialId::SAND);

        let region = StructuralIntegritySystem::flood_fill_structural(&world, 50, 50);
        assert_eq!(region.len(), 0);
    }

    #[test]
    fn test_is_region_anchored_to_bedrock() {
        let mut world = make_test_world();

        // Create player-placed structure containing bedrock
        place_player_structure(&mut world, 50, 50, MaterialId::BEDROCK);

        let mut region = HashSet::new();
        region.insert(IVec2::new(50, 50));

        assert!(StructuralIntegritySystem::is_region_anchored(
            &world, &region
        ));
    }

    #[test]
    fn test_is_region_anchored_to_natural_terrain() {
        let mut world = make_test_world();

        // Place natural stone
        world.set_pixel(50, 49, MaterialId::STONE);

        // Place player structure adjacent to it
        place_player_structure(&mut world, 50, 50, MaterialId::STONE);

        let mut region = HashSet::new();
        region.insert(IVec2::new(50, 50));

        // Should be anchored because it's touching natural terrain
        assert!(StructuralIntegritySystem::is_region_anchored(
            &world, &region
        ));
    }

    #[test]
    fn test_is_region_not_anchored_floating() {
        let mut world = make_test_world();

        // Create floating player-placed structure (no adjacent natural terrain)
        place_player_structure(&mut world, 50, 50, MaterialId::STONE);

        let mut region = HashSet::new();
        region.insert(IVec2::new(50, 50));

        // Should not be anchored (floating in air)
        assert!(!StructuralIntegritySystem::is_region_anchored(
            &world, &region
        ));
    }

    #[test]
    fn test_is_region_not_anchored_only_player_placed() {
        let mut world = make_test_world();

        // Create two connected player-placed structures
        place_player_structure(&mut world, 50, 50, MaterialId::STONE);
        place_player_structure(&mut world, 51, 50, MaterialId::STONE);

        let mut region = HashSet::new();
        region.insert(IVec2::new(50, 50));
        region.insert(IVec2::new(51, 50));

        // Should not be anchored (only connected to each other)
        assert!(!StructuralIntegritySystem::is_region_anchored(
            &world, &region
        ));
    }

    #[test]
    fn test_convert_to_particles_small_debris() {
        let mut world = make_test_world();

        // Create small structure (< SMALL_DEBRIS_THRESHOLD)
        for x in 50..53 {
            place_player_structure(&mut world, x, 50, MaterialId::STONE);
        }

        let mut region = HashSet::new();
        for x in 50..53 {
            region.insert(IVec2::new(x, 50));
        }

        StructuralIntegritySystem::convert_to_particles(&mut world, region);

        // Should be converted to sand
        for x in 50..53 {
            let pixel = world.get_pixel(x, 50).unwrap();
            assert_eq!(pixel.material_id, MaterialId::SAND);
        }
    }

    #[test]
    fn test_process_checks_empty_queue() {
        let mut world = make_test_world();
        let positions = vec![];

        let count = StructuralIntegritySystem::process_checks(&mut world, positions);
        assert_eq!(count, 0);
    }

    #[test]
    fn test_process_checks_converts_unanchored() {
        let mut world = make_test_world();

        // Create floating player structure
        place_player_structure(&mut world, 50, 51, MaterialId::STONE);

        // Remove the pixel below it (trigger structural check)
        world.set_pixel(50, 50, MaterialId::AIR);

        // Process structural check at the removed position
        let positions = vec![IVec2::new(50, 50)];
        StructuralIntegritySystem::process_checks(&mut world, positions);

        // The floating structure should be converted to sand (small debris)
        let pixel = world.get_pixel(50, 51).unwrap();
        assert_eq!(pixel.material_id, MaterialId::SAND);
    }

    #[test]
    fn test_process_checks_preserves_anchored() {
        let mut world = make_test_world();

        // Place natural terrain
        world.set_pixel(50, 50, MaterialId::STONE);

        // Place player structure on top of natural terrain
        place_player_structure(&mut world, 50, 51, MaterialId::STONE);

        // Trigger structural check
        let positions = vec![IVec2::new(49, 51)];
        StructuralIntegritySystem::process_checks(&mut world, positions);

        // Should remain as stone (anchored to natural terrain below)
        let pixel = world.get_pixel(50, 51).unwrap();
        assert_eq!(pixel.material_id, MaterialId::STONE);
        assert!(pixel.flags & pixel_flags::PLAYER_PLACED != 0);
    }

    #[test]
    fn test_check_position_ignores_non_structural() {
        let mut world = make_test_world();

        // Place sand (non-structural) next to air
        place_player_structure(&mut world, 51, 50, MaterialId::SAND);

        // Check the air position
        let positions = vec![IVec2::new(50, 50)];
        StructuralIntegritySystem::process_checks(&mut world, positions);

        // Sand should remain unchanged (not subject to structural checks)
        let pixel = world.get_pixel(51, 50).unwrap();
        assert_eq!(pixel.material_id, MaterialId::SAND);
    }

    #[test]
    fn test_default_creates_empty_system() {
        let system = StructuralIntegritySystem::default();
        assert_eq!(system.check_queue.len(), 0);
    }
}
