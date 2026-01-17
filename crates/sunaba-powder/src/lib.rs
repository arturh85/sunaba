//! # Sunaba Powder - Powder Game Demo
//!
//! A classic Powder Game-style demo showcasing Sunaba's 58+ materials
//! with a simple, focused interface for experimenting with physics simulations.

pub mod app;
pub mod config;
pub mod render;
pub mod tools;
pub mod ui;

pub use app::App;

/// Common imports for internal use
pub mod prelude {
    pub use glam::{IVec2, Vec2};
    pub use sunaba_core::simulation::{MaterialId, MaterialType, Materials};
    pub use sunaba_core::world::{CHUNK_SIZE, Chunk, Pixel, World};
}
