//! In-game log viewer panel (native: egui_logger, WASM: in-memory buffer)

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
    pub fn render(&mut self, ctx: &egui::Context) {
        if !self.open {
            return;
        }

        egui::Window::new("Log")
            .default_pos(egui::pos2(10.0, 400.0))
            .default_size([500.0, 300.0])
            .resizable(true)
            .collapsible(true)
            .show(ctx, |ui| {
                self.render_contents(ui);
            });
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn render_contents(&mut self, ui: &mut egui::Ui) {
        egui_logger::logger_ui().show(ui);
    }

    #[cfg(target_arch = "wasm32")]
    fn render_contents(&mut self, ui: &mut egui::Ui) {
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
                            log::Level::Error => egui::Color32::RED,
                            log::Level::Warn => egui::Color32::YELLOW,
                            log::Level::Info => egui::Color32::LIGHT_BLUE,
                            log::Level::Debug => egui::Color32::GRAY,
                            log::Level::Trace => egui::Color32::DARK_GRAY,
                        };

                        ui.horizontal(|ui| {
                            ui.label(
                                egui::RichText::new(&entry.timestamp)
                                    .color(egui::Color32::GRAY)
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
