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

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;
    use rand_xoshiro::Xoshiro256StarStar;

    #[test]
    fn test_world_rng_gen_bool() {
        let mut rng = Xoshiro256StarStar::seed_from_u64(12345);

        // Should produce both true and false over many iterations
        let mut seen_true = false;
        let mut seen_false = false;

        for _ in 0..100 {
            if rng.gen_bool() {
                seen_true = true;
            } else {
                seen_false = true;
            }
        }

        assert!(seen_true);
        assert!(seen_false);
    }

    #[test]
    fn test_world_rng_gen_f32() {
        let mut rng = Xoshiro256StarStar::seed_from_u64(12345);

        for _ in 0..100 {
            let val = rng.gen_f32();
            assert!(val >= 0.0);
            assert!(val < 1.0);
        }
    }

    #[test]
    fn test_world_rng_check_probability_always_true() {
        let mut rng = Xoshiro256StarStar::seed_from_u64(12345);

        // With probability 1.0, should always return true
        for _ in 0..100 {
            assert!(rng.check_probability(1.0));
        }
    }

    #[test]
    fn test_world_rng_check_probability_always_false() {
        let mut rng = Xoshiro256StarStar::seed_from_u64(12345);

        // With probability 0.0, should always return false
        for _ in 0..100 {
            assert!(!rng.check_probability(0.0));
        }
    }

    #[test]
    fn test_world_rng_check_probability_mixed() {
        let mut rng = Xoshiro256StarStar::seed_from_u64(12345);

        // With probability 0.5, should produce both true and false
        let mut seen_true = false;
        let mut seen_false = false;

        for _ in 0..100 {
            if rng.check_probability(0.5) {
                seen_true = true;
            } else {
                seen_false = true;
            }
        }

        assert!(seen_true);
        assert!(seen_false);
    }

    #[test]
    fn test_world_rng_deterministic() {
        let mut rng1 = Xoshiro256StarStar::seed_from_u64(42);
        let mut rng2 = Xoshiro256StarStar::seed_from_u64(42);

        // Same seed should produce same sequence
        for _ in 0..100 {
            assert_eq!(rng1.gen_bool(), rng2.gen_bool());
        }

        let mut rng3 = Xoshiro256StarStar::seed_from_u64(42);
        let mut rng4 = Xoshiro256StarStar::seed_from_u64(42);

        for _ in 0..100 {
            assert_eq!(rng3.gen_f32(), rng4.gen_f32());
        }
    }
}
