use egui::{Color32, Context, Pos2, Rect, Rounding, Stroke, Vec2};
use crate::entity::player::Player;

/// Full inventory panel (opened with I key)
pub struct InventoryPanel {
    pub open: bool,
}

impl InventoryPanel {
    pub fn new() -> Self {
        InventoryPanel { open: false }
    }

    /// Toggle inventory panel
    pub fn toggle(&mut self) {
        self.open = !self.open;
    }

    /// Render the inventory panel
    pub fn render(&mut self, ctx: &Context, player: &Player, material_names: &[&str]) {
        if !self.open {
            return;
        }

        egui::Window::new("Inventory")
            .fixed_size([520.0, 600.0])
            .collapsible(false)
            .show(ctx, |ui| {
                ui.heading("Player Inventory");
                ui.separator();

                // Player stats section
                ui.horizontal(|ui| {
                    ui.label(format!("Health: {:.0}/{:.0}", player.health.current, player.health.max));
                    ui.separator();
                    ui.label(format!("Hunger: {:.0}/{:.0}", player.hunger.current, player.hunger.max));
                });

                ui.add_space(10.0);

                // Inventory usage
                ui.label(format!("Using {}/{} slots",
                    player.inventory.used_slot_count(),
                    player.inventory.max_slots
                ));

                ui.add_space(5.0);
                ui.separator();
                ui.add_space(5.0);

                // Inventory grid (10 columns x 5 rows = 50 slots)
                const COLS: usize = 10;
                const SLOT_SIZE: f32 = 45.0;
                const SPACING: f32 = 5.0;

                egui::ScrollArea::vertical()
                    .max_height(450.0)
                    .show(ui, |ui| {
                        for row in 0..5 {
                            ui.horizontal(|ui| {
                                for col in 0..COLS {
                                    let slot_index = row * COLS + col;
                                    self.render_inventory_slot(
                                        ui,
                                        player,
                                        slot_index,
                                        material_names,
                                        SLOT_SIZE,
                                    );
                                    ui.add_space(SPACING);
                                }
                            });
                            ui.add_space(SPACING);
                        }
                    });

                ui.add_space(10.0);
                ui.separator();
                ui.label("Press I to close");
            });
    }

    fn render_inventory_slot(
        &self,
        ui: &mut egui::Ui,
        player: &Player,
        slot_index: usize,
        material_names: &[&str],
        size: f32,
    ) {
        let slot_data = player.inventory.get_slot(slot_index);

        let (response, painter) = ui.allocate_painter(
            Vec2::new(size, size),
            egui::Sense::hover()
        );

        let rect = response.rect;

        // Determine if this is the selected slot (hotbar 0-9)
        let is_selected = slot_index == player.selected_slot && slot_index < 10;

        // Background color
        let bg_color = if is_selected {
            Color32::from_rgb(80, 80, 120) // Highlighted if selected
        } else {
            Color32::from_rgb(40, 40, 40)
        };

        painter.rect_filled(rect, Rounding::same(4.0), bg_color);

        // If slot has items, render them
        if let Some(Some(stack)) = slot_data {
            // Material color indicator (top bar)
            let material_color = self.get_material_color(stack.material_id);
            let color_bar = Rect::from_min_size(
                rect.min,
                Vec2::new(size, 8.0)
            );
            painter.rect_filled(color_bar, Rounding::same(2.0), material_color);

            // Material name (truncated)
            let name = if (stack.material_id as usize) < material_names.len() {
                material_names[stack.material_id as usize]
            } else {
                "Unknown"
            };

            let text_pos = Pos2::new(
                rect.center().x,
                rect.min.y + 18.0
            );

            painter.text(
                text_pos,
                egui::Align2::CENTER_CENTER,
                name,
                egui::FontId::proportional(10.0),
                Color32::WHITE,
            );

            // Count
            let count_text = if stack.count >= 1000 {
                format!("{}k", stack.count / 1000)
            } else {
                stack.count.to_string()
            };

            let count_pos = Pos2::new(
                rect.center().x,
                rect.max.y - 10.0
            );

            painter.text(
                count_pos,
                egui::Align2::CENTER_CENTER,
                &count_text,
                egui::FontId::proportional(12.0),
                Color32::from_rgb(200, 200, 200),
            );
        } else {
            // Empty slot - show slot number for hotbar
            if slot_index < 10 {
                let slot_text = format!("{}", (slot_index + 1) % 10);
                painter.text(
                    rect.center(),
                    egui::Align2::CENTER_CENTER,
                    &slot_text,
                    egui::FontId::proportional(16.0),
                    Color32::from_rgb(80, 80, 80),
                );
            }
        }

        // Border
        let border_color = if is_selected {
            Color32::from_rgb(150, 150, 200)
        } else {
            Color32::from_rgb(80, 80, 80)
        };

        let border_width = if is_selected { 2.0 } else { 1.0 };

        painter.rect_stroke(rect, Rounding::same(4.0), Stroke::new(border_width, border_color));

        // Tooltip on hover
        if response.hovered() {
            if let Some(Some(stack)) = slot_data {
                let name = if (stack.material_id as usize) < material_names.len() {
                    material_names[stack.material_id as usize]
                } else {
                    "Unknown"
                };

                response.on_hover_text(format!(
                    "{}\nCount: {}\nID: {}",
                    name,
                    stack.count,
                    stack.material_id
                ));
            } else if slot_index < 10 {
                response.on_hover_text(format!("Hotbar slot {} (empty)", (slot_index + 1) % 10));
            }
        }
    }

    fn get_material_color(&self, material_id: u16) -> Color32 {
        // Simple color mapping based on material ID
        // This should ideally come from the actual material definitions
        match material_id {
            0 => Color32::from_rgb(0, 0, 0),       // Air (shouldn't appear)
            1 => Color32::from_rgb(128, 128, 128), // Stone
            2 => Color32::from_rgb(128, 128, 128), // Stone (duplicate ID?)
            3 => Color32::from_rgb(194, 178, 128), // Sand
            4 => Color32::from_rgb(50, 100, 200),  // Water
            5 => Color32::from_rgb(139, 90, 43),   // Wood
            6 => Color32::from_rgb(255, 100, 0),   // Fire
            7 => Color32::from_rgb(100, 100, 100), // Smoke
            8 => Color32::from_rgb(200, 200, 255), // Steam
            9 => Color32::from_rgb(255, 69, 0),    // Lava
            10 => Color32::from_rgb(50, 50, 50),   // Oil
            11 => Color32::from_rgb(0, 255, 0),    // Acid
            12 => Color32::from_rgb(180, 220, 255),// Ice
            13 => Color32::from_rgb(200, 220, 220),// Glass
            14 => Color32::from_rgb(180, 180, 180),// Metal
            _ => Color32::from_rgb(150, 150, 150), // Default
        }
    }
}

impl Default for InventoryPanel {
    fn default() -> Self {
        Self::new()
    }
}
