//! Eraser tool

use super::{Tool, draw_circle};
use sunaba_core::simulation::MaterialId;
use sunaba_core::world::World;

/// Eraser tool that places AIR
pub struct EraseTool;

impl Tool for EraseTool {
    fn name(&self) -> &str {
        "Eraser"
    }

    fn apply(&self, world: &mut World, x: i32, y: i32, brush_size: u32) {
        draw_circle(world, x, y, brush_size, MaterialId::AIR);
    }
}
