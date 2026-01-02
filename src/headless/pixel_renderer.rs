//! CPU-based pixel buffer renderer for headless GIF capture
//!
//! Renders the world to a pixel buffer without GPU dependencies.

use glam::Vec2;

use crate::creature::CreatureRenderData;
use crate::simulation::Materials;
use crate::world::World;

/// CPU-based renderer that outputs to a pixel buffer
pub struct PixelRenderer {
    /// Width of the viewport in pixels
    pub width: usize,
    /// Height of the viewport in pixels
    pub height: usize,
    /// RGBA pixel buffer (4 bytes per pixel)
    pub buffer: Vec<u8>,
}

impl PixelRenderer {
    /// Create a new pixel renderer with given viewport size
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            buffer: vec![0u8; width * height * 4],
        }
    }

    /// Render the world centered on a position
    pub fn render(
        &mut self,
        world: &World,
        materials: &Materials,
        center: Vec2,
        creatures: &[CreatureRenderData],
    ) {
        let half_width = self.width as f32 / 2.0;
        let half_height = self.height as f32 / 2.0;

        // Calculate world bounds for this viewport
        let min_x = (center.x - half_width).floor() as i32;
        let min_y = (center.y - half_height).floor() as i32;

        // Clear buffer to sky color
        for pixel in self.buffer.chunks_exact_mut(4) {
            pixel[0] = 135; // R - sky blue
            pixel[1] = 206; // G
            pixel[2] = 235; // B
            pixel[3] = 255; // A
        }

        // Render world pixels
        for screen_y in 0..self.height {
            for screen_x in 0..self.width {
                let world_x = min_x + screen_x as i32;
                let world_y = min_y + screen_y as i32;

                if let Some(pixel) = world.get_pixel(world_x, world_y) {
                    let material = materials.get(pixel.material_id);
                    let color = material.color;

                    // Flip Y for screen coordinates (world Y increases upward)
                    let flipped_y = self.height - 1 - screen_y;
                    let idx = (flipped_y * self.width + screen_x) * 4;

                    self.buffer[idx] = color[0];
                    self.buffer[idx + 1] = color[1];
                    self.buffer[idx + 2] = color[2];
                    self.buffer[idx + 3] = 255;
                }
            }
        }

        // Render creatures on top
        for creature in creatures {
            self.render_creature(creature, center);
        }
    }

    /// Render a creature's body parts
    fn render_creature(&mut self, creature: &CreatureRenderData, center: Vec2) {
        let half_width = self.width as f32 / 2.0;
        let half_height = self.height as f32 / 2.0;

        for part in &creature.body_parts {
            // Convert world position to screen position
            let screen_x = (part.position.x - center.x + half_width) as i32;
            let screen_y = (center.y - part.position.y + half_height) as i32; // Flip Y

            // Draw a filled circle for each body part
            let radius = (part.radius * 2.0).max(2.0) as i32; // Scale up for visibility
            self.draw_filled_circle(screen_x, screen_y, radius, part.color);
        }
    }

    /// Draw a filled circle at screen coordinates
    fn draw_filled_circle(&mut self, cx: i32, cy: i32, radius: i32, color: [u8; 4]) {
        for dy in -radius..=radius {
            for dx in -radius..=radius {
                if dx * dx + dy * dy <= radius * radius {
                    let x = cx + dx;
                    let y = cy + dy;

                    if x >= 0 && x < self.width as i32 && y >= 0 && y < self.height as i32 {
                        let idx = (y as usize * self.width + x as usize) * 4;
                        self.buffer[idx] = color[0];
                        self.buffer[idx + 1] = color[1];
                        self.buffer[idx + 2] = color[2];
                        self.buffer[idx + 3] = color[3];
                    }
                }
            }
        }
    }

    /// Get the pixel buffer as RGB (without alpha) for GIF encoding
    pub fn get_rgb_buffer(&self) -> Vec<u8> {
        let mut rgb = Vec::with_capacity(self.width * self.height * 3);
        for chunk in self.buffer.chunks_exact(4) {
            rgb.push(chunk[0]);
            rgb.push(chunk[1]);
            rgb.push(chunk[2]);
        }
        rgb
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pixel_renderer_creation() {
        let renderer = PixelRenderer::new(128, 128);
        assert_eq!(renderer.width, 128);
        assert_eq!(renderer.height, 128);
        assert_eq!(renderer.buffer.len(), 128 * 128 * 4);
    }

    #[test]
    fn test_rgb_buffer_conversion() {
        let renderer = PixelRenderer::new(2, 2);
        let rgb = renderer.get_rgb_buffer();
        assert_eq!(rgb.len(), 2 * 2 * 3);
    }
}
