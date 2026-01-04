//! # Sunaba - 2D Falling Sand Physics Sandbox
//!
//! A survival game where every pixel is simulated with material properties.

pub mod animation;
pub mod app;
pub mod assets;
#[cfg(not(target_arch = "wasm32"))]
pub mod config;
pub mod hot_reload;
pub mod render;
pub mod ui;

// Headless training module (native only)
#[cfg(all(not(target_arch = "wasm32"), feature = "headless"))]
pub mod headless;

// Multiplayer module (native: Rust SDK, WASM: TypeScript SDK via JS)
#[cfg(feature = "multiplayer")]
pub mod multiplayer;

// Encoding module for chunk synchronization (multiplayer only)
#[cfg(feature = "multiplayer")]
pub mod encoding;

// Re-export core modules for convenience
pub use sunaba_core::creature;
pub use sunaba_core::entity;
pub use sunaba_core::levels;
pub use sunaba_core::simulation;
pub use sunaba_core::world;

pub use app::App;

/// Common imports for internal use
pub mod prelude {
    pub use glam::{IVec2, Vec2};
    pub use sunaba_core::simulation::{MaterialId, MaterialType, Materials};
    pub use sunaba_core::world::{CHUNK_SIZE, Chunk, Pixel, World};
}

// WASM entry point
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(start)]
pub fn start() {
    // Set up panic hook for better error messages in the browser console
    console_error_panic_hook::set_once();

    // Initialize logging for WASM
    console_log::init_with_level(log::Level::Info).expect("Failed to initialize logger");

    log::info!("Sunaba WASM module initialized");
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub async fn run() -> Result<(), JsValue> {
    log::info!("Starting Sunaba (WASM)");

    let (app, event_loop) = App::new()
        .await
        .map_err(|e| JsValue::from_str(&format!("Failed to create app: {}", e)))?;

    App::run(event_loop, app).map_err(|e| JsValue::from_str(&format!("Failed to run app: {}", e)))
}
