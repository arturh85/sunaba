//! # Sunaba - 2D Falling Sand Physics Sandbox
//!
//! A survival game where every pixel is simulated with material properties.

pub mod app;
pub mod world;
pub mod simulation;
pub mod physics;
pub mod render;
pub mod ui;
pub mod levels;
pub mod entity;

pub use app::App;

/// Common imports for internal use
pub mod prelude {
    pub use crate::world::{Chunk, World, Pixel, CHUNK_SIZE};
    pub use crate::simulation::{Materials, MaterialId, MaterialType};
    pub use glam::{Vec2, IVec2};
}
