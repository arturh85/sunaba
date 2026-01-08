use crate::entity::player::Player;
use crate::entity::tools::ToolRegistry;
use crate::ui::theme::GameColors;
use egui::{Color32, Context, CornerRadius, Rect, Stroke, StrokeKind, Vec2};

/// Heads-up display showing player health, hunger, and hotbar
pub struct Hud {
    show: bool,
}

impl Hud {
    pub fn new() -> Self {
        Hud { show: true }
    }

    /// Toggle HUD visibility
    pub fn toggle(&mut self) {
        self.show = !self.show;
    }

    /// Render the HUD overlay
    pub fn render(
        &self,
        ctx: &Context,
        player: &Player,
        selected_material: u16,
        material_names: &[&str],
        tool_registry: &ToolRegistry,
        theme_colors: &GameColors,
    ) {
        if !self.show {
            return;
        }

        egui::Area::new("hud".into())
            .fixed_pos(egui::pos2(10.0, 10.0))
            .show(ctx, |ui| {
                ui.set_width(300.0);

                // Health bar
                self.render_stat_bar(
                    ui,
                    "Health",
                    player.health.current,
                    player.health.max,
                    theme_colors.health_full,
                    theme_colors.health_bg,
                );

                ui.add_space(5.0);

                // Hunger bar
                let hunger_color = if player.is_starving() {
                    theme_colors.hunger_starving
                } else {
                    theme_colors.hunger_full
                };

                self.render_stat_bar(
                    ui,
                    "Hunger",
                    player.hunger.current,
                    player.hunger.max,
                    hunger_color,
                    theme_colors.hunger_bg,
                );

                ui.add_space(5.0);

                // Inventory summary
                ui.label(format!(
                    "Inventory: {}/{} slots",
                    player.inventory.used_slot_count(),
                    player.inventory.max_slots
                ));

                // Show selected material
                if (selected_material as usize) < material_names.len() {
                    let count = player.inventory.count_item(selected_material);
                    ui.label(format!(
                        "Selected: {} ({})",
                        material_names[selected_material as usize], count
                    ));
                }

                ui.add_space(5.0);

                // Equipped tool display
                if let Some(tool_id) = player.equipped_tool {
                    if let Some(tool) = tool_registry.get(tool_id) {
                        let durability = player.inventory.get_tool_durability(tool_id).unwrap_or(0);
                        ui.label(format!(
                            "Equipped: {} ({}/{}⚒)",
                            tool.name,
                            durability,
                            tool.max_durability()
                        ));
                    }
                } else {
                    ui.label("Equipped: Hands (slow)");
                }

                // Mining progress bar
                if player.mining_progress.is_mining() {
                    ui.add_space(5.0);
                    ui.separator();
                    ui.label("Mining...");

                    let progress = player.mining_progress.progress;
                    let bar = egui::ProgressBar::new(progress)
                        .fill(Color32::from_rgb(100, 200, 255))
                        .animate(true);
                    ui.add(bar);
                }

                ui.add_space(5.0);

                // Controls hint
                ui.separator();
                ui.label("Left-click: Place | Hold Right-click: Mine");
                ui.label("I: Inventory | C: Crafting | H: Help");
            });
    }

    fn render_stat_bar(
        &self,
        ui: &mut egui::Ui,
        label: &str,
        current: f32,
        max: f32,
        fill_color: Color32,
        bg_color: Color32,
    ) {
        let percentage = (current / max).clamp(0.0, 1.0);

        ui.horizontal(|ui| {
            ui.label(format!("{}: ", label));

            let bar_width = 200.0;
            let bar_height = 20.0;

            let (response, painter) =
                ui.allocate_painter(Vec2::new(bar_width, bar_height), egui::Sense::hover());

            let rect = response.rect;

            // Background
            painter.rect_filled(rect, CornerRadius::same(4), bg_color);

            // Fill
            let fill_width = bar_width * percentage;
            let fill_rect = Rect::from_min_size(rect.min, Vec2::new(fill_width, bar_height));
            painter.rect_filled(fill_rect, CornerRadius::same(4), fill_color);

            // Border
            painter.rect_stroke(
                rect,
                CornerRadius::same(4),
                Stroke::new(1.5, Color32::BLACK),
                StrokeKind::Outside, // egui 0.33+ requires StrokeKind
            );

            // Text overlay
            let text = format!("{:.0}/{:.0}", current, max);
            let text_pos = rect.center();
            painter.text(
                text_pos,
                egui::Align2::CENTER_CENTER,
                &text,
                egui::FontId::proportional(14.0),
                Color32::WHITE,
            );
        });
    }
}

impl Default for Hud {
    fn default() -> Self {
        Self::new()
    }
}
