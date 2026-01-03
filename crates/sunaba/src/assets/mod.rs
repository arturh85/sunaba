//! Asset management with hot-reloading support (native only)
//!
//! On native platforms, assets are loaded from the filesystem with automatic hot-reloading.
//! On WASM, assets are embedded at compile time.

#[cfg(not(target_arch = "wasm32"))]
mod native;

#[cfg(not(target_arch = "wasm32"))]
pub use native::*;

#[cfg(target_arch = "wasm32")]
mod embedded;

#[cfg(target_arch = "wasm32")]
pub use embedded::*;
