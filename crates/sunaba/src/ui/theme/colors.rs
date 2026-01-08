//! Game-specific semantic colors for UI theming
//!
//! This module defines the `GameColors` struct which contains all semantic color roles
//! used throughout the game UI. These colors layer on top of catppuccin base themes to
//! provide game-specific visual feedback for health, materials, tools, crafting, etc.

use egui::Color32;

/// Semantic color palette for game-specific UI elements
///
/// This struct contains all the colors needed for game mechanics that don't fit
/// into standard UI widget theming (which is handled by catppuccin base theme).
/// Colors are organized by function rather than by hue.
#[derive(Debug, Clone)]
pub struct GameColors {
    // ===== Health & Vitality =====
    /// Health bar fill color (full health)
    pub health_full: Color32,
    /// Health bar fill color (low health, <30%)
    pub health_low: Color32,
    /// Health bar fill color (critical health, <10%)
    pub health_critical: Color32,
    /// Health bar background color
    pub health_bg: Color32,

    // ===== Hunger & Stamina =====
    /// Hunger bar fill color (well fed)
    pub hunger_full: Color32,
    /// Hunger bar fill color (getting hungry, <50%)
    pub hunger_low: Color32,
    /// Hunger bar fill color (starving, <10%)
    pub hunger_starving: Color32,
    /// Hunger bar background color
    pub hunger_bg: Color32,

    // ===== Materials (Semantic Categories) =====
    /// Earth/soil materials (dirt, sand, clay)
    pub material_earth: Color32,
    /// Fire/lava materials (flame, molten)
    pub material_fire: Color32,
    /// Water/liquid materials (water, oil)
    pub material_water: Color32,
    /// Air/gas materials (steam, smoke)
    pub material_air: Color32,
    /// Wood/organic materials (wood, plant matter)
    pub material_wood: Color32,
    /// Metal materials (iron, copper, gold)
    pub material_metal: Color32,
    /// Ore materials (raw ores before smelting)
    pub material_ore: Color32,
    /// Organic materials (seeds, plant fiber)
    pub material_organic: Color32,
    /// Stone materials (stone, rock, gravel)
    pub material_stone: Color32,
    /// Crystal materials (gems, glass)
    pub material_crystal: Color32,
    /// Magic/alchemical materials
    pub material_magic: Color32,
    /// Explosive materials (gunpowder, TNT)
    pub material_explosive: Color32,
    /// Acid/corrosive materials
    pub material_acid: Color32,
    /// Ice/frozen materials
    pub material_ice: Color32,
    /// Toxic materials (poison, pollution)
    pub material_toxic: Color32,
    /// Energy/plasma materials
    pub material_energy: Color32,

    // ===== Tools & Equipment =====
    /// Basic tools (hands, wooden tools)
    pub tool_basic: Color32,
    /// Advanced tools (stone, iron tools)
    pub tool_advanced: Color32,
    /// Legendary tools (gold, diamond, magical)
    pub tool_legendary: Color32,
    /// Tool durability full (>70%)
    pub tool_durability_full: Color32,
    /// Tool durability low (<30%)
    pub tool_durability_low: Color32,

    // ===== Crafting Feedback =====
    /// Recipe is craftable (all materials available)
    pub craftable: Color32,
    /// Recipe not craftable (missing materials)
    pub not_craftable: Color32,
    /// Crafting in progress
    pub crafting_in_progress: Color32,

    // ===== General Feedback =====
    /// Success state (achievement unlocked, action succeeded)
    pub success: Color32,
    /// Warning state (low resources, caution needed)
    pub warning: Color32,
    /// Error state (action failed, danger)
    pub error: Color32,
    /// Info state (neutral information)
    pub info: Color32,

    // ===== Selection & Interaction =====
    /// Selected inventory slot background
    pub selection_bg: Color32,
    /// Selected inventory slot border
    pub selection_border: Color32,
    /// Hover highlight color
    pub hover_highlight: Color32,

    // ===== Inventory UI =====
    /// Empty inventory slot background
    pub slot_empty: Color32,
    /// Filled inventory slot background
    pub slot_filled: Color32,
    /// Normal slot border
    pub slot_border: Color32,

    // ===== Mining & Progress =====
    /// Mining progress indicator
    pub mining_progress: Color32,
    /// XP/level bar color
    pub xp_bar: Color32,

    // ===== Text =====
    /// Primary text color (used when catppuccin text doesn't fit)
    pub text_primary: Color32,
    /// Secondary/muted text
    pub text_secondary: Color32,
    /// Disabled text
    pub text_disabled: Color32,
    /// Highlighted text
    pub text_highlight: Color32,

    // ===== Borders =====
    /// Normal border color
    pub border_normal: Color32,
    /// Active/focused border color
    pub border_active: Color32,
    /// Hover border color
    pub border_hover: Color32,
}

impl GameColors {
    /// Create the "Cozy Alchemist" color palette
    ///
    /// Warm, inviting theme with alchemy-inspired colors.
    /// Pairs with catppuccin MOCHA base theme.
    pub fn cozy_alchemist() -> Self {
        Self {
            // Health & Vitality (warm coral tones)
            health_full: Color32::from_rgb(220, 138, 120), // rosewater
            health_low: Color32::from_rgb(243, 139, 168),  // red
            health_critical: Color32::from_rgb(235, 160, 172), // maroon
            health_bg: Color32::from_rgb(69, 71, 90),      // surface1

            // Hunger (warm golds)
            hunger_full: Color32::from_rgb(249, 226, 175), // yellow
            hunger_low: Color32::from_rgb(250, 179, 135),  // peach
            hunger_starving: Color32::from_rgb(243, 139, 168), // red
            hunger_bg: Color32::from_rgb(69, 71, 90),      // surface1

            // Materials (alchemy-inspired)
            material_earth: Color32::from_rgb(148, 226, 213), // teal (mystical earth)
            material_fire: Color32::from_rgb(243, 139, 168),  // red (flame)
            material_water: Color32::from_rgb(137, 180, 250), // sapphire (water)
            material_air: Color32::from_rgb(180, 190, 254),   // lavender (ethereal)
            material_wood: Color32::from_rgb(166, 227, 161),  // green (organic)
            material_metal: Color32::from_rgb(166, 173, 200), // subtext0 (metallic)
            material_ore: Color32::from_rgb(249, 226, 175),   // yellow (gold ore)
            material_organic: Color32::from_rgb(166, 227, 161), // green
            material_stone: Color32::from_rgb(147, 153, 178), // overlay2 (gray stone)
            material_crystal: Color32::from_rgb(203, 166, 247), // mauve (magical crystal)
            material_magic: Color32::from_rgb(203, 166, 247), // mauve (arcane)
            material_explosive: Color32::from_rgb(250, 179, 135), // peach (explosive)
            material_acid: Color32::from_rgb(166, 227, 161),  // green (acid)
            material_ice: Color32::from_rgb(137, 220, 235),   // sky (ice)
            material_toxic: Color32::from_rgb(166, 227, 161), // green (toxic)
            material_energy: Color32::from_rgb(245, 194, 231), // pink (energy)

            // Tools
            tool_basic: Color32::from_rgb(166, 173, 200), // subtext0
            tool_advanced: Color32::from_rgb(148, 226, 213), // teal
            tool_legendary: Color32::from_rgb(249, 226, 175), // yellow (gold)
            tool_durability_full: Color32::from_rgb(166, 227, 161), // green
            tool_durability_low: Color32::from_rgb(243, 139, 168), // red

            // Crafting
            craftable: Color32::from_rgb(166, 227, 161), // green
            not_craftable: Color32::from_rgb(108, 112, 134), // overlay0 (grayed)
            crafting_in_progress: Color32::from_rgb(249, 226, 175), // yellow

            // Feedback
            success: Color32::from_rgb(166, 227, 161), // green
            warning: Color32::from_rgb(250, 179, 135), // peach
            error: Color32::from_rgb(243, 139, 168),   // red
            info: Color32::from_rgb(137, 220, 235),    // sky

            // Selection
            selection_bg: Color32::from_rgb(88, 91, 112), // surface2
            selection_border: Color32::from_rgb(203, 166, 247), // mauve (magical highlight)
            hover_highlight: Color32::from_rgb(88, 91, 112), // surface2

            // Inventory
            slot_empty: Color32::from_rgb(49, 50, 68), // surface0
            slot_filled: Color32::from_rgb(69, 71, 90), // surface1
            slot_border: Color32::from_rgb(108, 112, 134), // overlay0

            // Progress
            mining_progress: Color32::from_rgb(137, 220, 235), // sky (cyan)
            xp_bar: Color32::from_rgb(166, 227, 161),          // green

            // Text (fallback when catppuccin text doesn't fit)
            text_primary: Color32::from_rgb(205, 214, 244), // text
            text_secondary: Color32::from_rgb(186, 194, 222), // subtext1
            text_disabled: Color32::from_rgb(108, 112, 134), // overlay0
            text_highlight: Color32::from_rgb(255, 240, 210), // bright highlight

            // Borders
            border_normal: Color32::from_rgb(108, 112, 134), // overlay0
            border_active: Color32::from_rgb(203, 166, 247), // mauve
            border_hover: Color32::from_rgb(147, 153, 178),  // overlay2
        }
    }

    /// Create the "Dark Cavern" color palette
    ///
    /// High-contrast underground mining aesthetic.
    /// Pairs with catppuccin MOCHA (darkest variant).
    pub fn dark_cavern() -> Self {
        Self {
            // Health (stark red for visibility)
            health_full: Color32::from_rgb(243, 139, 168),
            health_low: Color32::from_rgb(235, 160, 172),
            health_critical: Color32::from_rgb(180, 99, 122),
            health_bg: Color32::from_rgb(49, 50, 68),

            // Hunger (torchlight glow)
            hunger_full: Color32::from_rgb(249, 226, 175),
            hunger_low: Color32::from_rgb(250, 179, 135),
            hunger_starving: Color32::from_rgb(243, 139, 168),
            hunger_bg: Color32::from_rgb(49, 50, 68),

            // Materials (underground/mineral palette)
            material_earth: Color32::from_rgb(108, 112, 134), // gray stone
            material_fire: Color32::from_rgb(243, 139, 168),  // molten lava
            material_water: Color32::from_rgb(116, 199, 236), // underground pool
            material_air: Color32::from_rgb(147, 153, 178),   // cave air
            material_wood: Color32::from_rgb(166, 227, 161),  // rare underground wood
            material_metal: Color32::from_rgb(147, 153, 178), // cold metal
            material_ore: Color32::from_rgb(249, 226, 175),   // gold ore
            material_organic: Color32::from_rgb(166, 227, 161),
            material_stone: Color32::from_rgb(108, 112, 134),
            material_crystal: Color32::from_rgb(203, 166, 247), // glowing crystal
            material_magic: Color32::from_rgb(203, 166, 247),
            material_explosive: Color32::from_rgb(250, 179, 135),
            material_acid: Color32::from_rgb(166, 227, 161),
            material_ice: Color32::from_rgb(137, 220, 235),
            material_toxic: Color32::from_rgb(166, 227, 161),
            material_energy: Color32::from_rgb(245, 194, 231),

            // Tools
            tool_basic: Color32::from_rgb(147, 153, 178),
            tool_advanced: Color32::from_rgb(250, 179, 135), // warm metal
            tool_legendary: Color32::from_rgb(249, 226, 175), // torchlight gold
            tool_durability_full: Color32::from_rgb(148, 226, 213),
            tool_durability_low: Color32::from_rgb(243, 139, 168),

            // Crafting
            craftable: Color32::from_rgb(148, 226, 213), // mineral ready
            not_craftable: Color32::from_rgb(108, 112, 134), // dull/unavailable
            crafting_in_progress: Color32::from_rgb(249, 226, 175),

            // Feedback
            success: Color32::from_rgb(148, 226, 213), // valuable find
            warning: Color32::from_rgb(250, 179, 135), // caution
            error: Color32::from_rgb(243, 139, 168),   // danger/collapse
            info: Color32::from_rgb(116, 199, 236),    // water seepage

            // Selection
            selection_bg: Color32::from_rgb(69, 71, 90), // coal black
            selection_border: Color32::from_rgb(249, 226, 175), // torchlight
            hover_highlight: Color32::from_rgb(88, 91, 112),

            // Inventory
            slot_empty: Color32::from_rgb(30, 30, 46), // very dark
            slot_filled: Color32::from_rgb(49, 50, 68),
            slot_border: Color32::from_rgb(88, 91, 112), // stone edge

            // Progress
            mining_progress: Color32::from_rgb(137, 220, 235),
            xp_bar: Color32::from_rgb(148, 226, 213),

            // Text
            text_primary: Color32::from_rgb(205, 214, 244),
            text_secondary: Color32::from_rgb(186, 194, 222),
            text_disabled: Color32::from_rgb(108, 112, 134),
            text_highlight: Color32::from_rgb(249, 226, 175),

            // Borders
            border_normal: Color32::from_rgb(88, 91, 112),
            border_active: Color32::from_rgb(249, 226, 175),
            border_hover: Color32::from_rgb(108, 112, 134),
        }
    }

    /// Create the "Pixel Adventure" color palette
    ///
    /// Pure retro NES/SNES-inspired saturated colors.
    /// Uses custom palette (NOT catppuccin-based).
    pub fn pixel_adventure() -> Self {
        Self {
            // Health (NES red)
            health_full: Color32::from_rgb(228, 59, 68), // bright red
            health_low: Color32::from_rgb(172, 50, 50),  // dark red
            health_critical: Color32::from_rgb(102, 30, 30), // very dark red
            health_bg: Color32::from_rgb(44, 33, 55),    // dark purple

            // Hunger (NES yellow)
            hunger_full: Color32::from_rgb(251, 242, 54), // pure yellow
            hunger_low: Color32::from_rgb(251, 146, 43),  // orange
            hunger_starving: Color32::from_rgb(228, 59, 68), // red
            hunger_bg: Color32::from_rgb(44, 33, 55),

            // Materials (16-color palette, high saturation)
            material_earth: Color32::from_rgb(138, 111, 48), // brown dirt
            material_fire: Color32::from_rgb(228, 59, 68),   // red flame
            material_water: Color32::from_rgb(79, 103, 129), // blue water
            material_air: Color32::from_rgb(139, 155, 180),  // light blue
            material_wood: Color32::from_rgb(55, 148, 110),  // green
            material_metal: Color32::from_rgb(139, 155, 180), // gray-blue
            material_ore: Color32::from_rgb(251, 242, 54),   // yellow gold
            material_organic: Color32::from_rgb(55, 148, 110), // green
            material_stone: Color32::from_rgb(102, 57, 49),  // dark brown
            material_crystal: Color32::from_rgb(181, 80, 136), // magenta
            material_magic: Color32::from_rgb(181, 80, 136), // magenta
            material_explosive: Color32::from_rgb(251, 146, 43),
            material_acid: Color32::from_rgb(55, 148, 110),
            material_ice: Color32::from_rgb(137, 220, 235),
            material_toxic: Color32::from_rgb(55, 148, 110),
            material_energy: Color32::from_rgb(251, 242, 54),

            // Tools
            tool_basic: Color32::from_rgb(102, 57, 49), // brown
            tool_advanced: Color32::from_rgb(139, 155, 180), // gray
            tool_legendary: Color32::from_rgb(251, 242, 54), // yellow gold
            tool_durability_full: Color32::from_rgb(55, 148, 110),
            tool_durability_low: Color32::from_rgb(228, 59, 68),

            // Crafting
            craftable: Color32::from_rgb(55, 148, 110), // green (ready!)
            not_craftable: Color32::from_rgb(102, 57, 49), // brown (disabled)
            crafting_in_progress: Color32::from_rgb(251, 242, 54),

            // Feedback (arcade style)
            success: Color32::from_rgb(55, 148, 110), // green (1UP!)
            warning: Color32::from_rgb(251, 146, 43), // orange (alert)
            error: Color32::from_rgb(228, 59, 68),    // red (game over)
            info: Color32::from_rgb(79, 103, 129),    // blue (info)

            // Selection
            selection_bg: Color32::from_rgb(79, 103, 129), // blue selected
            selection_border: Color32::from_rgb(251, 242, 54), // yellow highlight
            hover_highlight: Color32::from_rgb(79, 103, 129),

            // Inventory
            slot_empty: Color32::from_rgb(23, 19, 34), // near-black
            slot_filled: Color32::from_rgb(44, 33, 55),
            slot_border: Color32::from_rgb(139, 155, 180), // gray outline

            // Progress
            mining_progress: Color32::from_rgb(137, 220, 235),
            xp_bar: Color32::from_rgb(55, 148, 110),

            // Text
            text_primary: Color32::from_rgb(255, 255, 255), // pure white
            text_secondary: Color32::from_rgb(139, 155, 180),
            text_disabled: Color32::from_rgb(102, 57, 49),
            text_highlight: Color32::from_rgb(251, 242, 54),

            // Borders
            border_normal: Color32::from_rgb(139, 155, 180),
            border_active: Color32::from_rgb(251, 242, 54),
            border_hover: Color32::from_rgb(79, 103, 129),
        }
    }
}
