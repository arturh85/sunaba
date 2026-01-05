//! Client-side multiplayer metrics collection and tracking

use std::collections::VecDeque;
use std::time::Duration;
use web_time::Instant;

#[cfg(not(target_arch = "wasm32"))]
use super::generated::ServerMetrics;

#[cfg(target_arch = "wasm32")]
use super::js_client::ServerMetrics;

/// Client-side multiplayer metrics
#[derive(Clone, Debug)]
pub struct MultiplayerMetrics {
    // Connection health
    pub ping_ms: f32,
    pub updates_per_second: f32,
    pub connection_status: ConnectionStatus,
    pub last_update_time: Option<Instant>,

    // Server metrics (from server_metrics table)
    pub server_tick_time_ms: f32,
    pub server_active_chunks: u32,
    pub server_online_players: u32,
    pub server_creatures_alive: u32,

    // Historical data (for graphs) - (timestamp, value)
    pub ping_history: VecDeque<(f64, f32)>,
    pub update_rate_history: VecDeque<(f64, f32)>,
    pub server_tick_history: VecDeque<(f64, f32)>,
}

impl Default for MultiplayerMetrics {
    fn default() -> Self {
        Self {
            ping_ms: 0.0,
            updates_per_second: 0.0,
            connection_status: ConnectionStatus::Disconnected,
            last_update_time: None,
            server_tick_time_ms: 0.0,
            server_active_chunks: 0,
            server_online_players: 0,
            server_creatures_alive: 0,
            ping_history: VecDeque::new(),
            update_rate_history: VecDeque::new(),
            server_tick_history: VecDeque::new(),
        }
    }
}

/// Connection status states
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ConnectionStatus {
    Disconnected,
    Connecting,
    Connected,
    Reconnecting,
}

/// Metrics collector for multiplayer
pub struct MetricsCollector {
    metrics: MultiplayerMetrics,

    // Ping measurement
    ping_start_time: Option<Instant>,
    ping_interval: Duration,
    last_ping_time: Instant,

    // Update rate tracking
    update_counter: u32,
    last_update_rate_calc: Instant,

    // History limits
    max_history_points: usize,

    // Time tracking for history timestamps
    start_time: Instant,
}

impl MetricsCollector {
    const PING_INTERVAL_MS: u64 = 1000; // 1 ping/second
    const MAX_HISTORY_POINTS: usize = 600; // 10 minutes at 1fps

    pub fn new() -> Self {
        let now = Instant::now();
        Self {
            metrics: MultiplayerMetrics::default(),
            ping_start_time: None,
            ping_interval: Duration::from_millis(Self::PING_INTERVAL_MS),
            last_ping_time: now,
            update_counter: 0,
            last_update_rate_calc: now,
            max_history_points: Self::MAX_HISTORY_POINTS,
            start_time: now,
        }
    }

    /// Send ping request to server (if interval has elapsed)
    pub fn send_ping(&mut self, client: &super::MultiplayerClient) {
        let now = Instant::now();
        if now.duration_since(self.last_ping_time) > self.ping_interval {
            self.ping_start_time = Some(now);
            let timestamp_ms = now.duration_since(self.start_time).as_millis() as u64;

            if let Err(e) = client.request_ping(timestamp_ms) {
                log::warn!("Failed to send ping: {}", e);
            }

            self.last_ping_time = now;
        }
    }

    /// Called when ping response received
    pub fn on_ping_response(&mut self) {
        if let Some(start) = self.ping_start_time.take() {
            let ping_ms = start.elapsed().as_millis() as f32;
            self.metrics.ping_ms = ping_ms;

            let time = Instant::now().duration_since(self.start_time).as_secs_f64();
            self.metrics.ping_history.push_back((time, ping_ms));

            if self.metrics.ping_history.len() > self.max_history_points {
                self.metrics.ping_history.pop_front();
            }
        }
    }

    /// Record an update received from server
    pub fn record_update(&mut self) {
        self.update_counter += 1;
        self.metrics.last_update_time = Some(Instant::now());

        // Calculate UPS every second
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_update_rate_calc);
        if elapsed.as_secs() >= 1 {
            self.metrics.updates_per_second = self.update_counter as f32 / elapsed.as_secs_f32();
            self.update_counter = 0;
            self.last_update_rate_calc = now;

            let time = now.duration_since(self.start_time).as_secs_f64();
            let ups = self.metrics.updates_per_second;
            self.metrics.update_rate_history.push_back((time, ups));

            if self.metrics.update_rate_history.len() > self.max_history_points {
                self.metrics.update_rate_history.pop_front();
            }
        }
    }

    /// Update from server_metrics table
    pub fn update_server_metrics(&mut self, server_metrics: &ServerMetrics) {
        self.metrics.server_tick_time_ms = server_metrics.world_tick_time_ms;
        self.metrics.server_active_chunks = server_metrics.active_chunks;
        self.metrics.server_online_players = server_metrics.online_players;
        self.metrics.server_creatures_alive = server_metrics.creatures_alive;

        let time = Instant::now().duration_since(self.start_time).as_secs_f64();
        self.metrics
            .server_tick_history
            .push_back((time, server_metrics.world_tick_time_ms));

        if self.metrics.server_tick_history.len() > self.max_history_points {
            self.metrics.server_tick_history.pop_front();
        }
    }

    /// Update connection status
    pub fn set_connection_status(&mut self, status: ConnectionStatus) {
        self.metrics.connection_status = status;
    }

    /// Get current metrics (read-only reference)
    pub fn metrics(&self) -> &MultiplayerMetrics {
        &self.metrics
    }
}

impl Default for MetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}
