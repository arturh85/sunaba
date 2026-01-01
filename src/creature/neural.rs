//! Graph Neural Network controller for creatures
//!
//! Implements NerveNet-style GNN that adapts to variable morphologies.

use glam::Vec2;

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
        ];
        features.extend(&self.raycast_distances);
        features.extend(&self.contact_materials);
        features
    }

    /// Get feature dimension
    pub fn feature_dim(num_raycasts: usize, num_materials: usize) -> usize {
        6 + num_raycasts + num_materials
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

    /// Forward pass: features -> motor commands
    /// Simple 2-layer network: input -> hidden (tanh) -> output (tanh)
    pub fn forward(&self, input: &[f32]) -> Vec<f32> {
        assert_eq!(input.len(), self.input_dim, "Input dimension mismatch");

        // Layer 1: input -> hidden
        let input_to_hidden_weights = &self.weights[0..self.input_dim * self.hidden_dim];
        let mut hidden = vec![0.0; self.hidden_dim];

        #[allow(clippy::needless_range_loop)]
        for h in 0..self.hidden_dim {
            let mut sum = 0.0;
            for i in 0..self.input_dim {
                let weight_idx = h * self.input_dim + i;
                sum += input[i] * input_to_hidden_weights[weight_idx];
            }
            hidden[h] = sum.tanh(); // Activation
        }

        // Layer 2: hidden -> output
        let hidden_to_output_start = self.input_dim * self.hidden_dim;
        let hidden_to_output_weights = &self.weights[hidden_to_output_start..];
        let mut output = vec![0.0; self.output_dim];

        #[allow(clippy::needless_range_loop)]
        for o in 0..self.output_dim {
            let mut sum = 0.0;
            for h in 0..self.hidden_dim {
                let weight_idx = o * self.hidden_dim + h;
                sum += hidden[h] * hidden_to_output_weights[weight_idx];
            }
            output[o] = sum.tanh(); // Activation
        }

        output
    }
}

/// Extract features from physics state
/// Simplified for Phase 6 - returns placeholder features
pub fn extract_body_part_features(
    morphology: &CreatureMorphology,
    _physics_world: &crate::physics::PhysicsWorld,
    sensory_input: &super::sensors::SensoryInput,
) -> Vec<BodyPartFeatures> {
    let num_parts = morphology.body_parts.len();
    let mut features = Vec::with_capacity(num_parts);

    // For Phase 6: Create placeholder features for each body part
    // Later this will extract real physics data (joint angles, velocities, etc.)
    for _i in 0..num_parts {
        let raycast_distances: Vec<f32> = sensory_input
            .raycasts
            .iter()
            .map(|hit| hit.distance)
            .collect();

        // Simplified contact materials (just count unique materials)
        let contact_materials = vec![0.0; 5]; // Placeholder for top 5 material types

        features.push(BodyPartFeatures {
            joint_angle: 0.0,            // TODO: Extract from physics
            joint_angular_velocity: 0.0, // TODO: Extract from physics
            orientation: 0.0,            // TODO: Extract from physics
            velocity: Vec2::ZERO,        // TODO: Extract from physics
            ground_contact: 0.0,         // TODO: Raycast downward
            raycast_distances,
            contact_materials,
        });
    }

    features
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
        };

        let vec = features.to_vec();
        assert_eq!(vec.len(), 10); // 6 base + 2 raycasts + 2 materials

        // Check values are in correct order
        assert_eq!(vec[0], 0.5); // joint_angle
        assert_eq!(vec[3], 1.0); // velocity.x
        assert_eq!(vec[6], 0.5); // first raycast
    }

    #[test]
    fn test_feature_dim_calculation() {
        let dim = BodyPartFeatures::feature_dim(8, 5);
        assert_eq!(dim, 19); // 6 base + 8 raycasts + 5 materials
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
        use crate::creature::genome::ControllerGenome;

        let genome = ControllerGenome::random(16, 3); // hidden_dim=16, message_passing_steps=3
        let controller = SimpleNeuralController::from_genome(&genome, 10, 5);

        assert_eq!(controller.input_dim, 10);
        assert_eq!(controller.output_dim, 5);
        assert_eq!(controller.hidden_dim, 16);
    }

    #[test]
    fn test_morphology_graph_from_morphology() {
        use crate::creature::morphology::CreatureMorphology;

        let morphology = CreatureMorphology::test_biped();
        let graph = MorphologyGraph::from_morphology(&morphology);

        // Biped has 3 body parts
        assert_eq!(graph.num_nodes, 3);

        // Biped has 2 joints (2 edges bidirectional = 4 total)
        assert_eq!(graph.edges.len(), 4);
    }

    #[test]
    fn test_extract_body_part_features() {
        use crate::creature::morphology::CreatureMorphology;
        use crate::creature::sensors::{SensorConfig, SensoryInput};
        use crate::physics::PhysicsWorld;
        use crate::world::World;

        let morphology = CreatureMorphology::test_biped();
        let physics_world = PhysicsWorld::new();
        let world = World::new();
        let config = SensorConfig::default();

        let sensory_input = SensoryInput::gather(&world, Vec2::new(100.0, 100.0), &config);

        let features = extract_body_part_features(&morphology, &physics_world, &sensory_input);

        // Should have features for each body part
        assert_eq!(features.len(), morphology.body_parts.len());

        // Each feature should have raycast data
        for feature in &features {
            assert_eq!(feature.raycast_distances.len(), config.num_raycasts);
        }
    }

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
