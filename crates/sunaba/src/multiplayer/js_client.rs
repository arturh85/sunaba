//! JavaScript/TypeScript SpacetimeDB SDK bindings for WASM
//!
//! This module provides Rust bindings to the SpacetimeDB TypeScript SDK
//! running in the browser via wasm-bindgen.

use wasm_bindgen::prelude::*;

// Import JavaScript functions from the global window object
// These will be implemented in web/index.html using the TypeScript SDK
#[wasm_bindgen]
extern "C" {
    /// Connect to SpacetimeDB server
    #[wasm_bindgen(js_namespace = ["window", "spacetimeClient"], js_name = "connect", catch)]
    async fn js_connect(host: &str, db_name: &str) -> Result<JsValue, JsValue>;

    /// Subscribe to world state tables
    #[wasm_bindgen(js_namespace = ["window", "spacetimeClient"], js_name = "subscribeWorld", catch)]
    async fn js_subscribe_world() -> Result<(), JsValue>;

    /// Send player position update
    #[wasm_bindgen(js_namespace = ["window", "spacetimeClient"], js_name = "updatePlayerPosition", catch)]
    fn js_update_player_position(x: f32, y: f32) -> Result<(), JsValue>;

    /// Send material placement
    #[wasm_bindgen(js_namespace = ["window", "spacetimeClient"], js_name = "placeMaterial", catch)]
    fn js_place_material(x: i32, y: i32, material_id: u16) -> Result<(), JsValue>;

    /// Send mining action
    #[wasm_bindgen(js_namespace = ["window", "spacetimeClient"], js_name = "mine", catch)]
    fn js_mine(x: i32, y: i32) -> Result<(), JsValue>;

    /// Check if connected to server
    #[wasm_bindgen(js_namespace = ["window", "spacetimeClient"], js_name = "isConnected")]
    fn js_is_connected() -> bool;
}

/// SpacetimeDB client wrapper for WASM (uses TypeScript SDK via JavaScript)
pub struct MultiplayerClient {
    connected: bool,
}

impl MultiplayerClient {
    /// Create a new multiplayer client (not yet connected)
    pub fn new() -> Self {
        Self { connected: false }
    }

    /// Connect to SpacetimeDB server
    pub async fn connect(
        &mut self,
        host: impl Into<String>,
        db_name: impl Into<String>,
    ) -> anyhow::Result<()> {
        let host = host.into();
        let db_name = db_name.into();

        log::info!(
            "Connecting to SpacetimeDB at {}/{} (via JS SDK)",
            host,
            db_name
        );

        js_connect(&host, &db_name)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to connect: {:?}", e))?;

        self.connected = true;
        log::info!("Connected to SpacetimeDB via JavaScript SDK");

        Ok(())
    }

    /// Subscribe to world state (chunks, players, creatures)
    pub async fn subscribe_world(&mut self) -> anyhow::Result<()> {
        log::info!("Subscribing to world state tables (via JS SDK)");

        js_subscribe_world()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to subscribe: {:?}", e))?;

        log::info!("Subscribed to world state");

        Ok(())
    }

    /// Send player position update to server
    pub fn update_player_position(&self, x: f32, y: f32) -> anyhow::Result<()> {
        if !self.connected {
            anyhow::bail!("Not connected to server");
        }

        js_update_player_position(x, y)
            .map_err(|e| anyhow::anyhow!("Failed to update player position: {:?}", e))?;

        Ok(())
    }

    /// Request material placement at position
    pub fn place_material(&self, x: i32, y: i32, material_id: u16) -> anyhow::Result<()> {
        if !self.connected {
            anyhow::bail!("Not connected to server");
        }

        js_place_material(x, y, material_id)
            .map_err(|e| anyhow::anyhow!("Failed to place material: {:?}", e))?;

        Ok(())
    }

    /// Request mining at position
    pub fn mine(&self, x: i32, y: i32) -> anyhow::Result<()> {
        if !self.connected {
            anyhow::bail!("Not connected to server");
        }

        js_mine(x, y).map_err(|e| anyhow::anyhow!("Failed to mine: {:?}", e))?;

        Ok(())
    }

    /// Get chunk data from local cache (for rendering)
    ///
    /// Note: Chunk data flows through subscription callbacks in JavaScript
    pub fn get_chunk(&self, _x: i32, _y: i32) -> Option<Vec<u8>> {
        // TODO: Implement chunk cache access via JavaScript
        None
    }

    /// Check if connected to server
    pub fn is_connected(&self) -> bool {
        self.connected && js_is_connected()
    }

    /// Disconnect from server
    pub async fn disconnect(&mut self) -> anyhow::Result<()> {
        log::info!("Disconnecting from SpacetimeDB (JS SDK)");
        self.connected = false;
        Ok(())
    }
}

impl Default for MultiplayerClient {
    fn default() -> Self {
        Self::new()
    }
}
