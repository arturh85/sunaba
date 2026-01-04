//! Chunk status and management utilities

use glam::{IVec2, Vec2};

use super::chunk_manager::ChunkManager;
use super::{CHUNK_SIZE, Chunk};

/// Chunk status query and management utilities
pub struct ChunkStatus;

impl ChunkStatus {
    /// Check if a chunk needs CA update based on dirty state of itself and neighbors
    /// Returns true if this chunk or any of its 8 neighbors have dirty_rect set or simulation_active
    pub fn needs_ca_update(chunk_manager: &ChunkManager, pos: IVec2) -> bool {
        // Check the chunk itself
        if let Some(chunk) = chunk_manager.chunks.get(&pos)
            && (chunk.dirty_rect.is_some() || chunk.is_simulation_active())
        {
            return true;
        }

        // Check all 8 neighbors - materials can flow in from any direction
        for dy in -1..=1 {
            for dx in -1..=1 {
                if dx == 0 && dy == 0 {
                    continue;
                }
                let neighbor_pos = IVec2::new(pos.x + dx, pos.y + dy);
                if let Some(neighbor) = chunk_manager.chunks.get(&neighbor_pos)
                    && (neighbor.dirty_rect.is_some() || neighbor.is_simulation_active())
                {
                    return true;
                }
            }
        }

        false
    }

    /// Update active chunks: remove distant chunks and re-activate nearby loaded chunks
    /// Returns the number of chunks added to the active list
    pub fn update_active_chunks(
        chunk_manager: &mut ChunkManager,
        player_position: Vec2,
        active_chunk_radius: i32,
    ) -> usize {
        let player_chunk_x = (player_position.x as i32).div_euclid(CHUNK_SIZE as i32);
        let player_chunk_y = (player_position.y as i32).div_euclid(CHUNK_SIZE as i32);

        // 1. Remove distant chunks from active list
        chunk_manager.active_chunks.retain(|pos| {
            let dist_x = (pos.x - player_chunk_x).abs();
            let dist_y = (pos.y - player_chunk_y).abs();
            dist_x <= active_chunk_radius && dist_y <= active_chunk_radius
        });

        // 2. Add nearby loaded chunks that aren't currently active
        let mut added_count = 0;
        for cy in (player_chunk_y - active_chunk_radius)..=(player_chunk_y + active_chunk_radius) {
            for cx in
                (player_chunk_x - active_chunk_radius)..=(player_chunk_x + active_chunk_radius)
            {
                let pos = IVec2::new(cx, cy);

                // If chunk is loaded but not active, add it to active list
                if chunk_manager.chunks.contains_key(&pos)
                    && !chunk_manager.active_chunks.contains(&pos)
                {
                    chunk_manager.active_chunks.push(pos);
                    added_count += 1;

                    // Mark newly activated chunks for simulation so physics/chemistry runs
                    if let Some(chunk) = chunk_manager.chunks.get_mut(&pos) {
                        chunk.set_simulation_active(true);
                    }
                }
            }
        }

        added_count
    }

    /// Ensure chunks exist for rectangular area (pre-allocation)
    /// Creates empty chunks for the given world coordinate bounds if they don't exist
    /// Used by headless training to set up scenarios without full world generation
    pub fn ensure_chunks_for_area(
        chunk_manager: &mut ChunkManager,
        min_x: i32,
        min_y: i32,
        max_x: i32,
        max_y: i32,
    ) {
        let (min_chunk, _, _) = ChunkManager::world_to_chunk_coords(min_x, min_y);
        let (max_chunk, _, _) = ChunkManager::world_to_chunk_coords(max_x, max_y);

        for cy in min_chunk.y..=max_chunk.y {
            for cx in min_chunk.x..=max_chunk.x {
                let pos = IVec2::new(cx, cy);
                chunk_manager
                    .chunks
                    .entry(pos)
                    .or_insert_with(|| Chunk::new(cx, cy));
            }
        }
    }
}
