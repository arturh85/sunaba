//! World - manages chunks and simulation

use glam::IVec2;
use std::collections::HashMap;

use super::generation::WorldGenerator;
use super::persistence::{ChunkPersistence, WorldMetadata};
use super::{CHUNK_SIZE, Chunk, Pixel, pixel_flags};
use crate::entity::crafting::RecipeRegistry;
use crate::entity::player::Player;
use crate::entity::tools::ToolRegistry;
use crate::physics::PhysicsWorld;
use crate::simulation::{
    LightPropagation, MaterialId, MaterialType, Materials, ReactionRegistry, RegenerationSystem,
    StateChangeSystem, StructuralIntegritySystem, TemperatureSimulator, add_heat_at_pixel,
    get_temperature_at_pixel, mining::calculate_mining_time,
};

/// The game world, composed of chunks
pub struct World {
    /// Loaded chunks, keyed by chunk coordinates
    pub chunks: HashMap<IVec2, Chunk>,

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

    /// Light propagation system
    light_propagation: LightPropagation,

    /// Resource regeneration system
    regeneration_system: RegenerationSystem,

    /// Physics world for rigid bodies
    physics_world: PhysicsWorld,

    /// Creature manager (spawning, AI, behavior)
    pub creature_manager: crate::creature::spawning::CreatureManager,

    /// The player entity
    pub player: Player,

    /// Which chunks are currently active (being simulated)
    active_chunks: Vec<IVec2>,

    /// Last player chunk position for dynamic chunk loading
    last_load_chunk_pos: Option<IVec2>,

    /// Simulation time accumulator
    time_accumulator: f32,

    /// Light propagation time accumulator (15fps throttled)
    light_time_accumulator: f32,

    /// Day/night cycle time (in seconds, 0-1200, where 1200s = 20 min = 24 hours game time)
    day_night_time: f32,

    /// Growth cycle timer (0-10 seconds, wraps) for tooltip progress display
    growth_timer: f32,

    /// Chunk persistence manager (None for demo levels)
    persistence: Option<ChunkPersistence>,

    /// World generator for new chunks
    generator: WorldGenerator,

    /// Maximum number of chunks to keep loaded in memory
    loaded_chunk_limit: usize,

    /// Demo mode flag - prevents dynamic chunk loading
    demo_mode: bool,
}

impl World {
    pub fn new() -> Self {
        let mut world = Self {
            chunks: HashMap::new(),
            materials: Materials::new(),
            temperature_sim: TemperatureSimulator::new(),
            reactions: ReactionRegistry::new(),
            tool_registry: ToolRegistry::new(),
            recipe_registry: RecipeRegistry::new(),
            structural_system: StructuralIntegritySystem::new(),
            light_propagation: LightPropagation::new(),
            regeneration_system: RegenerationSystem::new(),
            physics_world: PhysicsWorld::new(),
            creature_manager: crate::creature::spawning::CreatureManager::new(200), // Max 200 creatures
            player: Player::new(glam::Vec2::new(0.0, 100.0)),
            active_chunks: Vec::new(),
            last_load_chunk_pos: None,
            time_accumulator: 0.0,
            light_time_accumulator: 0.0,
            day_night_time: 600.0, // Start at noon (midpoint of 0-1200)
            growth_timer: 0.0,
            persistence: None,
            generator: WorldGenerator::new(42), // Default seed
            loaded_chunk_limit: 3000,           // ~19MB max memory
            demo_mode: false,
        };

        // Don't pre-generate - let chunks generate on-demand as player explores
        // (Demo levels still call generate_test_world() explicitly)

        // Initialize light levels before first CA update
        world.initialize_light();

        // Spawn 3 test creatures near spawn point with spacing
        use crate::creature::genome::CreatureGenome;

        world.creature_manager.spawn_creature(
            CreatureGenome::test_biped(),
            glam::Vec2::new(-20.0, 100.0),
            &mut world.physics_world,
        );

        world.creature_manager.spawn_creature(
            CreatureGenome::test_quadruped(),
            glam::Vec2::new(0.0, 100.0),
            &mut world.physics_world,
        );

        world.creature_manager.spawn_creature(
            CreatureGenome::test_worm(),
            glam::Vec2::new(20.0, 100.0),
            &mut world.physics_world,
        );

        log::info!("Spawned 3 test creatures at startup");

        world
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
                self.chunks.insert(IVec2::new(cx, cy), chunk);
                self.active_chunks.push(IVec2::new(cx, cy));
            }
        }

        log::info!(
            "Generated test world: {} chunks, {} pixels",
            self.chunks.len(),
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

    /// Player movement speed in pixels per second
    const PLAYER_SPEED: f32 = 200.0;

    /// Active chunk simulation radius (chunks from player)
    const ACTIVE_CHUNK_RADIUS: i32 = 3; // 7×7 grid = 49 chunks

    /// Check if a rectangle collides with solid materials
    fn check_solid_collision(&self, x: f32, y: f32, width: f32, height: f32) -> bool {
        use crate::simulation::MaterialType;

        // Add collision tolerance - shrink hitbox slightly to prevent snagging on single pixels
        const TOLERANCE: f32 = 0.5; // Pixels of wiggle room
        let effective_width = width - TOLERANCE;
        let effective_height = height - TOLERANCE;

        // Check 8 points around hitbox
        let check_points = [
            (x - effective_width / 2.0, y - effective_height / 2.0), // Bottom-left
            (x + effective_width / 2.0, y - effective_height / 2.0), // Bottom-right
            (x - effective_width / 2.0, y + effective_height / 2.0), // Top-left
            (x + effective_width / 2.0, y + effective_height / 2.0), // Top-right
            (x, y - effective_height / 2.0),                         // Bottom-center
            (x, y + effective_height / 2.0),                         // Top-center
            (x - effective_width / 2.0, y),                          // Left-center
            (x + effective_width / 2.0, y),                          // Right-center
        ];

        for (px, py) in check_points {
            if let Some(pixel) = self.get_pixel(px as i32, py as i32)
                && !pixel.is_empty()
            {
                let material = self.materials.get(pixel.material_id);
                // Collide only with solid materials
                if material.material_type == MaterialType::Solid {
                    return true;
                }
            }
        }
        false
    }

    /// Check if player is standing on ground
    fn is_player_grounded(&self) -> bool {
        use crate::simulation::MaterialType;

        // Check 3 points just below player's feet (more forgiving range)
        let check_y = self.player.position.y - (crate::entity::player::Player::HEIGHT / 2.0) - 1.5;
        let check_points = [
            (
                self.player.position.x - crate::entity::player::Player::WIDTH / 4.0,
                check_y,
            ), // Left
            (self.player.position.x, check_y), // Center
            (
                self.player.position.x + crate::entity::player::Player::WIDTH / 4.0,
                check_y,
            ), // Right
        ];

        for (px, py) in check_points {
            if let Some(pixel) = self.get_pixel(px as i32, py as i32)
                && !pixel.is_empty()
            {
                let material = self.materials.get(pixel.material_id);
                if material.material_type == MaterialType::Solid {
                    return true;
                }
            }
        }
        false
    }

    /// Check if a circle collides with solid materials
    /// Used for creature body part collision detection
    pub fn check_circle_collision(&self, x: f32, y: f32, radius: f32) -> bool {
        use crate::simulation::MaterialType;

        // Check center and 8 points around the perimeter
        let check_points = [
            (x, y),                                   // Center
            (x + radius, y),                          // Right
            (x - radius, y),                          // Left
            (x, y + radius),                          // Top
            (x, y - radius),                          // Bottom
            (x + radius * 0.707, y + radius * 0.707), // Top-right
            (x - radius * 0.707, y + radius * 0.707), // Top-left
            (x + radius * 0.707, y - radius * 0.707), // Bottom-right
            (x - radius * 0.707, y - radius * 0.707), // Bottom-left
        ];

        for (px, py) in check_points {
            if let Some(pixel) = self.get_pixel(px as i32, py as i32)
                && !pixel.is_empty()
            {
                let material = self.materials.get(pixel.material_id);
                if material.material_type == MaterialType::Solid {
                    return true;
                }
            }
        }
        false
    }

    /// Check if any body part in a list is grounded (touching solid below)
    /// positions: Vec of (center, radius) for each body part
    pub fn is_creature_grounded(&self, positions: &[(glam::Vec2, f32)]) -> bool {
        use crate::simulation::MaterialType;

        for (center, radius) in positions {
            // Check 3 points just below the body part
            let check_y = center.y - radius - 1.0;
            let check_points = [
                (center.x - radius * 0.5, check_y),
                (center.x, check_y),
                (center.x + radius * 0.5, check_y),
            ];

            for (px, py) in check_points {
                if let Some(pixel) = self.get_pixel(px as i32, py as i32)
                    && !pixel.is_empty()
                {
                    let material = self.materials.get(pixel.material_id);
                    if material.material_type == MaterialType::Solid {
                        return true;
                    }
                }
            }
        }
        false
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

        // 5. Vertical movement (gravity + jump)
        if self.player.jump_buffer > 0.0 && self.player.coyote_time > 0.0 {
            // Jump!
            self.player.velocity.y = Player::JUMP_VELOCITY;
            self.player.jump_buffer = 0.0;
            self.player.coyote_time = 0.0;
            log::debug!("Player jumped!");
        } else if !self.player.grounded {
            // Apply gravity when airborne
            self.player.velocity.y -= Player::GRAVITY * dt;
            self.player.velocity.y = self.player.velocity.y.max(-Player::MAX_FALL_SPEED);
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
        self.active_chunks.retain(|pos| {
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
                if self.chunks.contains_key(&pos) && !self.active_chunks.contains(&pos) {
                    self.active_chunks.push(pos);
                    // Mark newly activated chunks for simulation so physics/chemistry runs
                    if let Some(chunk) = self.chunks.get_mut(&pos) {
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
        let (chunk_pos, _, _) = Self::world_to_chunk_coords(world_x, world_y);

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
        if let Some(chunk) = self.chunks.get(&chunk_pos) {
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
            .spawn_creature(genome, self.player.position, &mut self.physics_world)
    }

    /// Mine a single pixel and add it to player's inventory
    /// Returns true if successfully mined
    pub fn mine_pixel(&mut self, world_x: i32, world_y: i32) -> bool {
        let (chunk_pos, local_x, local_y) = Self::world_to_chunk_coords(world_x, world_y);

        if let Some(chunk) = self.chunks.get_mut(&chunk_pos) {
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
    pub fn update(&mut self, dt: f32, stats: &mut dyn crate::world::SimStats) {
        const FIXED_TIMESTEP: f32 = 1.0 / 60.0;

        // Update player (hunger, health, starvation damage)
        if self.player.update(dt) {
            log::warn!("Player died from starvation!");
            // TODO: Handle player death (respawn, game over screen, etc.)
        }

        // Update day/night cycle (20 min real-time = 24 hours game-time)
        const DAY_NIGHT_CYCLE_DURATION: f32 = 1200.0; // 20 minutes in seconds
        self.day_night_time = (self.day_night_time + dt) % DAY_NIGHT_CYCLE_DURATION;

        // Update growth timer (10-second cycle for tooltip progress)
        const GROWTH_CYCLE_DURATION: f32 = 10.0;
        self.growth_timer = (self.growth_timer + dt) % GROWTH_CYCLE_DURATION;

        self.time_accumulator += dt;

        // Cap simulation steps to prevent "spiral of death"
        // If FPS drops, simulation slows down gracefully instead of trying to catch up
        const MAX_STEPS_PER_FRAME: u32 = 2;
        let mut steps = 0;

        while self.time_accumulator >= FIXED_TIMESTEP && steps < MAX_STEPS_PER_FRAME {
            self.step_simulation(stats);
            self.time_accumulator -= FIXED_TIMESTEP;
            steps += 1;
        }

        // Clamp accumulator to prevent runaway
        if self.time_accumulator > FIXED_TIMESTEP * 2.0 {
            self.time_accumulator = FIXED_TIMESTEP;
        }
    }

    fn step_simulation(&mut self, stats: &mut dyn crate::world::SimStats) {
        const LIGHT_TIMESTEP: f32 = 1.0 / 15.0; // 15fps for light propagation

        // 0. Update active chunks (remove distant, re-activate nearby)
        self.update_active_chunks();

        // 0.5. Dynamic chunk loading when player enters new chunk
        let current_chunk = IVec2::new(
            (self.player.position.x as i32).div_euclid(CHUNK_SIZE as i32),
            (self.player.position.y as i32).div_euclid(CHUNK_SIZE as i32),
        );

        if self.last_load_chunk_pos != Some(current_chunk) {
            self.load_nearby_chunks();
            self.last_load_chunk_pos = Some(current_chunk);
        }

        // 1. Clear update flags
        for pos in &self.active_chunks {
            if let Some(chunk) = self.chunks.get_mut(pos) {
                chunk.clear_update_flags();
            }
        }

        // 2. CA updates (movement) - only process chunks that need updating
        // A chunk needs updating if it or any of its 8 neighbors had changes last frame
        let chunks_to_update: Vec<IVec2> = self
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
            if let Some(chunk) = self.chunks.get_mut(pos) {
                chunk.set_simulation_active(false);
            }
        }

        for pos in &chunks_to_update {
            self.update_chunk_ca(*pos, stats);
        }

        // 3. Temperature diffusion (30fps throttled) - active chunks only
        self.temperature_sim
            .update(&mut self.chunks, &self.active_chunks);

        // 4. Light propagation (15fps throttled) - active chunks only
        self.light_time_accumulator += 1.0 / 60.0; // Fixed timestep
        if self.light_time_accumulator >= LIGHT_TIMESTEP {
            let sky_light = self.calculate_sky_light();
            self.light_propagation.propagate_light(
                &mut self.chunks,
                &self.materials,
                sky_light,
                &self.active_chunks,
            );
            self.light_time_accumulator -= LIGHT_TIMESTEP;
        }

        // 5. State changes based on temperature
        for i in 0..self.active_chunks.len() {
            let pos = self.active_chunks[i];
            self.check_chunk_state_changes(pos, stats);
        }

        // 6. Process structural integrity checks
        let positions = self.structural_system.drain_queue();
        let checks_processed = StructuralIntegritySystem::process_checks(self, positions);
        if checks_processed > 0 {
            log::debug!("Processed {} structural integrity checks", checks_processed);
        }

        // 7. Update rigid body physics
        self.physics_world.step();

        // 8. Check for settled debris and reconstruct as pixels
        let settled = self.physics_world.get_settled_debris();
        for handle in settled {
            self.reconstruct_debris(handle);
        }

        // 9. Resource regeneration (fruit spawning)
        self.regeneration_system
            .update(&mut self.chunks, &self.active_chunks, 1.0 / 60.0);

        // 10. Update creatures (sensing, planning, neural control)
        // Temporarily take creature_manager and physics_world to avoid borrow checker issues
        let mut creature_manager = std::mem::replace(
            &mut self.creature_manager,
            crate::creature::spawning::CreatureManager::new(0), // Dummy placeholder
        );
        let mut physics_world = std::mem::replace(
            &mut self.physics_world,
            crate::physics::PhysicsWorld::empty(), // Lightweight placeholder
        );

        creature_manager.update(1.0 / 60.0, self, &mut physics_world);

        // 11. Execute creature actions (eat, mine, build)
        creature_manager.execute_actions(self, 1.0 / 60.0);

        // Put them back
        self.creature_manager = creature_manager;
        self.physics_world = physics_world;
    }

    /// Calculate sky light level based on day/night cycle (0-15)
    /// Day/night cycle: 0-1200 seconds (20 min real-time = 24 hours game-time)
    /// 0s = midnight, 300s = dawn, 600s = noon, 900s = dusk, 1200s = midnight
    fn calculate_sky_light(&self) -> u8 {
        const DAY_NIGHT_CYCLE_DURATION: f32 = 1200.0;

        // Convert time to angle (0-2π)
        let angle = (self.day_night_time / DAY_NIGHT_CYCLE_DURATION) * 2.0 * std::f32::consts::PI;

        // Cosine wave: -1 (midnight) to 1 (noon)
        // Shift so 0s = midnight (cos(0) = 1, we want -1)
        let cosine = -(angle.cos());

        // Map -1..1 to 0..15
        // -1 (midnight) → 0, 0 (dawn/dusk) → 7.5, 1 (noon) → 15
        let normalized = (cosine + 1.0) / 2.0; // 0..1
        (normalized * 15.0) as u8
    }

    /// Initialize light levels before first CA update
    /// This ensures that light_levels are valid before reactions start checking them
    fn initialize_light(&mut self) {
        let sky_light = self.calculate_sky_light();
        self.light_propagation.propagate_light(
            &mut self.chunks,
            &self.materials,
            sky_light,
            &self.active_chunks,
        );
        log::info!("Initialized light propagation (sky_light={})", sky_light);
    }

    /// Get growth progress as percentage (0-100) through the 10-second cycle
    pub fn get_growth_progress_percent(&self) -> f32 {
        (self.growth_timer / 10.0) * 100.0
    }

    /// Check if a chunk needs CA update based on dirty state of itself and neighbors
    /// Returns true if this chunk or any of its 8 neighbors have dirty_rect set or simulation_active
    fn chunk_needs_ca_update(&self, pos: IVec2) -> bool {
        // Check the chunk itself
        if let Some(chunk) = self.chunks.get(&pos)
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
                if let Some(neighbor) = self.chunks.get(&neighbor_pos)
                    && (neighbor.dirty_rect.is_some() || neighbor.is_simulation_active())
                {
                    return true;
                }
            }
        }

        false
    }

    fn update_chunk_ca(&mut self, chunk_pos: IVec2, stats: &mut dyn crate::world::SimStats) {
        // Update from bottom to top so falling works correctly
        for y in 0..CHUNK_SIZE {
            // Alternate direction each row for symmetry
            let x_iter: Box<dyn Iterator<Item = usize>> = if y % 2 == 0 {
                Box::new(0..CHUNK_SIZE)
            } else {
                Box::new((0..CHUNK_SIZE).rev())
            };

            for x in x_iter {
                self.update_pixel(chunk_pos, x, y, stats);
            }
        }
    }

    fn update_pixel(
        &mut self,
        chunk_pos: IVec2,
        x: usize,
        y: usize,
        stats: &mut dyn crate::world::SimStats,
    ) {
        let chunk = match self.chunks.get(&chunk_pos) {
            Some(c) => c,
            None => return,
        };

        let pixel = chunk.get_pixel(x, y);
        if pixel.is_empty() {
            return;
        }

        // Special handling for fire
        if pixel.material_id == MaterialId::FIRE {
            self.update_fire(chunk_pos, x, y, stats);
            return;
        }

        // Check if pixel should ignite (before movement)
        if pixel.flags & pixel_flags::BURNING == 0 {
            self.check_ignition(chunk_pos, x, y);
        }

        // Update burning materials
        if pixel.flags & pixel_flags::BURNING != 0 {
            self.update_burning_material(chunk_pos, x, y);
        }

        // Get material type for movement logic
        let material_type = self.materials.get(pixel.material_id).material_type;

        // Normal CA movement
        match material_type {
            MaterialType::Powder => {
                self.update_powder(chunk_pos, x, y, stats);
            }
            MaterialType::Liquid => {
                self.update_liquid(chunk_pos, x, y, stats);
            }
            MaterialType::Gas => {
                self.update_gas(chunk_pos, x, y, stats);
            }
            MaterialType::Solid => {
                // Solids don't move
            }
        }

        // Check reactions with neighbors (after movement)
        self.check_pixel_reactions(chunk_pos, x, y, stats);
    }

    fn update_powder(
        &mut self,
        chunk_pos: IVec2,
        x: usize,
        y: usize,
        stats: &mut dyn crate::world::SimStats,
    ) {
        // Convert to world coordinates
        let world_x = chunk_pos.x * CHUNK_SIZE as i32 + x as i32;
        let world_y = chunk_pos.y * CHUNK_SIZE as i32 + y as i32;

        // Try to fall down
        if self.try_move_world(world_x, world_y, world_x, world_y - 1, stats) {
            return;
        }

        // Try to fall diagonally (randomized for symmetry)
        let try_left_first = rand::random::<bool>();

        if try_left_first {
            if self.try_move_world(world_x, world_y, world_x - 1, world_y - 1, stats) {
                return;
            }
            if self.try_move_world(world_x, world_y, world_x + 1, world_y - 1, stats) {}
        } else {
            if self.try_move_world(world_x, world_y, world_x + 1, world_y - 1, stats) {
                return;
            }
            if self.try_move_world(world_x, world_y, world_x - 1, world_y - 1, stats) {}
        }
    }

    fn update_liquid(
        &mut self,
        chunk_pos: IVec2,
        x: usize,
        y: usize,
        stats: &mut dyn crate::world::SimStats,
    ) {
        // Convert to world coordinates
        let world_x = chunk_pos.x * CHUNK_SIZE as i32 + x as i32;
        let world_y = chunk_pos.y * CHUNK_SIZE as i32 + y as i32;

        // Try to fall down
        if self.try_move_world(world_x, world_y, world_x, world_y - 1, stats) {
            return;
        }

        // Try to fall diagonally
        let try_left_first = rand::random::<bool>();

        if try_left_first {
            if self.try_move_world(world_x, world_y, world_x - 1, world_y - 1, stats) {
                return;
            }
            if self.try_move_world(world_x, world_y, world_x + 1, world_y - 1, stats) {
                return;
            }
        } else {
            if self.try_move_world(world_x, world_y, world_x + 1, world_y - 1, stats) {
                return;
            }
            if self.try_move_world(world_x, world_y, world_x - 1, world_y - 1, stats) {
                return;
            }
        }

        // Try to flow horizontally
        if try_left_first {
            if self.try_move_world(world_x, world_y, world_x - 1, world_y, stats) {
                return;
            }
            if self.try_move_world(world_x, world_y, world_x + 1, world_y, stats) {}
        } else {
            if self.try_move_world(world_x, world_y, world_x + 1, world_y, stats) {
                return;
            }
            if self.try_move_world(world_x, world_y, world_x - 1, world_y, stats) {}
        }
    }

    fn update_gas(
        &mut self,
        chunk_pos: IVec2,
        x: usize,
        y: usize,
        stats: &mut dyn crate::world::SimStats,
    ) {
        // Convert to world coordinates
        let world_x = chunk_pos.x * CHUNK_SIZE as i32 + x as i32;
        let world_y = chunk_pos.y * CHUNK_SIZE as i32 + y as i32;

        // Gases rise (positive Y)
        if self.try_move_world(world_x, world_y, world_x, world_y + 1, stats) {
            return;
        }

        // Try to rise diagonally
        let try_left_first = rand::random::<bool>();

        if try_left_first {
            if self.try_move_world(world_x, world_y, world_x - 1, world_y + 1, stats) {
                return;
            }
            if self.try_move_world(world_x, world_y, world_x + 1, world_y + 1, stats) {
                return;
            }
        } else {
            if self.try_move_world(world_x, world_y, world_x + 1, world_y + 1, stats) {
                return;
            }
            if self.try_move_world(world_x, world_y, world_x - 1, world_y + 1, stats) {
                return;
            }
        }

        // Disperse horizontally
        if try_left_first {
            if self.try_move_world(world_x, world_y, world_x - 1, world_y, stats) {
                return;
            }
            if self.try_move_world(world_x, world_y, world_x + 1, world_y, stats) {}
        } else {
            if self.try_move_world(world_x, world_y, world_x + 1, world_y, stats) {
                return;
            }
            if self.try_move_world(world_x, world_y, world_x - 1, world_y, stats) {}
        }
    }

    /// Try to move a pixel, returns true if successful
    #[allow(dead_code)]
    fn try_move(
        &mut self,
        chunk_pos: IVec2,
        from_x: usize,
        from_y: usize,
        to_x: usize,
        to_y: usize,
    ) -> bool {
        // TODO: Handle cross-chunk movement
        let chunk = match self.chunks.get(&chunk_pos) {
            Some(c) => c,
            None => return false,
        };

        let target = chunk.get_pixel(to_x, to_y);

        // Can only move into empty space (for now)
        // TODO: Handle density-based displacement (water sinks under oil, etc.)
        if target.is_empty() {
            let chunk = self.chunks.get_mut(&chunk_pos).unwrap();
            chunk.swap_pixels(from_x, from_y, to_x, to_y);
            true
        } else {
            false
        }
    }

    /// Try to move a pixel using world coordinates (handles cross-chunk movement)
    fn try_move_world(
        &mut self,
        from_world_x: i32,
        from_world_y: i32,
        to_world_x: i32,
        to_world_y: i32,
        stats: &mut dyn crate::world::SimStats,
    ) -> bool {
        // Convert to chunk coordinates
        let (src_chunk_pos, src_x, src_y) = Self::world_to_chunk_coords(from_world_x, from_world_y);
        let (dst_chunk_pos, dst_x, dst_y) = Self::world_to_chunk_coords(to_world_x, to_world_y);

        // Phase 1: Read pixels (immutable borrows)
        let src_pixel = match self.chunks.get(&src_chunk_pos) {
            Some(c) => c.get_pixel(src_x, src_y),
            None => return false, // Source chunk not loaded
        };

        let dst_pixel = match self.chunks.get(&dst_chunk_pos) {
            Some(c) => c.get_pixel(dst_x, dst_y),
            None => return false, // Destination chunk not loaded
        };

        // Can only move into empty space (for now)
        // TODO: Handle density-based displacement (water sinks under oil, etc.)
        if !dst_pixel.is_empty() {
            return false;
        }

        // Phase 2: Write pixels (mutable borrows)
        if src_chunk_pos == dst_chunk_pos {
            // Same chunk - use swap for efficiency
            if let Some(chunk) = self.chunks.get_mut(&src_chunk_pos) {
                chunk.swap_pixels(src_x, src_y, dst_x, dst_y);
                chunk.set_simulation_active(true);
                stats.record_pixel_moved();
                return true;
            }
        } else {
            // Different chunks - sequential writes to avoid borrow checker issues
            // First, clear source
            if let Some(src_chunk) = self.chunks.get_mut(&src_chunk_pos) {
                src_chunk.set_pixel(src_x, src_y, Pixel::AIR);
                src_chunk.set_simulation_active(true);
            } else {
                return false;
            }

            // Then, set destination
            if let Some(dst_chunk) = self.chunks.get_mut(&dst_chunk_pos) {
                dst_chunk.set_pixel(dst_x, dst_y, src_pixel);
                dst_chunk.set_simulation_active(true);
                stats.record_pixel_moved();
                return true;
            } else {
                // Rollback: restore source pixel
                if let Some(src_chunk) = self.chunks.get_mut(&src_chunk_pos) {
                    src_chunk.set_pixel(src_x, src_y, src_pixel);
                }
                return false;
            }
        }

        false
    }

    /// Get pixel at world coordinates
    pub fn get_pixel(&self, world_x: i32, world_y: i32) -> Option<Pixel> {
        let (chunk_pos, local_x, local_y) = Self::world_to_chunk_coords(world_x, world_y);
        self.chunks
            .get(&chunk_pos)
            .map(|c| c.get_pixel(local_x, local_y))
    }

    /// Ensure chunks exist for a given pixel area (creates empty chunks if needed)
    /// Used by headless training to set up scenarios without full world generation
    pub fn ensure_chunks_for_area(&mut self, min_x: i32, min_y: i32, max_x: i32, max_y: i32) {
        let (min_chunk, _, _) = Self::world_to_chunk_coords(min_x, min_y);
        let (max_chunk, _, _) = Self::world_to_chunk_coords(max_x, max_y);

        for cy in min_chunk.y..=max_chunk.y {
            for cx in min_chunk.x..=max_chunk.x {
                let pos = IVec2::new(cx, cy);
                self.chunks.entry(pos).or_insert_with(|| Chunk::new(cx, cy));
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
        let (chunk_pos, local_x, local_y) = Self::world_to_chunk_coords(world_x, world_y);
        if let Some(chunk) = self.chunks.get_mut(&chunk_pos) {
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
        let (chunk_pos, local_x, local_y) = Self::world_to_chunk_coords(world_x, world_y);
        if let Some(chunk) = self.chunks.get(&chunk_pos) {
            get_temperature_at_pixel(chunk, local_x, local_y)
        } else {
            20.0 // Default ambient temperature
        }
    }

    /// Get light level at world coordinates (0-15)
    pub fn get_light_at(&self, world_x: i32, world_y: i32) -> Option<u8> {
        let (chunk_pos, local_x, local_y) = Self::world_to_chunk_coords(world_x, world_y);
        self.chunks
            .get(&chunk_pos)
            .map(|c| c.get_light(local_x, local_y))
    }

    /// Set light level at world coordinates (0-15)
    pub fn set_light_at(&mut self, world_x: i32, world_y: i32, level: u8) {
        let (chunk_pos, local_x, local_y) = Self::world_to_chunk_coords(world_x, world_y);
        if let Some(chunk) = self.chunks.get_mut(&chunk_pos) {
            chunk.set_light(local_x, local_y, level);
        }
    }

    /// Get material ID at world coordinates
    pub fn get_pixel_material(&self, world_x: i32, world_y: i32) -> Option<u16> {
        self.get_pixel(world_x, world_y).map(|p| p.material_id)
    }

    /// Convert world coordinates to chunk coordinates + local offset
    fn world_to_chunk_coords(world_x: i32, world_y: i32) -> (IVec2, usize, usize) {
        let chunk_x = world_x.div_euclid(CHUNK_SIZE as i32);
        let chunk_y = world_y.div_euclid(CHUNK_SIZE as i32);
        let local_x = world_x.rem_euclid(CHUNK_SIZE as i32) as usize;
        let local_y = world_y.rem_euclid(CHUNK_SIZE as i32) as usize;
        (IVec2::new(chunk_x, chunk_y), local_x, local_y)
    }

    /// Get iterator over active chunks
    pub fn active_chunks(&self) -> impl Iterator<Item = &Chunk> {
        self.active_chunks
            .iter()
            .filter_map(|pos| self.chunks.get(pos))
    }

    /// Get positions of active chunks
    pub fn active_chunk_positions(&self) -> &[IVec2] {
        &self.active_chunks
    }

    /// Get all loaded chunks
    pub fn chunks(&self) -> &HashMap<IVec2, Chunk> {
        &self.chunks
    }

    /// Get materials registry
    pub fn materials(&self) -> &Materials {
        &self.materials
    }

    /// Get active debris for rendering
    pub fn get_active_debris(&self) -> Vec<crate::physics::DebrisRenderData> {
        self.physics_world.get_debris_render_data()
    }

    /// Get creature render data for rendering
    pub fn get_creature_render_data(&self) -> Vec<crate::creature::CreatureRenderData> {
        self.creature_manager.get_render_data(&self.physics_world)
    }

    /// Add bedrock collider for a chunk (called when chunk is loaded)
    pub fn add_bedrock_collider(&mut self, chunk_x: i32, chunk_y: i32) {
        self.physics_world.add_bedrock_collider(chunk_x, chunk_y);
    }

    /// Create falling debris from a pixel region
    /// Removes pixels from world and creates a rigid body
    pub fn create_debris(
        &mut self,
        region: std::collections::HashSet<IVec2>,
    ) -> rapier2d::dynamics::RigidBodyHandle {
        log::info!("Creating debris from {} pixels", region.len());

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
            log::warn!("No valid pixels to create debris");
            // Return a dummy handle (this shouldn't happen in practice)
            return rapier2d::dynamics::RigidBodyHandle::from_raw_parts(0, 0);
        }

        // Remove pixels from world (convert to air)
        for pos in &region {
            self.set_pixel_direct(pos.x, pos.y, MaterialId::AIR);
        }

        // Create rigid body in physics world
        let handle = self.physics_world.create_debris(pixels);
        log::debug!("Created rigid body handle: {:?}", handle);
        handle
    }

    /// Set pixel without triggering structural checks (internal use)
    fn set_pixel_direct(&mut self, world_x: i32, world_y: i32, material_id: u16) {
        let (chunk_pos, local_x, local_y) = Self::world_to_chunk_coords(world_x, world_y);
        if let Some(chunk) = self.chunks.get_mut(&chunk_pos) {
            chunk.set_material(local_x, local_y, material_id);
        }
    }

    /// Set pixel without triggering structural checks, returns success/failure
    fn set_pixel_direct_checked(&mut self, world_x: i32, world_y: i32, material_id: u16) -> bool {
        let (chunk_pos, local_x, local_y) = Self::world_to_chunk_coords(world_x, world_y);
        if let Some(chunk) = self.chunks.get_mut(&chunk_pos) {
            chunk.set_material(local_x, local_y, material_id);
            true
        } else {
            log::trace!(
                "set_pixel_direct_checked: chunk {:?} not loaded for pixel at ({}, {})",
                chunk_pos,
                world_x,
                world_y
            );
            false
        }
    }

    /// Reconstruct debris that has settled as static pixels
    fn reconstruct_debris(&mut self, handle: rapier2d::dynamics::RigidBodyHandle) {
        log::info!("Reconstructing debris: handle={:?}", handle);

        // Get final position BEFORE removing debris
        let (final_center, rotation) = match self.physics_world.get_debris_transform(handle) {
            Some(transform) => transform,
            None => {
                log::warn!("Failed to get debris transform {:?}", handle);
                return;
            }
        };

        // Get debris from physics world (removes it)
        let debris = match self.physics_world.remove_debris(handle) {
            Some(d) => d,
            None => {
                log::warn!("Failed to remove debris {:?}", handle);
                return;
            }
        };

        log::debug!(
            "Reconstructing {} pixels at ({:.1}, {:.1}), rotation={:.2}°",
            debris.pixels.len(),
            final_center.x,
            final_center.y,
            rotation.to_degrees()
        );

        // Place pixels relative to new center position
        let mut placed_count = 0;
        let mut failed_count = 0;

        for (relative_pos, material_id) in debris.pixels.iter() {
            // Apply rotation (currently ignored - can be added later)
            let rotated_x = relative_pos.x as f32;
            let rotated_y = relative_pos.y as f32;

            // Translate to world position
            let world_x = (final_center.x + rotated_x).round() as i32;
            let world_y = (final_center.y + rotated_y).round() as i32;

            // Place pixel
            if self.set_pixel_direct_checked(world_x, world_y, *material_id) {
                placed_count += 1;
            } else {
                failed_count += 1;
            }
        }

        if failed_count > 0 {
            log::warn!(
                "Reconstruction: {} pixels placed, {} failed (chunk not loaded?)",
                placed_count,
                failed_count
            );
        } else {
            log::info!(
                "Reconstruction: {} pixels placed successfully",
                placed_count
            );
        }
    }

    /// Clear all chunks from the world
    pub fn clear_all_chunks(&mut self) {
        self.chunks.clear();
        self.active_chunks.clear();
        log::info!("Cleared all chunks");
    }

    /// Add a chunk to the world
    pub fn add_chunk(&mut self, chunk: Chunk) {
        let pos = IVec2::new(chunk.x, chunk.y);

        // Check if this chunk contains bedrock
        let has_bedrock = chunk
            .pixels()
            .iter()
            .any(|p| p.material_id == MaterialId::BEDROCK);

        self.chunks.insert(pos, chunk);

        // Add to active chunks if within range of player
        let dist_x = (pos.x - (self.player.position.x as i32 / CHUNK_SIZE as i32)).abs();
        let dist_y = (pos.y - (self.player.position.y as i32 / CHUNK_SIZE as i32)).abs();
        if dist_x <= Self::ACTIVE_CHUNK_RADIUS
            && dist_y <= Self::ACTIVE_CHUNK_RADIUS
            && !self.active_chunks.contains(&pos)
        {
            self.active_chunks.push(pos);
        }

        // Add physics collider for bedrock chunks
        if has_bedrock {
            self.add_bedrock_collider(pos.x, pos.y);
        }
    }

    /// Initialize persistent world (load or generate)
    pub fn load_persistent_world(&mut self) {
        // Reset demo mode when returning to persistent world
        self.demo_mode = false;

        // Clear any existing chunks (from test world generation)
        self.clear_all_chunks();

        let persistence =
            ChunkPersistence::new("default").expect("Failed to create chunk persistence");

        let metadata = persistence.load_metadata();

        self.generator = WorldGenerator::new(metadata.seed);

        // Restore player data if it exists, otherwise use spawn point
        if let Some(saved_player) = metadata.player_data {
            self.player = saved_player;
            log::info!(
                "Restored player data: inventory={}/{} slots, health={:.0}/{:.0}, hunger={:.0}/{:.0}",
                self.player.inventory.used_slot_count(),
                self.player.inventory.max_slots,
                self.player.health.current,
                self.player.health.max,
                self.player.hunger.current,
                self.player.hunger.max
            );
        } else {
            // New world - set player at spawn point
            self.player.position = glam::Vec2::new(metadata.spawn_point.0, metadata.spawn_point.1);
            log::info!("New world - player spawned at {:?}", self.player.position);
        }

        self.persistence = Some(persistence);

        // Load initial chunks around spawn
        self.load_chunks_around_player();

        // Initialize light levels before first CA update
        self.initialize_light();

        log::info!("Loaded persistent world (seed: {})", metadata.seed);
    }

    /// Disable persistence for demo levels
    /// This prevents dynamic chunk loading from overwriting demo level chunks
    pub fn disable_persistence(&mut self) {
        self.persistence = None;
        self.demo_mode = true;
        log::info!("Persistence disabled for demo mode");
    }

    /// Load chunks within active radius of player (17x17 = 289 chunks)
    fn load_chunks_around_player(&mut self) {
        let player_chunk_x = (self.player.position.x as i32).div_euclid(CHUNK_SIZE as i32);
        let player_chunk_y = (self.player.position.y as i32).div_euclid(CHUNK_SIZE as i32);

        for cy in (player_chunk_y - 8)..=(player_chunk_y + 8) {
            for cx in (player_chunk_x - 8)..=(player_chunk_x + 8) {
                self.load_or_generate_chunk(cx, cy);
            }
        }
    }

    /// Load nearby chunks dynamically as player moves (called when entering new chunk)
    fn load_nearby_chunks(&mut self) {
        // Don't auto-load chunks in demo mode - use only chunks the demo level created
        if self.demo_mode {
            return;
        }

        let player_chunk_x = (self.player.position.x as i32).div_euclid(CHUNK_SIZE as i32);
        let player_chunk_y = (self.player.position.y as i32).div_euclid(CHUNK_SIZE as i32);

        // Load chunks within 20-chunk radius (ensures chunks loaded beyond texture edge)
        const LOAD_RADIUS: i32 = 20;

        for cy in (player_chunk_y - LOAD_RADIUS)..=(player_chunk_y + LOAD_RADIUS) {
            for cx in (player_chunk_x - LOAD_RADIUS)..=(player_chunk_x + LOAD_RADIUS) {
                self.load_or_generate_chunk(cx, cy);
            }
        }
    }

    /// Load or generate a chunk at the given coordinates
    fn load_or_generate_chunk(&mut self, chunk_x: i32, chunk_y: i32) {
        let pos = IVec2::new(chunk_x, chunk_y);

        if self.chunks.contains_key(&pos) {
            log::trace!(
                "[LOAD] Chunk ({}, {}) already loaded, skipping",
                chunk_x,
                chunk_y
            );
            return; // Already loaded
        }

        let chunk = if let Some(persistence) = &self.persistence {
            log::debug!(
                "[LOAD] Requesting chunk ({}, {}) from persistence",
                chunk_x,
                chunk_y
            );
            persistence.load_chunk(chunk_x, chunk_y, &self.generator)
        } else {
            // Demo mode: use generator without saving
            log::debug!(
                "[GEN] Demo mode: generating chunk ({}, {}) without persistence",
                chunk_x,
                chunk_y
            );
            self.generator.generate_chunk(chunk_x, chunk_y)
        };

        let non_air = chunk.count_non_air();
        log::debug!(
            "[LOAD] Adding chunk ({}, {}) to world - {} non-air pixels",
            chunk_x,
            chunk_y,
            non_air
        );

        self.add_chunk(chunk);

        // LRU eviction if too many chunks loaded
        if self.chunks.len() > self.loaded_chunk_limit {
            self.evict_distant_chunks();
        }
    }

    /// Save and unload chunks far from player
    fn evict_distant_chunks(&mut self) {
        let player_chunk_x = (self.player.position.x as i32).div_euclid(CHUNK_SIZE as i32);
        let player_chunk_y = (self.player.position.y as i32).div_euclid(CHUNK_SIZE as i32);

        let mut to_evict = Vec::new();

        for pos in self.chunks.keys() {
            let dist_x = (pos.x - player_chunk_x).abs();
            let dist_y = (pos.y - player_chunk_y).abs();

            // Unload chunks >10 chunks away
            if dist_x > 10 || dist_y > 10 {
                to_evict.push(*pos);
            }
        }

        for pos in to_evict {
            if let Some(chunk) = self.chunks.remove(&pos)
                && chunk.dirty
                && let Some(persistence) = &self.persistence
            {
                if let Err(e) = persistence.save_chunk(&chunk) {
                    log::error!("Failed to save chunk ({}, {}): {}", pos.x, pos.y, e);
                } else {
                    log::debug!("Saved and evicted chunk ({}, {})", pos.x, pos.y);
                }
            }
        }
    }

    /// Save all dirty chunks (periodic auto-save)
    pub fn save_dirty_chunks(&mut self) {
        if let Some(persistence) = &self.persistence {
            let mut saved_count = 0;
            let total_dirty = self.chunks.values().filter(|c| c.dirty).count();

            if total_dirty > 0 {
                log::debug!("[SAVE] Starting auto-save of {} dirty chunks", total_dirty);
            }

            for chunk in self.chunks.values_mut() {
                if chunk.dirty {
                    let non_air = chunk.count_non_air();
                    log::debug!(
                        "[SAVE] Saving dirty chunk ({}, {}) - {} non-air pixels",
                        chunk.x,
                        chunk.y,
                        non_air
                    );

                    if let Err(e) = persistence.save_chunk(chunk) {
                        log::error!(
                            "[SAVE] Failed to save chunk ({}, {}): {}",
                            chunk.x,
                            chunk.y,
                            e
                        );
                    } else {
                        chunk.dirty = false;
                        saved_count += 1;
                    }
                }
            }

            if saved_count > 0 {
                log::info!("[SAVE] Auto-saved {} dirty chunks", saved_count);
            }
        }
    }

    /// Save all chunks and metadata (manual save)
    pub fn save_all_dirty_chunks(&mut self) {
        self.save_dirty_chunks();

        // Also save metadata with player data
        if let Some(persistence) = &self.persistence {
            #[cfg(not(target_arch = "wasm32"))]
            let last_played = chrono::Local::now().to_rfc3339();
            #[cfg(target_arch = "wasm32")]
            let last_played = "WASM Session".to_string();

            let metadata = WorldMetadata {
                version: 1,
                seed: self.generator.seed,
                spawn_point: (self.player.position.x, self.player.position.y),
                created_at: String::new(), // Preserved from load
                last_played,
                play_time_seconds: 0,                   // TODO: track play time
                player_data: Some(self.player.clone()), // Save player inventory, health, hunger
            };

            if let Err(e) = persistence.save_metadata(&metadata) {
                log::error!("Failed to save world metadata: {}", e);
            } else {
                log::info!("Saved world metadata with player data");
            }
        }
    }

    /// Check all pixels in a chunk for state changes based on temperature
    fn check_chunk_state_changes(
        &mut self,
        chunk_pos: IVec2,
        stats: &mut dyn crate::world::SimStats,
    ) {
        let chunk = match self.chunks.get_mut(&chunk_pos) {
            Some(c) => c,
            None => return,
        };

        for y in 0..CHUNK_SIZE {
            for x in 0..CHUNK_SIZE {
                let pixel = chunk.get_pixel(x, y);
                if pixel.is_empty() {
                    continue;
                }

                let material = self.materials.get(pixel.material_id);
                let temp = get_temperature_at_pixel(chunk, x, y);

                let mut new_pixel = pixel;
                if StateChangeSystem::check_state_change(&mut new_pixel, material, temp) {
                    chunk.set_pixel(x, y, new_pixel);
                    stats.record_state_change();
                }
            }
        }
    }

    /// Update fire pixel behavior
    fn update_fire(
        &mut self,
        chunk_pos: IVec2,
        x: usize,
        y: usize,
        stats: &mut dyn crate::world::SimStats,
    ) {
        // 1. Add heat to temperature field
        if let Some(chunk) = self.chunks.get_mut(&chunk_pos) {
            add_heat_at_pixel(chunk, x, y, 50.0); // Fire adds significant heat
        }

        // 2. Fire behaves like gas (rises)
        self.update_gas(chunk_pos, x, y, stats);

        // 3. Fire has limited lifetime - random chance to become smoke
        if rand::random::<f32>() < 0.02 {
            let world_x = chunk_pos.x * CHUNK_SIZE as i32 + x as i32;
            let world_y = chunk_pos.y * CHUNK_SIZE as i32 + y as i32;
            self.set_pixel(world_x, world_y, MaterialId::SMOKE);
        }
    }

    /// Check if a pixel should ignite based on temperature
    fn check_ignition(&mut self, chunk_pos: IVec2, x: usize, y: usize) {
        let chunk = match self.chunks.get(&chunk_pos) {
            Some(c) => c,
            None => return,
        };

        let pixel = chunk.get_pixel(x, y);
        let material = self.materials.get(pixel.material_id);

        if !material.flammable {
            return;
        }

        let temp = get_temperature_at_pixel(chunk, x, y);

        if let Some(ignition_temp) = material.ignition_temp
            && temp >= ignition_temp
        {
            // Mark pixel as burning
            let chunk = self.chunks.get_mut(&chunk_pos).unwrap();
            let mut new_pixel = pixel;
            new_pixel.flags |= pixel_flags::BURNING;
            chunk.set_pixel(x, y, new_pixel);

            // Try to spawn fire in adjacent air cell
            let world_x = chunk_pos.x * CHUNK_SIZE as i32 + x as i32;
            let world_y = chunk_pos.y * CHUNK_SIZE as i32 + y as i32;

            for (dx, dy) in [(0, 1), (1, 0), (-1, 0), (0, -1)] {
                if let Some(neighbor) = self.get_pixel(world_x + dx, world_y + dy)
                    && neighbor.is_empty()
                {
                    self.set_pixel(world_x + dx, world_y + dy, MaterialId::FIRE);
                    break;
                }
            }
        }
    }

    /// Update burning material (gradual consumption)
    fn update_burning_material(&mut self, chunk_pos: IVec2, x: usize, y: usize) {
        let chunk = match self.chunks.get(&chunk_pos) {
            Some(c) => c,
            None => return,
        };

        let pixel = chunk.get_pixel(x, y);
        let material = self.materials.get(pixel.material_id);

        // Probability check - material burns gradually
        if rand::random::<f32>() < material.burn_rate {
            let world_x = chunk_pos.x * CHUNK_SIZE as i32 + x as i32;
            let world_y = chunk_pos.y * CHUNK_SIZE as i32 + y as i32;

            // Transform to burns_to material (or air if not specified)
            let new_material = material.burns_to.unwrap_or(MaterialId::AIR);
            self.set_pixel(world_x, world_y, new_material);

            // Add heat from burning
            if let Some(chunk) = self.chunks.get_mut(&chunk_pos) {
                add_heat_at_pixel(chunk, x, y, 20.0);
            }
        }
    }

    /// Check for chemical reactions with neighboring pixels
    fn check_pixel_reactions(
        &mut self,
        chunk_pos: IVec2,
        x: usize,
        y: usize,
        stats: &mut dyn crate::world::SimStats,
    ) {
        use crate::simulation::MaterialId;

        let chunk = match self.chunks.get(&chunk_pos) {
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

        // Debug logging for plant matter growth conditions
        if pixel.material_id == MaterialId::PLANT_MATTER {
            // Check if there's water nearby
            let mut has_water = false;
            for (dx, dy) in [(0, 1), (1, 0), (0, -1), (-1, 0)] {
                if let Some(neighbor) = self.get_pixel(world_x + dx, world_y + dy)
                    && neighbor.material_id == MaterialId::WATER
                {
                    has_water = true;
                    break;
                }
            }

            // Log growth check conditions (throttled - only once every 600 frames ≈ 10 seconds)
            static mut LAST_LOG_FRAME: u32 = 0;
            static mut FRAME_COUNT: u32 = 0;
            unsafe {
                FRAME_COUNT += 1;
                if FRAME_COUNT - LAST_LOG_FRAME > 600 {
                    log::debug!(
                        "Plant growth check at ({}, {}): light={}, temp={:.1}°C, water={}, ready={}",
                        world_x,
                        world_y,
                        light_level,
                        temp,
                        has_water,
                        light_level >= 8 && (10.0..=40.0).contains(&temp) && has_water
                    );
                    LAST_LOG_FRAME = FRAME_COUNT;
                }
            }
        }

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
                if rand::random::<f32>() < reaction.probability {
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
        let (chunk_pos, local_x, local_y) = Self::world_to_chunk_coords(x, y);
        if let Some(chunk) = self.chunks.get(&chunk_pos) {
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
