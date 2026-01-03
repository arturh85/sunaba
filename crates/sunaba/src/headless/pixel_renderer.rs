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

        // Clear buffer to black background (better visibility for training GIFs)
        for pixel in self.buffer.chunks_exact_mut(4) {
            pixel[0] = 0; // R - black
            pixel[1] = 0; // G
            pixel[2] = 0; // B
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
    pub fn draw_filled_circle(&mut self, cx: i32, cy: i32, radius: i32, color: [u8; 4]) {
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

    /// Set a single pixel at screen coordinates
    pub fn set_pixel(&mut self, x: i32, y: i32, color: [u8; 4]) {
        if x >= 0 && x < self.width as i32 && y >= 0 && y < self.height as i32 {
            let idx = (y as usize * self.width + x as usize) * 4;
            self.buffer[idx] = color[0];
            self.buffer[idx + 1] = color[1];
            self.buffer[idx + 2] = color[2];
            self.buffer[idx + 3] = color[3];
        }
    }

    /// Draw a line using Bresenham's algorithm
    pub fn draw_line(&mut self, x0: i32, y0: i32, x1: i32, y1: i32, color: [u8; 4]) {
        let dx = (x1 - x0).abs();
        let dy = -(y1 - y0).abs();
        let sx = if x0 < x1 { 1 } else { -1 };
        let sy = if y0 < y1 { 1 } else { -1 };
        let mut err = dx + dy;
        let mut x = x0;
        let mut y = y0;

        loop {
            self.set_pixel(x, y, color);
            if x == x1 && y == y1 {
                break;
            }
            let e2 = 2 * err;
            if e2 >= dy {
                err += dy;
                x += sx;
            }
            if e2 <= dx {
                err += dx;
                y += sy;
            }
        }
    }

    /// Draw a velocity arrow from a point
    pub fn draw_arrow(
        &mut self,
        start_x: i32,
        start_y: i32,
        vel_x: f32,
        vel_y: f32,
        scale: f32,
        color: [u8; 4],
    ) {
        let end_x = start_x + (vel_x * scale) as i32;
        let end_y = start_y - (vel_y * scale) as i32; // Flip Y for screen coords

        // Draw main line
        self.draw_line(start_x, start_y, end_x, end_y, color);

        // Draw arrowhead if velocity is significant
        let len = (vel_x * vel_x + vel_y * vel_y).sqrt();
        if len > 1.0 {
            let dir_x = vel_x / len;
            let dir_y = -vel_y / len; // Flip Y

            // Perpendicular direction
            let perp_x = -dir_y;
            let perp_y = dir_x;

            // Arrowhead size
            let head_len = 4.0;
            let head_width = 2.5;

            // Arrowhead points
            let back_x = end_x as f32 - dir_x * head_len;
            let back_y = end_y as f32 - dir_y * head_len;

            let left_x = (back_x + perp_x * head_width) as i32;
            let left_y = (back_y + perp_y * head_width) as i32;
            let right_x = (back_x - perp_x * head_width) as i32;
            let right_y = (back_y - perp_y * head_width) as i32;

            self.draw_line(end_x, end_y, left_x, left_y, color);
            self.draw_line(end_x, end_y, right_x, right_y, color);
        }
    }

    /// Draw text using a simple 5x7 bitmap font
    pub fn draw_text(&mut self, x: i32, y: i32, text: &str, color: [u8; 4]) {
        let mut cursor_x = x;
        for c in text.chars() {
            self.draw_char(cursor_x, y, c, color);
            cursor_x += 6; // 5 pixels wide + 1 pixel spacing
        }
    }

    /// Draw a single character from 5x7 bitmap font
    fn draw_char(&mut self, x: i32, y: i32, c: char, color: [u8; 4]) {
        let glyph = get_font_glyph(c);
        for (row, &bits) in glyph.iter().enumerate() {
            for col in 0..5 {
                if bits & (1 << (4 - col)) != 0 {
                    self.set_pixel(x + col, y + row as i32, color);
                }
            }
        }
    }

    /// Draw a horizontal dashed line
    pub fn draw_dashed_hline(&mut self, y: i32, dash_len: i32, gap_len: i32, color: [u8; 4]) {
        let mut x = 0i32;
        let mut drawing = true;
        let mut counter = 0;

        while x < self.width as i32 {
            if drawing {
                self.set_pixel(x, y, color);
            }
            counter += 1;
            if drawing && counter >= dash_len {
                drawing = false;
                counter = 0;
            } else if !drawing && counter >= gap_len {
                drawing = true;
                counter = 0;
            }
            x += 1;
        }
    }

    /// Draw a vertical dashed line
    pub fn draw_dashed_vline(&mut self, x: i32, dash_len: i32, gap_len: i32, color: [u8; 4]) {
        let mut y = 0i32;
        let mut drawing = true;
        let mut counter = 0;

        while y < self.height as i32 {
            if drawing {
                self.set_pixel(x, y, color);
            }
            counter += 1;
            if drawing && counter >= dash_len {
                drawing = false;
                counter = 0;
            } else if !drawing && counter >= gap_len {
                drawing = true;
                counter = 0;
            }
            y += 1;
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

/// Get a 5x7 bitmap font glyph for a character
/// Each byte represents one row, with bits 4-0 being columns left-to-right
fn get_font_glyph(c: char) -> [u8; 7] {
    match c {
        '0' => [
            0b01110, 0b10001, 0b10011, 0b10101, 0b11001, 0b10001, 0b01110,
        ],
        '1' => [
            0b00100, 0b01100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110,
        ],
        '2' => [
            0b01110, 0b10001, 0b00001, 0b00110, 0b01000, 0b10000, 0b11111,
        ],
        '3' => [
            0b01110, 0b10001, 0b00001, 0b00110, 0b00001, 0b10001, 0b01110,
        ],
        '4' => [
            0b00010, 0b00110, 0b01010, 0b10010, 0b11111, 0b00010, 0b00010,
        ],
        '5' => [
            0b11111, 0b10000, 0b11110, 0b00001, 0b00001, 0b10001, 0b01110,
        ],
        '6' => [
            0b00110, 0b01000, 0b10000, 0b11110, 0b10001, 0b10001, 0b01110,
        ],
        '7' => [
            0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b01000, 0b01000,
        ],
        '8' => [
            0b01110, 0b10001, 0b10001, 0b01110, 0b10001, 0b10001, 0b01110,
        ],
        '9' => [
            0b01110, 0b10001, 0b10001, 0b01111, 0b00001, 0b00010, 0b01100,
        ],
        'A' => [
            0b01110, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001,
        ],
        'B' => [
            0b11110, 0b10001, 0b10001, 0b11110, 0b10001, 0b10001, 0b11110,
        ],
        'C' => [
            0b01110, 0b10001, 0b10000, 0b10000, 0b10000, 0b10001, 0b01110,
        ],
        'D' => [
            0b11110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b11110,
        ],
        'E' => [
            0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b11111,
        ],
        'F' => [
            0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b10000,
        ],
        'G' => [
            0b01110, 0b10001, 0b10000, 0b10111, 0b10001, 0b10001, 0b01110,
        ],
        'H' => [
            0b10001, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001,
        ],
        'I' => [
            0b01110, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110,
        ],
        'J' => [
            0b00111, 0b00010, 0b00010, 0b00010, 0b00010, 0b10010, 0b01100,
        ],
        'K' => [
            0b10001, 0b10010, 0b10100, 0b11000, 0b10100, 0b10010, 0b10001,
        ],
        'L' => [
            0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b11111,
        ],
        'M' => [
            0b10001, 0b11011, 0b10101, 0b10101, 0b10001, 0b10001, 0b10001,
        ],
        'N' => [
            0b10001, 0b11001, 0b10101, 0b10011, 0b10001, 0b10001, 0b10001,
        ],
        'O' => [
            0b01110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110,
        ],
        'P' => [
            0b11110, 0b10001, 0b10001, 0b11110, 0b10000, 0b10000, 0b10000,
        ],
        'Q' => [
            0b01110, 0b10001, 0b10001, 0b10001, 0b10101, 0b10010, 0b01101,
        ],
        'R' => [
            0b11110, 0b10001, 0b10001, 0b11110, 0b10100, 0b10010, 0b10001,
        ],
        'S' => [
            0b01110, 0b10001, 0b10000, 0b01110, 0b00001, 0b10001, 0b01110,
        ],
        'T' => [
            0b11111, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100,
        ],
        'U' => [
            0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110,
        ],
        'V' => [
            0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01010, 0b00100,
        ],
        'W' => [
            0b10001, 0b10001, 0b10001, 0b10101, 0b10101, 0b10101, 0b01010,
        ],
        'X' => [
            0b10001, 0b10001, 0b01010, 0b00100, 0b01010, 0b10001, 0b10001,
        ],
        'Y' => [
            0b10001, 0b10001, 0b01010, 0b00100, 0b00100, 0b00100, 0b00100,
        ],
        'Z' => [
            0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b10000, 0b11111,
        ],
        ':' => [
            0b00000, 0b00100, 0b00100, 0b00000, 0b00100, 0b00100, 0b00000,
        ],
        '.' => [
            0b00000, 0b00000, 0b00000, 0b00000, 0b00000, 0b00100, 0b00100,
        ],
        '-' => [
            0b00000, 0b00000, 0b00000, 0b11111, 0b00000, 0b00000, 0b00000,
        ],
        '+' => [
            0b00000, 0b00100, 0b00100, 0b11111, 0b00100, 0b00100, 0b00000,
        ],
        ' ' => [
            0b00000, 0b00000, 0b00000, 0b00000, 0b00000, 0b00000, 0b00000,
        ],
        '/' => [
            0b00001, 0b00010, 0b00010, 0b00100, 0b01000, 0b01000, 0b10000,
        ],
        _ => [
            0b11111, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b11111,
        ], // Box for unknown
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
