//! Raycasting utilities for line-of-sight queries

use super::chunk_manager::ChunkManager;
use crate::simulation::{MaterialId, MaterialType, Materials};
use glam::Vec2;

/// Raycasting utilities - stateless methods for line-of-sight queries
pub struct Raycasting;

impl Raycasting {
    /// Simple raycast from position in direction, stopping at first non-air material
    /// Returns (world_x, world_y, material_id) of hit pixel, or None if clear
    ///
    /// This is the simpler version that stops at ANY non-air material
    pub fn raycast(
        chunk_manager: &ChunkManager,
        from: Vec2,
        direction: Vec2,
        max_distance: f32,
    ) -> Option<(i32, i32, u16)> {
        let step = 0.5;
        let mut dist = 0.0;
        while dist < max_distance {
            let pos = from + direction * dist;
            let px = pos.x.round() as i32;
            let py = pos.y.round() as i32;

            let (chunk_pos, local_x, local_y) = ChunkManager::world_to_chunk_coords(px, py);
            if let Some(chunk) = chunk_manager.chunks.get(&chunk_pos) {
                let pixel = chunk.get_pixel(local_x, local_y);
                if pixel.material_id != MaterialId::AIR {
                    return Some((px, py, pixel.material_id));
                }
            }

            dist += step;
        }
        None
    }

    /// Raycast with material type filter (e.g., only solids)
    /// Returns hit position and material ID if matching type found
    ///
    /// Starts raycast from `radius` distance (useful for sensor raycasts from body surface)
    /// Stops at first pixel matching the material_type_filter
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

        // Step along the direction, checking for pixels matching filter
        let step_size = 1.0;
        let mut distance = radius; // Start from edge of body

        while distance < max_distance {
            let check_pos = from + dir * distance;
            let px = check_pos.x as i32;
            let py = check_pos.y as i32;

            let (chunk_pos, local_x, local_y) = ChunkManager::world_to_chunk_coords(px, py);
            if let Some(chunk) = chunk_manager.chunks.get(&chunk_pos) {
                let pixel = chunk.get_pixel(local_x, local_y);
                if !pixel.is_empty() {
                    let material = materials.get(pixel.material_id);
                    if material.material_type == material_type_filter {
                        return Some((px, py, pixel.material_id));
                    }
                }
            }

            distance += step_size;
        }

        None
    }
}
