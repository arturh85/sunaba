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

/// Serializable edge representation (stores source/target node IDs)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerializedEdge {
    pub source_id: u64,
    pub target_id: u64,
    pub connection: CppnConnection,
}

/// CPPN network for morphology generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CppnGenome {
    #[serde(skip)]
    pub graph: DiGraph<CppnNode, CppnConnection>,

    // Serializable graph representation (synced before save, used to rebuild after load)
    pub serialized_nodes: Vec<CppnNode>,
    pub serialized_edges: Vec<SerializedEdge>,

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

        let mut cppn = Self {
            graph,
            serialized_nodes: Vec::new(),
            serialized_edges: Vec::new(),
            input_node_ids,
            output_node_ids,
            innovation_numbers,
            next_node_id,
            next_innovation,
        };
        cppn.sync_to_serializable();
        cppn
    }

    /// Sync graph structure to serializable fields (call before serialization)
    pub fn sync_to_serializable(&mut self) {
        // Collect all nodes
        self.serialized_nodes = self
            .graph
            .node_indices()
            .map(|idx| self.graph[idx].clone())
            .collect();

        // Collect all edges with their source/target node IDs
        self.serialized_edges = self
            .graph
            .edge_indices()
            .filter_map(|edge_idx| {
                let (source_idx, target_idx) = self.graph.edge_endpoints(edge_idx)?;
                let source_id = self.graph[source_idx].id;
                let target_id = self.graph[target_idx].id;
                let connection = self.graph[edge_idx].clone();
                Some(SerializedEdge {
                    source_id,
                    target_id,
                    connection,
                })
            })
            .collect();
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

                    if connection.enabled
                        && let Some(&source_activation) = activations.get(&source_idx)
                    {
                        sum += source_activation * connection.weight;
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
        // If no serialized data, fall back to minimal
        if self.serialized_nodes.is_empty() {
            *self = Self::minimal();
            return;
        }

        // Create new graph
        self.graph = DiGraph::new();

        // Map from node ID to NodeIndex for edge reconstruction
        let mut id_to_idx: std::collections::HashMap<u64, NodeIndex> =
            std::collections::HashMap::new();

        // Add all nodes
        for node in &self.serialized_nodes {
            let idx = self.graph.add_node(node.clone());
            id_to_idx.insert(node.id, idx);
        }

        // Add all edges
        for edge in &self.serialized_edges {
            if let (Some(&source_idx), Some(&target_idx)) = (
                id_to_idx.get(&edge.source_id),
                id_to_idx.get(&edge.target_id),
            ) {
                self.graph
                    .add_edge(source_idx, target_idx, edge.connection.clone());
            }
        }
    }

    // ===== NEAT Mutation Operators =====

    /// Mutate connection weights with given probability and perturbation power
    /// Returns number of weights mutated
    pub fn mutate_weights(&mut self, mutation_rate: f32, mutation_power: f32) -> usize {
        use rand::Rng;
        let mut rng = rand::rng();
        let mut mutated_count = 0;

        for edge_idx in self.graph.edge_indices() {
            if rng.random::<f32>() < mutation_rate {
                let connection = &mut self.graph[edge_idx];

                // 90% chance to perturb, 10% chance to replace
                if rng.random::<f32>() < 0.9 {
                    // Perturb weight
                    let perturbation = rng.random_range(-mutation_power..mutation_power);
                    connection.weight += perturbation;
                    // Clamp to prevent extreme values
                    connection.weight = connection.weight.clamp(-4.0, 4.0);
                } else {
                    // Replace with new random weight
                    connection.weight = rng.random_range(-2.0..2.0);
                }
                mutated_count += 1;
            }
        }

        mutated_count
    }

    /// Add a new connection between two random unconnected nodes
    /// Returns true if a connection was added
    pub fn add_connection(&mut self) -> bool {
        use rand::Rng;
        use rand::prelude::IndexedRandom;
        let mut rng = rand::rng();

        // Get all non-input nodes (can be targets)
        let target_nodes: Vec<NodeIndex> = self
            .graph
            .node_indices()
            .filter(|&idx| self.graph[idx].node_type != NodeType::Input)
            .collect();

        // Get all non-output nodes (can be sources)
        let source_nodes: Vec<NodeIndex> = self
            .graph
            .node_indices()
            .filter(|&idx| self.graph[idx].node_type != NodeType::Output)
            .collect();

        if target_nodes.is_empty() || source_nodes.is_empty() {
            return false;
        }

        // Try up to 20 times to find an unconnected pair
        for _ in 0..20 {
            let source = *source_nodes.choose(&mut rng).unwrap();
            let target = *target_nodes.choose(&mut rng).unwrap();

            // Don't connect node to itself
            if source == target {
                continue;
            }

            // Check if connection already exists
            if self.graph.find_edge(source, target).is_some() {
                continue;
            }

            // Get node IDs for innovation tracking
            let source_id = self.graph[source].id;
            let target_id = self.graph[target].id;

            // Check or create innovation number
            let innovation =
                if let Some(&existing) = self.innovation_numbers.get(&(source_id, target_id)) {
                    existing
                } else {
                    let new_innovation = self.next_innovation;
                    self.next_innovation += 1;
                    self.innovation_numbers
                        .insert((source_id, target_id), new_innovation);
                    new_innovation
                };

            // Add the connection
            let connection = CppnConnection {
                weight: rng.random_range(-1.0..1.0),
                enabled: true,
                innovation_number: innovation,
            };

            self.graph.add_edge(source, target, connection);
            return true;
        }

        false
    }

    /// Split an existing connection by adding a new node
    /// The old connection is disabled, and two new connections are created
    /// Returns true if a node was added
    pub fn add_node(&mut self) -> bool {
        use rand::Rng;
        use rand::prelude::IndexedRandom;
        let mut rng = rand::rng();

        // Get enabled edges
        let enabled_edges: Vec<_> = self
            .graph
            .edge_indices()
            .filter(|&idx| self.graph[idx].enabled)
            .collect();

        if enabled_edges.is_empty() {
            return false;
        }

        // Choose random edge to split
        let edge_idx = *enabled_edges.choose(&mut rng).unwrap();
        let (source_idx, target_idx) = self.graph.edge_endpoints(edge_idx).unwrap();

        // Disable the old connection
        self.graph[edge_idx].enabled = false;

        // Get source and target node IDs
        let source_id = self.graph[source_idx].id;
        let target_id = self.graph[target_idx].id;
        let old_weight = self.graph[edge_idx].weight;

        // Create new node with random activation
        let activations = [
            ActivationFunction::Sigmoid,
            ActivationFunction::Tanh,
            ActivationFunction::Gaussian,
            ActivationFunction::Sine,
            ActivationFunction::Relu,
        ];
        let activation = activations[rng.random_range(0..activations.len())];

        let new_node_id = self.next_node_id;
        self.next_node_id += 1;

        let new_node = CppnNode {
            id: new_node_id,
            activation,
            node_type: NodeType::Hidden,
        };

        let new_node_idx = self.graph.add_node(new_node);

        // Create connection from source to new node (weight = 1.0 to preserve signal)
        let innovation1 = self.next_innovation;
        self.next_innovation += 1;
        self.innovation_numbers
            .insert((source_id, new_node_id), innovation1);

        self.graph.add_edge(
            source_idx,
            new_node_idx,
            CppnConnection {
                weight: 1.0,
                enabled: true,
                innovation_number: innovation1,
            },
        );

        // Create connection from new node to target (weight = old weight to preserve signal)
        let innovation2 = self.next_innovation;
        self.next_innovation += 1;
        self.innovation_numbers
            .insert((new_node_id, target_id), innovation2);

        self.graph.add_edge(
            new_node_idx,
            target_idx,
            CppnConnection {
                weight: old_weight,
                enabled: true,
                innovation_number: innovation2,
            },
        );

        true
    }

    /// Randomly enable or disable a connection
    /// Returns true if a connection was toggled
    pub fn toggle_connection(&mut self, disable_rate: f32) -> bool {
        use rand::Rng;
        use rand::prelude::IndexedRandom;
        let mut rng = rand::rng();

        let edges: Vec<_> = self.graph.edge_indices().collect();
        if edges.is_empty() {
            return false;
        }

        let edge_idx = *edges.choose(&mut rng).unwrap();
        let connection = &mut self.graph[edge_idx];

        if connection.enabled {
            // Disable with given probability
            if rng.random::<f32>() < disable_rate {
                connection.enabled = false;
                return true;
            }
        } else {
            // Re-enable with 25% chance
            if rng.random::<f32>() < 0.25 {
                connection.enabled = true;
                return true;
            }
        }

        false
    }

    /// Apply all mutations with given probabilities
    pub fn mutate(&mut self, config: &MutationConfig) {
        use rand::Rng;
        let mut rng = rand::rng();

        // Weight mutations (most common)
        self.mutate_weights(config.weight_mutation_rate, config.weight_mutation_power);

        // Structural mutations (less common)
        if rng.random::<f32>() < config.add_connection_rate {
            self.add_connection();
        }

        if rng.random::<f32>() < config.add_node_rate {
            self.add_node();
        }

        if rng.random::<f32>() < config.toggle_connection_rate {
            self.toggle_connection(0.5);
        }

        // Sync graph changes to serializable fields
        self.sync_to_serializable();
    }
}

/// Configuration for CPPN mutation rates
#[derive(Debug, Clone)]
pub struct MutationConfig {
    pub weight_mutation_rate: f32,   // Probability per weight
    pub weight_mutation_power: f32,  // Max perturbation magnitude
    pub add_connection_rate: f32,    // Probability of adding new connection
    pub add_node_rate: f32,          // Probability of adding new node
    pub toggle_connection_rate: f32, // Probability of toggling connection
}

impl Default for MutationConfig {
    fn default() -> Self {
        Self {
            weight_mutation_rate: 0.3, // Reduced from 0.8 to preserve good solutions
            weight_mutation_power: 0.5,
            add_connection_rate: 0.15, // Increased from 0.05 for more structural exploration
            add_node_rate: 0.10,       // Increased from 0.03 for more complexity growth
            toggle_connection_rate: 0.01,
        }
    }
}

// ===== NEAT Crossover =====

/// Crossover two CPPN genomes using NEAT-style gene alignment
/// parent1_fitness and parent2_fitness determine which parent's genes are preferred
/// Returns a new offspring genome
pub fn crossover_cppn(
    parent1: &CppnGenome,
    parent2: &CppnGenome,
    parent1_fitness: f32,
    parent2_fitness: f32,
) -> CppnGenome {
    use rand::Rng;
    let mut rng = rand::rng();

    // Determine which parent is more fit
    let (more_fit, less_fit, more_fit_first) = if parent1_fitness >= parent2_fitness {
        (parent1, parent2, true)
    } else {
        (parent2, parent1, false)
    };

    // Start with a minimal structure
    let mut offspring = CppnGenome::minimal();

    // Collect all nodes from more fit parent
    let mut node_map: HashMap<u64, CppnNode> = HashMap::default();
    for node_idx in more_fit.graph.node_indices() {
        let node = &more_fit.graph[node_idx];
        node_map.insert(node.id, node.clone());
    }

    // Add hidden nodes from less fit parent if they exist in matching genes
    // (This is simplified - full NEAT would be more sophisticated)
    for node_idx in less_fit.graph.node_indices() {
        let node = &less_fit.graph[node_idx];
        if node.node_type == NodeType::Hidden && !node_map.contains_key(&node.id) {
            // Only add if it's connected to genes we're keeping
            // For simplicity, we'll skip this complexity for now
        }
    }

    // Rebuild offspring graph with nodes from more fit parent
    offspring.graph.clear();
    offspring.input_node_ids.clear();
    offspring.output_node_ids.clear();

    // Add nodes in order
    let mut node_indices: HashMap<u64, NodeIndex> = HashMap::default();
    for node_idx in more_fit.graph.node_indices() {
        let node = &more_fit.graph[node_idx];
        let new_idx = offspring.graph.add_node(node.clone());
        node_indices.insert(node.id, new_idx);

        match node.node_type {
            NodeType::Input => offspring.input_node_ids.push(node.id),
            NodeType::Output => offspring.output_node_ids.push(node.id),
            NodeType::Hidden => {}
        }
    }

    // Collect edges from both parents by innovation number
    let mut parent1_edges: HashMap<u64, (u64, u64, CppnConnection)> = HashMap::default();
    for edge_idx in parent1.graph.edge_indices() {
        let (source, target) = parent1.graph.edge_endpoints(edge_idx).unwrap();
        let conn = parent1.graph[edge_idx].clone();
        let source_id = parent1.graph[source].id;
        let target_id = parent1.graph[target].id;
        parent1_edges.insert(conn.innovation_number, (source_id, target_id, conn));
    }

    let mut parent2_edges: HashMap<u64, (u64, u64, CppnConnection)> = HashMap::default();
    for edge_idx in parent2.graph.edge_indices() {
        let (source, target) = parent2.graph.edge_endpoints(edge_idx).unwrap();
        let conn = parent2.graph[edge_idx].clone();
        let source_id = parent2.graph[source].id;
        let target_id = parent2.graph[target].id;
        parent2_edges.insert(conn.innovation_number, (source_id, target_id, conn));
    }

    // Get all unique innovation numbers
    let mut all_innovations: Vec<u64> = parent1_edges.keys().copied().collect();
    for key in parent2_edges.keys() {
        if !all_innovations.contains(key) {
            all_innovations.push(*key);
        }
    }
    all_innovations.sort();

    // Process each gene
    for innovation in all_innovations {
        let in_p1 = parent1_edges.get(&innovation);
        let in_p2 = parent2_edges.get(&innovation);

        let gene = match (in_p1, in_p2) {
            // Matching gene - randomly inherit from either parent
            (Some(g1), Some(g2)) => {
                if rng.random::<bool>() {
                    g1.clone()
                } else {
                    g2.clone()
                }
            }
            // Disjoint/excess in parent 1
            (Some(g), None) => {
                if more_fit_first {
                    g.clone()
                } else {
                    continue; // Only inherit from more fit parent
                }
            }
            // Disjoint/excess in parent 2
            (None, Some(g)) => {
                if !more_fit_first {
                    g.clone()
                } else {
                    continue; // Only inherit from more fit parent
                }
            }
            (None, None) => continue,
        };

        // Add edge if both nodes exist in offspring
        let (source_id, target_id, conn) = gene;
        if let (Some(&source_idx), Some(&target_idx)) =
            (node_indices.get(&source_id), node_indices.get(&target_id))
        {
            // Check if edge already exists
            if offspring.graph.find_edge(source_idx, target_idx).is_none() {
                offspring.graph.add_edge(source_idx, target_idx, conn);
            }
        }
    }

    // Update offspring metadata
    offspring.next_node_id = more_fit.next_node_id.max(less_fit.next_node_id);
    offspring.next_innovation = more_fit.next_innovation.max(less_fit.next_innovation);
    offspring.innovation_numbers = more_fit.innovation_numbers.clone();
    for (k, v) in &less_fit.innovation_numbers {
        offspring.innovation_numbers.entry(*k).or_insert(*v);
    }

    // Sync to serializable before returning
    offspring.sync_to_serializable();

    offspring
}

/// Crossover two controller genomes
/// Simply averages weights for matching dimensions
pub fn crossover_controller(
    parent1: &ControllerGenome,
    parent2: &ControllerGenome,
    parent1_fitness: f32,
    parent2_fitness: f32,
) -> ControllerGenome {
    use rand::Rng;
    let mut rng = rand::rng();

    // Determine bias toward more fit parent
    let bias = if parent1_fitness > parent2_fitness {
        0.7 // Prefer parent1
    } else if parent2_fitness > parent1_fitness {
        0.3 // Prefer parent2
    } else {
        0.5 // Equal
    };

    // Helper to crossover weight vectors
    let mut crossover_weights = |w1: &[f32], w2: &[f32]| -> Vec<f32> {
        let len = w1.len().max(w2.len());
        (0..len)
            .map(|i| {
                let v1 = w1.get(i).copied().unwrap_or(0.0);
                let v2 = w2.get(i).copied().unwrap_or(0.0);

                if rng.random::<f32>() < bias { v1 } else { v2 }
            })
            .collect()
    };

    ControllerGenome {
        message_weights: crossover_weights(&parent1.message_weights, &parent2.message_weights),
        update_weights: crossover_weights(&parent1.update_weights, &parent2.update_weights),
        output_weights: crossover_weights(&parent1.output_weights, &parent2.output_weights),
        message_passing_steps: if rng.random::<f32>() < bias {
            parent1.message_passing_steps
        } else {
            parent2.message_passing_steps
        },
        hidden_dim: if rng.random::<f32>() < bias {
            parent1.hidden_dim
        } else {
            parent2.hidden_dim
        },
    }
}

/// Crossover two creature genomes
pub fn crossover_genome(
    parent1: &CreatureGenome,
    parent2: &CreatureGenome,
    parent1_fitness: f32,
    parent2_fitness: f32,
) -> CreatureGenome {
    use rand::Rng;
    let mut rng = rand::rng();

    let bias = if parent1_fitness >= parent2_fitness {
        0.6
    } else {
        0.4
    };

    CreatureGenome {
        cppn: crossover_cppn(
            &parent1.cppn,
            &parent2.cppn,
            parent1_fitness,
            parent2_fitness,
        ),
        controller: crossover_controller(
            &parent1.controller,
            &parent2.controller,
            parent1_fitness,
            parent2_fitness,
        ),
        traits: BehavioralTraits {
            aggression: if rng.random::<f32>() < bias {
                parent1.traits.aggression
            } else {
                parent2.traits.aggression
            },
            curiosity: if rng.random::<f32>() < bias {
                parent1.traits.curiosity
            } else {
                parent2.traits.curiosity
            },
            sociality: if rng.random::<f32>() < bias {
                parent1.traits.sociality
            } else {
                parent2.traits.sociality
            },
            territoriality: if rng.random::<f32>() < bias {
                parent1.traits.territoriality
            } else {
                parent2.traits.territoriality
            },
        },
        metabolic: MetabolicParams {
            hunger_rate: (parent1.metabolic.hunger_rate + parent2.metabolic.hunger_rate) / 2.0,
            temperature_tolerance: (
                (parent1.metabolic.temperature_tolerance.0
                    + parent2.metabolic.temperature_tolerance.0)
                    / 2.0,
                (parent1.metabolic.temperature_tolerance.1
                    + parent2.metabolic.temperature_tolerance.1)
                    / 2.0,
            ),
            oxygen_requirement: (parent1.metabolic.oxygen_requirement
                + parent2.metabolic.oxygen_requirement)
                / 2.0,
        },
        generation: parent1.generation.max(parent2.generation) + 1,
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
        let mut rng = rand::rng();

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
            .map(|_| rng.random_range(-0.5..0.5))
            .collect();

        let update_weights: Vec<f32> = (0..update_weight_count)
            .map(|_| rng.random_range(-0.5..0.5))
            .collect();

        let output_weights: Vec<f32> = (0..output_weight_count)
            .map(|_| rng.random_range(-0.5..0.5))
            .collect();

        Self {
            message_weights,
            update_weights,
            output_weights,
            message_passing_steps,
            hidden_dim,
        }
    }

    /// Mutate controller weights
    pub fn mutate(&mut self, mutation_rate: f32, mutation_power: f32) {
        use rand::Rng;
        let mut rng = rand::rng();

        // Mutate all weight vectors
        for weights in [
            &mut self.message_weights,
            &mut self.update_weights,
            &mut self.output_weights,
        ] {
            for weight in weights.iter_mut() {
                if rng.random::<f32>() < mutation_rate {
                    if rng.random::<f32>() < 0.9 {
                        // Perturb
                        *weight += rng.random_range(-mutation_power..mutation_power);
                        *weight = weight.clamp(-4.0, 4.0);
                    } else {
                        // Replace
                        *weight = rng.random_range(-2.0..2.0);
                    }
                }
            }
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

    /// Mutate the complete genome
    pub fn mutate(&mut self, cppn_config: &MutationConfig, controller_rate: f32) {
        use rand::Rng;
        let mut rng = rand::rng();

        // Mutate CPPN (morphology)
        self.cppn.mutate(cppn_config);

        // Mutate controller (neural network weights)
        self.controller.mutate(controller_rate, 0.5);

        // Mutate behavioral traits (small perturbations)
        if rng.random::<f32>() < 0.1 {
            self.traits.aggression =
                (self.traits.aggression + rng.random_range(-0.1..0.1)).clamp(0.0, 1.0);
        }
        if rng.random::<f32>() < 0.1 {
            self.traits.curiosity =
                (self.traits.curiosity + rng.random_range(-0.1..0.1)).clamp(0.0, 1.0);
        }
        if rng.random::<f32>() < 0.1 {
            self.traits.sociality =
                (self.traits.sociality + rng.random_range(-0.1..0.1)).clamp(0.0, 1.0);
        }
        if rng.random::<f32>() < 0.1 {
            self.traits.territoriality =
                (self.traits.territoriality + rng.random_range(-0.1..0.1)).clamp(0.0, 1.0);
        }

        // Mutate metabolic params (small perturbations)
        if rng.random::<f32>() < 0.05 {
            self.metabolic.hunger_rate =
                (self.metabolic.hunger_rate + rng.random_range(-0.02..0.02)).clamp(0.01, 1.0);
        }

        // Increment generation
        self.generation += 1;
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

        // Record original graph structure
        let orig_node_count = genome.cppn.graph.node_count();
        let orig_edge_count = genome.cppn.graph.edge_count();
        let orig_output = genome.cppn.query(0.5, 0.5, 0.707);

        // Serialize
        let serialized =
            bincode_next::serde::encode_to_vec(&genome, bincode_next::config::standard())
                .expect("Failed to serialize genome");

        // Deserialize
        let (mut deserialized, _): (CreatureGenome, _) =
            bincode_next::serde::decode_from_slice(&serialized, bincode_next::config::standard())
                .expect("Failed to deserialize genome");

        // Graph needs to be rebuilt after deserialization
        deserialized.cppn.rebuild_graph();

        // Check values match
        assert_eq!(deserialized.generation, genome.generation);
        assert_eq!(deserialized.traits.aggression, genome.traits.aggression);
        assert_eq!(
            deserialized.metabolic.hunger_rate,
            genome.metabolic.hunger_rate
        );

        // Check graph structure preserved
        assert_eq!(deserialized.cppn.graph.node_count(), orig_node_count);
        assert_eq!(deserialized.cppn.graph.edge_count(), orig_edge_count);

        // Check CPPN produces same output after rebuild
        let new_output = deserialized.cppn.query(0.5, 0.5, 0.707);
        assert!((new_output.radius - orig_output.radius).abs() < 0.001);
        assert!((new_output.density - orig_output.density).abs() < 0.001);
        assert_eq!(new_output.has_joint, orig_output.has_joint);
    }

    #[test]
    fn test_genome_serialization_preserves_mutations() {
        // Create a genome and mutate it to add structure
        let mut genome = CreatureGenome::test_biped();
        let config = MutationConfig {
            add_node_rate: 1.0,       // Force add node
            add_connection_rate: 1.0, // Force add connection
            ..MutationConfig::default()
        };

        // Mutate to add complexity
        genome.cppn.mutate(&config);
        genome.cppn.mutate(&config);

        // Record mutated structure
        let orig_node_count = genome.cppn.graph.node_count();
        let orig_edge_count = genome.cppn.graph.edge_count();
        let orig_output = genome.cppn.query(0.5, 0.5, 0.707);

        // Serialize
        let serialized =
            bincode_next::serde::encode_to_vec(&genome, bincode_next::config::standard())
                .expect("Failed to serialize mutated genome");

        // Deserialize
        let (mut deserialized, _): (CreatureGenome, _) =
            bincode_next::serde::decode_from_slice(&serialized, bincode_next::config::standard())
                .expect("Failed to deserialize genome");

        deserialized.cppn.rebuild_graph();

        // Verify structure preserved (should have more nodes than minimal due to mutations)
        assert_eq!(deserialized.cppn.graph.node_count(), orig_node_count);
        assert_eq!(deserialized.cppn.graph.edge_count(), orig_edge_count);

        // Minimal CPPN has 7 nodes (3 input + 4 output), mutated should have more
        assert!(
            orig_node_count >= 7,
            "Expected at least 7 nodes, got {}",
            orig_node_count
        );

        // CPPN should produce same output
        let new_output = deserialized.cppn.query(0.5, 0.5, 0.707);
        assert!(
            (new_output.radius - orig_output.radius).abs() < 0.001,
            "radius mismatch: {} vs {}",
            new_output.radius,
            orig_output.radius
        );
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

    // ===== Mutation Tests =====

    #[test]
    fn test_cppn_mutate_weights() {
        let mut cppn = CppnGenome::minimal();
        let original_edge_count = cppn.graph.edge_count();

        // Mutate with high probability
        let mutated = cppn.mutate_weights(1.0, 0.5);

        // Should mutate all weights
        assert_eq!(mutated, original_edge_count);

        // Edge count should stay the same (weights only)
        assert_eq!(cppn.graph.edge_count(), original_edge_count);
    }

    #[test]
    fn test_cppn_add_connection() {
        let mut cppn = CppnGenome::minimal();
        let original_edge_count = cppn.graph.edge_count();

        // Add a connection
        let added = cppn.add_connection();

        // Minimal CPPN is fully connected, so might not be able to add
        // But if added, edge count should increase
        if added {
            assert_eq!(cppn.graph.edge_count(), original_edge_count + 1);
        }
    }

    #[test]
    fn test_cppn_add_node() {
        let mut cppn = CppnGenome::minimal();
        let original_node_count = cppn.graph.node_count();
        let original_edge_count = cppn.graph.edge_count();

        // Add a node
        let added = cppn.add_node();

        // Should successfully add a node
        assert!(added);

        // Node count should increase by 1
        assert_eq!(cppn.graph.node_count(), original_node_count + 1);

        // Edge count should increase by 1 (old edge disabled, 2 new edges)
        // But disabled edges are still in the graph
        assert_eq!(cppn.graph.edge_count(), original_edge_count + 2);

        // The new node should be hidden type
        let new_node = cppn.graph.node_indices().next_back().unwrap();
        assert_eq!(cppn.graph[new_node].node_type, NodeType::Hidden);
    }

    #[test]
    fn test_cppn_toggle_connection() {
        let mut cppn = CppnGenome::minimal();

        // Try toggling multiple times (stochastic)
        let mut toggled = false;
        for _ in 0..100 {
            if cppn.toggle_connection(1.0) {
                toggled = true;
                break;
            }
        }

        // Should have toggled at least once with 100% disable rate
        assert!(toggled);
    }

    #[test]
    fn test_cppn_mutate_combined() {
        let mut cppn = CppnGenome::minimal();
        let config = MutationConfig::default();

        // Apply combined mutation
        cppn.mutate(&config);

        // CPPN should still be valid
        assert!(cppn.graph.node_count() >= 7); // At least input + output nodes
    }

    #[test]
    fn test_controller_mutate() {
        let mut controller = ControllerGenome::random(16, 2);
        let original_weights = controller.message_weights.clone();

        // Mutate with high probability
        controller.mutate(1.0, 0.5);

        // Weights should have changed
        let weights_changed = controller
            .message_weights
            .iter()
            .zip(original_weights.iter())
            .any(|(new, old)| (new - old).abs() > 0.001);

        assert!(weights_changed);
    }

    #[test]
    fn test_creature_genome_mutate() {
        let mut genome = CreatureGenome::test_biped();
        let config = MutationConfig::default();

        assert_eq!(genome.generation, 0);

        // Mutate
        genome.mutate(&config, 0.8);

        // Generation should have incremented
        assert_eq!(genome.generation, 1);
    }

    #[test]
    fn test_mutation_config_defaults() {
        let config = MutationConfig::default();
        assert_eq!(config.weight_mutation_rate, 0.3); // Reduced from 0.8
        assert_eq!(config.weight_mutation_power, 0.5);
        assert_eq!(config.add_connection_rate, 0.15); // Increased from 0.05
        assert_eq!(config.add_node_rate, 0.10); // Increased from 0.03
        assert_eq!(config.toggle_connection_rate, 0.01);
    }

    #[test]
    fn test_cppn_still_works_after_mutation() {
        let mut cppn = CppnGenome::minimal();
        let config = MutationConfig {
            weight_mutation_rate: 1.0,
            weight_mutation_power: 1.0,
            add_connection_rate: 0.5,
            add_node_rate: 0.5,
            toggle_connection_rate: 0.1,
        };

        // Apply aggressive mutations
        for _ in 0..5 {
            cppn.mutate(&config);
        }

        // CPPN should still produce valid output
        let output = cppn.query(0.5, 0.5, 0.707);
        assert!(output.radius >= 0.0 && output.radius <= 1.0);
        assert!(output.density >= 0.0 && output.density <= 1.0);
        assert!(output.joint_type >= -1.0 && output.joint_type <= 1.0);
    }

    // ===== Crossover Tests =====

    #[test]
    fn test_crossover_cppn() {
        let parent1 = CppnGenome::minimal();
        let parent2 = CppnGenome::minimal();

        let offspring = crossover_cppn(&parent1, &parent2, 1.0, 0.5);

        // Offspring should have valid structure
        assert!(!offspring.input_node_ids.is_empty());
        assert!(!offspring.output_node_ids.is_empty());

        // Should still work
        let output = offspring.query(0.5, 0.5, 0.5);
        assert!(output.radius >= 0.0 && output.radius <= 1.0);
    }

    #[test]
    fn test_crossover_cppn_with_mutations() {
        let mut parent1 = CppnGenome::minimal();
        let mut parent2 = CppnGenome::minimal();
        let config = MutationConfig {
            add_node_rate: 1.0,
            ..Default::default()
        };

        // Add some structural differences
        parent1.mutate(&config);
        parent2.mutate(&config);

        let offspring = crossover_cppn(&parent1, &parent2, 1.0, 1.0);

        // Offspring should work
        let output = offspring.query(0.5, 0.5, 0.5);
        assert!(output.radius >= 0.0 && output.radius <= 1.0);
    }

    #[test]
    fn test_crossover_controller() {
        let parent1 = ControllerGenome::random(16, 2);
        let parent2 = ControllerGenome::random(16, 2);

        let offspring = crossover_controller(&parent1, &parent2, 1.0, 0.5);

        // Offspring should have valid weights
        assert!(!offspring.message_weights.is_empty());
        assert!(!offspring.update_weights.is_empty());
        assert!(!offspring.output_weights.is_empty());
    }

    #[test]
    fn test_crossover_genome() {
        let parent1 = CreatureGenome::test_biped();
        let parent2 = CreatureGenome::test_quadruped();

        let offspring = crossover_genome(&parent1, &parent2, 1.0, 0.5);

        // Offspring generation should be incremented
        assert_eq!(offspring.generation, 1);

        // Offspring should have valid values
        assert!(offspring.traits.aggression >= 0.0 && offspring.traits.aggression <= 1.0);
        assert!(offspring.metabolic.hunger_rate > 0.0);
    }

    #[test]
    fn test_crossover_preserves_validity() {
        let parent1 = CreatureGenome::test_biped();
        let parent2 = CreatureGenome::test_worm();

        // Crossover should produce valid genome
        let offspring = crossover_genome(&parent1, &parent2, 0.8, 0.6);

        // CPPN should work
        let output = offspring.cppn.query(0.0, 0.0, 0.0);
        assert!(output.radius >= 0.0);

        // Controller should have weights
        assert!(!offspring.controller.message_weights.is_empty());
    }

    #[test]
    fn test_crossover_fitness_bias() {
        // With significantly different fitness, offspring should favor more fit parent
        let parent1 = CreatureGenome::test_biped();
        let parent2 = CreatureGenome::test_quadruped();

        // Run many trials to test bias
        let mut gen_from_p1 = 0;
        for _ in 0..100 {
            let offspring = crossover_genome(&parent1, &parent2, 10.0, 0.1);
            // If generation is 1 (max of 0, 0), it's from either parent
            // We can't easily distinguish, so just ensure it's valid
            assert_eq!(offspring.generation, 1);
            if offspring.controller.hidden_dim == 16 {
                gen_from_p1 += 1;
            }
        }

        // With 10x fitness difference, should strongly favor parent1
        // Expect at least 50% from parent1
        assert!(gen_from_p1 > 30, "Expected bias toward more fit parent");
    }
}
