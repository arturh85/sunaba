//! World access traits for creature-world interaction
//!
//! These traits define the interface between creatures and the world,
//! allowing the creature module to be decoupled from the World implementation.

use glam::Vec2;
use sunaba_simulation::{Materials, Pixel};

/// Read-only access to world state for creature sensing
pub trait WorldAccess {
    /// Get pixel at world coordinates
    fn get_pixel(&self, x: i32, y: i32) -> Option<Pixel>;

    /// Get temperature at world coordinates (Celsius)
    fn get_temperature_at_pixel(&self, x: i32, y: i32) -> f32;

    /// Get light level at world coordinates (0-15)
    fn get_light_at(&self, x: i32, y: i32) -> Option<u8>;

    /// Get material registry
    fn materials(&self) -> &Materials;

    /// Check if position is solid (blocks movement)
    fn is_solid_at(&self, x: i32, y: i32) -> bool;

    /// Check if a circle would collide with solid materials
    fn check_circle_collision(&self, x: f32, y: f32, radius: f32) -> bool;

    /// Cast a ray and find first blocking pixel
    /// Returns (pixel_x, pixel_y, material_id) of first hit, or None if no hit within max_distance
    fn raycast(&self, from: Vec2, direction: Vec2, max_distance: f32) -> Option<(i32, i32, u16)>;

    /// Get pressure at world coordinates
    fn get_pressure_at(&self, x: i32, y: i32) -> f32;

    /// Check if creature is grounded at given body part positions
    /// positions contains (center, radius) for each body part
    fn is_creature_grounded(&self, positions: &[(Vec2, f32)]) -> bool;

    /// Get blocking pixel when moving in a direction
    /// Returns (pixel_x, pixel_y, material_id) of first blocking pixel
    fn get_blocking_pixel(
        &self,
        from: Vec2,
        direction: Vec2,
        radius: f32,
        max_distance: f32,
    ) -> Option<(i32, i32, u16)>;
}

/// Mutable access to world for creature actions (eating, mining, building)
pub trait WorldMutAccess: WorldAccess {
    /// Set pixel at world coordinates by material ID
    fn set_pixel(&mut self, x: i32, y: i32, material_id: u16);

    /// Set full pixel at world coordinates (with flags)
    fn set_pixel_full(&mut self, x: i32, y: i32, pixel: Pixel);
}
