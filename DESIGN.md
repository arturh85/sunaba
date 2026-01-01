# Sunaba Design Document

This document contains detailed design specifications for future features (Phases 6-8).
For current coding guidelines and project structure, see [CLAUDE.md](./CLAUDE.md).

## Creature System Architecture

Pre-evolved populations of articulated creatures inhabit the world, using neural networks to control their morphologies and emergent behaviors to survive.

### Morphology System

Creatures have articulated bodies generated from CPPN genomes and simulated using rapier2d:

```rust
pub struct CreatureMorphology {
    pub body_parts: Vec<BodyPart>,      // segments, spheres, polygons
    pub joints: Vec<Joint>,             // connects body parts
    pub mass_distribution: Vec<f32>,
}

pub enum Joint {
    Revolute { angle_limit: (f32, f32) },  // legs, jaws
    Prismatic { extension_limit: (f32, f32) },  // tentacles
    Fixed,  // rigid skeleton connections
}

// CPPN generates morphology procedurally
pub fn generate_morphology(genome: &CppnGenome) -> CreatureMorphology {
    // Query CPPN network at different positions to get body structure
    // Convert to rapier2d RigidBody + JointSet
}
```

### Neural Control Architecture

Graph Neural Networks or Transformers control variable morphologies:

```rust
pub struct CreatureBrain {
    pub network_type: NetworkType,  // GNN (NerveNet) or Transformer
    pub input_dim: usize,           // joint sensors + raycasts + material sensors
    pub output_dim: usize,          // per-joint motor targets
}

pub enum NetworkType {
    GraphNeuralNet {
        node_features: usize,
        edge_features: usize,
        message_passing_steps: usize,
    },
    Transformer {
        embed_dim: usize,
        num_heads: usize,
        num_layers: usize,
    },
}

pub struct SensoryInput {
    pub joint_angles: Vec<f32>,
    pub joint_velocities: Vec<f32>,
    pub body_orientation: f32,
    pub raycasts: Vec<RaycastHit>,     // vision
    pub material_contacts: Vec<u16>,   // touch (material IDs)
    pub chemical_gradients: Vec<f32>,  // smell (food, danger)
}
```

### Genome Representation

```rust
pub struct CreatureGenome {
    pub cppn: CppnNetwork,              // morphology generation
    pub controller: ControllerGenome,   // brain topology/weights
    pub traits: BehavioralTraits,
    pub metabolic: MetabolicParams,
}

pub struct CppnNetwork {
    pub nodes: Vec<CppnNode>,
    pub connections: Vec<CppnConnection>,
    pub innovation_numbers: HashMap<(usize, usize), u64>,  // NEAT tracking
}

pub struct BehavioralTraits {
    pub aggression: f32,      // 0.0 - 1.0
    pub curiosity: f32,
    pub sociality: f32,
    pub territoriality: f32,
}

pub struct MetabolicParams {
    pub hunger_rate: f32,
    pub temperature_tolerance: (f32, f32),
    pub oxygen_requirement: f32,
}
```

### High-Level Behavior (GOAP)

```rust
pub struct CreatureNeeds {
    pub hunger: f32,        // 0.0 (satisfied) to 1.0 (starving)
    pub safety: f32,        // threat level
    pub reproduction: f32,  // breeding drive
    pub territory: f32,     // desire to claim area
}

pub enum CreatureAction {
    MoveTo { target: Vec2 },
    Attack { target: EntityId },
    Eat { material: u16 },
    MineMaterial { pos: (i32, i32), material: u16 },
    PlaceMaterial { pos: (i32, i32), material: u16 },
    Flee { from: Vec2 },
    Mate { partner: EntityId },
}

pub struct ActionDef {
    pub preconditions: Vec<Condition>,
    pub effects: Vec<Effect>,
    pub cost: f32,
}
```

### Creature-Physics Integration

```rust
pub struct CreatureWorldInteraction {
    // Sensing
    pub fn sense_materials(&self, world: &World, position: Vec2, radius: f32) -> Vec<u16>;
    pub fn raycast_vision(&self, world: &World, origin: Vec2, directions: &[Vec2]) -> Vec<RaycastHit>;

    // Modification
    pub fn dig_pixel(&mut self, world: &mut World, pos: (i32, i32)) -> Option<u16>;
    pub fn place_pixel(&mut self, world: &mut World, pos: (i32, i32), material: u16) -> bool;

    // Damage
    pub fn take_damage(&mut self, source: DamageSource, amount: f32);
}

pub enum DamageSource {
    Fire { temperature: f32 },
    Acid,
    Crushing { force: f32 },
    Starvation,
    Attack { attacker: EntityId },
}
```

### Population & Genetics

```rust
pub struct Reproduction {
    pub fn sexual_crossover(
        parent_a: &CreatureGenome,
        parent_b: &CreatureGenome,
    ) -> CreatureGenome {
        // NEAT-style crossover with innovation numbers
        // Preserve matching genes, randomly select disjoint/excess
    }

    pub fn mutate(genome: &mut CreatureGenome, mutation_rate: f32) {
        // Add/remove CPPN nodes
        // Add/remove connections
        // Perturb weights
        // Adjust behavioral traits
    }
}

pub struct Species {
    pub representative: CreatureGenome,
    pub members: Vec<EntityId>,
    pub compatibility_threshold: f32,
}
```

## ML Training Pipeline (Offline Evolution)

Creatures are pre-evolved in headless simulations before deployment to the game.

### MAP-Elites Quality-Diversity Archive

Maintains diverse behavioral repertoire across niches:

```rust
pub struct MapElites {
    pub behavior_dimensions: Vec<BehaviorDimension>,
    pub grid_resolution: Vec<usize>,
    pub elites: HashMap<GridCell, Elite>,  // one champion per niche
}

pub enum BehaviorDimension {
    Locomotion,  // terrestrial, aerial, aquatic, burrowing
    Diet,        // herbivore, carnivore, omnivore, mineralivore
    Social,      // solitary, pack, herd, eusocial
    Strategy,    // aggressive, defensive, stealthy, builder
}

pub struct Elite {
    pub genome: CreatureGenome,
    pub fitness: f32,
    pub behavior_characterization: Vec<f32>,
}
```

### CPPN-NEAT Evolution

```rust
pub struct NeatMutation {
    pub add_node_rate: f32,       // split connection
    pub add_connection_rate: f32,
    pub remove_connection_rate: f32,
    pub weight_perturbation: f32,
    pub weight_replacement_rate: f32,
    pub activation_mutation_rate: f32,
}

pub fn mutate_cppn(genome: &mut CppnGenome, config: &NeatMutation) {
    if rand() < config.add_node_rate {
        // Split random connection, insert new node
    }
    if rand() < config.add_connection_rate {
        // Add connection between unconnected nodes
    }
    // ... weight mutations, etc.
}
```

### Multi-Agent Training Scenarios

1. **Predator-Prey Co-evolution**
   - Prey population evolves escape/hiding strategies
   - Predator population evolves hunting strategies
   - Escalating arms race produces sophisticated behaviors

2. **Resource Competition**
   - Multiple creatures compete for limited food/water
   - Territorial behaviors and social hierarchies emerge
   - Efficient foraging and food caching strategies

3. **Hide-and-Seek (Tool Use)**
   - Hiders learn to build shelters from materials
   - Seekers learn to mine through obstacles
   - Emergent construction and destruction behaviors

4. **Combat Tournament**
   - Direct combat fitness selection
   - Evolution of attack/defense strategies
   - Diversity pressure prevents rock-paper-scissors collapse

### Fitness Functions

```rust
pub struct FitnessMetrics {
    pub survival_time: f32,         // primary: how long did it live?
    pub distance_traveled: f32,     // exploration tendency
    pub resources_gathered: f32,    // foraging success
    pub successful_hunts: u32,      // predator effectiveness
    pub structures_built: u32,      // construction capability
    pub offspring_produced: u32,    // reproductive success
}

pub fn compute_fitness(metrics: &FitnessMetrics) -> f32 {
    // Weighted combination, survival time is primary
    metrics.survival_time * 1.0 +
    metrics.distance_traveled * 0.01 +
    metrics.resources_gathered * 0.5 +
    metrics.successful_hunts as f32 * 2.0 +
    metrics.structures_built as f32 * 1.5
}
```

### Deployment Optimization

- **Model compression**: Quantize neural network weights (f32 -> f16 or int8)
- **Knowledge distillation**: Train smaller "student" networks to mimic evolved "teachers"
- **Batch inference**: Process multiple creature brains in parallel on GPU
- **LOD (Level of Detail)**: Simpler behavior for distant/off-screen creatures

## References

### Physics & Simulation

- [Noita GDC Talk](https://www.youtube.com/watch?v=prXuyMCgbTc) - "Exploring the Tech and Design of Noita"
- [Recreating Noita's Sand Simulation](https://www.youtube.com/watch?v=5Ka3tbbT-9E) - C/OpenGL implementation
- [Falling Sand Simulation Blog](https://blog.macuyiko.com/post/2020/an-exploration-of-cellular-automata-and-graph-based-game-systems-part-4.html)
- [wgpu Tutorial](https://sotrh.github.io/learn-wgpu/)
- [rapier2d Docs](https://rapier.rs/docs/)

### ML & Evolution

- [CPPN-NEAT](http://eplex.cs.ucf.edu/papers/stanley_gpem07.pdf) - "Compositional Pattern Producing Networks" (Stanley, 2007)
- [MAP-Elites](https://arxiv.org/abs/1504.04909) - "Illuminating the Space of Possible Behaviors" (Mouret & Clune, 2015)
- [NerveNet](https://arxiv.org/abs/1809.08693) - "Learning Transferable Graph Neural Networks"
- [AMORPHEUS](https://arxiv.org/abs/2302.14543) - "Transformer for Morphological Control"
- [Multi-Agent Autocurricula](https://arxiv.org/abs/1909.07528) - "Emergent Tool Use" (OpenAI, 2019)
- [GOAP](http://alumni.media.mit.edu/~jorkin/goap.html) - "Goal-Oriented Action Planning" (Orkin, 2006)
- [Quality-Diversity](https://quality-diversity.github.io/) - QD algorithms overview
