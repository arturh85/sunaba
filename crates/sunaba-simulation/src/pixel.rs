//! Pixel types and constants
//!
//! Foundational types for the pixel-based simulation.

use crate::MaterialId;
use serde::{Deserialize, Serialize};

/// Size of a chunk in pixels (64x64)
pub const CHUNK_SIZE: usize = 64;

/// Total pixels in a chunk
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pixel_air_constant() {
        assert_eq!(Pixel::AIR.material_id, 0);
        assert_eq!(Pixel::AIR.flags, 0);
        assert!(Pixel::AIR.is_empty());
    }

    #[test]
    fn test_pixel_new() {
        let pixel = Pixel::new(MaterialId::SAND);
        assert_eq!(pixel.material_id, MaterialId::SAND);
        assert_eq!(pixel.flags, 0);
    }

    #[test]
    fn test_pixel_is_empty() {
        let air = Pixel::new(MaterialId::AIR);
        assert!(air.is_empty());

        let sand = Pixel::new(MaterialId::SAND);
        assert!(!sand.is_empty());
    }

    #[test]
    fn test_chunk_constants() {
        assert_eq!(CHUNK_SIZE, 64);
        assert_eq!(CHUNK_AREA, 64 * 64);
    }

    #[test]
    fn test_pixel_flags() {
        assert_eq!(pixel_flags::UPDATED, 1);
        assert_eq!(pixel_flags::BURNING, 2);
        assert_eq!(pixel_flags::FALLING, 4);
        assert_eq!(pixel_flags::PLAYER_PLACED, 8);
    }

    #[test]
    fn test_pixel_default() {
        let pixel = Pixel::default();
        assert_eq!(pixel.material_id, 0);
        assert_eq!(pixel.flags, 0);
    }

    #[test]
    fn test_pixel_clone() {
        let pixel = Pixel::new(MaterialId::WATER);
        let cloned = pixel;
        assert_eq!(cloned.material_id, pixel.material_id);
        assert_eq!(cloned.flags, pixel.flags);
    }
}
