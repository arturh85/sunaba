//! Animation and tweening system for smooth transitions.
//!
//! Provides `AnimatedValue<T>` for generic value interpolation with easing,
//! and `AnimatedCamera` for smooth camera follow and zoom.

mod camera;
mod tweens;

pub use camera::AnimatedCamera;
pub use tweens::{AnimatedValue, EaseType, Tweenable};
