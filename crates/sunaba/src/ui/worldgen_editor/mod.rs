//! World Generation Editor - parameter-based editor with live preview
//!
//! Provides a UI for editing world generation parameters with:
//! - Tab-based parameter editing (terrain, caves, biomes, ores, vegetation)
//! - Live preview of generated terrain
//! - Preset save/load system
//! - Seed management for reproducible worlds

mod noise_widget;
mod preview;

use crate::simulation::Materials;
use sunaba_core::world::WorldGenConfig;

pub use noise_widget::noise_layer_editor;
pub use preview::PreviewState;

/// Active tab in the editor
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EditorTab {
    #[default]
    World,
    Terrain,
    Caves,
    Biomes,
    Ores,
    Vegetation,
    Features,
    Presets,
}

impl EditorTab {
    pub fn label(&self) -> &'static str {
        match self {
            EditorTab::World => "World",
            EditorTab::Terrain => "Terrain",
            EditorTab::Caves => "Caves",
            EditorTab::Biomes => "Biomes",
            EditorTab::Ores => "Ores",
            EditorTab::Vegetation => "Vegetation",
            EditorTab::Features => "Features",
            EditorTab::Presets => "Presets",
        }
    }

    pub fn all() -> &'static [EditorTab] {
        &[
            EditorTab::World,
            EditorTab::Terrain,
            EditorTab::Caves,
            EditorTab::Biomes,
            EditorTab::Ores,
            EditorTab::Vegetation,
            EditorTab::Features,
            EditorTab::Presets,
        ]
    }
}

/// World Generation Editor state
pub struct WorldGenEditor {
    /// Whether the editor window is open
    pub open: bool,

    /// Working copy of configuration (edited in UI)
    pub config: WorldGenConfig,

    /// Original config for reset
    original_config: WorldGenConfig,

    /// Preview seed (separate from world seed)
    pub preview_seed: u64,

    /// Which tab is selected
    pub active_tab: EditorTab,

    /// Live preview state
    pub preview: PreviewState,

    /// Track if config changed (triggers preview regeneration)
    config_dirty: bool,

    /// Throttle preview updates (time since last update)
    #[cfg(not(target_arch = "wasm32"))]
    last_preview_update: std::time::Instant,

    /// Request to apply config to world
    pub apply_requested: bool,
}

impl WorldGenEditor {
    pub fn new() -> Self {
        let config = WorldGenConfig::default();
        Self {
            open: false,
            config: config.clone(),
            original_config: config,
            preview_seed: 42,
            active_tab: EditorTab::default(),
            preview: PreviewState::new(),
            config_dirty: true,
            #[cfg(not(target_arch = "wasm32"))]
            last_preview_update: std::time::Instant::now(),
            apply_requested: false,
        }
    }

    /// Toggle editor visibility
    pub fn toggle(&mut self) {
        self.open = !self.open;
    }

    /// Mark config as changed (will trigger preview update)
    fn mark_dirty(&mut self) {
        self.config_dirty = true;
    }

    /// Render the editor window
    pub fn render(&mut self, ctx: &egui::Context, materials: &Materials) {
        if !self.open {
            return;
        }

        egui::Window::new("World Generation Editor")
            .default_pos(egui::pos2(100.0, 100.0))
            .default_size(egui::vec2(800.0, 600.0))
            .resizable(true)
            .collapsible(true)
            .show(ctx, |ui| {
                self.render_contents(ui, materials);
            });
    }

    fn render_contents(&mut self, ui: &mut egui::Ui, materials: &Materials) {
        // Top bar with seed and apply button
        ui.horizontal(|ui| {
            ui.label("Seed:");
            if ui
                .add(egui::DragValue::new(&mut self.preview_seed).speed(1.0))
                .changed()
            {
                self.mark_dirty();
            }
            if ui.button("🎲 Random").clicked() {
                self.preview_seed = rand::random();
                self.mark_dirty();
            }
            ui.separator();
            if ui.button("Apply to World").clicked() {
                self.apply_requested = true;
            }
            if ui.button("Reset").clicked() {
                self.config = self.original_config.clone();
                self.mark_dirty();
            }
        });

        ui.separator();

        // Main content: tabs on left, preview on right
        ui.horizontal(|ui| {
            // Left panel: tabs and parameters
            ui.vertical(|ui| {
                ui.set_min_width(350.0);
                ui.set_max_width(400.0);

                // Tab bar
                ui.horizontal(|ui| {
                    for tab in EditorTab::all() {
                        if ui
                            .selectable_label(self.active_tab == *tab, tab.label())
                            .clicked()
                        {
                            self.active_tab = *tab;
                        }
                    }
                });

                ui.separator();

                // Tab content in scroll area
                egui::ScrollArea::vertical()
                    .max_height(450.0)
                    .show(ui, |ui| {
                        let changed = match self.active_tab {
                            EditorTab::World => self.render_world_tab(ui),
                            EditorTab::Terrain => self.render_terrain_tab(ui),
                            EditorTab::Caves => self.render_caves_tab(ui),
                            EditorTab::Biomes => self.render_biomes_tab(ui),
                            EditorTab::Ores => self.render_ores_tab(ui),
                            EditorTab::Vegetation => self.render_vegetation_tab(ui),
                            EditorTab::Features => self.render_features_tab(ui),
                            EditorTab::Presets => self.render_presets_tab(ui),
                        };

                        if changed {
                            self.mark_dirty();
                        }
                    });
            });

            ui.separator();

            // Right panel: preview
            ui.vertical(|ui| {
                ui.heading("Preview");

                // Update preview if dirty (throttled)
                #[cfg(not(target_arch = "wasm32"))]
                {
                    if self.config_dirty && self.last_preview_update.elapsed().as_millis() > 100 {
                        self.preview
                            .update(&self.config, self.preview_seed, materials);
                        self.config_dirty = false;
                        self.last_preview_update = std::time::Instant::now();
                    }
                }

                #[cfg(target_arch = "wasm32")]
                {
                    if self.config_dirty {
                        self.preview
                            .update(&self.config, self.preview_seed, materials);
                        self.config_dirty = false;
                    }
                }

                // Render preview
                self.preview.render(ui, materials);
            });
        });
    }

    // ========================================================================
    // Tab renderers
    // ========================================================================

    fn render_world_tab(&mut self, ui: &mut egui::Ui) -> bool {
        let mut changed = false;

        ui.heading("World Boundaries");

        changed |= ui
            .add(
                egui::Slider::new(&mut self.config.world.surface_y, -100..=100)
                    .text("Surface Y (sea level)"),
            )
            .changed();

        changed |= ui
            .add(
                egui::Slider::new(&mut self.config.world.sky_height, 500..=2000).text("Sky Height"),
            )
            .changed();

        changed |= ui
            .add(
                egui::Slider::new(&mut self.config.world.bedrock_y, -5000..=-1000)
                    .text("Bedrock Y"),
            )
            .changed();

        ui.add_space(8.0);
        ui.heading("Underground Layers");

        changed |= ui
            .add(
                egui::Slider::new(
                    &mut self.config.world.underground_layers.shallow,
                    -1000..=-100,
                )
                .text("Shallow Layer"),
            )
            .changed();

        changed |= ui
            .add(
                egui::Slider::new(&mut self.config.world.underground_layers.deep, -2500..=-500)
                    .text("Deep Layer"),
            )
            .changed();

        changed |= ui
            .add(
                egui::Slider::new(
                    &mut self.config.world.underground_layers.cavern,
                    -4000..=-1500,
                )
                .text("Cavern Layer"),
            )
            .changed();

        changed
    }

    fn render_terrain_tab(&mut self, ui: &mut egui::Ui) -> bool {
        let mut changed = false;

        ui.heading("Terrain Height");

        changed |= ui
            .add(
                egui::Slider::new(&mut self.config.terrain.height_scale, 10.0..=300.0)
                    .text("Height Scale"),
            )
            .on_hover_text("Amplitude of terrain height variation")
            .changed();

        ui.add_space(8.0);
        ui.heading("Height Noise");
        changed |= noise_layer_editor(ui, &mut self.config.terrain.height_noise, "height");

        changed
    }

    fn render_caves_tab(&mut self, ui: &mut egui::Ui) -> bool {
        let mut changed = false;

        ui.heading("Cave Generation");

        changed |= ui
            .add(
                egui::Slider::new(&mut self.config.caves.min_cave_depth, 5..=50)
                    .text("Min Depth Below Surface"),
            )
            .changed();

        ui.add_space(8.0);
        ui.heading("Large Caverns");

        changed |= ui
            .add(
                egui::Slider::new(&mut self.config.caves.large_threshold, 0.05..=0.4)
                    .text("Threshold (lower = more)"),
            )
            .changed();

        changed |= noise_layer_editor(ui, &mut self.config.caves.large_caves, "large_caves");

        ui.add_space(8.0);
        ui.heading("Tunnels");

        changed |= ui
            .add(
                egui::Slider::new(&mut self.config.caves.tunnel_threshold, 0.1..=0.5)
                    .text("Threshold (lower = more)"),
            )
            .changed();

        changed |= noise_layer_editor(ui, &mut self.config.caves.tunnels, "tunnels");

        changed
    }

    fn render_biomes_tab(&mut self, ui: &mut egui::Ui) -> bool {
        let mut changed = false;

        ui.heading("Biome Selection Noise");

        ui.label("Temperature Noise:");
        changed |= noise_layer_editor(ui, &mut self.config.biomes.temperature_noise, "temperature");

        ui.add_space(4.0);
        ui.label("Moisture Noise:");
        changed |= noise_layer_editor(ui, &mut self.config.biomes.moisture_noise, "moisture");

        ui.add_space(8.0);
        ui.heading("Biomes");

        for biome in self.config.biomes.biomes.iter_mut() {
            ui.collapsing(&biome.name, |ui| {
                changed |= ui
                    .add(
                        egui::Slider::new(&mut biome.height_variance, 0.0..=2.0)
                            .text("Height Variance"),
                    )
                    .changed();

                changed |= ui
                    .add(
                        egui::Slider::new(&mut biome.height_offset, -50..=50).text("Height Offset"),
                    )
                    .changed();

                changed |= ui
                    .add(egui::Slider::new(&mut biome.stone_depth, 5..=50).text("Stone Depth"))
                    .changed();

                changed |= ui
                    .add(egui::Slider::new(&mut biome.tree_density, 0.0..=0.3).text("Tree Density"))
                    .changed();

                changed |= ui
                    .add(
                        egui::Slider::new(&mut biome.plant_density, 0.0..=0.5)
                            .text("Plant Density"),
                    )
                    .changed();

                changed |= ui
                    .add(
                        egui::Slider::new(&mut biome.cave_density_multiplier, 0.2..=2.0)
                            .text("Cave Density"),
                    )
                    .changed();
            });
        }

        changed
    }

    fn render_ores_tab(&mut self, ui: &mut egui::Ui) -> bool {
        let mut changed = false;

        ui.heading("Ore Generation");

        for ore in self.config.ores.iter_mut() {
            ui.collapsing(&ore.name, |ui| {
                changed |= ui
                    .add(
                        egui::Slider::new(&mut ore.threshold, 0.5..=0.95)
                            .text("Threshold (higher = rarer)"),
                    )
                    .changed();

                changed |= ui
                    .add(egui::Slider::new(&mut ore.min_depth, -4000..=0).text("Min Depth"))
                    .changed();

                changed |= ui
                    .add(egui::Slider::new(&mut ore.max_depth, -4000..=0).text("Max Depth"))
                    .changed();

                changed |= ui
                    .add(egui::Slider::new(&mut ore.noise_scale, 0.02..=0.2).text("Noise Scale"))
                    .changed();
            });
        }

        changed
    }

    fn render_vegetation_tab(&mut self, ui: &mut egui::Ui) -> bool {
        let mut changed = false;

        ui.heading("Vegetation");

        changed |= ui
            .add(
                egui::Slider::new(&mut self.config.vegetation.tree_noise_scale, 0.01..=0.1)
                    .text("Tree Noise Scale"),
            )
            .changed();

        changed |= ui
            .add(
                egui::Slider::new(&mut self.config.vegetation.plant_noise_scale, 0.01..=0.15)
                    .text("Plant Noise Scale"),
            )
            .changed();

        ui.add_space(8.0);
        ui.label("Tree Noise:");
        changed |= noise_layer_editor(ui, &mut self.config.vegetation.tree_noise, "tree");

        ui.add_space(4.0);
        ui.label("Plant Noise:");
        changed |= noise_layer_editor(ui, &mut self.config.vegetation.plant_noise, "plant");

        changed
    }

    fn render_features_tab(&mut self, ui: &mut egui::Ui) -> bool {
        let mut changed = false;

        ui.heading("Special Features");

        ui.collapsing("Lava Pools", |ui| {
            changed |= ui
                .checkbox(&mut self.config.features.lava_pools.enabled, "Enable Lava")
                .changed();

            if self.config.features.lava_pools.enabled {
                changed |= ui
                    .add(
                        egui::Slider::new(
                            &mut self.config.features.lava_pools.min_depth,
                            -4000..=-1000,
                        )
                        .text("Min Depth"),
                    )
                    .changed();

                changed |= ui
                    .add(
                        egui::Slider::new(
                            &mut self.config.features.lava_pools.threshold,
                            0.3..=0.9,
                        )
                        .text("Threshold (higher = less)"),
                    )
                    .changed();

                changed |= ui
                    .add(
                        egui::Slider::new(
                            &mut self.config.features.lava_pools.noise_scale,
                            0.01..=0.1,
                        )
                        .text("Noise Scale"),
                    )
                    .changed();
            }
        });

        changed
    }

    fn render_presets_tab(&mut self, ui: &mut egui::Ui) -> bool {
        let mut changed = false;

        ui.heading("Presets");

        ui.horizontal(|ui| {
            if ui.button("Default").clicked() {
                self.config = WorldGenConfig::default();
                changed = true;
            }
            if ui.button("Cave Heavy").clicked() {
                self.config = WorldGenConfig::preset_cave_heavy();
                changed = true;
            }
            if ui.button("Flat").clicked() {
                self.config = WorldGenConfig::preset_flat();
                changed = true;
            }
        });

        ui.horizontal(|ui| {
            if ui.button("Desert World").clicked() {
                self.config = WorldGenConfig::preset_desert_world();
                changed = true;
            }
            if ui.button("Mountain World").clicked() {
                self.config = WorldGenConfig::preset_mountain_world();
                changed = true;
            }
        });

        ui.add_space(16.0);
        ui.separator();

        ui.heading("Save/Load");
        ui.label("(Coming soon - save custom presets to disk)");

        // TODO: Implement preset save/load
        // - Text input for preset name
        // - Save button -> serialize to RON
        // - List of saved presets
        // - Load/Delete buttons per preset

        changed
    }
}

impl Default for WorldGenEditor {
    fn default() -> Self {
        Self::new()
    }
}
