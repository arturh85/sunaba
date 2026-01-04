//! Multiplayer client integration with SpacetimeDB
//!
//! This module provides two different client implementations:
//! - **Native builds**: Rust SDK (spacetimedb-sdk) for direct server connection
//! - **WASM builds**: TypeScript SDK via JavaScript bindings for browser compatibility

// Native client using Rust SDK
#[cfg(all(not(target_arch = "wasm32"), feature = "multiplayer"))]
mod generated;

#[cfg(all(not(target_arch = "wasm32"), feature = "multiplayer"))]
mod client;

// WASM client using TypeScript SDK via JavaScript
#[cfg(all(target_arch = "wasm32", feature = "multiplayer"))]
mod js_client;

// Connection manager and state (both platforms)
#[cfg(feature = "multiplayer")]
mod manager;

// Metrics available on both platforms when multiplayer enabled
#[cfg(feature = "multiplayer")]
pub mod metrics;

// Re-exports
#[cfg(all(not(target_arch = "wasm32"), feature = "multiplayer"))]
pub use client::MultiplayerClient;

#[cfg(all(target_arch = "wasm32", feature = "multiplayer"))]
pub use js_client::MultiplayerClient;

#[cfg(feature = "multiplayer")]
pub use manager::{MultiplayerManager, MultiplayerState};
