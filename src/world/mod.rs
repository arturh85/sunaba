//! World management - chunks, loading, saving

mod chunk;
#[allow(clippy::module_inception)]
mod world;
pub mod generation;
pub mod persistence;

pub use chunk::{Chunk, Pixel, CHUNK_SIZE, pixel_flags};
pub use world::World;
pub use generation::WorldGenerator;
pub use persistence::{ChunkPersistence, WorldMetadata};
