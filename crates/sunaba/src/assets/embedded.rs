//! Embedded asset loading for WASM builds
//!
//! On WASM, we embed assets at compile time since there's no filesystem access.

use anyhow::Result;

/// Embedded shader source
const SHADER_SOURCE: &str = include_str!("../../assets/shaders/shader.wgsl");

/// Embedded player sprite PNG
const PLAYER_SPRITE_PNG: &[u8] = include_bytes!("../../assets/sprites/player_sprite.png");

/// Custom asset type for PNG images (returns raw RGBA bytes)
#[derive(Debug, Clone)]
pub struct SpriteAsset {
    pub width: u32,
    pub height: u32,
    pub data: Vec<u8>,
}

/// Asset manager for WASM (uses embedded assets)
pub struct Assets;

impl Assets {
    /// Create a new asset manager (no-op for embedded assets)
    pub fn new() -> Result<Self> {
        Ok(Self)
    }

    /// Hot reload check (no-op for embedded assets)
    pub fn hot_reload(&self) {
        // No hot-reloading on WASM
    }

    /// Load the main shader
    pub fn load_shader(&self) -> Result<String> {
        Ok(SHADER_SOURCE.to_string())
    }

    /// Load the player sprite
    pub fn load_player_sprite(&self) -> Result<SpriteAsset> {
        let img = image::load_from_memory(PLAYER_SPRITE_PNG)
            .map_err(|e| anyhow::anyhow!("Failed to decode player sprite: {e}"))?
            .to_rgba8();
        Ok(SpriteAsset {
            width: img.width(),
            height: img.height(),
            data: img.into_raw(),
        })
    }
}

impl Default for Assets {
    fn default() -> Self {
        Self::new().expect("Failed to initialize embedded assets")
    }
}
