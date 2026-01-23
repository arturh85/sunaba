//! Main creature entity
//!
//! Combines genome, morphology, neural control, and behavior.

use glam::Vec2;
use serde::{Deserialize, Serialize};

use crate::types::{EntityId, Health, Hunger};

use super::behavior::{CreatureAction, CreatureNeeds, GoalPlanner};
use super::genome::CreatureGenome;
use super::morphology::CreatureMorphology;
use super::neural::DeepNeuralController;
use super::sensors::{SensorConfig, SensoryInput};
use super::simple_physics::CreaturePhysicsState;

/// Main creature entity
#[derive(Serialize, Deserialize)]
pub struct Creature {
    pub id: EntityId,
    pub genome: CreatureGenome,
    pub morphology: CreatureMorphology,

    /// Simple physics state (positions, rotations, motors) - no external physics engine
    pub physics_state: CreaturePhysicsState,

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

    /// Counter for blocks mined (for fitness evaluation)
    pub blocks_mined: u32,

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
    #[serde(skip)]
    pub pending_mine_strength: Option<f32>,
}

impl Creature {
    /// Create creature from genome with custom morphology config
    pub fn from_genome_with_config(
        genome: CreatureGenome,
        position: Vec2,
        config: &super::morphology::MorphologyConfig,
    ) -> Self {
        // Generate morphology from genome using provided config
        let morphology = CreatureMorphology::from_genome(&genome, config);

        // Create neural controller from genome
        let num_raycasts = 8;
        let num_materials = 5;
        let input_dim = morphology.body_parts.len()
            * super::neural::BodyPartFeatures::feature_dim(num_raycasts, num_materials);
        let output_dim = morphology.joints.len() + 1; // Motor commands + mining action

        let brain = DeepNeuralController::from_genome(&genome.controller, input_dim, output_dim);

        // Create planner
        let planner = GoalPlanner::new();

        // Create simple physics state
        let physics_state = CreaturePhysicsState::new(&morphology, position);

        Self {
            id: EntityId::new(),
            genome,
            morphology,
            physics_state,
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
            blocks_mined: 0,
            velocity: Vec2::ZERO,
            wander_target: None,
            wander_timer: 0.0,
            facing_direction: 1.0,
            grounded: false,
            pending_motor_commands: None,
            pending_mine_strength: None,
        }
    }

    /// Create creature from genome using default morphology config
    pub fn from_genome(genome: CreatureGenome, position: Vec2) -> Self {
        use super::morphology::MorphologyConfig;
        Self::from_genome_with_config(genome, position, &MorphologyConfig::default())
    }

    /// Create creature from genome with archetype morphology
    ///
    /// For fixed archetypes (Spider, Snake, Worm, Flyer), uses the predefined body plan.
    /// For Evolved archetype, generates morphology from CPPN genome.
    pub fn from_genome_with_archetype(
        genome: CreatureGenome,
        position: Vec2,
        config: &super::morphology::MorphologyConfig,
        archetype: super::morphology::CreatureArchetype,
    ) -> Self {
        // Generate morphology based on archetype
        let morphology = archetype.create_morphology(&genome, config);

        // Create neural controller from genome
        let num_raycasts = 8;
        let num_materials = 5;
        let input_dim = morphology.body_parts.len()
            * super::neural::BodyPartFeatures::feature_dim(num_raycasts, num_materials);
        let output_dim = morphology.joints.len() + 1; // Motor commands + mining action

        let brain = DeepNeuralController::from_genome(&genome.controller, input_dim, output_dim);

        // Create planner
        let planner = GoalPlanner::new();

        // Create simple physics state
        let physics_state = CreaturePhysicsState::new(&morphology, position);

        Self {
            id: EntityId::new(),
            genome,
            morphology,
            physics_state,
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
            blocks_mined: 0,
            velocity: Vec2::ZERO,
            wander_target: None,
            wander_timer: 0.0,
            facing_direction: 1.0,
            grounded: false,
            pending_motor_commands: None,
            pending_mine_strength: None,
        }
    }

    /// Update creature state
    /// Returns true if the creature died
    pub fn update(
        &mut self,
        delta_time: f32,
        sensory_input: &SensoryInput,
        world: &mut impl crate::WorldMutAccess,
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
        self.run_neural_control(delta_time, sensory_input, world);

        // 5. Auto-eating - proximity-based food consumption
        const AUTO_EAT_RADIUS: f32 = 8.0;

        // Optimization: Only scan pixels if sensors detected food nearby
        if let Some(food_pos) = sensory_input.nearest_food {
            let dist_to_food = (food_pos - self.position).length();

            if dist_to_food <= AUTO_EAT_RADIUS {
                // Scan 8-pixel radius around creature position
                let scan_radius = AUTO_EAT_RADIUS as i32;
                'eat_search: for dx in -scan_radius..=scan_radius {
                    for dy in -scan_radius..=scan_radius {
                        // Check if within circular radius
                        let dist_sq = (dx * dx + dy * dy) as f32;
                        if dist_sq <= AUTO_EAT_RADIUS * AUTO_EAT_RADIUS {
                            let check_x = (self.position.x + dx as f32).round() as i32;
                            let check_y = (self.position.y + dy as f32).round() as i32;

                            // Try to consume food at this position
                            let pos = Vec2::new(check_x as f32, check_y as f32);
                            if let Some(nutrition) =
                                super::world_interaction::consume_edible_material(
                                    world, pos, &self.id,
                                )
                            {
                                self.hunger.eat(nutrition);
                                self.food_eaten += 1;
                                // Only eat one item per update to avoid instant consumption
                                break 'eat_search;
                            }
                        }
                    }
                }
            }
        }

        // 6. Check if dead
        self.health.is_dead()
    }

    /// Run neural controller and return motor commands
    fn run_neural_control(
        &mut self,
        _delta_time: f32,
        sensory_input: &SensoryInput,
        world: &mut impl crate::WorldMutAccess,
    ) {
        // Extract features from physics state (uses simple_physics instead of rapier2d)
        let features = super::neural::extract_body_part_features_simple(
            &self.morphology,
            &self.physics_state,
            sensory_input,
            world,
            &self.sensor_config,
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
                let outputs = brain.forward(&input_vec);

                // Split outputs: joint motor commands + mining action
                let num_joints = self.morphology.joints.len();
                if outputs.len() > num_joints {
                    let (joint_commands, action_commands) = outputs.split_at(num_joints);
                    self.pending_motor_commands = Some(joint_commands.to_vec());
                    self.pending_mine_strength = action_commands.first().copied();
                } else {
                    // Fallback: all outputs are motor commands (no mining)
                    self.pending_motor_commands = Some(outputs);
                    self.pending_mine_strength = None;
                }
            }
        }
    }

    /// Apply pending motor commands to physics state
    fn apply_motor_commands_to_physics(&mut self, delta_time: f32) {
        if let Some(motor_commands) = self.pending_motor_commands.take() {
            // Apply motor commands to update target angles
            self.physics_state.apply_all_motor_commands(
                &motor_commands,
                &self.morphology,
                delta_time,
            );

            // Apply motor rotations to update body part positions
            self.physics_state
                .apply_motor_rotations(&self.morphology, self.position);
        }
    }

    /// Rebuild physics state (after loading from save or position change)
    pub fn rebuild_physics(&mut self) {
        self.physics_state = CreaturePhysicsState::new(&self.morphology, self.position);
    }

    /// Rebuild brain (after loading from save)
    pub fn rebuild_brain(&mut self) {
        let num_raycasts = self.sensor_config.num_raycasts;
        let num_materials = 5;

        let input_dim = self.morphology.body_parts.len()
            * super::neural::BodyPartFeatures::feature_dim(num_raycasts, num_materials);
        let output_dim = self.morphology.joints.len() + 1; // Motor commands + mining action

        let brain =
            DeepNeuralController::from_genome(&self.genome.controller, input_dim, output_dim);

        self.brain = Some(brain);
        self.planner = Some(GoalPlanner::new());
    }

    /// Get render data for this creature (body part positions and radii)
    pub fn get_render_data(&self) -> Option<super::CreatureRenderData> {
        use super::{BodyPartRenderData, BodyPartType, JointRenderData};
        use crate::morphology::JointType;

        // Build set of motorized body part indices for quick lookup
        let motor_indices: std::collections::HashSet<usize> = self
            .physics_state
            .motor_part_indices
            .iter()
            .copied()
            .collect();

        // Get positions from physics_state
        let positions: Vec<Option<Vec2>> = self
            .physics_state
            .part_positions
            .iter()
            .map(|&pos| Some(pos))
            .collect();

        let mut body_parts = Vec::new();

        // Classify and render each body part
        for (i, body_part) in self.morphology.body_parts.iter().enumerate() {
            if let Some(position) = positions.get(i).and_then(|p| *p) {
                // Determine body part type
                let part_type = if i == self.morphology.root_part_index {
                    BodyPartType::Root
                } else if motor_indices.contains(&i) {
                    BodyPartType::Motor
                } else {
                    BodyPartType::Fixed
                };

                // Calculate motor activity level (0.0-1.0)
                let motor_activity = if let Some(motor_idx) = self
                    .physics_state
                    .motor_part_indices
                    .iter()
                    .position(|&idx| idx == i)
                {
                    self.physics_state
                        .motor_angular_velocities
                        .get(motor_idx)
                        .map(|v| v.abs() / 3.0) // Normalize by max angular velocity
                        .unwrap_or(0.0)
                        .clamp(0.0, 1.0)
                } else {
                    0.0
                };

                // Blend color based on motor activity (brighter when moving)
                let base_color = part_type.color();
                let dim_color = part_type.dim_color();
                let color = [
                    lerp_u8(dim_color[0], base_color[0], motor_activity),
                    lerp_u8(dim_color[1], base_color[1], motor_activity),
                    lerp_u8(dim_color[2], base_color[2], motor_activity),
                    255,
                ];

                body_parts.push(BodyPartRenderData {
                    position,
                    radius: body_part.radius,
                    color,
                    part_type,
                    motor_activity,
                });
            }
        }

        if body_parts.is_empty() {
            return None;
        }

        // Generate joint connection data
        let mut joints = Vec::new();
        for joint in &self.morphology.joints {
            if let (Some(start_pos), Some(end_pos)) = (
                positions.get(joint.parent_index).and_then(|p| *p),
                positions.get(joint.child_index).and_then(|p| *p),
            ) {
                let is_motorized = matches!(joint.joint_type, JointType::Revolute { .. });

                // Get rotation angle for this joint if motorized
                let angle = if is_motorized {
                    // Find motor index for this joint's child
                    self.physics_state
                        .motor_part_indices
                        .iter()
                        .position(|&idx| idx == joint.child_index)
                        .and_then(|motor_idx| {
                            self.physics_state.motor_angles.get(motor_idx).copied()
                        })
                        .unwrap_or(0.0)
                } else {
                    0.0
                };

                joints.push(JointRenderData {
                    start: start_pos,
                    end: end_pos,
                    is_motorized,
                    angle,
                });
            }
        }

        Some(super::CreatureRenderData { body_parts, joints })
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

    /// Apply physics movement to creature (gravity, collision, motor-driven locomotion)
    /// Motor commands from the neural network drive horizontal movement.
    pub fn apply_movement(&mut self, world: &impl crate::WorldAccess, delta_time: f32) {
        const GRAVITY: f32 = 300.0;
        const MAX_SPEED: f32 = 80.0; // Max horizontal speed

        // Get body part positions from physics_state
        let body_positions: Vec<(Vec2, f32)> = self
            .physics_state
            .part_positions
            .iter()
            .zip(self.morphology.body_parts.iter())
            .map(|(&pos, part)| (pos, part.radius))
            .collect();

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

        // Apply motor commands from neural network (rotates body parts)
        // and extract locomotion velocity from motor activity
        self.apply_motor_commands_to_physics(delta_time);

        // Compute locomotion velocity from motor activity
        // The neural network controls movement through motor commands
        // Get motor activity: use angular velocities to drive movement
        // Positive angular velocity = clockwise = push right
        // Negative angular velocity = counterclockwise = push left
        let mut thrust_x = 0.0;
        let mut thrust_y = 0.0;
        let motor_count = self.physics_state.motor_angular_velocities.len();

        if motor_count > 0 {
            for (i, &angular_vel) in self
                .physics_state
                .motor_angular_velocities
                .iter()
                .enumerate()
            {
                // Get the body part position relative to center
                if let Some(&motor_idx) = self.physics_state.motor_part_indices.get(i)
                    && let Some(part) = self.morphology.body_parts.get(motor_idx)
                {
                    // Body parts on the left (negative x) contribute differently than right
                    // This creates asymmetric locomotion like legs
                    let side = if part.local_position.x < 0.0 {
                        -1.0
                    } else {
                        1.0
                    };
                    let height_factor = if part.local_position.y < 0.0 {
                        1.5 // Lower body parts contribute more (like legs)
                    } else {
                        0.5 // Upper parts contribute less
                    };

                    // Motor activity creates thrust
                    // Opposing sides with opposite rotations = forward motion
                    thrust_x += angular_vel * side * height_factor * 5.0;

                    // Vertical thrust (for jumping attempts)
                    if self.grounded && angular_vel.abs() > 2.0 {
                        thrust_y += angular_vel.abs() * height_factor * 0.5;
                    }

                    // Wing lift physics: wings oscillating in air create upward lift
                    // Higher angular velocity = more lift
                    if part.is_wing && !self.grounded {
                        // Wing oscillation creates lift proportional to angular velocity
                        // The faster the wing flaps, the more lift is generated
                        const WING_LIFT_FACTOR: f32 = 15.0;
                        let oscillation_intensity = angular_vel.abs();
                        let lift = oscillation_intensity * WING_LIFT_FACTOR;
                        thrust_y += lift;
                    }
                }
            }

            // Normalize by motor count for consistent behavior
            thrust_x /= motor_count as f32;
            thrust_y /= motor_count as f32;
        }

        // Apply thrust to velocity with damping
        self.velocity.x = self.velocity.x * 0.9 + thrust_x * delta_time * 100.0;
        self.velocity.x = self.velocity.x.clamp(-MAX_SPEED, MAX_SPEED);

        // Vertical thrust: jump from ground or wing lift while airborne
        if thrust_y > 0.5 {
            if self.grounded {
                // Jump or take off from ground
                self.velocity.y += thrust_y * delta_time * 50.0;
            } else {
                // Airborne: wing lift counters gravity (partial or full)
                // This allows sustained flight if wings oscillate fast enough
                self.velocity.y += thrust_y * delta_time * 40.0;
            }
        }

        // Update facing direction based on velocity
        if self.velocity.x.abs() > 1.0 {
            self.facing_direction = if self.velocity.x > 0.0 { 1.0 } else { -1.0 };
        }

        // Calculate new position
        let movement = self.velocity * delta_time;
        let new_x = self.position.x + movement.x;
        let new_y = self.position.y + movement.y;

        // Check collision for movement
        let root_radius = body_positions.first().map(|(_, r)| *r).unwrap_or(3.0);
        let can_move_x = !world.check_circle_collision(new_x, self.position.y, root_radius);
        let can_move_y = !world.check_circle_collision(self.position.x, new_y, root_radius);

        if can_move_x {
            self.position.x = new_x;
        } else {
            self.velocity.x = 0.0;
        }

        // Also check world bounds - stop at ground level (y=20 + radius)
        // This prevents falling through the world into negative coordinates
        let min_y = 20.0 + root_radius; // Ground is at y=0-20, stop just above it
        if can_move_y && new_y >= min_y {
            self.position.y = new_y;
        } else {
            self.velocity.y = 0.0;
            self.grounded = true;
            // Snap to ground if falling below minimum
            if self.position.y < min_y {
                self.position.y = min_y;
            }
        }

        // Sync physics_state positions with new creature position
        self.sync_physics_positions();
    }

    /// Sync creature position to all physics body parts
    fn sync_physics_positions(&mut self) {
        // Move root body part to creature position
        // Other body parts follow with their local offsets
        for (i, part) in self.morphology.body_parts.iter().enumerate() {
            // Update new parts array
            if let Some(part_state) = self.physics_state.parts.get_mut(i) {
                part_state.position = self.position + part.local_position;
            }
            // Also update legacy fields
            if let Some(pos) = self.physics_state.part_positions.get_mut(i) {
                *pos = self.position + part.local_position;
            }
        }
    }

    /// Apply full physics simulation update
    ///
    /// This uses the new physics system with proper forward kinematics,
    /// gravity per body part, and ground collision detection.
    pub fn apply_full_physics_update(
        &mut self,
        world: &impl crate::WorldAccess,
        delta_time: f32,
    ) {
        // Apply pending motor commands first
        if let Some(motor_commands) = self.pending_motor_commands.take() {
            self.physics_state.apply_motor_commands(&motor_commands);
        }

        // Create ground check closure that queries the world
        let ground_check = |pos: Vec2, radius: f32| -> Option<f32> {
            // Check for ground below this position
            let check_y = (pos.y - radius) as i32;
            let check_x = pos.x as i32;

            // Scan downward to find ground
            for dy in 0..5 {
                let y = check_y - dy;
                if let Some(pixel) = world.get_pixel(check_x, y) {
                    if pixel.material_id != 0 {
                        // Found solid ground
                        return Some((y + 1) as f32);
                    }
                }
            }
            None
        };

        // Run full physics update
        self.physics_state
            .update(&self.morphology, delta_time, ground_check);

        // Update creature position from physics root
        if let Some(root_pos) = self.physics_state.get_position() {
            self.position = root_pos;
        }

        // Update grounded state
        self.grounded = self.physics_state.is_any_grounded();

        // Update velocity from physics
        if let Some(avg_vel) = Some(self.physics_state.get_average_velocity()) {
            self.velocity = avg_vel;
        }

        // Update facing direction
        if self.velocity.x.abs() > 1.0 {
            self.facing_direction = if self.velocity.x > 0.0 { 1.0 } else { -1.0 };
        }
    }

    /// Get kinetic energy of the creature (for fitness evaluation)
    pub fn get_kinetic_energy(&self) -> f32 {
        self.physics_state.kinetic_energy
    }

    /// Get center of mass position
    pub fn get_center_of_mass(&self) -> Vec2 {
        self.physics_state.center_of_mass
    }

    /// Try to mine blocks based on neural network output
    ///
    /// When the mining output exceeds the threshold, mines blocks in the
    /// direction the creature is moving (toward food on the right).
    pub fn try_mine(&mut self, world: &mut impl crate::WorldMutAccess) {
        use sunaba_simulation::MaterialId;

        // Check if mining action was requested
        let mine_strength = match self.pending_mine_strength.take() {
            Some(s) => s,
            None => return,
        };

        // Mining threshold - neural output must exceed this to trigger mining
        // Output is tanh so range is [-1, 1], threshold at 0.3 means ~65% activation
        const MINE_THRESHOLD: f32 = 0.3;
        if mine_strength < MINE_THRESHOLD {
            return;
        }

        // Determine mining direction based on velocity or facing
        let mine_dir = if self.velocity.x.abs() > 0.5 {
            self.velocity.x.signum()
        } else {
            self.facing_direction
        };

        // Mine position: ahead of creature in movement direction
        let mine_offset = 8.0; // Distance ahead to mine
        let mine_x = (self.position.x + mine_dir * mine_offset) as i32;
        let mine_y = self.position.y as i32;

        // Mine a 3x3 area centered on the mining point
        for dx in -1..=1 {
            for dy in -1..=1 {
                let px = mine_x + dx;
                let py = mine_y + dy;

                if let Some(pixel) = world.get_pixel(px, py) {
                    // Only mine certain materials (stone, dirt, etc.)
                    // Don't mine bedrock or other protected materials
                    if pixel.material_id == MaterialId::STONE
                        || pixel.material_id == MaterialId::SAND
                    {
                        world.set_pixel(px, py, MaterialId::AIR);
                        self.blocks_mined += 1;
                    }
                }
            }
        }
    }

    /// Get all body part positions (for external use)
    pub fn get_body_positions(&self) -> Vec<(Vec2, f32)> {
        self.physics_state
            .part_positions
            .iter()
            .zip(self.morphology.body_parts.iter())
            .map(|(&pos, part)| (pos, part.radius))
            .collect()
    }
}

/// Linear interpolation for u8 values
fn lerp_u8(a: u8, b: u8, t: f32) -> u8 {
    let a = a as f32;
    let b = b as f32;
    (a + (b - a) * t.clamp(0.0, 1.0)) as u8
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
