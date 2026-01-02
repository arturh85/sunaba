//! Central UI state management

use super::controls_help::ControlsHelpState;
use super::crafting_ui::CraftingUI;
use super::hud::Hud;
use super::inventory_ui::InventoryPanel;
use super::level_selector::LevelSelectorState;
use super::stats::{SimulationStats, StatsCollector};
use super::tooltip::TooltipState;
use instant::Instant;

/// Central UI state container
pub struct UiState {
    /// Stats collector and display
    pub stats: StatsCollector,

    /// Whether stats window is visible
    pub stats_visible: bool,

    /// Cached stats for throttled display (updates 4x/sec)
    display_stats: SimulationStats,
    last_stats_update: Instant,

    /// Tooltip for mouseover information
    pub tooltip: TooltipState,

    /// Controls help panel
    pub controls_help: ControlsHelpState,

    /// Level selector panel
    pub level_selector: LevelSelectorState,

    /// HUD (health, hunger bars)
    pub hud: Hud,

    /// Inventory panel
    pub inventory: InventoryPanel,

    /// Crafting UI panel
    pub crafting_ui: CraftingUI,

    /// Toast notification (message, shown_at)
    pub toast_message: Option<(String, Instant)>,
}

impl UiState {
    /// Stats display update interval (4 updates per second)
    const STATS_UPDATE_INTERVAL_MS: u128 = 250;

    pub fn new() -> Self {
        Self {
            stats: StatsCollector::new(),
            stats_visible: true, // Start with stats visible
            display_stats: SimulationStats::default(),
            last_stats_update: Instant::now(),
            tooltip: TooltipState::new(),
            controls_help: ControlsHelpState::new(),
            level_selector: LevelSelectorState::new(),
            hud: Hud::new(),
            inventory: InventoryPanel::new(),
            crafting_ui: CraftingUI::new(),
            toast_message: None,
        }
    }

    /// Toggle stats visibility
    pub fn toggle_stats(&mut self) {
        self.stats_visible = !self.stats_visible;
    }

    /// Toggle controls help visibility
    pub fn toggle_help(&mut self) {
        self.controls_help.toggle();
    }

    /// Toggle level selector visibility
    pub fn toggle_level_selector(&mut self) {
        self.level_selector.toggle();
    }

    /// Toggle inventory visibility
    pub fn toggle_inventory(&mut self) {
        self.inventory.toggle();
    }

    /// Toggle crafting UI visibility
    pub fn toggle_crafting(&mut self) {
        self.crafting_ui.toggle();
    }

    /// Show a toast notification
    pub fn show_toast(&mut self, message: &str) {
        self.toast_message = Some((message.to_string(), Instant::now()));
    }

    /// Update tooltip with world data
    pub fn update_tooltip(
        &mut self,
        world: &crate::world::World,
        materials: &crate::simulation::Materials,
        mouse_world_pos: Option<(i32, i32)>,
        light_overlay_active: bool,
    ) {
        // Update creature tooltip first (takes priority)
        self.tooltip.update_creature(world, mouse_world_pos);

        // Update material tooltip
        self.tooltip
            .update(world, materials, mouse_world_pos, light_overlay_active);
    }

    /// Render all UI elements
    #[allow(clippy::too_many_arguments)]
    pub fn render(
        &mut self,
        ctx: &egui::Context,
        cursor_screen_pos: egui::Pos2,
        selected_material: u16,
        materials: &crate::simulation::Materials,
        game_mode_desc: &str,
        in_persistent_world: bool,
        level_manager: &crate::levels::LevelManager,
        player: &crate::entity::player::Player,
        tool_registry: &crate::entity::tools::ToolRegistry,
    ) {
        // Collect material names for UI display
        let material_names: Vec<&str> = (0..15).map(|id| materials.get(id).name.as_str()).collect();

        // Render HUD (always visible)
        self.hud.render(
            ctx,
            player,
            selected_material,
            &material_names,
            tool_registry,
        );

        // Render inventory panel (if open)
        self.inventory.render(ctx, player, &material_names);

        if self.stats_visible {
            self.render_stats(ctx);
        }

        // Render controls help with game mode description
        self.controls_help
            .render_with_level(ctx, selected_material, materials, game_mode_desc);

        // Render level selector
        self.level_selector
            .render(ctx, level_manager, game_mode_desc, in_persistent_world);

        // Render toast notifications
        if let Some((msg, shown_at)) = &self.toast_message {
            const TOAST_DURATION_SECS: u64 = 3;
            if shown_at.elapsed().as_secs() < TOAST_DURATION_SECS {
                egui::Area::new("toast_notification".into())
                    .anchor(egui::Align2::CENTER_TOP, [0.0, 50.0])
                    .show(ctx, |ui| {
                        ui.label(
                            egui::RichText::new(msg)
                                .size(20.0)
                                .color(egui::Color32::from_rgb(100, 255, 100)),
                        );
                    });
            } else {
                self.toast_message = None;
            }
        }

        // Render creature tooltip (takes priority over material tooltip)
        self.tooltip.render_creature(ctx, Some(cursor_screen_pos));

        // Render material tooltip (only if creature tooltip not visible)
        self.tooltip.render(ctx, cursor_screen_pos);
    }

    fn render_stats(&mut self, ctx: &egui::Context) {
        // Throttle stats display updates to 4 times per second
        if self.last_stats_update.elapsed().as_millis() >= Self::STATS_UPDATE_INTERVAL_MS {
            self.display_stats = self.stats.stats().clone();
            self.last_stats_update = Instant::now();
        }

        let stats = &self.display_stats;

        egui::Window::new("Debug Stats")
            .default_pos(egui::pos2(10.0, 10.0))
            .resizable(false)
            .collapsible(true)
            .show(ctx, |ui| {
                ui.heading("Performance");
                ui.label(format!("FPS: {:.1}", stats.fps));
                ui.label(format!("Frame: {:.2}ms", stats.frame_time_ms));
                ui.label(format!("  Sim: {:.2}ms", stats.sim_time_ms));
                ui.label(format!("  UI Build: {:.2}ms", stats.egui_build_time_ms));
                ui.label(format!("  Overlays: {:.2}ms", stats.overlay_time_ms));
                ui.label(format!("  Pixels: {:.2}ms", stats.pixel_buffer_time_ms));
                ui.label(format!("  Upload: {:.2}ms", stats.gpu_upload_time_ms));
                ui.label(format!("  Acquire: {:.2}ms", stats.acquire_time_ms));
                ui.label(format!("  egui GPU: {:.2}ms", stats.egui_time_ms));
                ui.label(format!("  Present: {:.2}ms", stats.present_time_ms));

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

                ui.separator();
                ui.heading("Activity");
                ui.label(format!("Moved: {} pixels", stats.pixels_moved));
                ui.label(format!("Reactions: {}", stats.reactions));
                ui.label(format!("State Changes: {}", stats.state_changes));

                ui.separator();
                ui.heading("Render");
                ui.label(format!("Dirty Chunks: {}", stats.render_dirty_chunks));
                ui.label(format!("Rendered Total: {}", stats.rendered_chunks_total));
            });
    }
}

impl Default for UiState {
    fn default() -> Self {
        Self::new()
    }
}
