//! Performance and simulation statistics tracking

use crate::world::World;
use instant::Instant;
use std::collections::VecDeque;

/// Simulation statistics
#[derive(Clone, Debug)]
pub struct SimulationStats {
    // Performance
    pub fps: f32,
    pub frame_time_ms: f32,
    pub sim_time_ms: f32,

    // World state
    pub active_chunks: usize,
    pub total_chunks: usize,
    pub active_pixels: usize,

    // Temperature
    pub min_temp: f32,
    pub max_temp: f32,
    pub avg_temp: f32,

    // Activity (per frame)
    pub pixels_moved: usize,
    pub state_changes: usize,
    pub reactions: usize,

    // Render stats
    pub render_dirty_chunks: usize,
    pub rendered_chunks_total: usize,

    // Render timing breakdown (ms)
    pub pixel_buffer_time_ms: f32,
    pub gpu_upload_time_ms: f32,
    pub acquire_time_ms: f32, // get_current_texture() blocking
    pub egui_time_ms: f32,
    pub present_time_ms: f32,

    // Frame loop timing (ms)
    pub egui_build_time_ms: f32, // Widget creation in egui_ctx.run()
    pub overlay_time_ms: f32,    // Temperature + light overlay updates
}

impl Default for SimulationStats {
    fn default() -> Self {
        Self {
            fps: 0.0,
            frame_time_ms: 0.0,
            sim_time_ms: 0.0,
            active_chunks: 0,
            total_chunks: 0,
            active_pixels: 0,
            min_temp: 20.0,
            max_temp: 20.0,
            avg_temp: 20.0,
            pixels_moved: 0,
            state_changes: 0,
            reactions: 0,
            render_dirty_chunks: 0,
            rendered_chunks_total: 0,
            pixel_buffer_time_ms: 0.0,
            gpu_upload_time_ms: 0.0,
            acquire_time_ms: 0.0,
            egui_time_ms: 0.0,
            present_time_ms: 0.0,
            egui_build_time_ms: 0.0,
            overlay_time_ms: 0.0,
        }
    }
}

/// Stats collector with timing and aggregation
pub struct StatsCollector {
    stats: SimulationStats,
    frame_times: VecDeque<f32>,
    sim_times: VecDeque<f32>,
    last_frame_instant: Instant,
    sim_start: Option<Instant>,
    /// Frame counter for throttling expensive stats
    frame_counter: u32,
}

impl StatsCollector {
    /// Collect expensive stats every N frames (6fps at 60fps)
    const STATS_THROTTLE_FRAMES: u32 = 10;

    pub fn new() -> Self {
        Self {
            stats: SimulationStats::default(),
            frame_times: VecDeque::with_capacity(60),
            sim_times: VecDeque::with_capacity(60),
            last_frame_instant: Instant::now(),
            sim_start: None,
            frame_counter: 0,
        }
    }

    /// Begin a new frame
    pub fn begin_frame(&mut self) {
        let now = Instant::now();
        let frame_time = self.last_frame_instant.elapsed().as_secs_f32() * 1000.0;

        self.frame_times.push_back(frame_time);
        if self.frame_times.len() > 60 {
            self.frame_times.pop_front();
        }

        // Calculate FPS from rolling average
        if !self.frame_times.is_empty() {
            let avg_frame_time: f32 =
                self.frame_times.iter().sum::<f32>() / self.frame_times.len() as f32;
            self.stats.fps = if avg_frame_time > 0.0 {
                1000.0 / avg_frame_time
            } else {
                60.0
            };
            self.stats.frame_time_ms = avg_frame_time;
        }

        self.last_frame_instant = now;

        // Reset per-frame counters
        self.stats.pixels_moved = 0;
        self.stats.state_changes = 0;
        self.stats.reactions = 0;
    }

    /// Mark start of simulation update
    pub fn begin_sim(&mut self) {
        self.sim_start = Some(Instant::now());
    }

    /// Mark end of simulation update
    pub fn end_sim(&mut self) {
        if let Some(start) = self.sim_start.take() {
            let sim_time = start.elapsed().as_secs_f32() * 1000.0;
            log::trace!("end_sim: raw sim_time = {:.2}ms", sim_time);

            // Use rolling average (same as frame_time) for consistent comparison
            self.sim_times.push_back(sim_time);
            if self.sim_times.len() > 60 {
                self.sim_times.pop_front();
            }

            if !self.sim_times.is_empty() {
                self.stats.sim_time_ms =
                    self.sim_times.iter().sum::<f32>() / self.sim_times.len() as f32;
            }
        }
    }

    /// Collect world statistics (throttled to reduce CPU overhead)
    pub fn collect_world_stats(&mut self, world: &World) {
        // Always update cheap stats
        self.stats.total_chunks = world.chunks().len();
        self.stats.active_chunks = world.active_chunks().count();

        // Throttle expensive stats collection
        self.frame_counter += 1;
        if self.frame_counter < Self::STATS_THROTTLE_FRAMES {
            return;
        }
        self.frame_counter = 0;

        // Count active pixels (expensive - ~200k iterations)
        let mut active = 0;
        for chunk in world.active_chunks() {
            for y in 0..64 {
                for x in 0..64 {
                    if !chunk.get_pixel(x, y).is_empty() {
                        active += 1;
                    }
                }
            }
        }
        self.stats.active_pixels = active;

        // Temperature stats
        self.collect_temperature_stats(world);
    }

    fn collect_temperature_stats(&mut self, world: &World) {
        let mut min = f32::MAX;
        let mut max = f32::MIN;
        let mut sum = 0.0;
        let mut count = 0;

        for chunk in world.chunks().values() {
            for &temp in &chunk.temperature {
                min = min.min(temp);
                max = max.max(temp);
                sum += temp;
                count += 1;
            }
        }

        self.stats.min_temp = if count > 0 { min } else { 20.0 };
        self.stats.max_temp = if count > 0 { max } else { 20.0 };
        self.stats.avg_temp = if count > 0 { sum / count as f32 } else { 20.0 };
    }

    /// Record a pixel movement
    pub fn record_pixel_moved(&mut self) {
        self.stats.pixels_moved += 1;
    }

    /// Record a state change
    pub fn record_state_change(&mut self) {
        self.stats.state_changes += 1;
    }

    /// Record a chemical reaction
    pub fn record_reaction(&mut self) {
        self.stats.reactions += 1;
    }

    /// Set render stats from renderer
    pub fn set_render_stats(&mut self, dirty_chunks: usize, rendered_total: usize) {
        self.stats.render_dirty_chunks = dirty_chunks;
        self.stats.rendered_chunks_total = rendered_total;
    }

    /// Set render timing breakdown
    pub fn set_render_timing(
        &mut self,
        pixel_buffer: f32,
        gpu_upload: f32,
        acquire: f32,
        egui: f32,
        present: f32,
    ) {
        self.stats.pixel_buffer_time_ms = pixel_buffer;
        self.stats.gpu_upload_time_ms = gpu_upload;
        self.stats.acquire_time_ms = acquire;
        self.stats.egui_time_ms = egui;
        self.stats.present_time_ms = present;
    }

    /// Set frame loop timing
    pub fn set_frame_loop_timing(&mut self, egui_build: f32, overlay: f32) {
        self.stats.egui_build_time_ms = egui_build;
        self.stats.overlay_time_ms = overlay;
    }

    /// Get current stats
    pub fn stats(&self) -> &SimulationStats {
        &self.stats
    }
}

impl Default for StatsCollector {
    fn default() -> Self {
        Self::new()
    }
}
