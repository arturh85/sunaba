//! Simple creature physics - no external physics engine
//!
//! Creatures use kinematic position-based movement.
//! Body parts are positioned relative to root with angles.
//! This is WASM-compatible and runs identically in native and SpacetimeDB.

use glam::Vec2;
use serde::{Deserialize, Serialize};
use std::f32::consts::PI;

use crate::morphology::{BodyJoint, CreatureMorphology, JointType};

/// Simple physics state for a creature's body
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct CreaturePhysicsState {
    /// Position of each body part (world coords)
    pub part_positions: Vec<Vec2>,
    /// Rotation of each body part (radians)
    pub part_rotations: Vec<f32>,
    /// Angular velocity of each motor joint (radians/sec)
    pub motor_angular_velocities: Vec<f32>,
    /// Current angle of each motor joint (radians)
    pub motor_angles: Vec<f32>,
    /// Target angle of each motor joint (radians, set by neural network)
    pub motor_target_angles: Vec<f32>,
    /// Which body part index each motor controls
    pub motor_part_indices: Vec<usize>,
}

impl CreaturePhysicsState {
    /// Create new physics state for a creature morphology at the given position
    pub fn new(morphology: &CreatureMorphology, position: Vec2) -> Self {
        let num_parts = morphology.body_parts.len();

        // Find motorized joints (revolute joints) and track their child body part indices
        let motor_part_indices: Vec<usize> = morphology
            .joints
            .iter()
            .filter_map(|joint| {
                if let JointType::Revolute { .. } = joint.joint_type {
                    Some(joint.child_index)
                } else {
                    None
                }
            })
            .collect();

        let num_motors = motor_part_indices.len();

        // Initialize part positions based on morphology local positions
        let part_positions: Vec<Vec2> = morphology
            .body_parts
            .iter()
            .map(|part| position + part.local_position)
            .collect();

        Self {
            part_positions,
            part_rotations: vec![0.0; num_parts],
            motor_angular_velocities: vec![0.0; num_motors],
            motor_angles: vec![0.0; num_motors],
            motor_target_angles: vec![0.0; num_motors],
            motor_part_indices,
        }
    }

    /// Update motor angles based on target angles (spring-damper model)
    pub fn update_motors(&mut self, dt: f32, motor_strength: f32) {
        const DAMPING: f32 = 0.85;
        const MAX_ANGULAR_VEL: f32 = 10.0; // rad/s

        for i in 0..self.motor_angles.len() {
            let target = self.motor_target_angles[i];
            let current = self.motor_angles[i];
            let diff = Self::angle_diff(target, current);

            // Spring force towards target
            let angular_accel = diff * motor_strength;
            self.motor_angular_velocities[i] += angular_accel * dt;
            self.motor_angular_velocities[i] *= DAMPING;
            self.motor_angular_velocities[i] =
                self.motor_angular_velocities[i].clamp(-MAX_ANGULAR_VEL, MAX_ANGULAR_VEL);

            // Integrate velocity
            self.motor_angles[i] += self.motor_angular_velocities[i] * dt;

            // Clamp to valid range
            self.motor_angles[i] = self.motor_angles[i].clamp(-PI, PI);
        }
    }

    /// Apply motor commands from neural network output
    pub fn apply_motor_commands(&mut self, commands: &[f32]) {
        for (i, &cmd) in commands.iter().enumerate() {
            if i < self.motor_target_angles.len() {
                // Neural network outputs are typically -1 to 1, scale to angle range
                self.motor_target_angles[i] = cmd.clamp(-1.0, 1.0) * PI;
            }
        }
    }

    /// Apply motor command to a specific motor (preserves existing functionality)
    pub fn apply_motor_command(
        &mut self,
        motor_index: usize,
        target: f32,
        morphology: &CreatureMorphology,
        delta_time: f32,
    ) -> f32 {
        if motor_index >= self.motor_part_indices.len() {
            return 0.0;
        }

        // Get the joint constraints for this motor
        let link_idx = self.motor_part_indices[motor_index];

        // Find the joint that controls this body part
        let joint = morphology.joints.iter().find(|j| j.child_index == link_idx);

        let (min_angle, max_angle) = match joint {
            Some(BodyJoint {
                joint_type:
                    JointType::Revolute {
                        min_angle,
                        max_angle,
                    },
                ..
            }) => (*min_angle, *max_angle),
            _ => (-PI / 4.0, PI / 4.0),
        };

        // Motor speed constant (radians per second at max command)
        const MAX_ANGULAR_VELOCITY: f32 = 3.0;

        // Calculate target angular velocity from normalized command [-1, 1]
        let target_angular_vel = target.clamp(-1.0, 1.0) * MAX_ANGULAR_VELOCITY;

        // Store angular velocity for this motor
        if motor_index < self.motor_angular_velocities.len() {
            self.motor_angular_velocities[motor_index] = target_angular_vel;
        }

        // Update target angle based on angular velocity
        if motor_index < self.motor_angles.len() {
            let current_angle = self.motor_angles[motor_index];
            let new_angle = current_angle + target_angular_vel * delta_time;
            // Clamp to joint limits
            self.motor_angles[motor_index] = new_angle.clamp(min_angle, max_angle);
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
        let num_motors = self.motor_part_indices.len().min(motor_commands.len());
        for (i, &command) in motor_commands.iter().enumerate().take(num_motors) {
            self.apply_motor_command(i, command, morphology, delta_time);
        }
    }

    /// Update body part positions based on current motor angles
    /// This rotates child body parts around their parent joint pivot
    pub fn apply_motor_rotations(&mut self, morphology: &CreatureMorphology, root_position: Vec2) {
        // First, update root position
        if !self.part_positions.is_empty() {
            self.part_positions[0] = root_position;
        }

        // For each motorized joint, rotate the child body part around the parent
        for (motor_idx, &link_idx) in self.motor_part_indices.iter().enumerate() {
            // Find the joint for this link
            let joint = morphology.joints.iter().find(|j| j.child_index == link_idx);

            if let Some(joint) = joint {
                let target_angle = self.motor_angles.get(motor_idx).copied().unwrap_or(0.0);

                // Get parent body position
                let parent_pos = self
                    .part_positions
                    .get(joint.parent_index)
                    .copied()
                    .unwrap_or(root_position);

                // Get child's local offset from parent
                let parent_part = &morphology.body_parts[joint.parent_index];
                let child_part = &morphology.body_parts[link_idx];
                let local_offset = child_part.local_position - parent_part.local_position;

                // Rotate the local offset by the target angle
                let rotated_offset = Vec2::new(
                    local_offset.x * target_angle.cos() - local_offset.y * target_angle.sin(),
                    local_offset.x * target_angle.sin() + local_offset.y * target_angle.cos(),
                );

                // New child position
                let new_child_pos = parent_pos + rotated_offset;

                // Update child body position and rotation
                if let Some(pos) = self.part_positions.get_mut(link_idx) {
                    *pos = new_child_pos;
                }
                if let Some(rot) = self.part_rotations.get_mut(link_idx) {
                    *rot = target_angle;
                }
            }
        }
    }

    /// Get motor angle for a specific motor index
    pub fn get_motor_angle(&self, motor_idx: usize) -> Option<f32> {
        self.motor_angles.get(motor_idx).copied()
    }

    /// Get angular velocity for a specific motor
    pub fn get_motor_velocity(&self, motor_idx: usize) -> Option<f32> {
        self.motor_angular_velocities.get(motor_idx).copied()
    }

    /// Get number of motors (for neural network output sizing)
    pub fn num_motors(&self) -> usize {
        self.motor_part_indices.len()
    }

    /// Get body part positions for rendering
    pub fn get_body_positions(&self) -> Vec<(Vec2, f32)> {
        self.part_positions
            .iter()
            .zip(self.part_rotations.iter())
            .map(|(&pos, &rot)| (pos, rot))
            .collect()
    }

    /// Get root position
    pub fn get_position(&self) -> Option<Vec2> {
        self.part_positions.first().copied()
    }

    /// Set root position
    pub fn set_position(&mut self, position: Vec2) {
        if !self.part_positions.is_empty() {
            self.part_positions[0] = position;
        }
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

        assert_eq!(state.part_positions.len(), morphology.body_parts.len());
        assert_eq!(state.part_rotations.len(), morphology.body_parts.len());
    }

    #[test]
    fn test_motor_spring() {
        let morphology = make_test_morphology();
        let mut state = CreaturePhysicsState::new(&morphology, Vec2::ZERO);

        if !state.motor_target_angles.is_empty() {
            state.motor_target_angles[0] = 1.0;

            // Simulate many steps with higher motor strength for faster convergence
            for _ in 0..500 {
                state.update_motors(0.016, 50.0);
            }

            // Should approach target (within 0.2 radians)
            assert!(
                (state.motor_angles[0] - 1.0).abs() < 0.2,
                "Motor angle {} should be close to 1.0",
                state.motor_angles[0]
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

        if !state.motor_target_angles.is_empty() {
            assert!((state.motor_target_angles[0] - 0.5 * PI).abs() < 0.01);
        }
    }

    #[test]
    fn test_apply_motor_rotations() {
        let morphology = make_test_morphology();
        let mut state = CreaturePhysicsState::new(&morphology, Vec2::new(0.0, 100.0));

        // Set a motor angle
        if !state.motor_angles.is_empty() {
            state.motor_angles[0] = PI / 4.0;
        }

        // Apply rotations
        state.apply_motor_rotations(&morphology, Vec2::new(0.0, 100.0));

        // Positions should have been updated
        // Root should still be at (0, 100)
        assert_eq!(state.part_positions[0], Vec2::new(0.0, 100.0));
    }
}
