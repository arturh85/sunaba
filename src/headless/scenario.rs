//! Training scenarios for creature evolution
//!
//! Each scenario defines a world configuration and evaluation criteria.

use glam::Vec2;

use crate::simulation::MaterialId;
use crate::world::World;

use super::fitness::{
    CompositeFitness, DistanceFitness, FitnessFunction, ForagingFitness, SurvivalFitness,
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

    /// Set up the world for this scenario
    pub fn setup_world(&self) -> World {
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

        // Add scenario-specific features
        match self.config.name.as_str() {
            "Foraging" => self.add_food_sources(&mut world),
            "Survival" => self.add_hazards(&mut world),
            "Balanced" => {
                self.add_food_sources(&mut world);
            }
            _ => {}
        }

        world
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
    fn add_food_sources(&self, world: &mut World) {
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
}
