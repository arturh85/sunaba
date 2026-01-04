//! Raycasting utilities for line-of-sight queries

use super::chunk_manager::ChunkManager;
use crate::simulation::{MaterialId, MaterialType, Materials};
use bresenham::Bresenham;
use glam::Vec2;

/// Raycasting utilities - stateless methods for line-of-sight queries
pub struct Raycasting;

impl Raycasting {
    /// Simple raycast from position in direction, stopping at first non-air material
    /// Returns (world_x, world_y, material_id) of hit pixel, or None if clear
    ///
    /// Uses Bresenham line algorithm for exact pixel traversal
    pub fn raycast(
        chunk_manager: &ChunkManager,
        from: Vec2,
        direction: Vec2,
        max_distance: f32,
    ) -> Option<(i32, i32, u16)> {
        // Calculate start and end points for Bresenham (uses isize)
        let from_i = (from.x.round() as isize, from.y.round() as isize);
        let to = from + direction.normalize_or_zero() * max_distance;
        let to_i = (to.x.round() as isize, to.y.round() as isize);

        // Use Bresenham line algorithm for exact pixel traversal
        for (x, y) in Bresenham::new(from_i, to_i) {
            let (chunk_pos, local_x, local_y) =
                ChunkManager::world_to_chunk_coords(x as i32, y as i32);
            if let Some(chunk) = chunk_manager.chunks.get(&chunk_pos) {
                let pixel = chunk.get_pixel(local_x, local_y);
                if pixel.material_id != MaterialId::AIR {
                    return Some((x as i32, y as i32, pixel.material_id));
                }
            }
        }
        None
    }

    /// Raycast with material type filter (e.g., only solids)
    /// Returns hit position and material ID if matching type found
    ///
    /// Starts raycast from `radius` distance (useful for sensor raycasts from body surface)
    /// Stops at first pixel matching the material_type_filter
    /// Uses Bresenham line algorithm for exact pixel traversal
    pub fn raycast_filtered(
        chunk_manager: &ChunkManager,
        materials: &Materials,
        from: Vec2,
        direction: Vec2,
        radius: f32,
        max_distance: f32,
        material_type_filter: MaterialType,
    ) -> Option<(i32, i32, u16)> {
        // Normalize direction
        let dir = direction.normalize_or_zero();
        if dir == Vec2::ZERO {
            return None;
        }

        // Calculate start and end points for Bresenham (uses isize)
        let start = from + dir * radius; // Start from edge of body
        let end = from + dir * max_distance;
        let start_i = (start.x.round() as isize, start.y.round() as isize);
        let end_i = (end.x.round() as isize, end.y.round() as isize);

        // Use Bresenham line algorithm for exact pixel traversal
        for (x, y) in Bresenham::new(start_i, end_i) {
            let (chunk_pos, local_x, local_y) =
                ChunkManager::world_to_chunk_coords(x as i32, y as i32);
            if let Some(chunk) = chunk_manager.chunks.get(&chunk_pos) {
                let pixel = chunk.get_pixel(local_x, local_y);
                if !pixel.is_empty() {
                    let material = materials.get(pixel.material_id);
                    if material.material_type == material_type_filter {
                        return Some((x as i32, y as i32, pixel.material_id));
                    }
                }
            }
        }

        None
    }
}
