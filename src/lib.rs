//! # Sunaba - 2D Falling Sand Physics Sandbox
//!
//! A survival game where every pixel is simulated with material properties.

pub mod app;
pub mod creature;
pub mod entity;
pub mod levels;
pub mod physics;
pub mod render;
pub mod simulation;
pub mod ui;
pub mod world;

// Headless training module (native only)
#[cfg(all(not(target_arch = "wasm32"), feature = "headless"))]
pub mod headless;

pub use app::App;

/// Common imports for internal use
pub mod prelude {
    pub use crate::simulation::{MaterialId, MaterialType, Materials};
    pub use crate::world::{Chunk, Pixel, World, CHUNK_SIZE};
    pub use glam::{IVec2, Vec2};
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
