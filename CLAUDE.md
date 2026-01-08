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
just check [crate]     # After code changes: clippy + fmt + check (10-50s) - USE THIS FIRST
just test [crate]      # Before commit: check + fast unit tests (1-2min)
just test-scenarios    # Run slow scenario integration tests (10-15s)
just test-all [crate]  # Run unit tests + scenario tests (2-3min)
just test-ci           # Before push: full CI validation (10-12min)
```

> **Development Workflow**: Always run `just check` after making code changes. Run `just test` before committing (fast unit tests only). Run `just test-scenarios` or `just test-all` for comprehensive validation. Run `just test-ci` before pushing to ensure full CI passes.

**Development:**
```bash
just start   # Run with --regenerate (new world)
just load    # Run release (load existing)
just profile # Run with puffin profiler (F3 to toggle flamegraph)
just web     # Build and serve WASM (localhost:8080)
```

**Fast Iteration (Hot Reload):**
```bash
# Auto-rebuild on file changes (single bacon instance, switch modes with keys)
just watch         # Auto-compile (press 't' to compile tests, 'b' for release, 'q' to quit)

# INSTANT LAUNCH (~100ms, no Cargo overhead) - use with `just watch`
just run           # Execute release binary directly (truly instant!)
just run <args>    # Pass any arguments (e.g., just run --some-flag)

# Traditional build (1-2s Cargo overhead for file locks)
just start         # cargo run with --regenerate
just load          # cargo run (load existing world)
```

> **Instant Launch Workflow** ⭐ **RECOMMENDED**:
> 1. **Terminal 1**: Run `just watch` - auto-compiles on every save
>    - **Build mode (`b`)**: For manual testing - compiles release binary, then use `just run` to test
>    - **Test mode (`t`)**: For Claude working - compiles test binaries (no spam), Claude runs `just test` to verify
>    - Press `q` to quit
> 2. **Terminal 2**: Edit code in your editor (or Claude edits), bacon compiles automatically
> 3. **Terminal 3**: **`just run`** (manual testing) or **`just test`** (Claude verification)
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
# List available screenshots
just list-levels             # List all available demo levels with IDs
just list-ui-panels          # List all available UI panels

# Headless level screenshots (world/materials only, no UI)
just screenshot <level_id>   # Capture screenshot of level (1920x1080)
just screenshot 3 800 600    # Custom resolution (800x600)
just screenshot-all          # Capture all demo levels at once

# Headless UI panel screenshots (automated, GPU-rendered)
just screenshot-ui inventory         # Screenshot single panel with sample data
just screenshot-ui crafting 1280 720 # Custom resolution
just screenshot-ui-all               # Screenshot all panels at once

# Composite screenshots (world + UI panels, automated)
just screenshot-composite 3 inventory           # Level 3 + inventory panel
just screenshot-composite 5 inventory,crafting  # Level 5 + multiple panels (comma-separated)
```

> **Visual Iteration Workflows**:
>
> **1. World/Material Screenshots (Headless, CPU-rendered)**:
> - Use for: Testing physics, materials, level layouts
> - No UI rendered, only world pixels
> 1. Make changes to your code
> 2. `just screenshot <level_id>` to capture the result
> 3. Use the Read tool to view: `screenshots/level_<id>.png`
> 4. Iterate based on visual feedback
>
> **2. UI Panel Screenshots (Headless, GPU-rendered)** ⭐ **IMPLEMENTED**:
> - Use for: Testing individual UI panels with sample data
> - Automated, no game launch required
> - Panels rendered in dock-like side panel (400px wide, authentic look)
> - Available panels: `params`, `inventory`, `crafting`, `logger`, `worldgen`, `levels`, `multiplayer`
> - Sample data: Realistic inventory items, tools with durability, varied stats
> 1. `just screenshot-ui inventory` to capture panel
> 2. Use the Read tool to view: `screenshots/ui_inventory.png`
> 3. Panel shows realistic sample data (10+ materials, 3 tools with varying durability)
> 4. Iterate on UI changes quickly without launching the game
>
> **3. Composite Screenshots (World + UI Panels)** ⭐ **IMPLEMENTED**:
> - Use for: Testing UI panels with realistic world backgrounds
> - Automated, no game launch required
> - Combines CPU-rendered world (PixelRenderer) with GPU-rendered UI
> - Supports multiple panels (comma-separated)
> 1. `just screenshot-composite 3 inventory` to capture level 3 with inventory panel
> 2. Use the Read tool to view: `screenshots/composite_level3_inventory.png`
> 3. Panel shows sample data overlaid on actual game world
> 4. Iterate on UI positioning and world interaction
>
> **Future Enhancements (Planned)**:
> - ⏳ Full layout screenshots (multiple panels + HUD + overlays arranged as in-game)
>
> **3. Manual Full UI Screenshots** (for complex layouts):
> - Use for: Testing full UI layouts with multiple panels arranged
> - Requires launching the game manually
> - Can capture any arrangement of panels and overlays
> 1. Claude requests a UI screenshot: "Please capture a screenshot showing the HUD + inventory + crafting panels"
> 2. User launches the game: `just start` or `just load`
> 3. User opens the requested panels (see keybindings below)
> 4. User captures screenshot with OS tool (Shift+Cmd+5 on Mac, PrintScreen on Windows/Linux)
> 5. User saves to `screenshots/ui_custom.png`
> 6. Claude uses Read tool to view and provide feedback
>
> **UI Panel Keybindings** (for manual screenshots):
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

## Scenario Testing & Remote Control

Sunaba provides two approaches for testing and controlling the game:

### 1. Scenario Testing (Automated, Headless)

**Use for:**
- ✅ Automated regression testing
- ✅ Verifying game mechanics (mining, crafting, physics)
- ✅ Reproducible test cases for CI/CD
- ✅ Quick iteration on game logic without UI
- ✅ Testing that doesn't require visual inspection

**Quick Start:**
```bash
# Method 1: File-based scenario
just test-scenario scenarios/test_mining.ron

# Method 2: Inline scenario (best for Claude)
cargo run --features headless -- --test-scenario-stdin <<'EOF'
(
    name: "Quick Test",
    description: "Test player mechanics",

    setup: [
        (type: "TeleportPlayer", x: 0.0, y: 100.0),
    ],

    actions: [
        (type: "MineCircle", center_x: 0, center_y: 50, radius: 10),
        (type: "WaitFrames", frames: 60),
        (type: "CaptureScreenshot", filename: "test.png"),
    ],

    verify: [
        (type: "RegionEmpty", region: (type: "Circle", center_x: 0, center_y: 50, radius: 8)),
    ],
)
EOF

# Method 3: Run all scenarios (CLI tool)
just test-scenario-all

# Method 4: Run as integration tests (Rust test framework)
cargo test --test scenarios --features headless              # Fast smoke tests only (~2.5s)
cargo test --test scenarios --features headless -- --ignored # Comprehensive tests (~10s)
just test-scenarios                                          # Same as above (recommended)
```

**Test Integration (Hybrid Approach):**

Scenarios can be run both as CLI tools AND as Rust integration tests:

- **Fast smoke tests** run by default with `cargo test` (~2.5s per test)
  - `test_basic_scenario_execution()` - validates scenario system works
  - Runs alongside unit tests for fast feedback

- **Comprehensive tests** are marked `#[ignore]` and run with `--ignored` (~3-5s per scenario)
  - `test_mining_mechanics()` - validates mining.ron scenario
  - `test_all_scenario_files()` - runs ALL .ron files in scenarios/
  - Run with `just test-scenarios` for validation

**When to use each:**
- `just check` → Fast unit tests only (includes smoke test)
- `just test` → Fast unit tests (includes smoke test)
- `just test-scenarios` → Slow comprehensive scenario tests
- `just test-all` → Both unit tests + scenario tests
- `just test-scenario <file>` → Run specific scenario as CLI tool (for iteration)

**Available Actions:**

*High-level game commands:*
- `TeleportPlayer { x, y }` - Instant teleport
- `MovePlayerTo { x, y, timeout }` - Smooth movement (simulates input)
- `MineCircle { center_x, center_y, radius }` - Mine circular area
- `MineRect { min_x, min_y, max_x, max_y }` - Mine rectangular area
- `PlaceMaterial { x, y, material, radius }` - Place material (u16 material ID)
- `FillRect { min_x, min_y, max_x, max_y, material }` - Fill rectangle
- `GiveItem { item, slot }` - Add to inventory (ItemStack)
- `SetPlayerHealth { health }` - Set health directly
- `LoadLevel { level_id }` - Load demo level (0-20)

*Low-level input simulation:*
- `SimulateKey { key, frames }` - Press W/A/S/D/Space for N frames
- `SimulateMouseClick { world_x, world_y, button, frames }` - Click at world coords
- `SimulateMouseMove { world_x, world_y }` - Move mouse

*Control flow:*
- `WaitFrames { frames }` - Advance simulation (60 frames = 1 second)
- `WaitUntil { condition, timeout_frames }` - Wait for condition
- `CaptureScreenshot { filename, width, height }` - Save PNG
- `Log { message }` - Output message
- `Sequence { actions }` - Run nested actions

**Available Verifications:**

*Material checks:*
- `MaterialCount { material, region, expected, tolerance }` - Exact count ±tolerance
- `MaterialCountRange { material, region, min, max }` - Count in range
- `MaterialAt { x, y, expected }` - Material at specific pixel
- `RegionEmpty { region }` - All air in region
- `RegionFilled { region }` - No air in region

*Player state:*
- `PlayerPosition { x, y, tolerance }` - Position check
- `PlayerInRegion { region }` - Player within region
- `PlayerHealth { expected, tolerance }` - Health check
- `PlayerGrounded { expected }` - On ground check
- `InventorySlot { slot, expected }` - Specific slot contents

*Regions (for verifications):*
- `Rect { min_x, min_y, max_x, max_y }` - Rectangular region
- `Circle { center_x, center_y, radius }` - Circular region
- `Whole` - Entire loaded world
- `ActiveChunks` - Only active chunks

**Workflow for Claude:**

1. **Write scenario inline** (no file needed):
```bash
cargo run --features headless -- --test-scenario-stdin <<'EOF'
(
    name: "Mining Verification",
    description: "Verify mining removes materials",

    setup: [
        (type: "FillRect", min_x: -10, min_y: 0, max_x: 10, max_y: 50, material: 1),
    ],

    actions: [
        (type: "MineCircle", center_x: 0, center_y: 25, radius: 5),
        (type: "WaitFrames", frames: 60),
    ],

    verify: [
        (type: "RegionEmpty", region: (type: "Circle", center_x: 0, center_y: 25, radius: 4)),
    ],
)
EOF
```

2. **User executes** the heredoc command

3. **Claude reads results:**
   - JSON results: `scenario_results/<scenario_name>_result.json`
   - Screenshots: `screenshots/<filename>.png`

4. **Iterate** based on results

**Example Scenarios:**

*Test material placement:*
```ron
(
    name: "Material Placement",
    actions: [
        (type: "PlaceMaterial", x: 0, y: 50, material: 2, radius: 5),
        (type: "WaitFrames", frames: 60),
    ],
    verify: [
        (type: "MaterialCountRange", material: 2, region: (type: "Circle", center_x: 0, center_y: 50, radius: 5), min: 50, max: 100),
    ],
)
```

*Test player movement:*
```ron
(
    name: "Movement Test",
    setup: [(type: "TeleportPlayer", x: 0.0, y: 100.0)],
    actions: [
        (type: "SimulateKey", key: "d", frames: 120),
        (type: "WaitFrames", frames: 60),
    ],
    verify: [
        (type: "PlayerPosition", x: 300.0, y: 100.0, tolerance: 50.0),
    ],
)
```

### 2. Real-Time Remote Control (Live Game Control)

**Use for:**
- ✅ Controlling a running game instance via commands
- ✅ Live debugging and testing while watching the game
- ✅ Interactive exploration and experimentation
- ✅ Quick iteration on game mechanics
- ✅ Demonstrating features or reproducing bugs

**Quick Start:**
```bash
# Terminal 1: Start game with remote control enabled
cargo run --release --features headless -- --remote-control

# Terminal 2: Send commands via netcat
echo '(type: "TeleportPlayer", x: 100.0, y: 200.0)' | nc localhost 7453
echo '(type: "MineCircle", center_x: 50, center_y: 50, radius: 10)' | nc localhost 7453
echo '(type: "PlaceMaterial", x: 0, y: 0, material: 1, radius: 5)' | nc localhost 7453
```

**How it works:**
- Game opens TCP socket on `localhost:7453`
- Send RON commands (newline-terminated)
- Receive JSON responses with success/failure

**Example workflow (Claude controlling game):**
```bash
# 1. User starts game:
cargo run --release --features headless -- --remote-control

# 2. Claude sends commands:
echo '(type: "TeleportPlayer", x: 0.0, y: 100.0)' | nc localhost 7453
# Response: {"success":true,"message":"Teleported player to (0, 100)"}

# 3. Claude mines and watches result:
echo '(type: "MineCircle", center_x: 0, center_y: 50, radius: 15)' | nc localhost 7453
# Response: {"success":true,"message":"Mined circle at (0, 50) r=15"}
```

**Currently Supported Commands:**
- `TeleportPlayer { x, y }` - Instantly teleport player
- `MineCircle { center_x, center_y, radius }` - Mine circular area
- `PlaceMaterial { x, y, material, radius }` - Place material (u16 material ID)

**More commands coming soon** - this is a new feature! Additional ScenarioActions will be implemented as needed.

**Test script:**
```bash
# Quick test of all commands
./test_remote_control.sh
```

### 3. Manual Game Control (Interactive, Visual)

**Use for:**
- ✅ UI development and testing
- ✅ Visual debugging and inspection
- ✅ Manual gameplay testing
- ✅ Screenshot capture of UI panels
- ✅ Tasks requiring human judgment

**Start the game:**
```bash
just start      # New world (--regenerate)
just load       # Load existing world
just run        # Instant launch (use with `just watch` for hot reload)
```

**Request screenshots from user:**
When you need to see UI or visual state:

1. **Ask user to capture screenshot:**
   - "Please capture a screenshot of the Parameters panel (press P to open)"
   - "Please take a screenshot showing the crafting UI"

2. **User captures** (OS tool: Shift+Cmd+5 on Mac, PrintScreen on Windows)

3. **User saves** to `screenshots/ui_<panel_name>.png`

4. **Claude reads** screenshot with Read tool

**UI Panel Keybindings:**
- **P** - Parameters/settings
- **I** - Inventory
- **C** - Crafting
- **L** - Logger
- **Tab** - Dock (worldgen editor, level selector)
- **Esc** - Close panel

### When to Use Which Approach

| Task | Scenario Testing | Remote Control | Manual Control |
|------|-----------------|----------------|----------------|
| Verify mining removes materials | ✅ Automated, fast | ✅ Live feedback | ❌ Tedious |
| Test crafting recipe logic | ✅ Reproducible | ⚠️ Manual verify | ❌ Slow |
| Check physics simulation | ✅ Headless, quick | ✅ Watch live | ⚠️ Visual only |
| Debug UI layout issues | ❌ No UI | ❌ No UI | ✅ Visual inspection |
| Verify button placement | ❌ No rendering | ❌ No rendering | ✅ Screenshots |
| Test player movement speed | ✅ Precise data | ✅ Interactive | ⚠️ Visual estimate |
| Regression testing | ✅ CI/CD ready | ❌ Manual | ❌ Manual effort |
| Screenshot UI panels | ❌ Limitation | ❌ Limitation | ✅ Full rendering |
| Test 1000 iterations | ✅ Automated | ❌ Too slow | ❌ Impossible |
| Quick experimentation | ⚠️ Edit scenario | ✅ Instant | ⚠️ Slow setup |
| Reproduce specific bug | ✅ Exact repro | ✅ Interactive | ⚠️ Hard to reproduce |

**Best Practices:**

1. **Default to scenarios** for automated game logic testing (CI/CD, regression)
2. **Use remote control** for interactive debugging and quick experimentation
3. **Request manual screenshots** only for UI/visual work
4. **Combine approaches**:
   - Scenarios → precise automated tests
   - Remote control → live exploration and debugging
   - Manual → UI verification and screenshots
5. **Batch scenarios** with `just test-scenario-all` for comprehensive testing

**Example Combined Workflow:**

```bash
# 1. Claude writes automated test
cargo run --features headless -- --test-scenario-stdin <<'EOF'
(
    name: "Crafting System",
    setup: [
        (type: "GiveItem", item: (Material(material_id: 4, count: 10)), slot: Some(0)),
    ],
    actions: [
        (type: "WaitFrames", frames: 10),
        (type: "CaptureScreenshot", filename: "inventory_state.png"),
    ],
    verify: [
        (type: "InventorySlot", slot: 0, expected: Some((Material(material_id: 4, count: 10)))),
    ],
)
EOF

# 2. If verification passes but visual check needed:
# "Please launch the game and capture a screenshot of the inventory UI"
```

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
