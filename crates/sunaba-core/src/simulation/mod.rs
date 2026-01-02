//! Simulation systems - materials, reactions, temperature, pressure

pub mod light;
pub mod mining;
pub mod regeneration;
pub mod state_changes;
pub mod structural;
pub mod temperature;

// Re-export from sunaba-simulation for backward compatibility
pub use sunaba_simulation::{
    CHUNK_AREA, CHUNK_SIZE, MaterialDef, MaterialId, MaterialTag, MaterialType, Materials, Pixel,
    Reaction, ReactionRegistry, pixel_flags,
};

pub use light::LightPropagation;
pub use regeneration::RegenerationSystem;
pub use state_changes::StateChangeSystem;
pub use structural::StructuralIntegritySystem;
pub use temperature::{TemperatureSimulator, add_heat_at_pixel, get_temperature_at_pixel};
