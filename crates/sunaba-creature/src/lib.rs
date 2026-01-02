//! ML-evolved creatures for Sunaba
//!
//! This crate implements:
//! - CPPN-NEAT genomes for articulated morphology generation
//! - Neural controllers for morphology-agnostic control
//! - GOAP behavior planning for high-level decision making
//! - World interaction traits for sensing, eating, mining, building

#![allow(clippy::module_inception)]

use glam::Vec2;

pub mod behavior;
pub mod creature;
pub mod genome;
pub mod morphology;
pub mod neural;
pub mod physics;
pub mod sensors;
pub mod spawning;
pub mod traits;
pub mod types;
pub mod world_interaction;

// Re-export main types for convenience
pub use creature::Creature;
pub use genome::CreatureGenome;
pub use morphology::{CreatureMorphology, MorphologyPhysics};
pub use physics::PhysicsWorld;
pub use spawning::CreatureManager;
pub use traits::{WorldAccess, WorldMutAccess};
pub use types::{EntityId, Health, Hunger};

/// Body part render data for a single body part
#[derive(Debug, Clone)]
pub struct BodyPartRenderData {
    pub position: Vec2,
    pub radius: f32,
    pub color: [u8; 4],
}

/// Render data for an entire creature
#[derive(Debug, Clone)]
pub struct CreatureRenderData {
    pub body_parts: Vec<BodyPartRenderData>,
}
