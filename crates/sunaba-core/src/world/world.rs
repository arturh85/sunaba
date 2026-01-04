//! World - manages chunks and simulation

use glam::IVec2;
use std::collections::HashMap;

use super::ca_update::CellularAutomataUpdater;
use super::chemistry_system::ChemistrySystem;
use super::chunk_manager::ChunkManager;
use super::collision::CollisionDetector;
use super::debris_system::DebrisSystem;
use super::light_system::LightSystem;
use super::persistence_system::PersistenceSystem;
use super::{CHUNK_SIZE, Chunk, Pixel, pixel_flags};
use crate::entity::crafting::RecipeRegistry;
use crate::entity::player::Player;
use crate::entity::tools::ToolRegistry;
use crate::simulation::{
    ChunkRenderData, FallingChunk, MaterialId, MaterialType,
    Materials, ReactionRegistry, RegenerationSystem, StructuralIntegritySystem,
    TemperatureSimulator, WorldCollisionQuery, get_temperature_at_pixel,
    mining::calculate_mining_time,
};
use crate::world::NoopStats;

/// The game world, composed of chunks
pub struct World {
    /// Chunk lifecycle manager (loading, unloading, active tracking)
    chunk_manager: ChunkManager,

    /// Material definitions
    pub materials: Materials,

    /// Temperature simulation system
    temperature_sim: TemperatureSimulator,

    /// Chemical reaction registry
    reactions: ReactionRegistry,

    /// Tool registry
    tool_registry: ToolRegistry,

    /// Recipe registry
    pub recipe_registry: RecipeRegistry,

    /// Structural integrity checker
    structural_system: StructuralIntegritySystem,

    /// Light system (day/night cycle, light propagation, growth timer)
    light_system: LightSystem,

    /// Resource regeneration system
    regeneration_system: RegenerationSystem,

    /// Debris system (kinematic falling chunks, simple debris physics, WASM-compatible)
    debris_system: DebrisSystem,

    /// Creature manager (spawning, AI, behavior)
    pub creature_manager: crate::creature::spawning::CreatureManager,

    /// The player entity
    pub player: Player,

    /// Simulation time accumulator
    time_accumulator: f32,

    /// Persistence system (chunk loading, saving, world lifecycle)
    persistence_system: PersistenceSystem,
}

impl World {
    pub fn new() -> Self {
        let mut world = Self {
            chunk_manager: ChunkManager::new(),
            materials: Materials::new(),
            temperature_sim: TemperatureSimulator::new(),
            reactions: ReactionRegistry::new(),
            tool_registry: ToolRegistry::new(),
            recipe_registry: RecipeRegistry::new(),
            structural_system: StructuralIntegritySystem::new(),
            light_system: LightSystem::new(),
            regeneration_system: RegenerationSystem::new(),
            debris_system: DebrisSystem::new(),
            creature_manager: crate::creature::spawning::CreatureManager::new(200), // Max 200 creatures
            player: Player::new(glam::Vec2::new(0.0, 100.0)),
            time_accumulator: 0.0,
            persistence_system: PersistenceSystem::new(42), // Default seed
        };

        // Don't pre-generate - let chunks generate on-demand as player explores
        // (Demo levels still call generate_test_world() explicitly)

        // Initialize light levels before first CA update
        let active_chunks = world.chunk_manager.active_chunks.clone();
        world.light_system.initialize_light(
            &mut world.chunk_manager,
            &world.materials,
            &active_chunks,
        );

        // Spawn 3 test creatures near spawn point with spacing
        #[cfg(feature = "evolution")]
        {
            use crate::creature::genome::CreatureGenome;

            world
                .creature_manager
                .spawn_creature(CreatureGenome::test_biped(), glam::Vec2::new(-20.0, 100.0));

            world.creature_manager.spawn_creature(
                CreatureGenome::test_quadruped(),
                glam::Vec2::new(0.0, 100.0),
            );

            world
                .creature_manager
                .spawn_creature(CreatureGenome::test_worm(), glam::Vec2::new(20.0, 100.0));

            log::info!("Spawned 3 test creatures at startup");
        }

        world
    }

    /// Set the world generator (for terrain generation with custom seed)
    pub fn set_generator(&mut self, seed: u64) {
        self.persistence_system.set_seed(seed);
    }

    /// Generate a simple test world for development
    /// Kept for testing and debugging purposes
    #[allow(dead_code)]
    fn generate_test_world(&mut self) {
        let mut total_pixels = 0;

        // Create a 15x15 grid of chunks around origin (225 chunks, ~1.4MB)
        for cy in -7..=7 {
            for cx in -7..=7 {
                let mut chunk = Chunk::new(cx, cy);
                let mut chunk_pixels = 0;

                // BEDROCK LAYER: Full bedrock at bottom chunk (cy == -2)
                if cy == -2 {
                    for y in 0..CHUNK_SIZE {
                        for x in 0..CHUNK_SIZE {
                            chunk.set_material(x, y, MaterialId::BEDROCK);
                            chunk_pixels += 1;
                        }
                    }
                }
                // Bottom of cy == -1: bedrock layer (8 pixels deep)
                else if cy == -1 {
                    for y in 0..8 {
                        for x in 0..CHUNK_SIZE {
                            chunk.set_material(x, y, MaterialId::BEDROCK);
                            chunk_pixels += 1;
                        }
                    }
                    // Stone above bedrock
                    for y in 8..32 {
                        for x in 0..CHUNK_SIZE {
                            chunk.set_material(x, y, MaterialId::STONE);
                            chunk_pixels += 1;
                        }
                    }
                }
                // Original generation for cy >= 0
                else {
                    // Fill bottom half with stone
                    for y in 0..32 {
                        for x in 0..CHUNK_SIZE {
                            chunk.set_material(x, y, MaterialId::STONE);
                            chunk_pixels += 1;
                        }
                    }

                    // Add some sand on top
                    if cy == 0 {
                        for x in 20..44 {
                            for y in 32..40 {
                                chunk.set_material(x, y, MaterialId::SAND);
                                chunk_pixels += 1;
                            }
                        }
                    }

                    // Add some water
                    if cx == 1 && cy == 0 {
                        for x in 10..30 {
                            for y in 35..50 {
                                chunk.set_material(x, y, MaterialId::WATER);
                                chunk_pixels += 1;
                            }
                        }
                    }
                }

                total_pixels += chunk_pixels;
                self.chunk_manager.chunks.insert(IVec2::new(cx, cy), chunk);
                self.chunk_manager.active_chunks.push(IVec2::new(cx, cy));
            }
        }

        log::info!(
            "Generated test world: {} chunks, {} pixels",
            self.chunk_manager.chunks.len(),
            total_pixels
        );
        log::info!(
            "  World bounds: ({}, {}) to ({}, {})",
            -2 * CHUNK_SIZE as i32,
            -2 * CHUNK_SIZE as i32,
            3 * CHUNK_SIZE as i32 - 1,
            3 * CHUNK_SIZE as i32 - 1
        );
        log::info!("  Player starts at: {:?}", self.player.position);
    }

    /// Check if a rectangle collides with solid materials
    fn check_solid_collision(&self, x: f32, y: f32, width: f32, height: f32) -> bool {
        CollisionDetector::check_solid_collision(
            &self.chunk_manager.chunks,
            &self.materials,
            x,
            y,
            width,
            height,
        )
    }

    /// Check if player is standing on ground
    fn is_player_grounded(&self) -> bool {
        CollisionDetector::is_rect_grounded(
            &self.chunk_manager.chunks,
            &self.materials,
            self.player.position.x,
            self.player.position.y,
            crate::entity::player::Player::WIDTH,
            crate::entity::player::Player::HEIGHT,
        )
    }

    /// Check if a circle collides with solid materials
    /// Used for creature body part collision detection
    pub fn check_circle_collision(&self, x: f32, y: f32, radius: f32) -> bool {
        CollisionDetector::check_circle_collision(
            &self.chunk_manager.chunks,
            &self.materials,
            x,
            y,
            radius,
        )
    }

    /// Check if any body part in a list is grounded (touching solid below)
    /// positions: Vec of (center, radius) for each body part
    pub fn is_creature_grounded(&self, positions: &[(glam::Vec2, f32)]) -> bool {
        CollisionDetector::is_creature_grounded(
            &self.chunk_manager.chunks,
            &self.materials,
            positions,
        )
    }

    /// Get the first blocking pixel in a direction from a position
    /// Returns (x, y, material_id) of the blocking pixel, or None if path is clear
    pub fn get_blocking_pixel(
        &self,
        from: glam::Vec2,
        direction: glam::Vec2,
        radius: f32,
        max_distance: f32,
    ) -> Option<(i32, i32, u16)> {
        use crate::simulation::MaterialType;

        // Normalize direction
        let dir = direction.normalize_or_zero();
        if dir == glam::Vec2::ZERO {
            return None;
        }

        // Step along the direction, checking for solid pixels
        let step_size = 1.0;
        let mut distance = radius; // Start from edge of body

        while distance < max_distance {
            let check_pos = from + dir * distance;
            let px = check_pos.x as i32;
            let py = check_pos.y as i32;

            if let Some(pixel) = self.get_pixel(px, py)
                && !pixel.is_empty()
            {
                let material = self.materials.get(pixel.material_id);
                if material.material_type == MaterialType::Solid {
                    return Some((px, py, pixel.material_id));
                }
            }

            distance += step_size;
        }

        None
    }

    /// Player movement speed in pixels per second
    const PLAYER_SPEED: f32 = 200.0;

    /// Active chunk simulation radius (chunks from player)
    const ACTIVE_CHUNK_RADIUS: i32 = 3; // 7×7 grid = 49 chunks

    /// Update player position based on input with gravity and jump
    pub fn update_player(&mut self, input: &crate::entity::InputState, dt: f32) {
        use crate::entity::player::Player;

        // 1. Check if grounded
        self.player.grounded = self.is_player_grounded();

        // 2. Update coyote time (grace period for jumping after leaving ground)
        if self.player.grounded {
            self.player.coyote_time = Player::COYOTE_TIME;
        } else {
            self.player.coyote_time = (self.player.coyote_time - dt).max(0.0);
        }

        // 3. Update jump buffer (allows jump input slightly before landing)
        if input.jump_pressed {
            self.player.jump_buffer = Player::JUMP_BUFFER;
        } else {
            self.player.jump_buffer = (self.player.jump_buffer - dt).max(0.0);
        }

        // 4. Horizontal movement (A/D keys) with friction
        const PLAYER_DECELERATION: f32 = 800.0; // px/s² (friction when no input)

        let mut horizontal_input = 0.0;
        if input.a_pressed {
            horizontal_input -= 1.0;
        }
        if input.d_pressed {
            horizontal_input += 1.0;
        }

        if horizontal_input != 0.0 {
            // Apply movement input
            self.player.velocity.x = horizontal_input * Self::PLAYER_SPEED;
        } else if self.player.grounded {
            // Apply friction when grounded and no input
            let friction = PLAYER_DECELERATION * dt;
            if self.player.velocity.x.abs() < friction {
                self.player.velocity.x = 0.0;
            } else {
                self.player.velocity.x -= self.player.velocity.x.signum() * friction;
            }
        }
        // Note: No friction in air - preserve momentum for better jump control

        // 5. Vertical movement (gravity + jump + flight)
        if self.player.jump_buffer > 0.0 && self.player.coyote_time > 0.0 {
            // Jump!
            self.player.velocity.y = Player::JUMP_VELOCITY;
            self.player.jump_buffer = 0.0;
            self.player.coyote_time = 0.0;
            log::debug!("Player jumped!");
        } else if !self.player.grounded {
            // Apply flight thrust if W pressed (Noita-style levitation)
            if input.w_pressed {
                self.player.velocity.y += Player::FLIGHT_THRUST * dt;
            }
            // Apply gravity when airborne
            self.player.velocity.y -= Player::GRAVITY * dt;
            // Clamp to terminal velocity (both up and down)
            self.player.velocity.y = self
                .player
                .velocity
                .y
                .clamp(-Player::MAX_FALL_SPEED, Player::MAX_FALL_SPEED);
        } else {
            // Grounded and not jumping - reset vertical velocity
            self.player.velocity.y = 0.0;
        }

        // 6. Integrate velocity into position with collision
        let movement = self.player.velocity * dt;

        // Check collision separately for X and Y
        let new_x = self.player.position.x + movement.x;
        let new_y = self.player.position.y + movement.y;

        let can_move_x = !self.check_solid_collision(
            new_x,
            self.player.position.y,
            Player::WIDTH,
            Player::HEIGHT,
        );

        let can_move_y = !self.check_solid_collision(
            self.player.position.x,
            new_y,
            Player::WIDTH,
            Player::HEIGHT,
        );

        // Apply movement only on non-colliding axes
        let final_movement = glam::Vec2::new(
            if can_move_x { movement.x } else { 0.0 },
            if can_move_y { movement.y } else { 0.0 },
        );

        // Stop vertical velocity if hit ceiling/floor
        if !can_move_y {
            self.player.velocity.y = 0.0;
        }

        // 7. Automatic unstuck mechanic - nudge player out of tight spaces if completely stuck
        if !can_move_x
            && !can_move_y
            && (input.a_pressed || input.d_pressed || input.w_pressed || input.s_pressed)
        {
            // Try small position adjustments to unstuck player
            const UNSTUCK_OFFSET: f32 = 0.5;
            let unstuck_attempts = [
                (UNSTUCK_OFFSET, 0.0),
                (-UNSTUCK_OFFSET, 0.0),
                (0.0, UNSTUCK_OFFSET),
                (0.0, -UNSTUCK_OFFSET),
            ];

            for (dx, dy) in unstuck_attempts {
                let test_x = self.player.position.x + dx;
                let test_y = self.player.position.y + dy;
                if !self.check_solid_collision(test_x, test_y, Player::WIDTH, Player::HEIGHT) {
                    self.player.position.x = test_x;
                    self.player.position.y = test_y;
                    log::debug!("Player unstuck: nudged ({}, {})", dx, dy);
                    return; // Exit early after unstucking
                }
            }
        }

        if final_movement.length() > 0.0 {
            log::trace!(
                "Player: {:?} → {:?} (vel: {:?}, grounded: {})",
                self.player.position,
                self.player.position + final_movement,
                self.player.velocity,
                self.player.grounded
            );
        }

        self.player.move_by(final_movement);
    }

    /// Update active chunks: remove distant chunks and re-activate nearby loaded chunks
    fn update_active_chunks(&mut self) {
        let player_chunk_x = (self.player.position.x as i32).div_euclid(CHUNK_SIZE as i32);
        let player_chunk_y = (self.player.position.y as i32).div_euclid(CHUNK_SIZE as i32);

        // 1. Remove distant chunks from active list
        self.chunk_manager.active_chunks.retain(|pos| {
            let dist_x = (pos.x - player_chunk_x).abs();
            let dist_y = (pos.y - player_chunk_y).abs();
            dist_x <= Self::ACTIVE_CHUNK_RADIUS && dist_y <= Self::ACTIVE_CHUNK_RADIUS
        });

        // 2. Add nearby loaded chunks that aren't currently active
        // Only check chunks within active radius (7×7 grid = 49 chunks max)
        for cy in (player_chunk_y - Self::ACTIVE_CHUNK_RADIUS)
            ..=(player_chunk_y + Self::ACTIVE_CHUNK_RADIUS)
        {
            for cx in (player_chunk_x - Self::ACTIVE_CHUNK_RADIUS)
                ..=(player_chunk_x + Self::ACTIVE_CHUNK_RADIUS)
            {
                let pos = IVec2::new(cx, cy);

                // If chunk is loaded but not active, add it to active list
                if self.chunk_manager.chunks.contains_key(&pos)
                    && !self.chunk_manager.active_chunks.contains(&pos)
                {
                    self.chunk_manager.active_chunks.push(pos);
                    // Mark newly activated chunks for simulation so physics/chemistry runs
                    if let Some(chunk) = self.chunk_manager.chunks.get_mut(&pos) {
                        chunk.set_simulation_active(true);
                    }
                }
            }
        }
    }

    /// Get the tool registry
    pub fn tool_registry(&self) -> &ToolRegistry {
        &self.tool_registry
    }

    /// Brush radius for material spawning (1 = 3x3, 2 = 5x5)
    const BRUSH_RADIUS: i32 = 1;

    /// Spawn material at world coordinates with circular brush
    pub fn spawn_material(&mut self, world_x: i32, world_y: i32, material_id: u16) {
        let material_name = &self.materials.get(material_id).name;
        let (chunk_pos, _, _) = ChunkManager::world_to_chunk_coords(world_x, world_y);

        log::info!(
            "[SPAWN] Spawning {} at world ({}, {}) in chunk ({}, {})",
            material_name,
            world_x,
            world_y,
            chunk_pos.x,
            chunk_pos.y
        );

        let mut spawned = 0;
        for dy in -Self::BRUSH_RADIUS..=Self::BRUSH_RADIUS {
            for dx in -Self::BRUSH_RADIUS..=Self::BRUSH_RADIUS {
                // Circular brush
                if dx * dx + dy * dy <= Self::BRUSH_RADIUS * Self::BRUSH_RADIUS {
                    let x = world_x + dx;
                    let y = world_y + dy;
                    self.set_pixel(x, y, material_id);
                    spawned += 1;
                }
            }
        }
        log::debug!("[SPAWN] Spawned {} pixels total", spawned);

        // Log chunk dirty status after spawning
        if let Some(chunk) = self.chunk_manager.chunks.get(&chunk_pos) {
            log::debug!(
                "[SPAWN] Chunk ({}, {}) dirty: {}, non-air pixels: {}",
                chunk_pos.x,
                chunk_pos.y,
                chunk.dirty,
                chunk.count_non_air()
            );
        }
    }

    /// Spawn creature at player's current position
    pub fn spawn_creature_at_player(
        &mut self,
        genome: crate::creature::genome::CreatureGenome,
    ) -> sunaba_creature::EntityId {
        self.creature_manager
            .spawn_creature(genome, self.player.position)
    }

    /// Mine a single pixel and add it to player's inventory
    /// Returns true if successfully mined
    pub fn mine_pixel(&mut self, world_x: i32, world_y: i32) -> bool {
        let (chunk_pos, local_x, local_y) = ChunkManager::world_to_chunk_coords(world_x, world_y);

        if let Some(chunk) = self.chunk_manager.chunks.get_mut(&chunk_pos) {
            let pixel = chunk.get_pixel(local_x, local_y);
            let material_id = pixel.material_id;

            // Can't mine air or bedrock
            if material_id == MaterialId::AIR || material_id == MaterialId::BEDROCK {
                return false;
            }

            // Try to add to inventory
            if self.player.mine_material(material_id) {
                // Successfully added to inventory, remove the pixel
                chunk.set_material(local_x, local_y, MaterialId::AIR);
                chunk.dirty = true;

                let material_name = &self.materials.get(material_id).name;
                log::debug!(
                    "[MINE] Mined {} at ({}, {})",
                    material_name,
                    world_x,
                    world_y
                );
                true
            } else {
                log::debug!(
                    "[MINE] Inventory full, can't mine at ({}, {})",
                    world_x,
                    world_y
                );
                false
            }
        } else {
            false
        }
    }

    /// Place material from player's inventory at world coordinates with circular brush
    /// Returns number of pixels successfully placed
    pub fn place_material_from_inventory(
        &mut self,
        world_x: i32,
        world_y: i32,
        material_id: u16,
    ) -> u32 {
        let material_name = self.materials.get(material_id).name.clone();
        let mut placed = 0;

        // Calculate how many pixels we want to place (circular brush)
        let mut pixels_needed = 0;
        for dy in -Self::BRUSH_RADIUS..=Self::BRUSH_RADIUS {
            for dx in -Self::BRUSH_RADIUS..=Self::BRUSH_RADIUS {
                if dx * dx + dy * dy <= Self::BRUSH_RADIUS * Self::BRUSH_RADIUS {
                    let x = world_x + dx;
                    let y = world_y + dy;
                    // Only count if target pixel is air (can be replaced)
                    if self
                        .get_pixel(x, y)
                        .map(|p| p.material_id == MaterialId::AIR)
                        .unwrap_or(false)
                    {
                        pixels_needed += 1;
                    }
                }
            }
        }

        if pixels_needed == 0 {
            return 0;
        }

        // Check if player has enough material
        if !self.player.inventory.has_item(material_id, pixels_needed) {
            log::debug!(
                "[PLACE] Not enough {} in inventory (need {}, have {})",
                material_name,
                pixels_needed,
                self.player.inventory.count_item(material_id)
            );
            return 0;
        }

        // Consume from inventory first
        let consumed = self
            .player
            .inventory
            .remove_item(material_id, pixels_needed);

        // Place the pixels with PLAYER_PLACED flag
        for dy in -Self::BRUSH_RADIUS..=Self::BRUSH_RADIUS {
            for dx in -Self::BRUSH_RADIUS..=Self::BRUSH_RADIUS {
                if dx * dx + dy * dy <= Self::BRUSH_RADIUS * Self::BRUSH_RADIUS {
                    let x = world_x + dx;
                    let y = world_y + dy;
                    if self
                        .get_pixel(x, y)
                        .map(|p| p.material_id == MaterialId::AIR)
                        .unwrap_or(false)
                    {
                        let mut pixel = Pixel::new(material_id);
                        pixel.flags |= pixel_flags::PLAYER_PLACED;
                        self.set_pixel_full(x, y, pixel);
                        placed += 1;
                    }
                }
            }
        }

        log::debug!(
            "[PLACE] Placed {} {} pixels at ({}, {}), consumed {} from inventory",
            placed,
            material_name,
            world_x,
            world_y,
            consumed
        );

        placed
    }

    /// Place material at world coordinates without consuming from inventory (debug mode)
    pub fn place_material_debug(&mut self, world_x: i32, world_y: i32, material_id: u16) -> u32 {
        let mut placed = 0;

        for dy in -Self::BRUSH_RADIUS..=Self::BRUSH_RADIUS {
            for dx in -Self::BRUSH_RADIUS..=Self::BRUSH_RADIUS {
                if dx * dx + dy * dy <= Self::BRUSH_RADIUS * Self::BRUSH_RADIUS {
                    let x = world_x + dx;
                    let y = world_y + dy;
                    if self
                        .get_pixel(x, y)
                        .map(|p| p.material_id == MaterialId::AIR)
                        .unwrap_or(false)
                    {
                        let mut pixel = Pixel::new(material_id);
                        pixel.flags |= pixel_flags::PLAYER_PLACED;
                        self.set_pixel_full(x, y, pixel);
                        placed += 1;
                    }
                }
            }
        }

        placed
    }

    /// Start mining a pixel (calculates required time based on material hardness and tool)
    pub fn start_mining(&mut self, world_x: i32, world_y: i32) {
        // Get the pixel
        let pixel = match self.get_pixel(world_x, world_y) {
            Some(p) => p,
            None => return, // Out of bounds
        };

        let material = self.materials.get(pixel.material_id);

        // Can't mine air or materials without hardness (bedrock)
        if material.hardness.is_none() {
            return;
        }

        // Get equipped tool
        let tool = self.player.get_equipped_tool(&self.tool_registry);

        // Calculate mining time
        let required_time = calculate_mining_time(1.0, material, tool);

        // Start mining
        self.player
            .mining_progress
            .start((world_x, world_y), required_time);

        log::debug!(
            "[MINING] Started mining {} at ({}, {}) - required time: {:.2}s (tool: {:?})",
            material.name,
            world_x,
            world_y,
            required_time,
            tool.map(|t| t.name.as_str())
        );
    }

    /// Update mining progress (called each frame)
    /// Returns true if mining completed this frame
    pub fn update_mining(&mut self, delta_time: f32) -> bool {
        if self.player.update_mining(delta_time) {
            // Mining completed
            if let Some((x, y)) = self.player.mining_progress.target_pixel {
                self.complete_mining(x, y);
                return true;
            }
        }
        false
    }

    /// Complete mining at the specified position
    fn complete_mining(&mut self, world_x: i32, world_y: i32) {
        // Get the pixel
        let pixel = match self.get_pixel(world_x, world_y) {
            Some(p) => p,
            None => {
                log::warn!(
                    "[MINING] Complete mining failed: pixel at ({}, {}) not found",
                    world_x,
                    world_y
                );
                return;
            }
        };

        let material_id = pixel.material_id;
        let material_name = self.materials.get(material_id).name.clone();

        // Add to inventory
        if self.player.mine_material(material_id) {
            // Remove pixel
            self.set_pixel(world_x, world_y, MaterialId::AIR);

            // Damage tool durability
            if let Some(tool_id) = self.player.equipped_tool {
                let broke = self.player.inventory.damage_tool(tool_id, 1);
                if broke {
                    let tool_name = self
                        .tool_registry
                        .get(tool_id)
                        .map(|t| t.name.as_str())
                        .unwrap_or("Unknown");
                    log::info!("[MINING] {} broke!", tool_name);
                    self.player.unequip_tool();
                }
            }

            log::debug!(
                "[MINING] Completed mining {} at ({}, {})",
                material_name,
                world_x,
                world_y
            );
        } else {
            log::warn!(
                "[MINING] Failed to add {} to inventory (full?)",
                material_name
            );
        }
    }

    /// DEBUG: Instantly mine all materials in a circle around position
    /// Used for quick world exploration during testing
    pub fn debug_mine_circle(&mut self, center_x: i32, center_y: i32, radius: i32) {
        // Iterate over square containing circle
        for dy in -radius..=radius {
            for dx in -radius..=radius {
                // Check if point is inside circle (Euclidean distance)
                if dx * dx + dy * dy <= radius * radius {
                    let x = center_x + dx;
                    let y = center_y + dy;

                    // Get pixel material
                    if let Some(pixel) = self.get_pixel(x, y) {
                        let material_id = pixel.material_id;

                        // Skip air and bedrock
                        if material_id == MaterialId::AIR {
                            continue;
                        }

                        let material = self.materials.get(material_id);
                        if material.hardness.is_none() {
                            continue; // Bedrock/unmineable materials
                        }

                        // Remove pixel and add to inventory
                        self.player.mine_material(material_id);
                        self.set_pixel(x, y, MaterialId::AIR);
                    }
                }
            }
        }

        log::debug!(
            "[DEBUG MINING] Mined circle at ({}, {}) with radius {}",
            center_x,
            center_y,
            radius
        );
    }

    /// Update simulation
    pub fn update<R: crate::world::WorldRng>(
        &mut self,
        dt: f32,
        stats: &mut dyn crate::world::SimStats,
        rng: &mut R,
    ) {
        const FIXED_TIMESTEP: f32 = 1.0 / 60.0;

        // Update player (hunger, health, starvation damage)
        if self.player.update(dt) {
            log::warn!("Player died from starvation!");
            // TODO: Handle player death (respawn, game over screen, etc.)
        }

        // Update light system (day/night cycle, growth timer)
        self.light_system.update(dt);

        self.time_accumulator += dt;

        // Cap simulation steps to prevent "spiral of death"
        // If FPS drops, simulation slows down gracefully instead of trying to catch up
        const MAX_STEPS_PER_FRAME: u32 = 2;
        let mut steps = 0;

        while self.time_accumulator >= FIXED_TIMESTEP && steps < MAX_STEPS_PER_FRAME {
            self.step_simulation(stats, rng);
            self.time_accumulator -= FIXED_TIMESTEP;
            steps += 1;
        }

        // Clamp accumulator to prevent runaway
        if self.time_accumulator > FIXED_TIMESTEP * 2.0 {
            self.time_accumulator = FIXED_TIMESTEP;
        }
    }

    fn step_simulation<R: crate::world::WorldRng>(
        &mut self,
        stats: &mut dyn crate::world::SimStats,
        rng: &mut R,
    ) {
        #[cfg(feature = "profiling")]
        puffin::profile_function!();

        // 0. Update active chunks (remove distant, re-activate nearby)
        self.update_active_chunks();

        // 0.5. Dynamic chunk loading when player enters new chunk
        let current_chunk = IVec2::new(
            (self.player.position.x as i32).div_euclid(CHUNK_SIZE as i32),
            (self.player.position.y as i32).div_euclid(CHUNK_SIZE as i32),
        );

        if self.chunk_manager.last_load_chunk_pos != Some(current_chunk) {
            self.persistence_system.load_nearby_chunks(&mut self.chunk_manager, self.player.position);
            self.chunk_manager.last_load_chunk_pos = Some(current_chunk);
        }

        // 1. Clear update flags
        for pos in &self.chunk_manager.active_chunks {
            if let Some(chunk) = self.chunk_manager.chunks.get_mut(pos) {
                chunk.clear_update_flags();
            }
        }

        // 2. CA updates (movement) - only process chunks that need updating
        // A chunk needs updating if it or any of its 8 neighbors had changes last frame
        let chunks_to_update: Vec<IVec2> = self
            .chunk_manager
            .active_chunks
            .iter()
            .copied()
            .filter(|&pos| self.chunk_needs_ca_update(pos))
            .collect();

        // Note: dirty_rect is cleared by the renderer after rendering
        // This allows proper dirty tracking for render optimization

        // Clear simulation_active for chunks we're about to update
        // It will be re-set by try_move_world() if any material actually moves
        for pos in &chunks_to_update {
            if let Some(chunk) = self.chunk_manager.chunks.get_mut(pos) {
                chunk.set_simulation_active(false);
            }
        }

        {
            #[cfg(feature = "profiling")]
            puffin::profile_scope!("ca_updates");
            for pos in &chunks_to_update {
                self.update_chunk_ca(*pos, stats, rng);
            }
        }

        // 3. Temperature diffusion (30fps throttled) - active chunks only
        {
            #[cfg(feature = "profiling")]
            puffin::profile_scope!("temperature");
            self.temperature_sim.update(
                &mut self.chunk_manager.chunks,
                &self.chunk_manager.active_chunks,
            );
        }

        // 4. Light propagation (15fps throttled) - active chunks only
        let active_chunks = self.chunk_manager.active_chunks.clone();
        self.light_system.update_light_propagation(
            &mut self.chunk_manager,
            &self.materials,
            &active_chunks,
        );

        // 5. State changes based on temperature
        for i in 0..self.chunk_manager.active_chunks.len() {
            let pos = self.chunk_manager.active_chunks[i];
            self.check_chunk_state_changes(pos, stats);
        }

        // 6. Process structural integrity checks
        let positions = self.structural_system.drain_queue();
        let checks_processed = StructuralIntegritySystem::process_checks(self, positions);
        if checks_processed > 0 {
            log::debug!("Processed {} structural integrity checks", checks_processed);
        }

        // 7. Update falling chunks (kinematic debris physics)
        // Temporarily take debris_system to avoid borrow checker issues with self as WorldCollisionQuery
        let mut debris_system = std::mem::take(&mut self.debris_system);
        {
            #[cfg(feature = "profiling")]
            puffin::profile_scope!("falling_chunks");
            let settled_chunks = debris_system.update(1.0 / 60.0, self);
            for chunk in settled_chunks {
                self.reconstruct_falling_chunk(chunk);
            }
        }
        self.debris_system = debris_system;

        // 9. Resource regeneration (fruit spawning)
        self.regeneration_system.update(
            &mut self.chunk_manager.chunks,
            &self.chunk_manager.active_chunks,
            1.0 / 60.0,
        );

        // 10. Update creatures (sensing, planning, neural control)
        // Temporarily take creature_manager to avoid borrow checker issues
        let mut creature_manager = std::mem::replace(
            &mut self.creature_manager,
            crate::creature::spawning::CreatureManager::new(0), // Dummy placeholder
        );

        {
            #[cfg(feature = "profiling")]
            puffin::profile_scope!("creatures");
            creature_manager.update(1.0 / 60.0, self);

            // 11. Execute creature actions (eat, mine, build)
            creature_manager.execute_actions(self, 1.0 / 60.0);
        }

        // Put it back
        self.creature_manager = creature_manager;
    }

    /// Get growth progress as percentage (0-100) through the 10-second cycle
    pub fn get_growth_progress_percent(&self) -> f32 {
        self.light_system.get_growth_progress_percent()
    }

    /// Check if a chunk needs CA update based on dirty state of itself and neighbors
    /// Returns true if this chunk or any of its 8 neighbors have dirty_rect set or simulation_active
    fn chunk_needs_ca_update(&self, pos: IVec2) -> bool {
        // Check the chunk itself
        if let Some(chunk) = self.chunk_manager.chunks.get(&pos)
            && (chunk.dirty_rect.is_some() || chunk.is_simulation_active())
        {
            return true;
        }

        // Check all 8 neighbors - materials can flow in from any direction
        for dy in -1..=1 {
            for dx in -1..=1 {
                if dx == 0 && dy == 0 {
                    continue;
                }
                let neighbor_pos = IVec2::new(pos.x + dx, pos.y + dy);
                if let Some(neighbor) = self.chunk_manager.chunks.get(&neighbor_pos)
                    && (neighbor.dirty_rect.is_some() || neighbor.is_simulation_active())
                {
                    return true;
                }
            }
        }

        false
    }

    fn update_chunk_ca<R: crate::world::WorldRng>(
        &mut self,
        chunk_pos: IVec2,
        stats: &mut dyn crate::world::SimStats,
        rng: &mut R,
    ) {
        // Update from bottom to top so falling works correctly
        for y in 0..CHUNK_SIZE {
            // Alternate direction each row for symmetry
            let x_iter: Box<dyn Iterator<Item = usize>> = if y % 2 == 0 {
                Box::new(0..CHUNK_SIZE)
            } else {
                Box::new((0..CHUNK_SIZE).rev())
            };

            for x in x_iter {
                self.update_pixel(chunk_pos, x, y, stats, rng);
            }
        }
    }

    fn update_pixel<R: crate::world::WorldRng>(
        &mut self,
        chunk_pos: IVec2,
        x: usize,
        y: usize,
        stats: &mut dyn crate::world::SimStats,
        rng: &mut R,
    ) {
        let chunk = match self.chunk_manager.chunks.get(&chunk_pos) {
            Some(c) => c,
            None => return,
        };

        let pixel = chunk.get_pixel(x, y);
        if pixel.is_empty() {
            return;
        }

        // Special handling for fire
        if pixel.material_id == MaterialId::FIRE {
            self.update_fire(chunk_pos, x, y, stats, rng);
            return;
        }

        // Check if pixel should ignite (before movement)
        if pixel.flags & pixel_flags::BURNING == 0 {
            self.check_ignition(chunk_pos, x, y);
        }

        // Update burning materials
        if pixel.flags & pixel_flags::BURNING != 0 {
            self.update_burning_material(chunk_pos, x, y, rng);
        }

        // Get material type for movement logic
        let material_type = self.materials.get(pixel.material_id).material_type;

        // Normal CA movement - delegate to CellularAutomataUpdater
        match material_type {
            MaterialType::Powder => {
                CellularAutomataUpdater::update_powder(
                    &mut self.chunk_manager.chunks,
                    chunk_pos,
                    x,
                    y,
                    &self.materials,
                    stats,
                    rng,
                );
            }
            MaterialType::Liquid => {
                CellularAutomataUpdater::update_liquid(
                    &mut self.chunk_manager.chunks,
                    chunk_pos,
                    x,
                    y,
                    &self.materials,
                    stats,
                    rng,
                );
            }
            MaterialType::Gas => {
                CellularAutomataUpdater::update_gas(
                    &mut self.chunk_manager.chunks,
                    chunk_pos,
                    x,
                    y,
                    &self.materials,
                    stats,
                    rng,
                );
            }
            MaterialType::Solid => {
                // Solids don't move
            }
        }

        // Check reactions with neighbors (after movement)
        self.check_pixel_reactions(chunk_pos, x, y, stats, rng);
    }

    /// Get pixel at world coordinates
    pub fn get_pixel(&self, world_x: i32, world_y: i32) -> Option<Pixel> {
        let (chunk_pos, local_x, local_y) = ChunkManager::world_to_chunk_coords(world_x, world_y);
        self.chunk_manager
            .chunks
            .get(&chunk_pos)
            .map(|c| c.get_pixel(local_x, local_y))
    }

    /// Ensure chunks exist for a given pixel area (creates empty chunks if needed)
    /// Used by headless training to set up scenarios without full world generation
    pub fn ensure_chunks_for_area(&mut self, min_x: i32, min_y: i32, max_x: i32, max_y: i32) {
        let (min_chunk, _, _) = ChunkManager::world_to_chunk_coords(min_x, min_y);
        let (max_chunk, _, _) = ChunkManager::world_to_chunk_coords(max_x, max_y);

        for cy in min_chunk.y..=max_chunk.y {
            for cx in min_chunk.x..=max_chunk.x {
                let pos = IVec2::new(cx, cy);
                self.chunk_manager
                    .chunks
                    .entry(pos)
                    .or_insert_with(|| Chunk::new(cx, cy));
            }
        }
    }

    /// Set pixel at world coordinates
    pub fn set_pixel(&mut self, world_x: i32, world_y: i32, material_id: u16) {
        self.set_pixel_full(world_x, world_y, Pixel::new(material_id));
    }

    /// Set pixel at world coordinates with full Pixel struct (including flags)
    pub fn set_pixel_full(&mut self, world_x: i32, world_y: i32, pixel: Pixel) {
        // Check if we're removing a player-placed structural material
        let schedule_check = if let Some(old_pixel) = self.get_pixel(world_x, world_y) {
            if !old_pixel.is_empty() {
                let old_material = self.materials.get(old_pixel.material_id);
                // Only schedule structural check for player-placed structural solids
                old_material.structural
                    && old_material.material_type == MaterialType::Solid
                    && (old_pixel.flags & pixel_flags::PLAYER_PLACED) != 0
            } else {
                false
            }
        } else {
            false
        };

        // Set the new pixel
        let (chunk_pos, local_x, local_y) = ChunkManager::world_to_chunk_coords(world_x, world_y);
        if let Some(chunk) = self.chunk_manager.chunks.get_mut(&chunk_pos) {
            let old_material_id = chunk.get_material(local_x, local_y);
            chunk.set_pixel(local_x, local_y, pixel);

            // Log only if actually changing something (not just setting same material)
            if old_material_id != pixel.material_id {
                let material_name = &self.materials.get(pixel.material_id).name;
                log::trace!(
                    "[MODIFY] Chunk ({}, {}) at local ({}, {}) world ({}, {}) set to {} (was {})",
                    chunk_pos.x,
                    chunk_pos.y,
                    local_x,
                    local_y,
                    world_x,
                    world_y,
                    material_name,
                    old_material_id
                );
            }
        } else {
            log::warn!(
                "set_pixel_full: chunk {:?} not loaded (world: {}, {})",
                chunk_pos,
                world_x,
                world_y
            );
            return;
        }

        // Schedule structural check if we removed player-placed structural material with AIR
        if schedule_check && pixel.material_id == MaterialId::AIR {
            self.structural_system.schedule_check(world_x, world_y);
        }
    }

    /// Get temperature at world coordinates
    pub fn get_temperature_at_pixel(&self, world_x: i32, world_y: i32) -> f32 {
        let (chunk_pos, local_x, local_y) = ChunkManager::world_to_chunk_coords(world_x, world_y);
        if let Some(chunk) = self.chunk_manager.chunks.get(&chunk_pos) {
            get_temperature_at_pixel(chunk, local_x, local_y)
        } else {
            20.0 // Default ambient temperature
        }
    }

    /// Get light level at world coordinates (0-15)
    pub fn get_light_at(&self, world_x: i32, world_y: i32) -> Option<u8> {
        self.light_system.get_light_at(&self.chunk_manager, world_x, world_y)
    }

    /// Set light level at world coordinates (0-15)
    pub fn set_light_at(&mut self, world_x: i32, world_y: i32, level: u8) {
        self.light_system.set_light_at(&mut self.chunk_manager, world_x, world_y, level);
    }

    /// Get material ID at world coordinates
    pub fn get_pixel_material(&self, world_x: i32, world_y: i32) -> Option<u16> {
        self.get_pixel(world_x, world_y).map(|p| p.material_id)
    }
    /// Get iterator over active chunks
    pub fn active_chunks(&self) -> impl Iterator<Item = &Chunk> {
        self.chunk_manager
            .active_chunks
            .iter()
            .filter_map(|pos| self.chunk_manager.chunks.get(pos))
    }

    /// Get positions of active chunks
    pub fn active_chunk_positions(&self) -> &[IVec2] {
        &self.chunk_manager.active_chunks
    }

    /// Get all loaded chunks
    pub fn chunks(&self) -> &HashMap<IVec2, Chunk> {
        &self.chunk_manager.chunks
    }

    /// Get all loaded chunks (mutable)
    pub fn chunks_mut(&mut self) -> &mut HashMap<IVec2, Chunk> {
        &mut self.chunk_manager.chunks
    }

    /// Check if chunk is loaded (for SpacetimeDB server)
    pub fn has_chunk(&self, pos: IVec2) -> bool {
        self.chunk_manager.chunks.contains_key(&pos)
    }

    /// Insert pre-loaded chunk (for SpacetimeDB server)
    pub fn insert_chunk(&mut self, pos: IVec2, chunk: Chunk) {
        self.chunk_manager.chunks.insert(pos, chunk);
    }

    /// Generate a single chunk at position (for SpacetimeDB server)
    pub fn generate_chunk(&mut self, pos: IVec2) {
        let chunk = self.persistence_system.generator.generate_chunk(pos.x, pos.y);
        self.chunk_manager.chunks.insert(pos, chunk);
    }

    /// Get chunk at position (for SpacetimeDB server)
    pub fn get_chunk(&self, x: i32, y: i32) -> Option<&Chunk> {
        self.chunk_manager.chunks.get(&IVec2::new(x, y))
    }

    /// Iterator over all chunks (for SpacetimeDB server)
    pub fn chunks_iter(&self) -> impl Iterator<Item = (&IVec2, &Chunk)> {
        self.chunk_manager.chunks.iter()
    }

    /// Simulate a single chunk for settlement (for SpacetimeDB server)
    /// Called multiple times to settle a chunk before players arrive
    pub fn update_chunk_settle<R: crate::world::WorldRng>(
        &mut self,
        chunk_x: i32,
        chunk_y: i32,
        rng: &mut R,
    ) {
        let pos = IVec2::new(chunk_x, chunk_y);
        if self.chunk_manager.chunks.contains_key(&pos) {
            // Simulate CA for this chunk only
            let mut no_op_stats = NoopStats;
            self.update_chunk_ca(pos, &mut no_op_stats, rng);
        }
    }

    /// Get materials registry
    pub fn materials(&self) -> &Materials {
        &self.materials
    }

    /// Get falling chunks for rendering (kinematic debris system)
    pub fn get_falling_chunks(&self) -> Vec<ChunkRenderData> {
        self.debris_system.get_render_data()
    }

    /// Get count of active falling chunks (for debug stats)
    pub fn falling_chunk_count(&self) -> usize {
        self.debris_system.chunk_count()
    }

    /// Get creature render data for rendering
    pub fn get_creature_render_data(&self) -> Vec<crate::creature::CreatureRenderData> {
        self.creature_manager.get_render_data()
    }

    /// Create falling debris from a pixel region
    /// Removes pixels from world and creates a falling chunk
    pub fn create_debris(&mut self, region: std::collections::HashSet<IVec2>) -> u64 {
        log::info!("Creating falling chunk from {} pixels", region.len());

        // Build pixel map with materials
        let mut pixels = std::collections::HashMap::new();
        for pos in &region {
            if let Some(pixel) = self.get_pixel(pos.x, pos.y)
                && !pixel.is_empty()
            {
                pixels.insert(*pos, pixel.material_id);
            }
        }

        if pixels.is_empty() {
            log::warn!("No valid pixels to create falling chunk");
            return 0;
        }

        // Remove pixels from world (convert to air)
        for pos in &region {
            DebrisSystem::set_pixel_direct(&mut self.chunk_manager, pos.x, pos.y, MaterialId::AIR);
        }

        // Create falling chunk (kinematic, WASM-compatible) - pass pixels directly
        let id = self.debris_system.create_chunk(pixels);
        log::debug!("Created falling chunk id: {}", id);
        id
    }

    /// Reconstruct a settled falling chunk back into static pixels
    fn reconstruct_falling_chunk(&mut self, chunk: FallingChunk) {
        let center_i = IVec2::new(chunk.center.x.round() as i32, chunk.center.y.round() as i32);

        log::info!(
            "Reconstructing falling chunk {} ({} pixels) at ({}, {})",
            chunk.id,
            chunk.pixels.len(),
            center_i.x,
            center_i.y
        );

        let mut placed = 0;
        let mut failed = 0;

        for (relative_pos, material_id) in chunk.pixels {
            let world_pos = center_i + relative_pos;

            // Only place if target position is empty (air)
            if let Some(existing) = self.get_pixel(world_pos.x, world_pos.y)
                && existing.is_empty()
            {
                if DebrisSystem::set_pixel_direct_checked(&mut self.chunk_manager, world_pos.x, world_pos.y, material_id) {
                    placed += 1;
                } else {
                    failed += 1;
                }
            }
        }

        if failed > 0 {
            log::warn!(
                "Falling chunk reconstruction: {} pixels placed, {} failed",
                placed,
                failed
            );
        } else {
            log::debug!("Placed {} pixels from falling chunk", placed);
        }
    }


    /// Clear all chunks from the world
    pub fn clear_all_chunks(&mut self) {
        self.persistence_system.clear_all_chunks(&mut self.chunk_manager);
    }

    /// Add a chunk to the world
    pub fn add_chunk(&mut self, chunk: Chunk) {
        self.persistence_system.add_chunk(&mut self.chunk_manager, chunk, self.player.position);
    }

    /// Initialize persistent world (load or generate)
    pub fn load_persistent_world(&mut self) {
        let _ = self.persistence_system.load_persistent_world(&mut self.chunk_manager, &mut self.player);
        // Initialize light levels before first CA update
        let active_chunks = self.chunk_manager.active_chunks.clone();
        self.light_system.initialize_light(
            &mut self.chunk_manager,
            &self.materials,
            &active_chunks,
        );
    }

    /// Disable persistence for demo levels
    pub fn disable_persistence(&mut self) {
        self.persistence_system.disable_persistence(&mut self.chunk_manager);
    }

    /// Save all dirty chunks (periodic auto-save)
    pub fn save_dirty_chunks(&mut self) {
        self.persistence_system.save_dirty_chunks(&mut self.chunk_manager);
    }

    /// Save all chunks and metadata (manual save)
    pub fn save_all_dirty_chunks(&mut self) {
        self.persistence_system.save_all_dirty_chunks(&mut self.chunk_manager, &self.player);
    }

    /// Check all pixels in a chunk for state changes based on temperature
    fn check_chunk_state_changes(
        &mut self,
        chunk_pos: IVec2,
        stats: &mut dyn crate::world::SimStats,
    ) {
        ChemistrySystem::check_chunk_state_changes(
            &mut self.chunk_manager.chunks,
            chunk_pos,
            &self.materials,
            stats,
        );
    }

    /// Update fire pixel behavior
    fn update_fire<R: crate::world::WorldRng>(
        &mut self,
        chunk_pos: IVec2,
        x: usize,
        y: usize,
        stats: &mut dyn crate::world::SimStats,
        rng: &mut R,
    ) {
        ChemistrySystem::update_fire(
            &mut self.chunk_manager.chunks,
            chunk_pos,
            x,
            y,
            &self.materials,
            stats,
            rng,
        );
    }

    /// Check if a pixel should ignite based on temperature
    fn check_ignition(&mut self, chunk_pos: IVec2, x: usize, y: usize) {
        ChemistrySystem::check_ignition(
            &mut self.chunk_manager.chunks,
            chunk_pos,
            x,
            y,
            &self.materials,
        );
    }

    /// Update burning material (gradual consumption)
    fn update_burning_material<R: crate::world::WorldRng>(
        &mut self,
        chunk_pos: IVec2,
        x: usize,
        y: usize,
        rng: &mut R,
    ) {
        ChemistrySystem::update_burning_material(
            &mut self.chunk_manager.chunks,
            chunk_pos,
            x,
            y,
            &self.materials,
            rng,
        );
    }

    /// Check for chemical reactions with neighboring pixels
    fn check_pixel_reactions<R: crate::world::WorldRng>(
        &mut self,
        chunk_pos: IVec2,
        x: usize,
        y: usize,
        stats: &mut dyn crate::world::SimStats,
        rng: &mut R,
    ) {
        // Need to avoid borrow checker issues by not capturing self in closures
        // Instead, inline the logic here
        let chunk = match self.chunk_manager.chunks.get(&chunk_pos) {
            Some(c) => c,
            None => return,
        };

        let pixel = chunk.get_pixel(x, y);
        if pixel.is_empty() {
            return;
        }

        let temp = get_temperature_at_pixel(chunk, x, y);
        let light_level = chunk.get_light(x, y);
        let world_x = chunk_pos.x * CHUNK_SIZE as i32 + x as i32;
        let world_y = chunk_pos.y * CHUNK_SIZE as i32 + y as i32;

        // Get pressure at this pixel
        let pressure = chunk.get_pressure_at(x, y);

        // Collect all 8 neighbor materials for catalyst checking
        let mut neighbor_materials = Vec::with_capacity(8);
        for (dx, dy) in [
            (-1, -1),
            (0, -1),
            (1, -1),
            (-1, 0),
            (1, 0),
            (-1, 1),
            (0, 1),
            (1, 1),
        ] {
            if let Some(p) = self.get_pixel(world_x + dx, world_y + dy) {
                neighbor_materials.push(p.material_id);
            }
        }

        // Check 4 neighbors for reactions
        for (dx, dy) in [(0, 1), (1, 0), (0, -1), (-1, 0)] {
            let neighbor = match self.get_pixel(world_x + dx, world_y + dy) {
                Some(p) => p,
                None => continue,
            };

            if neighbor.is_empty() {
                continue;
            }

            // Find matching reaction
            if let Some(reaction) = self.reactions.find_reaction(
                pixel.material_id,
                neighbor.material_id,
                temp,
                light_level,
                pressure,
                &neighbor_materials,
            ) {
                // Probability check
                if rng.check_probability(reaction.probability) {
                    // Apply reaction - get correct outputs based on material order
                    let (output_a, output_b) = self.reactions.get_outputs(
                        reaction,
                        pixel.material_id,
                        neighbor.material_id,
                    );

                    self.set_pixel(world_x, world_y, output_a);
                    self.set_pixel(world_x + dx, world_y + dy, output_b);
                    stats.record_reaction();
                    return; // Only one reaction per pixel per frame
                }
            }
        }
    }
}

impl Default for World {
    fn default() -> Self {
        Self::new()
    }
}

// Implement WorldCollisionQuery for falling chunks collision detection
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

// Implement WorldAccess trait for creature integration
impl sunaba_creature::WorldAccess for World {
    fn get_pixel(&self, x: i32, y: i32) -> Option<sunaba_simulation::Pixel> {
        World::get_pixel(self, x, y)
    }

    fn get_temperature_at_pixel(&self, x: i32, y: i32) -> f32 {
        World::get_temperature_at_pixel(self, x, y)
    }

    fn get_light_at(&self, x: i32, y: i32) -> Option<u8> {
        World::get_light_at(self, x, y)
    }

    fn materials(&self) -> &sunaba_simulation::Materials {
        World::materials(self)
    }

    fn is_solid_at(&self, x: i32, y: i32) -> bool {
        if let Some(pixel) = self.get_pixel(x, y) {
            let material = self.materials().get(pixel.material_id);
            matches!(
                material.material_type,
                sunaba_simulation::MaterialType::Solid | sunaba_simulation::MaterialType::Powder
            )
        } else {
            false
        }
    }

    fn check_circle_collision(&self, x: f32, y: f32, radius: f32) -> bool {
        World::check_circle_collision(self, x, y, radius)
    }

    fn raycast(
        &self,
        from: glam::Vec2,
        direction: glam::Vec2,
        max_distance: f32,
    ) -> Option<(i32, i32, u16)> {
        // Simple raycast implementation
        let step = 0.5;
        let mut dist = 0.0;
        while dist < max_distance {
            let pos = from + direction * dist;
            let px = pos.x.round() as i32;
            let py = pos.y.round() as i32;
            if let Some(pixel) = self.get_pixel(px, py)
                && pixel.material_id != sunaba_simulation::MaterialId::AIR
            {
                return Some((px, py, pixel.material_id));
            }
            dist += step;
        }
        None
    }

    fn get_pressure_at(&self, x: i32, y: i32) -> f32 {
        // Get pressure from chunk's coarse grid
        let (chunk_pos, local_x, local_y) = ChunkManager::world_to_chunk_coords(x, y);
        if let Some(chunk) = self.chunk_manager.chunks.get(&chunk_pos) {
            chunk.get_pressure_at(local_x, local_y)
        } else {
            1.0 // Default atmospheric pressure
        }
    }

    fn is_creature_grounded(&self, positions: &[(glam::Vec2, f32)]) -> bool {
        World::is_creature_grounded(self, positions)
    }

    fn get_blocking_pixel(
        &self,
        from: glam::Vec2,
        direction: glam::Vec2,
        radius: f32,
        max_distance: f32,
    ) -> Option<(i32, i32, u16)> {
        World::get_blocking_pixel(self, from, direction, radius, max_distance)
    }
}

impl sunaba_creature::WorldMutAccess for World {
    fn set_pixel(&mut self, x: i32, y: i32, material_id: u16) {
        World::set_pixel(self, x, y, material_id)
    }

    fn set_pixel_full(&mut self, x: i32, y: i32, pixel: sunaba_simulation::Pixel) {
        World::set_pixel_full(self, x, y, pixel)
    }
}
