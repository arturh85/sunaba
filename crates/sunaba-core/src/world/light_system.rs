//! Light system - day/night cycle, light propagation, and growth timers

use glam::IVec2;

use super::chunk_manager::ChunkManager;
use crate::simulation::{LightPropagation, Materials};

/// Day/night cycle duration in seconds (1200s = 20 minutes)
const DAY_NIGHT_CYCLE_DURATION: f32 = 1200.0;

/// Growth cycle duration in seconds (resources regenerate every 10s)
const GROWTH_CYCLE_DURATION: f32 = 10.0;

/// Light propagation update rate (15fps throttled to reduce cost)
const LIGHT_TIMESTEP: f32 = 1.0 / 15.0;

/// Manages lighting, day/night cycles, and growth timers
pub struct LightSystem {
    /// Light propagation system
    light_propagation: LightPropagation,

    /// Light propagation time accumulator (15fps throttled)
    light_time_accumulator: f32,

    /// Day/night cycle time (in seconds, 0-1200)
    pub day_night_time: f32,

    /// Growth cycle timer (0-10 seconds, wraps) for tooltip progress display
    growth_timer: f32,
}

impl LightSystem {
    pub fn new() -> Self {
        Self {
            light_propagation: LightPropagation::new(),
            light_time_accumulator: 0.0,
            day_night_time: 600.0, // Start at noon (midpoint of 0-1200)
            growth_timer: 0.0,
        }
    }

    /// Update day/night cycle and growth timer
    pub fn update(&mut self, dt: f32) {
        // Update day/night cycle
        self.day_night_time = (self.day_night_time + dt) % DAY_NIGHT_CYCLE_DURATION;

        // Update growth timer
        self.growth_timer = (self.growth_timer + dt) % GROWTH_CYCLE_DURATION;
    }

    /// Update light propagation (throttled to 15fps)
    pub fn update_light_propagation(
        &mut self,
        chunk_manager: &mut ChunkManager,
        materials: &Materials,
        active_chunks: &[IVec2],
    ) {
        self.light_time_accumulator += 1.0 / 60.0; // Fixed timestep
        if self.light_time_accumulator >= LIGHT_TIMESTEP {
            let sky_light = self.calculate_sky_light();
            self.light_propagation.propagate_light(
                &mut chunk_manager.chunks,
                materials,
                sky_light,
                active_chunks,
            );
            self.light_time_accumulator -= LIGHT_TIMESTEP;
        }
    }

    /// Calculate current sky light level based on day/night cycle (0-15)
    pub fn calculate_sky_light(&self) -> u8 {
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
    pub fn initialize_light(
        &mut self,
        chunk_manager: &mut ChunkManager,
        materials: &Materials,
        active_chunks: &[IVec2],
    ) {
        let sky_light = self.calculate_sky_light();
        self.light_propagation.propagate_light(
            &mut chunk_manager.chunks,
            materials,
            sky_light,
            active_chunks,
        );
        log::info!("Initialized light propagation (sky_light={})", sky_light);
    }

    /// Get light level at world coordinates (0-15)
    pub fn get_light_at(&self, chunk_manager: &ChunkManager, world_x: i32, world_y: i32) -> Option<u8> {
        let (chunk_pos, local_x, local_y) = ChunkManager::world_to_chunk_coords(world_x, world_y);
        chunk_manager
            .chunks
            .get(&chunk_pos)
            .map(|c| c.get_light(local_x, local_y))
    }

    /// Set light level at world coordinates (0-15)
    pub fn set_light_at(&mut self, chunk_manager: &mut ChunkManager, world_x: i32, world_y: i32, level: u8) {
        let (chunk_pos, local_x, local_y) = ChunkManager::world_to_chunk_coords(world_x, world_y);
        if let Some(chunk) = chunk_manager.chunks.get_mut(&chunk_pos) {
            chunk.set_light(local_x, local_y, level);
        }
    }

    /// Get growth progress as percentage (0-100) through the 10-second cycle
    pub fn get_growth_progress_percent(&self) -> f32 {
        (self.growth_timer / GROWTH_CYCLE_DURATION) * 100.0
    }
}

impl Default for LightSystem {
    fn default() -> Self {
        Self::new()
    }
}
