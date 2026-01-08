//! Screenshot layout templates
//!
//! Defines preset UI layouts for capturing screenshots that match in-game appearance.

use super::UiPanel;

/// Screenshot layout template
#[derive(Debug, Clone)]
pub struct ScreenshotLayout {
    /// Name of this layout
    pub name: &'static str,
    /// Description of this layout
    pub description: &'static str,
    /// Panels to show in the dock (in order)
    pub dock_panels: Vec<UiPanel>,
    /// Which panel should be active (focused) in the dock
    pub active_panel: Option<UiPanel>,
    /// Whether to show the HUD overlay
    pub show_hud: bool,
    /// Whether to show tooltip at cursor
    pub show_tooltip: bool,
    /// Cursor position (world coordinates) for tooltip, None = no cursor
    pub cursor_position: Option<(f32, f32)>,
    /// Whether to show toast notifications
    pub show_toasts: bool,
}

impl ScreenshotLayout {
    /// Get all available layout templates
    pub fn all_layouts() -> Vec<Self> {
        vec![
            Self::default_layout(),
            Self::inventory_layout(),
            Self::crafting_layout(),
            Self::debug_layout(),
            Self::minimal_layout(),
        ]
    }

    /// Get layout by name
    pub fn by_name(name: &str) -> Option<Self> {
        Self::all_layouts()
            .into_iter()
            .find(|layout| layout.name == name)
    }

    /// Default in-game layout: Logger + LevelSelector tabs, HUD overlay
    pub fn default_layout() -> Self {
        Self {
            name: "default",
            description: "Default in-game layout with logger and level selector",
            dock_panels: vec![UiPanel::Logger, UiPanel::LevelSelector],
            active_panel: Some(UiPanel::Logger),
            show_hud: true,
            show_tooltip: false,
            cursor_position: None,
            show_toasts: false,
        }
    }

    /// Inventory-focused layout: Inventory tab active, HUD overlay
    pub fn inventory_layout() -> Self {
        Self {
            name: "inventory",
            description: "Inventory panel with HUD overlay",
            dock_panels: vec![UiPanel::Inventory, UiPanel::Crafting],
            active_panel: Some(UiPanel::Inventory),
            show_hud: true,
            show_tooltip: false,
            cursor_position: None,
            show_toasts: false,
        }
    }

    /// Crafting-focused layout: Crafting tab active, HUD overlay
    pub fn crafting_layout() -> Self {
        Self {
            name: "crafting",
            description: "Crafting panel with HUD overlay",
            dock_panels: vec![UiPanel::Crafting, UiPanel::Inventory],
            active_panel: Some(UiPanel::Crafting),
            show_hud: true,
            show_tooltip: false,
            cursor_position: None,
            show_toasts: false,
        }
    }

    /// Debug layout: Logger + Params, no overlays
    pub fn debug_layout() -> Self {
        Self {
            name: "debug",
            description: "Debug panels without HUD overlays",
            dock_panels: vec![UiPanel::Logger, UiPanel::Params],
            active_panel: Some(UiPanel::Logger),
            show_hud: false,
            show_tooltip: false,
            cursor_position: None,
            show_toasts: false,
        }
    }

    /// Minimal layout: No UI panels, just world (for clean screenshots)
    pub fn minimal_layout() -> Self {
        Self {
            name: "minimal",
            description: "Clean world view with minimal UI",
            dock_panels: vec![],
            active_panel: None,
            show_hud: false,
            show_tooltip: false,
            cursor_position: None,
            show_toasts: false,
        }
    }
}
