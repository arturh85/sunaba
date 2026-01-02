//! Training environment for creature evolution
//!
//! Main training loop with parallel evaluation and checkpointing.

use anyhow::{Context, Result};
use indicatif::{ProgressBar, ProgressStyle};
use rayon::prelude::*;

use crate::creature::genome::{crossover_genome, CreatureGenome, MutationConfig};
use crate::creature::spawning::CreatureManager;
use crate::physics::PhysicsWorld;
use crate::simulation::Materials;

use super::fitness::BehaviorDescriptor;
use super::gif_capture::GifCapture;
use super::map_elites::MapElitesGrid;
use super::pixel_renderer::PixelRenderer;
use super::report::{CapturedGif, ReportGenerator};
use super::scenario::Scenario;

/// Configuration for the training run
#[derive(Debug, Clone)]
pub struct TrainingConfig {
    /// Number of generations to run
    pub generations: usize,
    /// Population size per generation
    pub population_size: usize,
    /// Evaluation duration per creature (seconds)
    pub eval_duration: f32,
    /// Mutation configuration
    pub mutation_config: MutationConfig,
    /// Controller mutation rate
    pub controller_mutation_rate: f32,
    /// How often to save checkpoints (every N generations)
    pub checkpoint_interval: usize,
    /// How often to capture GIFs (every N generations, 0 = only at end)
    pub gif_capture_interval: usize,
    /// GIF viewport size
    pub gif_size: u16,
    /// GIF frames per second
    pub gif_fps: u16,
    /// Output directory for reports
    pub output_dir: String,
}

impl Default for TrainingConfig {
    fn default() -> Self {
        Self {
            generations: 100,
            population_size: 50,
            eval_duration: 30.0,
            mutation_config: MutationConfig::default(),
            controller_mutation_rate: 0.5,
            checkpoint_interval: 10,
            gif_capture_interval: 10,
            gif_size: 128,
            gif_fps: 10,
            output_dir: "training_output".to_string(),
        }
    }
}

/// Statistics from a training run
#[derive(Debug, Clone)]
pub struct TrainingStats {
    /// Current generation
    pub generation: usize,
    /// Best fitness so far
    pub best_fitness: f32,
    /// Average fitness this generation
    pub avg_fitness: f32,
    /// Number of cells filled in MAP-Elites grid
    pub grid_coverage: f32,
    /// New elites discovered this generation
    pub new_elites: usize,
}

/// Single creature evaluation result
struct EvalResult {
    genome: CreatureGenome,
    fitness: f32,
    behavior: BehaviorDescriptor,
}

/// Main training environment
pub struct TrainingEnv {
    /// Training configuration
    pub config: TrainingConfig,
    /// Training scenario
    pub scenario: Scenario,
    /// MAP-Elites grid
    pub grid: MapElitesGrid,
    /// Current generation
    generation: usize,
    /// Statistics history
    pub stats_history: Vec<TrainingStats>,
    /// Report generator
    report_gen: ReportGenerator,
}

impl TrainingEnv {
    /// Create a new training environment
    pub fn new(config: TrainingConfig, scenario: Scenario) -> Self {
        let report_gen = ReportGenerator::new(&config.output_dir, &scenario.config);

        Self {
            config,
            scenario,
            grid: MapElitesGrid::default_grid(),
            generation: 0,
            stats_history: Vec::new(),
            report_gen,
        }
    }

    /// Create a progress bar style
    fn progress_style() -> ProgressStyle {
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta}) {msg}")
            .unwrap()
            .progress_chars("█▓░")
    }

    /// Run the full training loop
    pub fn run(&mut self) -> Result<()> {
        // Calculate total evaluations: init population + generations * population
        let total_evals = (self.config.generations as u64 + 1) * self.config.population_size as u64;

        // Create main progress bar for entire training
        let pb = ProgressBar::new(total_evals);
        pb.set_style(Self::progress_style());
        pb.enable_steady_tick(std::time::Duration::from_millis(100));

        pb.println(format!(
            "Starting training: {} generations, {} population",
            self.config.generations, self.config.population_size
        ));

        // Initialize with random population
        self.initialize_population_with_progress(&pb)?;

        // Main training loop
        for gen in 0..self.config.generations {
            self.generation = gen;

            pb.println(format!(
                "=== Generation {}/{} ===",
                gen + 1,
                self.config.generations
            ));

            // Generate offspring population
            let offspring = self.generate_offspring();

            // Evaluate offspring in parallel
            let results = self.evaluate_population_with_progress(&offspring, &pb)?;

            // Update grid and collect stats
            let stats = self.update_grid(results);
            self.stats_history.push(stats.clone());

            // Log progress
            pb.println(format!(
                "Gen {}: best={:.2}, avg={:.2}, coverage={:.1}%, new={}",
                gen,
                stats.best_fitness,
                stats.avg_fitness,
                stats.grid_coverage * 100.0,
                stats.new_elites
            ));

            // Checkpoint
            if self.config.checkpoint_interval > 0 && gen % self.config.checkpoint_interval == 0 {
                self.save_checkpoint(&pb)?;
            }
        }

        // Capture GIFs of evolved creatures
        let gifs = self.capture_all_gifs(&pb);

        // Final report with GIFs
        self.report_gen
            .generate_final_report(&self.grid, &self.stats_history, &gifs)?;

        pb.finish_with_message("Training complete!");
        Ok(())
    }

    /// Initialize with random population
    fn initialize_population_with_progress(&mut self, pb: &ProgressBar) -> Result<()> {
        pb.println(format!(
            "Initializing population with {} creatures",
            self.config.population_size
        ));

        // Use test_biped as a starting point and mutate for variety
        let genomes: Vec<CreatureGenome> = (0..self.config.population_size)
            .map(|_| {
                let mut genome = CreatureGenome::test_biped();
                genome.mutate(
                    &self.config.mutation_config,
                    self.config.controller_mutation_rate,
                );
                genome
            })
            .collect();

        let results = self.evaluate_population_with_progress(&genomes, pb)?;

        for result in results {
            self.grid
                .try_insert(result.genome, result.fitness, &result.behavior, 0);
        }

        pb.println(format!(
            "Initial grid coverage: {:.1}%",
            self.grid.coverage() * 100.0
        ));
        Ok(())
    }

    /// Generate offspring from current grid
    fn generate_offspring(&self) -> Vec<CreatureGenome> {
        let mut offspring = Vec::with_capacity(self.config.population_size);

        for _ in 0..self.config.population_size {
            let child = if let Some((parent1, parent2)) = self.grid.sample_parents() {
                // Crossover
                let mut child = crossover_genome(
                    &parent1.genome,
                    &parent2.genome,
                    parent1.fitness,
                    parent2.fitness,
                );
                child.mutate(
                    &self.config.mutation_config,
                    self.config.controller_mutation_rate,
                );
                child
            } else if let Some(parent) = self.grid.sample_elite() {
                // Mutation only
                let mut child = parent.genome.clone();
                child.mutate(
                    &self.config.mutation_config,
                    self.config.controller_mutation_rate,
                );
                child
            } else {
                // Random (shouldn't happen after initialization)
                let mut genome = CreatureGenome::test_biped();
                genome.mutate(
                    &self.config.mutation_config,
                    self.config.controller_mutation_rate,
                );
                genome
            };

            offspring.push(child);
        }

        offspring
    }

    /// Evaluate a population of genomes in parallel
    fn evaluate_population_with_progress(
        &self,
        genomes: &[CreatureGenome],
        pb: &ProgressBar,
    ) -> Result<Vec<EvalResult>> {
        let results: Vec<EvalResult> = genomes
            .par_iter()
            .map(|genome| {
                let result = self.evaluate_single(genome.clone());
                pb.inc(1);
                result
            })
            .collect();

        Ok(results)
    }

    /// Evaluate a single creature
    fn evaluate_single(&self, genome: CreatureGenome) -> EvalResult {
        // Set up world with cached food positions
        let (world, food_positions) = self.scenario.setup_world();
        let mut physics_world = PhysicsWorld::new();
        let mut creature_manager = CreatureManager::new(1);

        // Spawn creature (with partial hunger for Parcour scenario)
        let spawn_pos = self.scenario.config.spawn_position;
        let creature_id = if self.scenario.config.name == "Parcour" {
            // Start with 50% hunger for parcour - creates survival pressure
            creature_manager.spawn_creature_with_hunger(
                genome.clone(),
                spawn_pos,
                0.5,
                &mut physics_world,
            )
        } else {
            creature_manager.spawn_creature(genome.clone(), spawn_pos, &mut physics_world)
        };

        // Run simulation (physics only - skip world.update() for speed)
        let dt = 1.0 / 60.0;
        let steps = (self.config.eval_duration / dt) as usize;

        // Sensory update frequency: every 6 frames (10Hz instead of 60Hz)
        const SENSORY_SKIP: usize = 6;

        for step in 0..steps {
            // Only update sensory at 10Hz for performance (use cached positions)
            if step % SENSORY_SKIP == 0 {
                creature_manager.update_with_cache(
                    dt * SENSORY_SKIP as f32,
                    &world,
                    &mut physics_world,
                    &food_positions,
                );
            }
            physics_world.step();
        }

        // Get final creature state for evaluation
        let creature = creature_manager.get(creature_id);

        let (fitness, behavior) = if let Some(creature) = creature {
            let fitness = self.scenario.fitness.evaluate(
                creature,
                &world,
                spawn_pos,
                self.config.eval_duration,
            );
            let behavior = BehaviorDescriptor::from_evaluation(
                creature,
                spawn_pos,
                self.config.eval_duration,
                &world,
            );
            (fitness, behavior)
        } else {
            // Creature died
            (
                0.0,
                BehaviorDescriptor {
                    locomotion_efficiency: 0.0,
                    foraging_efficiency: 0.0,
                    exploration: 0.0,
                    activity: 0.0,
                },
            )
        };

        EvalResult {
            genome,
            fitness,
            behavior,
        }
    }

    /// Update grid with evaluation results
    fn update_grid(&mut self, results: Vec<EvalResult>) -> TrainingStats {
        let mut new_elites = 0;
        let mut total_fitness = 0.0;

        for result in &results {
            if self.grid.try_insert(
                result.genome.clone(),
                result.fitness,
                &result.behavior,
                self.generation,
            ) {
                new_elites += 1;
            }
            total_fitness += result.fitness;
        }

        let stats = self.grid.stats();

        TrainingStats {
            generation: self.generation,
            best_fitness: stats.best_fitness,
            avg_fitness: if results.is_empty() {
                0.0
            } else {
                total_fitness / results.len() as f32
            },
            grid_coverage: stats.coverage,
            new_elites,
        }
    }

    /// Save a checkpoint
    fn save_checkpoint(&self, pb: &ProgressBar) -> Result<()> {
        let checkpoint_dir = format!("{}/checkpoints", self.config.output_dir);
        std::fs::create_dir_all(&checkpoint_dir)
            .context("Failed to create checkpoint directory")?;

        // Save best genome
        if let Some(best) = self.grid.best_elite() {
            let path = format!("{}/gen_{:04}_best.genome", checkpoint_dir, self.generation);
            let data =
                bincode_next::serde::encode_to_vec(&best.genome, bincode_next::config::standard())
                    .context("Failed to serialize genome")?;
            std::fs::write(&path, data).context("Failed to write genome file")?;
        }

        pb.println(format!(
            "Saved checkpoint at generation {}",
            self.generation
        ));
        Ok(())
    }

    /// Capture a GIF of a creature running in the scenario
    fn capture_elite_gif(
        &self,
        genome: &CreatureGenome,
        label: &str,
        fitness: f32,
        behavior: &[f32],
    ) -> Result<CapturedGif> {
        let size = self.config.gif_size as usize;
        let mut gif = GifCapture::new(
            self.config.gif_size,
            self.config.gif_size,
            self.config.gif_fps,
        );
        let mut renderer = PixelRenderer::new(size, size);
        let materials = Materials::new();

        // Set up world with cached food positions
        let (world, food_positions) = self.scenario.setup_world();
        let mut physics_world = PhysicsWorld::new();
        let mut creature_manager = CreatureManager::new(1);
        let spawn_pos = self.scenario.config.spawn_position;
        let creature_id = if self.scenario.config.name == "Parcour" {
            creature_manager.spawn_creature_with_hunger(
                genome.clone(),
                spawn_pos,
                0.5,
                &mut physics_world,
            )
        } else {
            creature_manager.spawn_creature(genome.clone(), spawn_pos, &mut physics_world)
        };

        // Simulation with frame capture
        let dt = 1.0 / 60.0;
        let fps = self.config.gif_fps as usize;
        let capture_interval = if fps > 0 { 60 / fps } else { 6 }; // frames between captures

        // Sensory update frequency: every 6 frames (10Hz instead of 60Hz)
        const SENSORY_SKIP: usize = 6;

        // Capture for a shorter duration for GIFs (max 5 seconds)
        let gif_duration = self.config.eval_duration.min(5.0);
        let total_steps = (gif_duration / dt) as usize;

        for step in 0..total_steps {
            // Only update sensory at 10Hz for performance
            if step % SENSORY_SKIP == 0 {
                creature_manager.update_with_cache(
                    dt * SENSORY_SKIP as f32,
                    &world,
                    &mut physics_world,
                    &food_positions,
                );
            }
            physics_world.step();

            // Capture frame at intervals
            if step % capture_interval == 0 {
                if let Some(creature) = creature_manager.get(creature_id) {
                    let render_data = creature.get_render_data(&physics_world);
                    let creatures: Vec<_> = render_data.into_iter().collect();

                    // Center camera on creature
                    let center = creature.position;
                    renderer.render(&world, &materials, center, &creatures);

                    // Draw debug overlays
                    let half = (size / 2) as i32;

                    // Draw vertical reference lines every 50 pixels for motion visibility
                    let gray = [100, 100, 100, 200];
                    for world_x in (-500i32..500).step_by(50) {
                        let screen_x = half + world_x - center.x as i32;
                        if screen_x >= 0 && screen_x < size as i32 {
                            renderer.draw_dashed_vline(screen_x, 3, 5, gray);
                        }
                    }

                    // Draw ground level indicator (y=20 is ground in locomotion scenario)
                    let ground_y = 20.0;
                    let ground_screen_y = half - (ground_y - center.y) as i32;
                    if ground_screen_y >= 0 && ground_screen_y < size as i32 {
                        renderer.draw_dashed_hline(ground_screen_y, 5, 3, [139, 69, 19, 200]);
                        // Brown
                    }

                    // Spawn position marker (red dot showing where creature started)
                    let spawn_screen_x = half + (spawn_pos.x - center.x) as i32;
                    let spawn_screen_y = half - (spawn_pos.y - center.y) as i32;
                    if spawn_screen_x >= -10
                        && spawn_screen_x < size as i32 + 10
                        && spawn_screen_y >= -10
                        && spawn_screen_y < size as i32 + 10
                    {
                        renderer.draw_filled_circle(
                            spawn_screen_x,
                            spawn_screen_y,
                            4,
                            [255, 0, 0, 255],
                        );
                    }

                    // Velocity arrow (green) from creature center
                    let vel = creature.velocity;
                    renderer.draw_arrow(half, half, vel.x, vel.y, 3.0, [0, 255, 0, 255]);

                    // Speed text (white on dark background area - top left)
                    let speed = vel.length();
                    let speed_text = format!("SPD:{:.0}", speed);
                    renderer.draw_text(4, 4, &speed_text, [255, 255, 255, 255]);

                    // Distance from spawn (displacement)
                    let dist = (creature.position - spawn_pos).length();
                    let dist_text = format!("DST:{:.0}", dist);
                    renderer.draw_text(4, 12, &dist_text, [255, 255, 255, 255]);

                    // Current position
                    let pos_text = format!("X:{:.0}", creature.position.x);
                    renderer.draw_text(4, 20, &pos_text, [255, 255, 255, 255]);

                    // Food counter (white)
                    let food_text = format!("FOOD:{}", creature.food_eaten);
                    renderer.draw_text(4, 28, &food_text, [255, 255, 255, 255]);

                    // Timestamp overlay (yellow, top right) - shows where GIF starts/loops
                    let elapsed_time = step as f32 * dt;
                    let time_text = format!("T:{:.1}s", elapsed_time);
                    renderer.draw_text(size as i32 - 42, 4, &time_text, [255, 255, 100, 255]);

                    gif.capture_frame(&renderer);
                }
            }
        }

        // Encode GIF to bytes
        let data = gif.to_bytes().context("Failed to encode GIF")?;

        Ok(CapturedGif {
            label: label.to_string(),
            fitness,
            behavior: behavior.to_vec(),
            data,
        })
    }

    /// Capture GIFs for the best and diverse elites
    fn capture_all_gifs(&self, pb: &ProgressBar) -> Vec<CapturedGif> {
        let mut gifs = Vec::new();

        pb.println("Capturing GIFs of evolved creatures...");

        // Capture best elite
        if let Some(best) = self.grid.best_elite() {
            pb.println("  Capturing: Champion (best fitness)");
            match self.capture_elite_gif(&best.genome, "Champion", best.fitness, &best.behavior) {
                Ok(gif) => gifs.push(gif),
                Err(e) => log::warn!("Failed to capture champion GIF: {}", e),
            }
        }

        // Capture diverse elites
        let diverse = self.grid.sample_diverse_elites();
        for diverse_elite in diverse {
            pb.println(format!("  Capturing: {}", diverse_elite.label));
            match self.capture_elite_gif(
                &diverse_elite.elite.genome,
                &diverse_elite.label,
                diverse_elite.elite.fitness,
                &diverse_elite.elite.behavior,
            ) {
                Ok(gif) => gifs.push(gif),
                Err(e) => log::warn!("Failed to capture {} GIF: {}", diverse_elite.label, e),
            }
        }

        pb.println(format!("Captured {} GIFs", gifs.len()));
        gifs
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_training_config_default() {
        let config = TrainingConfig::default();
        assert_eq!(config.generations, 100);
        assert_eq!(config.population_size, 50);
        assert!((config.eval_duration - 30.0).abs() < 0.01);
    }

    #[test]
    fn test_training_env_creation() {
        let config = TrainingConfig {
            generations: 5,
            population_size: 10,
            ..Default::default()
        };
        let scenario = Scenario::locomotion();
        let env = TrainingEnv::new(config, scenario);

        assert_eq!(env.generation, 0);
        assert!(env.stats_history.is_empty());
    }
}
