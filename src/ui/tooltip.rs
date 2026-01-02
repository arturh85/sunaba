//! Mouseover tooltip showing pixel information

use crate::entity::EntityId;
use crate::simulation::Materials;
use crate::world::World;
use glam::Vec2;

/// Tooltip state for displaying pixel information at cursor
pub struct TooltipState {
    visible: bool,
    material_name: String,
    temperature: f32,
    world_pos: (i32, i32),
    // Growth-related data (simplified)
    light_level: u8,
    has_nearby_water: bool,
    growth_status: String, // Single simplified message
    // Light overlay state
    light_overlay_active: bool, // Show light level guide when true
    // Mining time estimate
    mining_time: Option<f32>, // None if can't mine, Some(time) otherwise
    mining_tool: String,      // Tool name being used

    // Creature tooltip state
    creature_visible: bool,
    creature_id: Option<EntityId>,
    creature_health: (f32, f32), // (current, max)
    creature_hunger: (f32, f32), // (current, max)
    creature_action: String,
    creature_body_parts: usize,
    creature_joints: usize,
    creature_generation: u64,
    creature_grounded: bool,
    creature_velocity: Vec2,
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
            mining_time: None,
            mining_tool: String::new(),
            // Creature tooltip fields
            creature_visible: false,
            creature_id: None,
            creature_health: (0.0, 0.0),
            creature_hunger: (0.0, 0.0),
            creature_action: String::new(),
            creature_body_parts: 0,
            creature_joints: 0,
            creature_generation: 0,
            creature_grounded: false,
            creature_velocity: Vec2::ZERO,
        }
    }

    /// Update tooltip with information from world at mouse position
    pub fn update(
        &mut self,
        world: &World,
        materials: &Materials,
        mouse_world_pos: Option<(i32, i32)>,
        light_overlay_active: bool,
    ) {
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
                    self.mining_time = None;
                    self.mining_tool = String::new();
                } else {
                    self.visible = true;
                    let material = materials.get(pixel.material_id);
                    self.material_name = material.name.clone();

                    // Get temperature at this pixel
                    self.temperature = world.get_temperature_at_pixel(wx, wy);

                    // Get light level
                    self.light_level = world.get_light_at(wx, wy).unwrap_or(0);

                    // Calculate mining time
                    use crate::simulation::mining::calculate_mining_time;
                    if material.hardness.is_some() {
                        let tool = world.player.get_equipped_tool(world.tool_registry());
                        self.mining_time = Some(calculate_mining_time(1.0, material, tool));
                        self.mining_tool = if let Some(t) = tool {
                            t.name.clone()
                        } else {
                            "Hands".to_string()
                        };
                    } else {
                        self.mining_time = None;
                        self.mining_tool = String::new();
                    }

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
                            self.growth_status =
                                format!("Growing: {:.0}% ({:.1}s)", progress, seconds);
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
                inner_margin: egui::Margin::same(8),
                outer_margin: egui::Margin::same(0),
                corner_radius: egui::CornerRadius::same(4),
                shadow: egui::epaint::Shadow::NONE,
            })
            .show(ctx, |ui| {
                ui.label(
                    egui::RichText::new(&self.material_name)
                        .color(egui::Color32::WHITE)
                        .strong(),
                );
                ui.label(
                    egui::RichText::new(format!("Temp: {:.0}°C", self.temperature))
                        .color(egui::Color32::LIGHT_GRAY)
                        .size(12.0),
                );
                ui.label(
                    egui::RichText::new(format!(
                        "Pos: ({}, {})",
                        self.world_pos.0, self.world_pos.1
                    ))
                    .color(egui::Color32::DARK_GRAY)
                    .size(11.0),
                );

                // Show mining time estimate
                if let Some(time) = self.mining_time {
                    ui.label(
                        egui::RichText::new(format!("Mine: {:.1}s ({})", time, self.mining_tool))
                            .color(egui::Color32::from_rgb(150, 200, 255))
                            .size(12.0),
                    );
                }

                // Show light level guide when overlay is active
                if self.light_overlay_active {
                    ui.separator();
                    ui.label(
                        egui::RichText::new("Light Level Guide:")
                            .color(egui::Color32::WHITE)
                            .strong()
                            .size(11.0),
                    );

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

                    ui.label(
                        egui::RichText::new(&self.growth_status)
                            .color(status_color)
                            .size(12.0)
                            .strong(),
                    );
                }
            });
    }

    /// Update creature tooltip with information from world at mouse position
    pub fn update_creature(&mut self, world: &World, mouse_pos: Option<(i32, i32)>) {
        self.creature_visible = false;

        let Some((mx, my)) = mouse_pos else {
            return;
        };
        let pos = Vec2::new(mx as f32, my as f32);

        // Find creature within 15 pixel radius of mouse
        let Some(creature) = world.creature_manager.get_creature_at_position(pos, 15.0) else {
            return;
        };

        self.creature_visible = true;
        self.creature_id = Some(creature.id);
        self.creature_health = (creature.health.current, creature.health.max);
        self.creature_hunger = (creature.hunger.current, creature.hunger.max);
        self.creature_action = creature
            .current_action
            .as_ref()
            .map(|a| a.to_string())
            .unwrap_or_else(|| "Idle".to_string());
        self.creature_body_parts = creature.morphology.body_parts.len();
        self.creature_joints = creature.morphology.joints.len();
        self.creature_generation = creature.generation;
        self.creature_grounded = creature.grounded;
        self.creature_velocity = creature.velocity;
    }

    /// Render creature tooltip near cursor position
    pub fn render_creature(&self, ctx: &egui::Context, cursor_pos: Option<egui::Pos2>) {
        if !self.creature_visible {
            return;
        }

        let Some(cursor) = cursor_pos else {
            return;
        };
        let tooltip_pos = egui::pos2(cursor.x + 20.0, cursor.y + 20.0);

        egui::Window::new("##creature_tooltip")
            .title_bar(false)
            .resizable(false)
            .fixed_pos(tooltip_pos)
            .frame(egui::Frame {
                fill: egui::Color32::from_rgba_unmultiplied(0, 0, 0, 220),
                stroke: egui::Stroke::new(1.0, egui::Color32::from_rgb(200, 50, 200)),
                inner_margin: egui::Margin::same(8),
                outer_margin: egui::Margin::same(0),
                corner_radius: egui::CornerRadius::same(4),
                shadow: egui::epaint::Shadow::NONE,
            })
            .show(ctx, |ui| {
                // Header with ID
                if let Some(id) = self.creature_id {
                    ui.label(
                        egui::RichText::new(format!("Creature #{}", id))
                            .color(egui::Color32::from_rgb(200, 50, 200))
                            .strong(),
                    );
                }
                ui.label(
                    egui::RichText::new(format!("Generation: {}", self.creature_generation))
                        .color(egui::Color32::LIGHT_GRAY)
                        .size(11.0),
                );
                ui.separator();

                // Current action
                ui.label(
                    egui::RichText::new(format!("Status: {}", self.creature_action))
                        .color(egui::Color32::WHITE),
                );

                // Health bar with color coding
                let health_pct = if self.creature_health.1 > 0.0 {
                    self.creature_health.0 / self.creature_health.1
                } else {
                    0.0
                };
                let health_color = if health_pct > 0.5 {
                    egui::Color32::GREEN
                } else if health_pct > 0.25 {
                    egui::Color32::YELLOW
                } else {
                    egui::Color32::RED
                };
                ui.label(
                    egui::RichText::new(format!(
                        "Health: {:.0}/{:.0} ({:.0}%)",
                        self.creature_health.0,
                        self.creature_health.1,
                        health_pct * 100.0
                    ))
                    .color(health_color)
                    .size(12.0),
                );

                // Hunger
                let hunger_pct = if self.creature_hunger.1 > 0.0 {
                    self.creature_hunger.0 / self.creature_hunger.1
                } else {
                    0.0
                };
                let hunger_color = if hunger_pct > 0.3 {
                    egui::Color32::from_rgb(255, 200, 100)
                } else {
                    egui::Color32::from_rgb(255, 100, 100)
                };
                ui.label(
                    egui::RichText::new(format!(
                        "Hunger: {:.0}/{:.0} ({:.0}%)",
                        self.creature_hunger.0,
                        self.creature_hunger.1,
                        hunger_pct * 100.0
                    ))
                    .color(hunger_color)
                    .size(12.0),
                );

                ui.separator();

                // Morphology
                ui.label(
                    egui::RichText::new(format!(
                        "Body: {} parts, {} joints",
                        self.creature_body_parts, self.creature_joints
                    ))
                    .color(egui::Color32::LIGHT_GRAY)
                    .size(11.0),
                );

                // Movement state
                let state = if self.creature_grounded {
                    "Grounded"
                } else {
                    "Airborne"
                };
                let speed = self.creature_velocity.length();
                ui.label(
                    egui::RichText::new(format!("{} | Speed: {:.1}", state, speed))
                        .color(egui::Color32::DARK_GRAY)
                        .size(11.0),
                );
            });
    }
}

impl Default for TooltipState {
    fn default() -> Self {
        Self::new()
    }
}
