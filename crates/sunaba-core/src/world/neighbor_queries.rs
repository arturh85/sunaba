//! Neighbor pixel collection utilities

use super::chunk_manager::ChunkManager;

/// Neighbor collection utilities - stateless methods for querying neighboring pixels
pub struct NeighborQueries;

impl NeighborQueries {
    /// Collect all 8 neighboring materials (cardinal + diagonal)
    /// Returns Vec of neighbor material IDs (may be empty if neighbors are air or out of bounds)
    ///
    /// Order: NW, N, NE, W, E, SW, S, SE
    pub fn get_8_neighbors(chunk_manager: &ChunkManager, center_x: i32, center_y: i32) -> Vec<u16> {
        let mut neighbors = Vec::with_capacity(8);

        for (dx, dy) in [
            (-1, -1), // NW
            (0, -1),  // N
            (1, -1),  // NE
            (-1, 0),  // W
            (1, 0),   // E
            (-1, 1),  // SW
            (0, 1),   // S
            (1, 1),   // SE
        ] {
            let x = center_x + dx;
            let y = center_y + dy;

            let (chunk_pos, local_x, local_y) = ChunkManager::world_to_chunk_coords(x, y);
            if let Some(chunk) = chunk_manager.chunks.get(&chunk_pos) {
                let pixel = chunk.get_pixel(local_x, local_y);
                neighbors.push(pixel.material_id);
            }
        }

        neighbors
    }

    /// Iterate over 4 orthogonal neighbors (N, E, S, W)
    /// Calls callback for each neighbor pixel that exists
    ///
    /// Order: S, E, N, W (matches common iteration pattern in reactions)
    pub fn for_each_orthogonal_neighbor<F>(
        chunk_manager: &ChunkManager,
        center_x: i32,
        center_y: i32,
        mut callback: F,
    ) where
        F: FnMut(i32, i32, u16),
    {
        for (dx, dy) in [(0, 1), (1, 0), (0, -1), (-1, 0)] {
            let x = center_x + dx;
            let y = center_y + dy;

            let (chunk_pos, local_x, local_y) = ChunkManager::world_to_chunk_coords(x, y);
            if let Some(chunk) = chunk_manager.chunks.get(&chunk_pos) {
                let pixel = chunk.get_pixel(local_x, local_y);
                callback(x, y, pixel.material_id);
            }
        }
    }

    /// Get pixels in circular radius around center
    /// Returns Vec of (x, y, material_id) for all pixels within radius
    ///
    /// Useful for area effects, spreading, erosion, etc.
    pub fn get_pixels_in_radius(
        chunk_manager: &ChunkManager,
        center_x: i32,
        center_y: i32,
        radius: i32,
    ) -> Vec<(i32, i32, u16)> {
        let mut pixels = Vec::new();

        // Iterate over square containing circle
        for dy in -radius..=radius {
            for dx in -radius..=radius {
                // Check if point is inside circle (Euclidean distance)
                if dx * dx + dy * dy <= radius * radius {
                    let x = center_x + dx;
                    let y = center_y + dy;

                    let (chunk_pos, local_x, local_y) = ChunkManager::world_to_chunk_coords(x, y);
                    if let Some(chunk) = chunk_manager.chunks.get(&chunk_pos) {
                        let pixel = chunk.get_pixel(local_x, local_y);
                        pixels.push((x, y, pixel.material_id));
                    }
                }
            }
        }

        pixels
    }
}
