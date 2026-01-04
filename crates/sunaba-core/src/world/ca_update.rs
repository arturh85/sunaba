//! Cellular automata update logic - material movement physics

use glam::IVec2;
use std::collections::HashMap;

use super::{CHUNK_SIZE, Chunk};
use crate::simulation::{MaterialType, Materials};
use crate::world::{SimStats, WorldRng};

/// Cellular automata updater - handles material movement physics
pub struct CellularAutomataUpdater;

impl CellularAutomataUpdater {
    /// Update powder material (falls down, disperses diagonally)
    pub fn update_powder<R: WorldRng>(
        chunks: &mut HashMap<IVec2, Chunk>,
        chunk_pos: IVec2,
        x: usize,
        y: usize,
        materials: &Materials,
        stats: &mut dyn SimStats,
        rng: &mut R,
    ) {
        let world_x = chunk_pos.x * CHUNK_SIZE as i32 + x as i32;
        let world_y = chunk_pos.y * CHUNK_SIZE as i32 + y as i32;

        // Try to move down
        if Self::try_move(
            chunks,
            world_x,
            world_y,
            world_x,
            world_y - 1,
            materials,
            stats,
        ) {
            return;
        }

        // Try diagonal dispersal (random direction)
        let dx = if rng.gen_bool() { -1 } else { 1 };
        if Self::try_move(
            chunks,
            world_x,
            world_y,
            world_x + dx,
            world_y - 1,
            materials,
            stats,
        ) {
            return;
        }

        // Try opposite diagonal
        if Self::try_move(
            chunks,
            world_x,
            world_y,
            world_x - dx,
            world_y - 1,
            materials,
            stats,
        ) {}
    }

    /// Update liquid material (flows horizontally and down)
    pub fn update_liquid<R: WorldRng>(
        chunks: &mut HashMap<IVec2, Chunk>,
        chunk_pos: IVec2,
        x: usize,
        y: usize,
        materials: &Materials,
        stats: &mut dyn SimStats,
        rng: &mut R,
    ) {
        let world_x = chunk_pos.x * CHUNK_SIZE as i32 + x as i32;
        let world_y = chunk_pos.y * CHUNK_SIZE as i32 + y as i32;

        // Try to move down first
        if Self::try_move(
            chunks,
            world_x,
            world_y,
            world_x,
            world_y - 1,
            materials,
            stats,
        ) {
            return;
        }

        // Try to flow horizontally (random direction first)
        let dx = if rng.gen_bool() { -1 } else { 1 };
        if Self::try_move(
            chunks,
            world_x,
            world_y,
            world_x + dx,
            world_y,
            materials,
            stats,
        ) {
            return;
        }

        // Try opposite direction
        if Self::try_move(
            chunks,
            world_x,
            world_y,
            world_x - dx,
            world_y,
            materials,
            stats,
        ) {
            return;
        }

        // Try diagonal down (for flowing over obstacles)
        if Self::try_move(
            chunks,
            world_x,
            world_y,
            world_x + dx,
            world_y - 1,
            materials,
            stats,
        ) {
            return;
        }

        if Self::try_move(
            chunks,
            world_x,
            world_y,
            world_x - dx,
            world_y - 1,
            materials,
            stats,
        ) {}
    }

    /// Update gas material (rises up, disperses)
    pub fn update_gas<R: WorldRng>(
        chunks: &mut HashMap<IVec2, Chunk>,
        chunk_pos: IVec2,
        x: usize,
        y: usize,
        materials: &Materials,
        stats: &mut dyn SimStats,
        rng: &mut R,
    ) {
        let world_x = chunk_pos.x * CHUNK_SIZE as i32 + x as i32;
        let world_y = chunk_pos.y * CHUNK_SIZE as i32 + y as i32;

        // Try to move up
        if Self::try_move(
            chunks,
            world_x,
            world_y,
            world_x,
            world_y + 1,
            materials,
            stats,
        ) {
            return;
        }

        // Try diagonal up (random direction)
        let dx = if rng.gen_bool() { -1 } else { 1 };
        if Self::try_move(
            chunks,
            world_x,
            world_y,
            world_x + dx,
            world_y + 1,
            materials,
            stats,
        ) {
            return;
        }

        // Try opposite diagonal
        if Self::try_move(
            chunks,
            world_x,
            world_y,
            world_x - dx,
            world_y + 1,
            materials,
            stats,
        ) {
            return;
        }

        // Try horizontal dispersal
        if Self::try_move(
            chunks,
            world_x,
            world_y,
            world_x + dx,
            world_y,
            materials,
            stats,
        ) {
            return;
        }

        if Self::try_move(
            chunks,
            world_x,
            world_y,
            world_x - dx,
            world_y,
            materials,
            stats,
        ) {}
    }

    /// Try to move a pixel from one position to another
    /// Returns true if the move succeeded
    fn try_move(
        chunks: &mut HashMap<IVec2, Chunk>,
        from_x: i32,
        from_y: i32,
        to_x: i32,
        to_y: i32,
        materials: &Materials,
        stats: &mut dyn SimStats,
    ) -> bool {
        use crate::world::chunk_manager::ChunkManager;

        let (src_chunk_pos, src_local_x, src_local_y) =
            ChunkManager::world_to_chunk_coords(from_x, from_y);
        let (dst_chunk_pos, dst_local_x, dst_local_y) =
            ChunkManager::world_to_chunk_coords(to_x, to_y);

        // Get source pixel
        let src_pixel = match chunks.get(&src_chunk_pos) {
            Some(chunk) => chunk.get_pixel(src_local_x, src_local_y),
            None => return false,
        };

        // Get destination pixel
        let dst_pixel = match chunks.get(&dst_chunk_pos) {
            Some(chunk) => chunk.get_pixel(dst_local_x, dst_local_y),
            None => return false,
        };

        // Check if move is valid (can only move into empty space or displace lighter materials)
        let src_material = materials.get(src_pixel.material_id);
        let dst_material = materials.get(dst_pixel.material_id);

        // Can't move into solid
        if dst_material.material_type == MaterialType::Solid {
            return false;
        }

        // Can only move into lighter material or empty space
        if !dst_pixel.is_empty() && dst_material.density >= src_material.density {
            return false;
        }

        // Perform the swap (same chunk or cross-chunk)
        if src_chunk_pos == dst_chunk_pos {
            // Same chunk - simple swap
            if let Some(chunk) = chunks.get_mut(&src_chunk_pos) {
                chunk.swap_pixels(src_local_x, src_local_y, dst_local_x, dst_local_y);
                chunk.set_simulation_active(true);
                stats.record_pixel_moved();
                return true;
            }
        } else {
            // Cross-chunk swap - need to handle carefully
            // First, copy the pixels
            let src_copy = src_pixel;
            let dst_copy = dst_pixel;

            // Update source chunk
            if let Some(src_chunk) = chunks.get_mut(&src_chunk_pos) {
                src_chunk.set_pixel(src_local_x, src_local_y, dst_copy);
                src_chunk.set_simulation_active(true);
            } else {
                return false;
            }

            // Update destination chunk
            if let Some(dst_chunk) = chunks.get_mut(&dst_chunk_pos) {
                dst_chunk.set_pixel(dst_local_x, dst_local_y, src_copy);
                dst_chunk.set_simulation_active(true);
            } else {
                // Rollback source chunk change
                if let Some(src_chunk) = chunks.get_mut(&src_chunk_pos) {
                    src_chunk.set_pixel(src_local_x, src_local_y, src_copy);
                }
                return false;
            }

            stats.record_pixel_moved();
            return true;
        }

        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::simulation::{MaterialId, Materials};
    use crate::world::{NoopStats, WorldRng};
    use std::collections::HashMap;

    /// Test RNG that returns deterministic values
    struct TestRng {
        bool_value: bool,
    }

    impl TestRng {
        fn new(bool_value: bool) -> Self {
            Self { bool_value }
        }
    }

    impl WorldRng for TestRng {
        fn gen_bool(&mut self) -> bool {
            self.bool_value
        }

        fn gen_f32(&mut self) -> f32 {
            0.5
        }

        fn check_probability(&mut self, _probability: f32) -> bool {
            true
        }
    }

    /// Helper to create a chunk with a pixel at a specific location
    fn make_chunk_with_pixel(
        chunk_x: i32,
        chunk_y: i32,
        local_x: usize,
        local_y: usize,
        material_id: u16,
    ) -> Chunk {
        let mut chunk = Chunk::new(chunk_x, chunk_y);
        chunk.set_material(local_x, local_y, material_id);
        chunk
    }

    /// Helper to create chunks HashMap
    fn make_chunks(chunks: Vec<(IVec2, Chunk)>) -> HashMap<IVec2, Chunk> {
        chunks.into_iter().collect()
    }

    #[test]
    fn test_update_powder_falls_down() {
        let materials = Materials::new();
        let mut chunks = make_chunks(vec![
            (
                IVec2::new(0, 0),
                make_chunk_with_pixel(0, 0, 32, 10, MaterialId::SAND),
            ),
            (IVec2::new(0, 0), Chunk::new(0, 0)), // Empty chunk below
        ]);

        // Set sand at (32, 10)
        chunks
            .get_mut(&IVec2::new(0, 0))
            .unwrap()
            .set_material(32, 10, MaterialId::SAND);

        let mut stats = NoopStats;
        let mut rng = TestRng::new(true);

        // Update powder - should fall down
        CellularAutomataUpdater::update_powder(
            &mut chunks,
            IVec2::new(0, 0),
            32,
            10,
            &materials,
            &mut stats,
            &mut rng,
        );

        // Check sand moved down
        let chunk = chunks.get(&IVec2::new(0, 0)).unwrap();
        assert_eq!(chunk.get_pixel(32, 10).material_id, MaterialId::AIR);
        assert_eq!(chunk.get_pixel(32, 9).material_id, MaterialId::SAND);
    }

    #[test]
    fn test_update_powder_stops_on_solid() {
        let materials = Materials::new();
        let mut chunks = make_chunks(vec![(IVec2::new(0, 0), Chunk::new(0, 0))]);

        let chunk = chunks.get_mut(&IVec2::new(0, 0)).unwrap();
        chunk.set_material(32, 10, MaterialId::SAND);
        chunk.set_material(32, 9, MaterialId::STONE); // Solid below
        chunk.set_material(31, 9, MaterialId::STONE); // Block diagonal left
        chunk.set_material(33, 9, MaterialId::STONE); // Block diagonal right

        let mut stats = NoopStats;
        let mut rng = TestRng::new(true);

        CellularAutomataUpdater::update_powder(
            &mut chunks,
            IVec2::new(0, 0),
            32,
            10,
            &materials,
            &mut stats,
            &mut rng,
        );

        // Sand should not move (completely blocked)
        let chunk = chunks.get(&IVec2::new(0, 0)).unwrap();
        assert_eq!(chunk.get_pixel(32, 10).material_id, MaterialId::SAND);
        assert_eq!(chunk.get_pixel(32, 9).material_id, MaterialId::STONE);
    }

    #[test]
    fn test_update_powder_slides_diagonally() {
        let materials = Materials::new();
        let mut chunks = make_chunks(vec![(IVec2::new(0, 0), Chunk::new(0, 0))]);

        let chunk = chunks.get_mut(&IVec2::new(0, 0)).unwrap();
        chunk.set_material(32, 10, MaterialId::SAND);
        chunk.set_material(32, 9, MaterialId::STONE); // Block straight down
        // Leave diagonal spots (31,9) and (33,9) empty

        let mut stats = NoopStats;
        let mut rng = TestRng::new(true); // Will try left diagonal first

        CellularAutomataUpdater::update_powder(
            &mut chunks,
            IVec2::new(0, 0),
            32,
            10,
            &materials,
            &mut stats,
            &mut rng,
        );

        // Sand should slide diagonally
        let chunk = chunks.get(&IVec2::new(0, 0)).unwrap();
        assert_eq!(chunk.get_pixel(32, 10).material_id, MaterialId::AIR);
        // Should be at (31, 9) since rng returns true (try left first)
        assert_eq!(chunk.get_pixel(31, 9).material_id, MaterialId::SAND);
    }

    #[test]
    fn test_update_liquid_falls_down() {
        let materials = Materials::new();
        let mut chunks = make_chunks(vec![(IVec2::new(0, 0), Chunk::new(0, 0))]);

        chunks
            .get_mut(&IVec2::new(0, 0))
            .unwrap()
            .set_material(32, 10, MaterialId::WATER);

        let mut stats = NoopStats;
        let mut rng = TestRng::new(true);

        CellularAutomataUpdater::update_liquid(
            &mut chunks,
            IVec2::new(0, 0),
            32,
            10,
            &materials,
            &mut stats,
            &mut rng,
        );

        // Water should fall down
        let chunk = chunks.get(&IVec2::new(0, 0)).unwrap();
        assert_eq!(chunk.get_pixel(32, 10).material_id, MaterialId::AIR);
        assert_eq!(chunk.get_pixel(32, 9).material_id, MaterialId::WATER);
    }

    #[test]
    fn test_update_liquid_flows_horizontally() {
        let materials = Materials::new();
        let mut chunks = make_chunks(vec![(IVec2::new(0, 0), Chunk::new(0, 0))]);

        let chunk = chunks.get_mut(&IVec2::new(0, 0)).unwrap();
        chunk.set_material(32, 10, MaterialId::WATER);
        chunk.set_material(32, 9, MaterialId::STONE); // Block down
        chunk.set_material(31, 9, MaterialId::STONE); // Block diagonal left
        chunk.set_material(33, 9, MaterialId::STONE); // Block diagonal right
        // Leave horizontal (31, 10) empty

        let mut stats = NoopStats;
        let mut rng = TestRng::new(true); // Try left first

        CellularAutomataUpdater::update_liquid(
            &mut chunks,
            IVec2::new(0, 0),
            32,
            10,
            &materials,
            &mut stats,
            &mut rng,
        );

        // Water should flow horizontally
        let chunk = chunks.get(&IVec2::new(0, 0)).unwrap();
        assert_eq!(chunk.get_pixel(32, 10).material_id, MaterialId::AIR);
        assert_eq!(chunk.get_pixel(31, 10).material_id, MaterialId::WATER);
    }

    #[test]
    fn test_update_gas_rises_up() {
        let materials = Materials::new();
        let mut chunks = make_chunks(vec![(IVec2::new(0, 0), Chunk::new(0, 0))]);

        chunks
            .get_mut(&IVec2::new(0, 0))
            .unwrap()
            .set_material(32, 10, MaterialId::SMOKE);

        let mut stats = NoopStats;
        let mut rng = TestRng::new(true);

        CellularAutomataUpdater::update_gas(
            &mut chunks,
            IVec2::new(0, 0),
            32,
            10,
            &materials,
            &mut stats,
            &mut rng,
        );

        // Smoke should rise up
        let chunk = chunks.get(&IVec2::new(0, 0)).unwrap();
        assert_eq!(chunk.get_pixel(32, 10).material_id, MaterialId::AIR);
        assert_eq!(chunk.get_pixel(32, 11).material_id, MaterialId::SMOKE);
    }

    #[test]
    fn test_update_gas_disperses_horizontally() {
        let materials = Materials::new();
        let mut chunks = make_chunks(vec![(IVec2::new(0, 0), Chunk::new(0, 0))]);

        let chunk = chunks.get_mut(&IVec2::new(0, 0)).unwrap();
        chunk.set_material(32, 10, MaterialId::SMOKE);
        chunk.set_material(32, 11, MaterialId::STONE); // Block up
        chunk.set_material(31, 11, MaterialId::STONE); // Block diagonal
        chunk.set_material(33, 11, MaterialId::STONE); // Block diagonal

        let mut stats = NoopStats;
        let mut rng = TestRng::new(true);

        CellularAutomataUpdater::update_gas(
            &mut chunks,
            IVec2::new(0, 0),
            32,
            10,
            &materials,
            &mut stats,
            &mut rng,
        );

        // Smoke should disperse horizontally
        let chunk = chunks.get(&IVec2::new(0, 0)).unwrap();
        assert_eq!(chunk.get_pixel(32, 10).material_id, MaterialId::AIR);
        assert_eq!(chunk.get_pixel(31, 10).material_id, MaterialId::SMOKE);
    }

    #[test]
    fn test_try_move_same_chunk() {
        let materials = Materials::new();
        let mut chunks = make_chunks(vec![(IVec2::new(0, 0), Chunk::new(0, 0))]);

        chunks
            .get_mut(&IVec2::new(0, 0))
            .unwrap()
            .set_material(32, 10, MaterialId::SAND);

        let mut stats = NoopStats;

        let result = CellularAutomataUpdater::try_move(
            &mut chunks,
            32,
            10, // from
            32,
            9, // to (down)
            &materials,
            &mut stats,
        );

        assert!(result);
        let chunk = chunks.get(&IVec2::new(0, 0)).unwrap();
        assert_eq!(chunk.get_pixel(32, 10).material_id, MaterialId::AIR);
        assert_eq!(chunk.get_pixel(32, 9).material_id, MaterialId::SAND);
    }

    #[test]
    fn test_try_move_blocked_by_solid() {
        let materials = Materials::new();
        let mut chunks = make_chunks(vec![(IVec2::new(0, 0), Chunk::new(0, 0))]);

        let chunk = chunks.get_mut(&IVec2::new(0, 0)).unwrap();
        chunk.set_material(32, 10, MaterialId::SAND);
        chunk.set_material(32, 9, MaterialId::STONE);

        let mut stats = NoopStats;

        let result =
            CellularAutomataUpdater::try_move(&mut chunks, 32, 10, 32, 9, &materials, &mut stats);

        assert!(!result);
        let chunk = chunks.get(&IVec2::new(0, 0)).unwrap();
        assert_eq!(chunk.get_pixel(32, 10).material_id, MaterialId::SAND);
        assert_eq!(chunk.get_pixel(32, 9).material_id, MaterialId::STONE);
    }

    #[test]
    fn test_try_move_cross_chunk() {
        let materials = Materials::new();
        let mut chunks = make_chunks(vec![
            (IVec2::new(0, 0), Chunk::new(0, 0)),
            (IVec2::new(0, -1), Chunk::new(0, -1)),
        ]);

        // Place sand at bottom of chunk (0,0) at y=0
        chunks
            .get_mut(&IVec2::new(0, 0))
            .unwrap()
            .set_material(32, 0, MaterialId::SAND);

        let mut stats = NoopStats;

        // Try to move from chunk (0,0) to chunk (0,-1)
        // World coords: (32, 0) -> (32, -1)
        let result = CellularAutomataUpdater::try_move(
            &mut chunks,
            32,
            0, // from (chunk 0,0, local 32,0)
            32,
            -1, // to (chunk 0,-1, local 32,63)
            &materials,
            &mut stats,
        );

        assert!(result);

        // Check sand moved to other chunk
        let chunk_src = chunks.get(&IVec2::new(0, 0)).unwrap();
        assert_eq!(chunk_src.get_pixel(32, 0).material_id, MaterialId::AIR);

        let chunk_dst = chunks.get(&IVec2::new(0, -1)).unwrap();
        assert_eq!(chunk_dst.get_pixel(32, 63).material_id, MaterialId::SAND);
    }

    #[test]
    fn test_try_move_cross_chunk_horizontal() {
        let materials = Materials::new();
        let mut chunks = make_chunks(vec![
            (IVec2::new(0, 0), Chunk::new(0, 0)),
            (IVec2::new(1, 0), Chunk::new(1, 0)),
        ]);

        // Place water at right edge of chunk (0,0)
        chunks
            .get_mut(&IVec2::new(0, 0))
            .unwrap()
            .set_material(63, 32, MaterialId::WATER);

        let mut stats = NoopStats;

        // Move from chunk (0,0) to chunk (1,0)
        let result = CellularAutomataUpdater::try_move(
            &mut chunks,
            63,
            32, // from (chunk 0,0, local 63,32)
            64,
            32, // to (chunk 1,0, local 0,32)
            &materials,
            &mut stats,
        );

        assert!(result);

        let chunk_src = chunks.get(&IVec2::new(0, 0)).unwrap();
        assert_eq!(chunk_src.get_pixel(63, 32).material_id, MaterialId::AIR);

        let chunk_dst = chunks.get(&IVec2::new(1, 0)).unwrap();
        assert_eq!(chunk_dst.get_pixel(0, 32).material_id, MaterialId::WATER);
    }

    #[test]
    fn test_try_move_respects_density() {
        let materials = Materials::new();
        let mut chunks = make_chunks(vec![(IVec2::new(0, 0), Chunk::new(0, 0))]);

        let chunk = chunks.get_mut(&IVec2::new(0, 0)).unwrap();
        chunk.set_material(32, 10, MaterialId::WATER);
        chunk.set_material(32, 9, MaterialId::SAND); // Denser than water

        let mut stats = NoopStats;

        // Water trying to move into sand should fail (sand is denser)
        let result =
            CellularAutomataUpdater::try_move(&mut chunks, 32, 10, 32, 9, &materials, &mut stats);

        assert!(!result);
        let chunk = chunks.get(&IVec2::new(0, 0)).unwrap();
        assert_eq!(chunk.get_pixel(32, 10).material_id, MaterialId::WATER);
        assert_eq!(chunk.get_pixel(32, 9).material_id, MaterialId::SAND);
    }

    #[test]
    fn test_try_move_missing_source_chunk() {
        let materials = Materials::new();
        let mut chunks: HashMap<IVec2, Chunk> = HashMap::new();
        let mut stats = NoopStats;

        let result =
            CellularAutomataUpdater::try_move(&mut chunks, 32, 10, 32, 9, &materials, &mut stats);

        assert!(!result);
    }

    #[test]
    fn test_try_move_missing_destination_chunk() {
        let materials = Materials::new();
        let mut chunks = make_chunks(vec![(IVec2::new(0, 0), Chunk::new(0, 0))]);
        chunks
            .get_mut(&IVec2::new(0, 0))
            .unwrap()
            .set_material(32, 10, MaterialId::SAND);

        let mut stats = NoopStats;

        // Try to move to unloaded chunk
        let result = CellularAutomataUpdater::try_move(
            &mut chunks,
            32,
            10, // chunk (0,0)
            1000,
            10, // chunk (15,0) - not loaded
            &materials,
            &mut stats,
        );

        assert!(!result);
    }
}
