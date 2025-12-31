//! World - manages chunks and simulation

use std::collections::HashMap;
use glam::IVec2;

use super::{Chunk, Pixel, CHUNK_SIZE, pixel_flags};
use super::generation::WorldGenerator;
use super::persistence::{ChunkPersistence, WorldMetadata};
use crate::simulation::{
    Materials, MaterialId, MaterialType,
    TemperatureSimulator, StateChangeSystem, ReactionRegistry,
    StructuralIntegritySystem,
    add_heat_at_pixel, get_temperature_at_pixel,
};
use crate::physics::PhysicsWorld;
use crate::entity::player::Player;

/// The game world, composed of chunks
pub struct World {
    /// Loaded chunks, keyed by chunk coordinates
    chunks: HashMap<IVec2, Chunk>,

    /// Material definitions
    pub materials: Materials,

    /// Temperature simulation system
    temperature_sim: TemperatureSimulator,

    /// Chemical reaction registry
    reactions: ReactionRegistry,

    /// Structural integrity checker
    structural_system: StructuralIntegritySystem,

    /// Physics world for rigid bodies
    physics_world: PhysicsWorld,

    /// The player entity
    pub player: Player,

    /// Which chunks are currently active (being simulated)
    active_chunks: Vec<IVec2>,

    /// Simulation time accumulator
    time_accumulator: f32,

    /// Chunk persistence manager (None for demo levels)
    persistence: Option<ChunkPersistence>,

    /// World generator for new chunks
    generator: WorldGenerator,

    /// Maximum number of chunks to keep loaded in memory
    loaded_chunk_limit: usize,
}

impl World {
    pub fn new() -> Self {
        let mut world = Self {
            chunks: HashMap::new(),
            materials: Materials::new(),
            temperature_sim: TemperatureSimulator::new(),
            reactions: ReactionRegistry::new(),
            structural_system: StructuralIntegritySystem::new(),
            physics_world: PhysicsWorld::new(),
            player: Player::new(glam::Vec2::new(0.0, 100.0)),
            active_chunks: Vec::new(),
            time_accumulator: 0.0,
            persistence: None,
            generator: WorldGenerator::new(42), // Default seed
            loaded_chunk_limit: 100,
        };

        // Initialize with some test chunks
        world.generate_test_world();

        world
    }
    
    /// Generate a simple test world for development
    fn generate_test_world(&mut self) {
        let mut total_pixels = 0;

        // Create a 5x5 grid of chunks around origin
        for cy in -2..=2 {
            for cx in -2..=2 {
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

        log::info!("Generated test world: {} chunks, {} pixels",
                   self.chunks.len(), total_pixels);
        log::info!("  World bounds: ({}, {}) to ({}, {})",
                   -2 * CHUNK_SIZE as i32, -2 * CHUNK_SIZE as i32,
                   3 * CHUNK_SIZE as i32 - 1, 3 * CHUNK_SIZE as i32 - 1);
        log::info!("  Player starts at: {:?}", self.player.position);
    }

    /// Player movement speed in pixels per second
    const PLAYER_SPEED: f32 = 200.0;

    /// Update player position based on input
    pub fn update_player(&mut self, input: &crate::app::InputState, dt: f32) {
        let mut velocity = glam::Vec2::ZERO;

        if input.w_pressed {
            velocity.y += 1.0;
        }
        if input.s_pressed {
            velocity.y -= 1.0;
        }
        if input.a_pressed {
            velocity.x -= 1.0;
        }
        if input.d_pressed {
            velocity.x += 1.0;
        }

        // Normalize diagonal movement
        if velocity.length() > 0.0 {
            velocity = velocity.normalize() * Self::PLAYER_SPEED;
            let movement = velocity * dt;
            log::debug!("Player: {:?} → {:?} (velocity: {:?})",
                       self.player.position, self.player.position + movement, velocity);
            self.player.move_by(movement);
            self.player.set_velocity(velocity);
        } else {
            self.player.set_velocity(glam::Vec2::ZERO);
        }
    }

    /// Brush radius for material spawning (1 = 3x3, 2 = 5x5)
    const BRUSH_RADIUS: i32 = 1;

    /// Spawn material at world coordinates with circular brush
    pub fn spawn_material(&mut self, world_x: i32, world_y: i32, material_id: u16) {
        let material_name = &self.materials.get(material_id).name;
        let (chunk_pos, _, _) = Self::world_to_chunk_coords(world_x, world_y);

        log::info!("[SPAWN] Spawning {} at world ({}, {}) in chunk ({}, {})",
                   material_name, world_x, world_y, chunk_pos.x, chunk_pos.y);

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
            log::debug!("[SPAWN] Chunk ({}, {}) dirty: {}, non-air pixels: {}",
                       chunk_pos.x, chunk_pos.y, chunk.dirty, chunk.count_non_air());
        }
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
                log::debug!("[MINE] Mined {} at ({}, {})", material_name, world_x, world_y);
                true
            } else {
                log::debug!("[MINE] Inventory full, can't mine at ({}, {})", world_x, world_y);
                false
            }
        } else {
            false
        }
    }

    /// Place material from player's inventory at world coordinates with circular brush
    /// Returns number of pixels successfully placed
    pub fn place_material_from_inventory(&mut self, world_x: i32, world_y: i32, material_id: u16) -> u32 {
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
                    if self.get_pixel(x, y).map(|p| p.material_id == MaterialId::AIR).unwrap_or(false) {
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
            log::debug!("[PLACE] Not enough {} in inventory (need {}, have {})",
                       material_name, pixels_needed, self.player.inventory.count_item(material_id));
            return 0;
        }

        // Consume from inventory first
        let consumed = self.player.inventory.remove_item(material_id, pixels_needed);

        // Place the pixels
        for dy in -Self::BRUSH_RADIUS..=Self::BRUSH_RADIUS {
            for dx in -Self::BRUSH_RADIUS..=Self::BRUSH_RADIUS {
                if dx * dx + dy * dy <= Self::BRUSH_RADIUS * Self::BRUSH_RADIUS {
                    let x = world_x + dx;
                    let y = world_y + dy;
                    if self.get_pixel(x, y).map(|p| p.material_id == MaterialId::AIR).unwrap_or(false) {
                        self.set_pixel(x, y, material_id);
                        placed += 1;
                    }
                }
            }
        }

        log::debug!("[PLACE] Placed {} {} pixels at ({}, {}), consumed {} from inventory",
                   placed, material_name, world_x, world_y, consumed);

        placed
    }

    /// Update simulation
    pub fn update(&mut self, dt: f32, stats: &mut crate::ui::StatsCollector) {
        const FIXED_TIMESTEP: f32 = 1.0 / 60.0;

        // Update player (hunger, health, starvation damage)
        if self.player.update(dt) {
            log::warn!("Player died from starvation!");
            // TODO: Handle player death (respawn, game over screen, etc.)
        }

        self.time_accumulator += dt;

        while self.time_accumulator >= FIXED_TIMESTEP {
            self.step_simulation(stats);
            self.time_accumulator -= FIXED_TIMESTEP;
        }
    }
    
    fn step_simulation(&mut self, stats: &mut crate::ui::StatsCollector) {
        // 1. Clear update flags
        for pos in &self.active_chunks {
            if let Some(chunk) = self.chunks.get_mut(pos) {
                chunk.clear_update_flags();
            }
        }

        // 2. CA updates (movement)
        // TODO: Implement Noita-style checkerboard update pattern
        // For now, simple sequential update
        for pos in self.active_chunks.clone() {
            self.update_chunk_ca(pos, stats);
        }

        // 3. Temperature diffusion (30fps throttled)
        self.temperature_sim.update(&mut self.chunks);

        // 4. State changes based on temperature
        for pos in &self.active_chunks.clone() {
            self.check_chunk_state_changes(*pos, stats);
        }

        // 5. Process structural integrity checks
        let positions = self.structural_system.drain_queue();
        let checks_processed = StructuralIntegritySystem::process_checks(self, positions);
        if checks_processed > 0 {
            log::debug!("Processed {} structural integrity checks", checks_processed);
        }

        // 6. Update rigid body physics
        self.physics_world.step();

        // 7. Check for settled debris and reconstruct as pixels
        let settled = self.physics_world.get_settled_debris();
        for handle in settled {
            self.reconstruct_debris(handle);
        }
    }
    
    fn update_chunk_ca(&mut self, chunk_pos: IVec2, stats: &mut crate::ui::StatsCollector) {
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
    
    fn update_pixel(&mut self, chunk_pos: IVec2, x: usize, y: usize, stats: &mut crate::ui::StatsCollector) {
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
    
    fn update_powder(&mut self, chunk_pos: IVec2, x: usize, y: usize, stats: &mut crate::ui::StatsCollector) {
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
            if self.try_move_world(world_x, world_y, world_x + 1, world_y - 1, stats) {
            }
        } else {
            if self.try_move_world(world_x, world_y, world_x + 1, world_y - 1, stats) {
                return;
            }
            if self.try_move_world(world_x, world_y, world_x - 1, world_y - 1, stats) {
            }
        }
    }
    
    fn update_liquid(&mut self, chunk_pos: IVec2, x: usize, y: usize, stats: &mut crate::ui::StatsCollector) {
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
            if self.try_move_world(world_x, world_y, world_x + 1, world_y, stats) {
            }
        } else {
            if self.try_move_world(world_x, world_y, world_x + 1, world_y, stats) {
                return;
            }
            if self.try_move_world(world_x, world_y, world_x - 1, world_y, stats) {
            }
        }
    }
    
    fn update_gas(&mut self, chunk_pos: IVec2, x: usize, y: usize, stats: &mut crate::ui::StatsCollector) {
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
            if self.try_move_world(world_x, world_y, world_x + 1, world_y, stats) {
            }
        } else {
            if self.try_move_world(world_x, world_y, world_x + 1, world_y, stats) {
                return;
            }
            if self.try_move_world(world_x, world_y, world_x - 1, world_y, stats) {
            }
        }
    }
    
    /// Try to move a pixel, returns true if successful
    #[allow(dead_code)]
    fn try_move(&mut self, chunk_pos: IVec2, from_x: usize, from_y: usize, to_x: usize, to_y: usize) -> bool {
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
        stats: &mut crate::ui::StatsCollector,
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
                stats.record_pixel_moved();
                return true;
            }
        } else {
            // Different chunks - sequential writes to avoid borrow checker issues
            // First, clear source
            if let Some(src_chunk) = self.chunks.get_mut(&src_chunk_pos) {
                src_chunk.set_pixel(src_x, src_y, Pixel::AIR);
            } else {
                return false;
            }

            // Then, set destination
            if let Some(dst_chunk) = self.chunks.get_mut(&dst_chunk_pos) {
                dst_chunk.set_pixel(dst_x, dst_y, src_pixel);
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
        self.chunks.get(&chunk_pos).map(|c| c.get_pixel(local_x, local_y))
    }
    
    /// Set pixel at world coordinates
    pub fn set_pixel(&mut self, world_x: i32, world_y: i32, material_id: u16) {
        // Check if we're removing a structural material
        let schedule_check = if let Some(old_pixel) = self.get_pixel(world_x, world_y) {
            if !old_pixel.is_empty() {
                let old_material = self.materials.get(old_pixel.material_id);
                old_material.structural && old_material.material_type == MaterialType::Solid
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
            chunk.set_material(local_x, local_y, material_id);

            // Log only if actually changing something (not just setting same material)
            if old_material_id != material_id {
                let material_name = &self.materials.get(material_id).name;
                log::trace!("[MODIFY] Chunk ({}, {}) at local ({}, {}) world ({}, {}) set to {} (was {})",
                           chunk_pos.x, chunk_pos.y, local_x, local_y, world_x, world_y,
                           material_name, old_material_id);
            }
        } else {
            log::warn!("set_pixel: chunk {:?} not loaded (world: {}, {})",
                      chunk_pos, world_x, world_y);
            return;
        }

        // Schedule structural check if we removed structural material with AIR
        if schedule_check && material_id == MaterialId::AIR {
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
        self.active_chunks.iter().filter_map(|pos| self.chunks.get(pos))
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

    /// Add bedrock collider for a chunk (called when chunk is loaded)
    pub fn add_bedrock_collider(&mut self, chunk_x: i32, chunk_y: i32) {
        self.physics_world.add_bedrock_collider(chunk_x, chunk_y);
    }

    /// Create falling debris from a pixel region
    /// Removes pixels from world and creates a rigid body
    pub fn create_debris(&mut self, region: std::collections::HashSet<IVec2>) -> rapier2d::dynamics::RigidBodyHandle {
        log::info!("Creating debris from {} pixels", region.len());

        // Build pixel map with materials
        let mut pixels = std::collections::HashMap::new();
        for pos in &region {
            if let Some(pixel) = self.get_pixel(pos.x, pos.y) {
                if !pixel.is_empty() {
                    pixels.insert(*pos, pixel.material_id);
                }
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
            log::trace!("set_pixel_direct_checked: chunk {:?} not loaded for pixel at ({}, {})",
                       chunk_pos, world_x, world_y);
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

        log::debug!("Reconstructing {} pixels at ({:.1}, {:.1}), rotation={:.2}°",
                   debris.pixels.len(), final_center.x, final_center.y, rotation.to_degrees());

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
            log::warn!("Reconstruction: {} pixels placed, {} failed (chunk not loaded?)",
                       placed_count, failed_count);
        } else {
            log::info!("Reconstruction: {} pixels placed successfully", placed_count);
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
        let has_bedrock = chunk.pixels().iter().any(|p| p.material_id == MaterialId::BEDROCK);

        self.chunks.insert(pos, chunk);

        // Add to active chunks if within range of player
        let dist_x = (pos.x - (self.player.position.x as i32 / CHUNK_SIZE as i32)).abs();
        let dist_y = (pos.y - (self.player.position.y as i32 / CHUNK_SIZE as i32)).abs();
        if dist_x <= 2 && dist_y <= 2
            && !self.active_chunks.contains(&pos) {
                self.active_chunks.push(pos);
            }

        // Add physics collider for bedrock chunks
        if has_bedrock {
            self.add_bedrock_collider(pos.x, pos.y);
        }
    }

    /// Initialize persistent world (load or generate)
    pub fn load_persistent_world(&mut self) {
        // Clear any existing chunks (from test world generation)
        self.clear_all_chunks();

        let persistence = ChunkPersistence::new("default")
            .expect("Failed to create chunk persistence");

        let metadata = persistence.load_metadata();

        self.generator = WorldGenerator::new(metadata.seed);

        // Restore player data if it exists, otherwise use spawn point
        if let Some(saved_player) = metadata.player_data {
            self.player = saved_player;
            log::info!("Restored player data: inventory={}/{} slots, health={:.0}/{:.0}, hunger={:.0}/{:.0}",
                      self.player.inventory.used_slot_count(),
                      self.player.inventory.max_slots,
                      self.player.health.current,
                      self.player.health.max,
                      self.player.hunger.current,
                      self.player.hunger.max);
        } else {
            // New world - set player at spawn point
            self.player.position = glam::Vec2::new(metadata.spawn_point.0, metadata.spawn_point.1);
            log::info!("New world - player spawned at {:?}", self.player.position);
        }

        self.persistence = Some(persistence);

        // Load initial chunks around spawn
        self.load_chunks_around_player();

        log::info!("Loaded persistent world (seed: {})", metadata.seed);
    }

    /// Load chunks within active radius of player
    fn load_chunks_around_player(&mut self) {
        let player_chunk_x = (self.player.position.x as i32).div_euclid(CHUNK_SIZE as i32);
        let player_chunk_y = (self.player.position.y as i32).div_euclid(CHUNK_SIZE as i32);

        for cy in (player_chunk_y - 2)..=(player_chunk_y + 2) {
            for cx in (player_chunk_x - 2)..=(player_chunk_x + 2) {
                self.load_or_generate_chunk(cx, cy);
            }
        }
    }

    /// Load or generate a chunk at the given coordinates
    fn load_or_generate_chunk(&mut self, chunk_x: i32, chunk_y: i32) {
        let pos = IVec2::new(chunk_x, chunk_y);

        if self.chunks.contains_key(&pos) {
            log::trace!("[LOAD] Chunk ({}, {}) already loaded, skipping", chunk_x, chunk_y);
            return; // Already loaded
        }

        let chunk = if let Some(persistence) = &self.persistence {
            log::debug!("[LOAD] Requesting chunk ({}, {}) from persistence", chunk_x, chunk_y);
            persistence.load_chunk(chunk_x, chunk_y, &self.generator)
        } else {
            // Demo mode: use generator without saving
            log::debug!("[GEN] Demo mode: generating chunk ({}, {}) without persistence", chunk_x, chunk_y);
            self.generator.generate_chunk(chunk_x, chunk_y)
        };

        let non_air = chunk.count_non_air();
        log::info!("[LOAD] Adding chunk ({}, {}) to world - {} non-air pixels", chunk_x, chunk_y, non_air);

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

            // Unload chunks >3 chunks away
            if dist_x > 3 || dist_y > 3 {
                to_evict.push(*pos);
            }
        }

        for pos in to_evict {
            if let Some(chunk) = self.chunks.remove(&pos) {
                if chunk.dirty {
                    if let Some(persistence) = &self.persistence {
                        if let Err(e) = persistence.save_chunk(&chunk) {
                            log::error!("Failed to save chunk ({}, {}): {}", pos.x, pos.y, e);
                        } else {
                            log::debug!("Saved and evicted chunk ({}, {})", pos.x, pos.y);
                        }
                    }
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
                    log::debug!("[SAVE] Saving dirty chunk ({}, {}) - {} non-air pixels", chunk.x, chunk.y, non_air);

                    if let Err(e) = persistence.save_chunk(chunk) {
                        log::error!("[SAVE] Failed to save chunk ({}, {}): {}", chunk.x, chunk.y, e);
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
            let metadata = WorldMetadata {
                version: 1,
                seed: self.generator.seed,
                spawn_point: (self.player.position.x, self.player.position.y),
                created_at: String::new(), // Preserved from load
                last_played: chrono::Local::now().to_rfc3339(),
                play_time_seconds: 0, // TODO: track play time
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
    fn check_chunk_state_changes(&mut self, chunk_pos: IVec2, stats: &mut crate::ui::StatsCollector) {
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
    fn update_fire(&mut self, chunk_pos: IVec2, x: usize, y: usize, stats: &mut crate::ui::StatsCollector) {
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

        if let Some(ignition_temp) = material.ignition_temp {
            if temp >= ignition_temp {
                // Mark pixel as burning
                let chunk = self.chunks.get_mut(&chunk_pos).unwrap();
                let mut new_pixel = pixel;
                new_pixel.flags |= pixel_flags::BURNING;
                chunk.set_pixel(x, y, new_pixel);

                // Try to spawn fire in adjacent air cell
                let world_x = chunk_pos.x * CHUNK_SIZE as i32 + x as i32;
                let world_y = chunk_pos.y * CHUNK_SIZE as i32 + y as i32;

                for (dx, dy) in [(0, 1), (1, 0), (-1, 0), (0, -1)] {
                    if let Some(neighbor) = self.get_pixel(world_x + dx, world_y + dy) {
                        if neighbor.is_empty() {
                            self.set_pixel(world_x + dx, world_y + dy, MaterialId::FIRE);
                            break;
                        }
                    }
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
    fn check_pixel_reactions(&mut self, chunk_pos: IVec2, x: usize, y: usize, stats: &mut crate::ui::StatsCollector) {
        let chunk = match self.chunks.get(&chunk_pos) {
            Some(c) => c,
            None => return,
        };

        let pixel = chunk.get_pixel(x, y);
        if pixel.is_empty() {
            return;
        }

        let temp = get_temperature_at_pixel(chunk, x, y);
        let world_x = chunk_pos.x * CHUNK_SIZE as i32 + x as i32;
        let world_y = chunk_pos.y * CHUNK_SIZE as i32 + y as i32;

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
