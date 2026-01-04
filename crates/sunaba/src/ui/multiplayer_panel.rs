//! Multiplayer connection panel with state-based UI
//!
//! Supports 5 connection states: Disconnected, Connecting, Connected, Reconnecting, Error

use egui::{Color32, ComboBox, Ui};
use egui_plot::{HLine, Legend, Line, Plot, PlotPoints};

#[cfg(feature = "multiplayer")]
use crate::config::MultiplayerConfig;
#[cfg(feature = "multiplayer")]
use crate::multiplayer::metrics::MultiplayerMetrics;
#[cfg(feature = "multiplayer")]
use crate::multiplayer::{MultiplayerManager, MultiplayerState, OAuthClaims};

/// UI state for the multiplayer panel
#[cfg(feature = "multiplayer")]
#[derive(Default)]
pub struct MultiplayerPanelState {
    /// Custom server URL being entered
    pub custom_url: String,
    /// Index of selected predefined server (None = custom)
    pub selected_server_index: Option<usize>,
    /// Flag: connect to server requested
    pub connect_requested: Option<String>,
    /// Flag: disconnect requested
    pub disconnect_requested: bool,
    /// Flag: cancel connection/reconnection requested
    pub cancel_requested: bool,

    // OAuth state
    /// Flag: OAuth login requested
    pub oauth_login_requested: bool,
    /// Flag: OAuth logout requested
    pub oauth_logout_requested: bool,
    /// Cached OAuth claims (available on both native and WASM)
    pub oauth_claims: Option<OAuthClaims>,

    // Admin actions
    /// Flag: rebuild world requested
    pub rebuild_world_requested: bool,

    // Nickname editing
    /// Nickname being edited (empty = use default)
    pub nickname_input: String,
    /// Flag: nickname change requested
    pub set_nickname_requested: Option<String>,
}

#[cfg(feature = "multiplayer")]
impl MultiplayerPanelState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Reset action flags (call after processing)
    pub fn reset_flags(&mut self) {
        self.connect_requested = None;
        self.disconnect_requested = false;
        self.cancel_requested = false;
        self.oauth_login_requested = false;
        self.oauth_logout_requested = false;
        self.rebuild_world_requested = false;
        self.set_nickname_requested = None;
    }
}

/// Render the multiplayer panel based on connection state
#[cfg(feature = "multiplayer")]
pub fn render_multiplayer_panel(
    ui: &mut Ui,
    manager: &MultiplayerManager,
    panel_state: &mut MultiplayerPanelState,
    metrics: Option<&MultiplayerMetrics>,
) {
    match &manager.state {
        MultiplayerState::Disconnected => {
            render_disconnected_ui(ui, &manager.config, panel_state);
        }
        MultiplayerState::Connecting { server_url, .. } => {
            render_connecting_ui(ui, server_url, panel_state);
        }
        MultiplayerState::Connected { server_url } => {
            let chunk_progress = manager.chunk_load_progress();
            render_connected_ui(ui, server_url, metrics, chunk_progress, panel_state);
        }
        MultiplayerState::Reconnecting {
            server_url,
            attempt,
            next_attempt_at,
        } => {
            render_reconnecting_ui(ui, server_url, *attempt, *next_attempt_at, panel_state);
        }
        MultiplayerState::Error {
            message,
            server_url,
        } => {
            render_error_ui(ui, message, server_url, panel_state);
        }
    }
}

/// UI for disconnected state - server selection and connect button
#[cfg(feature = "multiplayer")]
fn render_disconnected_ui(
    ui: &mut Ui,
    config: &MultiplayerConfig,
    state: &mut MultiplayerPanelState,
) {
    ui.heading("Multiplayer Server");
    ui.add_space(10.0);

    // OAuth login section (available on both native and WASM)
    ui.horizontal(|ui| {
        if let Some(ref claims) = state.oauth_claims {
            let email = claims
                .email
                .as_ref()
                .map(|s| s.as_str())
                .unwrap_or("Unknown");
            ui.label(format!("Logged in: {}", email));
            if ui.button("Logout").clicked() {
                state.oauth_logout_requested = true;
            }
        } else {
            ui.label("Not logged in (anonymous)");
            if ui.button("Login with Google").clicked() {
                state.oauth_login_requested = true;
            }
        }
    });
    ui.separator();

    ui.label("Select a server to connect:");
    ui.add_space(5.0);

    // Predefined server selection
    ui.horizontal(|ui| {
        ui.label("Predefined:");
        ComboBox::from_id_salt("server_select")
            .selected_text(
                state
                    .selected_server_index
                    .and_then(|i| config.servers.get(i))
                    .map(|s| s.name.as_str())
                    .unwrap_or("Select..."),
            )
            .show_ui(ui, |ui| {
                for (i, server) in config.servers.iter().enumerate() {
                    ui.selectable_value(&mut state.selected_server_index, Some(i), &server.name);
                }
            });
    });

    ui.add_space(5.0);

    // Custom URL input
    ui.horizontal(|ui| {
        ui.label("Custom URL:");
        let response = ui.text_edit_singleline(&mut state.custom_url);
        if response.changed() {
            // Clear predefined selection when typing custom URL
            if !state.custom_url.is_empty() {
                state.selected_server_index = None;
            }
        }
    });

    ui.add_space(10.0);

    // Connect button
    let url_to_connect = if !state.custom_url.is_empty() {
        Some(state.custom_url.clone())
    } else {
        state
            .selected_server_index
            .and_then(|i| config.servers.get(i))
            .map(|s| s.url.clone())
    };

    let can_connect = url_to_connect.is_some();

    // Nickname input (optional)
    ui.separator();
    ui.heading("Nickname");
    ui.horizontal(|ui| {
        ui.label("Nickname (optional):");
        ui.text_edit_singleline(&mut state.nickname_input);
    });
    ui.label("Leave empty for auto-generated nickname (Player_abc123)");
    ui.add_space(10.0);

    ui.horizontal(|ui| {
        if ui
            .add_enabled(can_connect, egui::Button::new("Connect"))
            .clicked()
        {
            if let Some(url) = url_to_connect {
                state.connect_requested = Some(url);
            }
        }

        if !can_connect {
            ui.label("(select a server or enter URL)");
        }
    });

    // Show last connected server
    if let Some(ref last_server) = config.last_server {
        ui.add_space(10.0);
        ui.label(format!("Last connected: {}", last_server));
    }
}

/// UI for connecting state - loading indicator and cancel button
#[cfg(feature = "multiplayer")]
fn render_connecting_ui(ui: &mut Ui, server_url: &str, state: &mut MultiplayerPanelState) {
    ui.heading("Connecting...");
    ui.add_space(10.0);

    ui.horizontal(|ui| {
        ui.spinner();
        ui.label(format!("Connecting to {}", server_url));
    });

    ui.add_space(10.0);

    if ui.button("Cancel").clicked() {
        state.cancel_requested = true;
    }
}

/// UI for connected state - stats and disconnect button
#[cfg(feature = "multiplayer")]
fn render_connected_ui(
    ui: &mut Ui,
    server_url: &str,
    metrics: Option<&MultiplayerMetrics>,
    chunk_progress: Option<(usize, usize)>,
    state: &mut MultiplayerPanelState,
) {
    ui.horizontal(|ui| {
        ui.heading("Connected");
        ui.label("|");
        ui.label(server_url);

        // Admin badge (available on both native and WASM)
        if state.oauth_claims.is_some() {
            ui.label("|");
            ui.colored_label(Color32::GOLD, "🛡 Admin");
        }
    });

    ui.add_space(5.0);

    // Show chunk loading progress if not complete
    if let Some((loaded, total)) = chunk_progress {
        if loaded < total {
            let percent = (loaded as f32 / total as f32) * 100.0;
            ui.label(format!(
                "Loading chunks: {}/{} ({:.0}%)",
                loaded, total, percent
            ));
            ui.add(egui::ProgressBar::new(loaded as f32 / total as f32).show_percentage());
            ui.add_space(5.0);
        } else {
            ui.label("✓ Chunks loaded");
            ui.add_space(5.0);
        }
    }

    // Admin actions section (available on both native and WASM)
    if state.oauth_claims.is_some() {
        ui.separator();
        ui.heading("Admin Actions");

        if ui.button("🔄 Rebuild World").clicked() {
            state.rebuild_world_requested = true;
        }
        ui.label("Clears all chunks and resets world state");

        ui.separator();
    }

    ui.add_space(5.0);

    // Nickname editing
    ui.separator();
    ui.heading("Nickname");
    ui.horizontal(|ui| {
        ui.label("Nickname:");
        let response = ui.text_edit_singleline(&mut state.nickname_input);
        if ui.button("Set").clicked()
            || (response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)))
        {
            if !state.nickname_input.trim().is_empty() {
                state.set_nickname_requested = Some(state.nickname_input.trim().to_string());
            }
        }
    });
    ui.label("Leave empty to use auto-generated nickname");
    ui.add_space(5.0);

    if ui.button("Disconnect").clicked() {
        state.disconnect_requested = true;
    }

    ui.separator();

    // Show stats if available
    if let Some(metrics) = metrics {
        ui.heading("Connection Health");
        render_connection_health(ui, metrics);

        ui.separator();
        ui.heading("Server Performance");
        render_server_performance(ui, metrics);

        ui.separator();
        ui.heading("Players & Creatures");
        render_population(ui, metrics);

        ui.separator();
        ui.heading("Historical Graphs");
        render_graphs(ui, metrics);
    } else {
        ui.label("Waiting for server data...");
    }
}

/// UI for reconnecting state - attempt count and cancel button
#[cfg(feature = "multiplayer")]
fn render_reconnecting_ui(
    ui: &mut Ui,
    server_url: &str,
    attempt: u32,
    next_attempt_at: web_time::Instant,
    state: &mut MultiplayerPanelState,
) {
    ui.heading("Connection Lost - Reconnecting");
    ui.add_space(10.0);

    ui.label(format!("Server: {}", server_url));
    ui.label(format!("Reconnection attempt: {}", attempt));

    let now = web_time::Instant::now();
    if next_attempt_at > now {
        let seconds_until = (next_attempt_at - now).as_secs_f32();
        ui.label(format!("Next attempt in: {:.1}s", seconds_until));
    } else {
        ui.label("Reconnecting now...");
    }

    ui.add_space(10.0);

    ui.horizontal(|ui| {
        ui.spinner();
        if ui.button("Cancel & Return to Singleplayer").clicked() {
            state.cancel_requested = true;
        }
    });
}

/// UI for error state - error message and retry/cancel buttons
#[cfg(feature = "multiplayer")]
fn render_error_ui(
    ui: &mut Ui,
    error_message: &str,
    server_url: &str,
    state: &mut MultiplayerPanelState,
) {
    ui.heading("Connection Error");
    ui.add_space(10.0);

    ui.colored_label(Color32::RED, "❌ Connection failed");
    ui.label(error_message);
    ui.label(format!("Server: {}", server_url));

    ui.add_space(10.0);

    ui.horizontal(|ui| {
        if ui.button("Retry").clicked() {
            state.connect_requested = Some(server_url.to_string());
        }

        if ui.button("Cancel").clicked() {
            state.cancel_requested = true;
        }
    });
}

// Helper rendering functions for stats (reused from old multiplayer_stats.rs)

#[cfg(feature = "multiplayer")]
fn render_connection_health(ui: &mut Ui, metrics: &MultiplayerMetrics) {
    // Ping with color coding
    let ping_color = if metrics.ping_ms < 50.0 {
        Color32::GREEN
    } else if metrics.ping_ms < 150.0 {
        Color32::YELLOW
    } else {
        Color32::RED
    };

    ui.horizontal(|ui| {
        ui.label("Ping:");
        ui.colored_label(ping_color, format!("{:.0} ms", metrics.ping_ms));
    });

    ui.label(format!("Updates/sec: {:.1}", metrics.updates_per_second));

    if let Some(last_update) = metrics.last_update_time {
        let elapsed = last_update.elapsed().as_secs_f32();
        let color = if elapsed < 1.0 {
            Color32::GREEN
        } else if elapsed < 3.0 {
            Color32::YELLOW
        } else {
            Color32::RED
        };
        ui.horizontal(|ui| {
            ui.label("Last update:");
            ui.colored_label(color, format!("{:.1}s ago", elapsed));
        });
    }
}

#[cfg(feature = "multiplayer")]
fn render_server_performance(ui: &mut Ui, metrics: &MultiplayerMetrics) {
    ui.label(format!("Tick time: {:.2} ms", metrics.server_tick_time_ms));
    ui.label(format!("Active chunks: {}", metrics.server_active_chunks));

    let health_color = if metrics.server_tick_time_ms < 10.0 {
        Color32::GREEN
    } else if metrics.server_tick_time_ms < 16.0 {
        Color32::YELLOW
    } else {
        Color32::RED
    };
    ui.horizontal(|ui| {
        ui.label("Server health:");
        ui.colored_label(health_color, "●");
    });
}

#[cfg(feature = "multiplayer")]
fn render_population(ui: &mut Ui, metrics: &MultiplayerMetrics) {
    ui.label(format!("Players online: {}", metrics.server_online_players));
    ui.label(format!(
        "Creatures alive: {}",
        metrics.server_creatures_alive
    ));
    ui.label(format!(
        "Total entities: {}",
        metrics.server_online_players + metrics.server_creatures_alive
    ));
}

#[cfg(feature = "multiplayer")]
fn render_graphs(ui: &mut Ui, metrics: &MultiplayerMetrics) {
    // Ping graph
    if !metrics.ping_history.is_empty() {
        let ping_points: PlotPoints = metrics
            .ping_history
            .iter()
            .map(|(time, ping)| [*time, *ping as f64])
            .collect();

        Plot::new("ping_plot")
            .height(100.0)
            .legend(Legend::default())
            .show(ui, |plot_ui| {
                plot_ui.line(Line::new("Ping (ms)", ping_points));
                plot_ui.hline(HLine::new("Target", 100.0).color(Color32::YELLOW));
            });
    }

    // Update rate graph
    if !metrics.update_rate_history.is_empty() {
        let ups_points: PlotPoints = metrics
            .update_rate_history
            .iter()
            .map(|(time, ups)| [*time, *ups as f64])
            .collect();

        Plot::new("ups_plot")
            .height(100.0)
            .legend(Legend::default())
            .show(ui, |plot_ui| {
                plot_ui.line(Line::new("Updates/sec", ups_points));
                plot_ui.hline(HLine::new("Target", 60.0).color(Color32::GREEN));
            });
    }

    // Server tick time graph
    if !metrics.server_tick_history.is_empty() {
        let tick_points: PlotPoints = metrics
            .server_tick_history
            .iter()
            .map(|(time, tick)| [*time, *tick as f64])
            .collect();

        Plot::new("tick_plot")
            .height(100.0)
            .legend(Legend::default())
            .show(ui, |plot_ui| {
                plot_ui.line(Line::new("Server tick (ms)", tick_points));
                plot_ui.hline(HLine::new("Target", 16.0).color(Color32::YELLOW));
            });
    }
}
