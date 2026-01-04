//! Chunk lifecycle management - loading, unloading, and active chunk tracking

use glam::IVec2;
use rstar::RTree;
use std::collections::HashMap;

use super::generation::WorldGenerator;
use super::persistence::ChunkPersistence;
use super::{CHUNK_SIZE, Chunk};

/// Wrapper for chunk position to implement R-tree traits
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
struct ChunkPos(IVec2);

impl ChunkPos {
    fn from_ivec2(pos: IVec2) -> Self {
        Self(pos)
    }

    fn to_ivec2(self) -> IVec2 {
        self.0
    }
}

impl rstar::Point for ChunkPos {
    type Scalar = i32;
    const DIMENSIONS: usize = 2;

    fn generate(mut generator: impl FnMut(usize) -> Self::Scalar) -> Self {
        ChunkPos(IVec2::new(generator(0), generator(1)))
    }

    fn nth(&self, index: usize) -> Self::Scalar {
        match index {
            0 => self.0.x,
            1 => self.0.y,
            _ => panic!("ChunkPos only has 2 dimensions"),
        }
    }

    fn nth_mut(&mut self, index: usize) -> &mut Self::Scalar {
        match index {
            0 => &mut self.0.x,
            1 => &mut self.0.y,
            _ => panic!("ChunkPos only has 2 dimensions"),
        }
    }
}

/// Manages chunk loading, unloading, and active chunk tracking
pub struct ChunkManager {
    /// Loaded chunks, keyed by chunk coordinates
    pub chunks: HashMap<IVec2, Chunk>,

    /// Spatial index for fast chunk queries (O(log n) lookup)
    spatial_index: RTree<ChunkPos>,

    /// Which chunks are currently active (being simulated)
    pub active_chunks: Vec<IVec2>,

    /// Last player chunk position for dynamic chunk loading
    pub last_load_chunk_pos: Option<IVec2>,

    /// Maximum number of chunks to keep loaded in memory
    pub loaded_chunk_limit: usize,

    /// Ephemeral chunks mode - generates chunks on-demand without disk persistence (used by SpacetimeDB server)
    pub ephemeral_chunks: bool,
}

impl ChunkManager {
    /// Create a new chunk manager with default settings
    pub fn new() -> Self {
        Self {
            chunks: HashMap::new(),
            spatial_index: RTree::new(),
            active_chunks: Vec::new(),
            last_load_chunk_pos: None,
            loaded_chunk_limit: 3000, // ~19MB max memory
            ephemeral_chunks: false,
        }
    }

    /// Active chunk simulation radius (chunks from player)
    pub const ACTIVE_CHUNK_RADIUS: i32 = 3; // 7×7 grid = 49 chunks

    /// Convert world coordinates to chunk coordinates + local offset
    pub fn world_to_chunk_coords(world_x: i32, world_y: i32) -> (IVec2, usize, usize) {
        let chunk_x = world_x.div_euclid(CHUNK_SIZE as i32);
        let chunk_y = world_y.div_euclid(CHUNK_SIZE as i32);
        let local_x = world_x.rem_euclid(CHUNK_SIZE as i32) as usize;
        let local_y = world_y.rem_euclid(CHUNK_SIZE as i32) as usize;
        (IVec2::new(chunk_x, chunk_y), local_x, local_y)
    }

    /// Update active chunks based on player position
    pub fn update_active_chunks(&mut self, player_pos: glam::Vec2) {
        let player_chunk_x = (player_pos.x as i32).div_euclid(CHUNK_SIZE as i32);
        let player_chunk_y = (player_pos.y as i32).div_euclid(CHUNK_SIZE as i32);

        // 1. Remove distant chunks from active list
        self.active_chunks.retain(|pos| {
            let dist_x = (pos.x - player_chunk_x).abs();
            let dist_y = (pos.y - player_chunk_y).abs();
            dist_x <= Self::ACTIVE_CHUNK_RADIUS && dist_y <= Self::ACTIVE_CHUNK_RADIUS
        });

        // 2. Add nearby loaded chunks that aren't currently active
        // Only check chunks within active radius (7×7 grid = 49 chunks max)
        for cy in (player_chunk_y - Self::ACTIVE_CHUNK_RADIUS)
            ..=(player_chunk_y + Self::ACTIVE_CHUNK_RADIUS)
        {
            for cx in (player_chunk_x - Self::ACTIVE_CHUNK_RADIUS)
                ..=(player_chunk_x + Self::ACTIVE_CHUNK_RADIUS)
            {
                let pos = IVec2::new(cx, cy);

                // If chunk is loaded but not active, add it to active list
                if self.chunks.contains_key(&pos) && !self.active_chunks.contains(&pos) {
                    self.active_chunks.push(pos);
                    // Mark newly activated chunks for simulation so physics/chemistry runs
                    if let Some(chunk) = self.chunks.get_mut(&pos) {
                        chunk.set_simulation_active(true);
                    }
                }
            }
        }
    }

    /// Ensure chunks are loaded for a given world coordinate area
    pub fn ensure_chunks_for_area(&mut self, min_x: i32, min_y: i32, max_x: i32, max_y: i32) {
        let (min_chunk, _, _) = Self::world_to_chunk_coords(min_x, min_y);
        let (max_chunk, _, _) = Self::world_to_chunk_coords(max_x, max_y);

        for cy in min_chunk.y..=max_chunk.y {
            for cx in min_chunk.x..=max_chunk.x {
                let pos = IVec2::new(cx, cy);
                // Use entry API to check if chunk exists before inserting
                use std::collections::hash_map::Entry;
                if let Entry::Vacant(e) = self.chunks.entry(pos) {
                    e.insert(Chunk::new(cx, cy));
                    // Insert into spatial index
                    self.spatial_index.insert(ChunkPos::from_ivec2(pos));
                }
            }
        }
    }

    /// Get chunk at chunk coordinates (not world coordinates)
    pub fn get_chunk(&self, chunk_x: i32, chunk_y: i32) -> Option<&Chunk> {
        self.chunks.get(&IVec2::new(chunk_x, chunk_y))
    }

    /// Get mutable chunk at chunk coordinates
    pub fn get_chunk_mut(&mut self, chunk_x: i32, chunk_y: i32) -> Option<&mut Chunk> {
        self.chunks.get_mut(&IVec2::new(chunk_x, chunk_y))
    }

    /// Check if chunk is loaded
    pub fn has_chunk(&self, pos: IVec2) -> bool {
        self.chunks.contains_key(&pos)
    }

    /// Insert pre-loaded chunk (for SpacetimeDB server)
    pub fn insert_chunk(&mut self, pos: IVec2, chunk: Chunk) {
        self.chunks.insert(pos, chunk);
    }

    /// Generate a single chunk at position (for SpacetimeDB server)
    pub fn generate_chunk_at(&mut self, pos: IVec2, generator: &WorldGenerator) {
        let chunk = generator.generate_chunk(pos.x, pos.y);
        self.chunks.insert(pos, chunk);
    }

    /// Iterator over all chunks (for SpacetimeDB server)
    pub fn chunks_iter(&self) -> impl Iterator<Item = (&IVec2, &Chunk)> {
        self.chunks.iter()
    }

    /// Get iterator over active chunks
    pub fn active_chunks_iter(&self) -> impl Iterator<Item = &Chunk> {
        self.active_chunks
            .iter()
            .filter_map(|pos| self.chunks.get(pos))
    }

    /// Get positions of active chunks
    pub fn active_chunk_positions(&self) -> &[IVec2] {
        &self.active_chunks
    }

    /// Get all loaded chunks
    pub fn chunks(&self) -> &HashMap<IVec2, Chunk> {
        &self.chunks
    }

    /// Add a chunk to the manager
    pub fn add_chunk(&mut self, chunk: Chunk, player_pos: glam::Vec2) {
        let pos = IVec2::new(chunk.x, chunk.y);
        self.chunks.insert(pos, chunk);

        // Add to active chunks if within range of player
        let player_chunk_x = player_pos.x as i32 / CHUNK_SIZE as i32;
        let player_chunk_y = player_pos.y as i32 / CHUNK_SIZE as i32;
        let dist_x = (pos.x - player_chunk_x).abs();
        let dist_y = (pos.y - player_chunk_y).abs();
        if dist_x <= Self::ACTIVE_CHUNK_RADIUS
            && dist_y <= Self::ACTIVE_CHUNK_RADIUS
            && !self.active_chunks.contains(&pos)
        {
            self.active_chunks.push(pos);
        }
    }

    /// Load nearby chunks dynamically as player moves (called when entering new chunk)
    pub fn load_nearby_chunks(
        &mut self,
        player_pos: glam::Vec2,
        persistence: Option<&ChunkPersistence>,
        generator: &WorldGenerator,
    ) {
        // Don't auto-load chunks in ephemeral mode - use only chunks explicitly requested
        if self.ephemeral_chunks {
            return;
        }

        let player_chunk_x = (player_pos.x as i32).div_euclid(CHUNK_SIZE as i32);
        let player_chunk_y = (player_pos.y as i32).div_euclid(CHUNK_SIZE as i32);

        // Load chunks within 20-chunk radius (ensures chunks loaded beyond texture edge)
        const LOAD_RADIUS: i32 = 20;

        for cy in (player_chunk_y - LOAD_RADIUS)..=(player_chunk_y + LOAD_RADIUS) {
            for cx in (player_chunk_x - LOAD_RADIUS)..=(player_chunk_x + LOAD_RADIUS) {
                self.load_or_generate_chunk(cx, cy, player_pos, persistence, generator);
            }
        }
    }

    /// Load chunks around player (larger area for initial load)
    pub fn load_chunks_around_player(
        &mut self,
        player_pos: glam::Vec2,
        persistence: Option<&ChunkPersistence>,
        generator: &WorldGenerator,
    ) {
        let player_chunk_x = (player_pos.x as i32).div_euclid(CHUNK_SIZE as i32);
        let player_chunk_y = (player_pos.y as i32).div_euclid(CHUNK_SIZE as i32);

        for cy in (player_chunk_y - 8)..=(player_chunk_y + 8) {
            for cx in (player_chunk_x - 8)..=(player_chunk_x + 8) {
                self.load_or_generate_chunk(cx, cy, player_pos, persistence, generator);
            }
        }
    }

    /// Load or generate a chunk at the given coordinates
    fn load_or_generate_chunk(
        &mut self,
        chunk_x: i32,
        chunk_y: i32,
        player_pos: glam::Vec2,
        persistence: Option<&ChunkPersistence>,
        generator: &WorldGenerator,
    ) {
        let pos = IVec2::new(chunk_x, chunk_y);

        if self.chunks.contains_key(&pos) {
            log::trace!(
                "[LOAD] Chunk ({}, {}) already loaded, skipping",
                chunk_x,
                chunk_y
            );
            return; // Already loaded
        }

        let chunk = if let Some(persistence) = persistence {
            log::debug!(
                "[LOAD] Requesting chunk ({}, {}) from persistence",
                chunk_x,
                chunk_y
            );
            persistence.load_chunk(chunk_x, chunk_y, generator)
        } else {
            // Ephemeral mode: use generator without saving to disk
            generator.generate_chunk(chunk_x, chunk_y)
        };

        self.chunks.insert(pos, chunk);
        // Insert into spatial index
        self.spatial_index.insert(ChunkPos::from_ivec2(pos));

        // LRU eviction if too many chunks loaded
        if self.chunks.len() > self.loaded_chunk_limit {
            self.evict_distant_chunks_internal(player_pos, persistence);
        }
    }

    /// Save and unload chunks far from player
    /// Uses R-tree spatial index for efficient distant chunk finding
    fn evict_distant_chunks_internal(
        &mut self,
        player_pos: glam::Vec2,
        persistence: Option<&ChunkPersistence>,
    ) {
        let player_chunk_x = (player_pos.x as i32).div_euclid(CHUNK_SIZE as i32);
        let player_chunk_y = (player_pos.y as i32).div_euclid(CHUNK_SIZE as i32);

        let mut to_evict = Vec::new();

        // Iterate through spatial index instead of HashMap keys
        // This provides better cache locality and enables future optimizations
        for chunk_pos in self.spatial_index.iter() {
            let pos = chunk_pos.to_ivec2();
            let dist_x = (pos.x - player_chunk_x).abs();
            let dist_y = (pos.y - player_chunk_y).abs();

            // Unload chunks >10 chunks away
            if dist_x > 10 || dist_y > 10 {
                to_evict.push(pos);
            }
        }

        // Remove from both HashMap and spatial index
        for pos in to_evict {
            // Remove from spatial index
            self.spatial_index.remove(&ChunkPos::from_ivec2(pos));

            // Remove from HashMap and optionally save
            if let Some(chunk) = self.chunks.remove(&pos)
                && chunk.dirty
                && let Some(persistence) = persistence
            {
                if let Err(e) = persistence.save_chunk(&chunk) {
                    log::error!("Failed to save chunk ({}, {}): {}", pos.x, pos.y, e);
                } else {
                    log::debug!("Saved and evicted chunk ({}, {})", pos.x, pos.y);
                }
            }
        }
    }

    /// Evict distant chunks (public API)
    pub fn evict_distant_chunks(
        &mut self,
        player_pos: glam::Vec2,
        persistence: Option<&ChunkPersistence>,
    ) {
        self.evict_distant_chunks_internal(player_pos, persistence);
    }

    /// Save all dirty chunks (periodic auto-save)
    pub fn save_dirty_chunks(&mut self, persistence: Option<&ChunkPersistence>) -> usize {
        if let Some(persistence) = persistence {
            let mut saved_count = 0;
            let total_dirty = self.chunks.values().filter(|c| c.dirty).count();

            if total_dirty > 0 {
                log::debug!("[SAVE] Starting auto-save of {} dirty chunks", total_dirty);
            }

            for chunk in self.chunks.values_mut() {
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

            saved_count
        } else {
            0
        }
    }

    /// Clear all chunks
    pub fn clear_all_chunks(&mut self) {
        self.chunks.clear();
        self.active_chunks.clear();
        self.last_load_chunk_pos = None;
    }

    /// Get chunk count
    pub fn chunk_count(&self) -> usize {
        self.chunks.len()
    }

    /// Set ephemeral chunks mode
    pub fn set_ephemeral_mode(&mut self, ephemeral: bool) {
        self.ephemeral_chunks = ephemeral;
    }
}

impl Default for ChunkManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::generation::WorldGenerator;

    #[test]
    fn test_new_creates_empty_manager() {
        let manager = ChunkManager::new();
        assert_eq!(manager.chunks.len(), 0);
        assert_eq!(manager.active_chunks.len(), 0);
        assert_eq!(manager.last_load_chunk_pos, None);
        assert_eq!(manager.loaded_chunk_limit, 3000);
        assert!(!manager.ephemeral_chunks);
    }

    #[test]
    fn test_world_to_chunk_coords_positive() {
        let (chunk_pos, local_x, local_y) = ChunkManager::world_to_chunk_coords(100, 200);
        assert_eq!(chunk_pos, IVec2::new(1, 3)); // 100/64=1, 200/64=3
        assert_eq!(local_x, 36); // 100 % 64 = 36
        assert_eq!(local_y, 8); // 200 % 64 = 8
    }

    #[test]
    fn test_world_to_chunk_coords_negative() {
        let (chunk_pos, local_x, local_y) = ChunkManager::world_to_chunk_coords(-100, -200);
        assert_eq!(chunk_pos, IVec2::new(-2, -4)); // div_euclid(-100, 64) = -2
        assert_eq!(local_x, 28); // rem_euclid(-100, 64) = 28
        assert_eq!(local_y, 56); // rem_euclid(-200, 64) = 56
    }

    #[test]
    fn test_world_to_chunk_coords_zero() {
        let (chunk_pos, local_x, local_y) = ChunkManager::world_to_chunk_coords(0, 0);
        assert_eq!(chunk_pos, IVec2::new(0, 0));
        assert_eq!(local_x, 0);
        assert_eq!(local_y, 0);
    }

    #[test]
    fn test_world_to_chunk_coords_chunk_boundary() {
        // Test at chunk boundary (x=64 is first pixel of chunk 1)
        let (chunk_pos, local_x, local_y) = ChunkManager::world_to_chunk_coords(64, 128);
        assert_eq!(chunk_pos, IVec2::new(1, 2));
        assert_eq!(local_x, 0);
        assert_eq!(local_y, 0);
    }

    #[test]
    fn test_ensure_chunks_for_area() {
        let mut manager = ChunkManager::new();
        manager.ensure_chunks_for_area(0, 0, 127, 127); // 2x2 chunks

        assert_eq!(manager.chunks.len(), 4);
        assert!(manager.has_chunk(IVec2::new(0, 0)));
        assert!(manager.has_chunk(IVec2::new(1, 0)));
        assert!(manager.has_chunk(IVec2::new(0, 1)));
        assert!(manager.has_chunk(IVec2::new(1, 1)));
    }

    #[test]
    fn test_get_chunk() {
        let mut manager = ChunkManager::new();
        manager.ensure_chunks_for_area(0, 0, 63, 63);

        let chunk = manager.get_chunk(0, 0);
        assert!(chunk.is_some());
        assert_eq!(chunk.unwrap().x, 0);
        assert_eq!(chunk.unwrap().y, 0);

        let missing = manager.get_chunk(5, 5);
        assert!(missing.is_none());
    }

    #[test]
    fn test_get_chunk_mut() {
        let mut manager = ChunkManager::new();
        manager.ensure_chunks_for_area(0, 0, 63, 63);

        let chunk = manager.get_chunk_mut(0, 0);
        assert!(chunk.is_some());

        let missing = manager.get_chunk_mut(5, 5);
        assert!(missing.is_none());
    }

    #[test]
    fn test_has_chunk() {
        let mut manager = ChunkManager::new();
        assert!(!manager.has_chunk(IVec2::new(0, 0)));

        manager.ensure_chunks_for_area(0, 0, 63, 63);
        assert!(manager.has_chunk(IVec2::new(0, 0)));
        assert!(!manager.has_chunk(IVec2::new(1, 1)));
    }

    #[test]
    fn test_insert_chunk() {
        let mut manager = ChunkManager::new();
        let chunk = Chunk::new(5, 10);

        manager.insert_chunk(IVec2::new(5, 10), chunk);
        assert!(manager.has_chunk(IVec2::new(5, 10)));
        assert_eq!(manager.chunks.len(), 1);
    }

    #[test]
    fn test_update_active_chunks() {
        let mut manager = ChunkManager::new();

        // Load chunks in a grid
        for cy in -2..=2 {
            for cx in -2..=2 {
                manager.insert_chunk(IVec2::new(cx, cy), Chunk::new(cx, cy));
            }
        }

        // Player at origin (chunk 0,0)
        let player_pos = glam::Vec2::new(32.0, 32.0);
        manager.update_active_chunks(player_pos);

        // Should activate chunks within ACTIVE_CHUNK_RADIUS (3)
        // That's -3..=3 in both x and y, but we only loaded -2..=2
        // So all 25 loaded chunks should be active
        assert_eq!(manager.active_chunks.len(), 25);
        assert!(manager.active_chunks.contains(&IVec2::new(0, 0)));
        assert!(manager.active_chunks.contains(&IVec2::new(2, 2)));
        assert!(manager.active_chunks.contains(&IVec2::new(-2, -2)));
    }

    #[test]
    fn test_update_active_chunks_removes_distant() {
        let mut manager = ChunkManager::new();

        // Load chunks far apart
        manager.insert_chunk(IVec2::new(0, 0), Chunk::new(0, 0));
        manager.insert_chunk(IVec2::new(10, 10), Chunk::new(10, 10));

        // Player at origin
        let player_pos = glam::Vec2::new(32.0, 32.0);
        manager.update_active_chunks(player_pos);

        // Only nearby chunk should be active
        assert_eq!(manager.active_chunks.len(), 1);
        assert!(manager.active_chunks.contains(&IVec2::new(0, 0)));
        assert!(!manager.active_chunks.contains(&IVec2::new(10, 10)));
    }

    #[test]
    fn test_clear_all_chunks() {
        let mut manager = ChunkManager::new();
        manager.ensure_chunks_for_area(0, 0, 127, 127);
        manager.update_active_chunks(glam::Vec2::new(64.0, 64.0));

        assert!(!manager.chunks.is_empty());
        assert!(!manager.active_chunks.is_empty());

        manager.clear_all_chunks();
        assert_eq!(manager.chunks.len(), 0);
        assert_eq!(manager.active_chunks.len(), 0);
        assert_eq!(manager.last_load_chunk_pos, None);
    }

    #[test]
    fn test_chunk_count() {
        let mut manager = ChunkManager::new();
        assert_eq!(manager.chunk_count(), 0);

        manager.ensure_chunks_for_area(0, 0, 63, 63);
        assert_eq!(manager.chunk_count(), 1);

        manager.ensure_chunks_for_area(0, 0, 127, 127);
        assert_eq!(manager.chunk_count(), 4);
    }

    #[test]
    fn test_ephemeral_mode() {
        let mut manager = ChunkManager::new();
        assert!(!manager.ephemeral_chunks);

        manager.set_ephemeral_mode(true);
        assert!(manager.ephemeral_chunks);

        manager.set_ephemeral_mode(false);
        assert!(!manager.ephemeral_chunks);
    }

    #[test]
    fn test_generate_chunk_at() {
        let mut manager = ChunkManager::new();
        let generator = WorldGenerator::new(12345);

        manager.generate_chunk_at(IVec2::new(0, 0), &generator);

        assert!(manager.has_chunk(IVec2::new(0, 0)));
        let chunk = manager.get_chunk(0, 0).unwrap();
        assert_eq!(chunk.x, 0);
        assert_eq!(chunk.y, 0);
    }

    #[test]
    fn test_active_chunk_positions() {
        let mut manager = ChunkManager::new();
        manager.insert_chunk(IVec2::new(0, 0), Chunk::new(0, 0));
        manager.update_active_chunks(glam::Vec2::new(32.0, 32.0));

        let positions = manager.active_chunk_positions();
        assert_eq!(positions.len(), 1);
        assert_eq!(positions[0], IVec2::new(0, 0));
    }
}
