//! Multiplayer statistics panel with connection health, server metrics, and historical graphs

use egui::{Color32, Ui};
use egui_plot::{HLine, Legend, Line, Plot, PlotPoints};

#[cfg(feature = "multiplayer")]
use crate::multiplayer::metrics::{ConnectionStatus, MultiplayerMetrics};

/// Render multiplayer statistics panel
#[cfg(feature = "multiplayer")]
pub fn render_multiplayer_stats(ui: &mut Ui, metrics: &MultiplayerMetrics) {
    ui.heading("Connection Health");
    render_connection_section(ui, metrics);

    ui.separator();
    ui.heading("Server Performance");
    render_server_section(ui, metrics);

    ui.separator();
    ui.heading("Players & Creatures");
    render_population_section(ui, metrics);

    ui.separator();
    ui.heading("Historical Graphs");
    render_graphs(ui, metrics);
}

#[cfg(feature = "multiplayer")]
fn render_connection_section(ui: &mut Ui, metrics: &MultiplayerMetrics) {
    // Status indicator
    let (status_text, status_color) = match metrics.connection_status {
        ConnectionStatus::Disconnected => ("Disconnected", Color32::RED),
        ConnectionStatus::Connecting => ("Connecting...", Color32::YELLOW),
        ConnectionStatus::Connected => ("Connected", Color32::GREEN),
        ConnectionStatus::Reconnecting => ("Reconnecting...", Color32::from_rgb(255, 165, 0)),
    };

    ui.horizontal(|ui| {
        ui.label("Status:");
        ui.colored_label(status_color, status_text);
    });

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

    // Update rate
    ui.label(format!("Updates/sec: {:.1}", metrics.updates_per_second));

    // Last update time
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
    } else {
        ui.label("Last update: N/A");
    }
}

#[cfg(feature = "multiplayer")]
fn render_server_section(ui: &mut Ui, metrics: &MultiplayerMetrics) {
    ui.label(format!("Tick time: {:.2} ms", metrics.server_tick_time_ms));
    ui.label(format!("Active chunks: {}", metrics.server_active_chunks));

    // Server load health indicator
    let tick_health = if metrics.server_tick_time_ms < 10.0 {
        ("Excellent", Color32::GREEN)
    } else if metrics.server_tick_time_ms < 16.0 {
        ("Good", Color32::from_rgb(144, 238, 144))
    } else if metrics.server_tick_time_ms < 25.0 {
        ("Slow", Color32::YELLOW)
    } else {
        ("Overloaded", Color32::RED)
    };

    ui.horizontal(|ui| {
        ui.label("Server load:");
        ui.colored_label(tick_health.1, tick_health.0);
    });

    // Performance hint
    if metrics.server_tick_time_ms >= 16.0 {
        ui.colored_label(Color32::YELLOW, "⚠ Server tick exceeding 60fps target");
    }
}

#[cfg(feature = "multiplayer")]
fn render_population_section(ui: &mut Ui, metrics: &MultiplayerMetrics) {
    ui.label(format!("Online players: {}", metrics.server_online_players));
    ui.label(format!(
        "Creatures alive: {}",
        metrics.server_creatures_alive
    ));

    // Combined population
    let total_entities = metrics.server_online_players + metrics.server_creatures_alive;
    ui.label(format!("Total entities: {}", total_entities));
}

#[cfg(feature = "multiplayer")]
fn render_graphs(ui: &mut Ui, metrics: &MultiplayerMetrics) {
    // Ping history graph
    if !metrics.ping_history.is_empty() {
        ui.label("Ping History");
        Plot::new("ping_plot")
            .legend(Legend::default())
            .height(150.0)
            .show(ui, |plot_ui| {
                let points: PlotPoints = metrics
                    .ping_history
                    .iter()
                    .map(|(t, ping)| [*t, *ping as f64])
                    .collect();

                plot_ui.line(Line::new("Ping", points).color(Color32::from_rgb(100, 200, 100)));

                // Reference line at 100ms (reasonable threshold)
                plot_ui.hline(HLine::new("100ms threshold", 100.0).color(Color32::DARK_RED));
            });
    }

    // Update rate graph
    if !metrics.update_rate_history.is_empty() {
        ui.add_space(8.0);
        ui.label("Update Rate");
        Plot::new("update_rate_plot")
            .legend(Legend::default())
            .height(150.0)
            .show(ui, |plot_ui| {
                let points: PlotPoints = metrics
                    .update_rate_history
                    .iter()
                    .map(|(t, ups)| [*t, *ups as f64])
                    .collect();

                plot_ui
                    .line(Line::new("Update Rate", points).color(Color32::from_rgb(100, 150, 255)));

                // Reference line at 60 UPS (ideal)
                plot_ui
                    .hline(HLine::new("60 UPS target", 60.0).color(Color32::from_rgb(50, 200, 50)));
            });
    }

    // Server tick time graph
    if !metrics.server_tick_history.is_empty() {
        ui.add_space(8.0);
        ui.label("Server Tick Time");
        Plot::new("server_tick_plot")
            .legend(Legend::default())
            .height(150.0)
            .show(ui, |plot_ui| {
                let points: PlotPoints = metrics
                    .server_tick_history
                    .iter()
                    .map(|(t, ms)| [*t, *ms as f64])
                    .collect();

                plot_ui
                    .line(Line::new("Server Tick", points).color(Color32::from_rgb(255, 150, 100)));

                // Reference line at 16ms (60fps target)
                plot_ui.hline(HLine::new("16ms (60fps)", 16.0).color(Color32::DARK_RED));
            });
    }

    // Show message if no history yet
    if metrics.ping_history.is_empty()
        && metrics.update_rate_history.is_empty()
        && metrics.server_tick_history.is_empty()
    {
        ui.label("No historical data yet. Graphs will appear as data is collected.");
    }
}
