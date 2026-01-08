//! Game configuration with layered loading
//!
//! Native: Loaded from multiple sources (1. defaults, 2. config.ron, 3. env vars)
//! WASM: Loaded from localStorage with fallback to defaults
//!
//! Example environment variable: `SUNABA_CAMERA__ZOOM_SPEED=1.5`

use anyhow::{Context, Result};
#[cfg(not(target_arch = "wasm32"))]
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

    #[serde(default)]
    pub rendering: RenderingConfig,

    #[serde(default)]
    #[cfg(feature = "multiplayer")]
    pub multiplayer: MultiplayerConfig,
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
    /// UI theme variant ("cozy_alchemist", "dark_cavern", "pixel_adventure")
    pub theme: String,
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            window_width: 1280,
            window_height: 720,
            show_stats_on_start: false,
            theme: "cozy_alchemist".to_string(),
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
    /// Brush radius for material placement (1-10)
    pub brush_size: u32,
}

impl Default for DebugConfig {
    fn default() -> Self {
        Self {
            debug_placement: true,
            verbose_logging: false,
            brush_size: 1,
        }
    }
}

/// Rendering/post-processing settings
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct RenderingConfig {
    /// Scanline effect intensity (0.0 = off, 0.5 = strong)
    pub scanline_intensity: f32,
    /// Vignette darkening intensity (0.0 = off, 0.5 = strong)
    pub vignette_intensity: f32,
    /// Bloom/glow intensity (0.0 = off, 1.0 = strong)
    pub bloom_intensity: f32,
    /// Water noise frequency (spatial detail, 0.01-0.2)
    pub water_noise_frequency: f32,
    /// Water noise speed (animation speed, 0.5-5.0)
    pub water_noise_speed: f32,
    /// Water noise amplitude (color variation, 0.0-0.2)
    pub water_noise_amplitude: f32,
    /// Lava noise frequency (spatial detail, 0.01-0.15)
    pub lava_noise_frequency: f32,
    /// Lava noise speed (animation speed, 0.5-3.0)
    pub lava_noise_speed: f32,
    /// Lava noise amplitude (glow variation, 0.0-0.3)
    pub lava_noise_amplitude: f32,
    /// Enable multi-pass bloom
    pub bloom_enabled: bool,
    /// Bloom quality (3=Low, 4=Medium, 5=High mip levels)
    pub bloom_quality: u32,
    /// Bloom threshold (brightness threshold, 0.4-0.8)
    pub bloom_threshold: f32,
}

impl Default for RenderingConfig {
    fn default() -> Self {
        Self {
            scanline_intensity: 0.15,
            vignette_intensity: 0.25,
            bloom_intensity: 0.3,
            water_noise_frequency: 0.08,
            water_noise_speed: 2.0,
            water_noise_amplitude: 0.06,
            lava_noise_frequency: 0.05,
            lava_noise_speed: 1.5,
            lava_noise_amplitude: 0.12,
            bloom_enabled: false, // Disabled by default (can enable via UI)
            bloom_quality: 4,     // Medium quality (4 mip levels)
            bloom_threshold: 0.6,
        }
    }
}

impl GameConfig {
    /// Load configuration with layered priority:
    /// Native: 1. Compiled defaults, 2. config.ron file, 3. Environment variables
    /// WASM: 1. Compiled defaults, 2. localStorage
    #[cfg(not(target_arch = "wasm32"))]
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
            .set_default("debug.brush_size", 1_i64)?
            .set_default("rendering.scanline_intensity", 0.15)?
            .set_default("rendering.vignette_intensity", 0.25)?
            .set_default("rendering.bloom_intensity", 0.3)?
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

    /// WASM: Load from localStorage, fall back to defaults
    #[cfg(target_arch = "wasm32")]
    pub fn load() -> Result<Self> {
        // Try to get localStorage
        let window = web_sys::window().ok_or_else(|| anyhow::anyhow!("No window"))?;
        let storage = window
            .local_storage()
            .map_err(|e| anyhow::anyhow!("localStorage error: {:?}", e))?
            .ok_or_else(|| anyhow::anyhow!("localStorage not available"))?;

        // Try to load from localStorage
        match storage.get_item("sunaba_config") {
            Ok(Some(config_str)) => {
                // Parse RON config
                match ron::from_str(&config_str) {
                    Ok(config) => {
                        log::info!("Loaded config from localStorage");
                        Ok(config)
                    }
                    Err(e) => {
                        log::warn!(
                            "Failed to parse config from localStorage: {}, using defaults",
                            e
                        );
                        Ok(Self::default())
                    }
                }
            }
            _ => {
                log::info!("No config in localStorage, using defaults");
                Ok(Self::default())
            }
        }
    }

    /// Save configuration (native: to file, WASM: to localStorage)
    #[cfg(not(target_arch = "wasm32"))]
    pub fn save(&self) -> Result<()> {
        let config_str = ron::ser::to_string_pretty(self, ron::ser::PrettyConfig::default())
            .context("Failed to serialize config")?;
        std::fs::write("config.ron", config_str).context("Failed to write config.ron")?;
        log::info!("Saved config to config.ron");
        Ok(())
    }

    /// WASM: Save to localStorage
    #[cfg(target_arch = "wasm32")]
    pub fn save(&self) -> Result<()> {
        let config_str = ron::ser::to_string_pretty(self, ron::ser::PrettyConfig::default())
            .context("Failed to serialize config")?;

        let window = web_sys::window().ok_or_else(|| anyhow::anyhow!("No window"))?;
        let storage = window
            .local_storage()
            .map_err(|e| anyhow::anyhow!("localStorage error: {:?}", e))?
            .ok_or_else(|| anyhow::anyhow!("localStorage not available"))?;

        storage
            .set_item("sunaba_config", &config_str)
            .map_err(|e| anyhow::anyhow!("Failed to save to localStorage: {:?}", e))?;

        log::info!("Saved config to localStorage");
        Ok(())
    }
}

/// Multiplayer configuration
#[cfg(feature = "multiplayer")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultiplayerConfig {
    pub servers: Vec<ServerEntry>,
    pub last_server: Option<String>,
    pub connection_timeout_secs: u64,
    pub reconnect_max_attempts: u32,
    pub reconnect_max_delay_secs: u64,
}

#[cfg(feature = "multiplayer")]
impl Default for MultiplayerConfig {
    fn default() -> Self {
        Self {
            servers: vec![
                ServerEntry {
                    name: "Local Dev Server".to_string(),
                    url: "http://localhost:3000".to_string(),
                },
                ServerEntry {
                    name: "Official Server".to_string(),
                    url: "http://sunaba.app42.blue".to_string(),
                },
            ],
            last_server: Some("http://sunaba.app42.blue".to_string()),
            connection_timeout_secs: 10,
            reconnect_max_attempts: 10,
            reconnect_max_delay_secs: 30,
        }
    }
}

/// Server entry in multiplayer config
#[cfg(feature = "multiplayer")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerEntry {
    pub name: String,
    pub url: String,
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
