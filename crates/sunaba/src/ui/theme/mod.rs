//! UI theming system combining catppuccin base with game-specific colors
//!
//! This module provides a centralized theming system that:
//! - Uses catppuccin-egui for base UI widget theming
//! - Layers game-specific semantic colors for health, materials, crafting, etc.
//! - Supports multiple theme variants (Cozy Alchemist, Dark Cavern, Pixel Adventure)
//! - Provides responsive sizing and font management

pub mod colors;
pub mod fonts;
pub mod sizing;

pub use colors::GameColors;
pub use fonts::FontSystem;
pub use sizing::{GridSpacing, ResponsiveSizing};

use egui::{Context, Shadow, Stroke};

/// Theme variant identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ThemeVariant {
    /// Warm, inviting alchemy lab aesthetic with modern readability
    #[default]
    CozyAlchemist,
    /// High-contrast underground mining theme
    DarkCavern,
    /// Pure retro NES/SNES-inspired pixelart aesthetic
    PixelAdventure,
}

impl ThemeVariant {
    /// Get the variant name as a string
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::CozyAlchemist => "cozy_alchemist",
            Self::DarkCavern => "dark_cavern",
            Self::PixelAdventure => "pixel_adventure",
        }
    }

    /// Parse a variant from a string
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "cozy_alchemist" => Some(Self::CozyAlchemist),
            "dark_cavern" => Some(Self::DarkCavern),
            "pixel_adventure" => Some(Self::PixelAdventure),
            _ => None,
        }
    }
}

/// Complete UI theme combining catppuccin base + game-specific colors
///
/// This struct encapsulates all theme configuration for the game UI.
/// It layers game-specific semantic colors on top of catppuccin's base theme.
#[derive(Debug, Clone)]
pub struct SunabaTheme {
    /// Catppuccin base theme (for panels, text, borders, widgets)
    pub base: catppuccin_egui::Theme,
    /// Game-specific semantic colors (for health, materials, crafting, etc.)
    pub game: GameColors,
    /// Theme variant identifier
    pub variant: ThemeVariant,
}

impl SunabaTheme {
    /// Create the "Cozy Alchemist" theme (default)
    ///
    /// Warm, inviting theme with alchemy-inspired colors.
    /// Uses catppuccin MOCHA as base.
    pub fn cozy_alchemist() -> Self {
        Self {
            base: catppuccin_egui::MOCHA,
            game: GameColors::cozy_alchemist(),
            variant: ThemeVariant::CozyAlchemist,
        }
    }

    /// Create the "Dark Cavern" theme
    ///
    /// High-contrast underground mining aesthetic.
    /// Uses catppuccin MOCHA (darkest variant).
    pub fn dark_cavern() -> Self {
        Self {
            base: catppuccin_egui::MOCHA,
            game: GameColors::dark_cavern(),
            variant: ThemeVariant::DarkCavern,
        }
    }

    /// Create the "Pixel Adventure" theme
    ///
    /// Pure retro NES/SNES-inspired colors.
    /// Uses catppuccin MOCHA as fallback (colors overridden in apply_to_ctx).
    pub fn pixel_adventure() -> Self {
        Self {
            base: catppuccin_egui::MOCHA,
            game: GameColors::pixel_adventure(),
            variant: ThemeVariant::PixelAdventure,
        }
    }

    /// Apply this theme to an egui context
    ///
    /// This sets both the catppuccin base theme and variant-specific
    /// style overrides (corner radius, borders, shadows, spacing).
    ///
    /// # Example
    /// ```no_run
    /// use sunaba::ui::theme::SunabaTheme;
    /// let ctx = egui::Context::default();
    /// let theme = SunabaTheme::cozy_alchemist();
    /// theme.apply_to_ctx(&ctx);
    /// ```
    pub fn apply_to_ctx(&self, ctx: &Context) {
        // First, apply catppuccin base theme
        catppuccin_egui::set_theme(ctx, self.base);

        // Then, apply variant-specific style overrides
        match self.variant {
            ThemeVariant::CozyAlchemist => self.apply_cozy_style(ctx),
            ThemeVariant::DarkCavern => self.apply_cavern_style(ctx),
            ThemeVariant::PixelAdventure => self.apply_pixel_style(ctx),
        }
    }

    /// Apply "Cozy Alchemist" style overrides
    ///
    /// - Rounded corners (6px)
    /// - Smooth borders (1.5px normal, 2.5px active)
    /// - Subtle shadows
    /// - Generous spacing
    fn apply_cozy_style(&self, ctx: &Context) {
        ctx.style_mut(|style| {
            // Smooth borders
            style.visuals.window_stroke.width = 1.5;
            style.visuals.widgets.inactive.bg_stroke.width = 1.5;
            style.visuals.widgets.active.bg_stroke.width = 2.5;

            // Subtle shadow
            style.visuals.window_shadow = Shadow {
                offset: [0, 2],
                blur: 4,
                spread: 0,
                color: egui::Color32::from_black_alpha(50),
            };
            style.visuals.popup_shadow = style.visuals.window_shadow;

            // Generous spacing
            style.spacing.item_spacing = [12.0, 8.0].into();
            style.spacing.window_margin = egui::Margin {
                left: 16,
                right: 16,
                top: 16,
                bottom: 16,
            };
            style.spacing.button_padding = [16.0, 8.0].into();
        });
    }

    /// Apply "Dark Cavern" style overrides
    ///
    /// - Sharp corners (3px)
    /// - Strong borders (2px normal, 3px active)
    /// - No shadows
    /// - Tight spacing (information-dense)
    fn apply_cavern_style(&self, ctx: &Context) {
        ctx.style_mut(|style| {
            // Strong borders
            style.visuals.window_stroke.width = 2.0;
            style.visuals.widgets.inactive.bg_stroke.width = 2.0;
            style.visuals.widgets.active.bg_stroke.width = 3.0;

            // No shadows
            style.visuals.window_shadow = Shadow::NONE;
            style.visuals.popup_shadow = Shadow::NONE;

            // Tight spacing (information-dense)
            style.spacing.item_spacing = [8.0, 6.0].into();
            style.spacing.window_margin = egui::Margin {
                left: 12,
                right: 12,
                top: 12,
                bottom: 12,
            };
            style.spacing.button_padding = [12.0, 6.0].into();
        });
    }

    /// Apply "Pixel Adventure" style overrides
    ///
    /// - Pixel-perfect sharp corners (0px)
    /// - Hard borders (2px solid)
    /// - No shadows
    /// - Grid-aligned spacing (8px multiples)
    fn apply_pixel_style(&self, ctx: &Context) {
        ctx.style_mut(|style| {
            // Hard borders
            style.visuals.window_stroke = Stroke::new(2.0, egui::Color32::BLACK);
            style.visuals.widgets.inactive.bg_stroke.width = 2.0;
            style.visuals.widgets.active.bg_stroke.width = 2.0;

            // No shadows
            style.visuals.window_shadow = Shadow::NONE;
            style.visuals.popup_shadow = Shadow::NONE;

            // Grid-aligned spacing (8px multiples)
            style.spacing.item_spacing = [8.0, 8.0].into();
            style.spacing.window_margin = egui::Margin {
                left: 8,
                right: 8,
                top: 8,
                bottom: 8,
            };
            style.spacing.button_padding = [16.0, 8.0].into();
            style.spacing.indent = 16.0;
        });
    }
}

impl Default for SunabaTheme {
    fn default() -> Self {
        Self::cozy_alchemist()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_theme_creation() {
        let theme = SunabaTheme::cozy_alchemist();
        assert_eq!(theme.variant, ThemeVariant::CozyAlchemist);

        let theme = SunabaTheme::dark_cavern();
        assert_eq!(theme.variant, ThemeVariant::DarkCavern);

        let theme = SunabaTheme::pixel_adventure();
        assert_eq!(theme.variant, ThemeVariant::PixelAdventure);
    }

    #[test]
    fn test_variant_string_conversion() {
        assert_eq!(ThemeVariant::CozyAlchemist.as_str(), "cozy_alchemist");
        assert_eq!(
            ThemeVariant::from_str("cozy_alchemist"),
            Some(ThemeVariant::CozyAlchemist)
        );
        assert_eq!(ThemeVariant::from_str("invalid"), None);
    }

    #[test]
    fn test_default_theme() {
        let theme = SunabaTheme::default();
        assert_eq!(theme.variant, ThemeVariant::CozyAlchemist);
    }
}
