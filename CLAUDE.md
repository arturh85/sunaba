# Sunaba - 2D Physics Sandbox Survival

A 2D falling-sand survival game combining Noita's emergent physics simulation with Terraria's persistent sandbox survival gameplay. Every pixel is simulated with material properties, enabling emergent behaviors like fire spreading, water eroding, gases rising, and structures collapsing.

**Core Pillars:**
1. **Emergent Physics**: Materials behave according to their properties, not special-case code
2. **ML-Evolved Creatures**: Articulated creatures with neural control, evolved via CPPN-NEAT + MAP-Elites
3. **Persistent World**: Player changes persist across sessions
4. **Survival Sandbox**: Terraria-style crafting, building, exploration, creature taming/breeding

## Quick Start

**Primary Commands:**
```bash
just check [crate]  # After code changes: clippy + fmt + check (10-50s) - USE THIS FIRST
just test [crate]   # Before commit: check + tests (1-2min)
just test-ci        # Before push: full CI validation (10-12min)
```

> **Development Workflow**: Always run `just check` after making code changes. Run `just test` before committing. Run `just test-ci` before pushing to ensure full CI passes.

**Development:**
```bash
just start   # Run with --regenerate (new world)
just load    # Run release (load existing)
just profile # Run with puffin profiler (F3 to toggle flamegraph)
just web     # Build and serve WASM (localhost:8080)
```

**Fast Iteration (Hot Reload):**
```bash
# Auto-rebuild on file changes (builds release profile)
just watch         # Auto-build on save
just watch-check   # Auto-compile with clippy
just watch-test    # Auto-compile and run tests
just watch-all     # Watch build AND tests in parallel (split tmux session)

# INSTANT LAUNCH (~100ms, no Cargo overhead) - use with `just watch`
just run           # Execute release binary directly (truly instant!)
just run <args>    # Pass any arguments (e.g., just run --some-flag)

# Traditional build (1-2s Cargo overhead for file locks)
just start         # cargo run with --regenerate
just load          # cargo run (load existing world)
```

> **Instant Launch Workflow** ⭐ **RECOMMENDED**:
> 1. **Terminal 1**: Run `just watch` - auto-builds release on every save
> 2. **Terminal 2**: Edit code in your editor, bacon compiles automatically
> 3. **Terminal 3**: **`just run`** - **Truly instant launch (~100ms)**!
>
> **Bacon keyboard shortcuts** (while `just watch` is running):
> - **b** - Build release (default, auto-selected)
> - **r** - Build and run release
> - **T** - Build tests (switch to this when working on tests for faster `just test`)
> - **c** - Type-check only (no build)
> - **l** - Lint with clippy
> - **t** - Run tests
>
> **Fast testing workflow**: When working on tests, press **'T'** in bacon to switch to test-building mode. Bacon will compile test binaries on save, making `just test` run instantly (5-10s vs 30-60s).
>
> **Why instant?** `just run` executes `./target/release/sunaba --regenerate` directly, bypassing Cargo's file lock and dependency checks (1-2s overhead). With `just watch` pre-building artifacts, you get true hot-reload feel!

**SpacetimeDB Multiplayer:**
```bash
just spacetime-build      # Build WASM server
just spacetime-start      # Start local server (localhost:3000)
just spacetime-stop       # Stop local server
just spacetime-publish    # Publish to server
just spacetime-logs-tail  # Follow logs
```

> For SpacetimeDB patterns, schema changes, and subscriptions, see `.claude/skills/spacetimedb/SKILL.md`

**Screenshots (Visual Iteration):**
```bash
# List available levels and UI panels
just list-levels             # List all available demo levels with IDs
cargo run --release --features headless -- --list-ui-panels  # List UI panels

# Headless screenshots (world/materials only, no UI)
just screenshot <level_id>   # Capture screenshot of level (1920x1080)
just screenshot 3 800 600    # Custom resolution (800x600)
just screenshot-all          # Capture all demo levels at once
```

> **Visual Iteration Workflow**:
>
> **For world/material screenshots (headless)**:
> 1. Make changes to your code
> 2. `just screenshot <level_id>` to capture the result
> 3. Use the Read tool to view: `screenshots/level_<id>.png`
> 4. Iterate based on visual feedback
>
> **For UI screenshots** (requires manual capture):
> 1. Claude requests a UI screenshot: "Please capture a screenshot of the Parameters panel"
> 2. User launches the game: `just start` or `just load`
> 3. User opens the requested panel (see keybindings below)
> 4. User captures screenshot with OS tool (Shift+Cmd+5 on Mac, PrintScreen on Windows/Linux)
> 5. User saves to `screenshots/ui_<panel_name>.png`
> 6. Claude uses Read tool to view and provide feedback
>
> **UI Panel Keybindings**:
> - **P** - Toggle Parameters/settings panel
> - **I** - Toggle Inventory panel
> - **C** - Toggle Crafting panel
> - **L** - Toggle Logger panel
> - **Tab** - Toggle dock (includes worldgen editor, level selector)
> - **Esc** - Close current panel/menu
>
> **Level Screenshot Examples**:
> - `just screenshot 0` - Basic Physics Playground
> - `just screenshot 3` - Material Showcase
> - `just screenshot 17` - Phase 5 Materials
> - `just screenshot 18` - Alchemy Lab
> - `just screenshot 20` - Day/Night Cycle
>
> **Custom scenarios**: To add custom screenshot scenarios, create new level generators in `crates/sunaba-core/src/levels/demo_levels.rs` and register them in `level_def.rs`.
>
> Screenshots are saved to `screenshots/` directory (gitignored). Headless screenshots simulate 60 frames (1 second) before capturing to let physics settle.

> **Note:** All commands support optional `<crate>` parameter for targeted operations.

## Architecture

### Workspace Structure

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

### Project Structure
```
crates/
├── sunaba-simulation/          # Material definitions, reactions, pixel data
│   └── src/ (materials.rs, reactions.rs, pixel.rs)
├── sunaba-creature/            # ML-evolved creatures, neural control, physics
│   └── src/ (genome.rs, morphology.rs, neural.rs, behavior.rs, creature.rs, ...)
├── sunaba-core/                # World + entity + levels
│   └── src/
│       ├── world/*.rs          # 20+ modules: world.rs, chunk*.rs, *_system.rs, *_queries.rs
│       ├── simulation/         # Temperature, state changes, structural, mining, light
│       ├── entity/             # Player, inventory, crafting, tools, health
│       └── levels/             # Level definitions, 16 demo scenarios
├── sunaba/                     # Main binary + rendering
│   └── src/
│       ├── main.rs, lib.rs, app.rs
│       ├── render/renderer.rs  # wgpu pipeline
│       ├── ui/                 # HUD, stats, inventory, crafting, tooltips
│       └── headless/           # Offline training (MAP-Elites)
└── sunaba-server/              # SpacetimeDB multiplayer
    └── src/ (tables.rs, state.rs, reducers/, helpers.rs, world_access.rs)
```

## Development Guide

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

### Development Phases

**Completed:**
- **Phase 1-4**: Core simulation, materials, structural integrity, persistence
- **Phase 5**: Extended materials, ore/mining, crafting, inventory, light system

**In Progress:** (See [DESIGN.md](./DESIGN.md) for design details and [PLAN.md](./PLAN.md) for detailed development plans)
- **Phase 6**: Creature architecture (CPPN-NEAT, neural control, GOAP)
- **Phase 7**: Offline evolution pipeline (MAP-Elites, training scenarios)
- **Phase 8**: Survival integration (taming, breeding, creature persistence)

## In-Game Controls

When changing or adding new controls, update help in web/index.html.

## Domain-Specific Notes

### Simulation
- **Start simple** - get basic functionality working before adding complexity
- **Profile early** - CA update loop is the hot path, measure before optimizing
- **Data-driven materials** - resist hardcoding material behaviors
- **Chunk boundaries** - most bugs occur at chunk edges, test thoroughly
- **GPU texture upload** is often the bottleneck (not CA logic)

### Creatures & Evolution
- **Data-driven creatures** - behaviors should emerge from evolution, not code
- **Neural inference profiling** - brain updates are hot path for many creatures
- **Deterministic evolution** - seeded RNG for reproducible training runs
- **Behavioral diversity** - MAP-Elites should produce genuinely different strategies
- **Morphology-controller coupling** - CPPN and brain genome should co-evolve together

### Multiplayer (SpacetimeDB)
- See `.claude/skills/spacetimedb/SKILL.md` for detailed patterns
- **Always run `just test` after schema changes** to regenerate and validate both Rust and TypeScript clients
- **Rand compatibility** - WorldRng abstraction handles `ctx.rng()` (SpacetimeDB) vs `thread_rng()` (client)
  - Use rand 0.8 stable APIs: `thread_rng()`, `gen_range()`, `r#gen()`, import `Rng` trait

### Development Workflow & Tooling
- **Rapid iteration**: `just check [crate]` for fast validation (clippy --fix, fmt, check)
- **Comprehensive validation**: `just test [crate]` or `just test` before pushing
- **Build caching (optional)**: Enable sccache for 50-80% faster clean builds
  - Install: `cargo install sccache` (or `brew install sccache` on macOS)
  - Enable: `export RUSTC_WRAPPER=sccache` in your shell profile (~/.zshrc or ~/.bashrc)
  - Verify: `sccache --show-stats` to see cache hits
  - Note: Not required for CI or coverage (automatically disabled where needed)
- **LSP tools**: Prefer `mcp__rust__lsp_*` tools for refactoring (rename_symbol, find_references, get_definitions)
  - These leverage rust-analyzer for accuracy with macros and trait implementations
