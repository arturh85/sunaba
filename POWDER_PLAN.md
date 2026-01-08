# Powder Game Recreation Plan

**Goal**: Create a faithful recreation of Powder Game 1 & 2 as a separate `sunaba-powder` crate that reuses Sunaba's material/physics/chemistry systems to demonstrate that Sunaba's simulation can match or exceed the classic Powder Game's emergent behavior.

## Table of Contents

- [Current State Analysis](#current-state-analysis)
- [Implementation Roadmap](#implementation-roadmap)
- [Phase 1: Core Systems](#phase-1-core-systems-foundation)
- [Phase 2: Powder Game Materials](#phase-2-powder-game-materials)
- [Phase 3: sunaba-powder Crate](#phase-3-sunaba-powder-crate)
- [Phase 4: Polish & Demo](#phase-4-polish--demo)
- [Phase 5: Optional Enhancements](#phase-5-optional-enhancements)
- [Material Reference](#material-reference)
- [Success Criteria](#success-criteria)

---

## Current State Analysis

### Sunaba's Strengths (38 materials, 26 reactions)

✅ **Temperature system** - 8x8 coarse grid, diffusion, state changes
✅ **Structural integrity** - flood fill, collapse, falling chunks
✅ **Fire/burning** - ignition temps, burn rates, products
✅ **Density-based physics** - materials displace lighter ones
✅ **Chemical reactions** - O(1) lookup, conditions, catalysts
✅ **Rich material properties** - tags, fuel, nutrition, hardness
✅ **Basic organic growth** - plant matter + water duplication

### Missing for Powder Game Parity

**Systems:**
- ❌ Electricity propagation (conductors exist, no simulation)
- ❌ Pressure mechanics (grid exists, no accumulation/explosions)
- ❌ Virus/clone spread behaviors
- ❌ Entity AI (ants pathfinding, birds flocking)
- ❌ Advanced plant growth (seeds → trees, directional growth)
- ❌ Triggered explosions (pressure-based, contact-based)

**Materials (Powder Game has 40-45):**
- ❌ Spark, Thunder, Laser (electricity elements)
- ❌ Nitro, C-4, Bomb (advanced explosives)
- ❌ Magma (distinct from lava - super hot)
- ❌ Mercury (heavy liquid metal)
- ❌ Salt, Seawater (dissolving mechanics)
- ❌ Soapy water, Bubbles
- ❌ Fuse (gradual burning wire)
- ❌ Vine (tangled growth)
- ❌ Ant, Bird, Fish (living entities)
- ❌ Clone element (copies neighbors)
- ❌ Virus element (transforms materials)
- ❌ Fan, Pump (interactive pressure tools)

**Interactive Elements:**
- ❌ Pressure visualization (high/low pressure zones)
- ❌ Temperature visualization (thermography mode)
- ❌ Wind tools (fan, cyclone, directional wind)
- ❌ Copy/paste tools
- ❌ Controllable entities (2P support)

---

## Implementation Roadmap

### Timeline Summary

| Phase | Duration | Deliverables |
|-------|----------|-------------|
| **Phase 1: Core Systems** | 2 weeks | Electricity, Pressure, Behaviors, Entity AI |
| **Phase 2: Materials** | 2 weeks | 20+ new materials, 30+ reactions |
| **Phase 3: Powder Crate** | 1 week | sunaba-powder crate, tools, UI |
| **Phase 4: Polish** | 1 week | Testing, scenarios, docs |
| **Phase 5: Enhancements** | Future | P2 features, integration, distribution |

**Total**: 6 weeks for faithful clone

---

## Phase 1: Core Systems Foundation

**Goal**: Extend sunaba-simulation with missing physics systems.

### 1.1 Electricity System

**Add to simulation**: Electrical charge propagation

- New pixel flags: `CONDUCTIVE`, `POWERED`, `SPARK_SOURCE`
- Electrical potential field (8x8 grid like temperature)
- Propagation: Powered conductors charge adjacent conductors
- Spark behavior: Travels along conductors, jumps small gaps
- Thunder: Instant propagation, destroys non-conductors in path
- Laser: Ray-traced beam, reflects off metal at angles

**Files to modify:**
- `sunaba-simulation/src/pixel.rs` - Add electrical flags
- `sunaba-simulation/src/world/electrical_system.rs` - **NEW** module
- `sunaba-simulation/src/materials.rs` - Add `electrical_conductivity` property
- `sunaba-core/src/world/world.rs` - Integrate electrical update step

**Materials enabled**: Spark, Thunder, Laser, Metal (exists), Battery, Wire

**Implementation notes:**
- Only check powered pixels (performance)
- Use dirty rects for electrical propagation
- Limit propagation depth per frame (prevent lag)
- Electrical resistance generates heat (fires)

### 1.2 Pressure System

**Add to simulation**: Pressure accumulation and propagation

- Already have pressure grid (8x8, unused)
- Gas materials increase pressure
- Explosions create pressure waves
- High pressure: Moves liquids/powders, breaks weak materials
- Low pressure: Attracts nearby materials
- Bubble physics: Surface tension, rising

**Files to modify:**
- `sunaba-simulation/src/world/pressure_system.rs` - **NEW** module
- `sunaba-core/src/world/world.rs` - Enable pressure accumulation
- `sunaba-simulation/src/reactions.rs` - Add pressure-triggered reactions

**Materials enabled**: Fan (pressure source), Pump (moves fluids), Soapy water, Bubbles, Nitro (pressure explosion)

**Implementation notes:**
- Damping factor to prevent oscillations
- Max pressure limits (stability)
- Pressure breaks structural materials (converts to powder)
- Explosions create radial pressure waves

### 1.3 Special Behaviors System

**Add to simulation**: Material-specific emergent behaviors

- **Virus**: Spreads to adjacent materials, transforms them (probabilistic)
- **Clone**: Copies adjacent material patterns (replication)
- **Fuse**: Burns gradually in one direction (directional burning)
- **Vine**: Grows in tangled random walk pattern
- **Advanced plant**: Seeds grow upward, roots grow down

**Files to modify:**
- `sunaba-simulation/src/world/special_behaviors.rs` - **NEW** module
- `sunaba-simulation/src/pixel.rs` - Add behavior state flags
- `sunaba-core/src/world/world.rs` - Call special behavior updates

**Materials enabled**: Virus, Clone, Fuse, Vine, Seed (improved)

**Implementation notes:**
- Each behavior type has update function called per frame
- Store state in pixel flags (direction, age, target material)
- Probabilistic spread (not instant, gradual)
- Clone needs pattern recognition (3x3 kernel)

### 1.4 Entity AI System

**Add to creature**: Simple pixel-based entities (not full creatures)

- **Ant**: Random walk, creates "path" material on solids
- **Bird**: Flocking behavior, flies upward, avoids obstacles
- **Fish**: Swims in water, schooling behavior
- Store entities as special pixels with state (direction, age, target)

**Files to modify:**
- `sunaba-creature/src/pixel_entity.rs` - **NEW** lightweight entity type
- `sunaba-core/src/world/entity_system.rs` - Update loop for pixel entities
- `sunaba-simulation/src/materials.rs` - Entity material types

**Materials enabled**: Ant, Bird, Fish

**Implementation notes:**
- Entities are special material types with AI state
- Simple rules (boids for birds, random walk for ants)
- Pathfinding optional (ants use random walk + pheromones)
- Schooling: Alignment, cohesion, separation (boids algorithm)

---

## Phase 2: Powder Game Materials

**Goal**: Add all 40-45 Powder Game materials using new systems.

### 2.1 Electricity Materials

| Material | ID | Type | Properties | Behavior |
|----------|----|----|------------|----------|
| **Spark** | 31 | Gas | Rises, conductive | Travels along conductors, creates fire on contact |
| **Thunder** | 32 | Gas | Instant propagation | Destroys most materials in path, very hot |
| **Laser** | 33 | Special | Ray-traced beam | Reflects off polished metal at angles, cuts materials |
| **Battery** | 34 | Solid | Power source | Slowly depletes, provides electrical charge |
| **Wire** | 35 | Solid | High conductivity | Thin connector, efficient charge transfer |

**New material properties:**
```rust
electrical_conductivity: f32,  // 0.0-1.0 (how well it conducts)
electrical_resistance: f32,    // Heat generation from current
spark_threshold: f32,          // Voltage to arc/jump gaps
```

### 2.2 Explosive Materials

| Material | ID | Type | Properties | Behavior |
|----------|----|----|------------|----------|
| **Nitro** | 36 | Liquid | Highly sensitive | Explodes on pressure/impact, creates large blast |
| **C-4** | 37 | Solid | Stable explosive | Requires trigger (electricity/fire), very powerful |
| **Bomb** | 38 | Solid | Contact explosive | Explodes on impact with powder/pressure |
| **Fuse** | 39 | Solid | Directional burning | Burns gradually, ignites adjacent explosives |

**Explosion mechanics:**
- High-energy reactions create pressure waves (radial)
- Pressure threshold breaks weak materials (converts to powder/air)
- Chain reactions (one explosion triggers nearby explosives)
- Explosion force scales with material energy_released

### 2.3 Fluid & Liquid Materials

| Material | ID | Type | Properties | Behavior |
|----------|----|----|------------|----------|
| **Magma** | 40 | Liquid | 2000°C, viscous | Hotter than lava, creates fire/smoke, ignites everything |
| **Mercury** | 41 | Liquid | Density 13.5, conductive | Very heavy, sinks through most liquids, conducts electricity |
| **Salt** | 42 | Powder | Dissolves in water | Salt + Water → Seawater |
| **Seawater** | 43 | Liquid | Salty, conductive | Conducts electricity better than water |
| **Soapy Water** | 44 | Liquid | Creates bubbles | Soapy + Air (pressure) → Bubble |
| **Bubble** | 45 | Special | Gas-filled shell | Rises, pops on contact with sharp materials |

### 2.4 Organic & Special Materials

| Material | ID | Type | Properties | Behavior |
|----------|----|----|------------|----------|
| **Vine** | 46 | Solid | Grows randomly | Tangled growth pattern, slow, flammable |
| **Ant** | 47 | Entity | Random walk | Creates path material on solids, avoids water |
| **Bird** | 48 | Entity | Flocking | Flies upward, avoids obstacles, groups together |
| **Fish** | 49 | Entity | Schooling | Swims in water only, grouping behavior |
| **Clone** | 50 | Special | Copies patterns | Analyzes 3x3 neighbors, replicates patterns |
| **Virus** | 51 | Special | Transforms materials | Spreads to adjacent, converts to virus |

---

## Phase 3: sunaba-powder Crate

**Goal**: Create standalone demo crate with Powder Game UI/tools.

### 3.1 Crate Structure

```
crates/sunaba-powder/
├── Cargo.toml
├── src/
│   ├── main.rs           # Binary entry point
│   ├── app.rs            # Powder Game app state
│   ├── tools/
│   │   ├── mod.rs        # Tool trait
│   │   ├── pen.rs        # Drawing tools (powder, water, etc.)
│   │   ├── wind.rs       # Wind/air pressure tools
│   │   ├── drag.rs       # Drag tool
│   │   ├── special.rs    # Clone, fireworks, etc.
│   │   └── erase.rs      # Erase/clear tools
│   ├── ui/
│   │   ├── toolbar.rs    # Material/tool selection
│   │   ├── menu.rs       # Settings menu
│   │   └── hud.rs        # Stats, FPS, dot count
│   └── renderer.rs       # Pixel rendering (reuse sunaba's)
├── assets/               # Powder Game assets
│   ├── fonts/           # Pixel fonts
│   └── icons/           # Tool icons
└── README.md
```

### 3.2 Dependencies

```toml
[package]
name = "sunaba-powder"
version = "0.1.0"
edition = "2021"

[dependencies]
sunaba-simulation = { path = "../sunaba-simulation" }
sunaba-core = { path = "../sunaba-core" }
wgpu = "27.0"
winit = "0.30"
egui = "0.33"
glam = "0.25"
log = "0.4"
env_logger = "0.11"
```

### 3.3 Powder Game Tools

**Pen tools** (left/right mouse):
- Select material for left/right mouse button
- Pen size: 1-10 pixels
- Free draw, line, paint fill modes

**Wind tools**:
- Apply pressure/wind in arrow direction
- Air tool: Left = increase pressure, Right = decrease
- Fan: Continuous directional wind
- Cyclone: Circular wind formation (Powder Game 2)

**Special tools**:
- **Drag**: Move materials with mouse (physics simulation)
- **Clone**: Click material to replicate pattern
- **Fireworks**: Particle effects on click
- **Block/Erase**: Draw walls, delete materials
- **Copy/Paste**: Select region, duplicate patterns
- **Text**: Write with material pixels (custom font)

### 3.4 UI Elements

**Toolbar** (bottom of screen):
- 40-45 material buttons with icons
- Left-click: Select for left mouse
- Right-click: Select for right mouse
- Pen size slider (1-10)

**Menu** (top bar):
- Speed control: 0.1x to 4x simulation speed
- Background effects: Air (pressure), TG (temperature), etc.
- Grid: Show/hide grid lines
- Reset: Clear world
- Upload/Save/Load (optional)

**HUD** (top-right corner):
- Dot count: Active particles (10,234 dots)
- FPS counter
- Selected material name (left/right)
- Current tool name

**Background visualization modes**:
- **None**: Black background
- **Air**: Pressure (green=high, blue=low)
- **Line**: Pressure streamlines (wind flow)
- **TG**: Thermography (temperature gradient colors)
- **Blur**: Motion blur
- **Mesh**: Wind vectors as lines
- **Light**: Additive synthesis (glow effects)

---

## Phase 4: Polish & Demo

### 4.1 Faithful Recreation

**Visual matching**:
- Match Powder Game material colors exactly
- Replicate particle sizes (1 pixel per dot)
- Match UI layout and fonts
- Background effects similar to original

**Behavior matching**:
- Replicate reaction speeds/probabilities
- Match explosion radiuses and forces
- Test all material combinations against original
- Tune parameters to feel identical

### 4.2 Demo Scenarios

Create preloaded test scenarios:

1. **Electricity Demo**: Battery → Wire → Spark → Explosions
2. **Pressure Demo**: Fan → Bubbles, Explosions → Pressure waves
3. **Chemistry Demo**: Acid reactions, Salt dissolving, Oil burning
4. **Life Demo**: Vines growing, Ants pathfinding, Birds flocking
5. **Explosives Demo**: Fuse → Gunpowder → Nitro chain reactions
6. **Fluid Dynamics**: Water + Oil + Mercury density layers

Save as `.ron` scenario files in `scenarios/powder/`.

### 4.3 Performance Testing

**Benchmarks**:
- 10,000 active particles at 60fps
- Multiple explosions per frame (5+ simultaneous)
- Electricity propagating through large networks (100+ conductors)
- Complex vine/virus spread (1000+ infected pixels)

**Profiling**:
- Profile CA update loop (should be <10ms per frame)
- Profile electrical system (should be <2ms)
- Profile pressure system (should be <2ms)
- Identify bottlenecks, optimize if needed

**Optimizations** (if needed):
- SIMD for CA updates (already partially done)
- Spatial hashing for entity queries
- Dirty rects for electrical/pressure updates
- Limit propagation depth per frame

### 4.4 Documentation

Create comprehensive documentation:

**POWDER_PLAN.md** (this document):
- Implementation roadmap
- Architecture decisions
- Phase breakdown

**POWDER_MATERIALS.md**:
- Complete material reference (all 51 materials)
- Properties table (density, temp, conductivity, etc.)
- Behavior descriptions

**POWDER_REACTIONS.md**:
- Reaction matrix (material A + B → C)
- All 50+ reactions listed
- Conditions (temperature, pressure, light)

**README.md** (in crate):
- How to run: `cargo run -p sunaba-powder --release`
- Controls: Mouse tools, keyboard shortcuts
- Features: Material list, tool descriptions
- Credits: Powder Game tribute, original by Dan-Ball

---

## Phase 5: Optional Enhancements

**Future additions** (not required for initial release):

### 5.1 Powder Game 2 Features

- **Multiplayer**: 2P controls (WASD + Arrow keys)
- **Player/Fighter entities**: Controllable stickmen
- **Wheels/Joints**: Structural building system (up to 999 joints)
- **Advanced tools**: Cyclone, Create, Pump
- **Zoom levels**: x1 to x16 magnification

### 5.2 Sunaba Integration

- **F8 overlay toggle**: Add to main Sunaba game
- **Mode switcher**: Toggle between "Powder Game Mode" and "Sunaba Mode"
- **Shared materials**: Some materials exist in both modes
- **Teaching tool**: Use Powder Game as demo level in Sunaba

### 5.3 Distribution

- **WASM build**: Compile to web (`wasm32-unknown-unknown`)
- **GitHub Pages**: Host online (like original Powder Game)
- **Standalone binary**: Package for Windows/Mac/Linux
- **Steam**: Potential release as "Powder Game tribute" (with permission)

---

## Material Reference

### Current Sunaba Materials (38)

| ID | Name | Type | Key Properties |
|----|------|------|----------------|
| 0 | AIR | Gas | Empty space |
| 1 | STONE | Solid | Structural, melts to lava |
| 2 | SAND | Powder | Melts to glass |
| 3 | WATER | Liquid | Freezes to ice, boils to steam |
| 4 | WOOD | Solid | Flammable, burns to ash |
| 5 | FIRE | Gas | Rises, adds heat |
| 6 | SMOKE | Gas | Rises |
| 7 | STEAM | Gas | Condenses to water |
| 8 | LAVA | Liquid | Very hot, freezes to stone |
| 9 | OIL | Liquid | Floats on water, flammable |
| 10 | ACID | Liquid | Corrosive |
| 11 | ICE | Solid | Melts to water |
| 12 | GLASS | Solid | Semi-transparent |
| 13 | METAL | Solid | Conductive, hard |
| 14 | BEDROCK | Solid | Indestructible |
| 15 | DIRT | Powder | Easy to mine |
| 16 | PLANT_MATTER | Solid | Grows with water |
| 17 | FRUIT | Powder | High nutrition |
| 18 | FLESH | Powder | High nutrition, decays |
| 19 | BONE | Solid | Structural |
| 20 | ASH | Powder | Burn product |
| 21 | COAL_ORE | Solid | Flammable fuel |
| 22 | IRON_ORE | Solid | Smelts to ingot |
| 23 | COPPER_ORE | Solid | Smelts to ingot |
| 24 | GOLD_ORE | Solid | Smelts to ingot |
| 25 | COPPER_INGOT | Solid | Conductive |
| 26 | IRON_INGOT | Solid | Conductive, hard |
| 27 | BRONZE_INGOT | Solid | Structural |
| 28 | STEEL_INGOT | Solid | Strongest |
| 29 | GOLD_INGOT | Solid | Conductive, soft |
| 30 | GUNPOWDER | Powder | Explosive |
| 31 | POISON_GAS | Gas | Toxic |
| 32 | FERTILIZER | Powder | Plant growth |
| 33 | MOSSY_STONE | Solid | Softer stone |
| 34 | CRYSTAL | Solid | Semi-transparent |
| 35 | BASALT | Solid | Dense volcanic rock |
| 36 | GLOWING_MUSHROOM | Solid | Edible, glows |
| 37 | OBSIDIAN | Solid | Very hard glass |

### New Powder Game Materials (20+)

| ID | Name | Type | Key Feature |
|----|------|------|-------------|
| 38 | SPARK | Gas | Electricity carrier |
| 39 | THUNDER | Gas | Instant electrical destruction |
| 40 | LASER | Special | Ray-traced beam |
| 41 | BATTERY | Solid | Electrical power source |
| 42 | WIRE | Solid | High conductivity |
| 43 | NITRO | Liquid | Pressure-sensitive explosive |
| 44 | C-4 | Solid | Triggered explosive |
| 45 | BOMB | Solid | Contact explosive |
| 46 | FUSE | Solid | Directional burning |
| 47 | MAGMA | Liquid | Super-hot lava |
| 48 | MERCURY | Liquid | Heavy conductive liquid |
| 49 | SALT | Powder | Dissolves in water |
| 50 | SEAWATER | Liquid | Salty, conductive |
| 51 | SOAPY_WATER | Liquid | Creates bubbles |
| 52 | BUBBLE | Special | Gas-filled shell |
| 53 | VINE | Solid | Tangled growth |
| 54 | ANT | Entity | Random walk AI |
| 55 | BIRD | Entity | Flocking AI |
| 56 | FISH | Entity | Schooling AI |
| 57 | CLONE | Special | Pattern replication |
| 58 | VIRUS | Special | Material transformation |

**Total**: 58 materials (38 existing + 20 new)

---

## Critical Files to Modify

### sunaba-simulation (Core Systems)

```
crates/sunaba-simulation/src/
├── pixel.rs                  # Add electrical/behavior flags
├── materials.rs              # Add 20+ materials, new properties
├── reactions.rs              # Add 30+ reactions
└── world/
    ├── electrical_system.rs  # NEW: Electricity propagation
    ├── pressure_system.rs    # NEW: Pressure accumulation
    └── special_behaviors.rs  # NEW: Virus, Clone, Fuse, Vine
```

### sunaba-core (Integration)

```
crates/sunaba-core/src/
└── world/
    ├── world.rs              # Call new system updates
    └── update.rs             # Integrate electrical/pressure/behavior steps
```

### sunaba-creature (Pixel Entities)

```
crates/sunaba-creature/src/
├── pixel_entity.rs           # NEW: Lightweight entity (Ant, Bird, Fish)
└── behavior.rs               # Simple AI (random walk, flocking, schooling)
```

### sunaba-powder (New Crate)

```
crates/sunaba-powder/src/
├── main.rs                   # Binary entry point
├── app.rs                    # Powder Game app state
├── tools/
│   ├── pen.rs               # Drawing tools
│   ├── wind.rs              # Pressure tools
│   ├── drag.rs              # Drag tool
│   └── special.rs           # Clone, fireworks
├── ui/
│   ├── toolbar.rs           # Material palette
│   ├── menu.rs              # Settings
│   └── hud.rs               # Stats
└── renderer.rs              # Pixel rendering
```

---

## Verification Plan

### Testing Strategy

**1. Material Tests** (20+ scenarios):
- Create one scenario per new material
- Verify state changes (melting, freezing, boiling)
- Verify reactions (all combinations)
- Verify special behaviors (virus spreads, clone copies)

**2. System Tests** (isolated):
- **Electricity**: Spark propagates through conductors correctly
- **Pressure**: Explosions create pressure waves, fan moves fluids
- **Behaviors**: Virus transforms materials, vine grows correctly
- **Entities**: Ants pathfind, birds flock, fish school

**3. Integration Tests** (cross-system):
- Electricity + Explosives: Thunder triggers C-4
- Pressure + Fluids: Fan pushes water, creates bubbles
- Temperature + State Changes: Mercury conducts heat well

**4. Performance Tests**:
- 10,000 active particles at 60fps
- Multiple explosions per frame
- Electricity propagating through large networks
- Complex vine/virus spread

**5. Visual Validation**:
- Compare screenshots to original Powder Game
- Material colors match
- Reaction products correct
- Explosion effects similar

### Testing Commands

```bash
# Unit tests for new systems
cargo test -p sunaba-simulation electrical_system
cargo test -p sunaba-simulation pressure_system
cargo test -p sunaba-simulation special_behaviors

# Integration tests for materials
just test-scenario scenarios/powder/electricity.ron
just test-scenario scenarios/powder/pressure.ron
just test-scenario scenarios/powder/explosives.ron

# Run powder game demo
cargo run -p sunaba-powder --release

# Performance benchmark
cargo run -p sunaba-powder --release --features benchmark

# Web build for distribution
just web-powder  # New justfile command
```

---

## Success Criteria

1. ✅ All 40-45 Powder Game materials implemented
2. ✅ All major Powder Game reactions work correctly
3. ✅ Electricity system propagates realistically
4. ✅ Pressure system creates explosions/movement
5. ✅ Special behaviors (virus, clone, vine) emerge naturally
6. ✅ Entity AI (ants, birds) looks believable
7. ✅ 60fps with 10k+ active particles
8. ✅ Separate sunaba-powder crate compiles/runs standalone
9. ✅ Visual output matches Powder Game aesthetics
10. ✅ Comprehensive documentation (POWDER_*.md files)

---

## Risks & Mitigations

**Risk**: Electricity system too slow (every pixel checked)
- **Mitigation**: Only check powered pixels, use dirty rects, limit propagation depth

**Risk**: Pressure system causes instability (oscillations)
- **Mitigation**: Damping factor, max pressure limits, smooth diffusion

**Risk**: Entity AI too complex for pixel-based approach
- **Mitigation**: Start with simple rules, optimize later, consider moving to sunaba-creature if needed

**Risk**: Material count exceeds u16 limit
- **Mitigation**: Not a real risk (~58 materials), but could namespace Powder Game materials separately

**Risk**: Reactions become too slow (O(n²) checks)
- **Mitigation**: Already O(1) with HashMap, just need more entries (~50 total reactions)

---

## References

- **Powder Game**: https://dan-ball.jp/en/javagame/dust/
- **Powder Game Wiki**: https://danball.fandom.com/wiki/Powder_Game
- **Powder Game 2**: https://dan-ball.jp/en/javagame/dust2/
- **Reaction Table**: https://danball.fandom.com/wiki/Powder_Game_Reaction_Table

---

## Next Steps

1. ✅ **Create POWDER_PLAN.md** (this document)
2. ⏳ **Week 1**: Implement electricity system
   - ✅ **Phase 1.1 Foundation** (2024-01-08)
     - Added 5 electrical properties to MaterialDef (conductivity, resistance, spark_threshold, power_generation, power_decay_rate)
     - Added 3 pixel flags (CONDUCTIVE, POWERED, SPARK_SOURCE)
     - Added electrical_potential[64] grid to Chunk (8×8 coarse, mirrors temperature/pressure)
     - All code compiles, `just check` passes
   - ⏳ **Phase 1.2**: Create ElectricalSystem module (~300 lines)
   - ⏳ **Phase 1.3**: Add 5 electrical materials
   - ⏳ **Phase 1.4**: Add electrical reactions
   - ⏳ **Phase 1.5**: Implement special behaviors (Spark/Thunder)
   - ⏳ **Phase 1.6**: Testing & validation
3. ⏳ **Week 2**: Implement pressure system
4. ⏳ **Week 3**: Add special behaviors
5. ⏳ **Week 4**: Add entity AI
6. ⏳ **Week 5**: Add all 20+ materials
7. ⏳ **Week 6**: Create sunaba-powder crate
8. ⏳ **Week 7**: Polish, test, document

---

**This plan provides a clear roadmap to bring Sunaba's simulation to Powder Game parity while maintaining the architectural benefits of a separate crate that reuses the core simulation systems.**
