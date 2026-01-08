//! Temporary light source manager
//!
//! Manages short-lived light sources for visual effects like:
//! - Mining flashes (brief bright light at mining point)
//! - Explosion flashes (gunpowder, TNT)
//! - Tool impacts (metal sparks)
//!
//! These lights bypass the flood-fill propagation system for performance,
//! applying direct light overrides at their world positions.

use glam::IVec2;
use std::collections::HashMap;

use crate::world::{Chunk, ChunkManager};

/// A single temporary light source
#[derive(Debug, Clone)]
struct TempLight {
    /// World position (pixel coordinates)
    world_x: i32,
    world_y: i32,
    /// Light intensity (0-15, same scale as regular lights)
    intensity: u8,
    /// Remaining frames until expiration (at 60 FPS)
    frames_remaining: u32,
}

/// Manages temporary light sources that expire after a short duration
///
/// These lights are applied directly to chunk light levels without propagation,
/// making them very cheap for short flashes.
pub struct TemporaryLightManager {
    /// Active temporary lights
    lights: Vec<TempLight>,
}

impl Default for TemporaryLightManager {
    fn default() -> Self {
        Self::new()
    }
}

impl TemporaryLightManager {
    /// Create a new temporary light manager
    pub fn new() -> Self {
        Self {
            lights: Vec::with_capacity(32), // Most scenes have few temp lights
        }
    }

    /// Add a temporary light flash at a world position
    ///
    /// # Parameters
    /// - `world_x`, `world_y` - World position in pixel coordinates
    /// - `intensity` - Light level (0-15, clamped to max)
    /// - `duration_seconds` - How long the light lasts (converted to frames at 60 FPS)
    ///
    /// # Examples
    /// ```ignore
    /// // Mining flash: bright (10), short (0.1s = 6 frames)
    /// manager.add_flash(100, 50, 10, 0.1);
    ///
    /// // Explosion flash: very bright (15), medium (0.3s = 18 frames)
    /// manager.add_flash(200, 100, 15, 0.3);
    /// ```
    pub fn add_flash(&mut self, world_x: i32, world_y: i32, intensity: u8, duration_seconds: f32) {
        // Clamp intensity to valid range (0-15)
        let intensity = intensity.min(15);

        // Convert duration to frames at 60 FPS
        let frames_remaining = (duration_seconds * 60.0).max(1.0) as u32;

        self.lights.push(TempLight {
            world_x,
            world_y,
            intensity,
            frames_remaining,
        });
    }

    /// Update all temporary lights (decrement frame counters, remove expired)
    ///
    /// Call this once per frame (60 FPS)
    pub fn update(&mut self) {
        // Decrement all frame counters
        for light in &mut self.lights {
            light.frames_remaining = light.frames_remaining.saturating_sub(1);
        }

        // Remove expired lights (frames_remaining == 0)
        self.lights.retain(|light| light.frames_remaining > 0);
    }

    /// Apply all active temporary lights to chunks
    ///
    /// This directly overrides chunk light levels for pixels where temp lights exist.
    /// Only affects pixels that exist in loaded chunks.
    ///
    /// # Parameters
    /// - `chunks` - Mutable reference to chunk map
    ///
    /// # Implementation Notes
    /// - Does NOT propagate light (for performance)
    /// - Only sets the exact pixel at the light's world position
    /// - Uses `max()` to avoid darkening existing light
    pub fn apply_to_chunks(&self, chunks: &mut HashMap<IVec2, Chunk>) {
        for light in &self.lights {
            // Convert world coords to chunk coords
            let (chunk_pos, local_x, local_y) =
                ChunkManager::world_to_chunk_coords(light.world_x, light.world_y);

            // Apply light to chunk if it exists
            if let Some(chunk) = chunks.get_mut(&chunk_pos) {
                let idx = local_y * crate::world::CHUNK_SIZE + local_x;
                if idx < chunk.light_levels.len() {
                    // Use max to avoid darkening existing light sources
                    chunk.light_levels[idx] = chunk.light_levels[idx].max(light.intensity);
                }
            }
        }
    }

    /// Get the number of active temporary lights (for debug stats)
    pub fn active_count(&self) -> usize {
        self.lights.len()
    }

    /// Clear all temporary lights
    pub fn clear(&mut self) {
        self.lights.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_flash() {
        let mut manager = TemporaryLightManager::new();
        manager.add_flash(10, 20, 12, 0.1);

        assert_eq!(manager.active_count(), 1);
    }

    #[test]
    fn test_flash_duration_conversion() {
        let mut manager = TemporaryLightManager::new();
        manager.add_flash(0, 0, 10, 0.1); // 0.1s = 6 frames at 60fps

        assert_eq!(manager.lights[0].frames_remaining, 6);
    }

    #[test]
    fn test_intensity_clamping() {
        let mut manager = TemporaryLightManager::new();
        manager.add_flash(0, 0, 255, 0.1); // Way over max

        assert_eq!(manager.lights[0].intensity, 15); // Clamped to max
    }

    #[test]
    fn test_update_decrements_frames() {
        let mut manager = TemporaryLightManager::new();
        manager.add_flash(0, 0, 10, 0.1); // 6 frames

        let initial_frames = manager.lights[0].frames_remaining;
        manager.update();
        assert_eq!(manager.lights[0].frames_remaining, initial_frames - 1);
    }

    #[test]
    fn test_expired_lights_removed() {
        let mut manager = TemporaryLightManager::new();
        manager.add_flash(0, 0, 10, 1.0 / 60.0); // 1 frame

        assert_eq!(manager.active_count(), 1);

        manager.update(); // Decrement to 0
        assert_eq!(manager.active_count(), 0); // Should be removed
    }

    #[test]
    fn test_multiple_lights() {
        let mut manager = TemporaryLightManager::new();
        manager.add_flash(0, 0, 10, 0.1);
        manager.add_flash(10, 10, 12, 0.2);
        manager.add_flash(20, 20, 8, 0.05);

        assert_eq!(manager.active_count(), 3);
    }

    #[test]
    fn test_clear() {
        let mut manager = TemporaryLightManager::new();
        manager.add_flash(0, 0, 10, 0.1);
        manager.add_flash(10, 10, 12, 0.2);

        manager.clear();
        assert_eq!(manager.active_count(), 0);
    }

    #[test]
    fn test_zero_duration_gives_one_frame() {
        let mut manager = TemporaryLightManager::new();
        manager.add_flash(0, 0, 10, 0.0); // 0 seconds

        // Should get at least 1 frame (max with 1.0)
        assert_eq!(manager.lights[0].frames_remaining, 1);
    }
}
