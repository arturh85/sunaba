//! World management - chunks, loading, saving

pub mod biome;
mod ca_update;
mod chemistry_system;
mod chunk;
mod chunk_manager;
mod collision;
mod debris_system;
pub mod generation;
mod light_system;
pub mod persistence;
mod persistence_system;
pub mod rng_trait;
pub mod stats;
#[allow(clippy::module_inception)]
mod world;

pub use biome::{BiomeDefinition, BiomeRegistry, BiomeType};
pub use chunk::{CHUNK_SIZE, Chunk, Pixel, pixel_flags};
pub use chunk_manager::ChunkManager;
pub use debris_system::DebrisSystem;
pub use generation::WorldGenerator;
pub use light_system::LightSystem;
pub use persistence::{ChunkPersistence, WorldMetadata};
pub use persistence_system::PersistenceSystem;
pub use rng_trait::WorldRng;
pub use stats::{NoopStats, SimStats};
pub use world::World;
