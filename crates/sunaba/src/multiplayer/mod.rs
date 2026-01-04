//! Multiplayer client integration with SpacetimeDB
//!
//! This module provides two different client implementations:
//! - **Native builds**: Rust SDK (spacetimedb-sdk) for direct server connection
//! - **WASM builds**: TypeScript SDK via JavaScript bindings for browser compatibility

// Native client using Rust SDK
#[cfg(all(not(target_arch = "wasm32"), feature = "multiplayer_native"))]
mod client;

#[cfg(all(not(target_arch = "wasm32"), feature = "multiplayer_native"))]
pub use client::MultiplayerClient;

// WASM client using TypeScript SDK via JavaScript
#[cfg(all(target_arch = "wasm32", feature = "multiplayer_wasm"))]
mod js_client;

#[cfg(all(target_arch = "wasm32", feature = "multiplayer_wasm"))]
pub use js_client::MultiplayerClient;
