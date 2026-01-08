//! Game over screen shown when player dies

use crate::ui::theme::GameColors;
use egui::{Align2, Color32, CornerRadius, Vec2};

/// Game over panel state
pub struct GameOverPanelState {
    /// User clicked respawn button
    pub respawn_requested: bool,
}

impl GameOverPanelState {
    pub fn new() -> Self {
        Self {
            respawn_requested: false,
        }
    }

    /// Reset action flags after processing
    pub fn reset_flags(&mut self) {
        self.respawn_requested = false;
    }

    /// Render game over overlay (fullscreen)
    pub fn render(&mut self, ctx: &egui::Context, theme_colors: &GameColors) {
        // Fullscreen dark overlay
        egui::Area::new("game_over_overlay".into())
            .fixed_pos(egui::pos2(0.0, 0.0))
            .anchor(Align2::LEFT_TOP, Vec2::ZERO)
            .show(ctx, |ui| {
                // Semi-transparent black background
                let screen_rect = ctx.content_rect();
                ui.painter().rect_filled(
                    screen_rect,
                    CornerRadius::ZERO,
                    Color32::from_black_alpha(200),
                );

                // Centered game over window
                egui::Window::new("game_over_window")
                    .title_bar(false)
                    .resizable(false)
                    .anchor(Align2::CENTER_CENTER, Vec2::ZERO)
                    .show(ctx, |ui| {
                        ui.vertical_centered(|ui| {
                            ui.add_space(20.0);

                            // "You Died" message
                            ui.heading(
                                egui::RichText::new("YOU DIED")
                                    .color(theme_colors.error)
                                    .size(48.0),
                            );

                            ui.add_space(40.0);

                            // Respawn button
                            let button =
                                egui::Button::new(egui::RichText::new("Respawn").size(24.0))
                                    .min_size(Vec2::new(200.0, 60.0));

                            if ui.add(button).clicked() {
                                self.respawn_requested = true;
                            }

                            ui.add_space(20.0);
                        });
                    });
            });
    }
}

impl Default for GameOverPanelState {
    fn default() -> Self {
        Self::new()
    }
}
