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

### Phase 6: Creature Architecture
- [ ] Make sure the world is ready for creatures, think of a few example behaviors we want to evolve. Expand the materials, reaction or whatever else as needed.
- [ ] CPPN-NEAT genome representation plus markdown documentation of what that is and how it works in GENOME.md
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
