//! Training scenarios for creature evolution
//!
//! Each scenario defines a world configuration and evaluation criteria.

use glam::Vec2;

use crate::simulation::MaterialId;
use crate::world::World;

use super::fitness::{
    CompositeFitness, DistanceFitness, FitnessFunction, FoodCollectionFitness, ForagingFitness,
    SurvivalFitness,
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
    pub fn parcour() -> Self {
        Self {
            config: ScenarioConfig {
                name: "Parcour".to_string(),
                description: "Food parcour with wall obstacle requiring mining".to_string(),
                expected_behavior: "Navigate to food, mine through wall".to_string(),
                spawn_position: Vec2::new(50.0, 50.0),
                eval_duration: 30.0,
                world_width: 400,
                world_height: 128,
            },
            fitness: Box::new(FoodCollectionFitness::new()),
        }
    }

    /// Set up the world for this scenario
    /// Returns the world and a list of food positions for optimized sensing
    pub fn setup_world(&self) -> (World, Vec<Vec2>) {
        let mut world = World::new();

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

    /// Add food parcour course with wall obstacle
    ///
    /// Layout (ground at y=20):
    /// ```text
    /// Spawn(50)  Food1(100) Food2(150) Food3(200)  WALL(250-260)  Food4(280) Food5(310) Food6(340) Food7(360) Food8(380)
    ///    *          o          o          o        ████████████      o         o          o          o          o
    /// ___________________________________________________________________________________________________
    /// ```
    /// Returns food positions for optimized sensing
    fn add_parcour_course(&self, world: &mut World) -> Vec<Vec2> {
        let ground_y = 20;
        let food_y = 25; // Slightly above ground

        // 3 food items before the wall (reachable by walking)
        let food_positions_before_wall: [(i32, i32); 3] =
            [(100, food_y), (150, food_y), (200, food_y)];

        // Stone wall at x=250-260, from ground up to y=50
        let wall_x_start = 250;
        let wall_width = 10;
        let wall_height = 30;

        // 5 food items after the wall
        let food_positions_after_wall: [(i32, i32); 5] = [
            (280, food_y),
            (310, food_y),
            (340, food_y),
            (360, food_y),
            (380, food_y),
        ];

        // Place food before wall (3x3 patches of FRUIT)
        for (x, y) in food_positions_before_wall {
            for dx in -1..=1 {
                for dy in -1..=1 {
                    world.set_pixel(x + dx, y + dy, MaterialId::FRUIT);
                }
            }
        }

        // Create stone wall
        for dx in 0..wall_width {
            for dy in 0..wall_height {
                world.set_pixel(wall_x_start + dx, ground_y + dy, MaterialId::STONE);
            }
        }

        // Place food after wall (3x3 patches of FRUIT)
        for (x, y) in food_positions_after_wall {
            for dx in -1..=1 {
                for dy in -1..=1 {
                    world.set_pixel(x + dx, y + dy, MaterialId::FRUIT);
                }
            }
        }

        // Return all food positions as Vec2
        food_positions_before_wall
            .iter()
            .chain(food_positions_after_wall.iter())
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
        assert_eq!(scenario.fitness.name(), "FoodCollection");
        assert_eq!(scenario.config.spawn_position, Vec2::new(50.0, 50.0));
    }

    #[test]
    fn test_parcour_world_setup() {
        let scenario = Scenario::parcour();
        let (world, food_positions) = scenario.setup_world();

        // Check that ground exists
        let ground_pixel = world.get_pixel(100, 10);
        assert!(ground_pixel.is_some());
        assert_eq!(ground_pixel.unwrap().material_id, MaterialId::STONE);

        // Check that food exists before wall
        let food_pixel = world.get_pixel(100, 25);
        assert!(food_pixel.is_some());
        assert_eq!(food_pixel.unwrap().material_id, MaterialId::FRUIT);

        // Check that wall exists
        let wall_pixel = world.get_pixel(255, 30);
        assert!(wall_pixel.is_some());
        assert_eq!(wall_pixel.unwrap().material_id, MaterialId::STONE);

        // Check that food exists after wall
        let food_after = world.get_pixel(280, 25);
        assert!(food_after.is_some());
        assert_eq!(food_after.unwrap().material_id, MaterialId::FRUIT);

        // Check food positions were returned
        assert_eq!(food_positions.len(), 8); // 3 before + 5 after wall
    }
}
