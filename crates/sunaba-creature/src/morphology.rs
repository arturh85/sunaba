//! Morphology generation from CPPN genomes
//!
//! Converts CPPN genomes into articulated physics bodies using rapier2d.

use glam::Vec2;
use rapier2d::prelude::{MultibodyJointHandle, RigidBodyHandle};
use serde::{Deserialize, Serialize};

use super::genome::CreatureGenome;

/// Joint type between body parts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum JointType {
    Revolute { min_angle: f32, max_angle: f32 },
    Fixed,
}

/// Body part specification (physics-independent)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BodyPart {
    pub local_position: Vec2,
    pub radius: f32,
    pub density: f32,
    pub index: usize,
}

/// Joint connecting two body parts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BodyJoint {
    pub parent_index: usize,
    pub child_index: usize,
    pub joint_type: JointType,
}

/// Abstract morphology (physics-independent, serializable)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreatureMorphology {
    pub body_parts: Vec<BodyPart>,
    pub joints: Vec<BodyJoint>,
    pub root_part_index: usize,
    pub total_mass: f32,
}

impl CreatureMorphology {
    /// Generate morphology from CPPN genome
    pub fn from_genome(genome: &CreatureGenome, config: &MorphologyConfig) -> Self {
        use std::collections::HashMap;

        let grid_size = config.grid_resolution;
        let mut body_parts = Vec::new();
        let mut joints = Vec::new();
        let mut part_index = 0;

        // Sample CPPN on a grid to find body parts
        let mut grid: HashMap<(i32, i32), usize> = HashMap::new();

        for y in 0..grid_size {
            for x in 0..grid_size {
                // Convert to normalized coordinates [-1, 1]
                let norm_x = (x as f32 / grid_size as f32) * 2.0 - 1.0;
                let norm_y = (y as f32 / grid_size as f32) * 2.0 - 1.0;
                let distance = (norm_x * norm_x + norm_y * norm_y).sqrt();

                // Query CPPN
                let output = genome.cppn.query(norm_x, norm_y, distance);

                // Only create body part if radius is significant
                if output.radius > 0.3 {
                    let radius =
                        config.min_radius + output.radius * (config.max_radius - config.min_radius);

                    let body_part = BodyPart {
                        local_position: Vec2::new(norm_x * 20.0, norm_y * 20.0), // Scale to world units
                        radius,
                        density: output.density.max(0.1), // Ensure minimum density
                        index: part_index,
                    };

                    body_parts.push(body_part);
                    grid.insert((x as i32, y as i32), part_index);
                    part_index += 1;

                    // Stop if we hit max body parts
                    if part_index >= config.max_body_parts {
                        break;
                    }
                }
            }
            if part_index >= config.max_body_parts {
                break;
            }
        }

        // If no body parts, create at least one
        if body_parts.is_empty() {
            body_parts.push(BodyPart {
                local_position: Vec2::ZERO,
                radius: 5.0,
                density: 1.0,
                index: 0,
            });
        }

        // Connect adjacent body parts with joints
        for y in 0..grid_size as i32 {
            for x in 0..grid_size as i32 {
                if let Some(&current_idx) = grid.get(&(x, y)) {
                    // Check right neighbor
                    if let Some(&right_idx) = grid.get(&(x + 1, y))
                        && current_idx != right_idx
                    {
                        // Query CPPN at midpoint to determine joint type
                        let mid_x = ((x as f32 + 0.5) / grid_size as f32) * 2.0 - 1.0;
                        let mid_y = (y as f32 / grid_size as f32) * 2.0 - 1.0;
                        let mid_d = (mid_x * mid_x + mid_y * mid_y).sqrt();
                        let output = genome.cppn.query(mid_x, mid_y, mid_d);

                        let joint_type = if output.has_joint {
                            JointType::Revolute {
                                min_angle: -std::f32::consts::PI / 4.0,
                                max_angle: std::f32::consts::PI / 4.0,
                            }
                        } else {
                            JointType::Fixed
                        };

                        joints.push(BodyJoint {
                            parent_index: current_idx.min(right_idx),
                            child_index: current_idx.max(right_idx),
                            joint_type,
                        });
                    }

                    // Check down neighbor
                    if let Some(&down_idx) = grid.get(&(x, y + 1))
                        && current_idx != down_idx
                    {
                        let mid_x = (x as f32 / grid_size as f32) * 2.0 - 1.0;
                        let mid_y = ((y as f32 + 0.5) / grid_size as f32) * 2.0 - 1.0;
                        let mid_d = (mid_x * mid_x + mid_y * mid_y).sqrt();
                        let output = genome.cppn.query(mid_x, mid_y, mid_d);

                        let joint_type = if output.has_joint {
                            JointType::Revolute {
                                min_angle: -std::f32::consts::PI / 4.0,
                                max_angle: std::f32::consts::PI / 4.0,
                            }
                        } else {
                            JointType::Fixed
                        };

                        joints.push(BodyJoint {
                            parent_index: current_idx.min(down_idx),
                            child_index: current_idx.max(down_idx),
                            joint_type,
                        });
                    }
                }
            }
        }

        // Calculate total mass
        let total_mass: f32 = body_parts
            .iter()
            .map(|part| {
                let area = std::f32::consts::PI * part.radius * part.radius;
                area * part.density
            })
            .sum();

        Self {
            body_parts,
            joints,
            root_part_index: 0,
            total_mass,
        }
    }

    /// Create simple test morphology (biped)
    pub fn test_biped() -> Self {
        let mut body_parts = Vec::new();
        let mut joints = Vec::new();

        // Central body
        body_parts.push(BodyPart {
            local_position: Vec2::new(0.0, 0.0),
            radius: 8.0,
            density: 1.0,
            index: 0,
        });

        // Left leg
        body_parts.push(BodyPart {
            local_position: Vec2::new(-5.0, 10.0),
            radius: 4.0,
            density: 0.8,
            index: 1,
        });

        // Right leg
        body_parts.push(BodyPart {
            local_position: Vec2::new(5.0, 10.0),
            radius: 4.0,
            density: 0.8,
            index: 2,
        });

        // Connect body to legs with revolute joints
        joints.push(BodyJoint {
            parent_index: 0,
            child_index: 1,
            joint_type: JointType::Revolute {
                min_angle: -std::f32::consts::PI / 3.0,
                max_angle: std::f32::consts::PI / 3.0,
            },
        });

        joints.push(BodyJoint {
            parent_index: 0,
            child_index: 2,
            joint_type: JointType::Revolute {
                min_angle: -std::f32::consts::PI / 3.0,
                max_angle: std::f32::consts::PI / 3.0,
            },
        });

        let total_mass = body_parts
            .iter()
            .map(|p| std::f32::consts::PI * p.radius * p.radius * p.density)
            .sum();

        Self {
            body_parts,
            joints,
            root_part_index: 0,
            total_mass,
        }
    }

    /// Create simple test morphology (quadruped)
    pub fn test_quadruped() -> Self {
        let mut body_parts = Vec::new();
        let mut joints = Vec::new();

        // Central body
        body_parts.push(BodyPart {
            local_position: Vec2::new(0.0, 0.0),
            radius: 10.0,
            density: 1.0,
            index: 0,
        });

        // Four legs
        for i in 0..4 {
            let angle = i as f32 * std::f32::consts::PI / 2.0;
            let offset = 12.0;
            body_parts.push(BodyPart {
                local_position: Vec2::new(angle.cos() * offset, angle.sin() * offset),
                radius: 4.0,
                density: 0.8,
                index: i + 1,
            });

            joints.push(BodyJoint {
                parent_index: 0,
                child_index: i + 1,
                joint_type: JointType::Revolute {
                    min_angle: -std::f32::consts::PI / 4.0,
                    max_angle: std::f32::consts::PI / 4.0,
                },
            });
        }

        let total_mass = body_parts
            .iter()
            .map(|p| std::f32::consts::PI * p.radius * p.radius * p.density)
            .sum();

        Self {
            body_parts,
            joints,
            root_part_index: 0,
            total_mass,
        }
    }

    /// Validate morphology (connectivity, mass distribution)
    pub fn validate(&self) -> Result<(), String> {
        // Check we have at least one body part
        if self.body_parts.is_empty() {
            return Err("Morphology has no body parts".to_string());
        }

        // Check root index is valid
        if self.root_part_index >= self.body_parts.len() {
            return Err(format!(
                "Invalid root index: {} (only {} parts)",
                self.root_part_index,
                self.body_parts.len()
            ));
        }

        // Check all joint indices are valid
        for joint in &self.joints {
            if joint.parent_index >= self.body_parts.len() {
                return Err(format!(
                    "Invalid parent index in joint: {}",
                    joint.parent_index
                ));
            }
            if joint.child_index >= self.body_parts.len() {
                return Err(format!(
                    "Invalid child index in joint: {}",
                    joint.child_index
                ));
            }
        }

        // Check total mass is reasonable
        if self.total_mass <= 0.0 {
            return Err(format!("Invalid total mass: {}", self.total_mass));
        }

        Ok(())
    }
}

/// Configuration for morphology generation
#[derive(Debug, Clone)]
pub struct MorphologyConfig {
    pub grid_resolution: usize, // Sample CPPN at NxN grid
    pub max_body_parts: usize,
    pub min_radius: f32,
    pub max_radius: f32,
}

impl Default for MorphologyConfig {
    fn default() -> Self {
        Self {
            grid_resolution: 8,
            max_body_parts: 20,
            min_radius: 2.0,
            max_radius: 10.0,
        }
    }
}

/// Runtime physics representation (not serialized)
pub struct MorphologyPhysics {
    pub multibody_handle: MultibodyJointHandle,
    pub link_handles: Vec<RigidBodyHandle>,
    /// Indices of body parts that have motorized joints (revolute joints only)
    pub motor_link_indices: Vec<usize>,
    /// Current target angles for each motor joint (matches motor_link_indices order)
    pub motor_target_angles: Vec<f32>,
    /// Current angular velocities for motor interpolation
    pub motor_angular_velocities: Vec<f32>,
}

impl MorphologyPhysics {
    /// Build rapier2d bodies from morphology
    /// Uses kinematic bodies so we control position manually via pixel-based collision
    pub fn from_morphology(
        morphology: &CreatureMorphology,
        position: Vec2,
        physics_world: &mut crate::physics::PhysicsWorld,
    ) -> Self {
        use rapier2d::prelude::*;

        let mut link_handles = Vec::new();

        // Create kinematic rigid bodies for each body part
        // We use kinematic_position_based so we can set position directly
        for part in &morphology.body_parts {
            let world_pos = position + part.local_position;

            let rigid_body = RigidBodyBuilder::kinematic_position_based()
                .translation(vector![world_pos.x, world_pos.y])
                .build();

            let body_handle = physics_world.rigid_body_set_mut().insert(rigid_body);

            link_handles.push(body_handle);
        }

        // Create colliders in a second pass (for rendering and future collision)
        for (i, part) in morphology.body_parts.iter().enumerate() {
            let collider = ColliderBuilder::ball(part.radius)
                .density(part.density)
                .friction(0.5)
                .restitution(0.1)
                .build();

            let body_handle = link_handles[i];
            physics_world.add_collider_with_parent(collider, body_handle);
        }

        // Find motorized joints (revolute joints) and track their child body part indices
        let mut motor_link_indices = Vec::new();
        for joint in &morphology.joints {
            if let JointType::Revolute { .. } = joint.joint_type {
                // The child body part of a revolute joint can be motorized
                motor_link_indices.push(joint.child_index);
            }
        }

        // Initialize target angles and velocities for motors
        let num_motors = motor_link_indices.len();
        let motor_target_angles = vec![0.0; num_motors];
        let motor_angular_velocities = vec![0.0; num_motors];

        // For Phase 6, we'll use a placeholder multibody handle
        // In a future phase, we can create actual multibody joints
        let multibody_handle = MultibodyJointHandle::from_raw_parts(0, 0);

        Self {
            multibody_handle,
            link_handles,
            motor_link_indices,
            motor_target_angles,
            motor_angular_velocities,
        }
    }

    /// Remove physics body from world
    pub fn cleanup(self, physics_world: &mut crate::physics::PhysicsWorld) {
        // Remove all rigid bodies
        for handle in self.link_handles {
            physics_world.remove_rigid_body(handle);
        }
    }

    /// Get current position of root body part
    pub fn get_position(&self, physics_world: &crate::physics::PhysicsWorld) -> Option<Vec2> {
        // Get position of first body part (root)
        self.link_handles.first().and_then(|&handle| {
            physics_world.rigid_body_set().get(handle).map(|rb| {
                let pos = rb.translation();
                Vec2::new(pos.x, pos.y)
            })
        })
    }

    /// Apply motor command to joint
    /// target is the target angular velocity in radians per second [-1, 1] normalized
    /// Returns the actual angular velocity applied
    pub fn apply_motor_command(
        &mut self,
        motor_index: usize,
        target: f32,
        morphology: &CreatureMorphology,
        delta_time: f32,
    ) -> f32 {
        if motor_index >= self.motor_link_indices.len() {
            return 0.0;
        }

        // Get the joint constraints for this motor
        let link_idx = self.motor_link_indices[motor_index];

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
            _ => (-std::f32::consts::PI / 4.0, std::f32::consts::PI / 4.0),
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
        if motor_index < self.motor_target_angles.len() {
            let current_target = self.motor_target_angles[motor_index];
            let new_target = current_target + target_angular_vel * delta_time;
            // Clamp to joint limits
            self.motor_target_angles[motor_index] = new_target.clamp(min_angle, max_angle);
        }

        target_angular_vel
    }

    /// Apply all motor commands from neural network output
    /// motor_commands should be a slice of values in [-1, 1], one per motor
    pub fn apply_all_motor_commands(
        &mut self,
        motor_commands: &[f32],
        morphology: &CreatureMorphology,
        delta_time: f32,
    ) {
        let num_motors = self.motor_link_indices.len().min(motor_commands.len());
        for (i, &command) in motor_commands.iter().enumerate().take(num_motors) {
            self.apply_motor_command(i, command, morphology, delta_time);
        }
    }

    /// Apply motor target angles to physics bodies
    /// This rotates child body parts around their parent joint pivot
    pub fn apply_motor_rotations(
        &self,
        morphology: &CreatureMorphology,
        root_position: Vec2,
        physics_world: &mut crate::physics::PhysicsWorld,
    ) {
        use rapier2d::prelude::*;

        // Start with root position and work outward
        // For each motorized joint, rotate the child body part around the parent

        for (motor_idx, &link_idx) in self.motor_link_indices.iter().enumerate() {
            // Find the joint for this link
            let joint = morphology.joints.iter().find(|j| j.child_index == link_idx);

            if let Some(joint) = joint {
                let target_angle = self
                    .motor_target_angles
                    .get(motor_idx)
                    .copied()
                    .unwrap_or(0.0);

                // Get parent body position
                let parent_handle = self.link_handles.get(joint.parent_index);
                let child_handle = self.link_handles.get(link_idx);

                if let (Some(&parent_handle), Some(&child_handle)) = (parent_handle, child_handle) {
                    // Get parent position
                    let parent_pos = physics_world
                        .rigid_body_set()
                        .get(parent_handle)
                        .map(|rb| {
                            let t = rb.translation();
                            Vec2::new(t.x, t.y)
                        })
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
                    if let Some(rb) = physics_world.rigid_body_set_mut().get_mut(child_handle) {
                        rb.set_translation(vector![new_child_pos.x, new_child_pos.y], true);
                        rb.set_rotation(Rotation::new(target_angle), true);
                    }
                }
            }
        }
    }

    /// Get number of motors (for neural network output sizing)
    pub fn num_motors(&self) -> usize {
        self.motor_link_indices.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::genome::CreatureGenome;

    #[test]
    fn test_biped_morphology() {
        let morph = CreatureMorphology::test_biped();

        // Should have 3 body parts (body + 2 legs)
        assert_eq!(morph.body_parts.len(), 3);

        // Should have 2 joints
        assert_eq!(morph.joints.len(), 2);

        // Should validate successfully
        assert!(morph.validate().is_ok());

        // Root should be first part
        assert_eq!(morph.root_part_index, 0);

        // Total mass should be positive
        assert!(morph.total_mass > 0.0);
    }

    #[test]
    fn test_quadruped_morphology() {
        let morph = CreatureMorphology::test_quadruped();

        // Should have 5 body parts (body + 4 legs)
        assert_eq!(morph.body_parts.len(), 5);

        // Should have 4 joints
        assert_eq!(morph.joints.len(), 4);

        // Should validate successfully
        assert!(morph.validate().is_ok());
    }

    #[test]
    fn test_morphology_from_genome() {
        let genome = CreatureGenome::test_biped();
        let config = MorphologyConfig::default();

        let morph = CreatureMorphology::from_genome(&genome, &config);

        // Should have at least one body part
        assert!(!morph.body_parts.is_empty());

        // Should validate
        assert!(morph.validate().is_ok());

        // Total mass should be positive
        assert!(morph.total_mass > 0.0);
    }

    #[test]
    fn test_morphology_validation() {
        let morph = CreatureMorphology::test_biped();
        assert!(morph.validate().is_ok());

        // Create invalid morphology (empty)
        let invalid_morph = CreatureMorphology {
            body_parts: Vec::new(),
            joints: Vec::new(),
            root_part_index: 0,
            total_mass: 0.0,
        };

        assert!(invalid_morph.validate().is_err());
    }

    #[test]
    fn test_body_part_properties() {
        let morph = CreatureMorphology::test_biped();

        // Check root body part
        let root = &morph.body_parts[morph.root_part_index];
        assert!(root.radius > 0.0);
        assert!(root.density > 0.0);
        assert_eq!(root.index, 0);
    }

    #[test]
    fn test_joint_types() {
        let morph = CreatureMorphology::test_biped();

        for joint in &morph.joints {
            match joint.joint_type {
                JointType::Revolute {
                    min_angle,
                    max_angle,
                } => {
                    assert!(max_angle > min_angle);
                }
                JointType::Fixed => {}
            }
        }
    }

    #[test]
    fn test_morphology_config_defaults() {
        let config = MorphologyConfig::default();
        assert_eq!(config.grid_resolution, 8);
        assert_eq!(config.max_body_parts, 20);
        assert_eq!(config.min_radius, 2.0);
        assert_eq!(config.max_radius, 10.0);
    }
}
