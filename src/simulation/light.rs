//! Light propagation system
//!
//! Implements flood-fill light propagation with material-based transmission.
//! Light levels range from 0 (dark) to 15 (full light).

use crate::world::CHUNK_SIZE;
use crate::simulation::{MaterialType, Materials};
use std::collections::VecDeque;

/// Light sources and their intensities
pub const LIGHT_FIRE: u8 = 15;
pub const LIGHT_LAVA: u8 = 12;
pub const LIGHT_MAX: u8 = 15;

/// Light propagation manager
pub struct LightPropagation {
    /// Queue for flood-fill algorithm (world_x, world_y, light_level)
    queue: VecDeque<(i32, i32, u8)>,
}

impl LightPropagation {
    pub fn new() -> Self {
        Self {
            queue: VecDeque::with_capacity(10000),
        }
    }

    /// Propagate light through the world
    /// Should be called at lower frequency than CA update (e.g., 15fps vs 60fps)
    pub fn propagate_light(
        &mut self,
        chunks: &mut std::collections::HashMap<glam::IVec2, crate::world::Chunk>,
        materials: &Materials,
        sky_light: u8,
    ) {
        // Clear all light levels and mark dirty chunks
        self.reset_light_levels(chunks);

        // Add sky light to surface pixels
        self.add_sky_light(chunks, sky_light);

        // Add light sources (fire, lava)
        self.add_light_sources(chunks);

        // Flood-fill propagation
        self.flood_fill_light(chunks, materials);

        // Mark all chunks as light-clean
        for chunk in chunks.values_mut() {
            chunk.light_dirty = false;
        }
    }

    /// Reset all light levels to 0
    fn reset_light_levels(
        &mut self,
        chunks: &mut std::collections::HashMap<glam::IVec2, crate::world::Chunk>,
    ) {
        for chunk in chunks.values_mut() {
            if chunk.light_dirty {
                chunk.light_levels.fill(0);
            }
        }
    }

    /// Add sky light to surface pixels (y > surface level)
    fn add_sky_light(
        &mut self,
        chunks: &mut std::collections::HashMap<glam::IVec2, crate::world::Chunk>,
        sky_light: u8,
    ) {
        if sky_light == 0 {
            return; // Night time, no sky light
        }

        // Surface level from world generator (y=32)
        const SURFACE_LEVEL: i32 = 32;

        // Add sky light to all air pixels above surface
        for (&chunk_pos, chunk) in chunks.iter_mut() {
            let chunk_world_y_min = chunk_pos.y * CHUNK_SIZE as i32;
            let chunk_world_y_max = chunk_world_y_min + CHUNK_SIZE as i32;

            // Skip chunks entirely below surface
            if chunk_world_y_max <= SURFACE_LEVEL {
                continue;
            }

            for local_y in 0..CHUNK_SIZE {
                let world_y = chunk_world_y_min + local_y as i32;

                // Only pixels above surface
                if world_y <= SURFACE_LEVEL {
                    continue;
                }

                for local_x in 0..CHUNK_SIZE {
                    let material_id = chunk.get_material(local_x, local_y);

                    // Only set sky light on air pixels
                    if material_id == crate::simulation::MaterialId::AIR {
                        chunk.set_light(local_x, local_y, sky_light);

                        // Add to propagation queue
                        let world_x = chunk_pos.x * CHUNK_SIZE as i32 + local_x as i32;
                        self.queue.push_back((world_x, world_y, sky_light));
                    }
                }
            }
        }
    }

    /// Add light sources (fire, lava, etc.)
    fn add_light_sources(
        &mut self,
        chunks: &mut std::collections::HashMap<glam::IVec2, crate::world::Chunk>,
    ) {
        for (&chunk_pos, chunk) in chunks.iter_mut() {
            for local_y in 0..CHUNK_SIZE {
                for local_x in 0..CHUNK_SIZE {
                    let material_id = chunk.get_material(local_x, local_y);

                    // Check if this material emits light
                    let light_level = self.get_light_emission(material_id);

                    if light_level > 0 {
                        chunk.set_light(local_x, local_y, light_level);

                        // Add to propagation queue
                        let world_x = chunk_pos.x * CHUNK_SIZE as i32 + local_x as i32;
                        let world_y = chunk_pos.y * CHUNK_SIZE as i32 + local_y as i32;
                        self.queue.push_back((world_x, world_y, light_level));
                    }
                }
            }
        }
    }

    /// Get light emission for a material
    fn get_light_emission(&self, material_id: u16) -> u8 {
        use crate::simulation::MaterialId;

        match material_id {
            MaterialId::FIRE => LIGHT_FIRE,
            MaterialId::LAVA => LIGHT_LAVA,
            _ => 0,
        }
    }

    /// Flood-fill light propagation from sources
    fn flood_fill_light(
        &mut self,
        chunks: &mut std::collections::HashMap<glam::IVec2, crate::world::Chunk>,
        materials: &Materials,
    ) {
        const NEIGHBORS: [(i32, i32); 4] = [
            (0, 1),   // Down
            (0, -1),  // Up
            (1, 0),   // Right
            (-1, 0),  // Left
        ];

        while let Some((wx, wy, light)) = self.queue.pop_front() {
            // Light diminishes as it propagates
            if light == 0 {
                continue;
            }

            // Propagate to neighbors
            for (dx, dy) in &NEIGHBORS {
                let nx = wx + dx;
                let ny = wy + dy;

                // Convert world coordinates to chunk coordinates
                let (chunk_pos, local_x, local_y) = Self::world_to_chunk_coords(nx, ny);

                let chunk = match chunks.get_mut(&chunk_pos) {
                    Some(c) => c,
                    None => continue, // Chunk not loaded
                };

                // Get current light and material
                let current_light = chunk.get_light(local_x, local_y);
                let material_id = chunk.get_material(local_x, local_y);
                let material = materials.get(material_id);

                // Calculate light transmission
                let transmitted_light = self.calculate_transmission(light, &material.material_type);

                // Only update if new light is brighter
                if transmitted_light > current_light {
                    chunk.set_light(local_x, local_y, transmitted_light);

                    // Continue propagating
                    if transmitted_light > 0 {
                        self.queue.push_back((nx, ny, transmitted_light));
                    }
                }
            }
        }
    }

    /// Convert world coordinates to chunk coordinates + local offset
    fn world_to_chunk_coords(world_x: i32, world_y: i32) -> (glam::IVec2, usize, usize) {
        let chunk_x = world_x.div_euclid(CHUNK_SIZE as i32);
        let chunk_y = world_y.div_euclid(CHUNK_SIZE as i32);
        let local_x = world_x.rem_euclid(CHUNK_SIZE as i32) as usize;
        let local_y = world_y.rem_euclid(CHUNK_SIZE as i32) as usize;
        (glam::IVec2::new(chunk_x, chunk_y), local_x, local_y)
    }

    /// Calculate light transmission through material
    fn calculate_transmission(&self, light: u8, material_type: &MaterialType) -> u8 {
        match material_type {
            MaterialType::Gas => {
                // Air/gas: Full transmission, -1 per distance
                light.saturating_sub(1)
            }
            MaterialType::Liquid => {
                // Liquids: -2 per pixel
                light.saturating_sub(2)
            }
            MaterialType::Solid | MaterialType::Powder => {
                // Solids block light completely
                0
            }
        }
    }
}

impl Default for LightPropagation {
    fn default() -> Self {
        Self::new()
    }
}
