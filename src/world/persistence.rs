use crate::entity::player::Player;
use crate::world::chunk::Chunk;
use crate::world::generation::WorldGenerator;
#[allow(unused_imports)]
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

#[cfg(not(target_arch = "wasm32"))]
use std::path::{Path, PathBuf};

/// World metadata stored in world.meta file (RON format)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldMetadata {
    pub version: u32,
    pub seed: u64,
    pub spawn_point: (f32, f32),
    pub created_at: String,
    pub last_played: String,
    pub play_time_seconds: u64,

    /// Player save data (inventory, health, hunger)
    #[serde(default)]
    pub player_data: Option<Player>,
}

impl Default for WorldMetadata {
    fn default() -> Self {
        #[cfg(not(target_arch = "wasm32"))]
        let (created_at, last_played) = {
            let now = chrono::Local::now().to_rfc3339();
            (now.clone(), now)
        };

        #[cfg(target_arch = "wasm32")]
        let (created_at, last_played) = {
            let now = "WASM Session".to_string();
            (now.clone(), now)
        };

        Self {
            version: 1,
            seed: rand::random(),
            spawn_point: (0.0, 100.0), // Start above ground
            created_at,
            last_played,
            play_time_seconds: 0,
            player_data: None, // Will be populated on first save
        }
    }
}

/// Manages chunk save/load operations with compression
pub struct ChunkPersistence {
    #[cfg(not(target_arch = "wasm32"))]
    world_dir: PathBuf,
    #[cfg(target_arch = "wasm32")]
    _phantom: (),
}

#[cfg(not(target_arch = "wasm32"))]
impl ChunkPersistence {
    /// Create a new persistence manager for the given world name
    pub fn new(world_name: &str) -> Result<Self> {
        let world_dir = PathBuf::from("worlds").join(world_name);

        // Create directories if they don't exist
        std::fs::create_dir_all(world_dir.join("chunks"))
            .context("Failed to create world directories")?;

        Ok(Self { world_dir })
    }

    /// Save a chunk to disk with compression
    pub fn save_chunk(&self, chunk: &Chunk) -> Result<()> {
        let path = self.chunk_path(chunk.x, chunk.y);
        let non_air = chunk.count_non_air();

        log::info!(
            "[SAVE] Chunk ({}, {}) - {} non-air pixels - {:?}",
            chunk.x,
            chunk.y,
            non_air,
            path
        );

        // Serialize with bincode
        let serialized =
            bincode_next::serde::encode_to_vec(chunk, bincode_next::config::standard())
                .context("Failed to serialize chunk")?;

        // Compress with lz4
        let compressed = lz4_flex::compress_prepend_size(&serialized);
        let compressed_size = compressed.len();

        // Atomic write: write to temp file, then rename
        let temp_path = path.with_extension("tmp");
        std::fs::write(&temp_path, compressed).context("Failed to write chunk temp file")?;
        std::fs::rename(temp_path, &path).context("Failed to rename chunk file")?;

        log::info!(
            "[SAVE] Chunk ({}, {}) saved successfully ({} bytes compressed)",
            chunk.x,
            chunk.y,
            compressed_size
        );

        Ok(())
    }

    /// Load a chunk from disk, or generate if missing
    pub fn load_chunk(&self, chunk_x: i32, chunk_y: i32, generator: &WorldGenerator) -> Chunk {
        let path = self.chunk_path(chunk_x, chunk_y);

        if !path.exists() {
            // Generate new chunk
            log::debug!(
                "[GEN] Chunk ({}, {}) - file doesn't exist, generating",
                chunk_x,
                chunk_y
            );
            let chunk = generator.generate_chunk(chunk_x, chunk_y);
            let non_air = chunk.count_non_air();
            log::debug!(
                "[GEN] Chunk ({}, {}) generated - {} non-air pixels",
                chunk_x,
                chunk_y,
                non_air
            );
            return chunk;
        }

        // Try to load from disk
        match self.load_chunk_file(&path) {
            Ok(chunk) => {
                let non_air = chunk.count_non_air();
                log::debug!(
                    "[LOAD] Chunk ({}, {}) from disk - {} non-air pixels",
                    chunk_x,
                    chunk_y,
                    non_air
                );
                chunk
            }
            Err(e) => {
                log::warn!(
                    "[LOAD] Failed to load chunk ({}, {}): {}, regenerating",
                    chunk_x,
                    chunk_y,
                    e
                );
                let chunk = generator.generate_chunk(chunk_x, chunk_y);
                let non_air = chunk.count_non_air();
                log::info!(
                    "[GEN] Chunk ({}, {}) regenerated after load failure - {} non-air pixels",
                    chunk_x,
                    chunk_y,
                    non_air
                );
                chunk
            }
        }
    }

    fn load_chunk_file(&self, path: &Path) -> Result<Chunk> {
        let compressed = std::fs::read(path).context("Failed to read chunk file")?;
        log::debug!("Read {} bytes from {:?}", compressed.len(), path);

        let serialized = lz4_flex::decompress_size_prepended(&compressed)
            .context("Failed to decompress chunk")?;
        log::debug!("Decompressed to {} bytes", serialized.len());

        let (chunk, _): (Chunk, _) =
            bincode_next::serde::decode_from_slice(&serialized, bincode_next::config::standard())
                .map_err(|e| {
                log::error!("Bincode deserialization error: {:?}", e);
                anyhow::anyhow!("Failed to deserialize chunk: {:?}", e)
            })?;
        log::debug!("Successfully deserialized chunk");
        Ok(chunk)
    }

    fn chunk_path(&self, x: i32, y: i32) -> PathBuf {
        self.world_dir
            .join("chunks")
            .join(format!("chunk_{}_{}.bin", x, y))
    }

    /// Save world metadata to disk
    pub fn save_metadata(&self, meta: &WorldMetadata) -> Result<()> {
        let path = self.world_dir.join("world.meta");
        let serialized = ron::ser::to_string_pretty(meta, Default::default())
            .context("Failed to serialize metadata")?;
        std::fs::write(path, serialized).context("Failed to write metadata file")?;
        Ok(())
    }

    /// Load world metadata from disk, or create default
    pub fn load_metadata(&self) -> WorldMetadata {
        let path = self.world_dir.join("world.meta");

        if !path.exists() {
            log::info!("No world metadata found, creating new world");
            return WorldMetadata::default();
        }

        match std::fs::read_to_string(&path) {
            Ok(contents) => match ron::from_str(&contents) {
                Ok(meta) => {
                    log::info!("Loaded world metadata");
                    meta
                }
                Err(e) => {
                    log::warn!("Failed to parse metadata: {}, using defaults", e);
                    WorldMetadata::default()
                }
            },
            Err(e) => {
                log::warn!("Failed to read metadata: {}, using defaults", e);
                WorldMetadata::default()
            }
        }
    }

    /// Delete all chunks and metadata (used by --regenerate)
    pub fn delete_world(world_name: &str) -> Result<()> {
        let world_dir = PathBuf::from("worlds").join(world_name);
        if world_dir.exists() {
            std::fs::remove_dir_all(&world_dir).context("Failed to delete world directory")?;
            log::info!("Deleted world: {}", world_name);
        }
        Ok(())
    }
}

// WASM stub implementation - no persistence in browser for now
#[cfg(target_arch = "wasm32")]
impl ChunkPersistence {
    pub fn new(_world_name: &str) -> Result<Self> {
        log::info!("[WASM] Chunk persistence disabled - chunks will be generated on demand");
        Ok(Self { _phantom: () })
    }

    pub fn save_chunk(&self, _chunk: &Chunk) -> Result<()> {
        // No-op in WASM
        Ok(())
    }

    pub fn load_chunk(&self, chunk_x: i32, chunk_y: i32, generator: &WorldGenerator) -> Chunk {
        // Always generate in WASM
        generator.generate_chunk(chunk_x, chunk_y)
    }

    pub fn save_metadata(&self, _meta: &WorldMetadata) -> Result<()> {
        // No-op in WASM
        Ok(())
    }

    pub fn load_metadata(&self) -> WorldMetadata {
        WorldMetadata::default()
    }

    pub fn delete_world(_world_name: &str) -> Result<()> {
        // No-op in WASM
        Ok(())
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;
    use crate::world::chunk::Chunk;

    #[test]
    fn test_chunk_save_load_roundtrip() -> Result<()> {
        let test_world = "test_roundtrip";
        let persistence = ChunkPersistence::new(test_world)?;

        // Create a test chunk with some data
        let mut chunk = Chunk::new(5, -3);
        chunk.set_material(10, 20, 42);
        chunk.set_material(30, 40, 99);

        // Save it
        persistence.save_chunk(&chunk)?;

        // Load it back
        let generator = WorldGenerator::new(0);
        let loaded = persistence.load_chunk(5, -3, &generator);

        // Verify data matches
        assert_eq!(loaded.x, 5);
        assert_eq!(loaded.y, -3);
        assert_eq!(loaded.get_material(10, 20), 42);
        assert_eq!(loaded.get_material(30, 40), 99);

        // Cleanup
        ChunkPersistence::delete_world(test_world)?;
        Ok(())
    }

    #[test]
    fn test_metadata_save_load() -> Result<()> {
        let test_world = "test_metadata";
        let persistence = ChunkPersistence::new(test_world)?;

        let meta = WorldMetadata {
            version: 1,
            seed: 12345,
            spawn_point: (100.0, 200.0),
            created_at: "2024-01-01T00:00:00Z".to_string(),
            last_played: "2024-01-02T00:00:00Z".to_string(),
            play_time_seconds: 3600,
            player_data: None,
        };

        // Save and load
        persistence.save_metadata(&meta)?;
        let loaded = persistence.load_metadata();

        assert_eq!(loaded.seed, 12345);
        assert_eq!(loaded.spawn_point, (100.0, 200.0));
        assert_eq!(loaded.play_time_seconds, 3600);

        // Cleanup
        ChunkPersistence::delete_world(test_world)?;
        Ok(())
    }
}
