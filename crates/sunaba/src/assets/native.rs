//! Native asset loading with hot-reloading support using assets_manager

use anyhow::Result;
use assets_manager::{AssetCache, BoxedError, asset::FileAsset};
use std::borrow::Cow;

/// Custom asset type for WGSL shaders
#[derive(Debug, Clone)]
pub struct ShaderAsset {
    pub source: String,
}

impl FileAsset for ShaderAsset {
    const EXTENSION: &'static str = "wgsl";

    fn from_bytes(bytes: Cow<'_, [u8]>) -> std::result::Result<Self, BoxedError> {
        let source = std::str::from_utf8(&bytes)
            .map_err(|e| format!("Invalid UTF-8 in shader: {e}"))?
            .to_string();
        Ok(Self { source })
    }
}

/// Custom asset type for PNG images (returns raw RGBA bytes)
#[derive(Debug, Clone)]
pub struct SpriteAsset {
    pub width: u32,
    pub height: u32,
    pub data: Vec<u8>,
}

impl FileAsset for SpriteAsset {
    const EXTENSION: &'static str = "png";

    fn from_bytes(bytes: Cow<'_, [u8]>) -> std::result::Result<Self, BoxedError> {
        let img = image::load_from_memory(&bytes)
            .map_err(|e| format!("Failed to decode image: {e}"))?
            .to_rgba8();
        Ok(Self {
            width: img.width(),
            height: img.height(),
            data: img.into_raw(),
        })
    }
}

/// Asset manager for the game
pub struct Assets {
    cache: AssetCache,
}

impl Assets {
    /// Create a new asset manager loading from the specified directory
    pub fn new() -> Result<Self> {
        // Find the assets directory - try multiple paths for different run contexts
        let paths = ["crates/sunaba/assets", "assets", "../sunaba/assets"];

        for path in paths {
            if std::path::Path::new(path).exists() {
                log::info!("Loading assets from: {}", path);
                return Ok(Self {
                    cache: AssetCache::new(path)
                        .map_err(|e| anyhow::anyhow!("Failed to create asset cache: {e}"))?,
                });
            }
        }

        anyhow::bail!("Could not find assets directory. Tried: {:?}", paths)
    }

    /// Check for and apply hot-reloaded changes
    /// Hot-reloading happens automatically when loading assets
    pub fn hot_reload(&self) {
        // In assets_manager 0.13+, hot-reloading is automatic
        // Assets are automatically updated when files change on disk
    }

    /// Load the main shader
    pub fn load_shader(&self) -> Result<String> {
        let handle = self
            .cache
            .load::<ShaderAsset>("shaders.shader")
            .map_err(|e| anyhow::anyhow!("Failed to load shader: {e}"))?;
        Ok(handle.read().source.clone())
    }

    /// Load the player sprite
    pub fn load_player_sprite(&self) -> Result<SpriteAsset> {
        let handle = self
            .cache
            .load::<SpriteAsset>("sprites.player_sprite")
            .map_err(|e| anyhow::anyhow!("Failed to load player sprite: {e}"))?;
        Ok(handle.read().clone())
    }
}

impl Default for Assets {
    fn default() -> Self {
        Self::new().expect("Failed to initialize assets")
    }
}
