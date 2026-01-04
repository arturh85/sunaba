//! Read-only pixel property queries

use super::chunk_manager::ChunkManager;
use super::light_system::LightSystem;
use crate::simulation::get_temperature_at_pixel;

/// Pixel property query utilities - stateless methods for querying pixel properties
pub struct PixelQueries;

impl PixelQueries {
    /// Get material ID at world coordinates
    /// Returns None if pixel doesn't exist or is out of bounds
    pub fn get_material(chunk_manager: &ChunkManager, x: i32, y: i32) -> Option<u16> {
        let (chunk_pos, local_x, local_y) = ChunkManager::world_to_chunk_coords(x, y);
        chunk_manager
            .chunks
            .get(&chunk_pos)
            .map(|c| c.get_pixel(local_x, local_y).material_id)
    }

    /// Get temperature at world coordinates (from 8x8 coarse grid)
    /// Returns default ambient temperature (20.0) if chunk not loaded
    pub fn get_temperature(chunk_manager: &ChunkManager, x: i32, y: i32) -> f32 {
        let (chunk_pos, local_x, local_y) = ChunkManager::world_to_chunk_coords(x, y);
        if let Some(chunk) = chunk_manager.chunks.get(&chunk_pos) {
            get_temperature_at_pixel(chunk, local_x, local_y)
        } else {
            20.0 // Default ambient temperature
        }
    }

    /// Get light level at world coordinates (0-15)
    /// Returns None if pixel doesn't exist or is out of bounds
    pub fn get_light(
        light_system: &LightSystem,
        chunk_manager: &ChunkManager,
        x: i32,
        y: i32,
    ) -> Option<u8> {
        light_system.get_light_at(chunk_manager, x, y)
    }

    /// Get pressure at world coordinates (from coarse grid)
    /// Returns default atmospheric pressure (1.0) if chunk not loaded
    pub fn get_pressure(chunk_manager: &ChunkManager, x: i32, y: i32) -> f32 {
        let (chunk_pos, local_x, local_y) = ChunkManager::world_to_chunk_coords(x, y);
        if let Some(chunk) = chunk_manager.chunks.get(&chunk_pos) {
            chunk.get_pressure_at(local_x, local_y)
        } else {
            1.0 // Default atmospheric pressure
        }
    }
}
