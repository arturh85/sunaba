//! Animated camera with smooth follow and zoom.

use glam::Vec2;

use super::tweens::{AnimatedValue, EaseType};

/// Camera with animated position and zoom for smooth movement.
///
/// This wraps the camera state with animation, providing smooth following
/// of targets and eased zoom transitions.
pub struct AnimatedCamera {
    /// Animated position (world coordinates)
    position: AnimatedValue<[f32; 2]>,
    /// Animated zoom level
    zoom: AnimatedValue<f32>,
    /// Smoothing factor for follow mode (lower = faster response)
    /// This is expressed as the time constant in seconds.
    /// A value of 0.1 means the camera reaches ~63% of the way to target in 0.1s.
    follow_smoothness: f32,
}

impl AnimatedCamera {
    /// Create a new animated camera at the given position and zoom.
    pub fn new(initial_pos: Vec2, initial_zoom: f32) -> Self {
        Self {
            position: AnimatedValue::new([initial_pos.x, initial_pos.y]),
            zoom: AnimatedValue::new(initial_zoom),
            follow_smoothness: 0.1, // Reach ~63% in 0.1 seconds
        }
    }

    /// Set the follow smoothness factor.
    ///
    /// Lower values = faster camera response (more responsive, less smooth)
    /// Higher values = slower camera response (more smooth, less responsive)
    ///
    /// Reasonable range: 0.05 (snappy) to 0.3 (floaty)
    pub fn set_follow_smoothness(&mut self, smoothness: f32) {
        self.follow_smoothness = smoothness.max(0.001);
    }

    /// Smoothly follow a target position.
    ///
    /// Uses exponential smoothing for natural-feeling camera follow.
    /// Call this every frame with the current target position.
    pub fn follow_target(&mut self, target: Vec2, dt: f32) {
        let current = self.position.value();
        let current_vec = Vec2::new(current[0], current[1]);

        // Exponential smoothing: move a fraction of the remaining distance each frame
        // The formula `1 - e^(-dt/tau)` gives frame-rate-independent smoothing
        let t = 1.0 - (-dt / self.follow_smoothness).exp();
        let new_pos = current_vec.lerp(target, t);

        self.position.set_immediate([new_pos.x, new_pos.y]);
    }

    /// Smoothly zoom to a new level with easing.
    ///
    /// # Arguments
    /// * `target_zoom` - The target zoom level
    /// * `duration` - Animation duration in seconds
    pub fn zoom_to(&mut self, target_zoom: f32, duration: f32) {
        self.zoom
            .animate_to(target_zoom, duration, EaseType::QuadOut);
    }

    /// Set zoom immediately without animation.
    pub fn set_zoom_immediate(&mut self, zoom: f32) {
        self.zoom.set_immediate(zoom);
    }

    /// Apply a zoom delta multiplier with animation.
    ///
    /// # Arguments
    /// * `delta` - Multiplier (e.g., 1.1 for 10% zoom in, 0.9 for 10% zoom out)
    /// * `min_zoom` - Minimum allowed zoom
    /// * `max_zoom` - Maximum allowed zoom
    /// * `duration` - Animation duration in seconds
    pub fn zoom_by(&mut self, delta: f32, min_zoom: f32, max_zoom: f32, duration: f32) {
        let target = (self.zoom.target() * delta).clamp(min_zoom, max_zoom);
        self.zoom_to(target, duration);
    }

    /// Update all animations.
    ///
    /// # Arguments
    /// * `dt` - Delta time in seconds
    pub fn update(&mut self, dt: f32) {
        self.position.update(dt);
        self.zoom.update(dt);
    }

    /// Get the current camera position.
    pub fn position(&self) -> Vec2 {
        let pos = self.position.value();
        Vec2::new(pos[0], pos[1])
    }

    /// Get the current camera position as array (for GPU uniform).
    pub fn position_array(&self) -> [f32; 2] {
        self.position.value()
    }

    /// Get the current zoom level.
    pub fn zoom(&self) -> f32 {
        self.zoom.value()
    }

    /// Check if the camera is currently animating (zoom only, position uses smoothing).
    pub fn is_zoom_animating(&self) -> bool {
        self.zoom.is_animating()
    }

    /// Set position immediately without animation.
    pub fn set_position_immediate(&mut self, pos: Vec2) {
        self.position.set_immediate([pos.x, pos.y]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_camera_follow() {
        let mut camera = AnimatedCamera::new(Vec2::ZERO, 1.0);
        camera.set_follow_smoothness(0.1);

        // Follow a target
        camera.follow_target(Vec2::new(100.0, 0.0), 0.1);

        // Should have moved toward target
        let pos = camera.position();
        assert!(pos.x > 0.0);
        assert!(pos.x < 100.0);
    }

    #[test]
    fn test_camera_zoom() {
        let mut camera = AnimatedCamera::new(Vec2::ZERO, 1.0);
        camera.zoom_to(2.0, 0.5);

        // Update halfway through
        camera.update(0.25);
        let zoom = camera.zoom();
        assert!(zoom > 1.0);
        assert!(zoom < 2.0);

        // Complete animation
        camera.update(0.25);
        assert!((camera.zoom() - 2.0).abs() < 0.01);
    }

    #[test]
    fn test_camera_zoom_by() {
        let mut camera = AnimatedCamera::new(Vec2::ZERO, 1.0);
        camera.zoom_by(2.0, 0.5, 4.0, 0.0); // Instant zoom

        camera.update(0.0);
        assert!((camera.zoom() - 2.0).abs() < 0.01);
    }
}
