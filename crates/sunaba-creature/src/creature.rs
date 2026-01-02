//! Main creature entity
//!
//! Combines genome, morphology, neural control, and behavior.

use glam::Vec2;
use serde::{Deserialize, Serialize};

use crate::types::{EntityId, Health, Hunger};

use super::behavior::{CreatureAction, CreatureNeeds, GoalPlanner};
use super::genome::CreatureGenome;
use super::morphology::{CreatureMorphology, MorphologyPhysics};
use super::neural::DeepNeuralController;
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
    pub brain: Option<DeepNeuralController>,

    pub current_action: Option<CreatureAction>,
    pub action_timer: f32,

    pub sensor_config: SensorConfig,

    pub position: Vec2,
    pub generation: u64,

    /// Counter for food items eaten (for fitness evaluation)
    pub food_eaten: u32,

    // Movement state (not serialized - runtime only)
    #[serde(skip)]
    pub velocity: Vec2,
    #[serde(skip)]
    pub wander_target: Option<Vec2>,
    #[serde(skip)]
    pub wander_timer: f32,
    #[serde(skip)]
    pub facing_direction: f32, // -1.0 = left, 1.0 = right
    #[serde(skip)]
    pub grounded: bool,
    #[serde(skip)]
    pub pending_motor_commands: Option<Vec<f32>>,
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

        let brain = DeepNeuralController::from_genome(&genome.controller, input_dim, output_dim);

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
            food_eaten: 0,
            velocity: Vec2::ZERO,
            wander_target: None,
            wander_timer: 0.0,
            facing_direction: 1.0,
            grounded: false,
            pending_motor_commands: None,
        }
    }

    /// Update creature state
    /// Returns true if the creature died
    pub fn update(
        &mut self,
        delta_time: f32,
        sensory_input: &SensoryInput,
        physics_world: &crate::PhysicsWorld,
        world: &impl crate::WorldAccess,
    ) -> bool {
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

        // 4. Neural control - run brain and get motor commands
        self.run_neural_control(delta_time, sensory_input, physics_world, world);

        // 5. Check if dead
        self.health.is_dead()
    }

    /// Run neural controller and return motor commands
    fn run_neural_control(
        &mut self,
        _delta_time: f32,
        sensory_input: &SensoryInput,
        physics_world: &crate::PhysicsWorld,
        world: &impl crate::WorldAccess,
    ) {
        // Get physics handles for feature extraction
        let physics_handles: Option<&[rapier2d::prelude::RigidBodyHandle]> =
            self.physics.as_ref().map(|p| p.link_handles.as_slice());

        // Extract features from physics state
        let features = super::neural::extract_body_part_features(
            &self.morphology,
            physics_world,
            sensory_input,
            physics_handles,
            world,
        );

        // Flatten features into input vector for neural network
        let mut input_vec = Vec::new();
        for feature in &features {
            input_vec.extend(feature.to_vec());
        }

        // Run neural network forward pass
        if let Some(ref mut brain) = self.brain {
            // Ensure input dimensions match
            if input_vec.len() == brain.input_dim() {
                let motor_commands = brain.forward(&input_vec);

                // Store motor commands to be applied during apply_movement
                self.pending_motor_commands = Some(motor_commands);
            }
        }
    }

    /// Apply pending motor commands to physics
    fn apply_motor_commands_to_physics(
        &mut self,
        delta_time: f32,
        physics_world: &mut crate::PhysicsWorld,
    ) {
        if let (Some(physics), Some(motor_commands)) =
            (&mut self.physics, self.pending_motor_commands.take())
        {
            // Apply motor commands to update target angles
            physics.apply_all_motor_commands(&motor_commands, &self.morphology, delta_time);

            // Apply motor rotations to physics bodies
            physics.apply_motor_rotations(&self.morphology, self.position, physics_world);
        }
    }

    /// Rebuild physics body (after loading from save)
    pub fn rebuild_physics(&mut self, physics_world: &mut crate::PhysicsWorld) {
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
            DeepNeuralController::from_genome(&self.genome.controller, input_dim, output_dim);

        self.brain = Some(brain);
        self.planner = Some(GoalPlanner::new());
    }

    /// Get render data for this creature (body part positions and radii)
    pub fn get_render_data(
        &self,
        physics_world: &crate::PhysicsWorld,
    ) -> Option<super::CreatureRenderData> {
        let physics = self.physics.as_ref()?;

        let mut body_parts = Vec::new();

        // Iterate through body parts and their physics handles
        for (i, body_part) in self.morphology.body_parts.iter().enumerate() {
            if let Some(&handle) = physics.link_handles.get(i)
                && let Some(rigid_body) = physics_world.rigid_body_set().get(handle)
            {
                let translation = rigid_body.translation();
                let position = Vec2::new(translation.x, translation.y);

                // Magenta color to stand out from environment
                let color = [200, 50, 200, 255];

                body_parts.push(super::BodyPartRenderData {
                    position,
                    radius: body_part.radius,
                    color,
                });
            }
        }

        if body_parts.is_empty() {
            return None;
        }

        Some(super::CreatureRenderData { body_parts })
    }

    /// Execute current action (called by CreatureManager)
    pub fn execute_action(
        &mut self,
        world: &mut impl crate::WorldMutAccess,
        _delta_time: f32,
    ) -> bool {
        if let Some(ref action) = self.current_action {
            match action {
                CreatureAction::Eat { position, .. } => {
                    if let Some(nutrition) = super::world_interaction::consume_edible_material(
                        world, *position, &self.id,
                    ) {
                        self.hunger.eat(nutrition);
                        self.food_eaten += 1;
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
                    // MoveTo, Wander, Flee, Rest - handled by apply_movement
                }
            }
        }

        false
    }

    /// Apply physics movement to creature (gravity, collision, wandering)
    /// This is the main movement logic that replaces rapier2d dynamic physics
    pub fn apply_movement(
        &mut self,
        world: &impl crate::WorldAccess,
        physics_world: &mut crate::PhysicsWorld,
        delta_time: f32,
    ) {
        use rand::Rng;

        const GRAVITY: f32 = 300.0;
        const MOVE_SPEED: f32 = 30.0;
        const WANDER_INTERVAL: f32 = 3.0;

        // Get body part positions from physics
        let body_positions: Vec<(Vec2, f32)> = if let Some(ref physics) = self.physics {
            self.morphology
                .body_parts
                .iter()
                .enumerate()
                .filter_map(|(i, part)| {
                    physics.link_handles.get(i).and_then(|&handle| {
                        physics_world.rigid_body_set().get(handle).map(|rb| {
                            let pos = rb.translation();
                            (Vec2::new(pos.x, pos.y), part.radius)
                        })
                    })
                })
                .collect()
        } else {
            return;
        };

        if body_positions.is_empty() {
            return;
        }

        // Check if grounded
        self.grounded = world.is_creature_grounded(&body_positions);

        // Apply gravity if not grounded
        if !self.grounded {
            self.velocity.y -= GRAVITY * delta_time;
            self.velocity.y = self.velocity.y.max(-500.0); // Terminal velocity
        } else {
            self.velocity.y = 0.0;
        }

        // Update wander timer and pick new target if needed
        self.wander_timer -= delta_time;
        if self.wander_timer <= 0.0 || self.wander_target.is_none() {
            let mut rng = rand::rng();
            let angle = rng.random::<f32>() * std::f32::consts::TAU;
            let dist = rng.random::<f32>() * 30.0 + 10.0;
            self.wander_target =
                Some(self.position + Vec2::new(angle.cos() * dist, angle.sin().abs() * dist * 0.3));
            self.wander_timer = WANDER_INTERVAL;
        }

        // Calculate movement toward target
        if let Some(target) = self.wander_target {
            let to_target = target - self.position;
            let distance = to_target.length();

            if distance > 2.0 {
                let dir = to_target.normalize();
                self.facing_direction = if dir.x >= 0.0 { 1.0 } else { -1.0 };

                // Check for blocking pixels
                let root_radius = body_positions.first().map(|(_, r)| *r).unwrap_or(3.0);
                if let Some((bx, by, material_id)) =
                    world.get_blocking_pixel(self.position, dir, root_radius, root_radius + 3.0)
                {
                    // 70% chance to mine, 30% to turn around
                    let mut rng = rand::rng();
                    if rng.random::<f32>() < 0.7 {
                        // Check if it's not bedrock (material_id 14)
                        if material_id != 14 {
                            self.current_action = Some(CreatureAction::Mine {
                                position: Vec2::new(bx as f32, by as f32),
                                material_id,
                            });
                            self.action_timer = 0.5; // Mining takes time
                        }
                    }
                    // Either way, pick new target
                    self.wander_target = None;
                } else {
                    // Move toward target
                    self.velocity.x = dir.x * MOVE_SPEED;
                }
            } else {
                // Reached target, pick new one next frame
                self.wander_target = None;
                self.velocity.x = 0.0;
            }
        }

        // Calculate new position
        let movement = self.velocity * delta_time;
        let new_x = self.position.x + movement.x;
        let new_y = self.position.y + movement.y;

        // Check collision for horizontal movement
        let root_radius = body_positions.first().map(|(_, r)| *r).unwrap_or(3.0);
        let can_move_x = !world.check_circle_collision(new_x, self.position.y, root_radius);
        let can_move_y = !world.check_circle_collision(self.position.x, new_y, root_radius);

        if can_move_x {
            self.position.x = new_x;
        } else {
            self.velocity.x = 0.0;
        }

        if can_move_y {
            self.position.y = new_y;
        } else {
            self.velocity.y = 0.0;
        }

        // Apply motor commands from neural network (rotates body parts)
        self.apply_motor_commands_to_physics(delta_time, physics_world);

        // Update physics body positions (for non-motorized parts)
        self.sync_physics_positions(physics_world);
    }

    /// Sync creature position to all physics body parts
    fn sync_physics_positions(&mut self, physics_world: &mut crate::PhysicsWorld) {
        use rapier2d::prelude::*;

        let Some(ref physics) = self.physics else {
            return;
        };

        // Move root body part to creature position
        // Other body parts follow with their local offsets
        for (i, part) in self.morphology.body_parts.iter().enumerate() {
            if let Some(&handle) = physics.link_handles.get(i)
                && let Some(rb) = physics_world.rigid_body_set_mut().get_mut(handle)
            {
                let world_pos = self.position + part.local_position;
                rb.set_translation(vector![world_pos.x, world_pos.y], true);
            }
        }
    }

    /// Get all body part positions (for external use)
    pub fn get_body_positions(&self, physics_world: &crate::PhysicsWorld) -> Vec<(Vec2, f32)> {
        if let Some(ref physics) = self.physics {
            self.morphology
                .body_parts
                .iter()
                .enumerate()
                .filter_map(|(i, part)| {
                    physics.link_handles.get(i).and_then(|&handle| {
                        physics_world.rigid_body_set().get(handle).map(|rb| {
                            let pos = rb.translation();
                            (Vec2::new(pos.x, pos.y), part.radius)
                        })
                    })
                })
                .collect()
        } else {
            vec![]
        }
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
    #[ignore] // Requires concrete World implementation from sunaba-core
    fn test_creature_update_hunger() {
        use crate::PhysicsWorld;
        use crate::sensors::ChemicalGradient;

        let genome = CreatureGenome::test_biped();
        let creature = Creature::from_genome(genome, Vec2::ZERO);
        let physics_world = PhysicsWorld::new();
        // Note: World::new() is in sunaba-core, not available here
        // let world = World::new();

        let initial_hunger = creature.hunger.percentage();

        // Create simple sensory input
        let sensory = SensoryInput {
            raycasts: vec![],
            contact_materials: vec![],
            gradients: ChemicalGradient {
                food: 0.0,
                danger: 0.0,
                mate: 0.0,
            },
            nearest_food: None,
            nearest_threat: None,
            food_direction: None,
            food_distance: 1.0,
        };

        // This test requires a concrete World implementation
        // Update for 1 second
        // let died = creature.update(1.0, &sensory, &physics_world, &world);

        // Should not have died yet
        // assert!(!died);

        // Hunger should have decreased (percentage goes down as you get hungrier)
        // assert!(creature.hunger.percentage() < initial_hunger);
        let _ = (initial_hunger, sensory, physics_world);
    }

    #[test]
    #[ignore] // Requires concrete World implementation from sunaba-core
    fn test_creature_starvation_damage() {
        use crate::PhysicsWorld;
        use crate::sensors::ChemicalGradient;

        let genome = CreatureGenome::test_biped();
        let mut creature = Creature::from_genome(genome, Vec2::ZERO);
        let physics_world = PhysicsWorld::new();
        // Note: World::new() is in sunaba-core, not available here

        // Set hunger to starving (0.0 = completely empty)
        creature.hunger.set(0.0);

        let initial_health = creature.health.current;

        let sensory = SensoryInput {
            raycasts: vec![],
            contact_materials: vec![],
            gradients: ChemicalGradient {
                food: 0.0,
                danger: 0.0,
                mate: 0.0,
            },
            nearest_food: None,
            nearest_threat: None,
            food_direction: None,
            food_distance: 1.0,
        };

        // This test requires a concrete World implementation
        // Update for 1 second while starving
        // creature.update(1.0, &sensory, &physics_world, &world);

        // Health should have decreased
        // assert!(creature.health.current < initial_health);
        let _ = (initial_health, sensory, physics_world);
    }

    #[test]
    #[ignore] // Requires concrete World implementation from sunaba-core
    fn test_creature_action_planning() {
        use crate::PhysicsWorld;
        use crate::sensors::ChemicalGradient;

        let genome = CreatureGenome::test_biped();
        let _creature = Creature::from_genome(genome, Vec2::ZERO);
        let physics_world = PhysicsWorld::new();
        // Note: World::new() is in sunaba-core, not available here

        let sensory = SensoryInput {
            raycasts: vec![],
            contact_materials: vec![],
            gradients: ChemicalGradient {
                food: 0.8,
                danger: 0.0,
                mate: 0.0,
            },
            nearest_food: Some(Vec2::new(10.0, 10.0)),
            nearest_threat: None,
            food_direction: Some(Vec2::new(1.0, 0.0)),
            food_distance: 0.1,
        };

        // This test requires a concrete World implementation
        // Update to trigger planning
        // creature.update(0.1, &sensory, &physics_world, &world);

        // Should have some action planned
        // assert!(creature.current_action.is_some());
        let _ = (sensory, physics_world);
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
