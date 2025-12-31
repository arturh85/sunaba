//! Temperature simulation system
//!
//! Manages heat diffusion across the 8x8 temperature grid within each chunk.
//! Hot materials (fire, lava) heat their surroundings, and temperature spreads
//! to neighboring cells over time.

use crate::world::Chunk;
use std::collections::HashMap;
use glam::IVec2;

/// Temperature simulator with 30fps throttling
pub struct TemperatureSimulator {
    /// Counter for throttling updates to 30fps (every 2 frames at 60fps)
    update_counter: u8,
}

impl TemperatureSimulator {
    pub fn new() -> Self {
        Self {
            update_counter: 0,
        }
    }

    /// Update temperature diffusion for all chunks
    /// Throttled to 30fps for performance
    pub fn update(&mut self, chunks: &mut HashMap<IVec2, Chunk>) {
        // Throttle to 30fps (every 2 frames at 60fps)
        self.update_counter += 1;
        if self.update_counter < 2 {
            return;
        }
        self.update_counter = 0;

        // For each chunk, diffuse temperature internally
        for chunk in chunks.values_mut() {
            self.diffuse_chunk_temperature(chunk);
        }
    }

    /// Diffuse temperature within a single chunk
    fn diffuse_chunk_temperature(&self, chunk: &mut Chunk) {
        const DIFFUSION_RATE: f32 = 0.1; // 0.0 - 1.0, how fast heat spreads

        let mut new_temps = chunk.temperature;

        // Update each temperature cell (8x8 grid)
        for cy in 0..8 {
            for cx in 0..8 {
                let idx = cy * 8 + cx;
                let current_temp = chunk.temperature[idx];

                // Average with 4 neighbors (von Neumann neighborhood)
                let mut neighbor_sum = 0.0;
                let mut neighbor_count = 0;

                for (dx, dy) in [(-1, 0), (1, 0), (0, -1), (0, 1)] {
                    let nx = cx as i32 + dx;
                    let ny = cy as i32 + dy;

                    if (0..8).contains(&nx) && (0..8).contains(&ny) {
                        neighbor_sum += chunk.temperature[ny as usize * 8 + nx as usize];
                        neighbor_count += 1;
                    }
                    // Note: Cross-chunk diffusion skipped for MVP simplicity
                    // Chunk boundaries act as insulated barriers for now
                }

                if neighbor_count > 0 {
                    let avg_neighbor = neighbor_sum / neighbor_count as f32;
                    // Diffuse toward average of neighbors
                    new_temps[idx] = current_temp + (avg_neighbor - current_temp) * DIFFUSION_RATE;
                }
            }
        }

        chunk.temperature = new_temps;
    }
}

impl Default for TemperatureSimulator {
    fn default() -> Self {
        Self::new()
    }
}

/// Convert pixel coordinates (0-63) to temperature cell coordinates (0-7)
/// Each temperature cell covers 8x8 pixels
#[inline]
pub fn pixel_to_temp_coords(pixel_x: usize, pixel_y: usize) -> (usize, usize) {
    (pixel_x / 8, pixel_y / 8)
}

/// Convert temperature cell coordinates to array index
#[inline]
pub fn temp_to_index(cx: usize, cy: usize) -> usize {
    cy * 8 + cx // Row-major order
}

/// Add heat to the temperature cell containing the given pixel
pub fn add_heat_at_pixel(chunk: &mut Chunk, x: usize, y: usize, heat: f32) {
    let (cx, cy) = pixel_to_temp_coords(x, y);
    let idx = temp_to_index(cx, cy);
    chunk.temperature[idx] += heat;
}

/// Get temperature at the cell containing the given pixel
pub fn get_temperature_at_pixel(chunk: &Chunk, x: usize, y: usize) -> f32 {
    let (cx, cy) = pixel_to_temp_coords(x, y);
    let idx = temp_to_index(cx, cy);
    chunk.temperature[idx]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pixel_to_temp_coords() {
        assert_eq!(pixel_to_temp_coords(0, 0), (0, 0));
        assert_eq!(pixel_to_temp_coords(7, 7), (0, 0));
        assert_eq!(pixel_to_temp_coords(8, 8), (1, 1));
        assert_eq!(pixel_to_temp_coords(63, 63), (7, 7));
    }

    #[test]
    fn test_temp_to_index() {
        assert_eq!(temp_to_index(0, 0), 0);
        assert_eq!(temp_to_index(1, 0), 1);
        assert_eq!(temp_to_index(0, 1), 8);
        assert_eq!(temp_to_index(7, 7), 63);
    }

    #[test]
    fn test_add_and_get_temperature() {
        let mut chunk = Chunk::new(0, 0);

        // Initial temperature should be room temp (20.0)
        assert_eq!(get_temperature_at_pixel(&chunk, 0, 0), 20.0);

        // Add heat
        add_heat_at_pixel(&mut chunk, 0, 0, 100.0);
        assert_eq!(get_temperature_at_pixel(&chunk, 0, 0), 120.0);

        // All pixels in same 8x8 cell share temperature
        assert_eq!(get_temperature_at_pixel(&chunk, 7, 7), 120.0);

        // Different cell unaffected
        assert_eq!(get_temperature_at_pixel(&chunk, 8, 8), 20.0);
    }
}
