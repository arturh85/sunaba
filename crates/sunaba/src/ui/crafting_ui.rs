//! Crafting UI panel

use crate::entity::crafting::{Recipe, RecipeOutput, RecipeRegistry};
use crate::entity::inventory::Inventory;
use crate::simulation::Materials;
use crate::ui::theme::GameColors;
use egui::{Color32, CornerRadius, RichText};

/// Render fullscreen crafting overlay with dark backdrop
/// Sets `pending_craft` to the recipe name when user clicks a craft button
pub fn render_crafting_overlay(
    ctx: &egui::Context,
    inventory: &Inventory,
    recipes: &RecipeRegistry,
    materials: &Materials,
    theme_colors: &GameColors,
    pending_craft: &mut Option<String>,
) {
    // Dark semi-transparent backdrop covering entire screen
    egui::Area::new(egui::Id::new("crafting_backdrop"))
        .fixed_pos(egui::pos2(0.0, 0.0))
        .show(ctx, |ui| {
            let screen_rect = ctx.screen_rect();
            ui.painter().rect_filled(
                screen_rect,
                CornerRadius::ZERO,
                Color32::from_black_alpha(180),
            );
        });

    // Centered crafting window
    egui::Window::new("Crafting")
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .fixed_size([500.0, 500.0])
        .collapsible(false)
        .title_bar(true)
        .frame(egui::Frame::window(&ctx.style()).fill(ctx.style().visuals.window_fill()))
        .show(ctx, |ui| {
            ui.heading("Available Recipes");
            ui.add_space(10.0);

            let craftable = recipes.get_craftable(inventory);

            if craftable.is_empty() {
                ui.colored_label(theme_colors.text_disabled, "No craftable recipes");
                ui.label("Gather more materials to unlock recipes.");
            } else {
                egui::ScrollArea::vertical()
                    .max_height(380.0)
                    .show(ui, |ui| {
                        // Render each craftable recipe with craft button
                        for recipe in &craftable {
                            ui.separator();
                            if render_recipe_with_button(
                                ui,
                                recipe,
                                inventory,
                                materials,
                                theme_colors,
                            ) {
                                // User clicked craft - store the recipe name for the App to process
                                *pending_craft = Some(recipe.name.clone());
                            }
                        }
                    });
            }

            ui.add_space(10.0);
            ui.separator();
            ui.horizontal(|ui| {
                ui.label(format!("Total recipes: {}", recipes.all_recipes().len()));
                ui.separator();
                ui.label(format!("Craftable: {}", craftable.len()));
            });
            ui.add_space(5.0);
            ui.colored_label(theme_colors.text_disabled, "Press C or ESC to close");
        });
}

/// Render a single recipe with a craft button
/// Returns true if the craft button was clicked
fn render_recipe_with_button(
    ui: &mut egui::Ui,
    recipe: &Recipe,
    inventory: &Inventory,
    materials: &Materials,
    theme_colors: &GameColors,
) -> bool {
    let mut craft_clicked = false;

    ui.horizontal(|ui| {
        // Recipe name
        ui.label(RichText::new(&recipe.name).size(16.0).strong());

        ui.add_space(10.0);

        // Craft button
        if ui.button("Craft").clicked() {
            craft_clicked = true;
        }
    });

    // Show inputs
    ui.horizontal(|ui| {
        ui.label(RichText::new("Requires:").color(theme_colors.text_disabled));

        for (i, (mat_id, count)) in recipe.inputs.iter().enumerate() {
            if i > 0 {
                ui.label("+");
            }

            let mat_name = &materials.get(*mat_id).name;
            let has_count = inventory.count_item(*mat_id);

            let text = format!("{} x{}", mat_name, count);
            let color = if has_count >= *count {
                theme_colors.craftable
            } else {
                theme_colors.not_craftable
            };

            ui.colored_label(color, text);
        }
    });

    // Show output
    ui.horizontal(|ui| {
        ui.label(RichText::new("Produces:").color(theme_colors.text_disabled));

        match &recipe.output {
            RecipeOutput::Material { id, count } => {
                let mat_name = &materials.get(*id).name;
                ui.colored_label(theme_colors.info, format!("{} x{}", mat_name, count));
            }
            RecipeOutput::Tool {
                tool_id,
                durability,
            } => {
                let tool_name = match *tool_id {
                    1000 => "Wood Pickaxe",
                    1001 => "Stone Pickaxe",
                    1002 => "Iron Pickaxe",
                    _ => "Unknown Tool",
                };
                ui.colored_label(
                    theme_colors.tool_legendary,
                    format!("{} ({}⚒)", tool_name, durability),
                );
            }
        }
    });

    ui.add_space(5.0);

    craft_clicked
}

pub struct CraftingUI {
    pub visible: bool,
}

impl CraftingUI {
    pub fn new() -> Self {
        Self { visible: false }
    }

    pub fn toggle(&mut self) {
        self.visible = !self.visible;
    }

    /// Render the crafting window
    /// Returns Some(output) if a recipe was crafted this frame
    pub fn render(
        &mut self,
        ctx: &egui::Context,
        inventory: &mut Inventory,
        recipes: &RecipeRegistry,
        materials: &Materials,
        theme_colors: &GameColors,
    ) -> Option<RecipeOutput> {
        if !self.visible {
            return None;
        }

        let mut crafted_output = None;

        egui::Window::new("Crafting")
            .default_width(500.0)
            .open(&mut self.visible)
            .frame(egui::Frame::window(&ctx.style()).fill(ctx.style().visuals.window_fill()))
            .show(ctx, |ui| {
                ui.heading("Available Recipes");
                ui.add_space(10.0);

                let craftable = recipes.get_craftable(inventory);

                if craftable.is_empty() {
                    ui.colored_label(theme_colors.text_disabled, "No craftable recipes");
                    ui.label("Gather more materials to unlock recipes.");
                    return;
                }

                // Render each craftable recipe
                for recipe in &craftable {
                    ui.separator();
                    crafted_output = Self::render_recipe(
                        ui,
                        recipe,
                        inventory,
                        recipes,
                        materials,
                        theme_colors,
                    );
                    if crafted_output.is_some() {
                        break; // Only craft one item per frame
                    }
                }

                ui.add_space(10.0);
                ui.separator();
                ui.label(format!("Total recipes: {}", recipes.all_recipes().len()));
                ui.label(format!("Craftable: {}", craftable.len()));
            });

        crafted_output
    }

    /// Render a single recipe
    /// Returns Some(output) if the recipe was crafted
    fn render_recipe(
        ui: &mut egui::Ui,
        recipe: &Recipe,
        inventory: &mut Inventory,
        recipes: &RecipeRegistry,
        materials: &Materials,
        theme_colors: &GameColors,
    ) -> Option<RecipeOutput> {
        let mut crafted = None;

        ui.horizontal(|ui| {
            // Recipe name
            ui.label(RichText::new(&recipe.name).size(16.0).strong());

            ui.add_space(10.0);

            // Craft button
            if ui.button("Craft").clicked() {
                crafted = recipes.try_craft(recipe, inventory);
                if crafted.is_some() {
                    log::info!("[CRAFTING] Crafted: {}", recipe.name);
                } else {
                    log::warn!("[CRAFTING] Failed to craft: {}", recipe.name);
                }
            }
        });

        // Show inputs
        ui.horizontal(|ui| {
            ui.label(RichText::new("Requires:").color(theme_colors.text_disabled));

            for (i, (mat_id, count)) in recipe.inputs.iter().enumerate() {
                if i > 0 {
                    ui.label("+");
                }

                let mat_name = &materials.get(*mat_id).name;
                let has_count = inventory.count_item(*mat_id);

                let text = format!("{} x{}", mat_name, count);
                let color = if has_count >= *count {
                    theme_colors.craftable
                } else {
                    theme_colors.not_craftable
                };

                ui.colored_label(color, text);
            }
        });

        // Show output
        ui.horizontal(|ui| {
            ui.label(RichText::new("Produces:").color(theme_colors.text_disabled));

            match &recipe.output {
                RecipeOutput::Material { id, count } => {
                    let mat_name = &materials.get(*id).name;
                    ui.colored_label(theme_colors.info, format!("{} x{}", mat_name, count));
                }
                RecipeOutput::Tool {
                    tool_id,
                    durability,
                } => {
                    let tool_name = match *tool_id {
                        1000 => "Wood Pickaxe",
                        1001 => "Stone Pickaxe",
                        1002 => "Iron Pickaxe",
                        _ => "Unknown Tool",
                    };
                    ui.colored_label(
                        theme_colors.tool_legendary,
                        format!("{} ({}⚒)", tool_name, durability),
                    );
                }
            }
        });

        ui.add_space(5.0);

        crafted
    }
}

impl Default for CraftingUI {
    fn default() -> Self {
        Self::new()
    }
}
