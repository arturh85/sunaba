//! Simulation systems - materials, reactions, temperature, pressure

mod materials;
pub mod temperature;
pub mod state_changes;
pub mod reactions;
pub mod structural;
pub mod light;

pub use materials::{Materials, MaterialDef, MaterialType, MaterialId, MaterialTag};
pub use temperature::{TemperatureSimulator, add_heat_at_pixel, get_temperature_at_pixel};
pub use state_changes::StateChangeSystem;
pub use reactions::{Reaction, ReactionRegistry};
pub use structural::StructuralIntegritySystem;
pub use light::LightPropagation;
