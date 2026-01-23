//! Drag tool for moving pixels around

use sunaba_core::simulation::MaterialId;
use sunaba_core::world::World;

/// Drag tool that moves pixels by dragging
pub struct DragTool {
    /// Last position for drag delta calculation
    last_pos: Option<(i32, i32)>,
}

impl DragTool {
    /// Create a new drag tool
    pub fn new() -> Self {
        Self { last_pos: None }
    }

    /// Start a drag operation
    pub fn start_drag(&mut self, x: i32, y: i32) {
        self.last_pos = Some((x, y));
    }

    /// End the drag operation
    pub fn end_drag(&mut self) {
        self.last_pos = None;
    }

    /// Check if dragging is active
    pub fn is_dragging(&self) -> bool {
        self.last_pos.is_some()
    }

    /// Apply drag movement to pixels
    pub fn apply_drag(&mut self, world: &mut World, x: i32, y: i32, brush_size: u32) {
        if let Some((last_x, last_y)) = self.last_pos {
            let dx = x - last_x;
            let dy = y - last_y;

            // Only move if there's actual movement
            if dx != 0 || dy != 0 {
                // Move pixels in brush area by delta
                // We need to be careful about order to avoid overwriting
                let r = brush_size as i32;

                // Collect pixels to move first
                let mut moves = Vec::new();

                for py in -r..=r {
                    for px in -r..=r {
                        if px * px + py * py <= r * r {
                            let src_x = last_x + px;
                            let src_y = last_y + py;
                            let dst_x = src_x + dx;
                            let dst_y = src_y + dy;

                            if let Some(pixel) = world.get_pixel(src_x, src_y)
                                && pixel.material_id != MaterialId::AIR
                            {
                                // Check destination is air
                                if let Some(dst_pixel) = world.get_pixel(dst_x, dst_y)
                                    && dst_pixel.material_id == MaterialId::AIR
                                {
                                    moves.push((src_x, src_y, dst_x, dst_y, pixel.material_id));
                                }
                            }
                        }
                    }
                }

                // Apply moves
                for (src_x, src_y, dst_x, dst_y, material_id) in moves {
                    world.set_pixel(dst_x, dst_y, material_id);
                    world.set_pixel(src_x, src_y, MaterialId::AIR);
                }
            }
        }

        // Update last position
        self.last_pos = Some((x, y));
    }
}

impl Default for DragTool {
    fn default() -> Self {
        Self::new()
    }
}
