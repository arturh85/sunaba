//! World management - chunks, loading, saving

pub mod biome;
mod ca_update;
mod chemistry_system;
mod chunk;
mod chunk_manager;
mod chunk_status;
mod collision;
pub mod context_scanner;
mod debris_system;
pub mod generation;
mod light_system;
mod mining_system;
mod neighbor_queries;
pub mod persistence;
mod persistence_system;
mod pixel_queries;
mod player_physics;
mod raycasting;
pub mod rng_trait;
pub mod stats;
#[allow(clippy::module_inception)]
mod world;
pub mod worldgen_config;

pub use biome::{BiomeDefinition, BiomeRegistry, BiomeType};
pub use chunk::{CHUNK_SIZE, Chunk, Pixel, pixel_flags};
pub use chunk_manager::ChunkManager;
pub use chunk_status::ChunkStatus;
pub use context_scanner::{
    ContextScanner, MAX_SCAN_DISTANCE, PlacementContext, PlacementPredicate,
};
pub use debris_system::DebrisSystem;
pub use generation::WorldGenerator;
pub use light_system::LightSystem;
pub use mining_system::MiningSystem;
pub use neighbor_queries::NeighborQueries;
pub use persistence::{ChunkPersistence, WorldMetadata};
pub use persistence_system::PersistenceSystem;
pub use pixel_queries::PixelQueries;
pub use player_physics::PlayerPhysicsSystem;
pub use raycasting::Raycasting;
pub use rng_trait::WorldRng;
pub use stats::{NoopStats, SimStats};
pub use world::World;
pub use worldgen_config::{
    BiomeConfig, BiomeParams, CaveParams, FeatureParams, FractalTypeConfig, LavaPoolConfig,
    NoiseLayerConfig, NoiseTypeConfig, OreConfig, TerrainParams, UndergroundLayers,
    VegetationParams, WorldGenConfig, WorldParams,
};
