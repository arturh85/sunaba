# Migration: Remove Rapier2d and Add SpacetimeDB Support

## Overview

This migration removes the rapier2d physics engine dependency and adds SpacetimeDB multiplayer support with **zero code duplication**. The simulation code is shared between native (game + training) and SpacetimeDB (multiplayer server).

### Architecture After Migration

```
┌─────────────────────────────────────────────────────────┐
│                   SHARED CODE (compiles to native + WASM)│
│  ┌─────────────────┐  ┌─────────────────┐              │
│  │sunaba-simulation│  │ sunaba-creature │              │
│  │  (CA, materials)│  │ (creatures, AI) │              │
│  └────────┬────────┘  └────────┬────────┘              │
│           └──────────┬─────────┘                        │
│              ┌───────┴───────┐                          │
│              │  sunaba-core  │                          │
│              │ (world, chunks│                          │
│              │ falling_chunks)│                          │
│              └───────┬───────┘                          │
└──────────────────────┼──────────────────────────────────┘
                       │
         ┌─────────────┼─────────────┐
         │             │             │
         ▼             ▼             ▼
   ┌──────────┐  ┌──────────┐  ┌──────────────┐
   │  sunaba  │  │  sunaba  │  │sunaba-server │
   │ (native) │  │  (WASM)  │  │(SpacetimeDB) │
   │ game +   │  │  browser │  │  thin wrapper│
   │ training │  │          │  │              │
   └──────────┘  └──────────┘  └──────────────┘
```

### What Changes

| Component | Before | After |
|-----------|--------|-------|
| Debris physics | rapier2d rigid bodies | Simple kinematic falling chunks |
| Creature physics | rapier2d kinematic | Direct position/angle math |
| Training | Native binary | Native binary (unchanged) |
| Local play | Native binary | Native binary (unchanged) |
| Multiplayer | N/A | SpacetimeDB server using shared code |

### What Stays the Same

- **Training speed**: Uses native binary, no SpacetimeDB involved
- **Local development**: `just start`, `just test` work exactly as before
- **CA simulation**: Identical algorithm, just WASM-compatible now
- **Web build**: Still works via `just web`

---

## Phase 1: Create Kinematic Chunk System (Replace Debris Physics)

This replaces rapier2d rigid body debris with simple gravity-based falling chunks.

### 1.1 Create `crates/sunaba-core/src/simulation/falling_chunks.rs`

```rust
//! Kinematic falling chunks - simple physics without rapier2d
//!
//! Large debris falls as a unit with gravity, no rotation.
//! When it hits ground, it settles back into static pixels.
//!
//! This is WASM-compatible and used by both native game and SpacetimeDB server.

use glam::{IVec2, Vec2};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A chunk of pixels falling as a unit
#[derive(Clone, Serialize, Deserialize)]
pub struct FallingChunk {
    /// Pixels relative to center, with their material IDs
    pub pixels: HashMap<IVec2, u16>,
    /// Center position in world space (floating point for smooth movement)
    pub center: Vec2,
    /// Vertical velocity (pixels per second, negative = falling)
    pub velocity_y: f32,
    /// Unique ID for tracking
    pub id: u64,
}

/// Render data for a falling chunk (used by renderer)
#[derive(Clone)]
pub struct ChunkRenderData {
    pub center: Vec2,
    pub pixels: HashMap<IVec2, u16>,
}

/// Manages all falling chunks
#[derive(Clone, Default, Serialize, Deserialize)]
pub struct FallingChunkSystem {
    chunks: Vec<FallingChunk>,
    next_id: u64,
}

/// Trait for world collision queries (implemented by World)
pub trait WorldCollisionQuery {
    fn is_solid_at(&self, x: i32, y: i32) -> bool;
}

impl FallingChunkSystem {
    pub fn new() -> Self {
        Self {
            chunks: Vec::new(),
            next_id: 0,
        }
    }

    /// Create a new falling chunk from a set of world positions with materials
    /// Returns the chunk ID
    pub fn create_chunk(&mut self, pixels: HashMap<IVec2, u16>) -> u64 {
        if pixels.is_empty() {
            return 0;
        }

        // Calculate center of mass
        let center = Self::calculate_center(&pixels);

        // Convert to relative positions
        let center_i = IVec2::new(center.x.round() as i32, center.y.round() as i32);
        let relative_pixels: HashMap<IVec2, u16> = pixels
            .into_iter()
            .map(|(pos, mat)| (pos - center_i, mat))
            .collect();

        let id = self.next_id;
        self.next_id += 1;

        log::info!(
            "FallingChunks: Created chunk {} with {} pixels at ({:.1}, {:.1})",
            id,
            relative_pixels.len(),
            center.x,
            center.y
        );

        self.chunks.push(FallingChunk {
            pixels: relative_pixels,
            center,
            velocity_y: 0.0,
            id,
        });

        id
    }

    /// Update all chunks with gravity, returns list of chunks that have settled
    pub fn update<W: WorldCollisionQuery>(&mut self, dt: f32, world: &W) -> Vec<FallingChunk> {
        const GRAVITY: f32 = -300.0; // pixels/s^2 (negative = down)
        const TERMINAL_VELOCITY: f32 = -500.0;
        const SETTLE_VELOCITY: f32 = -5.0; // Velocity threshold to consider settled

        let mut settled = Vec::new();
        let mut i = 0;

        while i < self.chunks.len() {
            let chunk = &mut self.chunks[i];

            // Apply gravity
            chunk.velocity_y = (chunk.velocity_y + GRAVITY * dt).max(TERMINAL_VELOCITY);

            // Calculate desired movement
            let delta_y = chunk.velocity_y * dt;

            // Check if we can move down
            if delta_y < 0.0 {
                let steps = (-delta_y).ceil() as i32;
                let mut moved = 0;

                for _ in 0..steps {
                    if Self::can_move_chunk(chunk, 0, -1, world) {
                        chunk.center.y -= 1.0;
                        moved += 1;
                    } else {
                        // Hit something - stop vertical velocity
                        chunk.velocity_y = 0.0;
                        break;
                    }
                }

                // If we couldn't move at all and velocity is low, settle
                if moved == 0 && chunk.velocity_y.abs() < SETTLE_VELOCITY.abs() {
                    log::info!(
                        "FallingChunks: Chunk {} settled at ({:.1}, {:.1})",
                        chunk.id,
                        chunk.center.x,
                        chunk.center.y
                    );
                    settled.push(self.chunks.remove(i));
                    continue;
                }
            }

            i += 1;
        }

        settled
    }

    /// Get all chunks for rendering
    pub fn get_render_data(&self) -> Vec<ChunkRenderData> {
        self.chunks
            .iter()
            .map(|c| ChunkRenderData {
                center: c.center,
                pixels: c.pixels.clone(),
            })
            .collect()
    }

    /// Get number of active falling chunks
    pub fn chunk_count(&self) -> usize {
        self.chunks.len()
    }

    /// Check if a chunk can move by (dx, dy) without collision
    fn can_move_chunk<W: WorldCollisionQuery>(
        chunk: &FallingChunk,
        dx: i32,
        dy: i32,
        world: &W,
    ) -> bool {
        let center_i = IVec2::new(chunk.center.x.round() as i32, chunk.center.y.round() as i32);

        for relative_pos in chunk.pixels.keys() {
            let new_world_pos = center_i + *relative_pos + IVec2::new(dx, dy);
            if world.is_solid_at(new_world_pos.x, new_world_pos.y) {
                return false;
            }
        }
        true
    }

    /// Calculate center of mass from pixel positions
    fn calculate_center(pixels: &HashMap<IVec2, u16>) -> Vec2 {
        if pixels.is_empty() {
            return Vec2::ZERO;
        }
        let sum: Vec2 = pixels
            .keys()
            .map(|p| Vec2::new(p.x as f32, p.y as f32))
            .sum();
        sum / pixels.len() as f32
    }
}
```

### 1.2 Update `crates/sunaba-core/src/simulation/mod.rs`

Add the new module to the exports:

```rust
mod falling_chunks;
pub use falling_chunks::{ChunkRenderData, FallingChunk, FallingChunkSystem, WorldCollisionQuery};
```

### 1.3 Verification checkpoint

```bash
cargo check -p sunaba-core 2>&1 | head -20
# Should compile (falling_chunks is standalone)
```

---

## Phase 2: Integrate FallingChunkSystem into World

### 2.1 Update `crates/sunaba-core/src/world/world.rs`

**Step 2.1.1**: Add imports at the top:

```rust
use crate::simulation::{FallingChunkSystem, ChunkRenderData, WorldCollisionQuery};
```

**Step 2.1.2**: Replace `physics_world` field in `World` struct:

```rust
// REMOVE:
// physics_world: PhysicsWorld,

// ADD:
falling_chunks: FallingChunkSystem,
```

**Step 2.1.3**: Update `World::new()` and any constructors:

```rust
// REMOVE:
// physics_world: PhysicsWorld::new(),

// ADD:
falling_chunks: FallingChunkSystem::new(),
```

**Step 2.1.4**: Implement `WorldCollisionQuery` for World:

```rust
impl WorldCollisionQuery for World {
    fn is_solid_at(&self, x: i32, y: i32) -> bool {
        match self.get_pixel(x, y) {
            Some(pixel) if !pixel.is_empty() => {
                let material = self.materials().get(pixel.material_id);
                material.material_type == MaterialType::Solid
            }
            Some(_) => false, // Empty pixel
            None => true,     // Out of bounds = solid (prevents falling forever)
        }
    }
}
```

**Step 2.1.5**: Update the `update()` method - replace physics step with falling chunks:

Find the section that does:
```rust
// 7. Update rigid body physics
self.physics_world.step();

// 8. Check for settled debris and reconstruct as pixels
let settled = self.physics_world.get_settled_debris();
for handle in settled {
    self.reconstruct_debris(handle);
}
```

Replace with:
```rust
// 7. Update falling chunks (simple kinematic physics)
let settled_chunks = self.falling_chunks.update(dt, self);

// 8. Reconstruct settled chunks as static pixels
for chunk in settled_chunks {
    self.reconstruct_falling_chunk(chunk);
}
```

**Step 2.1.6**: Add new methods to World:

```rust
/// Create a falling chunk from pixel positions (called by structural integrity)
pub fn create_falling_chunk(&mut self, pixels: std::collections::HashMap<glam::IVec2, u16>) -> u64 {
    self.falling_chunks.create_chunk(pixels)
}

/// Reconstruct a settled falling chunk back into static pixels
fn reconstruct_falling_chunk(&mut self, chunk: crate::simulation::FallingChunk) {
    let center_i = glam::IVec2::new(
        chunk.center.x.round() as i32,
        chunk.center.y.round() as i32,
    );

    log::info!(
        "Reconstructing chunk {} ({} pixels) at ({}, {})",
        chunk.id,
        chunk.pixels.len(),
        center_i.x,
        center_i.y
    );

    let mut placed = 0;
    for (relative_pos, material_id) in chunk.pixels {
        let world_pos = center_i + relative_pos;

        // Only place if target position is empty (air)
        if let Some(existing) = self.get_pixel(world_pos.x, world_pos.y) {
            if existing.is_empty() {
                self.set_pixel_internal(world_pos.x, world_pos.y, material_id);
                placed += 1;
            }
        }
    }

    log::debug!("Placed {}/{} pixels from chunk", placed, chunk.pixels.len());
}

/// Get falling chunks for rendering
pub fn get_falling_chunks(&self) -> Vec<crate::simulation::ChunkRenderData> {
    self.falling_chunks.get_render_data()
}

/// Get count of active falling chunks (for debug stats)
pub fn falling_chunk_count(&self) -> usize {
    self.falling_chunks.chunk_count()
}
```

**Step 2.1.7**: Remove old methods:

Delete or comment out:
- `reconstruct_debris()`
- `get_active_debris()`
- Any method referencing `physics_world` for debris

**Step 2.1.8**: Update `create_debris()` method signature:

Find the `create_debris` method and update it to use the new system:

```rust
/// Create falling debris from a pixel region (called by structural integrity)
pub fn create_debris(&mut self, region: std::collections::HashSet<glam::IVec2>) -> u64 {
    log::info!("Creating falling chunk from {} pixels", region.len());

    // Collect pixel materials before removing from world
    let pixels: std::collections::HashMap<glam::IVec2, u16> = region
        .iter()
        .filter_map(|pos| {
            self.get_pixel(pos.x, pos.y)
                .filter(|p| !p.is_empty())
                .map(|p| (*pos, p.material_id))
        })
        .collect();

    if pixels.is_empty() {
        log::warn!("No valid pixels to create falling chunk");
        return 0;
    }

    // Remove pixels from world
    for pos in &region {
        self.set_pixel_internal(pos.x, pos.y, MaterialId::AIR);
    }

    // Create falling chunk
    self.create_falling_chunk(pixels)
}
```

### 2.2 Verification checkpoint

```bash
cargo check -p sunaba-core 2>&1 | grep -E "^error" | head -20
# Expect errors about PhysicsWorld in creature code - that's Phase 3
```

---

## Phase 3: Simplify Creature Physics

Replace rapier2d-based creature physics with simple position/angle tracking.

### 3.1 Create `crates/sunaba-creature/src/simple_physics.rs`

```rust
//! Simple creature physics - no external physics engine
//!
//! Creatures use kinematic position-based movement.
//! Body parts are positioned relative to root with angles.
//! This is WASM-compatible and runs identically in native and SpacetimeDB.

use glam::Vec2;
use serde::{Deserialize, Serialize};
use std::f32::consts::PI;

/// Simple physics state for a creature's body
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct CreaturePhysicsState {
    /// Position of each body part (world coords)
    pub part_positions: Vec<Vec2>,
    /// Rotation of each body part (radians)
    pub part_rotations: Vec<f32>,
    /// Angular velocity of each motor joint (radians/sec)
    pub motor_angular_velocities: Vec<f32>,
    /// Current angle of each motor joint (radians)
    pub motor_angles: Vec<f32>,
    /// Target angle of each motor joint (radians, set by neural network)
    pub motor_target_angles: Vec<f32>,
    /// Which body part index each motor controls
    pub motor_part_indices: Vec<usize>,
}

impl CreaturePhysicsState {
    /// Create new physics state for a creature
    pub fn new(num_parts: usize, num_motors: usize) -> Self {
        Self {
            part_positions: vec![Vec2::ZERO; num_parts],
            part_rotations: vec![0.0; num_parts],
            motor_angular_velocities: vec![0.0; num_motors],
            motor_angles: vec![0.0; num_motors],
            motor_target_angles: vec![0.0; num_motors],
            motor_part_indices: Vec::with_capacity(num_motors),
        }
    }

    /// Update motor angles based on target angles (spring-damper model)
    pub fn update_motors(&mut self, dt: f32, motor_strength: f32) {
        const DAMPING: f32 = 0.85;
        const MAX_ANGULAR_VEL: f32 = 10.0; // rad/s

        for i in 0..self.motor_angles.len() {
            let target = self.motor_target_angles[i];
            let current = self.motor_angles[i];
            let diff = Self::angle_diff(target, current);

            // Spring force towards target
            let angular_accel = diff * motor_strength;
            self.motor_angular_velocities[i] += angular_accel * dt;
            self.motor_angular_velocities[i] *= DAMPING;
            self.motor_angular_velocities[i] = self.motor_angular_velocities[i]
                .clamp(-MAX_ANGULAR_VEL, MAX_ANGULAR_VEL);

            // Integrate velocity
            self.motor_angles[i] += self.motor_angular_velocities[i] * dt;

            // Clamp to valid range
            self.motor_angles[i] = self.motor_angles[i].clamp(-PI, PI);
        }
    }

    /// Apply motor commands from neural network output
    pub fn apply_motor_commands(&mut self, commands: &[f32]) {
        for (i, &cmd) in commands.iter().enumerate() {
            if i < self.motor_target_angles.len() {
                // Neural network outputs are typically -1 to 1, scale to angle range
                self.motor_target_angles[i] = cmd.clamp(-1.0, 1.0) * PI;
            }
        }
    }

    /// Get motor angle for a specific motor index
    pub fn get_motor_angle(&self, motor_idx: usize) -> Option<f32> {
        self.motor_angles.get(motor_idx).copied()
    }

    /// Get angular velocity for a specific motor
    pub fn get_motor_velocity(&self, motor_idx: usize) -> Option<f32> {
        self.motor_angular_velocities.get(motor_idx).copied()
    }

    /// Calculate shortest angle difference (handles wraparound)
    fn angle_diff(target: f32, current: f32) -> f32 {
        let mut diff = target - current;
        while diff > PI {
            diff -= 2.0 * PI;
        }
        while diff < -PI {
            diff += 2.0 * PI;
        }
        diff
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_motor_spring() {
        let mut state = CreaturePhysicsState::new(2, 1);
        state.motor_target_angles[0] = 1.0;

        // Simulate several steps
        for _ in 0..100 {
            state.update_motors(0.016, 10.0);
        }

        // Should approach target
        assert!((state.motor_angles[0] - 1.0).abs() < 0.1);
    }

    #[test]
    fn test_angle_diff_wraparound() {
        assert!((CreaturePhysicsState::angle_diff(PI, -PI)).abs() < 0.01);
        assert!((CreaturePhysicsState::angle_diff(-PI * 0.9, PI * 0.9) - (-0.2 * PI)).abs() < 0.01);
    }
}
```

### 3.2 Update `crates/sunaba-creature/src/lib.rs`

Add the new module and remove old physics:

```rust
// ADD:
mod simple_physics;
pub use simple_physics::CreaturePhysicsState;

// REMOVE or comment out:
// mod physics;
// pub use physics::PhysicsWorld;
```

### 3.3 Update `crates/sunaba-creature/src/morphology.rs`

**Step 3.3.1**: Remove rapier2d imports:

```rust
// REMOVE:
// use rapier2d::prelude::{MultibodyJointHandle, RigidBodyHandle};
```

**Step 3.3.2**: Remove `MorphologyPhysics` struct entirely (or keep a simplified version)

**Step 3.3.3**: Add method to create physics state from morphology:

```rust
use crate::simple_physics::CreaturePhysicsState;

impl Morphology {
    /// Create physics state for this morphology
    pub fn create_physics_state(&self, position: glam::Vec2) -> CreaturePhysicsState {
        let num_parts = self.parts.len();
        let motor_indices: Vec<usize> = self.parts
            .iter()
            .enumerate()
            .filter(|(_, p)| p.is_motor)
            .map(|(i, _)| i)
            .collect();
        let num_motors = motor_indices.len();

        let mut state = CreaturePhysicsState::new(num_parts, num_motors);
        state.motor_part_indices = motor_indices;

        // Initialize root position
        if !state.part_positions.is_empty() {
            state.part_positions[0] = position;
        }

        state
    }
}
```

### 3.4 Update `crates/sunaba-creature/src/creature.rs`

This requires significant changes. Key modifications:

**Step 3.4.1**: Replace physics field:

```rust
// REMOVE:
// pub physics: Option<MorphologyPhysics>,

// ADD:
pub physics_state: CreaturePhysicsState,
```

**Step 3.4.2**: Remove all `PhysicsWorld` parameters from methods:

- `update()` - remove `physics_world` parameter
- `run_neural_control()` - remove `physics_world` parameter
- `apply_physics_movement()` - remove `physics_world` parameter
- `get_body_positions()` - remove `physics_world` parameter, use `self.physics_state`

**Step 3.4.3**: Update creature initialization:

```rust
// In new() or spawn methods:
physics_state: morphology.create_physics_state(position),
```

**Step 3.4.4**: Update motor control to use simple physics:

```rust
fn apply_motor_commands(&mut self, delta_time: f32) {
    if let Some(ref commands) = self.pending_motor_commands.take() {
        self.physics_state.apply_motor_commands(commands);
    }
    self.physics_state.update_motors(delta_time, 10.0); // motor_strength = 10.0
}
```

**Step 3.4.5**: Update `get_body_positions()`:

```rust
pub fn get_body_positions(&self) -> Vec<(Vec2, f32)> {
    self.physics_state.part_positions
        .iter()
        .zip(self.physics_state.part_rotations.iter())
        .map(|(&pos, &rot)| (pos, rot))
        .collect()
}
```

### 3.5 Update `crates/sunaba-creature/src/neural.rs`

Remove rapier2d references:

```rust
// REMOVE parameter:
// physics_handles: Option<&[rapier2d::prelude::RigidBodyHandle]>,

// Use CreaturePhysicsState instead for motor velocities/angles
```

### 3.6 Update `crates/sunaba-creature/Cargo.toml`

Remove rapier2d:

```toml
[dependencies]
sunaba-simulation = { path = "../sunaba-simulation" }
ahash = "0.8"
glam = { version = "0.25", features = ["serde"] }
log = "0.4"
petgraph = "0.6"
rand = "0.9"
rand_xoshiro = "0.7"
# REMOVE: rapier2d = "0.18"
serde = { version = "1.0", features = ["derive"] }
```

### 3.7 Update `crates/sunaba-core/Cargo.toml`

Remove rapier2d:

```toml
[dependencies]
# ... other deps ...
# REMOVE: rapier2d = "0.18"
```

### 3.8 Delete old physics file

```bash
rm crates/sunaba-creature/src/physics.rs
```

### 3.9 Verification checkpoint

```bash
cargo check -p sunaba-creature
cargo check -p sunaba-core
cargo test -p sunaba-creature
```

---

## Phase 4: Update Main Crate (sunaba)

### 4.1 Update `crates/sunaba/src/render/renderer.rs`

Update debris/chunk rendering to use new API:

```rust
// REMOVE old debris rendering that uses physics transforms

// ADD new chunk rendering:
pub fn render_falling_chunks(&mut self, world: &World, /* other params */) {
    for chunk_data in world.get_falling_chunks() {
        let center_x = chunk_data.center.x.round() as i32;
        let center_y = chunk_data.center.y.round() as i32;

        for (relative_pos, material_id) in &chunk_data.pixels {
            let world_x = center_x + relative_pos.x;
            let world_y = center_y + relative_pos.y;

            // Use your existing pixel rendering logic
            // e.g., set pixel color based on material_id
        }
    }
}
```

### 4.2 Update creature spawning/management

Find code that passes `PhysicsWorld` to creatures and remove those parameters.

### 4.3 Update `crates/sunaba/Cargo.toml` if needed

Ensure no direct rapier2d dependency.

### 4.4 Verification checkpoint

```bash
cargo build -p sunaba --release
cargo test --workspace
just start  # Manual test: break structures, watch them fall
```

---

## Phase 5: Add SpacetimeDB Server Module

### 5.1 Create crate directory

```bash
mkdir -p crates/sunaba-server/src
```

### 5.2 Create `crates/sunaba-server/Cargo.toml`

```toml
[package]
name = "sunaba-server"
version.workspace = true
edition.workspace = true
description = "SpacetimeDB multiplayer server for Sunaba - thin wrapper around shared simulation code"

[lib]
crate-type = ["cdylib"]

[dependencies]
# Shared simulation code - same as native game uses
sunaba-simulation = { path = "../sunaba-simulation" }
sunaba-core = { path = "../sunaba-core", default-features = false }

# SpacetimeDB
spacetimedb = "1.0"

# Common deps
glam = { version = "0.25", features = ["serde"] }
log = "0.4"
rand = "0.9"
rand_xoshiro = "0.7"
serde = { version = "1.0", features = ["derive"] }
```

### 5.3 Create `crates/sunaba-server/src/lib.rs`

```rust
//! SpacetimeDB multiplayer server for Sunaba
//!
//! This is a THIN WRAPPER around sunaba-core. The actual simulation logic
//! is 100% shared with the native game - no duplication.
//!
//! SpacetimeDB compiles this to WASM and runs it as a database module.
//! Clients connect directly and subscribe to table changes.

use spacetimedb::{spacetimedb, Identity, ReducerContext, Table};

// Re-use the exact same simulation code as the native game
use sunaba_core::world::Chunk;
use sunaba_simulation::{MaterialId, Materials, CHUNK_SIZE};

// ============================================================================
// TABLES - These define the multiplayer state stored in SpacetimeDB
// ============================================================================

/// Connected player state
#[spacetimedb(table)]
#[derive(Clone)]
pub struct Player {
    #[primary_key]
    pub identity: Identity,
    pub name: String,
    pub position_x: f32,
    pub position_y: f32,
    pub health: f32,
    pub hunger: f32,
    pub online: bool,
}

/// Chunk data synchronized to clients
/// Only chunks near players are kept active
#[spacetimedb(table)]
#[derive(Clone)]
pub struct SyncedChunk {
    #[primary_key]
    #[autoinc]
    pub id: u64,

    /// Chunk coordinates
    pub chunk_x: i32,
    pub chunk_y: i32,

    /// Compressed pixel data (same format as native game saves)
    pub pixel_data: Vec<u8>,

    /// Whether this chunk needs CA simulation this tick
    pub simulation_active: bool,

    /// Last server tick this chunk was modified
    pub last_modified_tick: u64,
}

/// Incremental chunk updates (for bandwidth efficiency)
/// Clients apply these deltas instead of receiving full chunks
#[spacetimedb(table)]
#[derive(Clone)]
pub struct ChunkDelta {
    #[primary_key]
    #[autoinc]
    pub id: u64,

    pub chunk_x: i32,
    pub chunk_y: i32,
    pub tick: u64,

    /// RLE-encoded changes: Vec<(x, y, new_material_id)>
    pub changes: Vec<u8>,
}

/// Server state singleton
#[spacetimedb(table)]
#[derive(Clone)]
pub struct ServerState {
    #[primary_key]
    pub id: u32, // Always 0
    pub tick: u64,
    pub world_seed: u64,
}

// ============================================================================
// REDUCERS - Network entry points that clients can call
// ============================================================================

/// Called when a client connects
#[spacetimedb(reducer)]
pub fn on_connect(ctx: &ReducerContext) {
    log::info!("Client connected: {:?}", ctx.sender);
}

/// Called when a client disconnects
#[spacetimedb(reducer)]
pub fn on_disconnect(ctx: &ReducerContext) {
    log::info!("Client disconnected: {:?}", ctx.sender);

    // Mark player as offline (don't delete - preserve inventory)
    if let Some(mut player) = Player::filter_by_identity(&ctx.sender).next() {
        player.online = false;
        Player::update_by_identity(&ctx.sender, player);
    }
}

/// Player joins the game with a name
#[spacetimedb(reducer)]
pub fn join_game(ctx: &ReducerContext, name: String) {
    log::info!("Player {:?} joining as '{}'", ctx.sender, name);

    // Check if returning player
    if let Some(mut player) = Player::filter_by_identity(&ctx.sender).next() {
        player.online = true;
        player.name = name;
        Player::update_by_identity(&ctx.sender, player);
        return;
    }

    // New player
    Player::insert(Player {
        identity: ctx.sender,
        name,
        position_x: 0.0,
        position_y: 100.0, // Spawn above ground
        health: 100.0,
        hunger: 100.0,
        online: true,
    });
}

/// Player moves (client authoritative for now, can add validation later)
#[spacetimedb(reducer)]
pub fn update_position(ctx: &ReducerContext, x: f32, y: f32) {
    if let Some(mut player) = Player::filter_by_identity(&ctx.sender).next() {
        player.position_x = x;
        player.position_y = y;
        Player::update_by_identity(&ctx.sender, player);

        // TODO: Mark chunks around player as needing subscription
    }
}

/// Player places a pixel
#[spacetimedb(reducer)]
pub fn place_pixel(ctx: &ReducerContext, world_x: i32, world_y: i32, material_id: u16) {
    // Validate player exists and is nearby
    let player = match Player::filter_by_identity(&ctx.sender).next() {
        Some(p) => p,
        None => {
            log::warn!("place_pixel from unknown player {:?}", ctx.sender);
            return;
        }
    };

    // Distance check (anti-cheat)
    let dx = world_x as f32 - player.position_x;
    let dy = world_y as f32 - player.position_y;
    let dist = (dx * dx + dy * dy).sqrt();
    if dist > 100.0 {
        log::warn!("place_pixel too far from player: {} blocks", dist);
        return;
    }

    // TODO: Update chunk data
    // TODO: Mark chunk as simulation_active
    // TODO: Create ChunkDelta for other clients

    log::info!(
        "Player {} placed material {} at ({}, {})",
        player.name,
        material_id,
        world_x,
        world_y
    );
}

/// Player mines a pixel
#[spacetimedb(reducer)]
pub fn mine_pixel(ctx: &ReducerContext, world_x: i32, world_y: i32) {
    // Similar validation as place_pixel
    let player = match Player::filter_by_identity(&ctx.sender).next() {
        Some(p) => p,
        None => return,
    };

    // TODO: Validate tool, distance, etc.
    // TODO: Update chunk, add to inventory
    // TODO: Mark chunk as simulation_active

    log::info!(
        "Player {} mined at ({}, {})",
        player.name,
        world_x,
        world_y
    );
}

/// Server simulation tick - called by scheduled reducer or client trigger
#[spacetimedb(reducer)]
pub fn server_tick(_ctx: &ReducerContext) {
    // Get or create server state
    let state = ServerState::filter_by_id(&0).next().unwrap_or_else(|| {
        let s = ServerState {
            id: 0,
            tick: 0,
            world_seed: 12345,
        };
        ServerState::insert(s.clone());
        s
    });

    let current_tick = state.tick + 1;

    // Update all active chunks using SHARED simulation code
    for chunk in SyncedChunk::iter() {
        if chunk.simulation_active {
            // TODO: Deserialize chunk.pixel_data into sunaba_core::Chunk
            // TODO: Call the SAME CA update logic as native game
            // TODO: Serialize back and check if anything changed
            // TODO: If changed, create ChunkDelta and update chunk
            // TODO: If settled, set simulation_active = false
        }
    }

    // Update tick counter
    ServerState::update_by_id(
        &0,
        ServerState {
            id: 0,
            tick: current_tick,
            world_seed: state.world_seed,
        },
    );
}

// ============================================================================
// INITIALIZATION
// ============================================================================

#[spacetimedb(init)]
pub fn init() {
    log::info!("Sunaba multiplayer server initialized");

    ServerState::insert(ServerState {
        id: 0,
        tick: 0,
        world_seed: 12345,
    });
}
```

### 5.4 Update workspace `Cargo.toml`

Add the new crate to the workspace:

```toml
[workspace]
members = [
    "crates/sunaba-simulation",
    "crates/sunaba-creature",
    "crates/sunaba-core",
    "crates/sunaba",
    "crates/sunaba-server",
]
resolver = "2"
```

### 5.5 Verification checkpoint

```bash
cargo check -p sunaba-server
# Note: Full build requires spacetime CLI, check is enough for now
```

---

## Phase 6: Add Justfile Commands

### 6.1 Add SpacetimeDB commands to `justfile`

Append these commands:

```just
# ============================================================================
# SpacetimeDB Multiplayer Commands
# ============================================================================

# Install SpacetimeDB CLI (one-time setup)
[unix]
spacetime-install:
    @echo "Installing SpacetimeDB CLI..."
    curl -fsSL https://install.spacetimedb.com | bash
    @echo "Done! Restart your terminal or run: source ~/.bashrc"

[windows]
spacetime-install:
    @echo "Installing SpacetimeDB CLI..."
    powershell -Command "iwr https://install.spacetimedb.com/install.ps1 -useb | iex"
    @echo "Done! Restart your terminal."

# Build the SpacetimeDB server module
spacetime-build:
    @echo "Building SpacetimeDB module..."
    cd crates/sunaba-server && spacetime build --skip-clippy
    @echo "Build complete!"

# Start local SpacetimeDB instance (runs in background)
[unix]
spacetime-start:
    @echo "Starting local SpacetimeDB..."
    spacetime start &
    @sleep 2
    @echo "SpacetimeDB running at localhost:3000"

[windows]
spacetime-start:
    @echo "Starting local SpacetimeDB..."
    Start-Process -NoNewWindow spacetime -ArgumentList "start"
    Start-Sleep -Seconds 2
    @echo "SpacetimeDB running at localhost:3000"

# Stop local SpacetimeDB instance
spacetime-stop:
    spacetime stop

# Publish module to local SpacetimeDB instance
spacetime-publish-local: spacetime-build
    @echo "Publishing to local SpacetimeDB..."
    spacetime publish --skip-clippy sunaba-local crates/sunaba-server
    @echo "Published! Module name: sunaba-local"

# Publish module to SpacetimeDB cloud (requires: spacetime login)
spacetime-publish-cloud: spacetime-build
    @echo "Publishing to SpacetimeDB cloud..."
    spacetime publish --skip-clippy sunaba crates/sunaba-server
    @echo "Published to cloud!"

# View server logs (follow mode)
spacetime-logs:
    spacetime logs -f sunaba-local

# View server logs (last 100 lines)
spacetime-logs-tail:
    spacetime logs sunaba-local | tail -100

# Call server_tick reducer manually (for testing)
spacetime-tick:
    spacetime call sunaba-local server_tick

# Generate TypeScript client SDK
spacetime-generate-ts:
    @mkdir -p web/src/spacetime
    spacetime generate --lang typescript --out-dir web/src/spacetime crates/sunaba-server
    @echo "Generated TypeScript client in web/src/spacetime/"

# Generate Rust client SDK
spacetime-generate-rust:
    @mkdir -p crates/sunaba/src/multiplayer/generated
    spacetime generate --lang rust --out-dir crates/sunaba/src/multiplayer/generated crates/sunaba-server
    @echo "Generated Rust client in crates/sunaba/src/multiplayer/generated/"

# Full local development setup
spacetime-dev: spacetime-build spacetime-publish-local
    @echo ""
    @echo "=== SpacetimeDB local dev environment ready ==="
    @echo "Module: sunaba-local"
    @echo "Logs:   just spacetime-logs"
    @echo "Test:   just spacetime-tick"
    @echo ""

# Reset local database (delete all data)
spacetime-reset:
    @echo "Deleting local module..."
    spacetime delete sunaba-local || true
    @echo "Republishing..."
    just spacetime-publish-local

# Check SpacetimeDB CLI version
spacetime-version:
    spacetime version
```

---

## Phase 7: Final Cleanup and Testing

### 7.1 Search for remaining rapier2d references

```bash
# Should return nothing after cleanup
grep -r "rapier2d" crates/ --include="*.rs" --include="*.toml"
grep -r "RigidBody" crates/ --include="*.rs" | grep -v "// "
grep -r "PhysicsWorld" crates/ --include="*.rs" | grep -v "// "
```

### 7.2 Update Cargo.lock

```bash
cargo update
```

### 7.3 Run full test suite

```bash
just test
```

### 7.4 Manual testing checklist

```bash
# 1. Native game works
just start
# - [ ] Game starts without errors
# - [ ] Break a structure (place blocks, remove support)
# - [ ] Watch falling chunks fall and settle
# - [ ] Creatures spawn and move

# 2. Web build works (proves WASM compatibility)
just web
# - [ ] Builds without errors
# - [ ] Game runs in browser at localhost:8080

# 3. Training works
just train parcour 10 20
# - [ ] Training runs without errors
# - [ ] Creatures evolve (fitness improves)

# 4. SpacetimeDB module builds
just spacetime-build
# - [ ] Compiles to WASM without errors
```

---

## Summary

### Files Created
- `crates/sunaba-core/src/simulation/falling_chunks.rs`
- `crates/sunaba-creature/src/simple_physics.rs`
- `crates/sunaba-server/Cargo.toml`
- `crates/sunaba-server/src/lib.rs`

### Files Modified
- `crates/sunaba-core/src/simulation/mod.rs`
- `crates/sunaba-core/src/world/world.rs`
- `crates/sunaba-core/Cargo.toml`
- `crates/sunaba-creature/src/lib.rs`
- `crates/sunaba-creature/src/morphology.rs`
- `crates/sunaba-creature/src/creature.rs`
- `crates/sunaba-creature/src/neural.rs`
- `crates/sunaba-creature/Cargo.toml`
- `crates/sunaba/src/render/renderer.rs`
- `Cargo.toml` (workspace)
- `justfile`

### Files Deleted
- `crates/sunaba-creature/src/physics.rs`

### Key Principle

**Zero code duplication.** The simulation runs identically whether compiled for:
- Native (game + training)
- Browser WASM
- SpacetimeDB server WASM

The only SpacetimeDB-specific code is the thin wrapper in `sunaba-server` that defines tables and reducers.
