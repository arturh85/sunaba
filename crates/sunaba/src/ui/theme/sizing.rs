//! Responsive sizing system for UI elements
//!
//! This module provides a grid-based sizing system that scales UI elements
//! appropriately at different window sizes.

use egui::{Context, Vec2};

/// Responsive sizing system for UI scaling across different resolutions
///
/// Provides utilities for scaling UI elements based on window size while
/// maintaining readability and grid alignment.
#[derive(Debug, Clone)]
pub struct ResponsiveSizing {
    /// Base scale factor (1.0 at 1280x720 baseline)
    base_scale: f32,
    /// Current window width
    window_width: f32,
    /// Current window height
    window_height: f32,
}

impl ResponsiveSizing {
    /// Create a new responsive sizing system for the given window dimensions
    ///
    /// # Arguments
    /// * `width` - Window width in pixels
    /// * `height` - Window height in pixels
    ///
    /// # Example
    /// ```no_run
    /// use sunaba::ui::theme::ResponsiveSizing;
    /// let sizing = ResponsiveSizing::new(1920.0, 1080.0);
    /// let scaled_button_size = sizing.scale(40.0);
    /// ```
    pub fn new(width: f32, height: f32) -> Self {
        let base_scale = Self::calculate_scale(width, height);
        Self {
            base_scale,
            window_width: width,
            window_height: height,
        }
    }

    /// Calculate scale factor based on window dimensions
    ///
    /// Uses 1280x720 as baseline (1.0x scale).
    /// Takes minimum of width/height scale to ensure UI fits.
    fn calculate_scale(width: f32, height: f32) -> f32 {
        const BASELINE_WIDTH: f32 = 1280.0;
        const BASELINE_HEIGHT: f32 = 720.0;

        let scale_x = width / BASELINE_WIDTH;
        let scale_y = height / BASELINE_HEIGHT;

        // Use minimum to ensure UI fits, clamp to reasonable range
        scale_x.min(scale_y).clamp(0.5, 2.0)
    }

    /// Apply this sizing to an egui context
    ///
    /// Sets the pixels-per-point value for the entire UI.
    pub fn apply(&self, ctx: &Context) {
        ctx.set_pixels_per_point(self.base_scale);
    }

    /// Scale a base size value according to window size
    ///
    /// # Arguments
    /// * `base_size` - The size at baseline resolution (1280x720)
    ///
    /// # Returns
    /// The scaled size for current window
    pub fn scale(&self, base_size: f32) -> f32 {
        base_size * self.base_scale
    }

    /// Check if window is in small size category
    ///
    /// Small: < 1024x576
    pub fn is_small(&self) -> bool {
        self.window_width < 1024.0 || self.window_height < 576.0
    }

    /// Check if window is in large size category
    ///
    /// Large: > 1920x1080
    pub fn is_large(&self) -> bool {
        self.window_width > 1920.0 && self.window_height > 1080.0
    }

    /// Get the base scale factor
    pub fn scale_factor(&self) -> f32 {
        self.base_scale
    }

    /// Get window width
    pub fn width(&self) -> f32 {
        self.window_width
    }

    /// Get window height
    pub fn height(&self) -> f32 {
        self.window_height
    }
}

impl Default for ResponsiveSizing {
    /// Default sizing for baseline resolution (1280x720)
    fn default() -> Self {
        Self::new(1280.0, 720.0)
    }
}

/// Grid-based spacing constants for pixel-perfect alignment
pub struct GridSpacing {
    /// Base grid unit (8px for pixelart, 16px for retro)
    pub grid_unit: f32,
}

impl GridSpacing {
    /// Create 8px grid spacing (standard pixelart)
    pub fn pixelart() -> Self {
        Self { grid_unit: 8.0 }
    }

    /// Create 16px grid spacing (retro NES/SNES style)
    pub fn retro() -> Self {
        Self { grid_unit: 16.0 }
    }

    /// Snap a value to the nearest grid unit
    pub fn snap(&self, value: f32) -> f32 {
        (value / self.grid_unit).round() * self.grid_unit
    }

    /// Snap a Vec2 to the nearest grid units
    pub fn snap_vec2(&self, vec: Vec2) -> Vec2 {
        Vec2::new(self.snap(vec.x), self.snap(vec.y))
    }

    /// Get spacing for N grid units
    pub fn units(&self, n: f32) -> f32 {
        self.grid_unit * n
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_baseline_scale() {
        let sizing = ResponsiveSizing::new(1280.0, 720.0);
        assert!((sizing.scale_factor() - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_larger_resolution() {
        let sizing = ResponsiveSizing::new(1920.0, 1080.0);
        assert!(sizing.scale_factor() > 1.0);
        assert!(sizing.is_large());
    }

    #[test]
    fn test_smaller_resolution() {
        let sizing = ResponsiveSizing::new(800.0, 600.0);
        assert!(sizing.scale_factor() < 1.0);
        assert!(sizing.is_small());
    }

    #[test]
    fn test_grid_snapping() {
        let grid = GridSpacing::pixelart();
        assert_eq!(grid.snap(12.0), 8.0); // Rounds down
        assert_eq!(grid.snap(18.0), 16.0); // Rounds up
        assert_eq!(grid.snap(16.0), 16.0); // Exact
    }
}
