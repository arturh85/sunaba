//! Fitness functions for evaluating creature performance
//!
//! Different fitness functions measure different aspects of creature behavior.

use glam::Vec2;

use crate::creature::Creature;
use crate::world::World;

/// Trait for fitness evaluation functions
pub trait FitnessFunction: Send + Sync {
    /// Evaluate creature fitness based on its performance
    fn evaluate(&self, creature: &Creature, world: &World, spawn_pos: Vec2, duration: f32) -> f32;

    /// Get the name of this fitness function
    fn name(&self) -> &str;

    /// Get a description of what this fitness measures
    fn description(&self) -> &str;
}

/// Measures actual movement with motor activity bonus
/// Designed to reward creatures that ACTUALLY MOVE, not just exist
pub struct MovementFitness {
    /// Weight for displacement score (pixels traveled)
    pub displacement_weight: f32,
    /// Minimum displacement to be considered "moving"
    pub min_displacement: f32,
    /// Penalty multiplier for not moving
    pub stationary_penalty: f32,
}

impl MovementFitness {
    pub fn new() -> Self {
        Self {
            displacement_weight: 10.0, // 10 points per pixel moved
            min_displacement: 5.0,     // Must move at least 5 pixels
            stationary_penalty: 0.1,   // 90% penalty for not moving
        }
    }
}

impl Default for MovementFitness {
    fn default() -> Self {
        Self::new()
    }
}

impl FitnessFunction for MovementFitness {
    fn evaluate(&self, creature: &Creature, _world: &World, spawn_pos: Vec2, duration: f32) -> f32 {
        let displacement = (creature.position - spawn_pos).length();

        // Primary fitness: displacement (heavily weighted)
        let movement_score = displacement * self.displacement_weight;

        // Survival bonus: only if creature moved
        let survival_bonus =
            if displacement >= self.min_displacement && creature.health.current > 0.0 {
                duration * 2.0 // 2 points per second survived while moving
            } else {
                0.0
            };

        // Penalty for not moving
        if displacement < self.min_displacement {
            (movement_score + survival_bonus) * self.stationary_penalty
        } else {
            movement_score + survival_bonus
        }
    }

    fn name(&self) -> &str {
        "Movement"
    }

    fn description(&self) -> &str {
        "Heavily rewards actual displacement, penalizes stationary creatures"
    }
}

/// Measures distance traveled from spawn point
pub struct DistanceFitness;

impl FitnessFunction for DistanceFitness {
    fn evaluate(
        &self,
        creature: &Creature,
        _world: &World,
        spawn_pos: Vec2,
        _duration: f32,
    ) -> f32 {
        let distance = (creature.position - spawn_pos).length();
        distance
    }

    fn name(&self) -> &str {
        "Distance"
    }

    fn description(&self) -> &str {
        "Measures horizontal/vertical distance traveled from spawn point"
    }
}

/// Measures total nutrition consumed
pub struct ForagingFitness;

impl FitnessFunction for ForagingFitness {
    fn evaluate(
        &self,
        creature: &Creature,
        _world: &World,
        _spawn_pos: Vec2,
        _duration: f32,
    ) -> f32 {
        // Use energy as proxy for nutrition (creatures gain energy from eating)
        creature.needs.energy
    }

    fn name(&self) -> &str {
        "Foraging"
    }

    fn description(&self) -> &str {
        "Measures total nutrition/energy gained from foraging"
    }
}

/// Measures survival time
pub struct SurvivalFitness;

impl FitnessFunction for SurvivalFitness {
    fn evaluate(
        &self,
        creature: &Creature,
        _world: &World,
        _spawn_pos: Vec2,
        duration: f32,
    ) -> f32 {
        // If creature survived the full duration, use duration
        // Otherwise, could track death time (not currently stored)
        if creature.health.current > 0.0 {
            duration
        } else {
            // Creature died - would need to track when
            duration * 0.5 // Penalty for dying
        }
    }

    fn name(&self) -> &str {
        "Survival"
    }

    fn description(&self) -> &str {
        "Measures time survived during evaluation"
    }
}

/// Measures food collected with distance bonus
/// Currently used in tests; kept for potential future scenario use
#[allow(dead_code)]
pub struct FoodCollectionFitness {
    /// Points per food item collected
    pub food_points: f32,
    /// Weight for distance bonus
    pub distance_bonus_weight: f32,
}

impl FoodCollectionFitness {
    /// Create with default values
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self {
            food_points: 10.0,
            distance_bonus_weight: 0.5,
        }
    }
}

impl Default for FoodCollectionFitness {
    fn default() -> Self {
        Self::new()
    }
}

impl FitnessFunction for FoodCollectionFitness {
    fn evaluate(
        &self,
        creature: &Creature,
        _world: &World,
        spawn_pos: Vec2,
        _duration: f32,
    ) -> f32 {
        // Main fitness: food collected
        let food_score = creature.food_eaten as f32 * self.food_points;

        // Bonus for distance traveled (encourages exploration)
        let distance = (creature.position - spawn_pos).length();
        let distance_bonus = (distance / 100.0) * self.distance_bonus_weight;

        food_score + distance_bonus
    }

    fn name(&self) -> &str {
        "FoodCollection"
    }

    fn description(&self) -> &str {
        "Measures food items collected with small distance bonus"
    }
}

/// Measures food collected with DIRECTIONAL bonus toward target
///
/// Unlike FoodCollectionFitness which rewards any movement, this fitness
/// specifically rewards movement toward a target direction (e.g., toward food).
/// Uses dot product to calculate directional progress.
pub struct DirectionalFoodFitness {
    /// Points per food item collected
    pub food_points: f32,
    /// Weight for directional movement bonus
    pub direction_bonus_weight: f32,
    /// Target direction for movement (normalized)
    /// Default: positive X direction (right)
    pub target_direction: Vec2,
    /// Points per pixel moved in correct direction
    pub movement_points: f32,
    /// Penalty for moving in wrong direction
    pub wrong_direction_penalty: f32,
    /// Points per block mined (rewards learning to mine through obstacles)
    pub mining_points: f32,
}

impl DirectionalFoodFitness {
    /// Create with default values (food to the right)
    pub fn new() -> Self {
        Self {
            food_points: 50.0,            // High reward for eating food
            direction_bonus_weight: 1.0,  // Full weight for directional bonus
            target_direction: Vec2::X,    // Food is to the RIGHT (positive X)
            movement_points: 1.0,         // 1 point per pixel in correct direction
            wrong_direction_penalty: 0.5, // 50% penalty for wrong direction
            mining_points: 2.0,           // Reward for mining through obstacles
        }
    }

    /// Create for parcour scenario - tuned to encourage mining
    ///
    /// Key changes from default:
    /// - Higher food reward (100) - food is the main goal
    /// - Lower movement bonus (0.2) - movement alone not enough
    /// - Higher mining reward (5.0) - mining is valuable
    pub fn parcour() -> Self {
        Self {
            food_points: 100.0,           // Food is king - high reward
            direction_bonus_weight: 0.2,  // Reduced - movement alone not enough
            target_direction: Vec2::X,    // Food is to the RIGHT
            movement_points: 0.5,         // Small - provides gradient signal only
            wrong_direction_penalty: 0.5, // 50% penalty for wrong direction
            mining_points: 5.0,           // High - mining is valuable
        }
    }
}

impl Default for DirectionalFoodFitness {
    fn default() -> Self {
        Self::new()
    }
}

impl FitnessFunction for DirectionalFoodFitness {
    fn evaluate(
        &self,
        creature: &Creature,
        _world: &World,
        spawn_pos: Vec2,
        _duration: f32,
    ) -> f32 {
        // Primary fitness: food collected (heavily weighted)
        let food_score = creature.food_eaten as f32 * self.food_points;

        // Mining score: reward for mining through obstacles
        let mining_score = creature.blocks_mined as f32 * self.mining_points;

        // Calculate movement vector
        let movement = creature.position - spawn_pos;
        let displacement = movement.length();

        // Calculate directional progress using dot product
        // Positive when moving toward target, negative when moving away
        let directional_progress = if displacement > 0.1 {
            movement.dot(self.target_direction)
        } else {
            0.0
        };

        // Reward or penalize based on direction
        let direction_score = if directional_progress > 0.0 {
            // Moving in correct direction: full reward
            directional_progress * self.movement_points * self.direction_bonus_weight
        } else {
            // Moving in wrong direction: reduced reward (or penalty)
            directional_progress * self.movement_points * self.wrong_direction_penalty
        };

        // Combined fitness
        // - Food is king (high points)
        // - Directional movement provides gradient signal
        // - Mining rewards breaking through obstacles
        let total = food_score + direction_score + mining_score;

        // Ensure non-negative fitness
        total.max(0.0)
    }

    fn name(&self) -> &str {
        "DirectionalFood"
    }

    fn description(&self) -> &str {
        "Rewards food collection and directional movement toward target (positive X)"
    }
}

/// Combines multiple fitness functions with weights
pub struct CompositeFitness {
    /// Component fitness functions with their weights
    pub components: Vec<(Box<dyn FitnessFunction>, f32)>,
}

impl CompositeFitness {
    /// Create a new composite fitness with given components and weights
    pub fn new(components: Vec<(Box<dyn FitnessFunction>, f32)>) -> Self {
        Self { components }
    }

    /// Create a balanced fitness combining distance and survival
    pub fn balanced() -> Self {
        Self::new(vec![
            (Box::new(DistanceFitness), 1.0),
            (Box::new(SurvivalFitness), 0.5),
            (Box::new(ForagingFitness), 0.3),
        ])
    }
}

impl FitnessFunction for CompositeFitness {
    fn evaluate(&self, creature: &Creature, world: &World, spawn_pos: Vec2, duration: f32) -> f32 {
        let mut total = 0.0;
        let mut total_weight = 0.0;

        for (func, weight) in &self.components {
            let score = func.evaluate(creature, world, spawn_pos, duration);
            total += score * weight;
            total_weight += weight;
        }

        if total_weight > 0.0 {
            total / total_weight
        } else {
            0.0
        }
    }

    fn name(&self) -> &str {
        "Composite"
    }

    fn description(&self) -> &str {
        "Weighted combination of multiple fitness metrics"
    }
}

/// Behavioral metrics for MAP-Elites dimensions
#[derive(Debug, Clone)]
pub struct BehaviorDescriptor {
    /// Locomotion efficiency (distance / energy spent)
    pub locomotion_efficiency: f32,
    /// Foraging efficiency (nutrition / time)
    pub foraging_efficiency: f32,
    /// Exploration (unique areas visited) - normalized 0-1
    pub exploration: f32,
    /// Activity level (actions per second)
    pub activity: f32,
}

impl BehaviorDescriptor {
    /// Create from creature evaluation results
    pub fn from_evaluation(
        creature: &Creature,
        spawn_pos: Vec2,
        duration: f32,
        _world: &World,
    ) -> Self {
        let distance = (creature.position - spawn_pos).length();
        let current_energy = creature.needs.energy;
        let energy_spent: f32 = 1.0 - current_energy; // Energy is 0-1, starting at 1

        Self {
            locomotion_efficiency: if energy_spent > 0.0 {
                distance / energy_spent.max(0.01)
            } else {
                distance
            },
            foraging_efficiency: current_energy / duration.max(1.0),
            exploration: (distance / 100.0).min(1.0), // Normalize to 0-1
            activity: 1.0, // Would need action tracking to compute properly
        }
    }

    /// Get the value for a specific dimension index (for MAP-Elites grid)
    pub fn get_dimension(&self, dim: usize) -> f32 {
        match dim {
            0 => self.locomotion_efficiency,
            1 => self.foraging_efficiency,
            2 => self.exploration,
            3 => self.activity,
            _ => 0.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_creature(position: Vec2, energy: f32, health: f32) -> Creature {
        use crate::creature::genome::CreatureGenome;
        let mut creature = Creature::from_genome(CreatureGenome::test_biped(), Vec2::ZERO);
        creature.position = position;
        creature.needs.energy = energy;
        creature.health.current = health;
        creature
    }

    fn make_test_creature_with_food(position: Vec2, food_eaten: u32) -> Creature {
        use crate::creature::genome::CreatureGenome;
        let mut creature = Creature::from_genome(CreatureGenome::test_biped(), Vec2::ZERO);
        creature.position = position;
        creature.food_eaten = food_eaten;
        creature
    }

    #[test]
    fn test_distance_fitness() {
        let fitness = DistanceFitness;
        let spawn = Vec2::ZERO;
        let creature = make_test_creature(Vec2::new(100.0, 0.0), 50.0, 100.0);
        let world = World::new(false);

        let score = fitness.evaluate(&creature, &world, spawn, 30.0);
        assert!((score - 100.0).abs() < 0.01);
    }

    #[test]
    fn test_foraging_fitness() {
        let fitness = ForagingFitness;
        let creature = make_test_creature(Vec2::ZERO, 75.0, 100.0);
        let world = World::new(false);

        let score = fitness.evaluate(&creature, &world, Vec2::ZERO, 30.0);
        assert!((score - 75.0).abs() < 0.01);
    }

    #[test]
    fn test_survival_fitness() {
        let fitness = SurvivalFitness;
        let world = World::new(false);

        // Alive creature
        let alive = make_test_creature(Vec2::ZERO, 50.0, 100.0);
        let score_alive = fitness.evaluate(&alive, &world, Vec2::ZERO, 30.0);
        assert!((score_alive - 30.0).abs() < 0.01);

        // Dead creature
        let dead = make_test_creature(Vec2::ZERO, 0.0, 0.0);
        let score_dead = fitness.evaluate(&dead, &world, Vec2::ZERO, 30.0);
        assert!(score_dead < score_alive);
    }

    #[test]
    fn test_composite_fitness() {
        let fitness = CompositeFitness::balanced();
        let creature = make_test_creature(Vec2::new(50.0, 0.0), 75.0, 100.0);
        let world = World::new(false);

        let score = fitness.evaluate(&creature, &world, Vec2::ZERO, 30.0);
        assert!(score > 0.0);
    }

    #[test]
    fn test_food_collection_fitness() {
        let fitness = FoodCollectionFitness::new();
        let world = World::new(false);

        // Creature with no food at spawn
        let creature_no_food = make_test_creature_with_food(Vec2::ZERO, 0);
        let score_no_food = fitness.evaluate(&creature_no_food, &world, Vec2::ZERO, 30.0);
        assert!(score_no_food < 0.01);

        // Creature with 3 food items
        let creature_with_food = make_test_creature_with_food(Vec2::ZERO, 3);
        let score_with_food = fitness.evaluate(&creature_with_food, &world, Vec2::ZERO, 30.0);
        assert!((score_with_food - 30.0).abs() < 0.01); // 3 * 10 = 30

        // Creature with food and distance bonus
        let creature_far = make_test_creature_with_food(Vec2::new(100.0, 0.0), 2);
        let score_far = fitness.evaluate(&creature_far, &world, Vec2::ZERO, 30.0);
        // 2 * 10 + (100/100) * 0.5 = 20.5
        assert!((score_far - 20.5).abs() < 0.01);
    }

    #[test]
    fn test_directional_food_fitness_right_direction() {
        let fitness = DirectionalFoodFitness::new();
        let world = World::new(false);
        let spawn = Vec2::ZERO;

        // Creature moved right (correct direction) with no food
        let creature_right = make_test_creature_with_food(Vec2::new(100.0, 0.0), 0);
        let score_right = fitness.evaluate(&creature_right, &world, spawn, 30.0);

        // Creature moved left (wrong direction) with no food
        let creature_left = make_test_creature_with_food(Vec2::new(-100.0, 0.0), 0);
        let score_left = fitness.evaluate(&creature_left, &world, spawn, 30.0);

        // Right should have HIGHER fitness than left
        assert!(
            score_right > score_left,
            "Right ({}) should be > Left ({})",
            score_right,
            score_left
        );

        // Right direction should give positive score
        // 100 pixels * 1.0 movement_points * 1.0 direction_weight = 100
        assert!(
            (score_right - 100.0).abs() < 0.01,
            "Expected 100, got {}",
            score_right
        );

        // Left direction should give reduced/negative score (but clamped to 0)
        // -100 * 1.0 * 0.5 = -50, clamped to 0
        assert!(score_left < 0.01, "Expected near 0, got {}", score_left);
    }

    #[test]
    fn test_directional_food_fitness_with_food() {
        let fitness = DirectionalFoodFitness::new();
        let world = World::new(false);
        let spawn = Vec2::ZERO;

        // Creature moved right and ate food
        let creature_right_food = make_test_creature_with_food(Vec2::new(50.0, 0.0), 2);
        let score = fitness.evaluate(&creature_right_food, &world, spawn, 30.0);

        // 2 food * 50 points + 50 pixels * 1.0 = 100 + 50 = 150
        assert!((score - 150.0).abs() < 0.01, "Expected 150, got {}", score);
    }

    #[test]
    fn test_directional_vs_nondirectional_fitness() {
        let dir_fitness = DirectionalFoodFitness::new();
        let old_fitness = FoodCollectionFitness::new();
        let world = World::new(false);
        let spawn = Vec2::new(50.0, 50.0); // Parcour spawn position

        // Champion that moved 2400px LEFT (the observed behavior)
        let creature_left = make_test_creature_with_food(Vec2::new(-2350.0, 29.0), 0);
        let dir_score_left = dir_fitness.evaluate(&creature_left, &world, spawn, 30.0);
        let old_score_left = old_fitness.evaluate(&creature_left, &world, spawn, 30.0);

        // Creature that moved 100px RIGHT (correct direction)
        let creature_right = make_test_creature_with_food(Vec2::new(150.0, 50.0), 0);
        let dir_score_right = dir_fitness.evaluate(&creature_right, &world, spawn, 30.0);
        let old_score_right = old_fitness.evaluate(&creature_right, &world, spawn, 30.0);

        // With OLD fitness: LEFT has higher score due to larger displacement
        assert!(
            old_score_left > old_score_right,
            "OLD: Left ({}) should be > Right ({}) due to pure distance",
            old_score_left,
            old_score_right
        );

        // With NEW directional fitness: RIGHT should have higher score
        assert!(
            dir_score_right > dir_score_left,
            "NEW: Right ({}) should be > Left ({}) due to direction",
            dir_score_right,
            dir_score_left
        );

        println!(
            "OLD fitness: left={:.1}, right={:.1}",
            old_score_left, old_score_right
        );
        println!(
            "NEW fitness: left={:.1}, right={:.1}",
            dir_score_left, dir_score_right
        );
    }
}
