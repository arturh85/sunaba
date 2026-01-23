//! Vertical debug panel menu system
//! Replaces egui_dock with a custom vertical menu + content area layout

use super::dock::{DockContext, DockTab, DockTabViewer};
use std::collections::HashSet;

/// Manages debug panel state and rendering
pub struct DebugPanelManager {
    /// Currently active panel (shown in content area)
    pub active_panel: Option<DockTab>,
    /// Panels that are open (visible in menu)
    pub open_panels: HashSet<DockTab>,
}

impl DebugPanelManager {
    pub fn new() -> Self {
        let mut open_panels = HashSet::new();
        // Default open panels (matching current dock defaults)
        open_panels.insert(DockTab::Logger);
        open_panels.insert(DockTab::Stats);
        #[cfg(feature = "multiplayer")]
        open_panels.insert(DockTab::MultiplayerStats);

        Self {
            active_panel: Some(DockTab::Logger), // Default active
            open_panels,
        }
    }

    /// Toggle panel visibility (for keybindings)
    pub fn toggle_tab(&mut self, tab: DockTab) {
        if self.open_panels.contains(&tab) {
            self.open_panels.remove(&tab);
            // If closing active panel, clear selection
            if self.active_panel == Some(tab) {
                self.active_panel = None;
            }
        } else {
            self.open_panels.insert(tab);
            self.active_panel = Some(tab); // Make newly opened panel active
        }
    }

    /// Select a panel to display (for menu clicks)
    pub fn select_tab(&mut self, tab: DockTab) {
        self.active_panel = Some(tab);
        self.open_panels.insert(tab); // Ensure it's in open set
    }

    /// Check if panel is currently active
    pub fn is_active(&self, tab: DockTab) -> bool {
        self.active_panel == Some(tab)
    }

    /// Check if panel is open (visible in menu)
    pub fn is_open(&self, tab: DockTab) -> bool {
        self.open_panels.contains(&tab)
    }
}

impl Default for DebugPanelManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Render debug panel menu + content area
pub fn render_debug_panels(
    ctx: &egui::Context,
    manager: &mut DebugPanelManager,
    dock_ctx: DockContext<'_>,
) {
    egui::SidePanel::right("debug_panels")
        .default_width(600.0) // Wider: 160px menu + 440px content
        .min_width(600.0) // Lock width to prevent resizing
        .max_width(600.0) // Lock width to prevent resizing
        .resizable(true)
        .frame(
            egui::Frame::NONE
                .fill(ctx.style().visuals.panel_fill)
                .inner_margin(4.0),
        )
        .show(ctx, |ui| {
            // Use horizontal_top to expand content to full height
            let available_height = ui.available_height();
            ui.horizontal_top(|ui| {
                // Left: Vertical menu (160px fixed)
                ui.vertical(|ui| {
                    ui.set_width(160.0);
                    ui.set_min_height(available_height);
                    ui.heading("Debug");
                    ui.separator();

                    render_panel_menu(ui, manager);
                });

                ui.separator(); // Vertical divider

                // Right: Content area (flexible width, full height)
                ui.vertical(|ui| {
                    ui.set_min_height(available_height);
                    if let Some(active) = manager.active_panel {
                        render_panel_content(ui, active, dock_ctx);
                    } else {
                        ui.centered_and_justified(|ui| {
                            ui.label("No panel selected");
                        });
                    }
                });
            });
        });
}

fn render_panel_menu(ui: &mut egui::Ui, manager: &mut DebugPanelManager) {
    // Render vertical list of selectable panels
    for tab in DockTab::all_variants() {
        let is_available = tab.is_available();
        let is_active = manager.is_active(tab);

        // Render button with active state
        let text = format!("{} {}", tab.icon(), tab);

        let button = egui::Button::new(text).fill(if is_active {
            ui.visuals().widgets.active.bg_fill
        } else {
            ui.visuals().widgets.inactive.bg_fill
        });

        let mut response = ui.add_enabled(is_available, button);

        // Add tooltip for disabled buttons
        if !is_available {
            response = response.on_disabled_hover_text(match tab {
                DockTab::Profiler => "Profiler requires compilation with --features profiling",
                DockTab::Parameters => "Parameters panel not available on WASM",
                _ => "Not available",
            });
        }

        if response.clicked() {
            manager.select_tab(tab);
        }
    }
}

fn render_panel_content(ui: &mut egui::Ui, tab: DockTab, ctx: DockContext<'_>) {
    // Reuse existing DockTabViewer rendering logic
    let mut viewer = DockTabViewer { ctx };

    match tab {
        DockTab::Stats => viewer.render_stats(ui),
        DockTab::Controls => viewer.render_controls(ui),
        DockTab::Logger => viewer.render_logger(ui),
        DockTab::LevelSelector => viewer.render_level_selector(ui),
        DockTab::Parameters => viewer.render_parameters(ui),
        #[cfg(feature = "multiplayer")]
        DockTab::MultiplayerStats => viewer.render_multiplayer_stats(ui),
        DockTab::Profiler => viewer.render_profiler(ui),
    }
}
