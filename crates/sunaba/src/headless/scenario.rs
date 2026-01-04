//! Training scenarios for creature evolution
//!
//! Each scenario defines a world configuration and evaluation criteria.

use glam::Vec2;

use crate::simulation::MaterialId;
use crate::world::World;

use super::fitness::{
    CompositeFitness, DirectionalFoodFitness, DistanceFitness, FitnessFunction, ForagingFitness,
    MovementFitness, SurvivalFitness,
};

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
            },
            fitness: Box::new(DirectionalFoodFitness::parcour()),
        }
    }

    /// Set up the world for this scenario
    /// Returns the world and a list of food positions for optimized sensing
    pub fn setup_world(&self) -> (World, Vec<Vec2>) {
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
}
