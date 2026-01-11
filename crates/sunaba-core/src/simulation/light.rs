//! Light propagation system
//!
//! Implements flood-fill light propagation with material-based transmission.
//! Light levels range from 0 (dark) to 15 (full light).

use crate::simulation::{MaterialType, Materials};
use crate::world::CHUNK_SIZE;
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

    /// Propagate light through the world (active chunks only)
    /// Should be called at lower frequency than CA update (e.g., 15fps vs 60fps)
    pub fn propagate_light(
        &mut self,
        chunks: &mut std::collections::HashMap<glam::IVec2, crate::world::Chunk>,
        materials: &Materials,
        sky_light: u8,
        active_chunks: &[glam::IVec2],
    ) {
        // Clear all light levels in active chunks only
        self.reset_light_levels(chunks, active_chunks);

        // Add sky light to surface pixels in active chunks
        self.add_sky_light(chunks, sky_light, active_chunks);

        // Add light sources (fire, lava) in active chunks
        self.add_light_sources(chunks, active_chunks);

        // Flood-fill propagation (can spill into neighboring chunks)
        self.flood_fill_light(chunks, materials);

        // Mark active chunks as light-clean
        for &pos in active_chunks {
            if let Some(chunk) = chunks.get_mut(&pos) {
                chunk.light_dirty = false;
            }
        }
    }

    /// Reset light levels in active chunks only
    fn reset_light_levels(
        &mut self,
        chunks: &mut std::collections::HashMap<glam::IVec2, crate::world::Chunk>,
        active_chunks: &[glam::IVec2],
    ) {
        #[cfg(all(not(target_arch = "wasm32"), feature = "regeneration"))]
        {
            use rayon::prelude::*;
            // Parallel reset of light levels in chunks
            active_chunks.par_iter().for_each(|&pos| unsafe {
                let chunks_ptr = chunks as *const _
                    as *mut std::collections::HashMap<glam::IVec2, crate::world::Chunk>;
                if let Some(chunk) = (*chunks_ptr).get_mut(&pos)
                    && chunk.light_dirty
                {
                    chunk.light_levels.fill(0);
                }
            });
        }

        #[cfg(any(target_arch = "wasm32", not(feature = "regeneration")))]
        {
            for &pos in active_chunks {
                if let Some(chunk) = chunks.get_mut(&pos)
                    && chunk.light_dirty
                {
                    chunk.light_levels.fill(0);
                }
            }
        }
    }

    /// Add sky light to surface pixels in active chunks
    fn add_sky_light(
        &mut self,
        chunks: &mut std::collections::HashMap<glam::IVec2, crate::world::Chunk>,
        sky_light: u8,
        active_chunks: &[glam::IVec2],
    ) {
        if sky_light == 0 {
            return; // Night time, no sky light
        }

        // Surface level from world generator (y=32)
        const SURFACE_LEVEL: i32 = 32;

        // Add sky light to all air pixels above surface in active chunks
        for &chunk_pos in active_chunks {
            let chunk = match chunks.get_mut(&chunk_pos) {
                Some(c) => c,
                None => continue,
            };

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

    /// Add light sources (fire, lava, etc.) in active chunks
    fn add_light_sources(
        &mut self,
        chunks: &mut std::collections::HashMap<glam::IVec2, crate::world::Chunk>,
        active_chunks: &[glam::IVec2],
    ) {
        for &chunk_pos in active_chunks {
            let chunk = match chunks.get_mut(&chunk_pos) {
                Some(c) => c,
                None => continue,
            };

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
            (0, 1),  // Down
            (0, -1), // Up
            (1, 0),  // Right
            (-1, 0), // Left
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::simulation::MaterialId;
    use crate::world::Chunk;

    fn setup_test_chunks() -> (
        std::collections::HashMap<glam::IVec2, crate::world::Chunk>,
        Materials,
    ) {
        let mut chunks = std::collections::HashMap::new();
        for cy in -1..=1 {
            for cx in -1..=1 {
                chunks.insert(glam::IVec2::new(cx, cy), Chunk::new(cx, cy));
            }
        }
        (chunks, Materials::new())
    }

    #[test]
    fn test_light_emission_fire() {
        let light = LightPropagation::new();
        assert_eq!(light.get_light_emission(MaterialId::FIRE), LIGHT_FIRE);
        assert_eq!(LIGHT_FIRE, 15, "Fire should emit maximum light");
    }

    #[test]
    fn test_light_emission_lava() {
        let light = LightPropagation::new();
        assert_eq!(light.get_light_emission(MaterialId::LAVA), LIGHT_LAVA);
        assert_eq!(LIGHT_LAVA, 12, "Lava should emit bright light");
    }

    #[test]
    fn test_light_emission_other_materials() {
        let light = LightPropagation::new();

        // Non-emitting materials should return 0
        assert_eq!(light.get_light_emission(MaterialId::AIR), 0);
        assert_eq!(light.get_light_emission(MaterialId::STONE), 0);
        assert_eq!(light.get_light_emission(MaterialId::WATER), 0);
        assert_eq!(light.get_light_emission(MaterialId::SAND), 0);
    }

    #[test]
    fn test_calculate_transmission_gas() {
        let light = LightPropagation::new();

        // Gas (air) transmits light with -1 per step
        assert_eq!(light.calculate_transmission(15, &MaterialType::Gas), 14);
        assert_eq!(light.calculate_transmission(5, &MaterialType::Gas), 4);
        assert_eq!(light.calculate_transmission(1, &MaterialType::Gas), 0);
        assert_eq!(light.calculate_transmission(0, &MaterialType::Gas), 0);
    }

    #[test]
    fn test_calculate_transmission_liquid() {
        let light = LightPropagation::new();

        // Liquids absorb more light (-2 per step)
        assert_eq!(light.calculate_transmission(15, &MaterialType::Liquid), 13);
        assert_eq!(light.calculate_transmission(5, &MaterialType::Liquid), 3);
        assert_eq!(light.calculate_transmission(2, &MaterialType::Liquid), 0);
        assert_eq!(light.calculate_transmission(1, &MaterialType::Liquid), 0);
    }

    #[test]
    fn test_calculate_transmission_solid() {
        let light = LightPropagation::new();

        // Solids block light completely
        assert_eq!(light.calculate_transmission(15, &MaterialType::Solid), 0);
        assert_eq!(light.calculate_transmission(5, &MaterialType::Solid), 0);
    }

    #[test]
    fn test_calculate_transmission_powder() {
        let light = LightPropagation::new();

        // Powder also blocks light completely
        assert_eq!(light.calculate_transmission(15, &MaterialType::Powder), 0);
    }

    #[test]
    fn test_world_to_chunk_coords_positive() {
        // Positive coordinates
        let (chunk, lx, ly) = LightPropagation::world_to_chunk_coords(65, 70);
        assert_eq!(chunk, glam::IVec2::new(1, 1));
        assert_eq!(lx, 1);
        assert_eq!(ly, 6);
    }

    #[test]
    fn test_world_to_chunk_coords_negative() {
        // Negative coordinates
        let (chunk, lx, ly) = LightPropagation::world_to_chunk_coords(-1, -1);
        assert_eq!(chunk, glam::IVec2::new(-1, -1));
        assert_eq!(lx, 63);
        assert_eq!(ly, 63);
    }

    #[test]
    fn test_world_to_chunk_coords_boundary() {
        // At chunk boundary
        let (chunk, lx, ly) = LightPropagation::world_to_chunk_coords(64, 64);
        assert_eq!(chunk, glam::IVec2::new(1, 1));
        assert_eq!(lx, 0);
        assert_eq!(ly, 0);
    }

    #[test]
    fn test_light_propagation_new() {
        let light = LightPropagation::new();
        // Queue should start empty
        assert_eq!(light.queue.len(), 0);
    }

    #[test]
    fn test_propagate_light_empty_world() {
        let (mut chunks, materials) = setup_test_chunks();
        let mut light = LightPropagation::new();

        // No active chunks, should complete without error
        let active_chunks = vec![];
        light.propagate_light(&mut chunks, &materials, 0, &active_chunks);

        // No crash means success
    }

    #[test]
    fn test_propagate_light_marks_clean() {
        let (mut chunks, materials) = setup_test_chunks();
        let mut light = LightPropagation::new();

        // Mark chunk as light dirty
        let chunk_pos = glam::IVec2::new(0, 0);
        chunks.get_mut(&chunk_pos).unwrap().light_dirty = true;

        let active_chunks = vec![chunk_pos];
        light.propagate_light(&mut chunks, &materials, 0, &active_chunks);

        // Should mark chunk as clean
        assert!(!chunks.get(&chunk_pos).unwrap().light_dirty);
    }

    #[test]
    fn test_light_constants() {
        assert_eq!(LIGHT_MAX, 15, "Max light should be 15");
        const { assert!(LIGHT_FIRE <= LIGHT_MAX, "Fire light should not exceed max") };
        const { assert!(LIGHT_LAVA <= LIGHT_MAX, "Lava light should not exceed max") };
    }
}
