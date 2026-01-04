# Sunaba - 2D Physics Sandbox Survival

A 2D falling-sand survival game combining Noita's emergent physics simulation with Terraria's persistent sandbox survival gameplay. Every pixel is simulated with material properties, enabling emergent behaviors like fire spreading, water eroding, gases rising, and structures collapsing.

**Core Pillars:**
1. **Emergent Physics**: Materials behave according to their properties, not special-case code
2. **ML-Evolved Creatures**: Articulated creatures with neural control, evolved via CPPN-NEAT + MAP-Elites
3. **Persistent World**: Player changes persist across sessions
4. **Survival Sandbox**: Terraria-style crafting, building, exploration, creature taming/breeding

## Commands

```bash
# Primary command - run this to validate all changes
just test    # fmt, clippy --fix, tests, release build, web build, spacetime build

# Development
just start   # Run with --regenerate (new world)
just load    # Run release (load existing world)
just profile # Run with puffin profiler (F3 to toggle flamegraph)
just web     # Build and serve web version at localhost:8080

# Individual commands (prefer just test)
cargo run -p sunaba --release
cargo test --workspace
cargo clippy --workspace
cargo fmt --all
```

## SpacetimeDB Multiplayer

```bash
just spacetime-build      # Build WASM server module
just spacetime-start      # Start local server (localhost:3000)
just spacetime-stop       # Stop local server
just spacetime-publish    # Publish module to local server
just spacetime-logs-tail  # Follow server logs
```

**For detailed SpacetimeDB patterns, schema changes, subscriptions, and performance optimization, see `.claude/skills/spacetimedb/SKILL.md` or consult the spacetimedb-reference skill.**

## Workspace Structure

sunaba is organized as a Cargo workspace with 5 crates:

| Crate               | Purpose                                                  | Key Dependencies                                |
|---------------------|----------------------------------------------------------|-------------------------------------------------|
| `sunaba-simulation` | Material definitions, reactions, pixel data              | serde, log                                      |
| `sunaba-creature`   | ML-evolved creatures, simple physics, neural control     | sunaba-simulation, petgraph, rand               |
| `sunaba-core`       | World, entity, levels (re-exports simulation + creature) | sunaba-simulation, sunaba-creature, noise       |
| `sunaba`            | Main binary, rendering, UI, headless training            | wgpu, egui, winit, sunaba-core                  |
| `sunaba-server`     | SpacetimeDB multiplayer server module                    | spacetimedb, sunaba-simulation, sunaba-creature |

### Crate Dependency Graph
```
sunaba (main binary + cdylib for WASM)
├── sunaba-core
│   ├── sunaba-simulation
│   └── sunaba-creature
│       └── sunaba-simulation
└── (render deps: wgpu, egui, winit)

sunaba-server (SpacetimeDB cdylib for WASM)
├── sunaba-simulation
├── sunaba-creature
└── spacetimedb
```

### Developing Individual Crates
```bash
# Test individual crates
cargo test -p sunaba-simulation
cargo test -p sunaba-creature
cargo test -p sunaba-core
cargo test -p sunaba

# Check workspace
cargo check --workspace

# Build only the game binary
cargo build --release -p sunaba
```

## Rust Coding Guidelines

### Error Handling
- Use `anyhow::Result` for all fallible functions
- Use `.context("message")` to add context to errors
- Use `anyhow::anyhow!("message")` for custom errors
- Avoid `.unwrap()` in library code - use `.expect("reason")` or propagate with `?`
- Use `.unwrap_or()` / `.unwrap_or_default()` for safe fallbacks

```rust
use anyhow::{Context, Result};

pub fn load_chunk(&self, x: i32, y: i32) -> Result<Chunk> {
    let path = self.chunk_path(x, y);
    let data = std::fs::read(&path)
        .context("Failed to read chunk file")?;
    let (chunk, _) = bincode::serde::decode_from_slice(&data, bincode::config::standard())
        .context("Failed to deserialize chunk")?;
    Ok(chunk)
}
```

### Async Runtime
- Minimal async - only for wgpu initialization
- Uses `pollster::block_on()` for single-threaded blocking
- Main game loop is synchronous (winit event loop)
- No tokio or async-std

### Memory Management
- Prefer direct ownership over smart pointers (Arc/Rc/RefCell)
- Clone liberally for data-driven types (`MaterialDef`, `ItemStack`, etc.)
- Use `AtomicU64` for thread-safe ID generation (see `entity/mod.rs`)
- Avoid interior mutability unless truly needed

### Testing
- Inline `#[cfg(test)] mod tests` at end of source files
- Use `assert_eq!()` and `assert!()` macros
- Create helper functions for test fixtures: `make_test_material()`, etc.
- No mocking libraries - instantiate real objects directly
- Run `just test` to validate all changes

### Code Style
- Use `rustfmt` defaults
- Use `log` + `env_logger` for logging
- Use `#[derive(Debug, Clone, Serialize, Deserialize)]` liberally
- Data-driven design: define behaviors in data, not code

### Performance
- Hot path (CA update loop) must avoid allocations
- Use `rayon` for parallel chunk updates (checkerboard pattern)
- Profile before optimizing - use `tracy` or `puffin`
- GPU texture upload is often the bottleneck

## Architecture Overview

### Tech Stack
| Component        | Crate                                         |
|------------------|-----------------------------------------------|
| Graphics         | wgpu 27.0                                     |
| Windowing        | winit 0.30                                    |
| UI               | egui 0.33                                     |
| Physics          | Simple kinematic (no external physics engine) |
| Math             | glam 0.25                                     |
| Neural Networks  | ndarray 0.16 (BLAS-accelerated)               |
| Spatial Indexing | rstar 0.12 (R-tree for chunk queries)         |
| Serialization    | serde + bincode + ron                         |
| Compression      | lz4_flex                                      |
| RNG              | rand + rand_xoshiro (deterministic)           |
| Neural/Graph     | petgraph 0.6 (with serde-1 for CPPN)          |
| Raycasting       | bresenham 0.1 (exact pixel traversal)         |
| Stack Vectors    | smallvec 1.13 (avoid heap for small arrays)   |
| Noise Generation | fastnoise-lite 1.1 (WASM-compatible)          |
| Profiling        | puffin + puffin_egui (opt-in feature)         |

### World Structure
```
World
├── Chunks (64x64 pixels each)
│   ├── pixel_data: [u32; 4096]     // material_id + flags
│   ├── temperature: [f32; 256]      // 8x8 coarse grid
│   └── dirty_rect: Option<Rect>     // for partial updates
├── Active chunks: ~25 around player
├── Loaded chunks: ~100 (cached)
└── Unloaded: serialized to disk (bincode + lz4)
```

### Simulation Layers
1. **Cellular Automata** (per-pixel, 60fps) - material movement, reactions
2. **Temperature** (8x8 grid, 30fps) - heat diffusion, state changes
3. **Structural Integrity** (event-driven) - debris conversion on disconnect
4. **Falling Chunks** (kinematic, 60fps) - debris falls with gravity, settles into world

## Project Structure

```
crates/
├── sunaba-simulation/                 # Material simulation foundation
│   └── src/
│       ├── lib.rs
│       ├── materials.rs               # MaterialDef, MaterialId, Materials
│       ├── reactions.rs               # Reaction, ReactionRegistry
│       └── pixel.rs                   # Pixel, pixel_flags, CHUNK_SIZE
│
├── sunaba-creature/                   # ML-evolved creatures + physics
│   └── src/
│       ├── lib.rs
│       ├── traits.rs                  # WorldAccess, WorldMutAccess traits
│       ├── types.rs                   # EntityId, Health, Hunger
│       ├── simple_physics.rs          # CreaturePhysicsState (no external engine)
│       ├── genome.rs                  # CPPN-NEAT genome
│       ├── morphology.rs              # Body generation from CPPN
│       ├── neural.rs                  # DeepNeuralController brain
│       ├── behavior.rs                # GOAP planner
│       ├── sensors.rs                 # Raycasts, material detection
│       ├── spawning.rs                # CreatureManager
│       ├── world_interaction.rs       # Eating, mining, building
│       └── creature.rs                # Main Creature entity
│
├── sunaba-core/                       # World + entity + levels
│   └── src/
│       ├── lib.rs                     # Re-exports simulation + creature
│       ├── world/                     # World management (20+ modules)
│       │   ├── world.rs               # Main orchestrator (~1,200 lines, refactored)
│       │   ├── chunk*.rs              # Chunk data, manager, status
│       │   ├── *_system.rs            # Extracted systems (chemistry, debris, light, mining, persistence, player_physics)
│       │   ├── *_queries.rs           # Stateless utilities (pixel, neighbor, raycasting, collision)
│       │   └── ...                    # Generation, biomes, stats, CA update, RNG traits
│       ├── simulation/
│       │   ├── temperature.rs         # Heat diffusion
│       │   ├── state_changes.rs       # Melt, freeze, boil
│       │   ├── structural.rs          # Structural integrity
│       │   ├── mining.rs              # Mining mechanics
│       │   ├── regeneration.rs        # Resource regeneration
│       │   └── light.rs               # Light propagation
│       ├── entity/
│       │   ├── player.rs              # Player controller
│       │   ├── input.rs               # InputState
│       │   ├── inventory.rs           # Inventory system
│       │   ├── crafting.rs            # Crafting recipes
│       │   ├── tools.rs               # Tool definitions
│       │   └── health.rs              # Health/hunger system
│       └── levels/
│           ├── level_def.rs           # Level definition
│           └── demo_levels.rs         # 16 demo scenarios
│
└── sunaba/                            # Main binary + rendering crate
    └── src/
        ├── main.rs                    # Entry point, CLI
        ├── lib.rs                     # Library root, WASM entry
        ├── app.rs                     # Application state, game loop
        ├── render/
        │   └── renderer.rs            # wgpu pipeline, camera
        ├── ui/
        │   ├── ui_state.rs            # Central UI state
        │   ├── hud.rs                 # Heads-up display
        │   ├── stats.rs               # Debug stats (F1)
        │   ├── tooltip.rs             # Mouse hover info
        │   ├── inventory_ui.rs        # Inventory panel
        │   ├── crafting_ui.rs         # Crafting interface
        │   ├── level_selector.rs      # Level dropdown (L)
        │   └── controls_help.rs       # Help overlay (H)
        └── headless/                  # Offline training (native only)
            ├── training_env.rs
            ├── scenario.rs
            └── map_elites.rs
│
└── sunaba-server/                     # SpacetimeDB multiplayer server
    └── src/
        ├── lib.rs                     # Module declarations + re-exports
        ├── tables.rs                  # SpacetimeDB table definitions (8 tables)
        ├── state.rs                   # Global server state (SERVER_WORLD)
        ├── reducers/
        │   ├── mod.rs                 # Reducer re-exports
        │   ├── lifecycle.rs           # init, client connect/disconnect
        │   ├── world_ticks.rs         # Scheduled simulation ticks (60fps, 30fps, 10fps)
        │   ├── player_actions.rs      # Player movement, placement, mining
        │   ├── creatures.rs           # Creature spawning + feature extraction
        │   ├── monitoring.rs          # Ping, metrics cleanup
        │   └── testing.rs             # Debug/test reducers
        ├── helpers.rs                 # Chunk loading, sync, physics utilities
        ├── world_access.rs            # WorldAccess impl over SpacetimeDB
        └── encoding.rs                # Bincode serialization helpers
```

## Development Phases

### Completed
- **Phase 1-4**: Core simulation, materials, structural integrity, persistence
- **Phase 5**: Extended materials, ore/mining, crafting, inventory, light system

### In Progress (See [DESIGN.md](./DESIGN.md) for design details and [PLAN.md](./PLAN.md) for detailed development plans)

- **Phase 6**: Creature architecture (CPPN-NEAT, neural control, GOAP)
- **Phase 7**: Offline evolution pipeline (MAP-Elites, training scenarios)
- **Phase 8**: Survival integration (taming, breeding, creature persistence)

## In-Game Controls

When changing or adding new controls, update help in web/index.html.

## Notes for Claude

1. **Start simple**: Get basic functionality working before adding complexity
2. **Profile early**: The CA loop is the hot path, measure before optimizing
3. **Data-driven materials**: Resist hardcoding material behaviors
4. **Chunk boundaries**: Most bugs occur at chunk edges - test thoroughly
5. **Rand compatibility**: Always use rand 0.8 stable APIs (`thread_rng()`, `gen_range()`, `r#gen()`). Import `Rng` trait. WorldRng abstraction handles SpacetimeDB `ctx.rng()` and client `thread_rng()`.
6. **SpacetimeDB development**: See `.claude/skills/spacetimedb/SKILL.md` for detailed patterns. CRITICAL: Always run `just test` after schema changes to regenerate and validate both Rust and TypeScript clients.
7. **Data-driven creatures**: Behaviors should emerge from evolution, not code
8. **Neural inference profiling**: Brain updates are hot path for many creatures
9. **Deterministic evolution**: Seeded RNG for reproducible training runs
10. **Behavioral diversity**: MAP-Elites should produce genuinely different strategies
11. **Morphology-controller coupling**: CPPN and brain genome should co-evolve together
12. **Multiplayer architecture**: See `.claude/skills/spacetimedb/SKILL.md` for runtime switching, client sync, subscriptions, and performance optimization patterns.
13. **Phase 2 optimizations (2026-01)**: Raycasting now uses Bresenham algorithm for exact pixel traversal (~2x faster). Neighbor queries use SmallVec for stack allocation (avoids heap for typical radii). CPPN genomes use petgraph's serde-1 feature directly (~100 lines removed, 50% memory reduction per genome, maintains SpacetimeDB bincode compatibility).
14. **Phase 7 optimizations (2026-01)**: Noise generation now uses fastnoise-lite for WASM-compatible procedural terrain. OpenSimplex2 with FBm for biomes, terrain height, caves, ores, and vegetation. 2-4× faster than previous noise crate. Light propagation uses VecDeque-based BFS flood-fill (already optimal, no changes needed).
