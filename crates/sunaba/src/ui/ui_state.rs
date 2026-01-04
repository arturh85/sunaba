//! Central UI state management

use super::dock::{DockManager, DockTab};
use super::hud::Hud;
use super::stats::{SimulationStats, StatsCollector};
use super::toasts::ToastManager;
use super::tooltip::TooltipState;
#[cfg(not(target_arch = "wasm32"))]
use crate::config::GameConfig;
use web_time::Instant;

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
}

impl UiState {
    /// Stats display update interval (4 updates per second)
    const STATS_UPDATE_INTERVAL_MS: u128 = 250;

    #[cfg(not(target_arch = "wasm32"))]
    #[allow(unused_variables)]
    pub fn new(config: &GameConfig) -> Self {
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
        );
        self.toasts.render(ctx);
        self.tooltip.render_creature(ctx, Some(cursor_screen_pos));
        self.tooltip.render(ctx, cursor_screen_pos);
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
        );
        self.toasts.render(ctx);
        self.tooltip.render_creature(ctx, Some(cursor_screen_pos));
        self.tooltip.render(ctx, cursor_screen_pos);
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
