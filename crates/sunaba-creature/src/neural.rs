//! Graph Neural Network controller for creatures
//!
//! Implements NerveNet-style GNN that adapts to variable morphologies.

use glam::Vec2;
use ndarray::{ArrayView1, ArrayView2};

use super::genome::ControllerGenome;
use super::morphology::CreatureMorphology;

/// Input features per body part (fed into GNN)
#[derive(Debug, Clone)]
pub struct BodyPartFeatures {
    pub joint_angle: f32,
    pub joint_angular_velocity: f32,
    pub orientation: f32,
    pub velocity: Vec2,
    pub ground_contact: f32,
    pub raycast_distances: Vec<f32>,
    pub contact_materials: Vec<f32>,
    // Global food direction (same for all body parts - provides navigation context)
    pub food_direction_x: f32, // -1.0 to 1.0, direction to nearest food
    pub food_direction_y: f32, // -1.0 to 1.0
    pub food_distance: f32,    // 0.0 to 1.0, normalized distance to food
}

impl BodyPartFeatures {
    /// Convert to flat feature vector
    pub fn to_vec(&self) -> Vec<f32> {
        let mut features = vec![
            self.joint_angle,
            self.joint_angular_velocity,
            self.orientation,
            self.velocity.x,
            self.velocity.y,
            self.ground_contact,
            self.food_direction_x,
            self.food_direction_y,
            self.food_distance,
        ];
        features.extend(&self.raycast_distances);
        features.extend(&self.contact_materials);
        features
    }

    /// Get feature dimension
    pub fn feature_dim(num_raycasts: usize, num_materials: usize) -> usize {
        9 + num_raycasts + num_materials // 6 base + 3 food direction + raycasts + materials
    }
}

/// Graph structure for GNN (mirrors morphology)
#[derive(Debug, Clone)]
pub struct MorphologyGraph {
    pub node_features: Vec<Vec<f32>>, // Per-node feature vectors
    pub edges: Vec<(usize, usize)>,   // Adjacency list
    pub num_nodes: usize,
}

impl MorphologyGraph {
    /// Build graph from morphology
    pub fn from_morphology(morphology: &CreatureMorphology) -> Self {
        let num_nodes = morphology.body_parts.len();
        let mut edges = Vec::new();

        // Build adjacency list from joints
        for joint in &morphology.joints {
            edges.push((joint.parent_index, joint.child_index));
            edges.push((joint.child_index, joint.parent_index)); // Bidirectional
        }

        Self {
            node_features: vec![Vec::new(); num_nodes],
            edges,
            num_nodes,
        }
    }
}

/// Simple feedforward neural network (Phase 6 implementation)
/// Full GNN with burn will be implemented in later stage
pub struct SimpleNeuralController {
    weights: Vec<f32>,
    hidden_dim: usize,
    input_dim: usize,
    output_dim: usize,
}

impl SimpleNeuralController {
    /// Create from genome weights
    /// Flattens GNN weights into simple feedforward weights for Phase 6
    pub fn from_genome(genome: &ControllerGenome, input_dim: usize, output_dim: usize) -> Self {
        let hidden_dim = genome.hidden_dim;

        // For Phase 6: Flatten all genome weights into simple feedforward structure
        // Later we'll use proper GNN with message passing
        let mut weights = Vec::new();
        weights.extend(&genome.message_weights);
        weights.extend(&genome.update_weights);
        weights.extend(&genome.output_weights);

        // Resize to match input/hidden/output architecture
        let expected_size = input_dim * hidden_dim + hidden_dim * output_dim;
        weights.resize(expected_size, 0.0);

        Self {
            weights,
            hidden_dim,
            input_dim,
            output_dim,
        }
    }

    /// Create random controller for testing
    #[cfg(feature = "evolution")]
    pub fn random(input_dim: usize, hidden_dim: usize, output_dim: usize) -> Self {
        use rand::Rng;
        let mut rng = rand::thread_rng();

        // Calculate weight count: input->hidden + hidden->output
        let input_to_hidden = input_dim * hidden_dim;
        let hidden_to_output = hidden_dim * output_dim;
        let total_weights = input_to_hidden + hidden_to_output;

        // Initialize with Xavier/Glorot uniform distribution
        let weights: Vec<f32> = (0..total_weights)
            .map(|_| rng.gen_range(-0.5..0.5))
            .collect();

        Self {
            weights,
            hidden_dim,
            input_dim,
            output_dim,
        }
    }

    /// Get input dimension
    pub fn input_dim(&self) -> usize {
        self.input_dim
    }

    /// Get output dimension
    pub fn output_dim(&self) -> usize {
        self.output_dim
    }

    /// Forward pass: features -> motor commands
    /// Simple 2-layer network: input -> hidden (tanh) -> output (tanh)
    /// Uses ndarray for BLAS-accelerated matrix operations
    pub fn forward(&self, input: &[f32]) -> Vec<f32> {
        assert_eq!(input.len(), self.input_dim, "Input dimension mismatch");

        // Convert input to Array1
        let input_array = ArrayView1::from(input);

        // Layer 1: input -> hidden
        // Weight matrix is stored as [hidden_dim, input_dim] in row-major order
        let w1_slice = &self.weights[0..self.input_dim * self.hidden_dim];
        let w1 = ArrayView2::from_shape((self.hidden_dim, self.input_dim), w1_slice)
            .expect("Failed to reshape input->hidden weights");

        // Matrix-vector multiplication: hidden = W1 * input
        let hidden_pre = w1.dot(&input_array);
        // Apply tanh activation
        let hidden = hidden_pre.mapv(|x| x.tanh());

        // Layer 2: hidden -> output
        let hidden_to_output_start = self.input_dim * self.hidden_dim;
        let w2_slice = &self.weights[hidden_to_output_start..];
        let w2 = ArrayView2::from_shape((self.output_dim, self.hidden_dim), w2_slice)
            .expect("Failed to reshape hidden->output weights");

        // Matrix-vector multiplication: output = W2 * hidden
        let output_pre = w2.dot(&hidden);
        // Apply tanh activation
        let output = output_pre.mapv(|x| x.tanh());

        output.to_vec()
    }
}

/// Deep neural controller with two hidden layers and optional recurrence
/// Architecture: input -> hidden1 (tanh) -> hidden2 (tanh) -> output (tanh)
/// Provides more representational capacity for learning complex gaits
pub struct DeepNeuralController {
    weights: Vec<f32>,
    hidden1_dim: usize, // First hidden layer (larger)
    hidden2_dim: usize, // Second hidden layer (smaller)
    input_dim: usize,
    output_dim: usize,
    /// Previous hidden state for simple recurrence
    prev_hidden: Option<Vec<f32>>,
    /// Recurrence blend factor (0.0 = no recurrence, 1.0 = full recurrence)
    recurrence_factor: f32,
}

impl DeepNeuralController {
    /// Create from genome weights with two hidden layers
    /// Scales architecture based on genome's hidden_dim:
    /// - hidden_dim=16 (biped) -> 48, 24
    /// - hidden_dim=24 (quadruped) -> 72, 36
    pub fn from_genome(genome: &ControllerGenome, input_dim: usize, output_dim: usize) -> Self {
        use crate::deterministic_rng::DeterministicRng;

        // Scale hidden layer sizes from genome's hidden_dim
        let scale = genome.hidden_dim as f32 / 16.0;
        let hidden1_dim = (48.0 * scale).round() as usize;
        let hidden2_dim = (24.0 * scale).round() as usize;

        // Expected weight count for 2-layer network:
        // input->hidden1 + hidden1->hidden2 + hidden2->output
        let expected_size =
            input_dim * hidden1_dim + hidden1_dim * hidden2_dim + hidden2_dim * output_dim;

        // Create seeded RNG from genome weights for deterministic initialization
        // This ensures same genome always produces same network
        let seed: u64 = genome
            .message_weights
            .iter()
            .chain(genome.update_weights.iter())
            .chain(genome.output_weights.iter())
            .fold(0u64, |acc, &w| acc.wrapping_add((w * 1000.0) as i64 as u64));
        let mut rng = DeterministicRng::from_seed(seed);

        // Generate all weights randomly, seeded by genome
        // Xavier/Glorot initialization scaled by layer sizes
        let mut weights = Vec::with_capacity(expected_size);
        for i in 0..expected_size {
            // Determine which layer this weight belongs to for proper scaling
            let in_h1 = input_dim * hidden1_dim;
            let h1_h2 = hidden1_dim * hidden2_dim;

            let scale = if i < in_h1 {
                (2.0 / (input_dim + hidden1_dim) as f32).sqrt()
            } else if i < in_h1 + h1_h2 {
                (2.0 / (hidden1_dim + hidden2_dim) as f32).sqrt()
            } else {
                (2.0 / (hidden2_dim + output_dim) as f32).sqrt()
            };

            weights.push(rng.gen_range_f32(-1.0, 1.0) * scale);
        }

        Self {
            weights,
            hidden1_dim,
            hidden2_dim,
            input_dim,
            output_dim,
            prev_hidden: None,
            recurrence_factor: 0.3, // Blend 30% of previous hidden state
        }
    }

    /// Create random controller for testing
    #[cfg(feature = "evolution")]
    pub fn random(
        input_dim: usize,
        hidden1_dim: usize,
        hidden2_dim: usize,
        output_dim: usize,
    ) -> Self {
        use rand::Rng;
        let mut rng = rand::thread_rng();

        let total_weights =
            input_dim * hidden1_dim + hidden1_dim * hidden2_dim + hidden2_dim * output_dim;

        // Xavier/Glorot initialization
        let weights: Vec<f32> = (0..total_weights)
            .map(|_| rng.gen_range(-0.5..0.5))
            .collect();

        Self {
            weights,
            hidden1_dim,
            hidden2_dim,
            input_dim,
            output_dim,
            prev_hidden: None,
            recurrence_factor: 0.3,
        }
    }

    /// Get input dimension
    pub fn input_dim(&self) -> usize {
        self.input_dim
    }

    /// Get output dimension
    pub fn output_dim(&self) -> usize {
        self.output_dim
    }

    /// Reset hidden state (call between episodes/evaluations)
    pub fn reset_hidden(&mut self) {
        self.prev_hidden = None;
    }

    /// Forward pass with two hidden layers and optional recurrence
    /// input -> hidden1 (tanh) -> hidden2 (tanh) -> output (tanh)
    /// Uses ndarray for BLAS-accelerated matrix operations
    pub fn forward(&mut self, input: &[f32]) -> Vec<f32> {
        #[cfg(feature = "profiling")]
        puffin::profile_function!();

        assert_eq!(input.len(), self.input_dim, "Input dimension mismatch");

        // Convert input to Array1
        let input_array = ArrayView1::from(input);

        let mut offset = 0;

        // Layer 1: input -> hidden1
        let w1_end = self.input_dim * self.hidden1_dim;
        let w1_slice = &self.weights[offset..w1_end];
        let w1 = ArrayView2::from_shape((self.hidden1_dim, self.input_dim), w1_slice)
            .expect("Failed to reshape input->hidden1 weights");
        offset = w1_end;

        // Matrix-vector multiplication: hidden1 = W1 * input
        let hidden1_pre = w1.dot(&input_array);
        let hidden1 = hidden1_pre.mapv(|x| x.tanh());

        // Layer 2: hidden1 -> hidden2
        let w2_end = offset + self.hidden1_dim * self.hidden2_dim;
        let w2_slice = &self.weights[offset..w2_end];
        let w2 = ArrayView2::from_shape((self.hidden2_dim, self.hidden1_dim), w2_slice)
            .expect("Failed to reshape hidden1->hidden2 weights");
        offset = w2_end;

        // Matrix-vector multiplication: hidden2 = W2 * hidden1
        let hidden2_pre = w2.dot(&hidden1);
        let mut hidden2 = hidden2_pre.mapv(|x| x.tanh());

        // Simple recurrence: blend with previous hidden state
        if let Some(ref prev) = self.prev_hidden {
            let blend = self.recurrence_factor;
            let prev_array = ArrayView1::from(prev);
            let min_dim = self.hidden2_dim.min(prev.len());

            // Blend current hidden2 with previous hidden state
            for h in 0..min_dim {
                hidden2[h] = (1.0 - blend) * hidden2[h] + blend * prev_array[h];
            }
        }
        self.prev_hidden = Some(hidden2.to_vec());

        // Layer 3: hidden2 -> output
        let w3_slice = &self.weights[offset..];
        let w3 = ArrayView2::from_shape((self.output_dim, self.hidden2_dim), w3_slice)
            .expect("Failed to reshape hidden2->output weights");

        // Matrix-vector multiplication: output = W3 * hidden2
        let output_pre = w3.dot(&hidden2);
        let output = output_pre.mapv(|x| x.tanh());

        output.to_vec()
    }
}

/// Extract features from simple physics state
/// Uses CreaturePhysicsState for position-based physics without rapier2d
pub fn extract_body_part_features_simple(
    morphology: &CreatureMorphology,
    physics_state: &super::simple_physics::CreaturePhysicsState,
    sensory_input: &super::sensors::SensoryInput,
    world: &impl crate::WorldAccess,
) -> Vec<BodyPartFeatures> {
    let num_parts = morphology.body_parts.len();
    let mut features = Vec::with_capacity(num_parts);

    // Extract global food direction from sensory input
    let (food_direction_x, food_direction_y, food_distance) = match sensory_input.food_direction {
        Some(dir) => (dir.x, dir.y, sensory_input.food_distance),
        None => (0.0, 0.0, 1.0), // No food detected - zero direction, max distance
    };

    // Get root orientation for relative calculations
    let root_orientation = physics_state.part_rotations.first().copied().unwrap_or(0.0);

    for (i, _part) in morphology.body_parts.iter().enumerate() {
        // Get this body part's data from physics_state
        let position = physics_state
            .part_positions
            .get(i)
            .copied()
            .unwrap_or(Vec2::ZERO);
        let orientation = physics_state.part_rotations.get(i).copied().unwrap_or(0.0);

        // Joint angle (from motor angles if this is a motorized part)
        let joint_angle = physics_state
            .motor_part_indices
            .iter()
            .position(|&idx| idx == i)
            .and_then(|motor_idx| physics_state.motor_angles.get(motor_idx).copied())
            .unwrap_or(0.0);

        // Joint angular velocity
        let joint_angular_velocity = physics_state
            .motor_part_indices
            .iter()
            .position(|&idx| idx == i)
            .and_then(|motor_idx| {
                physics_state
                    .motor_angular_velocities
                    .get(motor_idx)
                    .copied()
            })
            .unwrap_or(0.0);

        // Ground contact: raycast downward from body part
        let ground_contact = check_ground_contact(world, position, &morphology.body_parts[i]);

        // Raycast distances from sensory input
        let raycast_distances: Vec<f32> = sensory_input
            .raycasts
            .iter()
            .map(|hit| hit.distance)
            .collect();

        // Contact materials (one-hot encoding of nearby materials)
        let contact_materials = encode_contact_materials(&sensory_input.contact_materials);

        // Normalize orientation relative to root
        let relative_orientation = orientation - root_orientation;

        // Velocity is not tracked in simple physics - use zero
        let velocity = Vec2::ZERO;

        features.push(BodyPartFeatures {
            joint_angle,
            joint_angular_velocity,
            orientation: relative_orientation,
            velocity,
            ground_contact,
            raycast_distances,
            contact_materials,
            food_direction_x,
            food_direction_y,
            food_distance,
        });
    }

    features
}

/// Check if body part is in contact with ground
fn check_ground_contact(
    world: &impl crate::WorldAccess,
    position: Vec2,
    body_part: &super::morphology::BodyPart,
) -> f32 {
    // Raycast downward from bottom of body part
    let check_pos = position - Vec2::new(0.0, body_part.radius + 1.0);
    let (px, py) = (check_pos.x as i32, check_pos.y as i32);

    // Check a few pixels below the body part
    for dy in 0..3 {
        let pixel = world.get_pixel(px, py - dy);
        if let Some(p) = pixel
            && p.material_id != 0
        {
            // Not air
            // Return 1.0 if touching, fade based on distance
            return 1.0 - (dy as f32 * 0.3);
        }
    }

    0.0 // Not grounded
}

/// Encode contact materials as one-hot vector
fn encode_contact_materials(contact_materials: &[u16]) -> Vec<f32> {
    // Create 5-element vector for most common material types
    // 0=solid, 1=liquid, 2=powder, 3=gas, 4=other
    let mut encoded = vec![0.0; 5];

    for &material_id in contact_materials {
        // Map material to category based on material_id ranges
        // This is a simplified encoding - could be improved with actual material properties
        let category = match material_id {
            0 => continue,                    // Air - skip
            1 | 12 | 13 | 14 | 19 => 0,       // Stone, glass, metal, bedrock, bone = solid
            3 | 8 | 10 => 1,                  // Water, lava, acid = liquid
            2 | 5 | 6 | 7 | 9 | 11 | 17 => 2, // Sand, fire, smoke, steam, oil, ice, fruit = powder
            16 | 18 => 3,                     // Plant matter, flesh = organic
            _ => 4,                           // Other
        };
        encoded[category] = (encoded[category] + 0.2_f32).min(1.0);
    }

    encoded
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_feature_vector_conversion() {
        let features = BodyPartFeatures {
            joint_angle: 0.5,
            joint_angular_velocity: 0.1,
            orientation: 1.57,
            velocity: Vec2::new(1.0, 0.0),
            ground_contact: 1.0,
            raycast_distances: vec![0.5, 0.8],
            contact_materials: vec![1.0, 0.0],
            food_direction_x: 0.7,
            food_direction_y: 0.3,
            food_distance: 0.5,
        };

        let vec = features.to_vec();
        assert_eq!(vec.len(), 13); // 9 base + 2 raycasts + 2 materials

        // Check values are in correct order
        assert_eq!(vec[0], 0.5); // joint_angle
        assert_eq!(vec[3], 1.0); // velocity.x
        assert_eq!(vec[6], 0.7); // food_direction_x
        assert_eq!(vec[7], 0.3); // food_direction_y
        assert_eq!(vec[8], 0.5); // food_distance
        assert_eq!(vec[9], 0.5); // first raycast
    }

    #[test]
    fn test_feature_dim_calculation() {
        let dim = BodyPartFeatures::feature_dim(8, 5);
        assert_eq!(dim, 22); // 9 base + 8 raycasts + 5 materials
    }

    #[test]
    fn test_simple_controller_random() {
        let controller = SimpleNeuralController::random(10, 8, 5);

        assert_eq!(controller.input_dim, 10);
        assert_eq!(controller.hidden_dim, 8);
        assert_eq!(controller.output_dim, 5);

        // Weight count: 10*8 + 8*5 = 80 + 40 = 120
        assert_eq!(controller.weights.len(), 120);
    }

    #[test]
    fn test_simple_controller_forward() {
        let controller = SimpleNeuralController::random(10, 8, 5);
        let input = vec![0.5; 10];

        let output = controller.forward(&input);

        // Should produce 5 outputs
        assert_eq!(output.len(), 5);

        // Outputs should be in tanh range [-1, 1]
        for &val in &output {
            assert!((-1.0..=1.0).contains(&val));
        }
    }

    #[test]
    fn test_simple_controller_from_genome() {
        use crate::genome::ControllerGenome;

        let genome = ControllerGenome::random(16, 3); // hidden_dim=16, message_passing_steps=3
        let controller = SimpleNeuralController::from_genome(&genome, 10, 5);

        assert_eq!(controller.input_dim, 10);
        assert_eq!(controller.output_dim, 5);
        assert_eq!(controller.hidden_dim, 16);
    }

    #[test]
    fn test_morphology_graph_from_morphology() {
        use crate::morphology::CreatureMorphology;

        let morphology = CreatureMorphology::test_biped();
        let graph = MorphologyGraph::from_morphology(&morphology);

        // Biped has 3 body parts
        assert_eq!(graph.num_nodes, 3);

        // Biped has 2 joints (2 edges bidirectional = 4 total)
        assert_eq!(graph.edges.len(), 4);
    }

    // The following tests require World::new() which is in sunaba-core.
    // These tests are moved to sunaba-core as integration tests.
    // See sunaba-core/tests/creature_neural_test.rs

    #[test]
    fn test_neural_controller_deterministic() {
        let controller = SimpleNeuralController::random(5, 4, 3);
        let input = vec![0.1, 0.2, 0.3, 0.4, 0.5];

        let output1 = controller.forward(&input);
        let output2 = controller.forward(&input);

        // Same input should produce same output
        assert_eq!(output1, output2);
    }

    #[test]
    #[should_panic(expected = "Input dimension mismatch")]
    fn test_neural_controller_wrong_input_size() {
        let controller = SimpleNeuralController::random(10, 8, 5);
        let input = vec![0.5; 5]; // Wrong size!

        controller.forward(&input); // Should panic
    }
}
