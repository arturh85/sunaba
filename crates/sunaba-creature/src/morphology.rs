//! Morphology generation from CPPN genomes
//!
//! Converts CPPN genomes into articulated body structures.

use glam::Vec2;
use serde::{Deserialize, Serialize};

use super::genome::CreatureGenome;

/// Creature archetype for distinct body plans
/// Each archetype produces a morphology optimized for different locomotion styles
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum CreatureArchetype {
    /// Default: morphology generated from CPPN genome
    #[default]
    Evolved,
    /// Spider: central body with 8 radial legs (fast multi-legged crawling)
    Spider,
    /// Snake: chain of 6 connected segments (slithering wave propagation)
    Snake,
    /// Worm: 4 segments with very flexible joints (accordion compression)
    Worm,
    /// Flyer: central body with wing appendages (oscillation-based flight)
    Flyer,
}

impl CreatureArchetype {
    /// Get all available archetypes (excluding Evolved)
    pub fn all_fixed() -> &'static [CreatureArchetype] {
        &[
            CreatureArchetype::Spider,
            CreatureArchetype::Snake,
            CreatureArchetype::Worm,
            CreatureArchetype::Flyer,
        ]
    }

    /// Get all available archetypes (including Evolved)
    pub fn all_with_evolved() -> &'static [CreatureArchetype] {
        &[
            CreatureArchetype::Evolved,
            CreatureArchetype::Spider,
            CreatureArchetype::Snake,
            CreatureArchetype::Worm,
            CreatureArchetype::Flyer,
        ]
    }

    /// Create the morphology for this archetype
    pub fn create_morphology(
        &self,
        genome: &CreatureGenome,
        config: &MorphologyConfig,
    ) -> CreatureMorphology {
        match self {
            CreatureArchetype::Evolved => CreatureMorphology::from_genome(genome, config),
            CreatureArchetype::Spider => CreatureMorphology::archetype_spider(),
            CreatureArchetype::Snake => CreatureMorphology::archetype_snake(),
            CreatureArchetype::Worm => CreatureMorphology::archetype_worm(),
            CreatureArchetype::Flyer => CreatureMorphology::archetype_flyer(),
        }
    }

    /// Human-readable name
    pub fn name(&self) -> &'static str {
        match self {
            CreatureArchetype::Evolved => "Evolved",
            CreatureArchetype::Spider => "Spider",
            CreatureArchetype::Snake => "Snake",
            CreatureArchetype::Worm => "Worm",
            CreatureArchetype::Flyer => "Flyer",
        }
    }
}

impl std::fmt::Display for CreatureArchetype {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}

impl std::str::FromStr for CreatureArchetype {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "evolved" | "default" => Ok(CreatureArchetype::Evolved),
            "spider" => Ok(CreatureArchetype::Spider),
            "snake" => Ok(CreatureArchetype::Snake),
            "worm" => Ok(CreatureArchetype::Worm),
            "flyer" | "flying" => Ok(CreatureArchetype::Flyer),
            _ => Err(format!(
                "Unknown archetype: {}. Valid: evolved, spider, snake, worm, flyer",
                s
            )),
        }
    }
}

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
    /// Whether this body part acts as a wing (generates lift when oscillating)
    #[serde(default)]
    pub is_wing: bool,
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
                        is_wing: false, // Wings are specified by archetype, not CPPN (for now)
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
                is_wing: false,
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
            is_wing: false,
        });

        // Left leg
        body_parts.push(BodyPart {
            local_position: Vec2::new(-5.0, 10.0),
            radius: 4.0,
            density: 0.8,
            index: 1,
            is_wing: false,
        });

        // Right leg
        body_parts.push(BodyPart {
            local_position: Vec2::new(5.0, 10.0),
            radius: 4.0,
            density: 0.8,
            index: 2,
            is_wing: false,
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
            is_wing: false,
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
                is_wing: false,
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

    // ===== Archetype Morphologies =====
    // These create distinct body plans for creature diversity

    /// Spider archetype: central body with 8 radial legs
    /// Designed for fast, coordinated multi-legged locomotion
    pub fn archetype_spider() -> Self {
        let mut body_parts = Vec::new();
        let mut joints = Vec::new();

        // Large central body (spider cephalothorax)
        body_parts.push(BodyPart {
            local_position: Vec2::new(0.0, 0.0),
            radius: 10.0,
            density: 1.2,
            index: 0,
            is_wing: false,
        });

        // 8 legs distributed radially at 45° intervals
        // Legs extend outward and slightly downward
        for i in 0..8 {
            let angle = i as f32 * std::f32::consts::PI / 4.0; // 45° apart
            let leg_offset = 14.0; // Distance from center
            let x = angle.cos() * leg_offset;
            let y = angle.sin() * leg_offset + 2.0; // Slightly below center

            body_parts.push(BodyPart {
                local_position: Vec2::new(x, y),
                radius: 3.0,
                density: 0.6,
                index: i + 1,
                is_wing: false,
            });

            // Flexible revolute joints for leg movement
            joints.push(BodyJoint {
                parent_index: 0,
                child_index: i + 1,
                joint_type: JointType::Revolute {
                    min_angle: -std::f32::consts::PI / 3.0, // 60° range
                    max_angle: std::f32::consts::PI / 3.0,
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

    /// Snake archetype: chain of 6 connected segments
    /// Designed for slithering wave propagation locomotion
    pub fn archetype_snake() -> Self {
        let mut body_parts = Vec::new();
        let mut joints = Vec::new();

        // Head (front, larger)
        body_parts.push(BodyPart {
            local_position: Vec2::new(0.0, 0.0),
            radius: 6.0,
            density: 1.0,
            index: 0,
            is_wing: false,
        });

        // Body segments (5 more, tapering toward tail)
        let segment_spacing = 10.0;
        for i in 1..6 {
            let radius = 5.5 - (i as f32 * 0.5); // Taper from 5.0 to 3.0
            body_parts.push(BodyPart {
                local_position: Vec2::new(-(i as f32 * segment_spacing), 0.0),
                radius: radius.max(3.0),
                density: 0.8,
                index: i,
                is_wing: false,
            });

            // Connect to previous segment with flexible joint
            joints.push(BodyJoint {
                parent_index: i - 1,
                child_index: i,
                joint_type: JointType::Revolute {
                    min_angle: -std::f32::consts::PI / 4.0, // 45° each way
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

    /// Worm archetype: 4 segments with very flexible joints
    /// Designed for accordion-style compression/expansion locomotion
    pub fn archetype_worm() -> Self {
        let mut body_parts = Vec::new();
        let mut joints = Vec::new();

        // 4 equal-sized segments
        let segment_spacing = 10.0;
        for i in 0..4 {
            body_parts.push(BodyPart {
                local_position: Vec2::new(-(i as f32 * segment_spacing), 0.0),
                radius: 5.0,
                density: 0.9,
                index: i,
                is_wing: false,
            });

            if i > 0 {
                // Very flexible joints for accordion motion
                joints.push(BodyJoint {
                    parent_index: i - 1,
                    child_index: i,
                    joint_type: JointType::Revolute {
                        min_angle: -std::f32::consts::PI / 2.5, // ~72° each way (very flexible)
                        max_angle: std::f32::consts::PI / 2.5,
                    },
                });
            }
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

    /// Flyer archetype: central body with wing appendages
    /// Designed for flight via wing oscillation lift
    pub fn archetype_flyer() -> Self {
        let mut body_parts = Vec::new();
        let mut joints = Vec::new();

        // Central body (compact)
        body_parts.push(BodyPart {
            local_position: Vec2::new(0.0, 0.0),
            radius: 7.0,
            density: 1.0,
            index: 0,
            is_wing: false,
        });

        // Right wing (extends outward and slightly up)
        body_parts.push(BodyPart {
            local_position: Vec2::new(12.0, -3.0),
            radius: 4.0,
            density: 0.3, // Light wing
            index: 1,
            is_wing: true, // This is a wing!
        });

        // Left wing (mirror of right)
        body_parts.push(BodyPart {
            local_position: Vec2::new(-12.0, -3.0),
            radius: 4.0,
            density: 0.3,
            index: 2,
            is_wing: true, // This is a wing!
        });

        // Tail/stabilizer (below and behind)
        body_parts.push(BodyPart {
            local_position: Vec2::new(0.0, 8.0),
            radius: 3.0,
            density: 0.5,
            index: 3,
            is_wing: false,
        });

        // Wing joints (need good range of motion for flapping)
        joints.push(BodyJoint {
            parent_index: 0,
            child_index: 1, // Right wing
            joint_type: JointType::Revolute {
                min_angle: -std::f32::consts::PI / 2.0, // 90° range for flapping
                max_angle: std::f32::consts::PI / 2.0,
            },
        });

        joints.push(BodyJoint {
            parent_index: 0,
            child_index: 2, // Left wing
            joint_type: JointType::Revolute {
                min_angle: -std::f32::consts::PI / 2.0,
                max_angle: std::f32::consts::PI / 2.0,
            },
        });

        // Tail joint (limited motion for steering)
        joints.push(BodyJoint {
            parent_index: 0,
            child_index: 3,
            joint_type: JointType::Revolute {
                min_angle: -std::f32::consts::PI / 6.0, // 30° each way
                max_angle: std::f32::consts::PI / 6.0,
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

impl MorphologyConfig {
    /// Simple morphology config for basic locomotion training
    /// Produces creatures with 3-6 body parts that are easier to evolve
    pub fn simple() -> Self {
        Self {
            grid_resolution: 4, // 4x4 = 16 positions max (was 8x8=64)
            max_body_parts: 6,  // Max 6 parts (was 20)
            min_radius: 4.0,    // Larger, chunkier parts
            max_radius: 8.0,
        }
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

    /// Generate sample creatures and print statistics for analysis
    /// Run with: cargo test -p sunaba-creature analyze_morphology_statistics -- --nocapture
    #[test]
    fn analyze_morphology_statistics() {
        use crate::genome::MutationConfig;

        let configs = [
            ("default", MorphologyConfig::default()),
            ("simple", MorphologyConfig::simple()),
        ];

        for (config_name, config) in configs {
            println!("\n=== {} Morphology Config ===", config_name);
            println!(
                "Grid: {}x{}, Max parts: {}, Radius: {}-{}",
                config.grid_resolution,
                config.grid_resolution,
                config.max_body_parts,
                config.min_radius,
                config.max_radius
            );

            let mut total_parts = 0;
            let mut total_joints = 0;
            let mut total_motors = 0;
            let mut total_mass = 0.0;
            let num_samples = 20;

            let mutation_config = MutationConfig::default();

            for i in 0..num_samples {
                let mut genome = CreatureGenome::test_biped();
                // Apply multiple mutations to create variety
                for _ in 0..i {
                    genome.mutate(&mutation_config, 0.5);
                }

                let morph = CreatureMorphology::from_genome(&genome, &config);

                let motor_count = morph
                    .joints
                    .iter()
                    .filter(|j| matches!(j.joint_type, JointType::Revolute { .. }))
                    .count();

                total_parts += morph.body_parts.len();
                total_joints += morph.joints.len();
                total_motors += motor_count;
                total_mass += morph.total_mass;

                if i < 5 {
                    // Print details for first 5
                    println!(
                        "  Sample {}: {} parts, {} joints ({} motors), mass={:.1}",
                        i,
                        morph.body_parts.len(),
                        morph.joints.len(),
                        motor_count,
                        morph.total_mass
                    );

                    // Print part positions
                    for (idx, part) in morph.body_parts.iter().enumerate() {
                        let marker = if idx == morph.root_part_index {
                            "[ROOT]"
                        } else {
                            ""
                        };
                        println!(
                            "    Part {}: pos=({:.1},{:.1}), r={:.1}, d={:.2} {}",
                            idx,
                            part.local_position.x,
                            part.local_position.y,
                            part.radius,
                            part.density,
                            marker
                        );
                    }
                }
            }

            println!("\n--- Statistics over {} samples ---", num_samples);
            println!(
                "Avg body parts: {:.1}",
                total_parts as f32 / num_samples as f32
            );
            println!(
                "Avg joints: {:.1}",
                total_joints as f32 / num_samples as f32
            );
            println!(
                "Avg motors: {:.1}",
                total_motors as f32 / num_samples as f32
            );
            println!("Avg mass: {:.1}", total_mass / num_samples as f32);
        }
    }
}
