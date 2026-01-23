use crate::entity::player::Player;
use crate::ui::theme::GameColors;
use egui::{Color32, Context, CornerRadius, Pos2, Rect, Stroke, StrokeKind, Vec2};

/// Render fullscreen inventory overlay with dark backdrop
pub fn render_inventory_overlay(
    ctx: &egui::Context,
    player: &Player,
    material_names: &[&str],
    theme_colors: &GameColors,
) {
    // Dark semi-transparent backdrop covering entire screen
    egui::Area::new(egui::Id::new("inventory_backdrop"))
        .fixed_pos(egui::pos2(0.0, 0.0))
        .show(ctx, |ui| {
            let screen_rect = ctx.screen_rect();
            ui.painter().rect_filled(
                screen_rect,
                CornerRadius::ZERO,
                Color32::from_black_alpha(180),
            );
        });

    // Centered inventory window
    egui::Window::new("Inventory")
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .fixed_size([520.0, 600.0])
        .collapsible(false)
        .title_bar(true)
        .frame(egui::Frame::window(&ctx.style()).fill(ctx.style().visuals.window_fill()))
        .show(ctx, |ui| {
            ui.heading("Player Inventory");
            ui.separator();

            // Player stats section
            ui.horizontal(|ui| {
                ui.label(format!(
                    "Health: {:.0}/{:.0}",
                    player.health.current, player.health.max
                ));
                ui.separator();
                ui.label(format!(
                    "Hunger: {:.0}/{:.0}",
                    player.hunger.current, player.hunger.max
                ));
            });

            ui.add_space(10.0);

            // Inventory usage
            ui.label(format!(
                "Using {}/{} slots",
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
                                render_overlay_slot(
                                    ui,
                                    player,
                                    slot_index,
                                    material_names,
                                    SLOT_SIZE,
                                    theme_colors,
                                );
                                ui.add_space(SPACING);
                            }
                        });
                        ui.add_space(SPACING);
                    }
                });

            ui.add_space(10.0);
            ui.separator();
            ui.colored_label(theme_colors.text_disabled, "Press I or ESC to close");
        });
}

/// Render a single inventory slot for the overlay
fn render_overlay_slot(
    ui: &mut egui::Ui,
    player: &Player,
    slot_index: usize,
    material_names: &[&str],
    size: f32,
    theme_colors: &GameColors,
) {
    let slot_data = player.inventory.get_slot(slot_index);

    let (response, painter) = ui.allocate_painter(Vec2::new(size, size), egui::Sense::hover());

    let rect = response.rect;

    // Determine if this is the selected slot (hotbar 0-9)
    let is_selected = slot_index == player.selected_slot && slot_index < 10;

    // Background color
    let bg_color = if is_selected {
        theme_colors.selection_bg
    } else {
        theme_colors.slot_empty
    };

    painter.rect_filled(rect, CornerRadius::same(4), bg_color);

    // If slot has items, render them
    if let Some(Some(stack)) = slot_data {
        use crate::entity::inventory::ItemStack;

        match stack {
            ItemStack::Material { material_id, count } => {
                // Material color indicator (top bar)
                let material_color = get_material_color(*material_id);
                let color_bar = Rect::from_min_size(rect.min, Vec2::new(size, 8.0));
                painter.rect_filled(color_bar, CornerRadius::same(2), material_color);

                // Material name (truncated)
                let name = if (*material_id as usize) < material_names.len() {
                    material_names[*material_id as usize]
                } else {
                    "Unknown"
                };

                let text_pos = Pos2::new(rect.center().x, rect.min.y + 18.0);

                painter.text(
                    text_pos,
                    egui::Align2::CENTER_CENTER,
                    name,
                    egui::FontId::proportional(10.0),
                    Color32::WHITE,
                );

                // Count
                let count_text = if *count >= 1000 {
                    format!("{}k", count / 1000)
                } else {
                    count.to_string()
                };

                let count_pos = Pos2::new(rect.center().x, rect.max.y - 10.0);

                painter.text(
                    count_pos,
                    egui::Align2::CENTER_CENTER,
                    &count_text,
                    egui::FontId::proportional(12.0),
                    theme_colors.text_secondary,
                );
            }
            ItemStack::Tool {
                tool_id,
                durability,
            } => {
                // Tool indicator color (golden for tools)
                let tool_color = theme_colors.tool_legendary;
                let color_bar = Rect::from_min_size(rect.min, Vec2::new(size, 8.0));
                painter.rect_filled(color_bar, CornerRadius::same(2), tool_color);

                // Tool name (hardcoded for now - registry not accessible from UI)
                let name = match *tool_id {
                    1000 => "Wood Pick",
                    1001 => "Stone Pick",
                    1002 => "Iron Pick",
                    _ => "Tool",
                };

                let text_pos = Pos2::new(rect.center().x, rect.min.y + 18.0);

                painter.text(
                    text_pos,
                    egui::Align2::CENTER_CENTER,
                    name,
                    egui::FontId::proportional(10.0),
                    Color32::WHITE,
                );

                // Durability bar
                let durability_text = format!("{}", durability);
                let durability_pos = Pos2::new(rect.center().x, rect.max.y - 10.0);

                painter.text(
                    durability_pos,
                    egui::Align2::CENTER_CENTER,
                    &durability_text,
                    egui::FontId::proportional(12.0),
                    theme_colors.tool_durability_full,
                );
            }
        }
    } else {
        // Empty slot - show slot number for hotbar
        if slot_index < 10 {
            let slot_text = format!("{}", (slot_index + 1) % 10);
            painter.text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                &slot_text,
                egui::FontId::proportional(16.0),
                theme_colors.text_disabled,
            );
        }
    }

    // Border
    let border_color = if is_selected {
        theme_colors.selection_border
    } else {
        theme_colors.border_normal
    };

    let border_width = if is_selected { 2.0 } else { 1.0 };

    painter.rect_stroke(
        rect,
        CornerRadius::same(4),
        Stroke::new(border_width, border_color),
        StrokeKind::Outside,
    );

    // Tooltip on hover
    if response.hovered() {
        if let Some(Some(stack)) = slot_data {
            use crate::entity::inventory::ItemStack;

            match stack {
                ItemStack::Material { material_id, count } => {
                    let name = if (*material_id as usize) < material_names.len() {
                        material_names[*material_id as usize]
                    } else {
                        "Unknown"
                    };

                    response
                        .on_hover_text(format!("{}\nCount: {}\nID: {}", name, count, material_id));
                }
                ItemStack::Tool {
                    tool_id,
                    durability,
                } => {
                    let name = match *tool_id {
                        1000 => "Wood Pickaxe",
                        1001 => "Stone Pickaxe",
                        1002 => "Iron Pickaxe",
                        _ => "Unknown Tool",
                    };

                    response.on_hover_text(format!(
                        "{}\nDurability: {}\nTool ID: {}",
                        name, durability, tool_id
                    ));
                }
            }
        } else if slot_index < 10 {
            response.on_hover_text(format!("Hotbar slot {} (empty)", (slot_index + 1) % 10));
        }
    }
}

/// Get material color for overlay display
fn get_material_color(material_id: u16) -> Color32 {
    match material_id {
        0 => Color32::from_rgb(0, 0, 0),        // Air (shouldn't appear)
        1 => Color32::from_rgb(128, 128, 128),  // Stone
        2 => Color32::from_rgb(128, 128, 128),  // Stone (duplicate ID?)
        3 => Color32::from_rgb(194, 178, 128),  // Sand
        4 => Color32::from_rgb(50, 100, 200),   // Water
        5 => Color32::from_rgb(139, 90, 43),    // Wood
        6 => Color32::from_rgb(255, 100, 0),    // Fire
        7 => Color32::from_rgb(100, 100, 100),  // Smoke
        8 => Color32::from_rgb(200, 200, 255),  // Steam
        9 => Color32::from_rgb(255, 69, 0),     // Lava
        10 => Color32::from_rgb(50, 50, 50),    // Oil
        11 => Color32::from_rgb(0, 255, 0),     // Acid
        12 => Color32::from_rgb(180, 220, 255), // Ice
        13 => Color32::from_rgb(200, 220, 220), // Glass
        14 => Color32::from_rgb(180, 180, 180), // Metal
        _ => Color32::from_rgb(150, 150, 150),  // Default
    }
}

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
    pub fn render(
        &mut self,
        ctx: &Context,
        player: &Player,
        material_names: &[&str],
        theme_colors: &GameColors,
    ) {
        if !self.open {
            return;
        }

        egui::Window::new("Inventory")
            .fixed_size([520.0, 600.0])
            .collapsible(false)
            .frame(egui::Frame::window(&ctx.style()).fill(ctx.style().visuals.window_fill()))
            .show(ctx, |ui| {
                ui.heading("Player Inventory");
                ui.separator();

                // Player stats section
                ui.horizontal(|ui| {
                    ui.label(format!(
                        "Health: {:.0}/{:.0}",
                        player.health.current, player.health.max
                    ));
                    ui.separator();
                    ui.label(format!(
                        "Hunger: {:.0}/{:.0}",
                        player.hunger.current, player.hunger.max
                    ));
                });

                ui.add_space(10.0);

                // Inventory usage
                ui.label(format!(
                    "Using {}/{} slots",
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
                                        theme_colors,
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
        theme_colors: &GameColors,
    ) {
        let slot_data = player.inventory.get_slot(slot_index);

        let (response, painter) = ui.allocate_painter(Vec2::new(size, size), egui::Sense::hover());

        let rect = response.rect;

        // Determine if this is the selected slot (hotbar 0-9)
        let is_selected = slot_index == player.selected_slot && slot_index < 10;

        // Background color
        let bg_color = if is_selected {
            theme_colors.selection_bg
        } else {
            theme_colors.slot_empty
        };

        painter.rect_filled(rect, CornerRadius::same(4), bg_color);

        // If slot has items, render them
        if let Some(Some(stack)) = slot_data {
            use crate::entity::inventory::ItemStack;

            match stack {
                ItemStack::Material { material_id, count } => {
                    // Material color indicator (top bar)
                    let material_color = self.get_material_color(*material_id);
                    let color_bar = Rect::from_min_size(rect.min, Vec2::new(size, 8.0));
                    painter.rect_filled(color_bar, CornerRadius::same(2), material_color);

                    // Material name (truncated)
                    let name = if (*material_id as usize) < material_names.len() {
                        material_names[*material_id as usize]
                    } else {
                        "Unknown"
                    };

                    let text_pos = Pos2::new(rect.center().x, rect.min.y + 18.0);

                    painter.text(
                        text_pos,
                        egui::Align2::CENTER_CENTER,
                        name,
                        egui::FontId::proportional(10.0),
                        Color32::WHITE,
                    );

                    // Count
                    let count_text = if *count >= 1000 {
                        format!("{}k", count / 1000)
                    } else {
                        count.to_string()
                    };

                    let count_pos = Pos2::new(rect.center().x, rect.max.y - 10.0);

                    painter.text(
                        count_pos,
                        egui::Align2::CENTER_CENTER,
                        &count_text,
                        egui::FontId::proportional(12.0),
                        theme_colors.text_secondary,
                    );
                }
                ItemStack::Tool {
                    tool_id,
                    durability,
                } => {
                    // Tool indicator color (golden for tools)
                    let tool_color = theme_colors.tool_legendary;
                    let color_bar = Rect::from_min_size(rect.min, Vec2::new(size, 8.0));
                    painter.rect_filled(color_bar, CornerRadius::same(2), tool_color);

                    // Tool name (hardcoded for now - registry not accessible from UI)
                    let name = match *tool_id {
                        1000 => "Wood Pick",
                        1001 => "Stone Pick",
                        1002 => "Iron Pick",
                        _ => "Tool",
                    };

                    let text_pos = Pos2::new(rect.center().x, rect.min.y + 18.0);

                    painter.text(
                        text_pos,
                        egui::Align2::CENTER_CENTER,
                        name,
                        egui::FontId::proportional(10.0),
                        Color32::WHITE,
                    );

                    // Durability bar
                    let durability_text = format!("{}", durability);
                    let durability_pos = Pos2::new(rect.center().x, rect.max.y - 10.0);

                    painter.text(
                        durability_pos,
                        egui::Align2::CENTER_CENTER,
                        &durability_text,
                        egui::FontId::proportional(12.0),
                        theme_colors.tool_durability_full,
                    );
                }
            }
        } else {
            // Empty slot - show slot number for hotbar
            if slot_index < 10 {
                let slot_text = format!("{}", (slot_index + 1) % 10);
                painter.text(
                    rect.center(),
                    egui::Align2::CENTER_CENTER,
                    &slot_text,
                    egui::FontId::proportional(16.0),
                    theme_colors.text_disabled,
                );
            }
        }

        // Border
        let border_color = if is_selected {
            theme_colors.selection_border
        } else {
            theme_colors.border_normal
        };

        let border_width = if is_selected { 2.0 } else { 1.0 };

        painter.rect_stroke(
            rect,
            CornerRadius::same(4),
            Stroke::new(border_width, border_color),
            StrokeKind::Outside, // egui 0.33+ requires StrokeKind
        );

        // Tooltip on hover
        if response.hovered() {
            if let Some(Some(stack)) = slot_data {
                use crate::entity::inventory::ItemStack;

                match stack {
                    ItemStack::Material { material_id, count } => {
                        let name = if (*material_id as usize) < material_names.len() {
                            material_names[*material_id as usize]
                        } else {
                            "Unknown"
                        };

                        response.on_hover_text(format!(
                            "{}\nCount: {}\nID: {}",
                            name, count, material_id
                        ));
                    }
                    ItemStack::Tool {
                        tool_id,
                        durability,
                    } => {
                        let name = match *tool_id {
                            1000 => "Wood Pickaxe",
                            1001 => "Stone Pickaxe",
                            1002 => "Iron Pickaxe",
                            _ => "Unknown Tool",
                        };

                        response.on_hover_text(format!(
                            "{}\nDurability: {}\nTool ID: {}",
                            name, durability, tool_id
                        ));
                    }
                }
            } else if slot_index < 10 {
                response.on_hover_text(format!("Hotbar slot {} (empty)", (slot_index + 1) % 10));
            }
        }
    }

    fn get_material_color(&self, material_id: u16) -> Color32 {
        // Simple color mapping based on material ID
        // This should ideally come from the actual material definitions
        match material_id {
            0 => Color32::from_rgb(0, 0, 0),        // Air (shouldn't appear)
            1 => Color32::from_rgb(128, 128, 128),  // Stone
            2 => Color32::from_rgb(128, 128, 128),  // Stone (duplicate ID?)
            3 => Color32::from_rgb(194, 178, 128),  // Sand
            4 => Color32::from_rgb(50, 100, 200),   // Water
            5 => Color32::from_rgb(139, 90, 43),    // Wood
            6 => Color32::from_rgb(255, 100, 0),    // Fire
            7 => Color32::from_rgb(100, 100, 100),  // Smoke
            8 => Color32::from_rgb(200, 200, 255),  // Steam
            9 => Color32::from_rgb(255, 69, 0),     // Lava
            10 => Color32::from_rgb(50, 50, 50),    // Oil
            11 => Color32::from_rgb(0, 255, 0),     // Acid
            12 => Color32::from_rgb(180, 220, 255), // Ice
            13 => Color32::from_rgb(200, 220, 220), // Glass
            14 => Color32::from_rgb(180, 180, 180), // Metal
            _ => Color32::from_rgb(150, 150, 150),  // Default
        }
    }
}

impl Default for InventoryPanel {
    fn default() -> Self {
        Self::new()
    }
}
