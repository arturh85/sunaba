//! World - manages chunks and simulation

use glam::{IVec2, Vec2};
use std::collections::HashMap;

#[cfg(not(feature = "client"))]
use std::time::Instant;
#[cfg(feature = "client")]
use web_time::Instant;

use super::ca_update::CellularAutomataUpdater;
use super::chemistry_system::ChemistrySystem;
use super::chunk_manager::ChunkManager;
use super::chunk_status::ChunkStatus;
use super::collision::CollisionDetector;
use super::debris_system::DebrisSystem;
use super::light_system::LightSystem;
use super::mining_system::MiningSystem;
use super::persistence_system::PersistenceSystem;
use super::pixel_queries::PixelQueries;
use super::player_physics::PlayerPhysicsSystem;
use super::raycasting::Raycasting;
use super::{CHUNK_SIZE, Chunk, Pixel, pixel_flags};
use crate::entity::crafting::RecipeRegistry;
use crate::entity::player::Player;
use crate::entity::tools::ToolRegistry;
use crate::simulation::{
    ChunkRenderData, FallingChunk, MaterialId, MaterialType, Materials, ReactionRegistry,
    RegenerationSystem, StructuralIntegritySystem, TemperatureSimulator, WorldCollisionQuery,
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

    /// Session start time (for play time tracking)
    session_start: Instant,

    /// Total play time in seconds (accumulated across sessions)
    pub total_play_time_seconds: u64,
}

impl World {
    pub fn new(#[allow(unused_variables)] skip_initial_creatures: bool) -> Self {
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
            session_start: Instant::now(),
            total_play_time_seconds: 0,
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

        // Spawn 3 test creatures near spawn point with spacing (unless skipped for multiplayer)
        #[cfg(feature = "evolution")]
        if !skip_initial_creatures {
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

    /// Update the world generator config and regenerate all chunks
    pub fn update_generator_config(&mut self, config: super::worldgen_config::WorldGenConfig) {
        // Update the generator config
        self.persistence_system.update_generator_config(config);

        // Clear all chunks so they regenerate with new config
        self.persistence_system
            .clear_all_chunks(&mut self.chunk_manager);

        // Reload chunks around player
        let player_pos = self.player.position;
        self.persistence_system
            .load_chunks_around_player(&mut self.chunk_manager, player_pos);

        log::info!("World regenerated with new config");
    }

    /// Get the current generator config
    pub fn generator_config(&self) -> &super::worldgen_config::WorldGenConfig {
        self.persistence_system.generator_config()
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
    /// Note: Used via closure in PlayerPhysicsSystem::update() - compiler doesn't see this usage
    #[allow(dead_code)]
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
    /// Note: Used via closure in PlayerPhysicsSystem::update() - compiler doesn't see this usage
    #[allow(dead_code)]
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

        Raycasting::raycast_filtered(
            &self.chunk_manager,
            &self.materials,
            from,
            direction,
            radius,
            max_distance,
            MaterialType::Solid,
        )
    }

    /// Player movement speed in pixels per second
    const PLAYER_SPEED: f32 = 200.0;

    /// Active chunk simulation radius (chunks from player)
    const ACTIVE_CHUNK_RADIUS: i32 = 3; // 7×7 grid = 49 chunks

    /// Update player position based on input with gravity and jump
    pub fn update_player(&mut self, input: &crate::entity::InputState, dt: f32) {
        // Extract references to avoid borrow checker issues
        let chunks = &self.chunk_manager.chunks;
        let materials = &self.materials;
        let player_pos = self.player.position;
        let player_width = crate::entity::player::Player::WIDTH;
        let player_height = crate::entity::player::Player::HEIGHT;

        PlayerPhysicsSystem::update(
            &mut self.player,
            input,
            dt,
            Self::PLAYER_SPEED,
            || {
                CollisionDetector::is_rect_grounded(
                    chunks,
                    materials,
                    player_pos.x,
                    player_pos.y,
                    player_width,
                    player_height,
                )
            },
            |x, y, w, h| CollisionDetector::check_solid_collision(chunks, materials, x, y, w, h),
        );
    }

    /// Check if player is dead
    pub fn is_player_dead(&self) -> bool {
        self.player.is_dead
    }

    /// Respawn player at spawn point
    pub fn respawn_player(&mut self) {
        let spawn_point = glam::Vec2::new(0.0, 100.0); // Default spawn
        self.player.respawn(spawn_point);
        log::info!("Player respawned at ({}, {})", spawn_point.x, spawn_point.y);
    }

    /// Update active chunks: remove distant chunks and re-activate nearby loaded chunks
    fn update_active_chunks(&mut self) {
        ChunkStatus::update_active_chunks(
            &mut self.chunk_manager,
            self.player.position,
            Self::ACTIVE_CHUNK_RADIUS,
        );
    }

    /// Get the tool registry
    pub fn tool_registry(&self) -> &ToolRegistry {
        &self.tool_registry
    }

    /// Spawn material at world coordinates with circular brush
    pub fn spawn_material(
        &mut self,
        world_x: i32,
        world_y: i32,
        material_id: u16,
        brush_size: u32,
    ) {
        let brush_radius = brush_size as i32;
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
        for dy in -brush_radius..=brush_radius {
            for dx in -brush_radius..=brush_radius {
                // Circular brush
                if dx * dx + dy * dy <= brush_radius * brush_radius {
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
        MiningSystem::mine_pixel(
            &mut self.player,
            &mut self.chunk_manager,
            world_x,
            world_y,
            &self.materials,
        )
    }

    /// Place material from player's inventory at world coordinates with circular brush
    /// Returns number of pixels successfully placed
    pub fn place_material_from_inventory(
        &mut self,
        world_x: i32,
        world_y: i32,
        material_id: u16,
        brush_size: u32,
    ) -> u32 {
        let positions = MiningSystem::place_material_from_inventory(
            &mut self.player,
            &self.chunk_manager,
            world_x,
            world_y,
            material_id,
            &self.materials,
            brush_size,
        );

        let count = positions.len() as u32;
        for (x, y) in positions {
            let mut pixel = Pixel::new(material_id);
            pixel.flags |= pixel_flags::PLAYER_PLACED;
            self.set_pixel_full(x, y, pixel);
        }

        count
    }

    /// Place material at world coordinates without consuming from inventory (debug mode)
    pub fn place_material_debug(
        &mut self,
        world_x: i32,
        world_y: i32,
        material_id: u16,
        brush_size: u32,
    ) -> u32 {
        let positions = MiningSystem::place_material_debug(
            &self.chunk_manager,
            world_x,
            world_y,
            material_id,
            brush_size,
        );

        let count = positions.len() as u32;
        for (x, y) in positions {
            let mut pixel = Pixel::new(material_id);
            pixel.flags |= pixel_flags::PLAYER_PLACED;
            self.set_pixel_full(x, y, pixel);
        }

        count
    }

    /// Start mining a pixel (calculates required time based on material hardness and tool)
    pub fn start_mining(&mut self, world_x: i32, world_y: i32) {
        MiningSystem::start_mining(
            &mut self.player,
            &self.chunk_manager,
            world_x,
            world_y,
            &self.materials,
            &self.tool_registry,
        )
    }

    /// Update mining progress (called each frame)
    /// Returns true if mining completed this frame
    pub fn update_mining(&mut self, delta_time: f32) -> bool {
        if let Some((x, y)) = MiningSystem::update_mining(&mut self.player, delta_time) {
            self.complete_mining(x, y);
            true
        } else {
            false
        }
    }

    /// Complete mining at the specified position
    fn complete_mining(&mut self, world_x: i32, world_y: i32) {
        if MiningSystem::complete_mining(
            &mut self.player,
            &self.chunk_manager,
            world_x,
            world_y,
            &self.materials,
            &self.tool_registry,
        )
        .is_some()
        {
            // Successfully mined - remove the pixel
            self.set_pixel(world_x, world_y, MaterialId::AIR);
        }
    }

    /// DEBUG: Instantly mine all materials in a circle around position
    /// Used for quick world exploration during testing
    pub fn debug_mine_circle(&mut self, center_x: i32, center_y: i32, radius: i32) {
        let positions = MiningSystem::debug_mine_circle(
            &mut self.player,
            &self.chunk_manager,
            center_x,
            center_y,
            radius,
            &self.materials,
        );

        // Remove pixels
        for (x, y) in positions {
            self.set_pixel(x, y, MaterialId::AIR);
        }
    }

    /// Update simulation
    pub fn update<R: crate::world::WorldRng>(
        &mut self,
        dt: f32,
        stats: &mut dyn crate::world::SimStats,
        rng: &mut R,
        is_multiplayer_connected: bool,
    ) {
        const FIXED_TIMESTEP: f32 = 1.0 / 60.0;

        // Update player (hunger, health, starvation damage)
        if self.player.update(dt) {
            log::info!("Player died!");
            // Player death is now handled in UI/app layer via is_dead flag
            // Respawn will be triggered by player input (game over screen)
        }

        // Update light system (day/night cycle, growth timer)
        self.light_system.update(dt);

        self.time_accumulator += dt;

        // Cap simulation steps to prevent "spiral of death"
        // If FPS drops, simulation slows down gracefully instead of trying to catch up
        const MAX_STEPS_PER_FRAME: u32 = 2;
        let mut steps = 0;

        while self.time_accumulator >= FIXED_TIMESTEP && steps < MAX_STEPS_PER_FRAME {
            self.step_simulation(stats, rng, is_multiplayer_connected);
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
        is_multiplayer_connected: bool,
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
            self.persistence_system
                .load_nearby_chunks(&mut self.chunk_manager, self.player.position);
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
        let chunk_manager = &self.chunk_manager;
        let chunks_to_update: Vec<IVec2> = chunk_manager
            .active_chunks
            .iter()
            .copied()
            .filter(|&pos| ChunkStatus::needs_ca_update(chunk_manager, pos))
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
        // Skip creature updates when connected to multiplayer (server is authoritative)
        if !is_multiplayer_connected {
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
    }

    /// Get growth progress as percentage (0-100) through the 10-second cycle
    pub fn get_growth_progress_percent(&self) -> f32 {
        self.light_system.get_growth_progress_percent()
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
        ChunkStatus::ensure_chunks_for_area(&mut self.chunk_manager, min_x, min_y, max_x, max_y);
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
        PixelQueries::get_temperature(&self.chunk_manager, world_x, world_y)
    }

    /// Get light level at world coordinates (0-15)
    pub fn get_light_at(&self, world_x: i32, world_y: i32) -> Option<u8> {
        PixelQueries::get_light(&self.light_system, &self.chunk_manager, world_x, world_y)
    }

    /// Set light level at world coordinates (0-15)
    pub fn set_light_at(&mut self, world_x: i32, world_y: i32, level: u8) {
        self.light_system
            .set_light_at(&mut self.chunk_manager, world_x, world_y, level);
    }

    /// Get material ID at world coordinates
    pub fn get_pixel_material(&self, world_x: i32, world_y: i32) -> Option<u16> {
        PixelQueries::get_material(&self.chunk_manager, world_x, world_y)
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

    /// Evict chunks that are far from player position (for multiplayer memory management)
    ///
    /// Removes chunks >10 chunks away from player to keep memory usage bounded.
    /// In multiplayer mode, skips disk save (server is authoritative).
    /// In singleplayer mode, saves dirty chunks before eviction.
    pub fn evict_distant_chunks(&mut self, player_pos: Vec2) {
        let player_chunk = IVec2::new(
            (player_pos.x as i32).div_euclid(CHUNK_SIZE as i32),
            (player_pos.y as i32).div_euclid(CHUNK_SIZE as i32),
        );

        let mut to_evict = Vec::new();

        // Find chunks to evict (>10 chunks from player)
        for pos in self.chunk_manager.chunks.keys() {
            let dist = (*pos - player_chunk).abs().max_element();

            if dist > 10 {
                to_evict.push(*pos);
            }
        }

        // Evict chunks
        for pos in to_evict {
            if let Some(chunk) = self.chunk_manager.chunks.remove(&pos) {
                // In singleplayer: save dirty chunks to disk
                // In multiplayer: skip save (server is authoritative, persistence is disabled)
                if chunk.dirty
                    && self.persistence_system.persistence.is_some()
                    && let Err(e) = self
                        .persistence_system
                        .persistence
                        .as_ref()
                        .unwrap()
                        .save_chunk(&chunk)
                {
                    log::error!("Failed to save chunk ({}, {}): {}", pos.x, pos.y, e);
                }

                log::debug!(
                    "Evicted chunk {:?} (distance {})",
                    pos,
                    (pos - player_chunk).abs().max_element()
                );
            }
        }
    }

    /// Generate a single chunk at position (for SpacetimeDB server)
    pub fn generate_chunk(&mut self, pos: IVec2) {
        let chunk = self
            .persistence_system
            .generator
            .generate_chunk(pos.x, pos.y);
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
        DebrisSystem::reconstruct_falling_chunk(&mut self.chunk_manager, chunk);
    }

    /// Clear all chunks from the world
    pub fn clear_all_chunks(&mut self) {
        self.persistence_system
            .clear_all_chunks(&mut self.chunk_manager);
    }

    /// Add a chunk to the world
    pub fn add_chunk(&mut self, chunk: Chunk) {
        self.persistence_system
            .add_chunk(&mut self.chunk_manager, chunk, self.player.position);
    }

    /// Initialize persistent world (load or generate)
    pub fn load_persistent_world(&mut self) {
        // Load world data (this also loads metadata with play_time_seconds)
        let _ = self
            .persistence_system
            .load_persistent_world(&mut self.chunk_manager, &mut self.player);

        // Load play time from metadata
        use crate::world::persistence::ChunkPersistence;
        if let Ok(persistence) = ChunkPersistence::new("default") {
            let metadata = persistence.load_metadata();
            self.total_play_time_seconds = metadata.play_time_seconds;
            self.session_start = Instant::now(); // Reset session start
            log::info!("Loaded play time: {} seconds", self.total_play_time_seconds);
        }

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
        self.persistence_system
            .disable_persistence(&mut self.chunk_manager);
    }

    /// Save all dirty chunks (periodic auto-save)
    pub fn save_dirty_chunks(&mut self) {
        self.persistence_system
            .save_dirty_chunks(&mut self.chunk_manager);
    }

    /// Save all chunks and metadata (manual save)
    pub fn save_all_dirty_chunks(&mut self) {
        // Calculate total play time (accumulated + current session)
        let session_duration = self.session_start.elapsed().as_secs();
        let total_play_time = self.total_play_time_seconds + session_duration;

        self.persistence_system.save_all_dirty_chunks(
            &mut self.chunk_manager,
            &self.player,
            total_play_time,
        );
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
        ChemistrySystem::check_pixel_reactions(
            &mut self.chunk_manager.chunks,
            &self.reactions,
            chunk_pos,
            x,
            y,
            stats,
            rng,
        );
    }
}

impl Default for World {
    fn default() -> Self {
        Self::new(false) // Default: spawn creatures
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
        Raycasting::raycast(&self.chunk_manager, from, direction, max_distance)
    }

    fn get_pressure_at(&self, x: i32, y: i32) -> f32 {
        PixelQueries::get_pressure(&self.chunk_manager, x, y)
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

#[cfg(test)]
#[path = "world_tests.rs"]
mod world_tests;
