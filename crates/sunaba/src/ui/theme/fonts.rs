//! Font loading and configuration system
//!
//! This module handles loading custom fonts for different theme variants.
//! Fonts are embedded using `include_bytes!()` for both native and WASM targets.

use egui::{Context, FontData, FontDefinitions, FontFamily, FontId};

/// Font system for theme variants
pub struct FontSystem;

impl FontSystem {
    /// Load fonts for a specific theme variant
    ///
    /// This will replace the default egui fonts with theme-appropriate fonts.
    /// For now, all variants use Fira Code/Sans family for modern readability.
    ///
    /// # Example
    /// ```no_run
    /// use sunaba::ui::theme::{FontSystem, ThemeVariant};
    /// let ctx = egui::Context::default();
    /// FontSystem::load_for_variant(&ctx, ThemeVariant::CozyAlchemist);
    /// ```
    pub fn load_for_variant(ctx: &Context, _variant: crate::ui::theme::ThemeVariant) {
        let mut fonts = FontDefinitions::default();

        // Embed fonts for all platforms (including WASM)
        fonts.font_data.insert(
            "FiraCode".to_owned(),
            FontData::from_static(include_bytes!("../../../assets/fonts/FiraCode-Regular.ttf"))
                .into(),
        );
        fonts.font_data.insert(
            "FiraCode-Medium".to_owned(),
            FontData::from_static(include_bytes!("../../../assets/fonts/FiraCode-Medium.ttf"))
                .into(),
        );
        fonts.font_data.insert(
            "FiraSans".to_owned(),
            FontData::from_static(include_bytes!("../../../assets/fonts/FiraSans-Regular.ttf"))
                .into(),
        );
        fonts.font_data.insert(
            "FiraSans-SemiBold".to_owned(),
            FontData::from_static(include_bytes!(
                "../../../assets/fonts/FiraSans-SemiBold.ttf"
            ))
            .into(),
        );

        // Set font families
        // Proportional family: Fira Sans for UI text
        fonts.families.insert(
            FontFamily::Proportional,
            vec![
                "FiraSans".to_owned(),
                "FiraSans-SemiBold".to_owned(),
                "FiraCode".to_owned(),
            ],
        );

        // Monospace family: Fira Code for stats/numbers/code
        fonts
            .families
            .insert(FontFamily::Monospace, vec!["FiraCode".to_owned()]);

        ctx.set_fonts(fonts);
    }

    /// Get default proportional font size for UI elements
    pub fn proportional_size() -> f32 {
        14.0
    }

    /// Get default monospace font size for stats/numbers
    pub fn monospace_size() -> f32 {
        13.0
    }

    /// Get heading font size
    pub fn heading_size() -> f32 {
        18.0
    }

    /// Get small text size (tooltips, labels)
    pub fn small_size() -> f32 {
        12.0
    }

    /// Create a proportional FontId at default size
    pub fn proportional() -> FontId {
        FontId::proportional(Self::proportional_size())
    }

    /// Create a monospace FontId at default size
    pub fn monospace() -> FontId {
        FontId::monospace(Self::monospace_size())
    }

    /// Create a heading FontId
    pub fn heading() -> FontId {
        FontId::proportional(Self::heading_size())
    }

    /// Create a small FontId
    pub fn small() -> FontId {
        FontId::proportional(Self::small_size())
    }
}

// Font loading implementation for Stage 4 (commented out for now):
/*
impl FontSystem {
    pub fn load_for_variant(ctx: &Context, variant: ThemeVariant) {
        let mut fonts = FontDefinitions::default();

        match variant {
            ThemeVariant::CozyAlchemist | ThemeVariant::DarkCavern => {
                Self::load_fira_fonts(&mut fonts);
                Self::set_fira_families(&mut fonts);
            }
            ThemeVariant::PixelAdventure => {
                Self::load_fira_fonts(&mut fonts); // Still use Fira for now
                Self::set_fira_families(&mut fonts);
                // TODO: Add pixelart font option later
            }
        }

        ctx.set_fonts(fonts);
    }

    fn load_fira_fonts(fonts: &mut FontDefinitions) {
        // Embed fonts for all platforms (including WASM)
        fonts.font_data.insert(
            "FiraCode".to_owned(),
            FontData::from_static(include_bytes!("../../assets/fonts/FiraCode-Regular.ttf")),
        );
        fonts.font_data.insert(
            "FiraSans".to_owned(),
            FontData::from_static(include_bytes!("../../assets/fonts/FiraSans-Regular.ttf")),
        );
        fonts.font_data.insert(
            "FiraSans-SemiBold".to_owned(),
            FontData::from_static(include_bytes!("../../assets/fonts/FiraSans-SemiBold.ttf")),
        );
    }

    fn set_fira_families(fonts: &mut FontDefinitions) {
        // Proportional family: Fira Sans for UI
        fonts.families.insert(
            FontFamily::Proportional,
            vec!["FiraSans".to_owned(), "FiraCode".to_owned()],
        );

        // Monospace family: Fira Code for stats/numbers
        fonts.families.insert(
            FontFamily::Monospace,
            vec!["FiraCode".to_owned()],
        );
    }
}
*/
