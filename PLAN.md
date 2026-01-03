# Project Development Plan

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
- [x] Extended material properties (nutritional_value, toxicity, structural_strength)
- [x] Ore materials and mining mechanics
- [x] Organic materials (plant_matter, flesh, bone)
- [x] Enhanced chemistry system (20+ reactions: smelting, cooking, fermentation, explosives)
- [x] Resource nodes and regeneration (ore veins, plant growth, fruit/seeds)
- [x] Light propagation and vision system (day/night cycle, light sources)
- [x] Advanced structural mechanics for creature-built structures
- [x] **Player inventory system** (resource collection and storage)
- [x] **Basic crafting mechanics** (material transformation)

### Phase 6: Creature Architecture ✅ COMPLETED
- [x] Make sure the world is ready for creatures, think of a few example behaviors we want to evolve. Expand the materials, reaction or whatever else as needed.
- [x] CPPN-NEAT genome representation plus markdown documentation of what that is and how it works in GENOME.md
- [x] Morphology generation (CPPN → rapier2d bodies/joints)
- [x] Neural controller (GNN or Transformer) - SimpleNeuralController implemented
- [x] Sensory systems (raycasts, material sensors, chemical gradients)
- [x] GOAP behavior planner (needs, actions, planning)
- [x] Creature-world interaction (digging, building, damage)
- [x] Basic creature spawning system
- [x] **Creature movement with pixel-based collision** (gravity, wandering, 70% mining when blocked)
- [x] **Neural-physics integration** (extract_body_part_features extracts actual rapier2d physics data)
- [x] **Motor command wiring** (neural output controls joint actuation via target angles)
- [x] **NEAT mutation operators** (mutate_weights, add_connection, add_node, toggle_connection)
- [x] **NEAT crossover/reproduction** (crossover_cppn, crossover_controller, crossover_genome)
- [x] **CPPN graph serialization** (sync_to_serializable, rebuild_graph preserves structure)
- [ ] **Player-creature interaction foundation** (detection, targeting) - deferred to Phase 8

### Phase 7: Creature Evolution Research (Active)

This phase is iterative experimentation, not linear completion. Success is measured by creature behaviors, not checkboxes.

**Infrastructure (Complete):**
- [x] Headless training environment (`src/headless/` module with feature flag)
- [x] MAP-Elites implementation (10x10 behavioral diversity grid)
- [x] Fitness functions (Distance, Foraging, Survival, Movement, FoodCollection, Composite)
- [x] Training scenarios (Locomotion, SimpleLocomotion, Foraging, Survival, Balanced, Parcour)
- [x] Parallel simulation (rayon), checkpoints (bincode), HTML reports, GIF capture
- [x] CLI training (`--train`, `--scenario`, `--generations`, `--population`, `--output`)

#### 7a: Locomotion Research 🔬
Goal: Creatures that reliably move across flat terrain.
- [ ] Motor control tuning (joint forces, damping, friction)
- [ ] Body physics calibration (mass distribution, ground contact)
- [ ] Simple morphology experiments (biped, quadruped, worm)
- [ ] Fitness function refinement (distance vs energy efficiency)
- [ ] Baseline: creature moves >100px in 30 seconds consistently in a chosen direction.

#### 7b: Self-Sustaining Creatures 🔬
Goal: Creatures that find food and survive without intervention.
- [ ] Hunger-driven behavior (foraging when hungry)
- [ ] Food detection tuning (sensor range, gradient following)
- [ ] Energy economics (movement cost vs nutrition gain)
- [ ] Survival duration experiments (how long can they last?)
- [ ] Baseline: creature survives >60 seconds in foraging scenario

#### 7c: Training Pipeline Refinement 🔬
Goal: MAP-Elites produces genuinely diverse, viable creatures.
- [ ] Behavioral descriptor tuning (what axes matter?)
- [ ] Tournament vs uniform selection experiments
- [ ] Mutation rate calibration (exploration vs exploitation)
- [ ] Checkpoint analysis (when do good behaviors emerge?)
- [ ] Baseline: 50%+ grid coverage with viable creatures

#### 7d: Advanced Behaviors (Future)
Goal: Complex, goal-directed creature behaviors.
- [ ] Obstacle navigation (parcour scenario refinement)
- [ ] Mining behavior (break through walls to reach food)
- [ ] Multi-objective optimization (move AND eat AND survive)
- [ ] Tool use experiments (if applicable)

#### 7e: Multi-Agent Dynamics (Future)
Goal: Creatures that interact meaningfully with each other.
- [ ] Predator-prey scenarios
- [ ] Competition for resources
- [ ] Cooperative behaviors (if they emerge)
- [ ] Population dynamics

#### Future Infrastructure
- [ ] GPU-accelerated CA simulation (wgpu compute shaders for faster training)
- [ ] Pre-evolved creature library (100+ behavioral archetypes across niches)

### Phase 8: Survival Integration
Requires stable creature behaviors from Phase 7.

#### 8a: Creature Deployment
- [ ] Spawn pre-trained creatures in persistent world
- [ ] Regional creature populations (biome specialization)
- [ ] Creature persistence (save/load with world)
- [ ] Population limits and respawning
- [ ] Runtime neural inference optimization (model compression for many creatures)

#### 8b: Player Interaction
- [ ] Player-creature collision and combat
- [ ] Player health and needs (hunger, health, creature attacks)
- [ ] Taming mechanics (knockout, feeding, trust levels)
- [ ] Creature commands (follow, stay, attack)
- [ ] Tamed creature persistence

#### 8c: Breeding System
- [ ] Sexual reproduction (mate selection, NEAT crossover)
- [ ] Mutation during breeding
- [ ] Inheritance visualization UI
- [ ] Selective breeding for traits

#### 8d: Creature Management
- [ ] Creature stats panel (health, hunger, genetics)
- [ ] Breeding pen / incubation
- [ ] Creature inventory / storage

#### 8e: Advanced Systems (Future)
- [ ] Tool system (pickaxe, weapons for mining/combat)
- [ ] Advanced crafting (recipes, workstations)
