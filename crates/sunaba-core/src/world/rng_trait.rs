//! RNG trait abstraction for World simulation
//!
//! Allows World to work with both:
//! - SpacetimeDB's ctx.rng() (server-side deterministic)
//! - Rust's thread_rng() or seeded RNG (native game)

/// Random number generator trait for World simulation
pub trait WorldRng {
    /// Generate random boolean with 50% probability
    fn gen_bool(&mut self) -> bool;

    /// Generate random f32 in [0.0, 1.0)
    fn gen_f32(&mut self) -> f32;

    /// Check if random value is less than probability threshold
    fn check_probability(&mut self, probability: f32) -> bool {
        self.gen_f32() < probability
    }
}

// Blanket implementation for any type implementing rand::Rng
// This covers both ThreadRng (native game) and SpacetimeDB's ctx.rng()
// Note: SpacetimeDB provides its own rand, so this doesn't require regeneration feature
impl<T: ?Sized + rand::Rng> WorldRng for T {
    fn gen_bool(&mut self) -> bool {
        rand::Rng::r#gen(self)
    }

    fn gen_f32(&mut self) -> f32 {
        rand::Rng::r#gen(self)
    }
}
