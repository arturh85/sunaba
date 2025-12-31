# 砂場 Sunaba - 2D Physics Sandbox Survival

## Project Vision

A 2D falling-sand survival game combining Noita's emergent physics simulation with Terraria's persistent sandbox survival gameplay. Every pixel is simulated with material properties, enabling emergent behaviors like fire spreading, water eroding, gases rising, and structures collapsing.

**Core Pillars:**
1. **Emergent Physics**: Materials behave according to their properties, not special-case code
2. **ML-Evolved Creatures**: Articulated creatures with neural control, evolved via CPPN-NEAT + MAP-Elites
3. **Persistent World**: Player changes persist across sessions (unlike Noita's roguelike resets)
4. **Survival Sandbox**: Terraria-style crafting, building, exploration, creature taming/breeding

## Technical Architecture

### Tech Stack
- **Language**: Rust (stable)
- **Graphics**: wgpu (WebGPU API, cross-platform)
- **Windowing**: winit
- **Physics (rigid bodies)**: rapier2d
- **Audio**: rodio (future)
- **Serialization**: serde + bincode (chunk persistence)
- **ML/Neural Networks**: burn (with wgpu backend for GPU inference)
- **Graph Processing**: petgraph (for graph neural networks)
- **Evolution**: Custom/third-party NEAT implementation for neuroevolution

### World Structure

```
World
├── Chunks (64×64 pixels each)
│   ├── pixel_data: [u32; 4096]     // material_id (16-bit) + flags (16-bit)
│   ├── temperature: [f32; 256]      // 8×8 coarse grid for heat
│   ├── pressure: [f32; 256]         // 8×8 coarse grid for gas pressure
│   ├── dirty: bool                  // needs saving
│   └── active_rect: Option<Rect>    // dirty rectangle for updates
├── Active chunks: ~25 around player (3×3 to 5×5 grid)
├── Loaded chunks: ~100 (cached in memory)
└── Unloaded chunks: serialized to disk
```

### Simulation Layers (updated each frame)

1. **Cellular Automata** (per-pixel, 60fps target)
   - Update bottom-to-top for falling materials
   - Checkerboard pattern for parallelization
   - Material interactions and reactions

2. **Temperature/Pressure Fields** (8×8 cells per chunk, 30fps)
   - Heat diffusion between cells
   - State changes (melt, freeze, boil, condense)
   - Gas pressure equalization

3. **Structural Integrity** (event-driven, not per-frame)
   - Triggered when solid pixels removed
   - Flood-fill to find disconnected regions
   - Convert to falling rigid bodies or particles

4. **Rigid Body Physics** (rapier2d, 60fps)
   - Player, creatures, items, falling debris
   - Collision with pixel world boundary

### Material System

Materials defined in data (RON or JSON), not code:

```rust
pub struct MaterialDef {
    pub id: u16,
    pub name: String,
    pub material_type: MaterialType,  // Solid, Powder, Liquid, Gas
    pub density: f32,                 // affects falling, sinking, floating
    pub color: [u8; 4],               // RGBA
    
    // Physical properties
    pub hardness: Option<u8>,         // mining resistance (solids)
    pub friction: Option<f32>,        // sliding (powders)
    pub viscosity: Option<f32>,       // flow speed (liquids)
    
    // Thermal properties
    pub melting_point: Option<f32>,
    pub boiling_point: Option<f32>,
    pub freezing_point: Option<f32>,
    pub ignition_temp: Option<f32>,
    pub conducts_heat: f32,           // 0.0 - 1.0
    
    // State transitions
    pub melts_to: Option<u16>,        // material_id
    pub boils_to: Option<u16>,
    pub freezes_to: Option<u16>,
    pub burns_to: Option<u16>,
    pub burn_rate: Option<f32>,
    
    // Flags
    pub flammable: bool,
    pub structural: bool,             // can support other pixels
    pub conducts_electricity: bool,

    // Creature interaction properties
    pub nutritional_value: Option<f32>,   // calories for creatures (plant_matter, flesh)
    pub toxicity: Option<f32>,            // poison damage when consumed
    pub hardness_multiplier: Option<f32>, // affects mining speed (ore harder than dirt)
    pub structural_strength: Option<f32>, // max weight before collapse

    // Advanced chemistry
    pub oxidizer: bool,                   // enables combustion reactions
    pub fuel_value: Option<f32>,          // energy from burning
    pub electrical_conductivity: Option<f32>,
}

pub enum MaterialType {
    Solid,      // doesn't move (stone, wood, metal)
    Powder,     // falls, piles up (sand, gravel, ash)
    Liquid,     // flows, seeks level (water, oil, lava)
    Gas,        // rises, disperses (steam, smoke, toxic gas)
}
```

### Creature System Architecture

Pre-evolved populations of articulated creatures inhabit the world, using neural networks to control their morphologies and emergent behaviors to survive.

#### Morphology System

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

#### Neural Control Architecture

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

#### Genome Representation

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

#### High-Level Behavior (GOAP)

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

#### Creature-Physics Integration

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

#### Population & Genetics

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

### Chemistry/Reactions

```rust
pub struct Reaction {
    pub input_a: u16,                 // material_id
    pub input_b: u16,                 // material_id (or MATERIAL_ANY)
    pub conditions: ReactionConditions,
    pub output_a: u16,                // what input_a becomes
    pub output_b: u16,                // what input_b becomes
    pub probability: f32,             // 0.0 - 1.0 per contact per frame
}

pub struct ReactionConditions {
    pub min_temp: Option<f32>,
    pub max_temp: Option<f32>,
    pub min_pressure: Option<f32>,
    pub requires_light: bool,
}
```

Example reactions:
- water + lava → steam + stone
- acid + metal → toxic_gas + air
- fire + wood (ignition_temp) → fire + fire (spreads)
- plant + water (light) → plant + plant (growth)

### Chunk Persistence

- Chunks saved as compressed binary (bincode + lz4)
- File structure: `world/chunks/chunk_{x}_{y}.bin`
- Save on unload, load on approach
- Background thread for IO (don't block simulation)

### Performance Targets

| Metric | Target | Notes |
|--------|--------|-------|
| Visible pixels | 800K-1M | ~20 chunks at 64×64 |
| CA update rate | 60 fps | Parallel chunk updates |
| Temp/pressure update | 30 fps | Coarser grid, can lag |
| Rigid bodies | 100-200 | rapier2d handles easily |
| Chunk load time | <10ms | Background thread |
| Memory per chunk | ~20KB | With temp/pressure fields |
| Active creatures | 100+ | At 60 fps target |
| Neural inference/creature | <5ms | Brain update per frame |
| Creature system memory | <100MB | Genomes + runtime state |

## Project Structure

```
sunaba/
├── Cargo.toml
├── src/
│   ├── main.rs                 # Entry point, game loop
│   ├── lib.rs                  # Library root
│   ├── app.rs                  # Application state, wgpu setup
│   ├── world/
│   │   ├── mod.rs
│   │   ├── chunk.rs            # Chunk data structure
│   │   ├── world.rs            # World manager (load/unload/save)
│   │   └── generation.rs       # Procedural terrain generation
│   ├── simulation/
│   │   ├── mod.rs
│   │   ├── cellular.rs         # CA update loop
│   │   ├── materials.rs        # Material registry
│   │   ├── reactions.rs        # Chemistry system
│   │   ├── temperature.rs      # Heat diffusion
│   │   ├── pressure.rs         # Gas pressure
│   │   └── structural.rs       # Structural integrity
│   ├── physics/
│   │   ├── mod.rs
│   │   └── rigid_body.rs       # rapier2d integration
│   ├── render/
│   │   ├── mod.rs
│   │   ├── pipeline.rs         # wgpu render pipeline
│   │   ├── texture.rs          # Pixel buffer → GPU texture
│   │   └── camera.rs           # 2D camera with zoom/pan
│   ├── entity/
│   │   ├── mod.rs
│   │   ├── player.rs           # Player entity
│   │   └── creature.rs         # AI creatures (future)
│   ├── creature/
│   │   ├── mod.rs
│   │   ├── genome.rs           # CPPN-NEAT genome
│   │   ├── morphology.rs       # Body generation
│   │   ├── neural.rs           # Brain (GNN/Transformer)
│   │   ├── behavior.rs         # GOAP planner
│   │   ├── sensors.rs          # Material detection, raycasts
│   │   └── population.rs       # Spawning, breeding
│   ├── ml/
│   │   ├── mod.rs
│   │   ├── neat.rs             # NEAT implementation
│   │   ├── map_elites.rs       # Quality-diversity archive
│   │   └── training.rs         # Offline evolution harness
│   ├── ui/
│   │   ├── mod.rs
│   │   ├── ui_state.rs         # UI state management
│   │   ├── stats.rs            # Debug stats panel (F1)
│   │   ├── tooltip.rs          # Mouse hover tooltips
│   │   └── controls_help.rs    # Help overlay (H key)
│   └── levels/
│       ├── mod.rs
│       └── demo_levels.rs      # 16 demo scenarios
├── assets/
│   ├── materials.ron           # Material definitions
│   ├── reactions.ron           # Reaction definitions
│   └── creatures/
│       └── pretrained/         # Pre-evolved genome library
└── worlds/                     # Saved world data (gitignored)
```

## Development Phases

### Phase 1: Core Simulation ✅ COMPLETED
- [x] Project setup, wgpu boilerplate
- [x] Chunk data structure
- [x] Material registry (hard-coded, RON loading deferred)
- [x] Basic CA: sand, water, stone, air
- [x] Pixel buffer rendering
- [x] Player placeholder (rectangle, WASD movement)
- [x] Camera following player
- [x] Camera zoom controls (+/-, mouse wheel)

**Note:** Material registry is fully functional with 15 materials defined in code (air, stone, sand, water, wood, fire, smoke, steam, lava, oil, acid, ice, glass, metal, bedrock). RON file loading can be added later for modding support but is not blocking progression.

**Additional Phase 1 features implemented:**
- Temperature simulation and state changes (melting, freezing, boiling)
- Fire propagation and burning mechanics
- Chemical reaction system with configurable conditions
- Debug UI with egui integration (stats, help panel, tooltips)
- Demo level system with multiple scenarios
- Temperature overlay visualization

### Phase 2: Materials & Reactions ✅ COMPLETED*
- [x] Temperature field + diffusion
- [x] State changes (melt, freeze, boil)
- [x] Fire propagation
- [x] Gas behavior (rising, disperses - pressure field exists but not fully utilized)
- [x] Reaction system
- [x] More materials (oil, acid, lava, wood, ice, glass, metal, bedrock - 15 total)

**Note:** *Gas pressure equalization infrastructure exists but is not yet utilized. This is an optional enhancement - basic gas behavior (rising/dispersing) works via cellular automata based on density.

### Phase 3: Structural Integrity ✅ COMPLETED
- [x] Anchor detection
- [x] Disconnection check
- [x] Falling debris conversion
- [x] rapier2d integration for falling chunks

**Implementation Details:**
- Event-driven structural checking (triggered when structural pixels removed)
- Bedrock material serves as indestructible anchor
- Flood-fill algorithm finds disconnected regions (max 64px radius)
- Size-based debris conversion: <50 pixels → powder particles, ≥50 pixels → rigid bodies
- Full rapier2d physics integration with debris settling and reconstruction
- 8 dedicated demo levels (levels 9-16) for stress testing structural mechanics

### Additional Features Implemented

**UI System (egui):**
- Debug stats panel (F1 key) - FPS, chunk count, temperature stats, activity metrics
- Tooltip system - shows material name and temperature at cursor position
- Controls help panel (H key) - keyboard reference overlay
- UI state management for toggling overlays

**Demo Level System:**
- 16 demo scenarios showcasing different physics behaviors
- Level navigation with N (next) and P (previous) keys
- Levels 1-8: Physics, chemistry, thermal, and reaction demos
- Levels 9-16: Structural integrity stress tests (bridges, towers, castles, etc.)
- Each level demonstrates specific emergent behaviors

**Visualization:**
- Temperature overlay (T key) - GPU-accelerated heat map with color gradient
  - Blue (frozen) → Cyan (cold) → Green (cool) → Yellow (warm) → Orange (hot) → Red (extreme)
  - 40% opacity blend over world texture
- Debris rendering with rotation and translation

**Camera Controls:**
- Keyboard zoom: +/- or numpad +/-
- Mouse wheel zoom support
- Zoom range: 0.001 (max out) to 0.5 (max in)
- Smooth multiplicative zoom (1.1x per step)

**Input System:**
- Material selection via number keys (0-9)
- Material spawning with left mouse click
- Screen-to-world coordinate conversion
- egui input filtering (prevents world interaction when UI active)

### Phase 4: World Persistence ✅ COMPLETED
- [x] Chunk serialization (bincode + lz4 compression)
- [x] Auto-save on chunk unload + periodic (60s) + manual (F5)
- [x] Cave generation with multi-octave Perlin noise
- [x] Spawn point persistence in world metadata
- [x] Command-line --regenerate flag
- [x] Game mode separation (Persistent World vs Demo Levels)
- [x] Level selector UI (L key) with dropdown menu

### Phase 5: World Enhancement for Creatures
- [ ] Extended material properties (nutritional_value, toxicity, structural_strength)
- [ ] Ore materials and mining mechanics
- [ ] Organic materials (plant_matter, flesh, bone)
- [ ] Enhanced chemistry system (20+ reactions: smelting, cooking, fermentation, explosives)
- [ ] Resource nodes and regeneration (ore veins, plant growth, fruit/seeds)
- [ ] Light propagation and vision system (day/night cycle, light sources)
- [ ] Advanced structural mechanics for creature-built structures
- [ ] **Player inventory system** (resource collection and storage)
- [ ] **Basic crafting mechanics** (material transformation)

### Phase 6: Creature Architecture
- [ ] CPPN-NEAT genome representation
- [ ] Morphology generation (CPPN → rapier2d bodies/joints)
- [ ] Neural controller (GNN or Transformer)
- [ ] Sensory systems (raycasts, material sensors, chemical gradients)
- [ ] GOAP behavior planner (needs, actions, planning)
- [ ] Creature-world interaction (digging, building, damage)
- [ ] Basic creature spawning system
- [ ] **Player-creature interaction foundation** (detection, targeting)

### Phase 7: Offline Evolution Pipeline
- [ ] Headless training environment (simplified physics)
- [ ] MAP-Elites implementation (behavioral diversity grid)
- [ ] Fitness functions (survival, exploration, combat, building, reproduction)
- [ ] Multi-agent training scenarios (predator-prey, competition, hide-and-seek, tool use)
- [ ] Parallel simulation infrastructure (rayon + bevy ECS or custom)
- [ ] Checkpoint system (genome serialization, metrics logging)
- [ ] Pre-evolved creature library (100+ behavioral archetypes across niches)

### Phase 8: Survival Integration & Creature Deployment
- [ ] Creature spawning from pre-trained library
- [ ] Regional specialization (biome-appropriate creatures, population limits)
- [ ] **Tool system** (pickaxe, weapons for mining/combat)
- [ ] **Advanced crafting** (recipes, workstations)
- [ ] **Taming mechanics** (knockout/feeding, taming effectiveness)
- [ ] **Breeding system** (sexual reproduction, NEAT crossover, mutation, inheritance UI)
- [ ] **Player health and needs** (hunger, health, creature attacks)
- [ ] Runtime neural inference optimization (model compression for many creatures)
- [ ] Creature persistence (save/load with world)
- [ ] **Creature management UI** (stats, genetics, commands, breeding visualization)

## Coding Conventions

### Rust Style
- Use `rustfmt` defaults
- Prefer `thiserror` for error types
- Use `log` + `env_logger` for logging
- Avoid `unwrap()` in library code, use `expect()` with context or propagate errors
- Use `#[derive(Debug, Clone)]` liberally

### ECS-lite Approach
- Not using a full ECS (bevy_ecs, specs) to keep things simple
- Entities are structs with components as fields
- Systems are functions that take `&mut World` or specific components
- Can migrate to full ECS later if needed

### Performance Considerations
- Hot path (CA update) should avoid allocations
- Use `rayon` for parallel chunk updates
- Profile before optimizing - use `tracy` or `puffin`
- GPU texture upload is often the bottleneck, batch updates

## Commands

```bash
# Run in debug mode
cargo run

# Run in release mode (much faster simulation)
cargo run --release

# Run tests
cargo test

# Check formatting
cargo fmt --check

# Lint
cargo clippy
```

## In-Game Controls

```
# Movement
WASD           : Move player

# Camera
+/-            : Zoom in/out
Mouse Wheel    : Zoom in/out

# Material Placement
0-9            : Select material to place
Left Click     : Place selected material

# World & Levels
L              : Open level selector
F5             : Manual save (persistent world only)

# UI Toggles
H              : Toggle help panel
F1             : Toggle debug stats
T              : Toggle temperature overlay
```

## Key Algorithms

### CA Update Order (Noita-style)
```
For each frame:
  1. Checkerboard pass 1: Update chunks (0,0), (0,2), (2,0), (2,2)...
  2. Checkerboard pass 2: Update chunks (0,1), (0,3), (2,1), (2,3)...
  3. Checkerboard pass 3: Update chunks (1,0), (1,2), (3,0), (3,2)...
  4. Checkerboard pass 4: Update chunks (1,1), (1,3), (3,1), (3,3)...

Within each chunk:
  For y from bottom to top:
    For x (alternating left-right each row for symmetry):
      Update pixel at (x, y)
```

### Pixel Update Logic
```rust
fn update_pixel(chunk, x, y, materials, reactions) {
    let pixel = chunk.get(x, y);
    let material = materials.get(pixel.material_id);
    
    match material.material_type {
        Powder => update_powder(chunk, x, y, material),
        Liquid => update_liquid(chunk, x, y, material),
        Gas => update_gas(chunk, x, y, material),
        Solid => {}, // solids don't move
    }
    
    // Check reactions with neighbors
    for (nx, ny) in neighbors(x, y) {
        if let Some(reaction) = find_reaction(pixel, chunk.get(nx, ny)) {
            if random() < reaction.probability {
                apply_reaction(chunk, x, y, nx, ny, reaction);
            }
        }
    }
}
```

### Structural Integrity Check
```rust
fn check_integrity(world, removed_x, removed_y) {
    // Only check solid materials
    let region = flood_fill_solids(world, removed_x, removed_y, max_radius=64);
    
    // Is any pixel in region anchored?
    for (x, y) in &region {
        if is_anchor(world, x, y) {  // bedrock, or connected to bedrock
            return;  // stable
        }
    }
    
    // Region is floating - schedule conversion
    if region.len() < 50 {
        convert_to_particles(region);  // small debris
    } else {
        convert_to_rigid_body(region);  // falling chunk
    }
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

- **Model compression**: Quantize neural network weights (f32 → f16 or int8)
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

## Notes for Claude

When working on this project:

1. **Start simple**: Get pixels rendering before adding complexity
2. **Profile early**: The CA loop is the hot path, measure before optimizing
3. **Data-driven materials**: Resist hardcoding material behaviors
4. **Chunk boundaries**: Most bugs will be at chunk edges - test thoroughly
5. **Determinism**: Use seeded RNG for reproducible behavior (important for debugging)
6. **Data-driven creatures**: Resist hardcoding creature behaviors - they should emerge from evolution
7. **Neural inference profiling**: Brain updates are hot path for many creatures, optimize early
8. **Deterministic evolution**: Seeded RNG for reproducible training runs (critical for debugging)
9. **Behavioral diversity**: MAP-Elites should produce genuinely different strategies, not minor variations
10. **Morphology-controller coupling**: CPPN and brain genome should co-evolve together
