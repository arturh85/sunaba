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
just test    # fmt, clippy --fix, tests, release build, web build

# Development
just start   # Run with --regenerate (new world)
just load    # Run release (load existing world)
just web     # Build and serve web version at localhost:8080

# Individual commands (prefer just test)
cargo run --release
cargo test
cargo clippy
cargo fmt
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
| Component | Crate |
|-----------|-------|
| Graphics | wgpu 27.0 |
| Windowing | winit 0.30 |
| UI | egui 0.33 |
| Physics | rapier2d 0.18 |
| Math | glam 0.25 |
| Serialization | serde + bincode + ron |
| Compression | lz4_flex |
| RNG | rand + rand_xoshiro (deterministic) |
| ML | burn 0.18 + petgraph 0.6 |

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
4. **Rigid Body Physics** (rapier2d, 60fps) - player, creatures, debris

## Project Structure

```
src/
├── main.rs                 # Entry point, CLI
├── lib.rs                  # Library root, WASM entry
├── app.rs                  # Application state, game loop
├── world/
│   ├── chunk.rs            # Chunk data structure (64x64)
│   ├── world.rs            # World manager, chunk loading
│   ├── generation.rs       # Procedural terrain (Perlin noise)
│   ├── persistence.rs      # Save/load (bincode + lz4)
│   └── biome.rs            # Biome definitions
├── simulation/
│   ├── materials.rs        # Material registry (15+ materials)
│   ├── reactions.rs        # Chemistry system (20+ reactions)
│   ├── temperature.rs      # Heat diffusion
│   ├── state_changes.rs    # Melt, freeze, boil
│   ├── structural.rs       # Structural integrity
│   ├── mining.rs           # Mining mechanics
│   ├── regeneration.rs     # Resource regeneration
│   └── light.rs            # Light propagation
├── physics/
│   └── rigid_body.rs       # rapier2d integration, debris
├── render/
│   └── renderer.rs         # wgpu pipeline, camera
├── entity/
│   ├── player.rs           # Player controller
│   ├── inventory.rs        # Inventory system
│   ├── crafting.rs         # Crafting recipes
│   ├── tools.rs            # Tool definitions
│   └── health.rs           # Health/hunger system
├── creature/               # ML creatures (Phase 6+)
│   ├── genome.rs           # CPPN-NEAT genome
│   ├── morphology.rs       # Body generation
│   ├── neural.rs           # Brain (GNN/Transformer)
│   ├── behavior.rs         # GOAP planner
│   ├── sensors.rs          # Raycasts, material detection
│   ├── spawning.rs         # Creature manager
│   └── world_interaction.rs
├── ui/
│   ├── ui_state.rs         # Central UI state
│   ├── hud.rs              # Heads-up display
│   ├── stats.rs            # Debug stats (F1)
│   ├── tooltip.rs          # Mouse hover info
│   ├── inventory_ui.rs     # Inventory panel
│   ├── crafting_ui.rs      # Crafting interface
│   ├── level_selector.rs   # Level dropdown (L)
│   └── controls_help.rs    # Help overlay (H)
└── levels/
    ├── level_def.rs        # Level definition
    └── demo_levels.rs      # 16 demo scenarios
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

```
# Movement
WASD           : Move player
Space          : Jump

# Camera
+/-            : Zoom in/out
Mouse Wheel    : Zoom in/out

# Interaction
0-9            : Select material
Left Click     : Place material
Right Click    : Instant mine

# World
L              : Level selector
F5             : Manual save

# UI Toggles
H              : Help panel
F1             : Debug stats
T              : Temperature overlay
```

## Key Algorithms

### CA Update Order (Noita-style)
```
For each frame:
  Checkerboard pattern (4 passes) for parallel chunk updates

Within each chunk (bottom to top):
  For y from 0 to 63:
    For x (alternating direction each row):
      Update pixel based on material type
      Check reactions with neighbors
```

### Structural Integrity
```rust
fn check_integrity(world, removed_pos) {
    let region = flood_fill_solids(removed_pos, max_radius=64);

    if !region.iter().any(|p| is_anchored(p)) {
        if region.len() < 50 {
            convert_to_particles(region);
        } else {
            convert_to_rigid_body(region);
        }
    }
}
```

## Notes for Claude

1. **Start simple**: Get basic functionality working before adding complexity
2. **Profile early**: The CA loop is the hot path, measure before optimizing
3. **Data-driven materials**: Resist hardcoding material behaviors
4. **Chunk boundaries**: Most bugs occur at chunk edges - test thoroughly
5. **Determinism**: Use seeded RNG for reproducible behavior (important for debugging)
6. **Data-driven creatures**: Behaviors should emerge from evolution, not code
7. **Neural inference profiling**: Brain updates are hot path for many creatures
8. **Deterministic evolution**: Seeded RNG for reproducible training runs
9. **Behavioral diversity**: MAP-Elites should produce genuinely different strategies
10. **Morphology-controller coupling**: CPPN and brain genome should co-evolve together
