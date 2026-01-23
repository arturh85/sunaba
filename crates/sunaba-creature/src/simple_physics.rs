//! Simple creature physics - no external physics engine
//!
//! Implements proper forward kinematics with joint chain propagation,
//! velocity tracking, gravity, and ground collision.
//! This is WASM-compatible and runs identically in native and SpacetimeDB.

use glam::Vec2;
use serde::{Deserialize, Serialize};
use std::f32::consts::PI;

use crate::morphology::{CreatureMorphology, JointType};

/// Physics constants
pub mod constants {
    /// Gravity acceleration (pixels/sec²)
    pub const GRAVITY: f32 = 400.0;
    /// Air resistance (velocity damping factor per second)
    pub const AIR_DAMPING: f32 = 0.98;
    /// Ground friction coefficient
    pub const GROUND_FRICTION: f32 = 0.85;
    /// Maximum angular velocity for motors (radians/sec)
    pub const MAX_ANGULAR_VEL: f32 = 8.0;
    /// Motor spring constant (determines how quickly motors reach target)
    pub const MOTOR_SPRING_K: f32 = 25.0;
    /// Motor damping constant
    pub const MOTOR_DAMPING: f32 = 0.9;
    /// Ground collision penetration threshold (pixels)
    pub const GROUND_PENETRATION_THRESHOLD: f32 = 2.0;
    /// Bounce coefficient for ground collision
    pub const BOUNCE_COEFFICIENT: f32 = 0.2;
}

/// Physics state for a single body part
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct BodyPartState {
    /// World position
    pub position: Vec2,
    /// Linear velocity (pixels/sec)
    pub velocity: Vec2,
    /// Rotation angle (radians, relative to parent or world for root)
    pub rotation: f32,
    /// Angular velocity (radians/sec)
    pub angular_velocity: f32,
    /// Whether this part is touching ground
    pub grounded: bool,
    /// Accumulated force this frame (pixels/sec²)
    pub force: Vec2,
}

/// Motor state for a joint
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct MotorState {
    /// Current angle (radians)
    pub angle: f32,
    /// Angular velocity (radians/sec)
    pub angular_velocity: f32,
    /// Target angle (radians, set by neural network)
    pub target_angle: f32,
    /// Joint limits (min, max)
    pub limits: (f32, f32),
    /// Index of the child body part this motor controls
    pub child_part_index: usize,
    /// Index of the parent body part
    pub parent_part_index: usize,
}

/// Simple physics state for a creature's body
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct CreaturePhysicsState {
    /// State for each body part
    pub parts: Vec<BodyPartState>,
    /// Motor states (one per revolute joint)
    pub motors: Vec<MotorState>,
    /// Center of mass (computed each frame)
    pub center_of_mass: Vec2,
    /// Total kinetic energy (for fitness evaluation)
    pub kinetic_energy: f32,

    // Legacy fields for backwards compatibility
    /// Position of each body part (world coords) - legacy, use parts[i].position
    #[serde(skip)]
    pub part_positions: Vec<Vec2>,
    /// Rotation of each body part (radians) - legacy, use parts[i].rotation
    #[serde(skip)]
    pub part_rotations: Vec<f32>,
    /// Angular velocity of each motor joint (radians/sec)
    #[serde(skip)]
    pub motor_angular_velocities: Vec<f32>,
    /// Current angle of each motor joint (radians)
    #[serde(skip)]
    pub motor_angles: Vec<f32>,
    /// Target angle of each motor joint (radians, set by neural network)
    #[serde(skip)]
    pub motor_target_angles: Vec<f32>,
    /// Which body part index each motor controls
    #[serde(skip)]
    pub motor_part_indices: Vec<usize>,
}

impl CreaturePhysicsState {
    /// Create new physics state for a creature morphology at the given position
    pub fn new(morphology: &CreatureMorphology, position: Vec2) -> Self {

        // Initialize body part states
        let mut parts: Vec<BodyPartState> = morphology
            .body_parts
            .iter()
            .map(|part| BodyPartState {
                position: position + part.local_position,
                velocity: Vec2::ZERO,
                rotation: 0.0,
                angular_velocity: 0.0,
                grounded: false,
                force: Vec2::ZERO,
            })
            .collect();

        // Root part starts at the given position
        if !parts.is_empty() {
            parts[morphology.root_part_index].position = position;
        }

        // Initialize motor states from joints
        let motors: Vec<MotorState> = morphology
            .joints
            .iter()
            .filter_map(|joint| {
                if let JointType::Revolute {
                    min_angle,
                    max_angle,
                } = joint.joint_type
                {
                    Some(MotorState {
                        angle: 0.0,
                        angular_velocity: 0.0,
                        target_angle: 0.0,
                        limits: (min_angle, max_angle),
                        child_part_index: joint.child_index,
                        parent_part_index: joint.parent_index,
                    })
                } else {
                    None
                }
            })
            .collect();

        // Build legacy compatibility fields
        let part_positions: Vec<Vec2> = parts.iter().map(|p| p.position).collect();
        let part_rotations: Vec<f32> = parts.iter().map(|p| p.rotation).collect();
        let motor_angular_velocities: Vec<f32> = motors.iter().map(|m| m.angular_velocity).collect();
        let motor_angles: Vec<f32> = motors.iter().map(|m| m.angle).collect();
        let motor_target_angles: Vec<f32> = motors.iter().map(|m| m.target_angle).collect();
        let motor_part_indices: Vec<usize> = motors.iter().map(|m| m.child_part_index).collect();

        Self {
            parts,
            motors,
            center_of_mass: position,
            kinetic_energy: 0.0,
            part_positions,
            part_rotations,
            motor_angular_velocities,
            motor_angles,
            motor_target_angles,
            motor_part_indices,
        }
    }

    /// Full physics update step
    pub fn update(
        &mut self,
        morphology: &CreatureMorphology,
        dt: f32,
        ground_check: impl Fn(Vec2, f32) -> Option<f32>,
    ) {
        // 1. Update motors (spring-damper toward target)
        self.update_motors(dt);

        // 2. Apply gravity to all parts
        self.apply_gravity(morphology, dt);

        // 3. Apply forward kinematics (propagate joint angles through chain)
        self.apply_forward_kinematics(morphology);

        // 4. Integrate velocities and positions
        self.integrate(dt);

        // 5. Handle ground collisions
        self.resolve_ground_collisions(morphology, &ground_check);

        // 6. Compute center of mass and kinetic energy
        self.compute_derived_quantities(morphology);

        // 7. Sync legacy fields
        self.sync_legacy_fields();
    }

    /// Update motor angles based on target angles (spring-damper model)
    pub fn update_motors(&mut self, dt: f32) {
        use constants::*;

        for motor in &mut self.motors {
            // Spring force toward target
            let diff = Self::angle_diff(motor.target_angle, motor.angle);
            let spring_force = diff * MOTOR_SPRING_K;

            // Apply spring acceleration
            motor.angular_velocity += spring_force * dt;
            motor.angular_velocity *= MOTOR_DAMPING;
            motor.angular_velocity = motor.angular_velocity.clamp(-MAX_ANGULAR_VEL, MAX_ANGULAR_VEL);

            // Integrate angle
            motor.angle += motor.angular_velocity * dt;

            // Clamp to joint limits
            motor.angle = motor.angle.clamp(motor.limits.0, motor.limits.1);
        }
    }

    /// Apply gravity force to all body parts based on their mass
    fn apply_gravity(&mut self, morphology: &CreatureMorphology, _dt: f32) {
        use constants::GRAVITY;

        for (i, part_state) in self.parts.iter_mut().enumerate() {
            if !part_state.grounded {
                // Gravity is applied as acceleration (force per unit mass)
                // Heavier parts are not affected differently since F = ma -> a = F/m = g
                let body_part = &morphology.body_parts[i];

                // Wing parts have reduced gravity effect (they generate lift)
                let gravity_multiplier = if body_part.is_wing { 0.5 } else { 1.0 };

                part_state.force.y -= GRAVITY * gravity_multiplier;
            }
        }
    }

    /// Apply forward kinematics - propagate joint angles through the kinematic chain
    /// This positions child body parts correctly based on their parent's position and joint angles
    fn apply_forward_kinematics(&mut self, morphology: &CreatureMorphology) {
        // Build parent-child relationships
        // We need to process from root outward (BFS order)
        let num_parts = morphology.body_parts.len();
        let root_idx = morphology.root_part_index;

        // Track which parts have been processed
        let mut processed = vec![false; num_parts];

        // Start with root
        processed[root_idx] = true;

        // Process joints in order - each joint connects parent to child
        // We may need multiple passes if the graph is complex
        let mut changed = true;
        while changed {
            changed = false;

            for motor in &self.motors {
                let parent_idx = motor.parent_part_index;
                let child_idx = motor.child_part_index;

                // Only process if parent is done but child isn't
                if processed[parent_idx] && !processed[child_idx] {
                    let parent_pos = self.parts[parent_idx].position;
                    let parent_rot = self.parts[parent_idx].rotation;

                    // Get local offset of child relative to parent
                    let parent_part = &morphology.body_parts[parent_idx];
                    let child_part = &morphology.body_parts[child_idx];
                    let local_offset = child_part.local_position - parent_part.local_position;

                    // Apply parent rotation + motor angle to the local offset
                    let total_angle = parent_rot + motor.angle;
                    let rotated_offset = Vec2::new(
                        local_offset.x * total_angle.cos() - local_offset.y * total_angle.sin(),
                        local_offset.x * total_angle.sin() + local_offset.y * total_angle.cos(),
                    );

                    // Set child position and rotation
                    self.parts[child_idx].position = parent_pos + rotated_offset;
                    self.parts[child_idx].rotation = total_angle;

                    processed[child_idx] = true;
                    changed = true;
                }
            }

            // Also process fixed joints
            for joint in &morphology.joints {
                if let JointType::Fixed = joint.joint_type {
                    let parent_idx = joint.parent_index;
                    let child_idx = joint.child_index;

                    if processed[parent_idx] && !processed[child_idx] {
                        let parent_pos = self.parts[parent_idx].position;
                        let parent_rot = self.parts[parent_idx].rotation;

                        let parent_part = &morphology.body_parts[parent_idx];
                        let child_part = &morphology.body_parts[child_idx];
                        let local_offset = child_part.local_position - parent_part.local_position;

                        let rotated_offset = Vec2::new(
                            local_offset.x * parent_rot.cos() - local_offset.y * parent_rot.sin(),
                            local_offset.x * parent_rot.sin() + local_offset.y * parent_rot.cos(),
                        );

                        self.parts[child_idx].position = parent_pos + rotated_offset;
                        self.parts[child_idx].rotation = parent_rot;

                        processed[child_idx] = true;
                        changed = true;
                    }
                }
            }
        }
    }

    /// Integrate velocities and positions using semi-implicit Euler
    fn integrate(&mut self, dt: f32) {
        use constants::AIR_DAMPING;

        for part in &mut self.parts {
            // Apply accumulated force as acceleration
            part.velocity += part.force * dt;

            // Apply air damping
            let damping = AIR_DAMPING.powf(dt);
            part.velocity *= damping;

            // Clear force accumulator
            part.force = Vec2::ZERO;
        }
    }

    /// Resolve ground collisions for all body parts
    fn resolve_ground_collisions(
        &mut self,
        morphology: &CreatureMorphology,
        ground_check: &impl Fn(Vec2, f32) -> Option<f32>,
    ) {
        use constants::*;

        for (i, part_state) in self.parts.iter_mut().enumerate() {
            let radius = morphology.body_parts[i].radius;

            // Check for ground collision
            if let Some(ground_y) = ground_check(part_state.position, radius) {
                let bottom = part_state.position.y - radius;
                let penetration = ground_y - bottom;

                if penetration > 0.0 {
                    // Resolve penetration
                    part_state.position.y += penetration + 0.1;

                    // Apply bounce and friction
                    if part_state.velocity.y < 0.0 {
                        part_state.velocity.y *= -BOUNCE_COEFFICIENT;
                    }

                    // Ground friction
                    part_state.velocity.x *= GROUND_FRICTION;

                    part_state.grounded = true;
                } else {
                    part_state.grounded = false;
                }
            } else {
                part_state.grounded = false;
            }
        }
    }

    /// Compute center of mass and kinetic energy
    fn compute_derived_quantities(&mut self, morphology: &CreatureMorphology) {
        let mut total_mass = 0.0;
        let mut weighted_pos = Vec2::ZERO;
        let mut kinetic_energy = 0.0;

        for (i, part_state) in self.parts.iter().enumerate() {
            let part = &morphology.body_parts[i];
            let mass = PI * part.radius * part.radius * part.density;

            weighted_pos += part_state.position * mass;
            total_mass += mass;

            // Kinetic energy = 0.5 * m * v²
            kinetic_energy += 0.5 * mass * part_state.velocity.length_squared();
        }

        if total_mass > 0.0 {
            self.center_of_mass = weighted_pos / total_mass;
        }
        self.kinetic_energy = kinetic_energy;
    }

    /// Sync legacy fields for backwards compatibility
    fn sync_legacy_fields(&mut self) {
        // Resize if needed
        if self.part_positions.len() != self.parts.len() {
            self.part_positions = vec![Vec2::ZERO; self.parts.len()];
            self.part_rotations = vec![0.0; self.parts.len()];
        }
        if self.motor_angles.len() != self.motors.len() {
            self.motor_angles = vec![0.0; self.motors.len()];
            self.motor_angular_velocities = vec![0.0; self.motors.len()];
            self.motor_target_angles = vec![0.0; self.motors.len()];
            self.motor_part_indices = self.motors.iter().map(|m| m.child_part_index).collect();
        }

        // Copy data
        for (i, part) in self.parts.iter().enumerate() {
            self.part_positions[i] = part.position;
            self.part_rotations[i] = part.rotation;
        }

        for (i, motor) in self.motors.iter().enumerate() {
            self.motor_angles[i] = motor.angle;
            self.motor_angular_velocities[i] = motor.angular_velocity;
            self.motor_target_angles[i] = motor.target_angle;
        }
    }

    /// Apply motor commands from neural network output
    pub fn apply_motor_commands(&mut self, commands: &[f32]) {
        for (i, &cmd) in commands.iter().enumerate() {
            if i < self.motors.len() {
                // Neural network outputs are typically -1 to 1
                // Map to the joint's actual limits
                let motor = &mut self.motors[i];
                let range = motor.limits.1 - motor.limits.0;
                let mid = (motor.limits.0 + motor.limits.1) / 2.0;
                motor.target_angle = mid + cmd.clamp(-1.0, 1.0) * (range / 2.0);
            }
        }

        // Also update legacy field
        for (i, motor) in self.motors.iter().enumerate() {
            if i < self.motor_target_angles.len() {
                self.motor_target_angles[i] = motor.target_angle;
            }
        }
    }

    /// Apply motor command to a specific motor (preserves existing functionality)
    pub fn apply_motor_command(
        &mut self,
        motor_index: usize,
        target: f32,
        _morphology: &CreatureMorphology,
        delta_time: f32,
    ) -> f32 {
        if motor_index >= self.motors.len() {
            return 0.0;
        }

        let motor = &mut self.motors[motor_index];

        // Motor speed constant (radians per second at max command)
        const MAX_ANGULAR_VELOCITY: f32 = 3.0;

        // Calculate target angular velocity from normalized command [-1, 1]
        let target_angular_vel = target.clamp(-1.0, 1.0) * MAX_ANGULAR_VELOCITY;
        motor.angular_velocity = target_angular_vel;

        // Update angle based on velocity
        motor.angle += target_angular_vel * delta_time;
        motor.angle = motor.angle.clamp(motor.limits.0, motor.limits.1);

        // Sync legacy field
        if motor_index < self.motor_angular_velocities.len() {
            self.motor_angular_velocities[motor_index] = target_angular_vel;
            self.motor_angles[motor_index] = motor.angle;
        }

        target_angular_vel
    }

    /// Apply all motor commands from neural network output
    pub fn apply_all_motor_commands(
        &mut self,
        motor_commands: &[f32],
        morphology: &CreatureMorphology,
        delta_time: f32,
    ) {
        let num_motors = self.motors.len().min(motor_commands.len());
        for (i, &command) in motor_commands.iter().enumerate().take(num_motors) {
            self.apply_motor_command(i, command, morphology, delta_time);
        }
    }

    /// Update body part positions based on current motor angles
    /// This rotates child body parts around their parent joint pivot
    pub fn apply_motor_rotations(&mut self, morphology: &CreatureMorphology, root_position: Vec2) {
        // Update root position
        if !self.parts.is_empty() {
            self.parts[morphology.root_part_index].position = root_position;
        }

        // Apply forward kinematics
        self.apply_forward_kinematics(morphology);

        // Sync legacy fields
        self.sync_legacy_fields();
    }

    /// Get motor angle for a specific motor index
    pub fn get_motor_angle(&self, motor_idx: usize) -> Option<f32> {
        self.motors.get(motor_idx).map(|m| m.angle)
    }

    /// Get angular velocity for a specific motor
    pub fn get_motor_velocity(&self, motor_idx: usize) -> Option<f32> {
        self.motors.get(motor_idx).map(|m| m.angular_velocity)
    }

    /// Get number of motors (for neural network output sizing)
    pub fn num_motors(&self) -> usize {
        self.motors.len()
    }

    /// Get body part positions for rendering
    pub fn get_body_positions(&self) -> Vec<(Vec2, f32)> {
        self.parts
            .iter()
            .map(|p| (p.position, p.rotation))
            .collect()
    }

    /// Get root position
    pub fn get_position(&self) -> Option<Vec2> {
        self.parts.first().map(|p| p.position)
    }

    /// Set root position
    pub fn set_position(&mut self, position: Vec2) {
        if !self.parts.is_empty() {
            self.parts[0].position = position;
        }
    }

    /// Get velocity of a body part
    pub fn get_part_velocity(&self, part_idx: usize) -> Option<Vec2> {
        self.parts.get(part_idx).map(|p| p.velocity)
    }

    /// Set velocity of root body part
    pub fn set_root_velocity(&mut self, velocity: Vec2) {
        if !self.parts.is_empty() {
            self.parts[0].velocity = velocity;
        }
    }

    /// Check if any part is grounded
    pub fn is_any_grounded(&self) -> bool {
        self.parts.iter().any(|p| p.grounded)
    }

    /// Get total velocity (average of all parts)
    pub fn get_average_velocity(&self) -> Vec2 {
        if self.parts.is_empty() {
            return Vec2::ZERO;
        }
        let sum: Vec2 = self.parts.iter().map(|p| p.velocity).sum();
        sum / self.parts.len() as f32
    }

    /// Calculate shortest angle difference (handles wraparound)
    fn angle_diff(target: f32, current: f32) -> f32 {
        let mut diff = target - current;
        while diff > PI {
            diff -= 2.0 * PI;
        }
        while diff < -PI {
            diff += 2.0 * PI;
        }
        diff
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_morphology() -> CreatureMorphology {
        CreatureMorphology::test_biped()
    }

    #[test]
    fn test_create_physics_state() {
        let morphology = make_test_morphology();
        let state = CreaturePhysicsState::new(&morphology, Vec2::new(0.0, 100.0));

        assert_eq!(state.parts.len(), morphology.body_parts.len());
        assert_eq!(state.part_positions.len(), morphology.body_parts.len());
        assert_eq!(state.part_rotations.len(), morphology.body_parts.len());
    }

    #[test]
    fn test_motor_spring() {
        let morphology = make_test_morphology();
        let mut state = CreaturePhysicsState::new(&morphology, Vec2::ZERO);

        if !state.motors.is_empty() {
            state.motors[0].target_angle = 1.0;

            // Simulate many steps
            for _ in 0..500 {
                state.update_motors(0.016);
            }

            // Should approach target (within 0.2 radians)
            assert!(
                (state.motors[0].angle - 1.0).abs() < 0.2,
                "Motor angle {} should be close to 1.0",
                state.motors[0].angle
            );
        }
    }

    #[test]
    fn test_angle_diff_wraparound() {
        // PI and -PI are the same angle
        assert!((CreaturePhysicsState::angle_diff(PI, -PI)).abs() < 0.01);
        // From 0.9*PI to -0.9*PI, shortest path is 0.2*PI (going through PI/-PI boundary)
        assert!(
            (CreaturePhysicsState::angle_diff(-PI * 0.9, PI * 0.9) - (0.2 * PI)).abs() < 0.01,
            "Expected 0.2*PI, got {}",
            CreaturePhysicsState::angle_diff(-PI * 0.9, PI * 0.9)
        );
    }

    #[test]
    fn test_apply_motor_commands() {
        let morphology = make_test_morphology();
        let mut state = CreaturePhysicsState::new(&morphology, Vec2::ZERO);

        let commands = vec![0.5, -0.5];
        state.apply_motor_commands(&commands);

        if !state.motors.is_empty() {
            // Target should be set based on command and joint limits
            let motor = &state.motors[0];
            let range = motor.limits.1 - motor.limits.0;
            let mid = (motor.limits.0 + motor.limits.1) / 2.0;
            let expected = mid + 0.5 * (range / 2.0);
            assert!(
                (state.motors[0].target_angle - expected).abs() < 0.01,
                "Expected target {}, got {}",
                expected,
                state.motors[0].target_angle
            );
        }
    }

    #[test]
    fn test_apply_motor_rotations() {
        let morphology = make_test_morphology();
        let mut state = CreaturePhysicsState::new(&morphology, Vec2::new(0.0, 100.0));

        // Set a motor angle
        if !state.motors.is_empty() {
            state.motors[0].angle = PI / 4.0;
        }

        // Apply rotations
        state.apply_motor_rotations(&morphology, Vec2::new(0.0, 100.0));

        // Root should still be at (0, 100)
        assert_eq!(state.parts[0].position, Vec2::new(0.0, 100.0));
    }

    #[test]
    fn test_gravity_application() {
        let morphology = make_test_morphology();
        let mut state = CreaturePhysicsState::new(&morphology, Vec2::new(0.0, 100.0));

        // Mark all parts as not grounded
        for part in &mut state.parts {
            part.grounded = false;
        }

        // Apply gravity
        state.apply_gravity(&morphology, 0.016);

        // All parts should have downward force
        for part in &state.parts {
            assert!(part.force.y < 0.0, "Expected negative y force from gravity");
        }
    }

    #[test]
    fn test_forward_kinematics() {
        let morphology = make_test_morphology();
        let mut state = CreaturePhysicsState::new(&morphology, Vec2::new(0.0, 100.0));

        // Set motor angles
        for motor in &mut state.motors {
            motor.angle = PI / 6.0; // 30 degrees
        }

        // Apply forward kinematics
        state.apply_forward_kinematics(&morphology);

        // Child parts should have moved relative to their parents
        // Just verify they're not at their initial positions
        if state.parts.len() > 1 {
            // The child parts should have different positions due to rotation
            let child_pos = state.parts[1].position;
            let initial_offset = morphology.body_parts[1].local_position;
            // Position should differ from just root + offset due to rotation
            let simple_pos = state.parts[0].position + initial_offset;
            assert!(
                (child_pos - simple_pos).length() > 0.01,
                "Expected child position to differ from simple offset due to rotation"
            );
        }
    }

    #[test]
    fn test_velocity_integration() {
        let morphology = make_test_morphology();
        let mut state = CreaturePhysicsState::new(&morphology, Vec2::new(0.0, 100.0));

        // Apply a force
        state.parts[0].force = Vec2::new(100.0, 0.0);

        // Integrate
        state.integrate(0.1);

        // Velocity should have increased
        assert!(
            state.parts[0].velocity.x > 0.0,
            "Expected positive x velocity after force"
        );
    }

    #[test]
    fn test_center_of_mass() {
        let morphology = make_test_morphology();
        let mut state = CreaturePhysicsState::new(&morphology, Vec2::new(0.0, 100.0));

        state.compute_derived_quantities(&morphology);

        // Center of mass should be near the root for a symmetric creature
        assert!(
            (state.center_of_mass - Vec2::new(0.0, 100.0)).length() < 20.0,
            "Expected center of mass near root"
        );
    }

    #[test]
    fn test_is_any_grounded() {
        let morphology = make_test_morphology();
        let mut state = CreaturePhysicsState::new(&morphology, Vec2::ZERO);

        // Initially not grounded
        assert!(!state.is_any_grounded());

        // Mark one part as grounded
        state.parts[0].grounded = true;
        assert!(state.is_any_grounded());
    }

    #[test]
    fn test_average_velocity() {
        let morphology = make_test_morphology();
        let mut state = CreaturePhysicsState::new(&morphology, Vec2::ZERO);

        // Set velocities
        for part in &mut state.parts {
            part.velocity = Vec2::new(10.0, 0.0);
        }

        let avg = state.get_average_velocity();
        assert!((avg.x - 10.0).abs() < 0.01);
    }
}
