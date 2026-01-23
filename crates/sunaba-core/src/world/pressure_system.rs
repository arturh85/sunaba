//! Manages the accumulation and propagation of pressure through the world.
// Based on the POWDER_PLAN.md

use super::{CHUNK_SIZE, Chunk}; // From sunaba-core's own world module
use glam::IVec2;
use std::collections::{HashMap, VecDeque};
use sunaba_simulation::materials::{MaterialId, MaterialType, Materials}; // From sunaba-simulation
use sunaba_simulation::pixel::Pixel; // From sunaba-simulation

const MAX_PRESSURE_PROPAGATION_DEPTH: usize = 128;
const PRESSURE_QUEUE_MAX: usize = 256;
const MAX_PRESSURE: f32 = 100.0;
const MIN_PRESSURE: f32 = 0.0;
const PRESSURE_DECAY_RATE: f32 = 0.02; // How quickly pressure dissipates naturally
const PRESSURE_PROPAGATION_FACTOR: f32 = 0.4; // How much pressure transfers to neighbors
const PRESSURE_MOVE_THRESHOLD: f32 = 5.0; // Min pressure to move light materials

/// Manages the pressure simulation.
pub struct PressureSystem {
    /// Queue of pixels to update for pressure propagation.
    propagation_queue: VecDeque<(IVec2, usize, usize)>,
}

impl PressureSystem {
    pub fn new() -> Self {
        Self {
            propagation_queue: VecDeque::with_capacity(PRESSURE_QUEUE_MAX),
        }
    }

    /// Updates the pressure state for all active chunks.
    pub fn update(
        &mut self,
        chunks: &mut HashMap<IVec2, Chunk>,
        active_chunks: &[IVec2],
        materials: &Materials,
    ) {
        // 1. Decay existing pressure
        self.decay_pressure(chunks, active_chunks);

        // 2. Accumulate pressure from gas materials and sources
        self.accumulate_pressure(chunks, active_chunks, materials);

        // 3. Propagate pressure through the grid
        self.propagate_pressure(chunks, materials);

        // 4. Apply pressure effects (move materials, trigger reactions, break structural)
        self.apply_pressure_effects(chunks, active_chunks, materials);
    }

    /// Reduces pressure level of all coarse grid cells.
    fn decay_pressure(&self, chunks: &mut HashMap<IVec2, Chunk>, active_chunks: &[IVec2]) {
        for &chunk_pos in active_chunks {
            if let Some(chunk) = chunks.get_mut(&chunk_pos) {
                let mut needs_update = false;
                for i in 0..chunk.pressure.len() {
                    if chunk.pressure[i] > MIN_PRESSURE {
                        chunk.pressure[i] =
                            (chunk.pressure[i] - PRESSURE_DECAY_RATE).max(MIN_PRESSURE);
                        needs_update = true;
                    }
                }
                if needs_update {
                    chunk.dirty = true;
                }
            }
        }
    }

    /// Accumulates pressure from gas materials and pressure sources (e.g., FAN).
    fn accumulate_pressure(
        &mut self,
        chunks: &mut HashMap<IVec2, Chunk>,
        active_chunks: &[IVec2],
        materials: &Materials,
    ) {
        for &chunk_pos in active_chunks {
            if let Some(chunk) = chunks.get_mut(&chunk_pos) {
                for y in 0..CHUNK_SIZE {
                    for x in 0..CHUNK_SIZE {
                        let pixel = chunk.get_pixel(x, y);
                        let material_def = materials.get(pixel.material_id);
                        let coarse_idx = chunk.get_coarse_grid_index(x, y);

                        // Basic pressure from gas density
                        if material_def.material_type == MaterialType::Gas
                            && material_def.density > 0.001
                        // Ignore actual vacuum
                        {
                            let gas_pressure = material_def.density * 5.0; // Scale density to pressure
                            chunk.pressure[coarse_idx] =
                                (chunk.pressure[coarse_idx] + gas_pressure).min(MAX_PRESSURE);
                            if chunk.pressure[coarse_idx] > MIN_PRESSURE
                                && self.propagation_queue.len() < PRESSURE_QUEUE_MAX
                            {
                                self.propagation_queue.push_back((chunk_pos, x, y));
                            }
                        }

                        // Special pressure sources (e.g., FAN) - To be implemented as behaviors later
                        // if pixel.flags & pixel_flags::PRESSURE_SOURCE != 0 {
                        //     let source_pressure = material_def.pressure_generation;
                        //     chunk.pressure[coarse_idx] = (chunk.pressure[coarse_idx] + source_pressure).min(MAX_PRESSURE);
                        //     if chunk.pressure[coarse_idx] > MIN_PRESSURE && self.propagation_queue.len() < PRESSURE_QUEUE_MAX {
                        //         self.propagation_queue.push_back((chunk_pos, x, y));
                        //     }
                        // }
                    }
                }
            }
        }
    }

    /// Propagates pressure through the grid, smoothing out differences.
    fn propagate_pressure(&mut self, chunks: &mut HashMap<IVec2, Chunk>, _materials: &Materials) {
        let mut depth = 0;

        while let Some((chunk_pos, x, y)) = self.propagation_queue.pop_front() {
            if depth > MAX_PRESSURE_PROPAGATION_DEPTH {
                // To prevent infinite loops and limit per-frame computation
                break;
            }
            depth += 1;

            let source_pressure = {
                if let Some(chunk) = chunks.get(&chunk_pos) {
                    let coarse_idx = chunk.get_coarse_grid_index(x, y);
                    chunk.pressure[coarse_idx]
                } else {
                    continue;
                }
            };

            if source_pressure <= MIN_PRESSURE {
                continue;
            }

            for dy in -1..=1 {
                for dx in -1..=1 {
                    if dx == 0 && dy == 0 {
                        continue;
                    }

                    let nx = x as i32 + dx;
                    let ny = y as i32 + dy;
                    let (next_chunk_pos, next_x, next_y) = self.get_neighbor_pos(chunk_pos, nx, ny);

                    if let Some(neighbor_chunk) = chunks.get_mut(&next_chunk_pos) {
                        let neighbor_coarse_idx =
                            neighbor_chunk.get_coarse_grid_index(next_x, next_y);
                        let neighbor_pressure = &mut neighbor_chunk.pressure[neighbor_coarse_idx];

                        // Pressure equalization
                        let pressure_diff = source_pressure - *neighbor_pressure;
                        if pressure_diff > 0.1 {
                            // Only propagate if there's a significant difference
                            let transfer_amount = pressure_diff * PRESSURE_PROPAGATION_FACTOR;
                            *neighbor_pressure += transfer_amount;
                            // Source loses pressure is handled by overall decay.
                            // To prevent complex borrow issues, we only add to neighbors here.

                            // Clamp values
                            *neighbor_pressure = neighbor_pressure.min(MAX_PRESSURE);

                            // Add neighbor to queue if its pressure is now significant
                            if *neighbor_pressure > MIN_PRESSURE
                                && self.propagation_queue.len() < PRESSURE_QUEUE_MAX
                            {
                                self.propagation_queue
                                    .push_back((next_chunk_pos, next_x, next_y));
                            }
                        }
                        neighbor_chunk.dirty = true;
                    }
                }
            }
        }
        self.propagation_queue.clear();
    }

    /// Applies pressure effects to materials.
    /// This involves moving lighter materials, breaking weak structural materials,
    /// and triggering pressure-based reactions.
    fn apply_pressure_effects(
        &self,
        chunks: &mut HashMap<IVec2, Chunk>,
        active_chunks: &[IVec2],
        materials: &Materials,
    ) {
        // Collect movement actions to avoid borrow conflicts
        let mut movements = Vec::new();

        for &chunk_pos in active_chunks {
            if let Some(chunk) = chunks.get(&chunk_pos) {
                for y in 0..CHUNK_SIZE {
                    for x in 0..CHUNK_SIZE {
                        let coarse_idx = chunk.get_coarse_grid_index(x, y);
                        let current_pressure = chunk.pressure[coarse_idx];

                        if current_pressure > PRESSURE_MOVE_THRESHOLD {
                            // High pressure: try to move lighter materials away
                            let current_pixel = chunk.get_pixel(x, y);
                            let current_material = materials.get(current_pixel.material_id);

                            // Find a direction of lower pressure to push towards
                            let mut best_dir = None;
                            let mut lowest_neighbor_pressure = current_pressure;

                            for dy in -1..=1 {
                                for dx in -1..=1 {
                                    if dx == 0 && dy == 0 {
                                        continue;
                                    }
                                    let nx = x as i32 + dx;
                                    let ny = y as i32 + dy;
                                    let (neighbor_chunk_pos, neighbor_x, neighbor_y) =
                                        self.get_neighbor_pos(chunk_pos, nx, ny);

                                    // Check neighbor pressure
                                    if let Some(neighbor_chunk) = chunks.get(&neighbor_chunk_pos) {
                                        let neighbor_coarse_idx = neighbor_chunk
                                            .get_coarse_grid_index(neighbor_x, neighbor_y);
                                        let neighbor_pressure =
                                            neighbor_chunk.pressure[neighbor_coarse_idx];

                                        if neighbor_pressure < lowest_neighbor_pressure {
                                            lowest_neighbor_pressure = neighbor_pressure;
                                            best_dir = Some((dx, dy));
                                        }
                                    }
                                }
                            }

                            if let Some((dx, dy)) = best_dir {
                                let nx = x as i32 + dx;
                                let ny = y as i32 + dy;
                                let (target_chunk_pos, target_x, target_y) =
                                    self.get_neighbor_pos(chunk_pos, nx, ny);

                                if let Some(target_chunk) = chunks.get(&target_chunk_pos) {
                                    let target_pixel = target_chunk.get_pixel(target_x, target_y);
                                    let target_material = materials.get(target_pixel.material_id);

                                    // Push current pixel into air or lighter material if pressure is high
                                    if current_material.density < target_material.density // Push only if lighter
                                        || target_pixel.material_id == MaterialId::AIR
                                    {
                                        // Queue the movement
                                        movements.push((
                                            chunk_pos,
                                            x,
                                            y,
                                            target_chunk_pos,
                                            target_x,
                                            target_y,
                                            current_pixel,
                                        ));
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // Apply all collected movements
        for (src_chunk, src_x, src_y, dst_chunk, dst_x, dst_y, pixel) in movements {
            if src_chunk == dst_chunk {
                // Same chunk, can modify directly
                if let Some(chunk) = chunks.get_mut(&src_chunk) {
                    chunk.set_pixel(dst_x, dst_y, pixel);
                    chunk.set_pixel(src_x, src_y, Pixel::new(MaterialId::AIR));
                    chunk.dirty = true;
                }
            } else {
                // Different chunks, handle sequentially
                // First, set source to air
                if let Some(chunk) = chunks.get_mut(&src_chunk) {
                    chunk.set_pixel(src_x, src_y, Pixel::new(MaterialId::AIR));
                    chunk.dirty = true;
                }
                // Then, set destination to pixel
                if let Some(chunk) = chunks.get_mut(&dst_chunk) {
                    chunk.set_pixel(dst_x, dst_y, pixel);
                    chunk.dirty = true;
                }
            }
        }
    }

    /// Helper to get neighbor position, handling chunk boundaries.
    fn get_neighbor_pos(&self, chunk_pos: IVec2, x: i32, y: i32) -> (IVec2, usize, usize) {
        let mut next_chunk_pos = chunk_pos;
        let mut next_x = x;
        let mut next_y = y;

        if x < 0 {
            next_chunk_pos.x -= 1;
            next_x = CHUNK_SIZE as i32 - 1;
        } else if x >= CHUNK_SIZE as i32 {
            next_chunk_pos.x += 1;
            next_x = 0;
        }

        if y < 0 {
            next_chunk_pos.y -= 1;
            next_y = CHUNK_SIZE as i32 - 1;
        } else if y >= CHUNK_SIZE as i32 {
            next_chunk_pos.y += 1;
            next_y = 0;
        }

        (next_chunk_pos, next_x as usize, next_y as usize)
    }
}

impl Default for PressureSystem {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::simulation::{MaterialId, Materials};
    use std::collections::HashMap;

    #[test]
    fn test_pressure_accumulation_from_gas() {
        let mut system = PressureSystem::new();
        let materials = Materials::new();
        let mut chunks = HashMap::new();
        let chunk_pos = IVec2::new(0, 0);

        // Create a chunk with some gas (STEAM)
        let mut chunk = Chunk::new(0, 0);
        for y in 0..8 {
            for x in 0..8 {
                chunk.set_material(x, y, MaterialId::STEAM);
            }
        }
        chunks.insert(chunk_pos, chunk);
        let active_chunks = vec![chunk_pos];

        // Run update - gas should accumulate pressure
        system.update(&mut chunks, &active_chunks, &materials);

        // Check that pressure increased
        let chunk = chunks.get(&chunk_pos).unwrap();
        let coarse_idx = 0; // First coarse cell
        assert!(
            chunk.pressure[coarse_idx] > 0.0,
            "Pressure should accumulate from gas"
        );
    }

    #[test]
    fn test_pressure_decay() {
        let system = PressureSystem::new();
        let _materials = Materials::new();
        let mut chunks = HashMap::new();
        let chunk_pos = IVec2::new(0, 0);

        // Create a chunk with high pressure
        let mut chunk = Chunk::new(0, 0);
        chunk.pressure[0] = 50.0;
        chunks.insert(chunk_pos, chunk);
        let active_chunks = vec![chunk_pos];

        // Run decay
        system.decay_pressure(&mut chunks, &active_chunks);

        // Check that pressure decreased
        let chunk = chunks.get(&chunk_pos).unwrap();
        assert!(chunk.pressure[0] < 50.0, "Pressure should decay");
    }

    #[test]
    fn test_pressure_system_basic_flow() {
        let mut system = PressureSystem::new();
        let materials = Materials::new();
        let mut chunks = HashMap::new();
        let chunk_pos = IVec2::new(0, 0);

        // Create a chunk
        let chunk = Chunk::new(0, 0);
        chunks.insert(chunk_pos, chunk);
        let active_chunks = vec![chunk_pos];

        // Should not crash when updating empty chunk
        system.update(&mut chunks, &active_chunks, &materials);

        // Pressure should remain at default (0.0 or minimal)
        let chunk = chunks.get(&chunk_pos).unwrap();
        assert!(
            chunk.pressure[0] <= 1.0,
            "Empty chunk should have minimal pressure"
        );
    }
}
