//! CPPN-NEAT genome representation
//!
//! Implements Compositional Pattern Producing Networks (CPPN) combined with
//! NeuroEvolution of Augmenting Topologies (NEAT) for evolving creature morphologies.

use ahash::HashMap;
use petgraph::graph::{DiGraph, NodeIndex};
use serde::{Deserialize, Serialize};

/// Activation functions for CPPN nodes
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum ActivationFunction {
    Linear,
    Sigmoid,
    Tanh,
    Gaussian,
    Sine,
    Relu,
    Step,
}

impl ActivationFunction {
    /// Apply activation function to input
    pub fn activate(&self, x: f32) -> f32 {
        match self {
            Self::Linear => x,
            Self::Sigmoid => 1.0 / (1.0 + (-x).exp()),
            Self::Tanh => x.tanh(),
            Self::Gaussian => (-x * x).exp(),
            Self::Sine => x.sin(),
            Self::Relu => x.max(0.0),
            Self::Step => {
                if x > 0.0 {
                    1.0
                } else {
                    0.0
                }
            }
        }
    }
}

/// Node in CPPN network
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CppnNode {
    pub id: u64,
    pub activation: ActivationFunction,
    pub node_type: NodeType,
}

/// Type of CPPN node
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum NodeType {
    Input,  // Spatial coordinates (x, y, d)
    Hidden, // Internal computation
    Output, // Morphology properties
}

/// Connection between CPPN nodes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CppnConnection {
    pub weight: f32,
    pub enabled: bool,
    pub innovation_number: u64,
}

/// CPPN network for morphology generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CppnGenome {
    #[serde(skip)]
    pub graph: DiGraph<CppnNode, CppnConnection>,

    pub input_node_ids: Vec<u64>,  // Node IDs for input nodes
    pub output_node_ids: Vec<u64>, // Node IDs for output nodes

    pub innovation_numbers: HashMap<(u64, u64), u64>, // Track connection innovations
    pub next_node_id: u64,
    pub next_innovation: u64,
}

impl CppnGenome {
    /// Get node index from node ID (internal helper)
    pub fn node_index_from_id(&self, node_id: u64) -> Option<NodeIndex> {
        self.graph
            .node_indices()
            .find(|&idx| self.graph[idx].id == node_id)
    }

    /// Create minimal CPPN with input/output nodes only
    pub fn minimal() -> Self {
        let mut graph = DiGraph::new();
        let mut input_node_ids = Vec::new();
        let mut output_node_ids = Vec::new();
        let mut next_node_id = 0;

        // Create 3 input nodes: x, y, d (distance from origin)
        for activation in [
            ActivationFunction::Linear,
            ActivationFunction::Linear,
            ActivationFunction::Linear,
        ] {
            let node = CppnNode {
                id: next_node_id,
                activation,
                node_type: NodeType::Input,
            };
            graph.add_node(node);
            input_node_ids.push(next_node_id);
            next_node_id += 1;
        }

        // Create 4 output nodes: radius, density, has_joint, joint_type
        for activation in [
            ActivationFunction::Sigmoid, // radius (0-1, scaled later)
            ActivationFunction::Sigmoid, // density (0-1)
            ActivationFunction::Sigmoid, // has_joint (0-1, threshold at 0.5)
            ActivationFunction::Tanh,    // joint_type (-1 to 1)
        ] {
            let node = CppnNode {
                id: next_node_id,
                activation,
                node_type: NodeType::Output,
            };
            graph.add_node(node);
            output_node_ids.push(next_node_id);
            next_node_id += 1;
        }

        // Connect all inputs to all outputs with random weights
        let mut next_innovation = 0;
        let mut innovation_numbers = HashMap::default();

        for &input_id in &input_node_ids {
            for &output_id in &output_node_ids {
                let input_idx = graph
                    .node_indices()
                    .find(|&idx| graph[idx].id == input_id)
                    .unwrap();
                let output_idx = graph
                    .node_indices()
                    .find(|&idx| graph[idx].id == output_id)
                    .unwrap();

                let connection = CppnConnection {
                    weight: (rand::random::<f32>() * 2.0 - 1.0) * 0.5, // [-0.5, 0.5]
                    enabled: true,
                    innovation_number: next_innovation,
                };

                graph.add_edge(input_idx, output_idx, connection);
                innovation_numbers.insert((input_id, output_id), next_innovation);
                next_innovation += 1;
            }
        }

        Self {
            graph,
            input_node_ids,
            output_node_ids,
            innovation_numbers,
            next_node_id,
            next_innovation,
        }
    }

    /// Query CPPN at spatial position (x, y, d)
    pub fn query(&self, x: f32, y: f32, d: f32) -> CppnOutput {
        use petgraph::visit::Topo;

        let mut activations: HashMap<NodeIndex, f32> = HashMap::default();

        // Set input values
        let inputs = [x, y, d];
        for (i, &input_id) in self.input_node_ids.iter().enumerate() {
            if let Some(idx) = self.node_index_from_id(input_id) {
                activations.insert(idx, inputs[i]);
            }
        }

        // Topological sort and forward pass
        let mut topo = Topo::new(&self.graph);
        while let Some(node_idx) = topo.next(&self.graph) {
            let node = &self.graph[node_idx];

            // Skip if input (already set)
            if node.node_type == NodeType::Input {
                continue;
            }

            // Sum weighted inputs from all incoming edges
            let mut sum = 0.0;
            // Iterate over all nodes to find connections to this node
            for source_idx in self.graph.node_indices() {
                if let Some(edge_idx) = self.graph.find_edge(source_idx, node_idx) {
                    let connection = &self.graph[edge_idx];

                    if connection.enabled {
                        if let Some(&source_activation) = activations.get(&source_idx) {
                            sum += source_activation * connection.weight;
                        }
                    }
                }
            }

            // Apply activation function
            let output = node.activation.activate(sum);
            activations.insert(node_idx, output);
        }

        // Extract output values
        let mut outputs = [0.0; 4];
        for (i, &output_id) in self.output_node_ids.iter().enumerate() {
            if let Some(idx) = self.node_index_from_id(output_id) {
                outputs[i] = activations.get(&idx).copied().unwrap_or(0.0);
            }
        }

        CppnOutput {
            radius: outputs[0].clamp(0.0, 1.0),      // Clamp to [0, 1]
            density: outputs[1].clamp(0.0, 1.0),     // Clamp to [0, 1]
            has_joint: outputs[2] > 0.5,             // Threshold
            joint_type: outputs[3].clamp(-1.0, 1.0), // Clamp to [-1, 1]
        }
    }

    /// Rebuild graph from serialized data (called after deserialization)
    pub fn rebuild_graph(&mut self) {
        // Graph will be empty after deserialization (marked with #[serde(skip)])
        // For Phase 6, we'll just use the minimal structure
        // Full rebuild from serialized nodes/edges can be added in Phase 7
        *self = Self::minimal();
    }
}

/// Output from CPPN query
#[derive(Debug, Clone)]
pub struct CppnOutput {
    pub radius: f32,
    pub density: f32,
    pub has_joint: bool,
    pub joint_type: f32, // Continuous value mapped to joint type
}

/// Controller genome (GNN weights)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ControllerGenome {
    pub message_weights: Vec<f32>,    // Message passing layer
    pub update_weights: Vec<f32>,     // Node update layer
    pub output_weights: Vec<f32>,     // Motor command projection
    pub message_passing_steps: usize, // Number of GNN rounds
    pub hidden_dim: usize,            // Feature dimension
}

impl ControllerGenome {
    /// Create random controller genome
    pub fn random(hidden_dim: usize, message_passing_steps: usize) -> Self {
        use rand::Rng;
        let mut rng = rand::thread_rng();

        // For a simple feedforward network:
        // message_weights: input_dim -> hidden_dim
        // update_weights: hidden_dim -> hidden_dim
        // output_weights: hidden_dim -> output_dim (motor commands)

        // Estimate sizes (will be adjusted when morphology is known)
        let input_dim_estimate = 10; // Joint angles, velocities, contacts, etc.
        let output_dim_estimate = 5; // Motor commands per joint

        let message_weight_count = input_dim_estimate * hidden_dim;
        let update_weight_count = hidden_dim * hidden_dim;
        let output_weight_count = hidden_dim * output_dim_estimate;

        let message_weights: Vec<f32> = (0..message_weight_count)
            .map(|_| rng.gen_range(-0.5..0.5))
            .collect();

        let update_weights: Vec<f32> = (0..update_weight_count)
            .map(|_| rng.gen_range(-0.5..0.5))
            .collect();

        let output_weights: Vec<f32> = (0..output_weight_count)
            .map(|_| rng.gen_range(-0.5..0.5))
            .collect();

        Self {
            message_weights,
            update_weights,
            output_weights,
            message_passing_steps,
            hidden_dim,
        }
    }
}

/// Behavioral traits
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BehavioralTraits {
    pub aggression: f32,     // 0.0 - 1.0
    pub curiosity: f32,      // 0.0 - 1.0
    pub sociality: f32,      // 0.0 - 1.0
    pub territoriality: f32, // 0.0 - 1.0
}

impl Default for BehavioralTraits {
    fn default() -> Self {
        Self {
            aggression: 0.5,
            curiosity: 0.5,
            sociality: 0.5,
            territoriality: 0.5,
        }
    }
}

/// Metabolic parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetabolicParams {
    pub hunger_rate: f32,                  // Units per second
    pub temperature_tolerance: (f32, f32), // (min, max) in Celsius
    pub oxygen_requirement: f32,           // Units per second
}

impl Default for MetabolicParams {
    fn default() -> Self {
        Self {
            hunger_rate: 0.1,
            temperature_tolerance: (-10.0, 50.0),
            oxygen_requirement: 0.05,
        }
    }
}

/// Complete creature genome
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreatureGenome {
    pub cppn: CppnGenome,
    pub controller: ControllerGenome,
    pub traits: BehavioralTraits,
    pub metabolic: MetabolicParams,
    pub generation: u64,
}

impl CreatureGenome {
    /// Create test biped genome (for validation)
    /// Simple two-legged creature with central body
    pub fn test_biped() -> Self {
        let cppn = CppnGenome::minimal();
        let controller = ControllerGenome::random(16, 2);
        let traits = BehavioralTraits {
            aggression: 0.3,
            curiosity: 0.7,
            sociality: 0.4,
            territoriality: 0.2,
        };
        let metabolic = MetabolicParams {
            hunger_rate: 0.15,
            temperature_tolerance: (0.0, 40.0),
            oxygen_requirement: 0.05,
        };

        Self {
            cppn,
            controller,
            traits,
            metabolic,
            generation: 0,
        }
    }

    /// Create test quadruped genome (for validation)
    /// Four-legged creature
    pub fn test_quadruped() -> Self {
        let cppn = CppnGenome::minimal();
        let controller = ControllerGenome::random(24, 3);
        let traits = BehavioralTraits {
            aggression: 0.6,
            curiosity: 0.5,
            sociality: 0.6,
            territoriality: 0.7,
        };
        let metabolic = MetabolicParams {
            hunger_rate: 0.2,
            temperature_tolerance: (-5.0, 45.0),
            oxygen_requirement: 0.08,
        };

        Self {
            cppn,
            controller,
            traits,
            metabolic,
            generation: 0,
        }
    }

    /// Create test worm genome (for validation)
    /// Segmented creature with many body parts
    pub fn test_worm() -> Self {
        let cppn = CppnGenome::minimal();
        let controller = ControllerGenome::random(8, 1);
        let traits = BehavioralTraits {
            aggression: 0.1,
            curiosity: 0.3,
            sociality: 0.2,
            territoriality: 0.1,
        };
        let metabolic = MetabolicParams {
            hunger_rate: 0.05,
            temperature_tolerance: (5.0, 35.0),
            oxygen_requirement: 0.02,
        };

        Self {
            cppn,
            controller,
            traits,
            metabolic,
            generation: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_activation_functions() {
        assert_eq!(ActivationFunction::Linear.activate(2.0), 2.0);
        assert!((ActivationFunction::Sigmoid.activate(0.0) - 0.5).abs() < 0.001);
        assert!(ActivationFunction::Tanh.activate(0.0).abs() < 0.001);
        assert!((ActivationFunction::Gaussian.activate(0.0) - 1.0).abs() < 0.001);
        assert!(ActivationFunction::Relu.activate(-1.0) == 0.0);
        assert!(ActivationFunction::Relu.activate(1.0) == 1.0);
    }

    #[test]
    fn test_cppn_minimal_creation() {
        let cppn = CppnGenome::minimal();

        // Should have 3 input nodes and 4 output nodes
        assert_eq!(cppn.input_node_ids.len(), 3);
        assert_eq!(cppn.output_node_ids.len(), 4);

        // Should have 7 total nodes (3 inputs + 4 outputs)
        assert_eq!(cppn.graph.node_count(), 7);

        // Should have 12 connections (3 inputs × 4 outputs)
        assert_eq!(cppn.graph.edge_count(), 12);
    }

    #[test]
    fn test_cppn_query() {
        let cppn = CppnGenome::minimal();

        // Query at origin
        let output1 = cppn.query(0.0, 0.0, 0.0);
        assert!(output1.radius >= 0.0 && output1.radius <= 1.0);
        assert!(output1.density >= 0.0 && output1.density <= 1.0);
        assert!(output1.joint_type >= -1.0 && output1.joint_type <= 1.0);

        // Query at different position
        let output2 = cppn.query(1.0, 1.0, 1.414);
        assert!(output2.radius >= 0.0 && output2.radius <= 1.0);

        // Different inputs should give different outputs (usually)
        // This might occasionally fail due to random weights, but very unlikely
        assert!(
            (output1.radius - output2.radius).abs() > 0.001
                || (output1.density - output2.density).abs() > 0.001
        );
    }

    #[test]
    fn test_controller_genome_random() {
        let controller = ControllerGenome::random(16, 2);

        assert_eq!(controller.hidden_dim, 16);
        assert_eq!(controller.message_passing_steps, 2);
        assert!(!controller.message_weights.is_empty());
        assert!(!controller.update_weights.is_empty());
        assert!(!controller.output_weights.is_empty());

        // Check weights are in reasonable range
        for &weight in &controller.message_weights {
            assert!((-0.5..=0.5).contains(&weight));
        }
    }

    #[test]
    fn test_creature_genome_biped() {
        let genome = CreatureGenome::test_biped();

        assert_eq!(genome.generation, 0);
        assert_eq!(genome.controller.hidden_dim, 16);
        assert_eq!(genome.traits.aggression, 0.3);
        assert_eq!(genome.metabolic.hunger_rate, 0.15);
    }

    #[test]
    fn test_creature_genome_quadruped() {
        let genome = CreatureGenome::test_quadruped();

        assert_eq!(genome.generation, 0);
        assert_eq!(genome.controller.hidden_dim, 24);
        assert_eq!(genome.traits.aggression, 0.6);
    }

    #[test]
    fn test_creature_genome_worm() {
        let genome = CreatureGenome::test_worm();

        assert_eq!(genome.generation, 0);
        assert_eq!(genome.controller.hidden_dim, 8);
        assert_eq!(genome.traits.aggression, 0.1);
    }

    #[test]
    fn test_genome_serialization() {
        let genome = CreatureGenome::test_biped();

        // Serialize
        let serialized = bincode::serialize(&genome).expect("Failed to serialize genome");

        // Deserialize
        let mut deserialized: CreatureGenome =
            bincode::deserialize(&serialized).expect("Failed to deserialize genome");

        // Graph needs to be rebuilt after deserialization
        deserialized.cppn.rebuild_graph();

        // Check values match
        assert_eq!(deserialized.generation, genome.generation);
        assert_eq!(deserialized.traits.aggression, genome.traits.aggression);
        assert_eq!(
            deserialized.metabolic.hunger_rate,
            genome.metabolic.hunger_rate
        );

        // Check CPPN still works after rebuild
        let output = deserialized.cppn.query(0.5, 0.5, 0.707);
        assert!(output.radius >= 0.0 && output.radius <= 1.0);
    }

    #[test]
    fn test_behavioral_traits_defaults() {
        let traits = BehavioralTraits::default();
        assert_eq!(traits.aggression, 0.5);
        assert_eq!(traits.curiosity, 0.5);
        assert_eq!(traits.sociality, 0.5);
        assert_eq!(traits.territoriality, 0.5);
    }

    #[test]
    fn test_metabolic_params_defaults() {
        let metabolic = MetabolicParams::default();
        assert_eq!(metabolic.hunger_rate, 0.1);
        assert_eq!(metabolic.temperature_tolerance, (-10.0, 50.0));
        assert_eq!(metabolic.oxygen_requirement, 0.05);
    }
}
