//! Wind tool for creating pressure areas

use super::Tool;
use sunaba_core::world::World;

/// Wind tool that adds pressure to an area
pub struct WindTool {
    /// Pressure delta per application (default 15.0)
    strength: f32,
}

impl WindTool {
    /// Create a new wind tool with default strength
    pub fn new() -> Self {
        Self { strength: 15.0 }
    }

    /// Create a new wind tool with custom strength
    pub fn with_strength(strength: f32) -> Self {
        Self { strength }
    }

    /// Get the current strength
    pub fn strength(&self) -> f32 {
        self.strength
    }

    /// Set the strength
    pub fn set_strength(&mut self, strength: f32) {
        self.strength = strength;
    }
}

impl Default for WindTool {
    fn default() -> Self {
        Self::new()
    }
}

impl Tool for WindTool {
    fn name(&self) -> &str {
        "Wind"
    }

    fn apply(&self, world: &mut World, x: i32, y: i32, brush_size: u32) {
        // Apply pressure in brush area
        // The coarse grid mapping (8x8) happens inside add_pressure_at()
        let r = brush_size as i32;

        for dy in -r..=r {
            for dx in -r..=r {
                if dx * dx + dy * dy <= r * r {
                    world.add_pressure_at(x + dx, y + dy, self.strength);
                }
            }
        }
    }
}
