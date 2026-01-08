//! In-game log viewer panel (native: egui_logger, WASM: in-memory buffer)

use crate::ui::theme::GameColors;

#[cfg(target_arch = "wasm32")]
use std::sync::Mutex;

#[cfg(target_arch = "wasm32")]
lazy_static::lazy_static! {
    /// Global log buffer for WASM (last 1000 entries)
    static ref LOG_BUFFER: Mutex<Vec<LogEntry>> = Mutex::new(Vec::new());
}

#[cfg(target_arch = "wasm32")]
#[derive(Clone)]
struct LogEntry {
    level: log::Level,
    message: String,
    timestamp: String,
}

/// Panel for viewing application logs in-game
pub struct LoggerPanel {
    /// Whether the panel is open
    pub open: bool,
    #[cfg(target_arch = "wasm32")]
    auto_scroll: bool,
}

impl LoggerPanel {
    pub fn new() -> Self {
        Self {
            open: false,
            #[cfg(target_arch = "wasm32")]
            auto_scroll: true,
        }
    }

    /// Toggle panel visibility
    pub fn toggle(&mut self) {
        self.open = !self.open;
    }

    /// Render the logger panel
    pub fn render(&mut self, ctx: &egui::Context, theme_colors: &GameColors) {
        if !self.open {
            return;
        }

        egui::Window::new("Log")
            .default_pos(egui::pos2(10.0, 400.0))
            .default_size([500.0, 300.0])
            .resizable(true)
            .collapsible(true)
            .show(ctx, |ui| {
                self.render_contents(ui, theme_colors);
            });
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn render_contents(&mut self, _ui: &mut egui::Ui, _theme_colors: &GameColors) {
        // Native uses egui_logger which has its own styling
        // Theme colors would be applied to egui_logger if we wanted to customize it
    }

    #[cfg(target_arch = "wasm32")]
    fn render_contents(&mut self, ui: &mut egui::Ui, theme_colors: &GameColors) {
        ui.horizontal(|ui| {
            ui.label("WASM Logger");
            ui.checkbox(&mut self.auto_scroll, "Auto-scroll");
            if ui.button("Clear").clicked() {
                if let Ok(mut buffer) = LOG_BUFFER.lock() {
                    buffer.clear();
                }
            }
        });

        ui.separator();

        egui::ScrollArea::vertical()
            .auto_shrink([false; 2])
            .stick_to_bottom(self.auto_scroll)
            .show(ui, |ui| {
                if let Ok(buffer) = LOG_BUFFER.lock() {
                    for entry in buffer.iter() {
                        let color = match entry.level {
                            log::Level::Error => theme_colors.error,
                            log::Level::Warn => theme_colors.warning,
                            log::Level::Info => theme_colors.info,
                            log::Level::Debug => theme_colors.text_disabled,
                            log::Level::Trace => theme_colors.text_disabled,
                        };

                        ui.horizontal(|ui| {
                            ui.label(
                                egui::RichText::new(&entry.timestamp)
                                    .color(theme_colors.text_disabled)
                                    .monospace(),
                            );
                            ui.label(
                                egui::RichText::new(format!("[{}]", entry.level))
                                    .color(color)
                                    .monospace(),
                            );
                            ui.label(egui::RichText::new(&entry.message).monospace());
                        });
                    }
                }
            });
    }
}

#[cfg(target_arch = "wasm32")]
pub fn wasm_log(level: log::Level, message: String) {
    if let Ok(mut buffer) = LOG_BUFFER.lock() {
        // Cap at 1000 entries
        if buffer.len() >= 1000 {
            buffer.remove(0);
        }

        // Format timestamp (using web_time for WASM compatibility)
        let timestamp = format!("{:?}", web_time::Instant::now());

        buffer.push(LogEntry {
            level,
            message,
            timestamp,
        });
    }
}

impl Default for LoggerPanel {
    fn default() -> Self {
        Self::new()
    }
}
