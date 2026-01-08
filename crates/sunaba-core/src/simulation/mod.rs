//! Simulation systems - materials, reactions, temperature, pressure

pub mod falling_chunks;
pub mod knockback;
pub mod light;
pub mod mining;
pub mod regeneration;
pub mod state_changes;
pub mod structural;
pub mod temperature;
pub mod temporary_light_manager;

// Re-export from sunaba-simulation for backward compatibility
pub use sunaba_simulation::{
    CHUNK_AREA, CHUNK_SIZE, MaterialDef, MaterialId, MaterialTag, MaterialType, Materials, Pixel,
    Reaction, ReactionRegistry, apply_texture_variation, pixel_flags,
};

pub use falling_chunks::{ChunkRenderData, FallingChunk, FallingChunkSystem, WorldCollisionQuery};
pub use light::LightPropagation;
pub use regeneration::RegenerationSystem;
pub use state_changes::StateChangeSystem;
pub use structural::StructuralIntegritySystem;
pub use temperature::{TemperatureSimulator, add_heat_at_pixel, get_temperature_at_pixel};
