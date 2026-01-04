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

# SpacetimeDB Multiplayer
just spacetime-build      # Build WASM module
just spacetime-start      # Start local instance
just spacetime-stop       # Stop local instance
just spacetime-publish  # Publish to local server
just spacetime-logs-tail  # Follow server logs

# Individual commands (prefer just test)
cargo run -p sunaba --release
cargo test --workspace
cargo clippy --workspace
cargo fmt --all
```

## SpacetimeDB Multiplayer Architecture

Real-time multiplayer via [SpacetimeDB](https://spacetimedb.com/), a database-centric server framework compiling Rust to WASM.

**Quick Start:**
```bash
# Server commands
just spacetime-build          # Build server WASM module
just spacetime-start          # Start local server (localhost:3000)
just spacetime-publish        # Publish module to local server
just spacetime-logs-tail      # View server logs

# Client commands (runtime switchable!)
just start                    # Start in singleplayer (default)
just join                     # Connect to localhost:3000 on startup
just join-prod                # Connect to sunaba.app42.blue on startup
cargo run --features multiplayer_native -- --server <url>  # Custom server
```

### Runtime Connection Management

**Default Mode:** Singleplayer - Game starts disconnected, multiplayer is opt-in

**Connection Methods:**
1. **CLI Argument:** `--server <url>` connects on startup
2. **In-Game UI:** Press `M` key → Multiplayer panel → Select server → Connect
3. **Justfile Shortcuts:** `just join` or `just join-prod`

**Connection Flow:**
- **Singleplayer → Multiplayer:** Saves singleplayer world, switches to server-authoritative mode
- **Multiplayer → Singleplayer:** Restores singleplayer world from snapshot
- **Reconnection:** Automatic with exponential backoff (1s, 2s, 4s, 8s, max 30s)
- **Error Handling:** User-friendly messages with retry option

**UI States:** Disconnected (server selection) | Connecting | Connected (stats) | Reconnecting | Error

### Client Architecture (Dual SDK Approach)

| Platform | SDK | Implementation | Status |
|----------|-----|----------------|--------|
| **Native** | Rust SDK | `crates/sunaba/src/multiplayer/client.rs` | ✅ Runtime switchable |
| **WASM** | TypeScript SDK | `web/js/spacetime_bridge.js` → `window.spacetimeClient` | ✅ Runtime switchable |

**Feature flags:**
- `multiplayer` - Parent feature enabling all multiplayer code
- `multiplayer_native` - Native Rust SDK (depends on `multiplayer`)
- `multiplayer_wasm` - WASM TypeScript SDK (depends on `multiplayer`)
- When `multiplayer` disabled: singleplayer-only build, no server dependencies
- When `multiplayer` enabled: runtime-switchable between singleplayer/multiplayer

### Server Architecture

**Feature Gating:** Server builds **without** `evolution` and `regeneration` features, eliminating most `rand` dependencies. SpacetimeDB provides deterministic RNG via `ctx.rng()`.

**What runs server-side:**
- ✅ CA simulation (falling sand, fire, reactions) using `ctx.rng()`
- ✅ Creature AI (neural network inference only)
- ✅ WorldRng trait abstraction (works with both `thread_rng()` and `ctx.rng()`)
- ❌ Evolution/training (offline only)
- ❌ Regeneration system (offline only)

**Key tables:** `world_config`, `chunk_data`, `player`, `creature_data`, tick timers

### Rand Compatibility

**IMPORTANT:** Always use **rand 0.8** and **stable APIs** to avoid version conflicts:

```rust
use rand::{Rng, thread_rng};  // Always import Rng trait

let mut rng = thread_rng();   // Not rand::rng() (nightly only)
rng.gen_range(0..10);         // Not .random_range() (rand 0.9)
rng.r#gen::<f32>();           // Use r#gen in Rust 2024 ('gen' is keyword)
```

**WorldRng abstraction** (in `sunaba-core/src/world/world.rs`) allows World to work with any RNG source:
```rust
pub trait WorldRng {
    fn gen_bool(&mut self) -> bool;
    fn gen_f32(&mut self) -> f32;
    fn check_probability(&mut self, probability: f32) -> bool;
}
impl<T: ?Sized + rand::Rng> WorldRng for T { ... }  // Blanket impl
```

This supports `thread_rng()` (client), `ctx.rng()` (server), and `DeterministicRng` (genome→brain init).

### Development Workflow for Schema Changes

**CRITICAL:** When modifying the SpacetimeDB server schema, follow this checklist to prevent breaking clients.

#### Schema Change Checklist

When you modify `crates/sunaba-server/src/` (add/remove/modify tables in tables.rs or reducers in reducers/):

1. **Build the server module:**
   ```bash
   just spacetime-build
   ```

2. **Regenerate client bindings (auto-generated, type-safe, gitignored):**
   ```bash
   just spacetime-generate-rust  # Native Rust client
   just spacetime-generate-ts    # WASM TypeScript client
   ```
   - Updates `crates/sunaba/src/multiplayer/generated/` (Rust) - gitignored
   - Updates `web/src/spacetime/` (TypeScript) - gitignored
   - Both are fully auto-generated from server schema
   - **Note:** Generated clients are gitignored and regenerated on every build

3. **Run full test suite:**
   ```bash
   just test  # Includes both Rust and TypeScript verification
   ```
   - Verifies Rust client matches schema
   - Verifies TypeScript client matches schema
   - Runs `tsc --noEmit` to type-check TypeScript

4. **Test locally with server:**
   ```bash
   just spacetime-start
   just spacetime-publish
   # Test both native and WASM builds
   ```

#### Quick Commands

| Command                         | Purpose                                              |
|---------------------------------|------------------------------------------------------|
| `just spacetime-build`          | Build WASM server module                             |
| `just spacetime-generate-rust`  | Regenerate Rust client from server                   |
| `just spacetime-generate-ts`    | Regenerate TypeScript client from server             |
| `just spacetime-verify-clients` | Verify Rust client matches server schema             |
| `just spacetime-verify-ts`      | Verify TypeScript client is regenerated              |
| `just test`                     | Full validation (includes both client verifications) |

#### Common Pitfalls

❌ **DON'T:** Edit generated files (they're gitignored and auto-regenerated)
❌ **DON'T:** Commit generated client code (`generated/` and `src/spacetime/` are gitignored)
❌ **DON'T:** Forget to run `just spacetime-build` after modifying server schema
✅ **DO:** Run `just test` after schema changes (auto-regenerates clients)
✅ **DO:** Keep reducer signatures simple (avoid complex types)
✅ **DO:** Test both native and WASM builds after schema changes
✅ **DO:** Let generated clients handle all type safety automatically

### SpacetimeDB Subscription Best Practices

**CRITICAL:** SpacetimeDB subscriptions have specific SQL limitations and performance characteristics that require careful query design.

#### SQL Limitations

SpacetimeDB's SQL WHERE clauses have limited functionality:

- ❌ **No subqueries**: Cannot use `WHERE x = (SELECT chunk_x FROM player ...)`
- ❌ **No arithmetic in WHERE**: Cannot use `ABS(x - player.x) <= 10`
- ❌ **No functions in WHERE**: Cannot use `ABS()`, `SQRT()`, `POW()`, etc.
- ✅ **Use BETWEEN for ranges**: `WHERE x BETWEEN -10 AND 10 AND y BETWEEN -10 AND 10`
- ✅ **Basic comparisons only**: `=`, `<`, `>`, `<=`, `>=`, `!=`, `<>`

**Example:**
```rust
// ❌ WRONG - Uses ABS() function
subscribe("SELECT * FROM chunk_data WHERE ABS(x) <= 10 AND ABS(y) <= 10");

// ✅ CORRECT - Uses BETWEEN
subscribe("SELECT * FROM chunk_data WHERE x BETWEEN -10 AND 10 AND y BETWEEN -10 AND 10");

// ❌ WRONG - No subqueries or arithmetic
subscribe("SELECT * FROM chunk_data WHERE x = (SELECT chunk_x FROM player WHERE id = me)");

// ✅ CORRECT - Client-side filtering or re-subscription
let center = get_player_chunk_pos();
subscribe(&format!(
    "SELECT * FROM chunk_data WHERE x BETWEEN {} AND {} AND y BETWEEN {} AND {}",
    center.x - 10, center.x + 10, center.y - 10, center.y + 10
));
```

#### Subscription Management

**Zero-copy subscriptions:** Same query subscribed multiple times has no overhead (SpacetimeDB deduplicates internally)

**Overlapping queries have overhead:** Different queries with overlapping data cause server to process/serialize rows multiple times

**Update pattern:** Always unsubscribe → subscribe to minimize overlap

```rust
// ✅ CORRECT - Minimize overlap by unsubscribing first
if let Some(old_sub) = self.chunk_subscription.take() {
    old_sub.unsubscribe();  // Unsubscribe first
}
let new_sub = conn.subscribe("SELECT * FROM ..."); // Then subscribe
self.chunk_subscription = Some(new_sub);

// ❌ WRONG - Subscribe before unsubscribe causes overlap
let new_sub = conn.subscribe("SELECT * FROM ...");
if let Some(old_sub) = self.chunk_subscription.take() {
    old_sub.unsubscribe();  // Too late - overlap already happened
}
```

**Brief gaps are OK:** Chunks remain in client world during re-subscription, only subscription cache updates

#### Dynamic Filtering Workarounds

Since SpacetimeDB doesn't support dynamic WHERE clauses based on other table values:

1. **Client-side filtering**: Subscribe to large area, filter progressively on client
2. **Re-subscription**: Periodically re-subscribe with new center when player moves
3. **Rate limiting**: Control sync rate to avoid frame drops (2-3 items per frame)

**Example progressive loading pattern:**
```rust
// 1. Initial subscription: small radius for fast spawn
subscribe("SELECT * FROM chunk_data WHERE x BETWEEN -3 AND 3 AND y BETWEEN -3 AND 3");

// 2. After spawn loads: expand to larger radius
unsubscribe_old();
subscribe("SELECT * FROM chunk_data WHERE x BETWEEN -10 AND 10 AND y BETWEEN -10 AND 10");

// 3. When player moves >8 chunks: re-subscribe with new center
let new_center = player_chunk_pos;
subscribe(&format!(
    "SELECT * FROM chunk_data WHERE x BETWEEN {} AND {} AND y BETWEEN {} AND {}",
    new_center.x - 10, new_center.x + 10,
    new_center.y - 10, new_center.y + 10
));
```

#### Performance Tips

- **Start small**: Begin with small subscription for fast initial load (e.g., 3-chunk radius = 49 chunks)
- **Expand progressively**: Expand to larger subscription after critical data loaded (e.g., 10-chunk radius = 441 chunks)
- **Re-subscribe on movement**: Re-subscribe when player moves far from subscription center (>8 chunks for 10-radius subscription)
- **Use eviction**: Keep memory usage bounded by unloading distant chunks
- **Rate-limit client sync**: Process 2-3 chunks per frame to avoid frame drops

**Example implementation:**
```rust
// Fast initial load: 49 chunks (7x7 grid) loads in <1 second
subscribe("WHERE x BETWEEN -3 AND 3 AND y BETWEEN -3 AND 3");

// Progressive expansion: Stream remaining 392 chunks in background
// Use spiral iterator + ChunkLoadQueue for rate-limited loading

// Dynamic re-subscription: Update center as player explores
if player_moved_far {
    resubscribe_chunks(new_center, radius);  // Unsubscribe → subscribe pattern
}

// Memory management: Evict chunks >10 from player
world.evict_distant_chunks(player_pos);
```

## Workspace Structure

sunaba is organized as a Cargo workspace with 5 crates:

| Crate | Purpose | Key Dependencies |
|-------|---------|------------------|
| `sunaba-simulation` | Material definitions, reactions, pixel data | serde, log |
| `sunaba-creature` | ML-evolved creatures, simple physics, neural control | sunaba-simulation, petgraph, rand |
| `sunaba-core` | World, entity, levels (re-exports simulation + creature) | sunaba-simulation, sunaba-creature, noise |
| `sunaba` | Main binary, rendering, UI, headless training | wgpu, egui, winit, sunaba-core |
| `sunaba-server` | SpacetimeDB multiplayer server module | spacetimedb, sunaba-simulation, sunaba-creature |

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
| Component     | Crate                                         |
|---------------|-----------------------------------------------|
| Graphics      | wgpu 27.0                                     |
| Windowing     | winit 0.30                                    |
| UI            | egui 0.33                                     |
| Physics       | Simple kinematic (no external physics engine) |
| Math          | glam 0.25                                     |
| Serialization | serde + bincode + ron                         |
| Compression   | lz4_flex                                      |
| RNG           | rand + rand_xoshiro (deterministic)           |
| Neural/Graph  | petgraph 0.6                                  |
| Profiling     | puffin + puffin_egui (opt-in feature)         |

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

```
# Movement
A/D            : Move left/right
W              : Fly/Levitate (Noita-style)
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
F3             : Puffin profiler (requires --features profiling)
M              : Multiplayer panel (connection UI, server selection, stats)
T              : Temperature overlay
```

When adding new controls, update the above list in addition to the controls help in web/index.html

## Notes for Claude

1. **Start simple**: Get basic functionality working before adding complexity
2. **Profile early**: The CA loop is the hot path, measure before optimizing
3. **Data-driven materials**: Resist hardcoding material behaviors
4. **Chunk boundaries**: Most bugs occur at chunk edges - test thoroughly
5. **Rand compatibility**: Always use rand 0.8 stable APIs (`thread_rng()`, `gen_range()`, `r#gen()`). Import `Rng` trait. WorldRng abstraction handles SpacetimeDB `ctx.rng()` and client `thread_rng()`.
6. **SpacetimeDB schema changes**: ALWAYS run `just spacetime-generate-rust` AND `just spacetime-generate-ts` after modifying server schema. Both clients are auto-generated and type-safe. Run `just test` to validate both clients.
7. **Data-driven creatures**: Behaviors should emerge from evolution, not code
8. **Neural inference profiling**: Brain updates are hot path for many creatures
9. **Deterministic evolution**: Seeded RNG for reproducible training runs
10. **Behavioral diversity**: MAP-Elites should produce genuinely different strategies
11. **Morphology-controller coupling**: CPPN and brain genome should co-evolve together
12. **Multiplayer client sync**: Both Rust (native) and TypeScript (WASM) clients auto-generate from server schema. CI validates both via `just spacetime-verify-clients` and `just spacetime-verify-ts`.
13. **Multiplayer runtime switching**: Game defaults to singleplayer. Press M to open connection panel, select server, and connect/disconnect at runtime. Singleplayer world is saved before connecting and restored on disconnect. Use `--server <url>` CLI arg or `just join`/`just join-prod` to connect on startup. Server metrics sampled at 6fps, ping at 1Hz, retention: 3600 samples (10 minutes).
