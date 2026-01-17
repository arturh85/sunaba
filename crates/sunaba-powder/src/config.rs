//! Configuration for Sunaba Powder

use serde::{Deserialize, Serialize};

/// Main configuration for Powder Game demo
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PowderConfig {
    /// Window width in pixels
    pub window_width: u32,
    /// Window height in pixels
    pub window_height: u32,
    /// World texture size (pixels)
    pub world_size: u32,
    /// Default brush size (1-10)
    pub default_brush_size: u32,
    /// Default simulation speed multiplier (0.25-4.0)
    pub default_sim_speed: f32,
    /// Show FPS counter
    pub show_fps: bool,
    /// Show particle count
    pub show_particle_count: bool,
}

impl Default for PowderConfig {
    fn default() -> Self {
        Self {
            window_width: 1280,
            window_height: 720,
            world_size: 1024,
            default_brush_size: 3,
            default_sim_speed: 1.0,
            show_fps: true,
            show_particle_count: true,
        }
    }
}

impl PowderConfig {
    /// Load config with defaults
    pub fn load() -> Self {
        Self::default()
    }
}
