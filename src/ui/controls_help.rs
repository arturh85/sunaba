//! On-screen controls help display

/// Controls help panel state
pub struct ControlsHelpState {
    visible: bool,
}

impl ControlsHelpState {
    pub fn new() -> Self {
        Self {
            visible: false, // Start hidden
        }
    }

    /// Toggle help visibility
    pub fn toggle(&mut self) {
        self.visible = !self.visible;
    }

    /// Render controls help panel
    pub fn render(
        &self,
        ctx: &egui::Context,
        selected_material_id: u16,
        materials: &crate::simulation::Materials,
    ) {
        if !self.visible {
            return;
        }

        // Position on right side of screen
        let screen_rect = ctx.content_rect();
        let panel_width = 280.0;
        let panel_x = screen_rect.max.x - panel_width - 20.0;

        egui::Window::new("Controls")
            .fixed_pos(egui::pos2(panel_x, 20.0))
            .fixed_size(egui::vec2(panel_width, 0.0))
            .resizable(false)
            .collapsible(true)
            .show(ctx, |ui| {
                ui.heading("Movement");
                ui.label("W/A/S/D - Move player");
                ui.label("Space - Jump");

                ui.add_space(8.0);
                ui.heading("Camera");
                ui.label("+/- - Zoom in/out");
                ui.label("Mouse Wheel - Zoom in/out");

                ui.add_space(8.0);
                ui.heading("Materials");

                // Get selected material name for highlighting
                let selected_material = materials.get(selected_material_id);

                // List all materials 0-9
                let material_ids = [
                    (0, crate::simulation::MaterialId::AIR),
                    (1, crate::simulation::MaterialId::STONE),
                    (2, crate::simulation::MaterialId::SAND),
                    (3, crate::simulation::MaterialId::WATER),
                    (4, crate::simulation::MaterialId::WOOD),
                    (5, crate::simulation::MaterialId::FIRE),
                    (6, crate::simulation::MaterialId::SMOKE),
                    (7, crate::simulation::MaterialId::STEAM),
                    (8, crate::simulation::MaterialId::LAVA),
                    (9, crate::simulation::MaterialId::OIL),
                ];

                for (key, mat_id) in material_ids {
                    let material = materials.get(mat_id);
                    let text = format!("{} - {}", key, material.name);

                    if mat_id == selected_material_id {
                        // Highlight selected material
                        ui.label(
                            egui::RichText::new(text)
                                .color(egui::Color32::YELLOW)
                                .strong(),
                        );
                    } else {
                        ui.label(text);
                    }
                }

                ui.add_space(8.0);
                ui.heading("Actions");
                ui.label("Left Click - Spawn material");
                ui.label("G - Spawn creature");

                ui.add_space(8.0);
                ui.heading("UI");
                ui.label("T - Toggle temperature overlay");
                ui.label("F1 - Toggle stats");
                ui.label("F2 - Toggle active chunks");
                ui.label("H - Toggle this help");

                ui.add_space(8.0);
                ui.heading("World");
                ui.label("L - Level selector");
                ui.label("F5 - Save world");

                ui.add_space(8.0);
                ui.separator();
                ui.label(
                    egui::RichText::new(format!("Selected: {}", selected_material.name))
                        .color(egui::Color32::LIGHT_BLUE)
                        .strong(),
                );
            });
    }

    /// Render controls help panel with level name
    pub fn render_with_level(
        &self,
        ctx: &egui::Context,
        selected_material_id: u16,
        materials: &crate::simulation::Materials,
        level_name: &str,
    ) {
        if !self.visible {
            return;
        }

        // Position on right side of screen
        let screen_rect = ctx.content_rect();
        let panel_width = 280.0;
        let panel_x = screen_rect.max.x - panel_width - 20.0;

        egui::Window::new("Controls")
            .fixed_pos(egui::pos2(panel_x, 20.0))
            .fixed_size(egui::vec2(panel_width, 0.0))
            .resizable(false)
            .collapsible(true)
            .show(ctx, |ui| {
                ui.heading("Movement");
                ui.label("W/A/S/D - Move player");
                ui.label("Space - Jump");

                ui.add_space(8.0);
                ui.heading("Camera");
                ui.label("+/- - Zoom in/out");
                ui.label("Mouse Wheel - Zoom in/out");

                ui.add_space(8.0);
                ui.heading("Materials");

                // Get selected material name for highlighting
                let selected_material = materials.get(selected_material_id);

                // List all materials 0-9
                let material_ids = [
                    (0, crate::simulation::MaterialId::AIR),
                    (1, crate::simulation::MaterialId::STONE),
                    (2, crate::simulation::MaterialId::SAND),
                    (3, crate::simulation::MaterialId::WATER),
                    (4, crate::simulation::MaterialId::WOOD),
                    (5, crate::simulation::MaterialId::FIRE),
                    (6, crate::simulation::MaterialId::SMOKE),
                    (7, crate::simulation::MaterialId::STEAM),
                    (8, crate::simulation::MaterialId::LAVA),
                    (9, crate::simulation::MaterialId::OIL),
                ];

                for (key, mat_id) in material_ids {
                    let material = materials.get(mat_id);
                    let text = format!("{} - {}", key, material.name);

                    if mat_id == selected_material_id {
                        // Highlight selected material
                        ui.label(
                            egui::RichText::new(text)
                                .color(egui::Color32::YELLOW)
                                .strong(),
                        );
                    } else {
                        ui.label(text);
                    }
                }

                ui.add_space(8.0);
                ui.heading("Actions");
                ui.label("Left Click - Spawn material");
                ui.label("G - Spawn creature");

                ui.add_space(8.0);
                ui.heading("UI");
                ui.label("T - Toggle temperature overlay");
                ui.label("F1 - Toggle stats");
                ui.label("F2 - Toggle active chunks");
                ui.label("H - Toggle this help");

                ui.add_space(8.0);
                ui.heading("World");
                ui.label("L - Level selector");
                ui.label("F5 - Save world");

                ui.add_space(8.0);
                ui.separator();
                ui.label(
                    egui::RichText::new(format!("Selected: {}", selected_material.name))
                        .color(egui::Color32::LIGHT_BLUE)
                        .strong(),
                );
                ui.label(
                    egui::RichText::new(format!("Mode: {}", level_name))
                        .color(egui::Color32::LIGHT_GREEN)
                        .strong(),
                );
            });
    }
}

impl Default for ControlsHelpState {
    fn default() -> Self {
        Self::new()
    }
}
