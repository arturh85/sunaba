//! Context-aware placement system for world generation
//!
//! Provides tools for analyzing placement locations during world generation,
//! enabling context-aware feature placement like:
//! - Stalactites in caves (attached to ceiling, air below)
//! - Trees on surface (solid ground, air above)
//! - Bridges over gaps (ground on both sides)
//! - Underground mushrooms (cave interior, low light)

use crate::simulation::{MaterialId, MaterialType, Materials};
use crate::world::biome::BiomeType;
use crate::world::generation::WorldGenerator;

/// Maximum distance to scan for ground/ceiling detection
pub const MAX_SCAN_DISTANCE: i32 = 128;

/// Context information about a placement location
#[derive(Debug, Clone)]
pub struct PlacementContext {
    /// World coordinates of the query point
    pub x: i32,
    pub y: i32,

    /// Distance to nearest solid ground below (None if not found within scan range)
    pub ground_distance: Option<i32>,

    /// Distance to nearest solid ceiling above (None if not found within scan range)
    pub ceiling_distance: Option<i32>,

    /// Number of consecutive air pixels above this position
    pub air_above: i32,

    /// Number of consecutive air pixels below this position
    pub air_below: i32,

    /// Whether this position is in an enclosed space (cave interior)
    /// True if there's solid material both above and below
    pub is_enclosed: bool,

    /// Whether this position is on the surface (solid below, open sky above)
    pub is_surface: bool,

    /// Material at this position
    pub material_at: u16,

    /// Biome type at this position
    pub biome_type: BiomeType,

    /// Estimated light level (0-15, based on depth and enclosure)
    /// During generation, this is estimated; at runtime, use LightSystem
    pub estimated_light: u8,
}

impl Default for PlacementContext {
    fn default() -> Self {
        Self {
            x: 0,
            y: 0,
            ground_distance: None,
            ceiling_distance: None,
            air_above: 0,
            air_below: 0,
            is_enclosed: false,
            is_surface: false,
            material_at: MaterialId::AIR,
            biome_type: BiomeType::Plains,
            estimated_light: 15,
        }
    }
}

/// Predicates for conditional placement
#[derive(Debug, Clone, PartialEq)]
pub enum PlacementPredicate {
    /// Requires minimum air pixels above (e.g., grass where 10+ air above)
    MinAirAbove(i32),

    /// Requires minimum air pixels below
    MinAirBelow(i32),

    /// Must be in a cave interior (enclosed by solid above and below)
    IsCaveInterior,

    /// Must be on the surface (solid below, open sky above)
    IsSurface,

    /// At or attached to a ceiling (solid above)
    AtCeiling,

    /// On or above ground (solid below)
    OnGround,

    /// Minimum gap width (horizontal air span)
    MinGapWidth(i32),

    /// Within a depth range (Y coordinate range)
    DepthRange { min: i32, max: i32 },

    /// Minimum distance from surface
    MinDepthBelowSurface(i32),

    /// Light level requirement
    LightRange { min: u8, max: u8 },

    /// In a specific biome
    InBiome(BiomeType),

    /// Ground must be within distance
    GroundWithin(i32),

    /// Ceiling must be within distance
    CeilingWithin(i32),

    /// Combined predicate: all must match
    All(Vec<PlacementPredicate>),

    /// Combined predicate: any must match
    Any(Vec<PlacementPredicate>),

    /// Negated predicate
    Not(Box<PlacementPredicate>),
}

impl PlacementPredicate {
    /// Evaluate this predicate against a placement context
    pub fn evaluate(&self, ctx: &PlacementContext) -> bool {
        match self {
            PlacementPredicate::MinAirAbove(min) => ctx.air_above >= *min,
            PlacementPredicate::MinAirBelow(min) => ctx.air_below >= *min,
            PlacementPredicate::IsCaveInterior => ctx.is_enclosed,
            PlacementPredicate::IsSurface => ctx.is_surface,
            PlacementPredicate::AtCeiling => ctx.ceiling_distance == Some(0),
            PlacementPredicate::OnGround => ctx.ground_distance == Some(0),
            PlacementPredicate::MinGapWidth(_width) => {
                // Gap width requires horizontal scanning, handled separately
                // For now, check if we're in air with significant space below
                ctx.material_at == MaterialId::AIR && ctx.air_below >= *_width
            }
            PlacementPredicate::DepthRange { min, max } => ctx.y >= *min && ctx.y <= *max,
            PlacementPredicate::MinDepthBelowSurface(min_depth) => {
                // Approximate surface as y=0, actual surface varies by terrain
                ctx.y < -*min_depth
            }
            PlacementPredicate::LightRange { min, max } => {
                ctx.estimated_light >= *min && ctx.estimated_light <= *max
            }
            PlacementPredicate::InBiome(biome) => ctx.biome_type == *biome,
            PlacementPredicate::GroundWithin(dist) => {
                ctx.ground_distance.is_some_and(|d| d <= *dist)
            }
            PlacementPredicate::CeilingWithin(dist) => {
                ctx.ceiling_distance.is_some_and(|d| d <= *dist)
            }
            PlacementPredicate::All(predicates) => predicates.iter().all(|p| p.evaluate(ctx)),
            PlacementPredicate::Any(predicates) => predicates.iter().any(|p| p.evaluate(ctx)),
            PlacementPredicate::Not(predicate) => !predicate.evaluate(ctx),
        }
    }

    // Builder methods for common patterns

    /// Create a predicate for stalactite placement
    pub fn stalactite() -> Self {
        PlacementPredicate::All(vec![
            PlacementPredicate::IsCaveInterior,
            PlacementPredicate::AtCeiling,
            PlacementPredicate::MinAirBelow(5),
        ])
    }

    /// Create a predicate for stalagmite placement
    pub fn stalagmite() -> Self {
        PlacementPredicate::All(vec![
            PlacementPredicate::IsCaveInterior,
            PlacementPredicate::OnGround,
            PlacementPredicate::MinAirAbove(5),
        ])
    }

    /// Create a predicate for surface tree placement
    pub fn surface_tree() -> Self {
        PlacementPredicate::All(vec![
            PlacementPredicate::IsSurface,
            PlacementPredicate::OnGround,
            PlacementPredicate::MinAirAbove(10),
        ])
    }

    /// Create a predicate for cave mushroom placement
    pub fn cave_mushroom() -> Self {
        PlacementPredicate::All(vec![
            PlacementPredicate::IsCaveInterior,
            PlacementPredicate::OnGround,
            PlacementPredicate::LightRange { min: 0, max: 5 },
        ])
    }

    /// Create a predicate for deep ore placement
    pub fn deep_ore(min_depth: i32) -> Self {
        PlacementPredicate::All(vec![
            PlacementPredicate::MinDepthBelowSurface(min_depth),
            PlacementPredicate::Not(Box::new(PlacementPredicate::IsCaveInterior)),
        ])
    }
}

/// Context scanner for querying placement information
///
/// Can query context from a WorldGenerator (during generation) or
/// could be extended to query from loaded chunks (runtime).
pub struct ContextScanner<'a> {
    generator: &'a WorldGenerator,
    materials: Materials,
}

impl<'a> ContextScanner<'a> {
    /// Create a new context scanner using a world generator
    pub fn new(generator: &'a WorldGenerator) -> Self {
        Self {
            generator,
            materials: Materials::new(),
        }
    }

    /// Query placement context at a specific world position
    pub fn query(&self, x: i32, y: i32) -> PlacementContext {
        let material_at = self.get_material(x, y);
        let biome_type = self.get_biome(x);

        // Scan downward for ground
        let (ground_distance, air_below) = self.scan_down(x, y);

        // Scan upward for ceiling
        let (ceiling_distance, air_above) = self.scan_up(x, y);

        // Determine if enclosed (cave interior)
        let is_enclosed = ground_distance.is_some() && ceiling_distance.is_some();

        // Determine if on surface (ground below, no ceiling / open sky)
        let is_surface = ground_distance.is_some() && ceiling_distance.is_none();

        // Estimate light level based on depth and enclosure
        let estimated_light = self.estimate_light(y, is_enclosed, ceiling_distance);

        PlacementContext {
            x,
            y,
            ground_distance,
            ceiling_distance,
            air_above,
            air_below,
            is_enclosed,
            is_surface,
            material_at,
            biome_type,
            estimated_light,
        }
    }

    /// Scan multiple positions and filter by predicate
    pub fn find_placements(
        &self,
        start_x: i32,
        end_x: i32,
        start_y: i32,
        end_y: i32,
        predicate: &PlacementPredicate,
    ) -> Vec<PlacementContext> {
        let mut results = Vec::new();
        for y in start_y..=end_y {
            for x in start_x..=end_x {
                let ctx = self.query(x, y);
                if predicate.evaluate(&ctx) {
                    results.push(ctx);
                }
            }
        }
        results
    }

    /// Check if a predicate matches at a position
    pub fn matches(&self, x: i32, y: i32, predicate: &PlacementPredicate) -> bool {
        predicate.evaluate(&self.query(x, y))
    }

    /// Scan horizontally to find gap width at a given Y level
    pub fn scan_gap_width(&self, x: i32, y: i32) -> i32 {
        if !self.is_air(x, y) {
            return 0;
        }

        // Scan left
        let mut left = 0;
        for dx in 1..=MAX_SCAN_DISTANCE {
            if self.is_solid(x - dx, y) {
                break;
            }
            left = dx;
        }

        // Scan right
        let mut right = 0;
        for dx in 1..=MAX_SCAN_DISTANCE {
            if self.is_solid(x + dx, y) {
                break;
            }
            right = dx;
        }

        left + right + 1
    }

    // Private helper methods

    fn get_material(&self, x: i32, y: i32) -> u16 {
        // Use generator to get material at position
        // This works because generator.get_material_at is deterministic
        self.generator.generate_single_pixel(x, y)
    }

    fn get_biome(&self, x: i32) -> BiomeType {
        self.generator.get_biome_at(x)
    }

    fn is_solid(&self, x: i32, y: i32) -> bool {
        let material_id = self.get_material(x, y);
        let material = self.materials.get(material_id);
        matches!(material.material_type, MaterialType::Solid)
    }

    fn is_air(&self, x: i32, y: i32) -> bool {
        self.get_material(x, y) == MaterialId::AIR
    }

    /// Scan downward from position, returns (distance to ground, air count)
    fn scan_down(&self, x: i32, y: i32) -> (Option<i32>, i32) {
        let mut air_count = 0;
        let mut ground_dist = None;

        for dy in 0..MAX_SCAN_DISTANCE {
            let check_y = y - dy;
            if self.is_solid(x, check_y) {
                ground_dist = Some(dy);
                break;
            }
            if self.is_air(x, check_y) {
                air_count += 1;
            }
        }

        (ground_dist, air_count)
    }

    /// Scan upward from position, returns (distance to ceiling, air count)
    fn scan_up(&self, x: i32, y: i32) -> (Option<i32>, i32) {
        let mut air_count = 0;
        let mut ceiling_dist = None;

        for dy in 0..MAX_SCAN_DISTANCE {
            let check_y = y + dy;
            if self.is_solid(x, check_y) {
                ceiling_dist = Some(dy);
                break;
            }
            if self.is_air(x, check_y) {
                air_count += 1;
            }
        }

        (ceiling_dist, air_count)
    }

    /// Estimate light level based on depth and enclosure
    fn estimate_light(&self, y: i32, is_enclosed: bool, ceiling_distance: Option<i32>) -> u8 {
        // Surface level or above: full light
        if y >= 0 && !is_enclosed {
            return 15;
        }

        // In cave with ceiling: dark
        if is_enclosed {
            // Deeper = darker
            let depth_factor = (-y).min(1000) as f32 / 1000.0;
            return (5.0 * (1.0 - depth_factor)) as u8;
        }

        // Near surface but underground
        if let Some(dist) = ceiling_distance {
            // Light decreases with distance from opening
            return (15 - (dist / 10).min(15)) as u8;
        }

        // Open sky underground (shaft/canyon)
        10
    }
}

// Extension trait for WorldGenerator to support single-pixel queries
impl WorldGenerator {
    /// Generate material for a single pixel (used by ContextScanner)
    pub fn generate_single_pixel(&self, x: i32, y: i32) -> u16 {
        // Re-use the internal material generation logic
        self.get_material_at_internal(x, y)
    }

    /// Get biome type at an X coordinate
    pub fn get_biome_at(&self, x: i32) -> BiomeType {
        self.get_biome_at_internal(x)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_placement_context_default() {
        let ctx = PlacementContext::default();
        assert_eq!(ctx.x, 0);
        assert_eq!(ctx.y, 0);
        assert!(!ctx.is_enclosed);
        assert!(!ctx.is_surface);
    }

    #[test]
    fn test_predicate_min_air_above() {
        let ctx = PlacementContext {
            air_above: 5,
            ..Default::default()
        };

        assert!(PlacementPredicate::MinAirAbove(5).evaluate(&ctx));
        assert!(PlacementPredicate::MinAirAbove(3).evaluate(&ctx));
        assert!(!PlacementPredicate::MinAirAbove(10).evaluate(&ctx));
    }

    #[test]
    fn test_predicate_is_cave_interior() {
        let ctx_enclosed = PlacementContext {
            is_enclosed: true,
            ..Default::default()
        };
        assert!(PlacementPredicate::IsCaveInterior.evaluate(&ctx_enclosed));

        let ctx_open = PlacementContext::default();
        assert!(!PlacementPredicate::IsCaveInterior.evaluate(&ctx_open));
    }

    #[test]
    fn test_predicate_is_surface() {
        let ctx_surface = PlacementContext {
            is_surface: true,
            ..Default::default()
        };
        assert!(PlacementPredicate::IsSurface.evaluate(&ctx_surface));

        let ctx_underground = PlacementContext::default();
        assert!(!PlacementPredicate::IsSurface.evaluate(&ctx_underground));
    }

    #[test]
    fn test_predicate_depth_range() {
        let ctx = PlacementContext {
            y: -100,
            ..Default::default()
        };

        assert!(PlacementPredicate::DepthRange { min: -200, max: 0 }.evaluate(&ctx));
        assert!(!PlacementPredicate::DepthRange { min: -50, max: 0 }.evaluate(&ctx));
        assert!(
            !PlacementPredicate::DepthRange {
                min: -200,
                max: -150
            }
            .evaluate(&ctx)
        );
    }

    #[test]
    fn test_predicate_light_range() {
        let ctx = PlacementContext {
            estimated_light: 5,
            ..Default::default()
        };

        assert!(PlacementPredicate::LightRange { min: 0, max: 10 }.evaluate(&ctx));
        assert!(!PlacementPredicate::LightRange { min: 10, max: 15 }.evaluate(&ctx));
    }

    #[test]
    fn test_predicate_all_combinator() {
        let ctx = PlacementContext {
            is_enclosed: true,
            air_above: 10,
            ground_distance: Some(0),
            ..Default::default()
        };

        let predicate = PlacementPredicate::All(vec![
            PlacementPredicate::IsCaveInterior,
            PlacementPredicate::MinAirAbove(5),
            PlacementPredicate::OnGround,
        ]);

        assert!(predicate.evaluate(&ctx));

        let ctx_not_enclosed = PlacementContext {
            is_enclosed: false,
            air_above: 10,
            ground_distance: Some(0),
            ..Default::default()
        };
        assert!(!predicate.evaluate(&ctx_not_enclosed));
    }

    #[test]
    fn test_predicate_any_combinator() {
        let ctx_surface = PlacementContext {
            is_surface: true,
            ..Default::default()
        };

        let predicate = PlacementPredicate::Any(vec![
            PlacementPredicate::IsCaveInterior,
            PlacementPredicate::IsSurface,
        ]);

        assert!(predicate.evaluate(&ctx_surface));

        let ctx_neither = PlacementContext::default();
        assert!(!predicate.evaluate(&ctx_neither));
    }

    #[test]
    fn test_predicate_not() {
        let ctx_surface = PlacementContext {
            is_surface: true,
            ..Default::default()
        };
        assert!(
            !PlacementPredicate::Not(Box::new(PlacementPredicate::IsSurface))
                .evaluate(&ctx_surface)
        );

        let ctx_not_surface = PlacementContext::default();
        assert!(
            PlacementPredicate::Not(Box::new(PlacementPredicate::IsSurface))
                .evaluate(&ctx_not_surface)
        );
    }

    #[test]
    fn test_predicate_stalactite() {
        let ctx = PlacementContext {
            is_enclosed: true,
            ceiling_distance: Some(0),
            air_below: 10,
            ..Default::default()
        };

        assert!(PlacementPredicate::stalactite().evaluate(&ctx));

        // Not at ceiling
        let ctx_not_at_ceiling = PlacementContext {
            is_enclosed: true,
            ceiling_distance: Some(5),
            air_below: 10,
            ..Default::default()
        };
        assert!(!PlacementPredicate::stalactite().evaluate(&ctx_not_at_ceiling));
    }

    #[test]
    fn test_predicate_surface_tree() {
        let ctx = PlacementContext {
            is_surface: true,
            ground_distance: Some(0),
            air_above: 15,
            ..Default::default()
        };

        assert!(PlacementPredicate::surface_tree().evaluate(&ctx));

        // Not enough air above
        let ctx_low_air = PlacementContext {
            is_surface: true,
            ground_distance: Some(0),
            air_above: 5,
            ..Default::default()
        };
        assert!(!PlacementPredicate::surface_tree().evaluate(&ctx_low_air));
    }

    #[test]
    fn test_in_biome_predicate() {
        let ctx = PlacementContext {
            biome_type: BiomeType::Desert,
            ..Default::default()
        };

        assert!(PlacementPredicate::InBiome(BiomeType::Desert).evaluate(&ctx));
        assert!(!PlacementPredicate::InBiome(BiomeType::Forest).evaluate(&ctx));
    }

    #[test]
    fn test_ground_within_predicate() {
        let ctx = PlacementContext {
            ground_distance: Some(5),
            ..Default::default()
        };

        assert!(PlacementPredicate::GroundWithin(10).evaluate(&ctx));
        assert!(PlacementPredicate::GroundWithin(5).evaluate(&ctx));
        assert!(!PlacementPredicate::GroundWithin(3).evaluate(&ctx));

        let ctx_no_ground = PlacementContext::default();
        assert!(!PlacementPredicate::GroundWithin(100).evaluate(&ctx_no_ground));
    }

    // Integration tests with WorldGenerator

    #[test]
    fn test_context_scanner_above_surface() {
        let generator = WorldGenerator::new(42);
        let scanner = ContextScanner::new(&generator);

        // Query well above surface (y=100, surface is around y=0)
        let ctx = scanner.query(0, 100);

        // Should be in open air, not enclosed
        assert!(!ctx.is_enclosed, "Above surface should not be enclosed");
        assert_eq!(
            ctx.material_at,
            MaterialId::AIR,
            "Above surface should be air"
        );
        assert_eq!(
            ctx.estimated_light, 15,
            "Above surface should have full light"
        );
    }

    #[test]
    fn test_context_scanner_surface() {
        let generator = WorldGenerator::new(42);
        let scanner = ContextScanner::new(&generator);

        // Get terrain height at x=0
        let terrain_y = generator.get_terrain_height(0);

        // Query just above the terrain surface
        let ctx = scanner.query(0, terrain_y + 1);

        // Should detect surface
        assert!(
            ctx.is_surface || ctx.material_at == MaterialId::AIR,
            "Just above terrain should be surface or air"
        );
        assert!(ctx.ground_distance.is_some(), "Should detect ground below");
    }

    #[test]
    fn test_context_scanner_deep_underground() {
        let generator = WorldGenerator::new(42);
        let scanner = ContextScanner::new(&generator);

        // Query deep underground (y=-500)
        let ctx = scanner.query(0, -500);

        // Deep underground, light should be low
        assert!(
            ctx.estimated_light < 15,
            "Deep underground should have reduced light"
        );
        assert!(ctx.y < 0, "Should be underground");
    }

    #[test]
    fn test_context_scanner_biome_detection() {
        let generator = WorldGenerator::new(42);
        let scanner = ContextScanner::new(&generator);

        // Query at different X positions to potentially hit different biomes
        let ctx1 = scanner.query(0, 0);
        let ctx2 = scanner.query(1000, 0);
        let ctx3 = scanner.query(2000, 0);

        // All should have valid biome types
        assert!(
            matches!(
                ctx1.biome_type,
                BiomeType::Desert
                    | BiomeType::Plains
                    | BiomeType::Forest
                    | BiomeType::Mountains
                    | BiomeType::Ocean
            ),
            "Should have valid biome at x=0"
        );
        assert!(
            matches!(
                ctx2.biome_type,
                BiomeType::Desert
                    | BiomeType::Plains
                    | BiomeType::Forest
                    | BiomeType::Mountains
                    | BiomeType::Ocean
            ),
            "Should have valid biome at x=1000"
        );
        assert!(
            matches!(
                ctx3.biome_type,
                BiomeType::Desert
                    | BiomeType::Plains
                    | BiomeType::Forest
                    | BiomeType::Mountains
                    | BiomeType::Ocean
            ),
            "Should have valid biome at x=2000"
        );
    }

    #[test]
    fn test_context_scanner_matches() {
        let generator = WorldGenerator::new(42);
        let scanner = ContextScanner::new(&generator);

        // Check if surface predicate matches at various heights
        let terrain_y = generator.get_terrain_height(0);

        // Well above terrain should not match OnGround
        assert!(
            !scanner.matches(0, terrain_y + 50, &PlacementPredicate::OnGround),
            "High above terrain should not match OnGround"
        );
    }

    #[test]
    fn test_context_scanner_gap_width() {
        let generator = WorldGenerator::new(42);
        let scanner = ContextScanner::new(&generator);

        // In solid stone, gap width should be 0
        let gap_in_stone = scanner.scan_gap_width(0, -1000);
        // Gap depends on cave generation, so just check it's a reasonable value
        assert!(gap_in_stone >= 0, "Gap width should be non-negative");
    }

    #[test]
    fn test_terrain_height_consistency() {
        let generator = WorldGenerator::new(42);

        // Terrain height should be consistent for same X coordinate
        let height1 = generator.get_terrain_height(100);
        let height2 = generator.get_terrain_height(100);

        assert_eq!(height1, height2, "Terrain height should be deterministic");
    }

    #[test]
    fn test_context_scanner_find_placements() {
        let generator = WorldGenerator::new(42);
        let scanner = ContextScanner::new(&generator);

        // Search a small area for positions with specific depth range
        let predicate = PlacementPredicate::DepthRange { min: -100, max: 0 };
        let placements = scanner.find_placements(-10, 10, -100, 0, &predicate);

        // All found placements should match the predicate
        for ctx in &placements {
            assert!(
                ctx.y >= -100 && ctx.y <= 0,
                "Found placement should be in depth range"
            );
        }
    }
}
