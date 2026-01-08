//! Tunable parameters panel for real-time adjustment of game settings

use crate::config::GameConfig;

/// Panel for adjusting tunable game parameters in real-time
pub struct ParamsPanel {
    /// Whether the panel is open
    pub open: bool,
    /// Local copy of config for editing (synced back on change)
    config: GameConfig,
    /// Track if any value changed this frame
    changed: bool,
}

impl ParamsPanel {
    pub fn new(config: &GameConfig) -> Self {
        Self {
            open: false,
            config: config.clone(),
            changed: false,
        }
    }

    /// Toggle panel visibility
    pub fn toggle(&mut self) {
        self.open = !self.open;
    }

    /// Update the local config from external source
    pub fn update_config(&mut self, config: &GameConfig) {
        self.config = config.clone();
    }

    /// Get current config values (for applying to game systems)
    pub fn config(&self) -> &GameConfig {
        &self.config
    }

    /// Check if any values changed this frame
    pub fn take_changed(&mut self) -> bool {
        let changed = self.changed;
        self.changed = false;
        changed
    }

    /// Render the parameters panel
    pub fn render(&mut self, ctx: &egui::Context) {
        if !self.open {
            return;
        }

        egui::Window::new("Parameters")
            .default_pos(egui::pos2(400.0, 10.0))
            .default_width(300.0)
            .resizable(true)
            .collapsible(true)
            .show(ctx, |ui| {
                self.render_contents(ui);
            });
    }

    fn render_contents(&mut self, ui: &mut egui::Ui) {
        egui::ScrollArea::vertical().show(ui, |ui| {
            // Player Physics
            ui.collapsing("Player Physics", |ui| {
                self.changed |= ui
                    .add(
                        egui::Slider::new(&mut self.config.player.gravity, 200.0..=1600.0)
                            .text("Gravity"),
                    )
                    .changed();
                self.changed |= ui
                    .add(
                        egui::Slider::new(&mut self.config.player.jump_velocity, 100.0..=600.0)
                            .text("Jump Velocity"),
                    )
                    .changed();
                self.changed |= ui
                    .add(
                        egui::Slider::new(&mut self.config.player.flight_thrust, 400.0..=2400.0)
                            .text("Flight Thrust"),
                    )
                    .changed();
                self.changed |= ui
                    .add(
                        egui::Slider::new(&mut self.config.player.max_fall_speed, 200.0..=1000.0)
                            .text("Max Fall Speed"),
                    )
                    .changed();
                self.changed |= ui
                    .add(
                        egui::Slider::new(&mut self.config.player.move_speed, 100.0..=500.0)
                            .text("Move Speed"),
                    )
                    .changed();
            });

            ui.add_space(4.0);

            // World Simulation
            ui.collapsing("World Simulation", |ui| {
                self.changed |= ui
                    .add(
                        egui::Slider::new(&mut self.config.world.active_chunk_radius, 1..=5)
                            .text("Active Chunk Radius"),
                    )
                    .changed();
                if ui.button("Apply Chunk Radius").clicked() {
                    self.changed = true;
                }
                ui.label(format!(
                    "Active area: {}x{} chunks",
                    self.config.world.active_chunk_radius * 2 + 1,
                    self.config.world.active_chunk_radius * 2 + 1
                ));
            });

            ui.add_space(4.0);

            // Camera
            ui.collapsing("Camera", |ui| {
                self.changed |= ui
                    .add(
                        egui::Slider::new(&mut self.config.camera.zoom_speed, 1.01..=1.5)
                            .text("Zoom Speed"),
                    )
                    .changed();
                self.changed |= ui
                    .add(
                        egui::Slider::new(&mut self.config.camera.min_zoom, 0.0005..=0.005)
                            .text("Min Zoom")
                            .logarithmic(true),
                    )
                    .changed();
                self.changed |= ui
                    .add(
                        egui::Slider::new(&mut self.config.camera.max_zoom, 0.005..=0.05)
                            .text("Max Zoom")
                            .logarithmic(true),
                    )
                    .changed();
            });

            ui.add_space(4.0);

            // UI Theme
            ui.collapsing("UI Theme", |ui| {
                ui.label("Select theme (requires restart to fully apply):");

                let current_theme = &self.config.ui.theme;
                let mut new_theme = current_theme.clone();

                egui::ComboBox::from_label("Theme Variant")
                    .selected_text(match current_theme.as_str() {
                        "cozy_alchemist" => "Cozy Alchemist (Default)",
                        "dark_cavern" => "Dark Cavern",
                        "pixel_adventure" => "Pixel Adventure",
                        _ => "Unknown",
                    })
                    .show_ui(ui, |ui| {
                        ui.selectable_value(
                            &mut new_theme,
                            "cozy_alchemist".to_string(),
                            "Cozy Alchemist (Default)",
                        );
                        ui.selectable_value(
                            &mut new_theme,
                            "dark_cavern".to_string(),
                            "Dark Cavern",
                        );
                        ui.selectable_value(
                            &mut new_theme,
                            "pixel_adventure".to_string(),
                            "Pixel Adventure",
                        );
                    });

                if new_theme != *current_theme {
                    self.config.ui.theme = new_theme;
                    self.changed = true;
                }

                ui.add_space(8.0);
                ui.label("Theme descriptions:");
                ui.label("• Cozy Alchemist: Warm, inviting with smooth rounded corners");
                ui.label("• Dark Cavern: High-contrast underground mining aesthetic");
                ui.label("• Pixel Adventure: Retro NES/SNES-inspired pixelart theme");
            });

            ui.add_space(4.0);

            // Rendering / Post-Processing
            ui.collapsing("Rendering", |ui| {
                self.changed |= ui
                    .add(
                        egui::Slider::new(&mut self.config.rendering.scanline_intensity, 0.0..=0.5)
                            .text("Scanlines"),
                    )
                    .changed();
                self.changed |= ui
                    .add(
                        egui::Slider::new(&mut self.config.rendering.vignette_intensity, 0.0..=0.5)
                            .text("Vignette"),
                    )
                    .changed();
                self.changed |= ui
                    .add(
                        egui::Slider::new(&mut self.config.rendering.bloom_intensity, 0.0..=1.0)
                            .text("Bloom"),
                    )
                    .changed();

                ui.separator();
                ui.label("Water Animation:");
                self.changed |= ui
                    .add(
                        egui::Slider::new(
                            &mut self.config.rendering.water_noise_frequency,
                            0.01..=0.2,
                        )
                        .text("Frequency"),
                    )
                    .changed();
                self.changed |= ui
                    .add(
                        egui::Slider::new(&mut self.config.rendering.water_noise_speed, 0.5..=5.0)
                            .text("Speed"),
                    )
                    .changed();
                self.changed |= ui
                    .add(
                        egui::Slider::new(
                            &mut self.config.rendering.water_noise_amplitude,
                            0.0..=0.2,
                        )
                        .text("Amplitude"),
                    )
                    .changed();

                ui.separator();
                ui.label("Lava Animation:");
                self.changed |= ui
                    .add(
                        egui::Slider::new(
                            &mut self.config.rendering.lava_noise_frequency,
                            0.01..=0.15,
                        )
                        .text("Frequency"),
                    )
                    .changed();
                self.changed |= ui
                    .add(
                        egui::Slider::new(&mut self.config.rendering.lava_noise_speed, 0.5..=3.0)
                            .text("Speed"),
                    )
                    .changed();
                self.changed |= ui
                    .add(
                        egui::Slider::new(
                            &mut self.config.rendering.lava_noise_amplitude,
                            0.0..=0.3,
                        )
                        .text("Amplitude"),
                    )
                    .changed();

                ui.separator();
                ui.label("Multi-Pass Bloom:");
                self.changed |= ui
                    .checkbox(&mut self.config.rendering.bloom_enabled, "Enable Bloom")
                    .changed();
                if self.config.rendering.bloom_enabled {
                    self.changed |= ui
                        .add(
                            egui::Slider::new(&mut self.config.rendering.bloom_quality, 3..=5)
                                .text("Quality")
                                .custom_formatter(|n, _| {
                                    match n as u32 {
                                        3 => "Low (3 mips)",
                                        4 => "Medium (4 mips)",
                                        5 => "High (5 mips)",
                                        _ => "Unknown",
                                    }
                                    .to_string()
                                }),
                        )
                        .changed();
                    self.changed |= ui
                        .add(
                            egui::Slider::new(
                                &mut self.config.rendering.bloom_threshold,
                                0.4..=0.8,
                            )
                            .text("Threshold"),
                        )
                        .changed();
                }
            });

            ui.add_space(4.0);

            // Debug
            ui.collapsing("Debug", |ui| {
                self.changed |= ui
                    .checkbox(&mut self.config.debug.debug_placement, "Debug Placement")
                    .changed();
                self.changed |= ui
                    .checkbox(&mut self.config.debug.verbose_logging, "Verbose Logging")
                    .changed();
                self.changed |= ui
                    .add(
                        egui::Slider::new(&mut self.config.debug.brush_size, 1..=10)
                            .text("Brush Size"),
                    )
                    .on_hover_text(
                        "Circular brush radius for material placement (1 = 3x3, 2 = 5x5, etc.)",
                    )
                    .changed();
            });

            ui.add_space(8.0);

            // Save and Reset buttons
            ui.horizontal(|ui| {
                if ui.button("Save Config").clicked()
                    && let Err(e) = self.config.save()
                {
                    log::error!("Failed to save config: {}", e);
                }
                if ui.button("Reset to Defaults").clicked() {
                    self.config = GameConfig::default();
                    self.changed = true;
                }
            });
        });
    }
}

impl Default for ParamsPanel {
    fn default() -> Self {
        Self::new(&GameConfig::default())
    }
}
