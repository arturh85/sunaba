//! Pen tool for drawing materials

use super::{Tool, draw_circle};
use sunaba_core::world::World;

/// Pen tool that draws a specific material
pub struct PenTool {
    material_id: u16,
}

impl PenTool {
    /// Create a new pen tool for the given material
    pub fn new(material_id: u16) -> Self {
        Self { material_id }
    }

    /// Set the material this pen draws
    pub fn set_material(&mut self, material_id: u16) {
        self.material_id = material_id;
    }

    /// Get the current material
    pub fn material_id(&self) -> u16 {
        self.material_id
    }
}

impl Tool for PenTool {
    fn name(&self) -> &str {
        "Pen"
    }

    fn apply(&self, world: &mut World, x: i32, y: i32, brush_size: u32) {
        draw_circle(world, x, y, brush_size, self.material_id);
    }
}
