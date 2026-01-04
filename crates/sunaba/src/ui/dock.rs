//! Dockable panel system using egui_dock

use egui_dock::{DockArea, DockState, Style, TabViewer};

/// Identifiers for dockable panels
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DockTab {
    Stats,
    Controls,
    LevelSelector,
    Inventory,
    Crafting,
    #[cfg(not(target_arch = "wasm32"))]
    Logger,
    #[cfg(feature = "multiplayer")]
    MultiplayerStats,
    #[cfg(not(target_arch = "wasm32"))]
    Parameters,
    #[cfg(feature = "profiling")]
    Profiler,
}

impl std::fmt::Display for DockTab {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DockTab::Stats => write!(f, "Debug Stats"),
            DockTab::Controls => write!(f, "Controls"),
            DockTab::LevelSelector => write!(f, "Levels"),
            DockTab::Inventory => write!(f, "Inventory"),
            DockTab::Crafting => write!(f, "Crafting"),
            #[cfg(not(target_arch = "wasm32"))]
            DockTab::Logger => write!(f, "Log"),
            #[cfg(feature = "multiplayer")]
            DockTab::MultiplayerStats => write!(f, "Multiplayer"),
            #[cfg(not(target_arch = "wasm32"))]
            DockTab::Parameters => write!(f, "Parameters"),
            #[cfg(feature = "profiling")]
            DockTab::Profiler => write!(f, "Profiler"),
        }
    }
}

/// Dock state manager
pub struct DockManager {
    pub dock_state: DockState<DockTab>,
}

impl DockManager {
    #[cfg(not(target_arch = "wasm32"))]
    pub fn new() -> Self {
        // All tabs present from start, grouped together - Logger is the active tab
        let mut tabs = vec![DockTab::Logger, DockTab::Stats];

        #[cfg(feature = "multiplayer")]
        tabs.push(DockTab::MultiplayerStats);

        #[cfg(feature = "profiling")]
        tabs.push(DockTab::Profiler);

        tabs.push(DockTab::Parameters);

        let dock_state = DockState::new(tabs);

        Self { dock_state }
    }

    #[cfg(target_arch = "wasm32")]
    pub fn new() -> Self {
        // WASM: Stats tab present from start (but can be closed)
        #[cfg_attr(not(feature = "multiplayer"), allow(unused_mut))]
        let mut tabs = vec![DockTab::Stats];

        #[cfg(feature = "multiplayer")]
        tabs.push(DockTab::MultiplayerStats);

        let dock_state = DockState::new(tabs);
        Self { dock_state }
    }

    /// Check if a tab is currently open in the dock
    pub fn is_tab_open(&self, tab: DockTab) -> bool {
        self.dock_state.find_tab(&tab).is_some()
    }

    /// Toggle a specific tab open/closed
    pub fn toggle_tab(&mut self, tab: DockTab) {
        if let Some((surface, node, tab_index)) = self.dock_state.find_tab(&tab) {
            self.dock_state.remove_tab((surface, node, tab_index));
        } else {
            // Add to main surface root
            self.dock_state.push_to_first_leaf(tab);
        }
    }

    /// Add a tab if not already present
    pub fn open_tab(&mut self, tab: DockTab) {
        if !self.is_tab_open(tab) {
            self.dock_state.push_to_first_leaf(tab);
        }
    }

    /// Remove a tab if present
    pub fn close_tab(&mut self, tab: DockTab) {
        if let Some((surface, node, tab_index)) = self.dock_state.find_tab(&tab) {
            self.dock_state.remove_tab((surface, node, tab_index));
        }
    }
}

impl Default for DockManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Context needed for rendering dock tabs
pub struct DockContext<'a> {
    // Stats
    pub stats: &'a super::stats::SimulationStats,

    // Controls
    pub selected_material: u16,
    pub materials: &'a crate::simulation::Materials,
    pub game_mode_desc: &'a str,

    // Level selector
    pub level_manager: &'a crate::levels::LevelManager,
    pub in_persistent_world: bool,

    // Inventory
    pub player: &'a crate::entity::player::Player,
    pub tool_registry: &'a crate::entity::tools::ToolRegistry,

    // Crafting
    pub recipe_registry: &'a crate::entity::crafting::RecipeRegistry,

    // Parameters (native only)
    #[cfg(not(target_arch = "wasm32"))]
    pub params: &'a mut crate::config::GameConfig,
    #[cfg(not(target_arch = "wasm32"))]
    pub params_changed: &'a mut bool,

    // Multiplayer metrics (both native and WASM, when multiplayer feature enabled)
    #[cfg(feature = "multiplayer")]
    pub multiplayer_metrics: Option<&'a crate::multiplayer::metrics::MultiplayerMetrics>,

    // Multiplayer connection manager and panel state
    #[cfg(feature = "multiplayer")]
    pub multiplayer_manager: Option<&'a crate::multiplayer::MultiplayerManager>,
    #[cfg(feature = "multiplayer")]
    pub multiplayer_panel_state: &'a mut super::multiplayer_panel::MultiplayerPanelState,
}

/// Tab viewer implementation for dock
pub struct DockTabViewer<'a> {
    pub ctx: DockContext<'a>,
}

impl<'a> TabViewer for DockTabViewer<'a> {
    type Tab = DockTab;

    fn title(&mut self, tab: &mut Self::Tab) -> egui::WidgetText {
        tab.to_string().into()
    }

    fn ui(&mut self, ui: &mut egui::Ui, tab: &mut Self::Tab) {
        match tab {
            DockTab::Stats => self.render_stats(ui),
            DockTab::Controls => self.render_controls(ui),
            DockTab::LevelSelector => self.render_level_selector(ui),
            DockTab::Inventory => self.render_inventory(ui),
            DockTab::Crafting => self.render_crafting(ui),
            #[cfg(not(target_arch = "wasm32"))]
            DockTab::Logger => self.render_logger(ui),
            #[cfg(feature = "multiplayer")]
            DockTab::MultiplayerStats => self.render_multiplayer_stats(ui),
            #[cfg(not(target_arch = "wasm32"))]
            DockTab::Parameters => self.render_parameters(ui),
            #[cfg(feature = "profiling")]
            DockTab::Profiler => self.render_profiler(ui),
        }
    }

    fn closeable(&mut self, _tab: &mut Self::Tab) -> bool {
        true // All tabs can be closed
    }
}

impl<'a> DockTabViewer<'a> {
    fn render_stats(&self, ui: &mut egui::Ui) {
        let stats = self.ctx.stats;

        ui.heading("Performance");
        ui.label(format!("FPS: {:.1}", stats.fps));
        ui.label(format!("Frame: {:.2}ms", stats.frame_time_ms));
        ui.label(format!("  Sim: {:.2}ms", stats.sim_time_ms));
        ui.label(format!("  UI Build: {:.2}ms", stats.egui_build_time_ms));
        ui.label(format!("  Overlays: {:.2}ms", stats.overlay_time_ms));

        ui.separator();
        ui.heading("World");
        ui.label(format!("Active Chunks: {}", stats.active_chunks));
        ui.label(format!("Total Chunks: {}", stats.total_chunks));
        ui.label(format!("Active Pixels: {}", stats.active_pixels));

        ui.separator();
        ui.heading("Temperature");
        ui.label(format!(
            "Range: {:.0}°C - {:.0}°C",
            stats.min_temp, stats.max_temp
        ));
        ui.label(format!("Average: {:.1}°C", stats.avg_temp));
    }

    fn render_controls(&self, ui: &mut egui::Ui) {
        ui.heading("Movement");
        ui.label("W/A/S/D - Move player");
        ui.label("Space - Jump");

        ui.add_space(4.0);
        ui.heading("Camera");
        ui.label("+/- or Wheel - Zoom");

        ui.add_space(4.0);
        ui.heading("Materials");
        ui.label("0-9 - Select material");
        ui.label("Left Click - Place");

        ui.add_space(4.0);
        ui.heading("UI");
        ui.label("F1 - Stats | F2 - Chunks");
        ui.label("F4 - Params | F6 - Log");
        ui.label("H - Help | L - Levels");
        ui.label("I - Inventory | C - Craft");

        ui.add_space(4.0);
        ui.label(format!(
            "Selected: {}",
            self.ctx.materials.get(self.ctx.selected_material).name
        ));
    }

    fn render_level_selector(&self, ui: &mut egui::Ui) {
        ui.heading("Levels");
        ui.label(format!("Current: {}", self.ctx.game_mode_desc));

        ui.separator();

        if self.ctx.in_persistent_world {
            ui.label("Playing in Persistent World");
            ui.label("Use L key to open full selector");
        } else {
            ui.label("Demo level active");
        }
    }

    fn render_inventory(&self, ui: &mut egui::Ui) {
        ui.heading("Inventory");

        let inventory = &self.ctx.player.inventory;

        ui.label(format!(
            "Using {}/{} slots",
            inventory.used_slot_count(),
            inventory.max_slots
        ));

        ui.separator();

        // Show first few slots
        for i in 0..10.min(inventory.max_slots) {
            if let Some(Some(stack)) = inventory.get_slot(i) {
                match stack {
                    crate::entity::inventory::ItemStack::Material { material_id, count } => {
                        let mat = self.ctx.materials.get(*material_id);
                        ui.label(format!("[{}] {} x{}", i, mat.name, count));
                    }
                    crate::entity::inventory::ItemStack::Tool {
                        tool_id,
                        durability,
                    } => {
                        if let Some(tool_def) = self.ctx.tool_registry.get(*tool_id) {
                            ui.label(format!("[{}] {} ({})", i, tool_def.name, durability));
                        }
                    }
                }
            }
        }

        // Show equipped tool if any
        if let Some(tool_id) = self.ctx.player.equipped_tool {
            ui.separator();
            if let Some(tool_def) = self.ctx.tool_registry.get(tool_id) {
                ui.label(format!("Tool: {}", tool_def.name));
            }
        }
    }

    fn render_crafting(&self, ui: &mut egui::Ui) {
        ui.heading("Crafting");
        ui.label("Available recipes:");

        for recipe in self.ctx.recipe_registry.all_recipes() {
            ui.horizontal(|ui| {
                ui.label(&recipe.name);
                ui.label("-");
                // Show first input requirement
                if let Some((mat_id, count)) = recipe.inputs.first() {
                    let mat = self.ctx.materials.get(*mat_id);
                    ui.label(format!("{} x{}", mat.name, count));
                }
            });
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn render_logger(&self, ui: &mut egui::Ui) {
        egui_logger::logger_ui().show(ui);
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn render_parameters(&mut self, ui: &mut egui::Ui) {
        egui::ScrollArea::vertical().show(ui, |ui| {
            // Player Physics
            ui.heading("Player Physics");
            *self.ctx.params_changed |= ui
                .add(
                    egui::Slider::new(&mut self.ctx.params.player.move_speed, 50.0..=500.0)
                        .text("Move Speed"),
                )
                .changed();
            *self.ctx.params_changed |= ui
                .add(
                    egui::Slider::new(&mut self.ctx.params.player.gravity, 200.0..=1600.0)
                        .text("Gravity"),
                )
                .changed();
            *self.ctx.params_changed |= ui
                .add(
                    egui::Slider::new(&mut self.ctx.params.player.jump_velocity, 100.0..=600.0)
                        .text("Jump"),
                )
                .changed();
            *self.ctx.params_changed |= ui
                .add(
                    egui::Slider::new(&mut self.ctx.params.player.flight_thrust, 400.0..=2000.0)
                        .text("Flight Thrust"),
                )
                .changed();
            *self.ctx.params_changed |= ui
                .add(
                    egui::Slider::new(&mut self.ctx.params.player.max_fall_speed, 200.0..=1000.0)
                        .text("Max Fall Speed"),
                )
                .changed();

            ui.add_space(8.0);
            ui.heading("World");
            *self.ctx.params_changed |= ui
                .add(
                    egui::Slider::new(&mut self.ctx.params.world.active_chunk_radius, 1..=8)
                        .text("Active Chunk Radius"),
                )
                .changed();
            *self.ctx.params_changed |= ui
                .add(
                    egui::Slider::new(&mut self.ctx.params.world.autosave_interval_secs, 10..=300)
                        .text("Autosave (sec)"),
                )
                .changed();

            ui.add_space(8.0);
            ui.heading("Camera");
            *self.ctx.params_changed |= ui
                .add(
                    egui::Slider::new(&mut self.ctx.params.camera.zoom_speed, 1.01..=1.5)
                        .text("Zoom Speed"),
                )
                .changed();
            *self.ctx.params_changed |= ui
                .add(
                    egui::Slider::new(&mut self.ctx.params.camera.min_zoom, 0.001..=0.005)
                        .text("Min Zoom")
                        .logarithmic(true),
                )
                .changed();
            *self.ctx.params_changed |= ui
                .add(
                    egui::Slider::new(&mut self.ctx.params.camera.max_zoom, 0.005..=0.05)
                        .text("Max Zoom")
                        .logarithmic(true),
                )
                .changed();

            ui.add_space(8.0);
            ui.heading("Rendering");
            *self.ctx.params_changed |= ui
                .add(
                    egui::Slider::new(&mut self.ctx.params.rendering.scanline_intensity, 0.0..=0.5)
                        .text("Scanlines"),
                )
                .changed();
            *self.ctx.params_changed |= ui
                .add(
                    egui::Slider::new(&mut self.ctx.params.rendering.vignette_intensity, 0.0..=0.5)
                        .text("Vignette"),
                )
                .changed();
            *self.ctx.params_changed |= ui
                .add(
                    egui::Slider::new(&mut self.ctx.params.rendering.bloom_intensity, 0.0..=1.0)
                        .text("Bloom"),
                )
                .changed();

            ui.separator();
            ui.label("Water Animation:");
            *self.ctx.params_changed |= ui
                .add(
                    egui::Slider::new(
                        &mut self.ctx.params.rendering.water_noise_frequency,
                        0.01..=0.2,
                    )
                    .text("Frequency"),
                )
                .changed();
            *self.ctx.params_changed |= ui
                .add(
                    egui::Slider::new(&mut self.ctx.params.rendering.water_noise_speed, 0.5..=5.0)
                        .text("Speed"),
                )
                .changed();
            *self.ctx.params_changed |= ui
                .add(
                    egui::Slider::new(
                        &mut self.ctx.params.rendering.water_noise_amplitude,
                        0.0..=0.2,
                    )
                    .text("Amplitude"),
                )
                .changed();

            ui.separator();
            ui.label("Lava Animation:");
            *self.ctx.params_changed |= ui
                .add(
                    egui::Slider::new(
                        &mut self.ctx.params.rendering.lava_noise_frequency,
                        0.01..=0.15,
                    )
                    .text("Frequency"),
                )
                .changed();
            *self.ctx.params_changed |= ui
                .add(
                    egui::Slider::new(&mut self.ctx.params.rendering.lava_noise_speed, 0.5..=3.0)
                        .text("Speed"),
                )
                .changed();
            *self.ctx.params_changed |= ui
                .add(
                    egui::Slider::new(
                        &mut self.ctx.params.rendering.lava_noise_amplitude,
                        0.0..=0.3,
                    )
                    .text("Amplitude"),
                )
                .changed();

            ui.separator();
            ui.label("Multi-Pass Bloom:");
            *self.ctx.params_changed |= ui
                .checkbox(&mut self.ctx.params.rendering.bloom_enabled, "Enable Bloom")
                .changed();
            if self.ctx.params.rendering.bloom_enabled {
                *self.ctx.params_changed |= ui
                    .add(
                        egui::Slider::new(&mut self.ctx.params.rendering.bloom_quality, 3..=5)
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
                *self.ctx.params_changed |= ui
                    .add(
                        egui::Slider::new(
                            &mut self.ctx.params.rendering.bloom_threshold,
                            0.4..=0.8,
                        )
                        .text("Threshold"),
                    )
                    .changed();
            }

            ui.add_space(8.0);
            ui.heading("Debug");
            *self.ctx.params_changed |= ui
                .checkbox(
                    &mut self.ctx.params.debug.debug_placement,
                    "Debug Placement",
                )
                .changed();
            *self.ctx.params_changed |= ui
                .checkbox(
                    &mut self.ctx.params.debug.verbose_logging,
                    "Verbose Logging",
                )
                .changed();
            *self.ctx.params_changed |= ui
                .add(
                    egui::Slider::new(&mut self.ctx.params.debug.brush_size, 1..=10)
                        .text("Brush Size"),
                )
                .on_hover_text(
                    "Circular brush radius for material placement (1 = single pixel, 10 = large circle)",
                )
                .changed();
        });
    }

    #[cfg(feature = "multiplayer")]
    fn render_multiplayer_stats(&mut self, ui: &mut egui::Ui) {
        if let Some(manager) = self.ctx.multiplayer_manager {
            super::multiplayer_panel::render_multiplayer_panel(
                ui,
                manager,
                self.ctx.multiplayer_panel_state,
                self.ctx.multiplayer_metrics,
            );
        } else {
            ui.label("Multiplayer not available");
            ui.label("Rebuild with --features multiplayer_native to enable");
        }
    }

    #[cfg(feature = "profiling")]
    fn render_profiler(&self, ui: &mut egui::Ui) {
        puffin_egui::profiler_ui(ui);
    }
}

/// Render the dock area as a side panel on the right
pub fn render_dock(ctx: &egui::Context, dock_manager: &mut DockManager, dock_ctx: DockContext<'_>) {
    let mut viewer = DockTabViewer { ctx: dock_ctx };

    egui::SidePanel::right("debug_dock")
        .default_width(400.0)
        .resizable(true)
        .frame(egui::Frame::new())
        .show(ctx, |ui| {
            let style = Style::from_egui(ui.style().as_ref());
            DockArea::new(&mut dock_manager.dock_state)
                .style(style)
                .show_inside(ui, &mut viewer);
        });
}
