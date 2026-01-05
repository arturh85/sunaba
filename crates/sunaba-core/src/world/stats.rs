//! Simulation statistics collection trait

/// Trait for collecting simulation statistics
///
/// This allows sunaba-core to record stats without depending on the full
/// stats collection implementation in the main crate.
pub trait SimStats {
    /// Record that a pixel was moved during simulation
    fn record_pixel_moved(&mut self);

    /// Record that a state change occurred (e.g., melting, freezing)
    fn record_state_change(&mut self);

    /// Record that a chemical reaction occurred
    fn record_reaction(&mut self);
}

/// A no-op implementation for when stats collection is not needed
#[derive(Default)]
pub struct NoopStats;

impl SimStats for NoopStats {
    fn record_pixel_moved(&mut self) {}
    fn record_state_change(&mut self) {}
    fn record_reaction(&mut self) {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_noop_stats_default() {
        let _stats = NoopStats::default();
        // Should compile and not panic
    }

    #[test]
    fn test_noop_stats_record_pixel_moved() {
        let mut stats = NoopStats;
        // Should do nothing without panicking
        stats.record_pixel_moved();
        stats.record_pixel_moved();
        stats.record_pixel_moved();
    }

    #[test]
    fn test_noop_stats_record_state_change() {
        let mut stats = NoopStats;
        stats.record_state_change();
        stats.record_state_change();
    }

    #[test]
    fn test_noop_stats_record_reaction() {
        let mut stats = NoopStats;
        stats.record_reaction();
        stats.record_reaction();
    }

    #[test]
    fn test_noop_stats_all_methods() {
        let mut stats = NoopStats::default();

        // Mix of all operations
        for _ in 0..100 {
            stats.record_pixel_moved();
            stats.record_state_change();
            stats.record_reaction();
        }
        // No-op implementation should not track any state, just pass through
    }

    /// A simple implementation of SimStats for testing the trait
    struct CountingStats {
        pixels_moved: u32,
        state_changes: u32,
        reactions: u32,
    }

    impl SimStats for CountingStats {
        fn record_pixel_moved(&mut self) {
            self.pixels_moved += 1;
        }

        fn record_state_change(&mut self) {
            self.state_changes += 1;
        }

        fn record_reaction(&mut self) {
            self.reactions += 1;
        }
    }

    #[test]
    fn test_counting_stats_implementation() {
        let mut stats = CountingStats {
            pixels_moved: 0,
            state_changes: 0,
            reactions: 0,
        };

        stats.record_pixel_moved();
        stats.record_pixel_moved();
        stats.record_state_change();
        stats.record_reaction();
        stats.record_reaction();
        stats.record_reaction();

        assert_eq!(stats.pixels_moved, 2);
        assert_eq!(stats.state_changes, 1);
        assert_eq!(stats.reactions, 3);
    }
}
