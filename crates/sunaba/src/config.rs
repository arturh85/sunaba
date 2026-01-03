//! Game configuration with layered loading
//!
//! Configuration is loaded from multiple sources (lowest to highest priority):
//! 1. Compiled defaults
//! 2. `config.ron` file (if exists)
//! 3. Environment variables prefixed with `SUNABA_`
//!
//! Example environment variable: `SUNABA_CAMERA__ZOOM_SPEED=1.5`

use anyhow::{Context, Result};
use config::{Config, Environment, File};
use serde::{Deserialize, Serialize};

/// Main game configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GameConfig {
    #[serde(default)]
    pub camera: CameraConfig,

    #[serde(default)]
    pub player: PlayerConfig,

    #[serde(default)]
    pub world: WorldConfig,

    #[serde(default)]
    pub ui: UiConfig,

    #[serde(default)]
    pub debug: DebugConfig,
}

/// Camera/zoom settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CameraConfig {
    /// Multiplicative zoom factor per scroll/keypress
    pub zoom_speed: f32,
    /// Minimum zoom level (max zoom out)
    pub min_zoom: f32,
    /// Maximum zoom level (max zoom in)
    pub max_zoom: f32,
}

impl Default for CameraConfig {
    fn default() -> Self {
        Self {
            zoom_speed: 1.1,
            min_zoom: 0.002,
            max_zoom: 0.01,
        }
    }
}

/// Player physics settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerConfig {
    /// Horizontal movement speed in pixels/sec
    pub move_speed: f32,
    /// Gravity acceleration in pixels/sec^2
    pub gravity: f32,
    /// Jump velocity in pixels/sec
    pub jump_velocity: f32,
    /// Flight thrust in pixels/sec^2 (Noita-style levitation)
    pub flight_thrust: f32,
    /// Terminal velocity in pixels/sec
    pub max_fall_speed: f32,
}

impl Default for PlayerConfig {
    fn default() -> Self {
        Self {
            move_speed: 200.0,
            gravity: 800.0,
            jump_velocity: 300.0,
            flight_thrust: 1200.0,
            max_fall_speed: 500.0,
        }
    }
}

/// World simulation settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldConfig {
    /// Autosave interval in seconds
    pub autosave_interval_secs: u64,
    /// Active chunk simulation radius (radius of 3 = 7x7 grid = 49 chunks)
    pub active_chunk_radius: i32,
}

impl Default for WorldConfig {
    fn default() -> Self {
        Self {
            autosave_interval_secs: 60,
            active_chunk_radius: 3,
        }
    }
}

/// UI and window settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiConfig {
    /// Initial window width
    pub window_width: u32,
    /// Initial window height
    pub window_height: u32,
    /// Show debug stats on startup
    pub show_stats_on_start: bool,
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            window_width: 1280,
            window_height: 720,
            show_stats_on_start: false,
        }
    }
}

/// Debug/development settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DebugConfig {
    /// Allow placing materials without consuming from inventory
    pub debug_placement: bool,
    /// Enable verbose logging
    pub verbose_logging: bool,
}

impl Default for DebugConfig {
    fn default() -> Self {
        Self {
            debug_placement: true,
            verbose_logging: false,
        }
    }
}

impl GameConfig {
    /// Load configuration with layered priority:
    /// 1. Compiled defaults (lowest priority)
    /// 2. `config.ron` file (if exists)
    /// 3. Environment variables prefixed with `SUNABA_` (highest priority)
    pub fn load() -> Result<Self> {
        let builder = Config::builder()
            // Layer 1: Compiled defaults
            .set_default("camera.zoom_speed", 1.1)?
            .set_default("camera.min_zoom", 0.002)?
            .set_default("camera.max_zoom", 0.01)?
            .set_default("player.move_speed", 200.0)?
            .set_default("player.gravity", 800.0)?
            .set_default("player.jump_velocity", 300.0)?
            .set_default("player.flight_thrust", 1200.0)?
            .set_default("player.max_fall_speed", 500.0)?
            .set_default("world.autosave_interval_secs", 60_i64)?
            .set_default("world.active_chunk_radius", 3_i64)?
            .set_default("ui.window_width", 1280_i64)?
            .set_default("ui.window_height", 720_i64)?
            .set_default("ui.show_stats_on_start", false)?
            .set_default("debug.debug_placement", true)?
            .set_default("debug.verbose_logging", false)?
            // Layer 2: Config file (optional, won't error if missing)
            .add_source(
                File::with_name("config")
                    .format(config::FileFormat::Ron)
                    .required(false),
            )
            // Layer 3: Environment variables (SUNABA_CAMERA__ZOOM_SPEED, etc.)
            .add_source(Environment::with_prefix("SUNABA").separator("__"));

        let config = builder.build().context("Failed to build configuration")?;

        config
            .try_deserialize()
            .context("Failed to deserialize configuration")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = GameConfig::default();
        assert_eq!(config.camera.zoom_speed, 1.1);
        assert_eq!(config.player.move_speed, 200.0);
        assert_eq!(config.world.autosave_interval_secs, 60);
        assert_eq!(config.ui.window_width, 1280);
        assert!(config.debug.debug_placement);
    }

    #[test]
    fn test_load_config_with_defaults() {
        // Should load defaults when no config file exists
        let config = GameConfig::load().expect("Failed to load config");
        assert_eq!(config.camera.zoom_speed, 1.1);
        assert_eq!(config.player.move_speed, 200.0);
    }
}
