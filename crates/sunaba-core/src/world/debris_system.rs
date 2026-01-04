//! Debris system - kinematic falling chunks physics

use glam::IVec2;
use std::collections::HashMap;

use super::chunk_manager::ChunkManager;
use crate::simulation::{ChunkRenderData, FallingChunk, FallingChunkSystem, WorldCollisionQuery};

/// Manages falling debris chunks with kinematic physics
pub struct DebrisSystem {
    /// Kinematic falling chunks (simple debris physics, WASM-compatible)
    falling_chunks: FallingChunkSystem,
}

impl DebrisSystem {
    pub fn new() -> Self {
        Self {
            falling_chunks: FallingChunkSystem::new(),
        }
    }

    /// Update falling chunks physics and return settled chunks
    /// Takes self mutably and a WorldCollisionQuery for collision detection
    pub fn update<W: WorldCollisionQuery>(
        &mut self,
        dt: f32,
        world: &W,
    ) -> Vec<FallingChunk> {
        self.falling_chunks.update(dt, world)
    }

    /// Get render data for all falling chunks
    pub fn get_render_data(&self) -> Vec<ChunkRenderData> {
        self.falling_chunks.get_render_data()
    }

    /// Get count of active falling chunks (for debug stats)
    pub fn chunk_count(&self) -> usize {
        self.falling_chunks.chunk_count()
    }

    /// Create falling chunk directly from pixel map
    pub fn create_chunk(&mut self, pixels: HashMap<IVec2, u16>) -> u64 {
        self.falling_chunks.create_chunk(pixels)
    }

    /// Set pixel directly in chunk manager without triggering structural checks
    pub fn set_pixel_direct(
        chunk_manager: &mut ChunkManager,
        world_x: i32,
        world_y: i32,
        material_id: u16,
    ) {
        let (chunk_pos, local_x, local_y) = ChunkManager::world_to_chunk_coords(world_x, world_y);
        if let Some(chunk) = chunk_manager.chunks.get_mut(&chunk_pos) {
            chunk.set_material(local_x, local_y, material_id);
        }
    }

    /// Set pixel directly with success/failure return value
    pub fn set_pixel_direct_checked(
        chunk_manager: &mut ChunkManager,
        world_x: i32,
        world_y: i32,
        material_id: u16,
    ) -> bool {
        let (chunk_pos, local_x, local_y) = ChunkManager::world_to_chunk_coords(world_x, world_y);
        if let Some(chunk) = chunk_manager.chunks.get_mut(&chunk_pos) {
            chunk.set_material(local_x, local_y, material_id);
            true
        } else {
            log::trace!(
                "set_pixel_direct_checked: chunk {:?} not loaded for pixel at ({}, {})",
                chunk_pos,
                world_x,
                world_y
            );
            false
        }
    }
}

impl Default for DebrisSystem {
    fn default() -> Self {
        Self::new()
    }
}
