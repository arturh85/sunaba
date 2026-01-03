//! Generic tweening/interpolation types.

use keyframe::{ease, functions};

/// Easing function type for animations.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum EaseType {
    /// Constant speed interpolation
    #[default]
    Linear,
    /// Slow start, fast end
    EaseIn,
    /// Fast start, slow end
    EaseOut,
    /// Slow start and end, fast middle
    EaseInOut,
    /// Quadratic ease out (smoother than linear)
    QuadOut,
    /// Cubic ease out (even smoother)
    CubicOut,
}

impl EaseType {
    /// Apply easing function to a normalized time value (0.0 to 1.0).
    /// Uses keyframe crate's ease() function with appropriate easing type.
    fn apply(&self, t: f32) -> f32 {
        let t = (t as f64).clamp(0.0, 1.0);
        let result = match self {
            EaseType::Linear => ease(functions::Linear, 0.0, 1.0, t),
            EaseType::EaseIn => ease(functions::EaseIn, 0.0, 1.0, t),
            EaseType::EaseOut => ease(functions::EaseOut, 0.0, 1.0, t),
            EaseType::EaseInOut => ease(functions::EaseInOut, 0.0, 1.0, t),
            EaseType::QuadOut => ease(functions::EaseOutQuad, 0.0, 1.0, t),
            EaseType::CubicOut => ease(functions::EaseOutCubic, 0.0, 1.0, t),
        };
        result as f32
    }
}

/// Trait for types that can be interpolated (tweened).
pub trait Tweenable: Copy {
    /// Linear interpolation between two values.
    /// `t` should be 0.0 to 1.0, where 0.0 returns `a` and 1.0 returns `b`.
    fn lerp(a: Self, b: Self, t: f32) -> Self;
}

impl Tweenable for f32 {
    fn lerp(a: Self, b: Self, t: f32) -> Self {
        a + (b - a) * t
    }
}

impl Tweenable for glam::Vec2 {
    fn lerp(a: Self, b: Self, t: f32) -> Self {
        a.lerp(b, t)
    }
}

impl Tweenable for [f32; 2] {
    fn lerp(a: Self, b: Self, t: f32) -> Self {
        [f32::lerp(a[0], b[0], t), f32::lerp(a[1], b[1], t)]
    }
}

/// An animated value that smoothly interpolates toward a target over time.
///
/// Supports various easing functions for natural-feeling motion.
#[derive(Debug, Clone)]
pub struct AnimatedValue<T: Tweenable> {
    /// Current interpolated value
    current: T,
    /// Starting value for current animation
    start: T,
    /// Target value to animate toward
    target: T,
    /// Elapsed time in current animation (seconds)
    elapsed: f32,
    /// Total duration of current animation (seconds)
    duration: f32,
    /// Easing function to apply
    easing: EaseType,
}

impl<T: Tweenable> AnimatedValue<T> {
    /// Create a new animated value at the given initial position.
    pub fn new(initial: T) -> Self {
        Self {
            current: initial,
            start: initial,
            target: initial,
            elapsed: 0.0,
            duration: 0.0,
            easing: EaseType::Linear,
        }
    }

    /// Start animating toward a new target value.
    ///
    /// # Arguments
    /// * `target` - The value to animate toward
    /// * `duration` - How long the animation should take (seconds)
    /// * `easing` - The easing function to use
    pub fn animate_to(&mut self, target: T, duration: f32, easing: EaseType) {
        self.start = self.current;
        self.target = target;
        self.elapsed = 0.0;
        self.duration = duration.max(0.0);
        self.easing = easing;
    }

    /// Set the value immediately without animation.
    pub fn set_immediate(&mut self, value: T) {
        self.current = value;
        self.start = value;
        self.target = value;
        self.elapsed = 0.0;
        self.duration = 0.0;
    }

    /// Update the animation by the given delta time (seconds).
    ///
    /// Returns `true` if the animation is still in progress, `false` if complete.
    pub fn update(&mut self, dt: f32) -> bool {
        if self.duration <= 0.0 {
            self.current = self.target;
            return false;
        }

        self.elapsed += dt;

        if self.elapsed >= self.duration {
            self.current = self.target;
            self.elapsed = self.duration;
            return false;
        }

        let t = self.elapsed / self.duration;
        let eased_t = self.easing.apply(t);
        self.current = T::lerp(self.start, self.target, eased_t);
        true
    }

    /// Get the current interpolated value.
    pub fn value(&self) -> T {
        self.current
    }

    /// Get the target value.
    pub fn target(&self) -> T {
        self.target
    }

    /// Check if the animation is currently in progress.
    pub fn is_animating(&self) -> bool {
        self.elapsed < self.duration && self.duration > 0.0
    }

    /// Get remaining animation time in seconds.
    pub fn remaining(&self) -> f32 {
        (self.duration - self.elapsed).max(0.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_animated_value_linear() {
        let mut val = AnimatedValue::new(0.0f32);
        val.animate_to(100.0, 1.0, EaseType::Linear);

        // At t=0, should be at start
        assert!((val.value() - 0.0).abs() < 0.01);

        // At t=0.5, should be halfway
        val.update(0.5);
        assert!((val.value() - 50.0).abs() < 1.0);

        // At t=1.0, should be at target
        val.update(0.5);
        assert!((val.value() - 100.0).abs() < 0.01);
        assert!(!val.is_animating());
    }

    #[test]
    fn test_animated_value_immediate() {
        let mut val = AnimatedValue::new(0.0f32);
        val.set_immediate(50.0);
        assert!((val.value() - 50.0).abs() < 0.01);
        assert!(!val.is_animating());
    }

    #[test]
    fn test_animated_value_vec2() {
        let mut val = AnimatedValue::new(glam::Vec2::ZERO);
        val.animate_to(glam::Vec2::new(100.0, 200.0), 1.0, EaseType::Linear);

        val.update(0.5);
        let v = val.value();
        assert!((v.x - 50.0).abs() < 1.0);
        assert!((v.y - 100.0).abs() < 1.0);
    }

    #[test]
    fn test_ease_out_faster_at_start() {
        let mut val_linear = AnimatedValue::new(0.0f32);
        let mut val_ease_out = AnimatedValue::new(0.0f32);

        val_linear.animate_to(100.0, 1.0, EaseType::Linear);
        val_ease_out.animate_to(100.0, 1.0, EaseType::EaseOut);

        // At t=0.25, ease_out should be further along
        val_linear.update(0.25);
        val_ease_out.update(0.25);

        assert!(val_ease_out.value() > val_linear.value());
    }
}
