//! Escape menu system
//! Provides pause menu with Resume, Settings, Debug Panels, Save, Quit

use super::debug_panel::DebugPanelManager;
use super::dock::DockTab;

/// Escape menu state and rendering
pub struct EscapeMenu {
    pub visible: bool,
    pub submenu: Option<EscapeSubmenu>,
}

pub enum EscapeSubmenu {
    DebugPanels, // Checkboxes for panel toggles
}

impl EscapeMenu {
    pub fn new() -> Self {
        Self {
            visible: false,
            submenu: None,
        }
    }

    pub fn toggle(&mut self) {
        self.visible = !self.visible;
        if !self.visible {
            self.submenu = None; // Close submenu when hiding
        }
    }

    pub fn show(&mut self) {
        self.visible = true;
    }

    pub fn hide(&mut self) {
        self.visible = false;
        self.submenu = None;
    }

    pub fn is_visible(&self) -> bool {
        self.visible
    }
}

impl Default for EscapeMenu {
    fn default() -> Self {
        Self::new()
    }
}

/// Actions that can be triggered by the escape menu
#[derive(Default)]
pub struct EscapeMenuAction {
    pub resume: bool,
    pub quit: bool,
    pub save: bool,
}

/// Render escape menu overlay
pub fn render_escape_menu(
    ctx: &egui::Context,
    menu: &mut EscapeMenu,
    panels: &mut DebugPanelManager,
) -> EscapeMenuAction {
    let mut action = EscapeMenuAction::default();

    if !menu.visible {
        return action;
    }

    // Fullscreen semi-transparent backdrop
    egui::Area::new("escape_backdrop".into())
        .fixed_pos(egui::pos2(0.0, 0.0))
        .show(ctx, |ui| {
            let screen_rect = ctx.viewport_rect();
            ui.painter().rect_filled(
                screen_rect,
                egui::CornerRadius::ZERO,
                egui::Color32::from_black_alpha(204), // 80% opacity
            );
        });

    // Centered menu window
    egui::Window::new("##escape_menu")
        .title_bar(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
        .fixed_size([300.0, 400.0])
        .show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.heading("GAME PAUSED");
                ui.add_space(20.0);

                // Resume button
                if ui
                    .button(egui::RichText::new("▶ Resume").size(20.0))
                    .clicked()
                {
                    action.resume = true;
                }
                ui.add_space(10.0);

                // Settings button (opens Parameters panel)
                if ui
                    .button(egui::RichText::new("⚙ Settings").size(20.0))
                    .clicked()
                {
                    panels.select_tab(DockTab::Parameters);
                    action.resume = true; // Close menu, show settings
                }
                ui.add_space(10.0);

                // Debug Panels submenu
                if ui
                    .button(egui::RichText::new("🎮 Debug Panels").size(20.0))
                    .clicked()
                {
                    menu.submenu = Some(EscapeSubmenu::DebugPanels);
                }

                // Show debug panel toggles if submenu open
                if matches!(menu.submenu, Some(EscapeSubmenu::DebugPanels)) {
                    ui.separator();
                    ui.label("Toggle Debug Panels:");

                    for tab in DockTab::all_variants() {
                        if !tab.is_available() {
                            continue;
                        }

                        let mut is_open = panels.is_open(tab);
                        if ui.checkbox(&mut is_open, tab.to_string()).changed() {
                            panels.toggle_tab(tab);
                        }
                    }
                    ui.separator();
                }

                ui.add_space(10.0);

                // Save button
                if ui
                    .button(egui::RichText::new("💾 Save Game").size(20.0))
                    .clicked()
                {
                    action.save = true;
                }
                ui.add_space(10.0);

                // Quit button
                if ui
                    .button(egui::RichText::new("🚪 Quit").size(20.0))
                    .clicked()
                {
                    action.quit = true;
                }
            });
        });

    action
}
