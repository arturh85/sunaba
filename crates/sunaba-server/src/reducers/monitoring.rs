//! Multiplayer monitoring and metrics reducers

use spacetimedb::ReducerContext;

// ============================================================================
// Multiplayer Metrics & Monitoring
// ============================================================================

/// Ping-pong reducer for latency measurement
/// Client sends timestamp, measures round-trip time on response
#[spacetimedb::reducer]
pub fn request_ping(_ctx: &ReducerContext, _client_timestamp_ms: u64) {
    // Immediate response - client calculates RTT = now - client_timestamp_ms
    // No logging needed - this fires frequently for latency measurement
}
