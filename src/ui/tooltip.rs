//! Mouseover tooltip showing pixel information

use crate::world::World;
use crate::simulation::Materials;

/// Tooltip state for displaying pixel information at cursor
pub struct TooltipState {
    visible: bool,
    material_name: String,
    temperature: f32,
    world_pos: (i32, i32),
    // Growth-related data (simplified)
    light_level: u8,
    has_nearby_water: bool,
    growth_status: String,  // Single simplified message
    // Light overlay state
    light_overlay_active: bool,  // Show light level guide when true
}

impl TooltipState {
    pub fn new() -> Self {
        Self {
            visible: false,
            material_name: String::from("Air"),
            temperature: 20.0,
            world_pos: (0, 0),
            light_level: 0,
            has_nearby_water: false,
            growth_status: String::new(),
            light_overlay_active: false,
        }
    }

    /// Update tooltip with information from world at mouse position
    pub fn update(&mut self, world: &World, materials: &Materials, mouse_world_pos: Option<(i32, i32)>, light_overlay_active: bool) {
        use crate::simulation::MaterialId;

        self.light_overlay_active = light_overlay_active;

        if let Some((wx, wy)) = mouse_world_pos {
            self.world_pos = (wx, wy);

            // Query pixel at mouse position
            if let Some(pixel) = world.get_pixel(wx, wy) {
                if pixel.is_empty() {
                    self.visible = false;
                    self.material_name = String::from("Air");
                    self.temperature = 20.0;
                    self.light_level = 0;
                    self.has_nearby_water = false;
                    self.growth_status = String::new();
                } else {
                    self.visible = true;
                    let material = materials.get(pixel.material_id);
                    self.material_name = material.name.clone();

                    // Get temperature at this pixel
                    self.temperature = world.get_temperature_at_pixel(wx, wy);

                    // Get light level
                    self.light_level = world.get_light_at(wx, wy).unwrap_or(0);

                    // Check for plant matter - compute simplified growth status
                    if pixel.material_id == MaterialId::PLANT_MATTER {
                        // Check for water in 4 neighbors
                        self.has_nearby_water = false;
                        for (dx, dy) in [(0, 1), (1, 0), (0, -1), (-1, 0)] {
                            if let Some(neighbor) = world.get_pixel(wx + dx, wy + dy) {
                                if neighbor.material_id == MaterialId::WATER {
                                    self.has_nearby_water = true;
                                    break;
                                }
                            }
                        }

                        // Check growth conditions
                        let light_ok = self.light_level >= 8;
                        let temp_ok = self.temperature >= 10.0 && self.temperature <= 40.0;

                        // Generate simple status message
                        if !light_ok {
                            self.growth_status = "Cannot grow: no light".to_string();
                        } else if !self.has_nearby_water {
                            self.growth_status = "Cannot grow: no water".to_string();
                        } else if !temp_ok {
                            self.growth_status = "Cannot grow: wrong temperature".to_string();
                        } else {
                            // Growing - show time-based progress
                            let progress = world.get_growth_progress_percent();
                            let seconds = progress / 10.0;
                            self.growth_status = format!("Growing: {:.0}% ({:.1}s)", progress, seconds);
                        }
                    } else {
                        // Not a plant - clear growth fields
                        self.has_nearby_water = false;
                        self.growth_status = String::new();
                    }
                }
            } else {
                // No chunk loaded at this position
                self.visible = false;
            }
        } else {
            self.visible = false;
        }
    }

    /// Render tooltip near cursor position
    pub fn render(&self, ctx: &egui::Context, cursor_screen_pos: egui::Pos2) {
        if !self.visible {
            return;
        }

        // Offset tooltip slightly from cursor to avoid blocking view
        let tooltip_pos = egui::pos2(cursor_screen_pos.x + 20.0, cursor_screen_pos.y + 20.0);

        egui::Window::new("##tooltip")
            .title_bar(false)
            .resizable(false)
            .fixed_pos(tooltip_pos)
            .frame(egui::Frame {
                fill: egui::Color32::from_rgba_unmultiplied(0, 0, 0, 200),
                stroke: egui::Stroke::new(1.0, egui::Color32::from_rgb(100, 100, 100)),
                inner_margin: egui::Margin::same(8.0),
                outer_margin: egui::Margin::same(0.0),
                rounding: egui::Rounding::same(4.0),
                shadow: egui::epaint::Shadow::NONE,
            })
            .show(ctx, |ui| {
                ui.label(egui::RichText::new(&self.material_name)
                    .color(egui::Color32::WHITE)
                    .strong());
                ui.label(egui::RichText::new(format!("Temp: {:.0}°C", self.temperature))
                    .color(egui::Color32::LIGHT_GRAY)
                    .size(12.0));
                ui.label(egui::RichText::new(format!("Pos: ({}, {})", self.world_pos.0, self.world_pos.1))
                    .color(egui::Color32::DARK_GRAY)
                    .size(11.0));

                // Show light level guide when overlay is active
                if self.light_overlay_active {
                    ui.separator();
                    ui.label(egui::RichText::new("Light Level Guide:")
                        .color(egui::Color32::WHITE)
                        .strong()
                        .size(11.0));

                    ui.horizontal(|ui| {
                        ui.colored_label(egui::Color32::from_rgb(0, 0, 0), "■");
                        ui.label(egui::RichText::new("0: Dark").size(10.0));
                    });
                    ui.horizontal(|ui| {
                        ui.colored_label(egui::Color32::from_rgb(51, 0, 102), "■");
                        ui.label(egui::RichText::new("1-3: Very Dim").size(10.0));
                    });
                    ui.horizontal(|ui| {
                        ui.colored_label(egui::Color32::from_rgb(0, 77, 204), "■");
                        ui.label(egui::RichText::new("4-7: Dim").size(10.0));
                    });
                    ui.horizontal(|ui| {
                        ui.colored_label(egui::Color32::from_rgb(0, 204, 255), "■");
                        ui.label(egui::RichText::new("8-11: Moderate (growth)").size(10.0));
                    });
                    ui.horizontal(|ui| {
                        ui.colored_label(egui::Color32::WHITE, "■");
                        ui.label(egui::RichText::new("12-15: Bright").size(10.0));
                    });
                }

                // Show simplified growth status for Plant Matter
                if self.material_name == "plant_matter" && !self.growth_status.is_empty() {
                    ui.separator();

                    // Single status message (color-coded)
                    let status_color = if self.growth_status.starts_with("Cannot") {
                        egui::Color32::from_rgb(200, 200, 0) // Yellow warning
                    } else {
                        egui::Color32::GREEN // Growing
                    };

                    ui.label(egui::RichText::new(&self.growth_status)
                        .color(status_color)
                        .size(12.0)
                        .strong());
                }
            });
    }
}

impl Default for TooltipState {
    fn default() -> Self {
        Self::new()
    }
}
