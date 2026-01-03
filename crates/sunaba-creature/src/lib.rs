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
pub mod critter;
pub mod execution_state;
pub mod genome;
pub mod morphology;
pub mod neural;
pub mod physics;
pub mod sensors;
pub mod spawning;
pub mod traits;
pub mod types;
pub mod viability;
pub mod world_interaction;

// Re-export main types for convenience
pub use creature::Creature;
pub use critter::{Critter, CritterManager, CritterState};
pub use execution_state::{CreatureExecutionState, ExecutionInput, ExecutionState};
pub use genome::CreatureGenome;
pub use morphology::{CreatureArchetype, CreatureMorphology, MorphologyPhysics};
pub use physics::PhysicsWorld;
pub use spawning::CreatureManager;
pub use traits::{WorldAccess, WorldMutAccess};
pub use types::{EntityId, Health, Hunger};
pub use viability::{ViabilityScore, analyze_viability};

/// Classification of body part function for visualization
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BodyPartType {
    /// Central body / root anchor (yellow)
    Root,
    /// Motorized limb - can rotate via neural control (cyan/blue)
    Motor,
    /// Fixed structural connection (gray)
    Fixed,
}

impl BodyPartType {
    /// Get the default color for this body part type
    pub fn color(&self) -> [u8; 4] {
        match self {
            BodyPartType::Root => [255, 200, 50, 255], // Golden yellow
            BodyPartType::Motor => [50, 200, 255, 255], // Cyan/Blue
            BodyPartType::Fixed => [150, 150, 150, 255], // Gray
        }
    }

    /// Get a dimmed version for when motor is at rest
    pub fn dim_color(&self) -> [u8; 4] {
        match self {
            BodyPartType::Root => [200, 160, 40, 255],   // Darker gold
            BodyPartType::Motor => [40, 160, 200, 255],  // Darker cyan
            BodyPartType::Fixed => [120, 120, 120, 255], // Darker gray
        }
    }
}

/// Body part render data for a single body part
#[derive(Debug, Clone)]
pub struct BodyPartRenderData {
    pub position: Vec2,
    pub radius: f32,
    pub color: [u8; 4],
    pub part_type: BodyPartType,
    /// Motor activation level (0.0-1.0) for motorized parts
    pub motor_activity: f32,
}

/// Joint connection render data for visualizing connections
#[derive(Debug, Clone)]
pub struct JointRenderData {
    pub start: Vec2,
    pub end: Vec2,
    /// True if this is a motorized (revolute) joint
    pub is_motorized: bool,
    /// Joint rotation angle in radians (for motorized joints)
    pub angle: f32,
}

/// Render data for an entire creature
#[derive(Debug, Clone)]
pub struct CreatureRenderData {
    pub body_parts: Vec<BodyPartRenderData>,
    /// Joint connections between body parts
    pub joints: Vec<JointRenderData>,
}
