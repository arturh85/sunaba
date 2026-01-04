//! UI system - tooltips, overlays, stats, and controls

pub mod controls_help;
pub mod crafting_ui;
pub mod dock;
pub mod hud;
pub mod inventory_ui;
pub mod level_selector;
#[cfg(not(target_arch = "wasm32"))]
pub mod logger_panel;
#[cfg(all(not(target_arch = "wasm32"), feature = "multiplayer"))]
pub mod multiplayer_stats;
#[cfg(not(target_arch = "wasm32"))]
pub mod params_panel;
pub mod stats;
pub mod toasts;
pub mod tooltip;
pub mod ui_state;

pub use controls_help::ControlsHelpState;
pub use crafting_ui::CraftingUI;
pub use dock::{DockManager, DockTab};
pub use hud::Hud;
pub use inventory_ui::InventoryPanel;
pub use level_selector::LevelSelectorState;
#[cfg(not(target_arch = "wasm32"))]
pub use logger_panel::LoggerPanel;
#[cfg(not(target_arch = "wasm32"))]
pub use params_panel::ParamsPanel;
pub use stats::{SimulationStats, StatsCollector};
pub use toasts::ToastManager;
pub use tooltip::TooltipState;
pub use ui_state::UiState;
