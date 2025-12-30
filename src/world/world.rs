//! World - manages chunks and simulation

use std::collections::HashMap;
use glam::IVec2;

use super::{Chunk, Pixel, CHUNK_SIZE};
use crate::simulation::{Materials, MaterialId};

/// The game world, composed of chunks
pub struct World {
    /// Loaded chunks, keyed by chunk coordinates
    chunks: HashMap<IVec2, Chunk>,
    
    /// Material definitions
    pub materials: Materials,
    
    /// Player position (pixel coordinates)
    pub player_pos: glam::Vec2,
    
    /// Which chunks are currently active (being simulated)
    active_chunks: Vec<IVec2>,
    
    /// Simulation time accumulator
    time_accumulator: f32,
}

impl World {
    pub fn new() -> Self {
        let mut world = Self {
            chunks: HashMap::new(),
            materials: Materials::new(),
            player_pos: glam::Vec2::new(0.0, 100.0),
            active_chunks: Vec::new(),
            time_accumulator: 0.0,
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
        log::info!("  Player starts at: {:?}", self.player_pos);
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
            let new_pos = self.player_pos + velocity * dt;
            log::debug!("Player: {:?} → {:?} (velocity: {:?})",
                       self.player_pos, new_pos, velocity);
            self.player_pos = new_pos;
        }
    }

    /// Brush radius for material spawning (1 = 3x3, 2 = 5x5)
    const BRUSH_RADIUS: i32 = 1;

    /// Spawn material at world coordinates with circular brush
    pub fn spawn_material(&mut self, world_x: i32, world_y: i32, material_id: u16) {
        let material_name = &self.materials.get(material_id).name;
        log::info!("Spawning {} at world ({}, {})", material_name, world_x, world_y);

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
        log::debug!("  → Spawned {} pixels", spawned);
    }

    /// Update simulation
    pub fn update(&mut self, dt: f32) {
        const FIXED_TIMESTEP: f32 = 1.0 / 60.0;
        
        self.time_accumulator += dt;
        
        while self.time_accumulator >= FIXED_TIMESTEP {
            self.step_simulation();
            self.time_accumulator -= FIXED_TIMESTEP;
        }
    }
    
    fn step_simulation(&mut self) {
        // Clear update flags
        for pos in &self.active_chunks {
            if let Some(chunk) = self.chunks.get_mut(pos) {
                chunk.clear_update_flags();
            }
        }
        
        // TODO: Implement Noita-style checkerboard update pattern
        // For now, simple sequential update
        for pos in self.active_chunks.clone() {
            self.update_chunk_ca(pos);
        }
    }
    
    fn update_chunk_ca(&mut self, chunk_pos: IVec2) {
        // Update from bottom to top so falling works correctly
        for y in 0..CHUNK_SIZE {
            // Alternate direction each row for symmetry
            let x_iter: Box<dyn Iterator<Item = usize>> = if y % 2 == 0 {
                Box::new(0..CHUNK_SIZE)
            } else {
                Box::new((0..CHUNK_SIZE).rev())
            };
            
            for x in x_iter {
                self.update_pixel(chunk_pos, x, y);
            }
        }
    }
    
    fn update_pixel(&mut self, chunk_pos: IVec2, x: usize, y: usize) {
        let chunk = match self.chunks.get(&chunk_pos) {
            Some(c) => c,
            None => return,
        };
        
        let pixel = chunk.get_pixel(x, y);
        if pixel.is_empty() {
            return;
        }
        
        let material = self.materials.get(pixel.material_id);
        
        match material.material_type {
            crate::simulation::MaterialType::Powder => {
                self.update_powder(chunk_pos, x, y);
            }
            crate::simulation::MaterialType::Liquid => {
                self.update_liquid(chunk_pos, x, y);
            }
            crate::simulation::MaterialType::Gas => {
                self.update_gas(chunk_pos, x, y);
            }
            crate::simulation::MaterialType::Solid => {
                // Solids don't move
            }
        }
    }
    
    fn update_powder(&mut self, chunk_pos: IVec2, x: usize, y: usize) {
        // Convert to world coordinates
        let world_x = chunk_pos.x * CHUNK_SIZE as i32 + x as i32;
        let world_y = chunk_pos.y * CHUNK_SIZE as i32 + y as i32;

        // Try to fall down
        if self.try_move_world(world_x, world_y, world_x, world_y - 1) {
            return;
        }

        // Try to fall diagonally (randomized for symmetry)
        let try_left_first = rand::random::<bool>();

        if try_left_first {
            if self.try_move_world(world_x, world_y, world_x - 1, world_y - 1) {
                return;
            }
            if self.try_move_world(world_x, world_y, world_x + 1, world_y - 1) {
                return;
            }
        } else {
            if self.try_move_world(world_x, world_y, world_x + 1, world_y - 1) {
                return;
            }
            if self.try_move_world(world_x, world_y, world_x - 1, world_y - 1) {
                return;
            }
        }
    }
    
    fn update_liquid(&mut self, chunk_pos: IVec2, x: usize, y: usize) {
        // Convert to world coordinates
        let world_x = chunk_pos.x * CHUNK_SIZE as i32 + x as i32;
        let world_y = chunk_pos.y * CHUNK_SIZE as i32 + y as i32;

        // Try to fall down
        if self.try_move_world(world_x, world_y, world_x, world_y - 1) {
            return;
        }

        // Try to fall diagonally
        let try_left_first = rand::random::<bool>();

        if try_left_first {
            if self.try_move_world(world_x, world_y, world_x - 1, world_y - 1) {
                return;
            }
            if self.try_move_world(world_x, world_y, world_x + 1, world_y - 1) {
                return;
            }
        } else {
            if self.try_move_world(world_x, world_y, world_x + 1, world_y - 1) {
                return;
            }
            if self.try_move_world(world_x, world_y, world_x - 1, world_y - 1) {
                return;
            }
        }

        // Try to flow horizontally
        if try_left_first {
            if self.try_move_world(world_x, world_y, world_x - 1, world_y) {
                return;
            }
            if self.try_move_world(world_x, world_y, world_x + 1, world_y) {
                return;
            }
        } else {
            if self.try_move_world(world_x, world_y, world_x + 1, world_y) {
                return;
            }
            if self.try_move_world(world_x, world_y, world_x - 1, world_y) {
                return;
            }
        }
    }
    
    fn update_gas(&mut self, chunk_pos: IVec2, x: usize, y: usize) {
        // Convert to world coordinates
        let world_x = chunk_pos.x * CHUNK_SIZE as i32 + x as i32;
        let world_y = chunk_pos.y * CHUNK_SIZE as i32 + y as i32;

        // Gases rise (positive Y)
        if self.try_move_world(world_x, world_y, world_x, world_y + 1) {
            return;
        }

        // Try to rise diagonally
        let try_left_first = rand::random::<bool>();

        if try_left_first {
            if self.try_move_world(world_x, world_y, world_x - 1, world_y + 1) {
                return;
            }
            if self.try_move_world(world_x, world_y, world_x + 1, world_y + 1) {
                return;
            }
        } else {
            if self.try_move_world(world_x, world_y, world_x + 1, world_y + 1) {
                return;
            }
            if self.try_move_world(world_x, world_y, world_x - 1, world_y + 1) {
                return;
            }
        }

        // Disperse horizontally
        if try_left_first {
            if self.try_move_world(world_x, world_y, world_x - 1, world_y) {
                return;
            }
            if self.try_move_world(world_x, world_y, world_x + 1, world_y) {
                return;
            }
        } else {
            if self.try_move_world(world_x, world_y, world_x + 1, world_y) {
                return;
            }
            if self.try_move_world(world_x, world_y, world_x - 1, world_y) {
                return;
            }
        }
    }
    
    /// Try to move a pixel, returns true if successful
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
        let (chunk_pos, local_x, local_y) = Self::world_to_chunk_coords(world_x, world_y);
        if let Some(chunk) = self.chunks.get_mut(&chunk_pos) {
            chunk.set_material(local_x, local_y, material_id);
        } else {
            log::warn!("set_pixel: chunk {:?} not loaded (world: {}, {})",
                      chunk_pos, world_x, world_y);
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
}

impl Default for World {
    fn default() -> Self {
        Self::new()
    }
}
