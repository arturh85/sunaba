//! World management - chunks, loading, saving

pub mod biome;
mod chunk;
pub mod generation;
pub mod persistence;
pub mod rng_trait;
pub mod stats;
#[allow(clippy::module_inception)]
mod world;

pub use biome::{BiomeDefinition, BiomeRegistry, BiomeType};
pub use chunk::{CHUNK_SIZE, Chunk, Pixel, pixel_flags};
pub use generation::WorldGenerator;
pub use persistence::{ChunkPersistence, WorldMetadata};
pub use rng_trait::WorldRng;
pub use stats::{NoopStats, SimStats};
pub use world::World;
