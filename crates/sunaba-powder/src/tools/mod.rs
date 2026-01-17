//! Tools module for Powder Game demo

mod erase;
mod pen;

pub use erase::EraseTool;
pub use pen::PenTool;

use sunaba_core::world::World;

/// Trait for drawing tools
pub trait Tool {
    /// Tool display name
    fn name(&self) -> &str;

    /// Apply tool at position with given brush size
    fn apply(&self, world: &mut World, x: i32, y: i32, brush_size: u32);
}

/// Draw a filled circle of pixels
pub fn draw_circle(world: &mut World, center_x: i32, center_y: i32, radius: u32, material_id: u16) {
    let r = radius as i32;

    for dy in -r..=r {
        for dx in -r..=r {
            if dx * dx + dy * dy <= r * r {
                let x = center_x + dx;
                let y = center_y + dy;
                world.set_pixel(x, y, material_id);
            }
        }
    }
}
