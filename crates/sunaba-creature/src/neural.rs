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
    // Terrain-aware sensors for adaptive locomotion
    pub ground_slope: f32,             // -1.0 (downhill) to 1.0 (uphill)
    pub vertical_clearance: f32,       // 0.0 (blocked) to 1.0 (clear)
    pub gap_distance: f32,             // 0.0 (immediate) to 1.0 (far/none)
    pub gap_width: f32,                // 0.0 (no gap) to 1.0 (unjumpable)
    pub surface_material_encoded: f32, // 0.0-1.0 encoded material ID
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
        // Terrain sensors
        features.extend(&[
            self.ground_slope,
            self.vertical_clearance,
            self.gap_distance,
            self.gap_width,
            self.surface_material_encoded,
        ]);
        features
    }

    /// Get feature dimension
    pub fn feature_dim(num_raycasts: usize, num_materials: usize) -> usize {
        9 + num_raycasts + num_materials + 5 // 6 base + 3 food + raycasts + materials + 5 terrain
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

/// Graph Neural Network controller for creatures
///
/// Implements NerveNet-style message passing that adapts to variable morphologies.
/// Each body part is a node in the graph, and joints define edges.
/// Information flows through the morphology structure via message passing rounds.
///
/// Architecture:
/// 1. Input projection: Per-node features → hidden representation
/// 2. Message passing (K rounds):
///    - Each node sends messages to neighbors
///    - Messages are aggregated (sum)
///    - Node states are updated based on aggregated messages
/// 3. Output projection: Hidden states → motor commands
pub struct GraphNeuralController {
    /// Input projection weights: input_dim → hidden_dim
    input_weights: Vec<f32>,
    /// Message projection weights: hidden_dim → hidden_dim
    message_weights: Vec<f32>,
    /// Update weights: 2*hidden_dim → hidden_dim (combines self + aggregated messages)
    update_weights: Vec<f32>,
    /// Output projection weights: hidden_dim → 1 (one motor command per node)
    output_weights: Vec<f32>,
    /// Hidden dimension for node representations
    hidden_dim: usize,
    /// Number of message passing rounds
    message_passing_steps: usize,
    /// Input feature dimension per node
    input_dim: usize,
    /// Previous hidden states for temporal continuity
    prev_hidden: Option<Vec<Vec<f32>>>,
    /// Recurrence blend factor
    recurrence_factor: f32,
}

impl GraphNeuralController {
    /// Create from genome and morphology structure
    pub fn from_genome(
        genome: &ControllerGenome,
        _morphology: &CreatureMorphology,
        input_dim_per_node: usize,
    ) -> Self {
        use crate::deterministic_rng::DeterministicRng;

        let hidden_dim = genome.hidden_dim;
        let message_passing_steps = genome.message_passing_steps;

        // Create seeded RNG from genome weights for deterministic initialization
        let seed: u64 = genome
            .message_weights
            .iter()
            .chain(genome.update_weights.iter())
            .chain(genome.output_weights.iter())
            .fold(0u64, |acc, &w| acc.wrapping_add((w * 1000.0) as i64 as u64));
        let mut rng = DeterministicRng::from_seed(seed);

        // Input projection: input_dim → hidden_dim
        let input_weight_count = input_dim_per_node * hidden_dim;
        let input_scale = (2.0 / (input_dim_per_node + hidden_dim) as f32).sqrt();
        let input_weights: Vec<f32> = (0..input_weight_count)
            .map(|_| rng.gen_range_f32(-1.0, 1.0) * input_scale)
            .collect();

        // Message projection: hidden_dim → hidden_dim
        let message_weight_count = hidden_dim * hidden_dim;
        let message_scale = (2.0 / (hidden_dim * 2) as f32).sqrt();
        let message_weights: Vec<f32> = (0..message_weight_count)
            .map(|_| rng.gen_range_f32(-1.0, 1.0) * message_scale)
            .collect();

        // Update weights: 2*hidden_dim → hidden_dim (self + aggregated messages)
        let update_weight_count = (hidden_dim * 2) * hidden_dim;
        let update_scale = (2.0 / (hidden_dim * 3) as f32).sqrt();
        let update_weights: Vec<f32> = (0..update_weight_count)
            .map(|_| rng.gen_range_f32(-1.0, 1.0) * update_scale)
            .collect();

        // Output projection: hidden_dim → 1 (one motor per node, but we may have fewer motors)
        let output_weight_count = hidden_dim * 1; // 1 output per node
        let output_scale = (2.0 / (hidden_dim + 1) as f32).sqrt();
        let output_weights: Vec<f32> = (0..output_weight_count)
            .map(|_| rng.gen_range_f32(-1.0, 1.0) * output_scale)
            .collect();

        Self {
            input_weights,
            message_weights,
            update_weights,
            output_weights,
            hidden_dim,
            message_passing_steps,
            input_dim: input_dim_per_node,
            prev_hidden: None,
            recurrence_factor: 0.2, // Blend 20% of previous state for temporal continuity
        }
    }

    /// Reset hidden state (call between episodes)
    pub fn reset_hidden(&mut self) {
        self.prev_hidden = None;
    }

    /// Forward pass through the GNN
    ///
    /// # Arguments
    /// * `node_features` - Feature vector for each node (body part)
    /// * `graph` - Morphology graph defining connectivity
    ///
    /// # Returns
    /// Motor commands for each motor (one per node that has a motor)
    pub fn forward(
        &mut self,
        node_features: &[Vec<f32>],
        graph: &MorphologyGraph,
    ) -> Vec<f32> {
        #[cfg(feature = "profiling")]
        puffin::profile_function!();

        let num_nodes = graph.num_nodes;
        if num_nodes == 0 {
            return Vec::new();
        }

        // 1. Project input features to hidden dimension
        let mut hidden_states: Vec<Vec<f32>> = node_features
            .iter()
            .map(|features| self.project_input(features))
            .collect();

        // Apply recurrence from previous step
        if let Some(ref prev) = self.prev_hidden {
            for (i, h) in hidden_states.iter_mut().enumerate() {
                if let Some(prev_h) = prev.get(i) {
                    for (j, val) in h.iter_mut().enumerate() {
                        if let Some(&prev_val) = prev_h.get(j) {
                            *val = (1.0 - self.recurrence_factor) * *val
                                + self.recurrence_factor * prev_val;
                        }
                    }
                }
            }
        }

        // 2. Message passing rounds
        for _ in 0..self.message_passing_steps {
            hidden_states = self.message_passing_step(&hidden_states, graph);
        }

        // Store hidden states for next timestep
        self.prev_hidden = Some(hidden_states.clone());

        // 3. Project to output (one value per node)
        let outputs: Vec<f32> = hidden_states
            .iter()
            .map(|h| self.project_output(h))
            .collect();

        outputs
    }

    /// Project input features to hidden dimension
    fn project_input(&self, features: &[f32]) -> Vec<f32> {
        let mut hidden = vec![0.0; self.hidden_dim];

        // Pad or truncate features to match expected input_dim
        let features_len = features.len().min(self.input_dim);

        for h in 0..self.hidden_dim {
            let mut sum = 0.0;
            for i in 0..features_len {
                let w_idx = i * self.hidden_dim + h;
                if w_idx < self.input_weights.len() {
                    sum += features[i] * self.input_weights[w_idx];
                }
            }
            hidden[h] = sum.tanh();
        }

        hidden
    }

    /// Single message passing step
    fn message_passing_step(
        &self,
        hidden_states: &[Vec<f32>],
        graph: &MorphologyGraph,
    ) -> Vec<Vec<f32>> {
        let num_nodes = hidden_states.len();
        let mut new_states = vec![vec![0.0; self.hidden_dim]; num_nodes];

        // For each node, aggregate messages from neighbors
        for node_idx in 0..num_nodes {
            // Find all neighbors (nodes connected by edges)
            let neighbors: Vec<usize> = graph
                .edges
                .iter()
                .filter_map(|&(from, to)| {
                    if to == node_idx {
                        Some(from)
                    } else {
                        None
                    }
                })
                .collect();

            // Aggregate messages from neighbors
            let mut aggregated_message = vec![0.0; self.hidden_dim];
            for &neighbor_idx in &neighbors {
                let message = self.compute_message(&hidden_states[neighbor_idx]);
                for (i, m) in message.iter().enumerate() {
                    aggregated_message[i] += m;
                }
            }

            // Normalize by number of neighbors (mean aggregation)
            if !neighbors.is_empty() {
                let scale = 1.0 / neighbors.len() as f32;
                for m in &mut aggregated_message {
                    *m *= scale;
                }
            }

            // Update node state: combine self state with aggregated messages
            new_states[node_idx] = self.update_node(
                &hidden_states[node_idx],
                &aggregated_message,
            );
        }

        new_states
    }

    /// Compute message from a neighbor's hidden state
    fn compute_message(&self, hidden: &[f32]) -> Vec<f32> {
        let mut message = vec![0.0; self.hidden_dim];

        for h in 0..self.hidden_dim {
            let mut sum = 0.0;
            for i in 0..self.hidden_dim.min(hidden.len()) {
                let w_idx = i * self.hidden_dim + h;
                if w_idx < self.message_weights.len() {
                    sum += hidden[i] * self.message_weights[w_idx];
                }
            }
            message[h] = sum;
        }

        message
    }

    /// Update node state based on self and aggregated messages
    fn update_node(&self, self_hidden: &[f32], aggregated: &[f32]) -> Vec<f32> {
        let mut new_hidden = vec![0.0; self.hidden_dim];

        // Concatenate self_hidden and aggregated for input to update
        for h in 0..self.hidden_dim {
            let mut sum = 0.0;

            // Process self_hidden
            for i in 0..self.hidden_dim.min(self_hidden.len()) {
                let w_idx = i * self.hidden_dim + h;
                if w_idx < self.update_weights.len() {
                    sum += self_hidden[i] * self.update_weights[w_idx];
                }
            }

            // Process aggregated
            for i in 0..self.hidden_dim.min(aggregated.len()) {
                let w_idx = (self.hidden_dim + i) * self.hidden_dim + h;
                if w_idx < self.update_weights.len() {
                    sum += aggregated[i] * self.update_weights[w_idx];
                }
            }

            new_hidden[h] = sum.tanh();
        }

        new_hidden
    }

    /// Project hidden state to single output value
    fn project_output(&self, hidden: &[f32]) -> f32 {
        let mut sum = 0.0;
        for (i, &h) in hidden.iter().enumerate() {
            if i < self.output_weights.len() {
                sum += h * self.output_weights[i];
            }
        }
        sum.tanh()
    }

    /// Get the hidden dimension
    pub fn hidden_dim(&self) -> usize {
        self.hidden_dim
    }

    /// Get input dimension per node
    pub fn input_dim(&self) -> usize {
        self.input_dim
    }
}

/// Hybrid controller that uses GNN for morphology-aware processing
/// and falls back to feedforward for simpler creatures
pub struct HybridNeuralController {
    /// GNN for graph-based processing
    gnn: GraphNeuralController,
    /// Morphology graph structure
    graph: MorphologyGraph,
    /// Motor node mapping: which node indices have motors
    motor_node_indices: Vec<usize>,
    /// Output dimension (number of motors + action outputs)
    output_dim: usize,
}

impl HybridNeuralController {
    /// Create hybrid controller from genome and morphology
    pub fn from_genome(
        genome: &ControllerGenome,
        morphology: &CreatureMorphology,
        input_dim_per_node: usize,
    ) -> Self {
        let graph = MorphologyGraph::from_morphology(morphology);

        // Find which nodes have motors attached
        let motor_node_indices: Vec<usize> = morphology
            .joints
            .iter()
            .filter_map(|joint| {
                if matches!(joint.joint_type, crate::morphology::JointType::Revolute { .. }) {
                    Some(joint.child_index)
                } else {
                    None
                }
            })
            .collect();

        let gnn = GraphNeuralController::from_genome(genome, morphology, input_dim_per_node);
        let output_dim = motor_node_indices.len() + 1; // Motors + 1 action output

        Self {
            gnn,
            graph,
            motor_node_indices,
            output_dim,
        }
    }

    /// Reset hidden state
    pub fn reset_hidden(&mut self) {
        self.gnn.reset_hidden();
    }

    /// Forward pass
    ///
    /// # Arguments
    /// * `node_features` - Feature vectors per body part
    ///
    /// # Returns
    /// Motor commands + action output
    pub fn forward(&mut self, node_features: &[Vec<f32>]) -> Vec<f32> {
        // Run GNN forward pass
        let node_outputs = self.gnn.forward(node_features, &self.graph);

        // Map node outputs to motor commands
        let mut outputs = Vec::with_capacity(self.output_dim);

        // Get motor commands from motor nodes
        for &node_idx in &self.motor_node_indices {
            if let Some(&output) = node_outputs.get(node_idx) {
                outputs.push(output);
            } else {
                outputs.push(0.0);
            }
        }

        // Add action output (average of all node outputs)
        let action_output = if !node_outputs.is_empty() {
            node_outputs.iter().sum::<f32>() / node_outputs.len() as f32
        } else {
            0.0
        };
        outputs.push(action_output);

        outputs
    }

    /// Get output dimension
    pub fn output_dim(&self) -> usize {
        self.output_dim
    }
}

/// Extract features from simple physics state
/// Uses CreaturePhysicsState for position-based physics without rapier2d
pub fn extract_body_part_features_simple(
    morphology: &CreatureMorphology,
    physics_state: &super::simple_physics::CreaturePhysicsState,
    sensory_input: &super::sensors::SensoryInput,
    world: &impl crate::WorldAccess,
    sensor_config: &super::sensors::SensorConfig,
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

        // Get velocity from new physics state (parts array)
        let velocity = physics_state
            .parts
            .get(i)
            .map(|p| p.velocity)
            .unwrap_or(Vec2::ZERO);

        // Calculate facing direction from orientation (or default to right)
        let facing_direction = orientation.cos().signum();

        // Compute terrain sensors for adaptive locomotion
        let ground_slope =
            super::sensors::sense_ground_slope(world, position, facing_direction, sensor_config);

        let vertical_clearance =
            super::sensors::sense_vertical_clearance(world, position, sensor_config);

        let (gap_distance, gap_width) =
            super::sensors::sense_gap_info(world, position, facing_direction, sensor_config);

        let surface_material = super::sensors::sense_surface_material(
            world,
            position,
            morphology.body_parts[i].radius,
        );

        // Normalize surface material to [0, 1] (simple approach: ID / 50.0, clamped)
        let surface_material_encoded = (surface_material as f32 / 50.0).min(1.0);

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
            ground_slope,
            vertical_clearance,
            gap_distance,
            gap_width,
            surface_material_encoded,
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
            ground_slope: 0.2,
            vertical_clearance: 0.8,
            gap_distance: 0.9,
            gap_width: 0.1,
            surface_material_encoded: 0.3,
        };

        let vec = features.to_vec();
        assert_eq!(vec.len(), 18); // 9 base + 2 raycasts + 2 materials + 5 terrain

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
        assert_eq!(dim, 27); // 9 base + 8 raycasts + 5 materials + 5 terrain
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

    // ===== Graph Neural Network Tests =====

    #[test]
    fn test_gnn_controller_creation() {
        use crate::genome::ControllerGenome;
        use crate::morphology::CreatureMorphology;

        let genome = ControllerGenome::minimal(16, 2);
        let morphology = CreatureMorphology::test_biped();
        let input_dim = 10;

        let controller = GraphNeuralController::from_genome(&genome, &morphology, input_dim);

        assert_eq!(controller.hidden_dim, 16);
        assert_eq!(controller.message_passing_steps, 2);
        assert_eq!(controller.input_dim, input_dim);
    }

    #[test]
    fn test_gnn_controller_forward() {
        use crate::genome::ControllerGenome;
        use crate::morphology::CreatureMorphology;

        let genome = ControllerGenome::minimal(16, 2);
        let morphology = CreatureMorphology::test_biped();
        let graph = MorphologyGraph::from_morphology(&morphology);
        let input_dim = 10;

        let mut controller = GraphNeuralController::from_genome(&genome, &morphology, input_dim);

        // Create node features (3 nodes for biped)
        let node_features: Vec<Vec<f32>> = (0..3)
            .map(|_| vec![0.5; input_dim])
            .collect();

        let outputs = controller.forward(&node_features, &graph);

        // Should produce one output per node
        assert_eq!(outputs.len(), 3);

        // Outputs should be in tanh range
        for &val in &outputs {
            assert!((-1.0..=1.0).contains(&val), "Output {} out of range", val);
        }
    }

    #[test]
    fn test_gnn_message_passing() {
        use crate::genome::ControllerGenome;
        use crate::morphology::CreatureMorphology;

        let genome = ControllerGenome::minimal(8, 3); // 3 message passing steps
        let morphology = CreatureMorphology::test_biped();
        let graph = MorphologyGraph::from_morphology(&morphology);

        let mut controller = GraphNeuralController::from_genome(&genome, &morphology, 5);

        // Create different node features
        let node_features = vec![
            vec![1.0, 0.0, 0.0, 0.0, 0.0], // Node 0
            vec![0.0, 1.0, 0.0, 0.0, 0.0], // Node 1
            vec![0.0, 0.0, 1.0, 0.0, 0.0], // Node 2
        ];

        let outputs1 = controller.forward(&node_features, &graph);

        // With message passing, information should propagate
        // Running again with same input should give consistent output
        controller.reset_hidden();
        let outputs2 = controller.forward(&node_features, &graph);

        // After reset, outputs should be the same (deterministic)
        for (o1, o2) in outputs1.iter().zip(outputs2.iter()) {
            assert!((o1 - o2).abs() < 0.01, "Expected deterministic output");
        }
    }

    #[test]
    fn test_gnn_temporal_continuity() {
        use crate::genome::ControllerGenome;
        use crate::morphology::CreatureMorphology;

        let genome = ControllerGenome::minimal(8, 2);
        let morphology = CreatureMorphology::test_biped();
        let graph = MorphologyGraph::from_morphology(&morphology);

        let mut controller = GraphNeuralController::from_genome(&genome, &morphology, 5);

        let node_features: Vec<Vec<f32>> = (0..3)
            .map(|_| vec![0.5; 5])
            .collect();

        // First forward pass
        let outputs1 = controller.forward(&node_features, &graph);

        // Second forward pass (without reset) - should blend with previous hidden
        let outputs2 = controller.forward(&node_features, &graph);

        // Outputs should be slightly different due to recurrence
        // With recurrence, they should differ (though might be similar if weights are small)
        // This is a weak test - mainly checking it doesn't crash
        assert_eq!(outputs1.len(), outputs2.len());
    }

    #[test]
    fn test_hybrid_controller() {
        use crate::genome::ControllerGenome;
        use crate::morphology::CreatureMorphology;

        let genome = ControllerGenome::minimal(16, 2);
        let morphology = CreatureMorphology::test_biped();
        let input_dim = 10;

        let mut controller = HybridNeuralController::from_genome(&genome, &morphology, input_dim);

        // Create node features
        let node_features: Vec<Vec<f32>> = (0..3)
            .map(|_| vec![0.5; input_dim])
            .collect();

        let outputs = controller.forward(&node_features);

        // Should have motor outputs + 1 action output
        // Biped has 2 motors, so 2 + 1 = 3 outputs
        assert_eq!(outputs.len(), 3);

        // All outputs should be in valid range
        for &val in &outputs {
            assert!((-1.0..=1.0).contains(&val));
        }
    }

    #[test]
    fn test_gnn_different_morphologies() {
        use crate::genome::ControllerGenome;
        use crate::morphology::CreatureMorphology;

        // Test with different morphology sizes
        let morphologies = vec![
            CreatureMorphology::test_biped(),     // 3 parts
            CreatureMorphology::test_quadruped(), // 5 parts
            CreatureMorphology::archetype_spider(), // 9 parts
            CreatureMorphology::archetype_snake(),  // 6 parts
        ];

        for morphology in morphologies {
            let genome = ControllerGenome::minimal(8, 2);
            let graph = MorphologyGraph::from_morphology(&morphology);
            let num_parts = morphology.body_parts.len();

            let mut controller = GraphNeuralController::from_genome(&genome, &morphology, 5);

            let node_features: Vec<Vec<f32>> = (0..num_parts)
                .map(|_| vec![0.5; 5])
                .collect();

            let outputs = controller.forward(&node_features, &graph);

            // Should produce output for each node
            assert_eq!(outputs.len(), num_parts);
        }
    }
}
