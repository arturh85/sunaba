//! Multiplayer connection manager
//!
//! Manages multiplayer connection state, reconnection logic, and world switching.

use super::MultiplayerClient;
use super::chunk_loader::ChunkLoadQueue;
use crate::config::MultiplayerConfig;
use glam::IVec2;
use web_time::Instant;

/// Multiplayer connection state
#[derive(Debug, Clone)]
pub enum MultiplayerState {
    /// Not connected to any server
    Disconnected,

    /// Attempting to connect to a server
    Connecting { server_url: String, attempt: u32 },

    /// Successfully connected to a server
    Connected { server_url: String },

    /// Connection lost, attempting to reconnect
    Reconnecting {
        server_url: String,
        attempt: u32,
        next_attempt_at: Instant,
    },

    /// Connection error occurred
    Error { message: String, server_url: String },
}

impl MultiplayerState {
    /// Check if currently connected
    pub fn is_connected(&self) -> bool {
        matches!(self, MultiplayerState::Connected { .. })
    }

    /// Check if disconnected
    pub fn is_disconnected(&self) -> bool {
        matches!(self, MultiplayerState::Disconnected)
    }

    /// Check if currently connecting or reconnecting
    pub fn is_connecting(&self) -> bool {
        matches!(
            self,
            MultiplayerState::Connecting { .. } | MultiplayerState::Reconnecting { .. }
        )
    }

    /// Get the server URL if available
    pub fn server_url(&self) -> Option<&str> {
        match self {
            MultiplayerState::Disconnected => None,
            MultiplayerState::Connecting { server_url, .. }
            | MultiplayerState::Connected { server_url }
            | MultiplayerState::Reconnecting { server_url, .. }
            | MultiplayerState::Error { server_url, .. } => Some(server_url),
        }
    }
}

/// Manages multiplayer connection and state
pub struct MultiplayerManager {
    pub client: MultiplayerClient,
    pub state: MultiplayerState,
    pub config: MultiplayerConfig,

    /// Track if singleplayer world was saved before connecting
    saved_singleplayer: bool,

    /// Progressive chunk loading queue
    pub chunk_load_queue: Option<ChunkLoadQueue>,

    /// Subscription center in chunk coordinates (for re-subscription)
    pub subscription_center: IVec2,
}

impl MultiplayerManager {
    /// Create a new multiplayer manager (starts disconnected)
    pub fn new(config: MultiplayerConfig) -> Self {
        Self {
            client: MultiplayerClient::new(),
            state: MultiplayerState::Disconnected,
            config,
            saved_singleplayer: false,
            chunk_load_queue: None,
            subscription_center: IVec2::ZERO,
        }
    }

    /// Initiate connection to a server
    pub fn start_connecting(&mut self, server_url: String) {
        log::info!("Starting connection to {}", server_url);
        self.state = MultiplayerState::Connecting {
            server_url,
            attempt: 1,
        };
    }

    /// Mark connection as successful
    pub fn mark_connected(&mut self, server_url: String) {
        log::info!("Successfully connected to {}", server_url);
        self.state = MultiplayerState::Connected { server_url };

        // Update last connected server in config
        self.config.last_server = Some(self.state.server_url().unwrap().to_string());
    }

    /// Mark connection as failed with error message
    pub fn mark_error(&mut self, error_message: String, server_url: String) {
        log::error!("Connection error: {}", error_message);

        // Convert technical error to user-friendly message
        let user_message = Self::user_friendly_error(&error_message);

        self.state = MultiplayerState::Error {
            message: user_message,
            server_url,
        };
    }

    /// Start reconnection attempt
    pub fn start_reconnecting(&mut self, server_url: String, attempt: u32) {
        let backoff_seconds = Self::calculate_backoff(attempt);
        let next_attempt_at = Instant::now() + std::time::Duration::from_secs(backoff_seconds);

        log::info!(
            "Reconnection attempt {} scheduled in {}s",
            attempt,
            backoff_seconds
        );

        self.state = MultiplayerState::Reconnecting {
            server_url,
            attempt,
            next_attempt_at,
        };
    }

    /// Return to disconnected state
    pub fn mark_disconnected(&mut self) {
        log::info!("Disconnected from server");
        self.state = MultiplayerState::Disconnected;
        self.saved_singleplayer = false;
    }

    /// Set the saved singleplayer flag
    pub fn set_singleplayer_saved(&mut self, saved: bool) {
        self.saved_singleplayer = saved;
    }

    /// Check if singleplayer was saved before connection
    pub fn is_singleplayer_saved(&self) -> bool {
        self.saved_singleplayer
    }

    /// Calculate exponential backoff delay (1s, 2s, 4s, 8s, max 30s)
    fn calculate_backoff(attempt: u32) -> u64 {
        let delay = 2u64.pow(attempt.saturating_sub(1));
        delay.min(30) // Cap at 30 seconds
    }

    /// Convert technical error message to user-friendly version
    fn user_friendly_error(technical_error: &str) -> String {
        // Common error patterns
        if technical_error.contains("connection refused")
            || technical_error.contains("could not connect")
        {
            "Connection failed - Server not responding".to_string()
        } else if technical_error.contains("timeout") {
            "Connection timeout - Server took too long to respond".to_string()
        } else if technical_error.contains("authentication")
            || technical_error.contains("unauthorized")
        {
            "Authentication failed - Invalid credentials".to_string()
        } else if technical_error.contains("not found") || technical_error.contains("404") {
            "Server not found - Check the server URL".to_string()
        } else {
            // Generic fallback
            "Connection failed - Please try again".to_string()
        }
    }

    /// Get chunk load progress (loaded, total)
    pub fn chunk_load_progress(&self) -> Option<(usize, usize)> {
        self.chunk_load_queue.as_ref().map(|q| q.progress())
    }

    /// Check if progressive chunk loading is complete
    pub fn is_progressive_load_complete(&self) -> bool {
        self.chunk_load_queue
            .as_ref()
            .map(|q| q.is_complete())
            .unwrap_or(true)
    }
}
