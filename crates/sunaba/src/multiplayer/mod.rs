//! Multiplayer client integration with SpacetimeDB
//!
//! This module provides two different client implementations:
//! - **Native builds**: Rust SDK (spacetimedb-sdk) for direct server connection
//! - **WASM builds**: TypeScript SDK via JavaScript bindings for browser compatibility

// Native client using Rust SDK
#[cfg(all(not(target_arch = "wasm32"), feature = "multiplayer"))]
mod generated;

#[cfg(all(not(target_arch = "wasm32"), feature = "multiplayer"))]
pub(crate) mod client;

// OAuth for native builds
#[cfg(all(not(target_arch = "wasm32"), feature = "multiplayer_native"))]
mod oauth_native;

// WASM client using TypeScript SDK via JavaScript
#[cfg(all(target_arch = "wasm32", feature = "multiplayer"))]
mod js_client;

// Connection manager and state (both platforms)
#[cfg(feature = "multiplayer")]
mod manager;

// Progressive chunk loading (both platforms)
#[cfg(feature = "multiplayer")]
pub mod chunk_loader;

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

// ===== Shared OAuth Claims (Platform-Agnostic) =====

/// Shared OAuth claims type used by UI (works on both native and WASM)
#[cfg(feature = "multiplayer")]
#[derive(Debug, Clone)]
pub struct OAuthClaims {
    pub email: Option<String>,
    pub name: Option<String>,
}

// Convert WASM OAuth claims to shared type
#[cfg(all(target_arch = "wasm32", feature = "multiplayer_wasm"))]
impl From<js_client::OAuthClaims> for OAuthClaims {
    fn from(claims: js_client::OAuthClaims) -> Self {
        Self {
            email: claims.email,
            name: claims.name,
        }
    }
}

// Convert native OAuth claims to shared type
#[cfg(all(not(target_arch = "wasm32"), feature = "multiplayer_native"))]
impl From<oauth_native::OAuthClaims> for OAuthClaims {
    fn from(claims: oauth_native::OAuthClaims) -> Self {
        Self {
            email: claims.email,
            name: claims.name,
        }
    }
}
