//! Chunk - 64x64 region of pixels

use crate::simulation::MaterialId;
use serde::{Deserialize, Serialize};

pub const CHUNK_SIZE: usize = 64;
pub const CHUNK_AREA: usize = CHUNK_SIZE * CHUNK_SIZE;

/// A single pixel in the world
#[derive(Clone, Copy, Default, Debug, Serialize, Deserialize)]
pub struct Pixel {
    /// Material type (0 = air)
    pub material_id: u16,
    /// State flags (updated this frame, burning, etc.)
    pub flags: u16,
}

impl Pixel {
    pub const AIR: Pixel = Pixel {
        material_id: 0,
        flags: 0,
    };

    pub fn new(material_id: u16) -> Self {
        Self {
            material_id,
            flags: 0,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.material_id == MaterialId::AIR
    }
}

/// Flag bits for pixel state
pub mod pixel_flags {
    pub const UPDATED: u16 = 1 << 0; // Already updated this frame
    pub const BURNING: u16 = 1 << 1; // Currently on fire
    pub const FALLING: u16 = 1 << 2; // In free-fall
    pub const PLAYER_PLACED: u16 = 1 << 3; // Placed by player/creature, not world-generated
}

/// A 64x64 region of the world
#[derive(Clone, Serialize, Deserialize)]
pub struct Chunk {
    /// Chunk coordinates (in chunk space, not pixel space)
    pub x: i32,
    pub y: i32,

    /// Pixel data, row-major order
    /// Index = y * CHUNK_SIZE + x
    #[serde(with = "serde_big_array::BigArray")]
    pixels: [Pixel; CHUNK_AREA],

    /// Temperature field (8x8 coarse grid)
    #[serde(with = "serde_big_array::BigArray")]
    pub temperature: [f32; 64],

    /// Pressure field (8x8 coarse grid)
    #[serde(with = "serde_big_array::BigArray")]
    pub pressure: [f32; 64],

    /// Light levels per pixel (0-15, where 0=dark, 15=full light)
    #[serde(with = "serde_big_array::BigArray")]
    pub light_levels: [u8; CHUNK_AREA],

    /// Whether light needs recalculation (not persisted)
    #[serde(skip)]
    pub light_dirty: bool,

    /// Whether chunk has been modified since last save (not persisted)
    #[serde(skip)]
    pub dirty: bool,

    /// Bounding rect of modified pixels (not persisted)
    #[serde(skip)]
    pub dirty_rect: Option<DirtyRect>,

    /// Whether chunk has active physics/chemistry that needs simulation (not persisted)
    /// This is separate from dirty_rect because the renderer clears dirty_rect after rendering,
    /// but we need to keep simulating chunks with active materials until they settle.
    #[serde(skip)]
    pub simulation_active: bool,
}

#[derive(Clone, Copy, Debug)]
pub struct DirtyRect {
    pub min_x: usize,
    pub min_y: usize,
    pub max_x: usize,
    pub max_y: usize,
}

impl DirtyRect {
    pub fn new(x: usize, y: usize) -> Self {
        Self {
            min_x: x,
            min_y: y,
            max_x: x,
            max_y: y,
        }
    }

    pub fn expand(&mut self, x: usize, y: usize) {
        self.min_x = self.min_x.min(x);
        self.min_y = self.min_y.min(y);
        self.max_x = self.max_x.max(x);
        self.max_y = self.max_y.max(y);
    }
}

impl Chunk {
    pub fn new(x: i32, y: i32) -> Self {
        Self {
            x,
            y,
            pixels: [Pixel::AIR; CHUNK_AREA],
            temperature: [20.0; 64],       // Room temperature (Celsius)
            pressure: [1.0; 64],           // Atmospheric pressure
            light_levels: [0; CHUNK_AREA], // Start dark, will be calculated
            light_dirty: true,             // Needs initial light calculation
            dirty: false,
            dirty_rect: None,
            simulation_active: false,
        }
    }

    /// Get pixel at local coordinates (0-63, 0-63)
    #[inline]
    pub fn get_pixel(&self, x: usize, y: usize) -> Pixel {
        debug_assert!(x < CHUNK_SIZE && y < CHUNK_SIZE);
        self.pixels[y * CHUNK_SIZE + x]
    }

    /// Get material ID at local coordinates (helper method)
    #[inline]
    pub fn get_material(&self, x: usize, y: usize) -> u16 {
        self.get_pixel(x, y).material_id
    }

    /// Count non-air pixels (for debugging save/load)
    pub fn count_non_air(&self) -> usize {
        self.pixels
            .iter()
            .filter(|p| p.material_id != crate::simulation::MaterialId::AIR)
            .count()
    }

    /// Set pixel at local coordinates
    #[inline]
    pub fn set_pixel(&mut self, x: usize, y: usize, pixel: Pixel) {
        debug_assert!(x < CHUNK_SIZE && y < CHUNK_SIZE);
        self.pixels[y * CHUNK_SIZE + x] = pixel;
        self.mark_dirty(x, y);
    }

    /// Set pixel by material ID
    #[inline]
    pub fn set_material(&mut self, x: usize, y: usize, material_id: u16) {
        self.set_pixel(x, y, Pixel::new(material_id));
    }

    /// Swap two pixels (useful for falling simulation)
    #[inline]
    pub fn swap_pixels(&mut self, x1: usize, y1: usize, x2: usize, y2: usize) {
        let idx1 = y1 * CHUNK_SIZE + x1;
        let idx2 = y2 * CHUNK_SIZE + x2;
        self.pixels.swap(idx1, idx2);
        self.mark_dirty(x1, y1);
        self.mark_dirty(x2, y2);
    }

    /// Get light level at local coordinates (0-15)
    #[inline]
    pub fn get_light(&self, x: usize, y: usize) -> u8 {
        debug_assert!(x < CHUNK_SIZE && y < CHUNK_SIZE);
        self.light_levels[y * CHUNK_SIZE + x]
    }

    /// Set light level at local coordinates (0-15)
    #[inline]
    pub fn set_light(&mut self, x: usize, y: usize, level: u8) {
        debug_assert!(x < CHUNK_SIZE && y < CHUNK_SIZE);
        debug_assert!(level <= 15);
        self.light_levels[y * CHUNK_SIZE + x] = level;
    }

    /// Mark chunk as needing light recalculation
    pub fn mark_light_dirty(&mut self) {
        self.light_dirty = true;
    }

    fn mark_dirty(&mut self, x: usize, y: usize) {
        self.dirty = true;
        self.light_dirty = true; // Pixel changes require light recalculation
        match &mut self.dirty_rect {
            Some(rect) => rect.expand(x, y),
            None => self.dirty_rect = Some(DirtyRect::new(x, y)),
        }
    }

    /// Clear dirty flags for new frame
    pub fn clear_dirty_rect(&mut self) {
        self.dirty_rect = None;
    }

    /// Clear all "updated this frame" flags from pixels
    pub fn clear_update_flags(&mut self) {
        for pixel in &mut self.pixels {
            pixel.flags &= !pixel_flags::UPDATED;
        }
    }

    /// Mark chunk as having active simulation (materials moving)
    pub fn set_simulation_active(&mut self, active: bool) {
        self.simulation_active = active;
    }

    /// Check if chunk needs continued simulation
    pub fn is_simulation_active(&self) -> bool {
        self.simulation_active
    }

    /// Get raw pixel slice for rendering
    pub fn pixels(&self) -> &[Pixel] {
        &self.pixels
    }

    /// Get temperature at coarse grid position
    pub fn get_temperature(&self, cx: usize, cy: usize) -> f32 {
        self.temperature[cy * 8 + cx]
    }

    /// Set temperature at coarse grid position
    pub fn set_temperature(&mut self, cx: usize, cy: usize, temp: f32) {
        self.temperature[cy * 8 + cx] = temp;
    }

    /// Get pressure at pixel position (using coarse 8x8 grid)
    pub fn get_pressure_at(&self, x: usize, y: usize) -> f32 {
        let (cx, cy) = (x / 8, y / 8); // Convert pixel to coarse grid coords
        self.pressure[cy * 8 + cx]
    }
}

impl Default for Chunk {
    fn default() -> Self {
        Self::new(0, 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pixel_access() {
        let mut chunk = Chunk::new(0, 0);

        chunk.set_material(10, 20, 5);
        assert_eq!(chunk.get_pixel(10, 20).material_id, 5);

        chunk.set_material(0, 0, 1);
        chunk.set_material(63, 63, 2);
        assert_eq!(chunk.get_pixel(0, 0).material_id, 1);
        assert_eq!(chunk.get_pixel(63, 63).material_id, 2);
    }

    #[test]
    fn test_dirty_rect() {
        let mut chunk = Chunk::new(0, 0);

        chunk.set_material(10, 10, 1);
        chunk.set_material(50, 50, 1);

        let rect = chunk.dirty_rect.unwrap();
        assert_eq!(rect.min_x, 10);
        assert_eq!(rect.min_y, 10);
        assert_eq!(rect.max_x, 50);
        assert_eq!(rect.max_y, 50);
    }
}
