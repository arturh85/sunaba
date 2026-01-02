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
pub struct FoodCollectionFitness {
    /// Points per food item collected
    pub food_points: f32,
    /// Weight for distance bonus
    pub distance_bonus_weight: f32,
}

impl FoodCollectionFitness {
    /// Create with default values
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
        let world = World::new();

        let score = fitness.evaluate(&creature, &world, spawn, 30.0);
        assert!((score - 100.0).abs() < 0.01);
    }

    #[test]
    fn test_foraging_fitness() {
        let fitness = ForagingFitness;
        let creature = make_test_creature(Vec2::ZERO, 75.0, 100.0);
        let world = World::new();

        let score = fitness.evaluate(&creature, &world, Vec2::ZERO, 30.0);
        assert!((score - 75.0).abs() < 0.01);
    }

    #[test]
    fn test_survival_fitness() {
        let fitness = SurvivalFitness;
        let world = World::new();

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
        let world = World::new();

        let score = fitness.evaluate(&creature, &world, Vec2::ZERO, 30.0);
        assert!(score > 0.0);
    }

    #[test]
    fn test_food_collection_fitness() {
        let fitness = FoodCollectionFitness::new();
        let world = World::new();

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
}
