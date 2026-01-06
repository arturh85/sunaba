//! Persistence system for chunk loading, saving, and world lifecycle management

use anyhow::Result;
use glam::IVec2;

use super::chunk_manager::ChunkManager;
use super::generation::WorldGenerator;
use super::persistence::{ChunkPersistence, WorldMetadata};
use super::{CHUNK_SIZE, Chunk};
use crate::entity::player::Player;

/// Manages chunk persistence, loading, and eviction
pub struct PersistenceSystem {
    pub(super) persistence: Option<ChunkPersistence>,
    pub(super) generator: WorldGenerator,
}

impl PersistenceSystem {
    /// Create a new persistence system with the given seed
    pub fn new(seed: u64) -> Self {
        Self {
            persistence: None,
            generator: WorldGenerator::new(seed),
        }
    }

    /// Set the world generator (for terrain generation with custom seed)
    pub fn set_seed(&mut self, seed: u64) {
        self.generator = WorldGenerator::new(seed);
    }

    /// Get the current seed
    pub fn seed(&self) -> u64 {
        self.generator.seed
    }

    /// Update the generator config (keeps the same seed)
    pub fn update_generator_config(&mut self, config: super::worldgen_config::WorldGenConfig) {
        self.generator.update_config(config);
    }

    /// Get the current generator config
    pub fn generator_config(&self) -> &super::worldgen_config::WorldGenConfig {
        self.generator.config()
    }

    /// Clear all chunks from the chunk manager
    pub fn clear_all_chunks(&mut self, chunk_manager: &mut ChunkManager) {
        chunk_manager.chunks.clear();
        chunk_manager.active_chunks.clear();
        log::info!("Cleared all chunks");
    }

    /// Add a chunk to the world
    pub fn add_chunk(
        &mut self,
        chunk_manager: &mut ChunkManager,
        chunk: Chunk,
        player_pos: glam::Vec2,
    ) {
        let pos = IVec2::new(chunk.x, chunk.y);
        chunk_manager.chunks.insert(pos, chunk);

        // Add to active chunks if within range of player
        let player_chunk_x = (player_pos.x as i32).div_euclid(CHUNK_SIZE as i32);
        let player_chunk_y = (player_pos.y as i32).div_euclid(CHUNK_SIZE as i32);

        let dist_x = (pos.x - player_chunk_x).abs();
        let dist_y = (pos.y - player_chunk_y).abs();

        const ACTIVE_CHUNK_RADIUS: i32 = 3;
        if dist_x <= ACTIVE_CHUNK_RADIUS
            && dist_y <= ACTIVE_CHUNK_RADIUS
            && !chunk_manager.active_chunks.contains(&pos)
        {
            chunk_manager.active_chunks.push(pos);
        }
    }

    /// Initialize persistent world (load or generate)
    /// Returns the updated player and whether light initialization is needed
    pub fn load_persistent_world(
        &mut self,
        chunk_manager: &mut ChunkManager,
        player: &mut Player,
    ) -> Result<()> {
        // Reset ephemeral mode when returning to persistent world
        chunk_manager.ephemeral_chunks = false;

        // Clear any existing chunks (from test world generation)
        self.clear_all_chunks(chunk_manager);

        let persistence =
            ChunkPersistence::new("default").expect("Failed to create chunk persistence");

        let metadata = persistence.load_metadata();

        self.generator = WorldGenerator::new(metadata.seed);

        // Restore player data if it exists, otherwise use spawn point
        if let Some(saved_player) = metadata.player_data {
            *player = saved_player;
            log::info!(
                "Restored player data: inventory={}/{} slots, health={:.0}/{:.0}, hunger={:.0}/{:.0}",
                player.inventory.used_slot_count(),
                player.inventory.max_slots,
                player.health.current,
                player.health.max,
                player.hunger.current,
                player.hunger.max
            );
        } else {
            // New world - set player at spawn point
            player.position = glam::Vec2::new(metadata.spawn_point.0, metadata.spawn_point.1);
            log::info!("New world - player spawned at {:?}", player.position);
        }

        self.persistence = Some(persistence);

        // Load initial chunks around spawn
        self.load_chunks_around_player(chunk_manager, player.position);

        log::info!("Loaded persistent world (seed: {})", metadata.seed);

        Ok(())
    }

    /// Disable persistence for demo levels
    /// This prevents dynamic chunk loading from overwriting demo level chunks
    pub fn disable_persistence(&mut self, chunk_manager: &mut ChunkManager) {
        self.persistence = None;
        chunk_manager.ephemeral_chunks = true;
        log::info!("Persistence disabled - using ephemeral chunks");
    }

    /// Load chunks within active radius of player (17x17 = 289 chunks)
    pub fn load_chunks_around_player(
        &mut self,
        chunk_manager: &mut ChunkManager,
        player_pos: glam::Vec2,
    ) {
        let player_chunk_x = (player_pos.x as i32).div_euclid(CHUNK_SIZE as i32);
        let player_chunk_y = (player_pos.y as i32).div_euclid(CHUNK_SIZE as i32);

        for cy in (player_chunk_y - 8)..=(player_chunk_y + 8) {
            for cx in (player_chunk_x - 8)..=(player_chunk_x + 8) {
                self.load_or_generate_chunk(chunk_manager, cx, cy, player_pos);
            }
        }
    }

    /// Load nearby chunks dynamically as player moves (called when entering new chunk)
    pub fn load_nearby_chunks(&mut self, chunk_manager: &mut ChunkManager, player_pos: glam::Vec2) {
        // Don't auto-load chunks in ephemeral mode - use only chunks explicitly requested
        if chunk_manager.ephemeral_chunks {
            return;
        }

        let player_chunk_x = (player_pos.x as i32).div_euclid(CHUNK_SIZE as i32);
        let player_chunk_y = (player_pos.y as i32).div_euclid(CHUNK_SIZE as i32);

        // Load chunks within 20-chunk radius (ensures chunks loaded beyond texture edge)
        const LOAD_RADIUS: i32 = 20;

        for cy in (player_chunk_y - LOAD_RADIUS)..=(player_chunk_y + LOAD_RADIUS) {
            for cx in (player_chunk_x - LOAD_RADIUS)..=(player_chunk_x + LOAD_RADIUS) {
                self.load_or_generate_chunk(chunk_manager, cx, cy, player_pos);
            }
        }
    }

    /// Load or generate a chunk at the given coordinates
    pub fn load_or_generate_chunk(
        &mut self,
        chunk_manager: &mut ChunkManager,
        chunk_x: i32,
        chunk_y: i32,
        player_pos: glam::Vec2,
    ) {
        let pos = IVec2::new(chunk_x, chunk_y);

        if chunk_manager.chunks.contains_key(&pos) {
            log::trace!(
                "[LOAD] Chunk ({}, {}) already loaded, skipping",
                chunk_x,
                chunk_y
            );
            return; // Already loaded
        }

        let chunk = if let Some(persistence) = &self.persistence {
            log::debug!(
                "[LOAD] Requesting chunk ({}, {}) from persistence",
                chunk_x,
                chunk_y
            );
            persistence.load_chunk(chunk_x, chunk_y, &self.generator)
        } else {
            // Ephemeral mode: use generator without saving to disk
            self.generator.generate_chunk(chunk_x, chunk_y)
        };

        self.add_chunk(chunk_manager, chunk, player_pos);

        // LRU eviction if too many chunks loaded
        if chunk_manager.chunks.len() > chunk_manager.loaded_chunk_limit {
            self.evict_distant_chunks(chunk_manager, player_pos);
        }
    }

    /// Save and unload chunks far from player
    pub fn evict_distant_chunks(
        &mut self,
        chunk_manager: &mut ChunkManager,
        player_pos: glam::Vec2,
    ) {
        let player_chunk_x = (player_pos.x as i32).div_euclid(CHUNK_SIZE as i32);
        let player_chunk_y = (player_pos.y as i32).div_euclid(CHUNK_SIZE as i32);

        let mut to_evict = Vec::new();

        for pos in chunk_manager.chunks.keys() {
            let dist_x = (pos.x - player_chunk_x).abs();
            let dist_y = (pos.y - player_chunk_y).abs();

            // Unload chunks >10 chunks away
            if dist_x > 10 || dist_y > 10 {
                to_evict.push(*pos);
            }
        }

        for pos in to_evict {
            if let Some(chunk) = chunk_manager.chunks.remove(&pos)
                && chunk.dirty
                && let Some(persistence) = &self.persistence
            {
                if let Err(e) = persistence.save_chunk(&chunk) {
                    log::error!("Failed to save chunk ({}, {}): {}", pos.x, pos.y, e);
                } else {
                    log::debug!("Saved and evicted chunk ({}, {})", pos.x, pos.y);
                }
            }
        }
    }

    /// Save all dirty chunks (periodic auto-save)
    pub fn save_dirty_chunks(&mut self, chunk_manager: &mut ChunkManager) {
        if let Some(persistence) = &self.persistence {
            let mut saved_count = 0;
            let total_dirty = chunk_manager.chunks.values().filter(|c| c.dirty).count();

            if total_dirty > 0 {
                log::debug!("[SAVE] Starting auto-save of {} dirty chunks", total_dirty);
            }

            for chunk in chunk_manager.chunks.values_mut() {
                if chunk.dirty {
                    let non_air = chunk.count_non_air();
                    log::debug!(
                        "[SAVE] Saving dirty chunk ({}, {}) - {} non-air pixels",
                        chunk.x,
                        chunk.y,
                        non_air
                    );

                    if let Err(e) = persistence.save_chunk(chunk) {
                        log::error!(
                            "[SAVE] Failed to save chunk ({}, {}): {}",
                            chunk.x,
                            chunk.y,
                            e
                        );
                    } else {
                        chunk.dirty = false;
                        saved_count += 1;
                    }
                }
            }

            if saved_count > 0 {
                log::info!("[SAVE] Auto-saved {} dirty chunks", saved_count);
            }
        }
    }

    /// Save all chunks and metadata (manual save)
    pub fn save_all_dirty_chunks(
        &mut self,
        chunk_manager: &mut ChunkManager,
        player: &Player,
        play_time_seconds: u64,
    ) {
        self.save_dirty_chunks(chunk_manager);

        // Also save metadata with player data
        if let Some(persistence) = &self.persistence {
            #[cfg(not(target_arch = "wasm32"))]
            let last_played = chrono::Local::now().to_rfc3339();
            #[cfg(target_arch = "wasm32")]
            let last_played = "WASM Session".to_string();

            let metadata = WorldMetadata {
                version: 1,
                seed: self.generator.seed,
                spawn_point: (player.position.x, player.position.y),
                created_at: String::new(), // Preserved from load
                last_played,
                play_time_seconds, // Accumulated play time from World
                player_data: Some(player.clone()), // Save player inventory, health, hunger
            };

            if let Err(e) = persistence.save_metadata(&metadata) {
                log::error!("Failed to save world metadata: {}", e);
            } else {
                log::info!("Saved world metadata with player data");
            }
        }
    }
}
