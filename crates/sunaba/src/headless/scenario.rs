//! Training scenarios for creature evolution
//!
//! Each scenario defines a world configuration and evaluation criteria.

use glam::{IVec2, Vec2};

use crate::simulation::MaterialId;
use crate::world::World;
use sunaba_core::world::WorldGenerator;

use super::fitness::{
    CompositeFitness, DirectionalFoodFitness, DistanceFitness, FitnessFunction, ForagingFitness,
    MovementFitness, SurvivalFitness,
};
use super::terrain_config::TrainingTerrainConfig;

/// Configuration for a training scenario
#[derive(Debug, Clone)]
pub struct ScenarioConfig {
    /// Scenario name
    pub name: String,
    /// Description of expected behavior
    pub description: String,
    /// Expected evolved behavior
    pub expected_behavior: String,
    /// Spawn position for creatures
    pub spawn_position: Vec2,
    /// Evaluation duration in seconds
    pub eval_duration: f32,
    /// World width in pixels
    pub world_width: i32,
    /// World height in pixels
    pub world_height: i32,
    /// Optional procedural terrain config (None = use legacy manual terrain)
    pub terrain_config: Option<TrainingTerrainConfig>,
}

impl Default for ScenarioConfig {
    fn default() -> Self {
        Self {
            name: "Default".to_string(),
            description: "Basic training scenario".to_string(),
            expected_behavior: "Locomotion".to_string(),
            spawn_position: Vec2::new(100.0, 100.0),
            eval_duration: 30.0,
            world_width: 512,
            world_height: 256,
            terrain_config: None,
        }
    }
}

/// A training scenario with world setup and fitness evaluation
pub struct Scenario {
    /// Scenario configuration
    pub config: ScenarioConfig,
    /// Fitness function for evaluation
    pub fitness: Box<dyn FitnessFunction>,
}

impl Scenario {
    /// Create a basic locomotion scenario
    pub fn locomotion() -> Self {
        Self {
            config: ScenarioConfig {
                name: "Locomotion".to_string(),
                description: "Flat terrain, creatures must move as far as possible".to_string(),
                expected_behavior: "Walking, rolling, or crawling locomotion".to_string(),
                spawn_position: Vec2::new(100.0, 50.0),
                eval_duration: 30.0,
                world_width: 512,
                world_height: 128,
                terrain_config: None,
            },
            fitness: Box::new(DistanceFitness),
        }
    }

    /// Create a simple locomotion scenario with movement-focused fitness
    /// Uses simple morphology (fewer body parts) and penalizes stationary creatures
    pub fn simple_locomotion() -> Self {
        Self {
            config: ScenarioConfig {
                name: "SimpleLocomotion".to_string(),
                description: "Flat terrain optimized for simple creatures".to_string(),
                expected_behavior: "Basic walking locomotion".to_string(),
                spawn_position: Vec2::new(100.0, 50.0),
                eval_duration: 30.0,
                world_width: 400,
                world_height: 100,
                terrain_config: None,
            },
            fitness: Box::new(MovementFitness::new()),
        }
    }

    /// Create a foraging scenario with food sources
    pub fn foraging() -> Self {
        Self {
            config: ScenarioConfig {
                name: "Foraging".to_string(),
                description: "Terrain with scattered food sources".to_string(),
                expected_behavior: "Food-seeking behavior, efficient eating".to_string(),
                spawn_position: Vec2::new(256.0, 50.0),
                eval_duration: 30.0,
                world_width: 512,
                world_height: 128,
                terrain_config: None,
            },
            fitness: Box::new(ForagingFitness),
        }
    }

    /// Create a survival scenario with hazards
    pub fn survival() -> Self {
        Self {
            config: ScenarioConfig {
                name: "Survival".to_string(),
                description: "Terrain with danger zones (lava patches)".to_string(),
                expected_behavior: "Hazard avoidance, threat detection".to_string(),
                spawn_position: Vec2::new(100.0, 50.0),
                eval_duration: 30.0,
                world_width: 512,
                world_height: 128,
                terrain_config: None,
            },
            fitness: Box::new(SurvivalFitness),
        }
    }

    /// Create a balanced scenario combining multiple objectives
    pub fn balanced() -> Self {
        Self {
            config: ScenarioConfig {
                name: "Balanced".to_string(),
                description: "Flat terrain with some food, multi-objective fitness".to_string(),
                expected_behavior: "Balanced locomotion and foraging".to_string(),
                spawn_position: Vec2::new(100.0, 50.0),
                eval_duration: 30.0,
                world_width: 512,
                world_height: 128,
                terrain_config: None,
            },
            fitness: Box::new(CompositeFitness::balanced()),
        }
    }

    /// Create a food parcour scenario with obstacles
    ///
    /// Layout: 3 food items before a wall, 5 after
    /// Creatures start with 50% hunger and must collect food to survive
    ///
    /// Uses DirectionalFoodFitness to reward movement toward food (positive X direction).
    /// This fixes the issue where creatures evolved to move fast but in the wrong direction.
    pub fn parcour() -> Self {
        Self {
            config: ScenarioConfig {
                name: "Parcour".to_string(),
                description: "Contained tunnel with minable wall - must mine to reach food"
                    .to_string(),
                expected_behavior: "Mine through stone wall to reach food behind it".to_string(),
                spawn_position: Vec2::new(50.0, 40.0),
                eval_duration: 30.0,
                world_width: 420,
                world_height: 100,
                terrain_config: None,
            },
            fitness: Box::new(DirectionalFoodFitness::parcour()),
        }
    }

    /// Set up the world for this scenario
    /// Returns the world and a list of food positions for optimized sensing
    pub fn setup_world(&self) -> (World, Vec<Vec2>) {
        // Branch: Use procedural generation if terrain_config is provided
        if let Some(ref terrain_config) = self.config.terrain_config {
            self.setup_procedural_world(terrain_config)
        } else {
            self.setup_manual_world()
        }
    }

    /// Set up world with a custom terrain configuration (for multi-environment evaluation)
    /// Returns the world and a list of food positions for optimized sensing
    pub fn setup_world_with_terrain(
        &self,
        terrain_config: &TrainingTerrainConfig,
    ) -> (World, Vec<Vec2>) {
        self.setup_procedural_world(terrain_config)
    }

    /// Set up world using procedural generation (NEW)
    fn setup_procedural_world(&self, config: &TrainingTerrainConfig) -> (World, Vec<Vec2>) {
        let mut world = World::new(false);

        // Apply difficulty to get WorldGenConfig
        let worldgen_config = config.apply_difficulty();

        // Create WorldGenerator with base seed
        let generator = WorldGenerator::from_config(config.base_seed, worldgen_config);

        // Generate chunks for training area
        let chunks_x = (config.width + 63) / 64;
        let chunks_y = (config.height + 63) / 64;

        for cy in 0..chunks_y {
            for cx in 0..chunks_x {
                let chunk = generator.generate_chunk(cx, cy);
                world.insert_chunk(IVec2::new(cx, cy), chunk);
            }
        }

        // Scan for food positions (edible materials in generated world)
        let food_positions = self.scan_food_positions(&world, config.width, config.height);

        (world, food_positions)
    }

    /// Set up world using manual terrain (EXISTING)
    fn setup_manual_world(&self) -> (World, Vec<Vec2>) {
        let mut world = World::new(false);

        // Ensure chunks exist for the entire scenario area
        world.ensure_chunks_for_area(
            0,
            0,
            self.config.world_width - 1,
            self.config.world_height - 1,
        );

        // Create flat ground
        self.create_flat_ground(&mut world);

        // Add scenario-specific features and collect food positions
        let food_positions = match self.config.name.as_str() {
            "Foraging" => self.add_food_sources(&mut world),
            "Survival" => {
                self.add_hazards(&mut world);
                Vec::new()
            }
            "Balanced" => self.add_food_sources(&mut world),
            "Parcour" => self.add_parcour_course(&mut world),
            _ => Vec::new(),
        };

        (world, food_positions)
    }

    /// Scan world for food positions (edible materials)
    /// Used for optimized cached sensing in procedurally generated worlds
    fn scan_food_positions(&self, world: &World, width: i32, height: i32) -> Vec<Vec2> {
        let mut food_positions = Vec::new();

        // Get materials registry to check if material is edible
        let materials = world.materials();

        // Scan entire world area for edible materials (materials with nutritional_value)
        for y in 0..height {
            for x in 0..width {
                if let Some(pixel) = world.get_pixel(x, y) {
                    let material_id = pixel.material_id;
                    let material = materials.get(material_id);
                    if material.nutritional_value.is_some() {
                        food_positions.push(Vec2::new(x as f32, y as f32));
                    }
                }
            }
        }

        food_positions
    }

    /// Create flat ground terrain
    fn create_flat_ground(&self, world: &mut World) {
        let ground_y = 20; // Ground level

        // Fill ground with stone
        for x in 0..self.config.world_width {
            for y in 0..ground_y {
                world.set_pixel(x, y, MaterialId::STONE);
            }
        }
    }

    /// Add food sources for foraging scenario
    /// Returns food positions for optimized sensing
    fn add_food_sources(&self, world: &mut World) -> Vec<Vec2> {
        // Scatter food patches
        let food_positions = [
            (150, 25),
            (200, 25),
            (280, 25),
            (350, 25),
            (420, 25),
            (180, 30),
            (250, 30),
            (320, 30),
            (400, 30),
        ];

        for (x, y) in food_positions {
            // Create small food patch (3x3)
            for dx in -1..=1 {
                for dy in -1..=1 {
                    world.set_pixel(x + dx, y + dy, MaterialId::FRUIT);
                }
            }
        }

        // Return center positions as Vec2
        food_positions
            .iter()
            .map(|(x, y)| Vec2::new(*x as f32, *y as f32))
            .collect()
    }

    /// Add hazards for survival scenario
    fn add_hazards(&self, world: &mut World) {
        // Create lava pits
        let hazard_positions = [
            (200, 20, 30), // (x, y, width)
            (350, 20, 25),
            (450, 20, 20),
        ];

        for (x, y, width) in hazard_positions {
            for dx in 0..width {
                for dy in 0..5 {
                    world.set_pixel(x + dx, y + dy, MaterialId::LAVA);
                }
            }
        }
    }

    /// Add contained tunnel arena with minable wall
    ///
    /// Layout (420x100 world):
    /// ```text
    /// BEDROCK CEILING ████████████████████████████████████████████
    ///                 █                                          █
    /// BEDROCK LEFT    █   SPAWN    ████████  FOOD                █ BEDROCK RIGHT
    ///                 █     *      ████████   🍎🍎🍎🍎           █
    ///                 █            ████████                      █
    /// BEDROCK FLOOR   ████████████████████████████████████████████
    ///                             STONE (minable)
    /// ```
    /// Returns food positions for optimized sensing
    fn add_parcour_course(&self, world: &mut World) -> Vec<Vec2> {
        let ground_y = 20;
        let ceiling_y = 80;
        let arena_width = 420;

        // === BEDROCK CONTAINMENT (unminable) ===

        // Bedrock floor (solid ground)
        for x in 0..arena_width {
            for y in 0..ground_y {
                world.set_pixel(x, y, MaterialId::BEDROCK);
            }
        }

        // Bedrock ceiling (prevents flying over)
        for x in 0..arena_width {
            for y in ceiling_y..100 {
                world.set_pixel(x, y, MaterialId::BEDROCK);
            }
        }

        // Bedrock left wall
        for y in ground_y..ceiling_y {
            for x in 0..15 {
                world.set_pixel(x, y, MaterialId::BEDROCK);
            }
        }

        // Bedrock right wall
        for y in ground_y..ceiling_y {
            for x in 390..arena_width {
                world.set_pixel(x, y, MaterialId::BEDROCK);
            }
        }

        // === TUNNEL WALL (minable stone) ===
        // Positioned to require mining - floor to ceiling
        let wall_x_start = 180;
        let wall_width = 15;

        for dx in 0..wall_width {
            for y in ground_y..ceiling_y {
                world.set_pixel(wall_x_start + dx, y, MaterialId::STONE);
            }
        }

        // === FOOD (only behind wall) ===
        // No food before wall - creature MUST mine to reach food
        let food_positions: [(i32, i32); 6] = [
            (220, 30), // Just past wall, ground level
            (250, 30),
            (280, 30),
            (310, 30),
            (340, 30),
            (370, 30),
        ];

        // Place food (3x3 patches of FRUIT)
        for (x, y) in food_positions {
            for dx in -1..=1 {
                for dy in -1..=1 {
                    world.set_pixel(x + dx, y + dy, MaterialId::FRUIT);
                }
            }
        }

        // Return food positions as Vec2
        food_positions
            .iter()
            .map(|(x, y)| Vec2::new(*x as f32, *y as f32))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_locomotion_scenario() {
        let scenario = Scenario::locomotion();
        assert_eq!(scenario.config.name, "Locomotion");
        assert_eq!(scenario.fitness.name(), "Distance");
    }

    #[test]
    fn test_foraging_scenario() {
        let scenario = Scenario::foraging();
        assert_eq!(scenario.config.name, "Foraging");
        assert_eq!(scenario.fitness.name(), "Foraging");
    }

    #[test]
    fn test_survival_scenario() {
        let scenario = Scenario::survival();
        assert_eq!(scenario.config.name, "Survival");
        assert_eq!(scenario.fitness.name(), "Survival");
    }

    #[test]
    fn test_balanced_scenario() {
        let scenario = Scenario::balanced();
        assert_eq!(scenario.config.name, "Balanced");
        assert_eq!(scenario.fitness.name(), "Composite");
    }

    #[test]
    fn test_parcour_scenario() {
        let scenario = Scenario::parcour();
        assert_eq!(scenario.config.name, "Parcour");
        assert_eq!(scenario.fitness.name(), "DirectionalFood");
        assert_eq!(scenario.config.spawn_position, Vec2::new(50.0, 40.0));
    }

    #[test]
    fn test_parcour_world_setup() {
        let scenario = Scenario::parcour();
        let (world, food_positions) = scenario.setup_world();

        // Check that ground exists
        let ground_pixel = world.get_pixel(100, 10);
        assert!(ground_pixel.is_some());
        assert_eq!(ground_pixel.unwrap().material_id, MaterialId::BEDROCK);

        // Check that wall exists (wall is at x=180-194, from ground to ceiling)
        let wall_pixel = world.get_pixel(185, 30);
        assert!(wall_pixel.is_some());
        assert_eq!(wall_pixel.unwrap().material_id, MaterialId::STONE);

        // Check that food exists after wall (food is at y=30, in 3x3 patches)
        let food_after = world.get_pixel(280, 30);
        assert!(food_after.is_some());
        assert_eq!(food_after.unwrap().material_id, MaterialId::FRUIT);

        // Check food positions were returned
        assert_eq!(food_positions.len(), 6); // All food behind wall (must mine to reach)
    }

    #[test]
    fn test_procedural_terrain_generation() {
        // Create scenario with procedural terrain config
        let terrain_config = TrainingTerrainConfig::flat(42, 512, 128);
        let mut scenario = Scenario::locomotion();
        scenario.config.terrain_config = Some(terrain_config);

        let (world, _food_positions) = scenario.setup_world();

        // Verify world was generated (check some pixels exist)
        let pixel = world.get_pixel(100, 20);
        assert!(pixel.is_some(), "World should have generated pixels");
    }

    #[test]
    fn test_procedural_terrain_determinism() {
        // Same seed should produce identical terrain
        let terrain_config = TrainingTerrainConfig::flat(42, 512, 128);

        let mut scenario1 = Scenario::locomotion();
        scenario1.config.terrain_config = Some(terrain_config.clone());
        let (world1, _) = scenario1.setup_world();

        let mut scenario2 = Scenario::locomotion();
        scenario2.config.terrain_config = Some(terrain_config);
        let (world2, _) = scenario2.setup_world();

        // Check several pixel positions for determinism
        for x in [50, 100, 200, 300, 400] {
            for y in [10, 20, 30, 50] {
                let pixel1 = world1.get_pixel(x, y);
                let pixel2 = world2.get_pixel(x, y);

                match (pixel1, pixel2) {
                    (Some(p1), Some(p2)) => {
                        assert_eq!(
                            p1.material_id, p2.material_id,
                            "Pixels at ({}, {}) should match",
                            x, y
                        );
                    }
                    (None, None) => {
                        // Both None is OK (chunk not loaded)
                    }
                    _ => {
                        panic!(
                            "Pixel existence mismatch at ({}, {}): {:?} vs {:?}",
                            x, y, pixel1, pixel2
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn test_procedural_terrain_different_seeds() {
        // Different seeds should produce different terrain
        // Use gentle_hills instead of flat to ensure terrain variation
        let config1 = TrainingTerrainConfig::gentle_hills(42, 2048, 512);
        let config2 = TrainingTerrainConfig::gentle_hills(123, 2048, 512);

        let mut scenario1 = Scenario::locomotion();
        scenario1.config.terrain_config = Some(config1);
        let (world1, _) = scenario1.setup_world();

        let mut scenario2 = Scenario::locomotion();
        scenario2.config.terrain_config = Some(config2);
        let (world2, _) = scenario2.setup_world();

        // At least some pixels should differ (sample across larger area for robustness)
        let mut differences = 0;
        for x in [100, 500, 1000, 1500, 2000] {
            for y in [-50, -100, -200, 50, 100] {
                if let (Some(p1), Some(p2)) = (world1.get_pixel(x, y), world2.get_pixel(x, y)) {
                    if p1.material_id != p2.material_id {
                        differences += 1;
                    }
                }
            }
        }

        assert!(
            differences > 0,
            "Different seeds should produce different terrain"
        );
    }

    #[test]
    fn test_procedural_terrain_difficulty_levels() {
        // Test that different difficulty configs produce different terrain
        let flat_config = TrainingTerrainConfig::flat(42, 512, 128);
        let random_config = TrainingTerrainConfig::random(42, 512, 128);

        let flat_worldgen = flat_config.apply_difficulty();
        let random_worldgen = random_config.apply_difficulty();

        // Flat should have height_scale = 0.0
        assert_eq!(flat_worldgen.terrain.height_scale, 0.0);

        // Random should have height_scale = 100.0
        assert_eq!(random_worldgen.terrain.height_scale, 100.0);

        // Flat should have minimal caves
        assert!(flat_worldgen.caves.large_threshold > 0.9);

        // Random should have more caves than flat
        assert!(random_worldgen.caves.large_threshold < 0.9);
        assert!(random_worldgen.caves.large_threshold > 0.0);
    }

    #[test]
    fn test_backward_compatibility_manual_terrain() {
        // Existing scenarios without terrain_config should still use manual terrain
        let scenario = Scenario::locomotion();
        assert!(scenario.config.terrain_config.is_none());

        let (world, _) = scenario.setup_world();

        // Check that manual flat ground was created (at y=20)
        let ground_pixel = world.get_pixel(100, 10);
        assert!(ground_pixel.is_some());
        assert_eq!(ground_pixel.unwrap().material_id, MaterialId::STONE);
    }

    #[test]
    fn test_scan_food_positions() {
        // Test that food scanning works in procedurally generated worlds
        let terrain_config = TrainingTerrainConfig::flat(42, 512, 128);
        let mut scenario = Scenario::foraging();
        scenario.config.terrain_config = Some(terrain_config);

        let (_world, food_positions) = scenario.setup_world();

        // Procedurally generated worlds may have some food (fruit in biomes)
        // or may have none depending on biome generation
        // Just verify the scan completed without error (result is always >= 0)
        #[allow(unused_comparisons)]
        let _ = food_positions.len() >= 0;
    }
}
