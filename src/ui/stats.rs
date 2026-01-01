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
        }
    }
}

/// Stats collector with timing and aggregation
pub struct StatsCollector {
    stats: SimulationStats,
    frame_times: VecDeque<f32>,
    last_frame_instant: Instant,
    sim_start: Option<Instant>,
}

impl StatsCollector {
    pub fn new() -> Self {
        Self {
            stats: SimulationStats::default(),
            frame_times: VecDeque::with_capacity(60),
            last_frame_instant: Instant::now(),
            sim_start: None,
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
            self.stats.sim_time_ms = start.elapsed().as_secs_f32() * 1000.0;
        }
    }

    /// Collect world statistics
    pub fn collect_world_stats(&mut self, world: &World) {
        self.stats.total_chunks = world.chunks().len();
        self.stats.active_chunks = world.active_chunks().count();

        // Count active pixels
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
