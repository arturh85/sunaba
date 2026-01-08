//! Central UI state management

use super::dock::{DockManager, DockTab};
use super::hud::Hud;
use super::stats::{SimulationStats, StatsCollector};
use super::theme::SunabaTheme;
use super::toasts::ToastManager;
use super::tooltip::TooltipState;
use super::worldgen_editor::WorldGenEditor;
#[cfg(not(target_arch = "wasm32"))]
use crate::config::GameConfig;
use web_time::Instant;

/// Action returned from the loading overlay
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoadingAction {
    /// No action taken (still loading)
    None,
    /// User wants to return to local world
    ReturnToLocal,
    /// User wants to retry connection
    Retry,
}

/// Central UI state container
pub struct UiState {
    /// Stats collector and display
    pub stats: StatsCollector,

    /// Cached stats for throttled display (updates 4x/sec)
    pub display_stats: SimulationStats,
    last_stats_update: Instant,

    /// Tooltip for mouseover information
    pub tooltip: TooltipState,

    /// HUD (health, hunger bars)
    pub hud: Hud,

    /// Toast notification manager
    pub toasts: ToastManager,

    /// Dock manager for dockable panels
    pub dock: DockManager,

    /// Track if parameters were changed (for propagation to renderer)
    #[cfg(not(target_arch = "wasm32"))]
    pub params_changed: bool,

    /// Multiplayer metrics collector (both native and WASM)
    #[cfg(feature = "multiplayer")]
    pub metrics_collector: Option<crate::multiplayer::metrics::MetricsCollector>,

    /// Multiplayer panel state (connection UI)
    #[cfg(feature = "multiplayer")]
    pub multiplayer_panel: super::multiplayer_panel::MultiplayerPanelState,

    /// Game over panel state (death screen)
    pub game_over_panel: super::game_over_panel::GameOverPanelState,

    /// World generation editor (F7)
    pub worldgen_editor: WorldGenEditor,

    /// UI theme (catppuccin + game-specific colors)
    pub theme: SunabaTheme,
}

impl UiState {
    /// Stats display update interval (4 updates per second)
    const STATS_UPDATE_INTERVAL_MS: u128 = 250;

    #[cfg(not(target_arch = "wasm32"))]
    pub fn new(config: &GameConfig) -> Self {
        // Load theme from config
        let theme = match config.ui.theme.as_str() {
            "dark_cavern" => SunabaTheme::dark_cavern(),
            "pixel_adventure" => SunabaTheme::pixel_adventure(),
            _ => SunabaTheme::cozy_alchemist(), // Default fallback
        };

        Self {
            stats: StatsCollector::new(),
            display_stats: SimulationStats::default(),
            last_stats_update: Instant::now(),
            tooltip: TooltipState::new(),
            hud: Hud::new(),
            toasts: ToastManager::new(),
            dock: DockManager::new(),
            params_changed: false,
            #[cfg(feature = "multiplayer")]
            metrics_collector: None,
            #[cfg(feature = "multiplayer")]
            multiplayer_panel: super::multiplayer_panel::MultiplayerPanelState::new(),
            game_over_panel: super::game_over_panel::GameOverPanelState::new(),
            worldgen_editor: WorldGenEditor::new(),
            theme,
        }
    }

    #[cfg(target_arch = "wasm32")]
    pub fn new() -> Self {
        Self {
            stats: StatsCollector::new(),
            display_stats: SimulationStats::default(),
            last_stats_update: Instant::now(),
            tooltip: TooltipState::new(),
            hud: Hud::new(),
            toasts: ToastManager::new(),
            dock: DockManager::new(),
            #[cfg(feature = "multiplayer")]
            metrics_collector: None,
            #[cfg(feature = "multiplayer")]
            multiplayer_panel: super::multiplayer_panel::MultiplayerPanelState::new(),
            game_over_panel: super::game_over_panel::GameOverPanelState::new(),
            worldgen_editor: WorldGenEditor::new(),
            theme: SunabaTheme::default(), // Cozy Alchemist theme
        }
    }

    /// Toggle a dock tab (H, L, I, C hotkeys)
    pub fn toggle_tab(&mut self, tab: DockTab) {
        self.dock.toggle_tab(tab);
    }

    /// Take the params_changed flag (resets it to false)
    #[cfg(not(target_arch = "wasm32"))]
    pub fn take_params_changed(&mut self) -> bool {
        std::mem::take(&mut self.params_changed)
    }

    /// Show a success toast notification
    pub fn show_toast(&mut self, message: &str) {
        self.toasts.success(message);
    }

    /// Show an info toast notification
    pub fn show_toast_info(&mut self, message: &str) {
        self.toasts.info(message);
    }

    /// Show a warning toast notification
    pub fn show_toast_warning(&mut self, message: &str) {
        self.toasts.warning(message);
    }

    /// Show an error toast notification
    pub fn show_toast_error(&mut self, message: &str) {
        self.toasts.error(message);
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
    #[cfg(not(target_arch = "wasm32"))]
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
        recipe_registry: &crate::entity::crafting::RecipeRegistry,
        config: &mut crate::config::GameConfig,
        show_game_over: bool,
        #[cfg(feature = "multiplayer")] multiplayer_manager: Option<
            &crate::multiplayer::MultiplayerManager,
        >,
    ) {
        // Update stats display (throttled)
        if self.last_stats_update.elapsed().as_millis() >= Self::STATS_UPDATE_INTERVAL_MS {
            self.display_stats = self.stats.stats().clone();
            self.last_stats_update = Instant::now();
        }

        // Render dock with all panels
        let dock_ctx = super::dock::DockContext {
            stats: &self.display_stats,
            selected_material,
            materials,
            game_mode_desc,
            level_manager,
            in_persistent_world,
            player,
            tool_registry,
            recipe_registry,
            params: config,
            params_changed: &mut self.params_changed,
            #[cfg(feature = "multiplayer")]
            multiplayer_metrics: self.metrics_collector.as_ref().map(|c| c.metrics()),
            #[cfg(feature = "multiplayer")]
            multiplayer_manager,
            #[cfg(feature = "multiplayer")]
            multiplayer_panel_state: &mut self.multiplayer_panel,
        };
        super::dock::render_dock(ctx, &mut self.dock, dock_ctx);

        // Render overlays (outside dock)
        let material_names: Vec<&str> = (0..15).map(|id| materials.get(id).name.as_str()).collect();
        self.hud.render(
            ctx,
            player,
            selected_material,
            &material_names,
            tool_registry,
            &self.theme.game,
        );
        self.toasts.render(ctx);
        self.tooltip.render_creature(ctx, Some(cursor_screen_pos));
        self.tooltip.render(ctx, cursor_screen_pos);

        // Render worldgen editor (F7)
        self.worldgen_editor.render(ctx, materials);

        // Render game over screen (if player is dead)
        if show_game_over {
            self.game_over_panel.render(ctx, &self.theme.game);
        }
    }

    /// Render all UI elements (WASM version)
    #[allow(clippy::too_many_arguments)]
    #[cfg(target_arch = "wasm32")]
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
        recipe_registry: &crate::entity::crafting::RecipeRegistry,
        show_game_over: bool,
        #[cfg(feature = "multiplayer")] multiplayer_manager: Option<
            &crate::multiplayer::MultiplayerManager,
        >,
    ) {
        // Update stats display (throttled)
        if self.last_stats_update.elapsed().as_millis() >= Self::STATS_UPDATE_INTERVAL_MS {
            self.display_stats = self.stats.stats().clone();
            self.last_stats_update = Instant::now();
        }

        // Render dock with all panels (WASM - no params)
        let dock_ctx = super::dock::DockContext {
            stats: &self.display_stats,
            selected_material,
            materials,
            game_mode_desc,
            level_manager,
            in_persistent_world,
            player,
            tool_registry,
            recipe_registry,
            #[cfg(feature = "multiplayer")]
            multiplayer_metrics: self.metrics_collector.as_ref().map(|c| c.metrics()),
            #[cfg(feature = "multiplayer")]
            multiplayer_manager,
            #[cfg(feature = "multiplayer")]
            multiplayer_panel_state: &mut self.multiplayer_panel,
        };
        super::dock::render_dock(ctx, &mut self.dock, dock_ctx);

        // Render overlays (outside dock)
        let material_names: Vec<&str> = (0..15).map(|id| materials.get(id).name.as_str()).collect();
        self.hud.render(
            ctx,
            player,
            selected_material,
            &material_names,
            tool_registry,
            &self.theme.game,
        );
        self.toasts.render(ctx);
        self.tooltip.render_creature(ctx, Some(cursor_screen_pos));
        self.tooltip.render(ctx, cursor_screen_pos);

        // Render worldgen editor (F7)
        self.worldgen_editor.render(ctx, materials);

        // Render game over screen (if player is dead)
        if show_game_over {
            self.game_over_panel.render(ctx, &self.theme.game);
        }
    }

    /// Render fullscreen loading overlay for multiplayer chunk loading
    /// Returns LoadingAction based on user interaction
    #[cfg(feature = "multiplayer")]
    pub fn render_loading_overlay(
        ctx: &egui::Context,
        chunks_loaded: usize,
        total_chunks: usize,
        elapsed: std::time::Duration,
        timed_out: bool,
    ) -> LoadingAction {
        use egui::{Align2, Color32, CornerRadius, FontId, Vec2};

        let mut action = LoadingAction::None;

        // Fullscreen overlay
        egui::Area::new("loading_overlay".into())
            .fixed_pos(egui::pos2(0.0, 0.0))
            .anchor(Align2::LEFT_TOP, Vec2::ZERO)
            .show(ctx, |ui| {
                // Dark semi-transparent background covering entire screen
                let screen_rect = ctx.viewport_rect();
                ui.painter().rect_filled(
                    screen_rect,
                    CornerRadius::ZERO,
                    Color32::from_black_alpha(220),
                );

                // Center panel
                egui::Window::new("loading_panel")
                    .title_bar(false)
                    .resizable(false)
                    .anchor(Align2::CENTER_CENTER, Vec2::ZERO)
                    .show(ctx, |ui| {
                        ui.set_min_width(400.0);
                        ui.vertical_centered(|ui| {
                            ui.add_space(20.0);

                            if timed_out {
                                // ERROR STATE
                                ui.label(
                                    egui::RichText::new("⚠")
                                        .size(64.0)
                                        .color(Color32::from_rgb(255, 100, 100)),
                                );
                                ui.add_space(10.0);

                                ui.label(
                                    egui::RichText::new("Failed to Load World")
                                        .font(FontId::proportional(24.0))
                                        .color(Color32::WHITE),
                                );
                                ui.add_space(10.0);

                                ui.label(
                                    egui::RichText::new(format!(
                                        "Waited {}s without receiving chunks from server",
                                        elapsed.as_secs()
                                    ))
                                    .font(FontId::proportional(14.0))
                                    .color(Color32::LIGHT_GRAY),
                                );

                                ui.add_space(20.0);

                                ui.horizontal(|ui| {
                                    if ui
                                        .add_sized(
                                            [150.0, 40.0],
                                            egui::Button::new("Back to Local World"),
                                        )
                                        .clicked()
                                    {
                                        action = LoadingAction::ReturnToLocal;
                                    }

                                    ui.add_space(10.0);

                                    if ui
                                        .add_sized([150.0, 40.0], egui::Button::new("Retry"))
                                        .clicked()
                                    {
                                        action = LoadingAction::Retry;
                                    }
                                });
                            } else {
                                // LOADING STATE
                                ui.add(egui::Spinner::new().size(64.0));
                                ui.add_space(10.0);

                                ui.label(
                                    egui::RichText::new("Loading World from Server...")
                                        .font(FontId::proportional(24.0))
                                        .color(Color32::WHITE),
                                );
                                ui.add_space(10.0);

                                // Progress bar
                                let progress = chunks_loaded as f32 / total_chunks as f32;
                                ui.add(
                                    egui::ProgressBar::new(progress)
                                        .text(format!(
                                            "{}/{} chunks loaded",
                                            chunks_loaded, total_chunks
                                        ))
                                        .desired_width(300.0),
                                );

                                ui.add_space(10.0);

                                ui.label(
                                    egui::RichText::new(format!("Waited {}s", elapsed.as_secs()))
                                        .font(FontId::proportional(14.0))
                                        .color(Color32::LIGHT_GRAY),
                                );
                            }

                            ui.add_space(20.0);
                        });
                    });
            });

        action
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl Default for UiState {
    fn default() -> Self {
        Self::new(&GameConfig::default())
    }
}

#[cfg(target_arch = "wasm32")]
impl Default for UiState {
    fn default() -> Self {
        Self::new()
    }
}
