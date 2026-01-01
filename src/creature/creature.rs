//! Main creature entity
//!
//! Combines genome, morphology, neural control, and behavior.

use glam::Vec2;
use serde::{Deserialize, Serialize};

use crate::entity::{health::Health, health::Hunger, EntityId};

use super::behavior::{CreatureAction, CreatureNeeds, GoalPlanner};
use super::genome::CreatureGenome;
use super::morphology::{CreatureMorphology, MorphologyPhysics};
use super::neural::SimpleNeuralController;
use super::sensors::{SensorConfig, SensoryInput};

/// Main creature entity
#[derive(Serialize, Deserialize)]
pub struct Creature {
    pub id: EntityId,
    pub genome: CreatureGenome,
    pub morphology: CreatureMorphology,

    #[serde(skip)] // Rebuilt from morphology on load
    pub physics: Option<MorphologyPhysics>,

    pub health: Health,
    pub hunger: Hunger,
    pub needs: CreatureNeeds,

    #[serde(skip)] // Rebuilt on load
    pub planner: Option<GoalPlanner>,

    #[serde(skip)] // Rebuilt from genome on load
    pub brain: Option<SimpleNeuralController>,

    pub current_action: Option<CreatureAction>,
    pub action_timer: f32,

    pub sensor_config: SensorConfig,

    pub position: Vec2,
    pub generation: u64,
}

impl Creature {
    /// Create creature from genome
    pub fn from_genome(genome: CreatureGenome, position: Vec2) -> Self {
        use super::morphology::MorphologyConfig;

        // Generate morphology from genome
        let config = MorphologyConfig::default();
        let morphology = CreatureMorphology::from_genome(&genome, &config);

        // Create neural controller from genome
        let num_raycasts = 8;
        let num_materials = 5;
        let input_dim = morphology.body_parts.len()
            * super::neural::BodyPartFeatures::feature_dim(num_raycasts, num_materials);
        let output_dim = morphology.joints.len(); // One motor command per joint

        let brain = SimpleNeuralController::from_genome(&genome.controller, input_dim, output_dim);

        // Create planner
        let planner = GoalPlanner::new();

        Self {
            id: EntityId::new(),
            genome,
            morphology,
            physics: None, // Will be built when spawned
            health: Health::new(100.0),
            hunger: Hunger::new(100.0, 0.5, 5.0), // max=100, drain=0.5/s, starvation_dmg=5/s
            needs: CreatureNeeds::new(),
            planner: Some(planner),
            brain: Some(brain),
            current_action: None,
            action_timer: 0.0,
            sensor_config: SensorConfig::default(),
            position,
            generation: 0,
        }
    }

    /// Update creature state
    /// Returns true if the creature died
    pub fn update(&mut self, delta_time: f32, sensory_input: &SensoryInput) -> bool {
        // 1. Update hunger (depletes over time)
        self.hunger.update(delta_time);

        // Check for starvation damage
        if self.hunger.is_starving() {
            self.health.take_damage(5.0 * delta_time);
        }

        // 2. Update needs from sensory input
        self.needs.update(sensory_input, self.hunger.percentage());

        // 3. Update behavior planning
        if let Some(ref mut planner) = self.planner {
            // Check if current plan is still valid
            if !planner.is_plan_valid(sensory_input) {
                // Re-plan
                planner.update_goal(&self.needs);
                planner.evaluate_world_state(sensory_input, self.hunger.percentage());
                planner.plan(sensory_input, self.position);
            }

            // Execute current action
            if self.current_action.is_none() || self.action_timer <= 0.0 {
                // Get next action from plan
                self.current_action = planner.next_action();
                if let Some(ref action) = self.current_action {
                    self.action_timer = action.duration();
                }
            }

            // Decrement action timer
            self.action_timer -= delta_time;
        }

        // 4. Neural control (placeholder for Phase 6 - motors not yet applied)
        if let Some(ref _brain) = self.brain {
            // TODO: Extract features, run forward pass, apply motor commands to physics
            // This will be fully implemented when we integrate with physics
        }

        // 5. Check if dead
        self.health.is_dead()
    }

    /// Rebuild physics body (after loading from save)
    pub fn rebuild_physics(&mut self, physics_world: &mut crate::physics::PhysicsWorld) {
        let physics =
            MorphologyPhysics::from_morphology(&self.morphology, self.position, physics_world);
        self.physics = Some(physics);
    }

    /// Rebuild brain (after loading from save)
    pub fn rebuild_brain(&mut self) {
        let num_raycasts = self.sensor_config.num_raycasts;
        let num_materials = 5;

        let input_dim = self.morphology.body_parts.len()
            * super::neural::BodyPartFeatures::feature_dim(num_raycasts, num_materials);
        let output_dim = self.morphology.joints.len();

        let brain =
            SimpleNeuralController::from_genome(&self.genome.controller, input_dim, output_dim);

        self.brain = Some(brain);
        self.planner = Some(GoalPlanner::new());
    }

    /// Execute current action (called by CreatureManager)
    pub fn execute_action(&mut self, world: &mut crate::world::World, _delta_time: f32) -> bool {
        if let Some(ref action) = self.current_action {
            match action {
                CreatureAction::Eat { position, .. } => {
                    if let Some(nutrition) = super::world_interaction::consume_edible_material(
                        world, *position, &self.id,
                    ) {
                        self.hunger.eat(nutrition);
                        return true;
                    }
                }
                CreatureAction::Mine { position, .. } => {
                    if let Some(_material_id) =
                        super::world_interaction::mine_world_pixel(world, *position, &self.id)
                    {
                        // Mining successful
                        return true;
                    }
                }
                CreatureAction::Build {
                    position,
                    material_id,
                } => {
                    if super::world_interaction::place_material(
                        world,
                        *position,
                        *material_id,
                        &self.id,
                    ) {
                        return true;
                    }
                }
                _ => {
                    // MoveTo, Wander, Flee, Rest - handled by neural controller/physics
                }
            }
        }

        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_creature_creation() {
        let genome = CreatureGenome::test_biped();
        let creature = Creature::from_genome(genome, Vec2::new(100.0, 100.0));

        assert_eq!(creature.position, Vec2::new(100.0, 100.0));
        assert!(creature.health.current > 0.0);
        assert!(creature.brain.is_some());
        assert!(creature.planner.is_some());
        assert_eq!(creature.generation, 0);
    }

    #[test]
    fn test_creature_entity_id() {
        // Test that creatures can be created with unique IDs
        let genome = CreatureGenome::test_biped();
        let creature1 = Creature::from_genome(genome.clone(), Vec2::ZERO);
        let creature2 = Creature::from_genome(genome, Vec2::ZERO);
        assert_ne!(creature1.id, creature2.id);
    }

    #[test]
    fn test_creature_has_morphology() {
        let genome = CreatureGenome::test_biped();
        let creature = Creature::from_genome(genome, Vec2::ZERO);

        // Should have some body parts and joints
        assert!(!creature.morphology.body_parts.is_empty());
        assert!(!creature.morphology.joints.is_empty());
    }

    #[test]
    fn test_creature_update_hunger() {
        let genome = CreatureGenome::test_biped();
        let mut creature = Creature::from_genome(genome, Vec2::ZERO);

        let initial_hunger = creature.hunger.percentage();

        // Create simple sensory input
        let sensory = SensoryInput {
            raycasts: vec![],
            contact_materials: vec![],
            gradients: crate::creature::sensors::ChemicalGradient {
                food: 0.0,
                danger: 0.0,
                mate: 0.0,
            },
            nearest_food: None,
            nearest_threat: None,
        };

        // Update for 1 second
        let died = creature.update(1.0, &sensory);

        // Should not have died yet
        assert!(!died);

        // Hunger should have decreased (percentage goes down as you get hungrier)
        assert!(creature.hunger.percentage() < initial_hunger);
    }

    #[test]
    fn test_creature_starvation_damage() {
        let genome = CreatureGenome::test_biped();
        let mut creature = Creature::from_genome(genome, Vec2::ZERO);

        // Set hunger to starving (0.0 = completely empty)
        creature.hunger.set(0.0);

        let initial_health = creature.health.current;

        let sensory = SensoryInput {
            raycasts: vec![],
            contact_materials: vec![],
            gradients: crate::creature::sensors::ChemicalGradient {
                food: 0.0,
                danger: 0.0,
                mate: 0.0,
            },
            nearest_food: None,
            nearest_threat: None,
        };

        // Update for 1 second while starving
        creature.update(1.0, &sensory);

        // Health should have decreased
        assert!(creature.health.current < initial_health);
    }

    #[test]
    fn test_creature_action_planning() {
        let genome = CreatureGenome::test_biped();
        let mut creature = Creature::from_genome(genome, Vec2::ZERO);

        let sensory = SensoryInput {
            raycasts: vec![],
            contact_materials: vec![],
            gradients: crate::creature::sensors::ChemicalGradient {
                food: 0.8,
                danger: 0.0,
                mate: 0.0,
            },
            nearest_food: Some(Vec2::new(10.0, 10.0)),
            nearest_threat: None,
        };

        // Update to trigger planning
        creature.update(0.1, &sensory);

        // Should have some action planned
        assert!(creature.current_action.is_some());
    }

    #[test]
    fn test_creature_rebuild_brain() {
        let genome = CreatureGenome::test_biped();
        let mut creature = Creature::from_genome(genome, Vec2::ZERO);

        // Clear brain to simulate post-load state
        creature.brain = None;
        creature.planner = None;

        // Rebuild
        creature.rebuild_brain();

        // Should have brain and planner again
        assert!(creature.brain.is_some());
        assert!(creature.planner.is_some());
    }

    #[test]
    fn test_different_genomes_produce_creatures() {
        let biped = CreatureGenome::test_biped();
        let quadruped = CreatureGenome::test_quadruped();
        let worm = CreatureGenome::test_worm();

        let creature1 = Creature::from_genome(biped, Vec2::ZERO);
        let creature2 = Creature::from_genome(quadruped, Vec2::ZERO);
        let creature3 = Creature::from_genome(worm, Vec2::ZERO);

        // All should have valid morphologies
        assert!(!creature1.morphology.body_parts.is_empty());
        assert!(!creature2.morphology.body_parts.is_empty());
        assert!(!creature3.morphology.body_parts.is_empty());

        // Different controller architectures
        assert_ne!(
            creature1.genome.controller.hidden_dim,
            creature3.genome.controller.hidden_dim
        );
    }
}
