//! Headless training environment for creature evolution
//!
//! This module provides infrastructure for evolving creatures offline without GUI:
//! - Pixel buffer rendering for GIF capture
//! - Training scenarios (locomotion, foraging, survival)
//! - Fitness functions to evaluate creature performance
//! - MAP-Elites for maintaining diverse populations
//! - HTML report generation with animated visualizations
//! - Procedural terrain generation for curriculum learning

mod fitness;
mod gif_capture;
mod map_elites;
mod pixel_renderer;
mod report;
mod scenario;
mod terrain_config;
mod training_env;

pub use fitness::{
    CompositeFitness, DistanceFitness, FitnessFunction, ForagingFitness, SurvivalFitness,
};
pub use gif_capture::GifCapture;
pub use map_elites::{DiverseElite, Elite, MapElitesGrid};
pub use pixel_renderer::PixelRenderer;
pub use report::ReportGenerator;
pub use scenario::{Scenario, ScenarioConfig};
pub use terrain_config::{DifficultyConfig, TrainingTerrainConfig};
pub use training_env::{TrainingConfig, TrainingEnv, TrainingStats};
