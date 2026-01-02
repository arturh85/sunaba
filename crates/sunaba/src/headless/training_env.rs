//! Training environment for creature evolution
//!
//! Main training loop with parallel evaluation and checkpointing.

use anyhow::{Context, Result};
use indicatif::{ProgressBar, ProgressStyle};
use rayon::prelude::*;

use crate::creature::genome::{CreatureGenome, MutationConfig, crossover_genome};
use crate::creature::morphology::{CreatureMorphology, MorphologyConfig};
use crate::creature::spawning::CreatureManager;
use crate::creature::viability::analyze_viability;
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
    /// Use simple morphology (fewer body parts, viability filter)
    pub use_simple_morphology: bool,
    /// Minimum viability score to accept a creature (0.0-1.0)
    pub min_viability: f32,
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
            use_simple_morphology: false,
            min_viability: 0.3,
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
    /// Maximum displacement this generation (for debugging)
    pub max_displacement: f32,
    /// Average displacement this generation
    pub avg_displacement: f32,
}

/// Single creature evaluation result
struct EvalResult {
    genome: CreatureGenome,
    fitness: f32,
    behavior: BehaviorDescriptor,
    /// Actual displacement from spawn position (for debugging)
    displacement: f32,
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
    /// Morphology configuration (simple or default)
    morphology_config: MorphologyConfig,
}

impl TrainingEnv {
    /// Create a new training environment
    pub fn new(config: TrainingConfig, scenario: Scenario) -> Self {
        let report_gen = ReportGenerator::new(&config.output_dir, &scenario.config);
        let morphology_config = if config.use_simple_morphology {
            MorphologyConfig::simple()
        } else {
            MorphologyConfig::default()
        };

        Self {
            config,
            scenario,
            grid: MapElitesGrid::default_grid(),
            generation: 0,
            stats_history: Vec::new(),
            report_gen,
            morphology_config,
        }
    }

    /// Check if a genome produces a viable morphology
    fn is_viable(&self, genome: &CreatureGenome) -> bool {
        let morphology = CreatureMorphology::from_genome(genome, &self.morphology_config);
        let viability = analyze_viability(&morphology);
        viability.overall >= self.config.min_viability && viability.has_locomotion
    }

    /// Generate a viable genome (retries until viability threshold met)
    fn generate_viable_genome(&self) -> CreatureGenome {
        const MAX_ATTEMPTS: usize = 100;

        for _ in 0..MAX_ATTEMPTS {
            let mut genome = CreatureGenome::test_biped();
            genome.mutate(
                &self.config.mutation_config,
                self.config.controller_mutation_rate,
            );

            if self.is_viable(&genome) {
                return genome;
            }
        }

        // Fallback: return test_biped without mutation (known to be viable)
        CreatureGenome::test_biped()
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
        for generation_num in 0..self.config.generations {
            self.generation = generation_num;

            if generation_num % 5 == 0 {
                pb.println(format!(
                    "=== Generation {}/{} ===",
                    generation_num + 1,
                    self.config.generations
                ));
            }

            // Generate offspring population
            let offspring = self.generate_offspring();

            // Evaluate offspring in parallel
            let results = self.evaluate_population_with_progress(&offspring, &pb)?;

            // Update grid and collect stats
            let stats = self.update_grid(results);
            self.stats_history.push(stats.clone());

            // Log progress
            if generation_num % 5 == 0 {
                pb.println(format!(
                    "Gen {}: best={:.2}, avg={:.2}, coverage={:.1}%, new={}, disp_max={:.1}px, disp_avg={:.1}px",
                    generation_num,
                    stats.best_fitness,
                    stats.avg_fitness,
                    stats.grid_coverage * 100.0,
                    stats.new_elites,
                    stats.max_displacement,
                    stats.avg_displacement,
                ));
            }

            // Checkpoint
            if self.config.checkpoint_interval > 0
                && generation_num % self.config.checkpoint_interval == 0
            {
                self.save_checkpoint(&pb)?;
            }
        }

        // Evaluate champion displacement for verification with detailed logging
        let champion_displacement = if let Some(best) = self.grid.best_elite() {
            // Set up world and evaluate champion with position tracking
            let (world, food_positions) = self.scenario.setup_world();
            let mut physics_world = PhysicsWorld::new();
            let mut creature_manager = CreatureManager::new(1);
            let spawn_pos = self.scenario.config.spawn_position;

            let creature_id = creature_manager.spawn_creature_with_config(
                best.genome.clone(),
                spawn_pos,
                &mut physics_world,
                &self.morphology_config,
            );

            // Log initial state
            pb.println(format!("=== Champion Debug ==="));
            pb.println(format!(
                "  Spawn position: ({:.1}, {:.1})",
                spawn_pos.x, spawn_pos.y
            ));

            // Run simulation and track positions
            let dt = 1.0 / 60.0;
            let steps = (self.config.eval_duration / dt) as usize;
            const SENSORY_SKIP: usize = 6;

            let mut min_x = spawn_pos.x;
            let mut max_x = spawn_pos.x;
            let mut pos_samples: Vec<glam::Vec2> = Vec::new();

            for step in 0..steps {
                if step % SENSORY_SKIP == 0 {
                    creature_manager.update_with_cache(
                        dt * SENSORY_SKIP as f32,
                        &world,
                        &mut physics_world,
                        &food_positions,
                    );
                }
                physics_world.step();

                // Sample position every 5 seconds
                if step % (60 * 5) == 0 {
                    if let Some(creature) = creature_manager.get(creature_id) {
                        pos_samples.push(creature.position);
                        min_x = min_x.min(creature.position.x);
                        max_x = max_x.max(creature.position.x);
                    }
                }
            }

            // Get final position
            let final_displacement = if let Some(creature) = creature_manager.get(creature_id) {
                let disp = (creature.position - spawn_pos).length();
                let debug_info = format!(
                    "=== Champion Debug ===\n\
                     Spawn position: ({:.1}, {:.1})\n\
                     Final position: ({:.1}, {:.1})\n\
                     Displacement: {:.1}px\n\
                     X range: {:.1} to {:.1} (width: {:.1})\n\
                     Position samples: {:?}",
                    spawn_pos.x,
                    spawn_pos.y,
                    creature.position.x,
                    creature.position.y,
                    disp,
                    min_x,
                    max_x,
                    max_x - min_x,
                    pos_samples
                        .iter()
                        .map(|p| format!("({:.0},{:.0})", p.x, p.y))
                        .collect::<Vec<_>>()
                );
                eprintln!("{}", debug_info);
                pb.println(debug_info.clone());

                // Also write to file for easier access
                let debug_path = format!("{}/champion_debug.txt", self.config.output_dir);
                let _ = std::fs::write(&debug_path, &debug_info);

                disp
            } else {
                0.0
            };

            let result = self.evaluate_single(best.genome.clone());
            let eval_info = format!(
                "Fitness from evaluate_single: {:.2}\n\
                 Displacement from evaluate_single: {:.1}px",
                result.fitness, result.displacement
            );
            eprintln!("{}", eval_info);
            pb.println(eval_info);

            final_displacement
        } else {
            0.0
        };

        // Check movement threshold (25px = half distance to first food in parcour)
        const MOVEMENT_THRESHOLD: f32 = 25.0;
        if champion_displacement < MOVEMENT_THRESHOLD {
            pb.println(format!(
                "⚠️  WARNING: Champion displacement ({:.1}px) below threshold ({:.0}px)!",
                champion_displacement, MOVEMENT_THRESHOLD
            ));
            pb.println("   Creatures may not be moving properly.");
        } else {
            pb.println(format!(
                "✓ Champion displacement ({:.1}px) meets threshold ({:.0}px)",
                champion_displacement, MOVEMENT_THRESHOLD
            ));
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
        let mode = if self.config.use_simple_morphology {
            "simple morphology + viability filter"
        } else {
            "default morphology"
        };
        pb.println(format!(
            "Initializing population with {} creatures ({})",
            self.config.population_size, mode
        ));

        // Generate viable genomes (or default if simple morphology disabled)
        let genomes: Vec<CreatureGenome> = if self.config.use_simple_morphology {
            (0..self.config.population_size)
                .map(|_| self.generate_viable_genome())
                .collect()
        } else {
            // Use test_biped as a starting point and mutate for variety
            (0..self.config.population_size)
                .map(|_| {
                    let mut genome = CreatureGenome::test_biped();
                    genome.mutate(
                        &self.config.mutation_config,
                        self.config.controller_mutation_rate,
                    );
                    genome
                })
                .collect()
        };

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
            // Try to generate a viable child (up to 10 attempts if using simple morphology)
            let max_attempts = if self.config.use_simple_morphology {
                10
            } else {
                1
            };

            let mut child = None;
            for _ in 0..max_attempts {
                let candidate = if let Some((parent1, parent2)) = self.grid.sample_parents() {
                    // Crossover
                    let mut c = crossover_genome(
                        &parent1.genome,
                        &parent2.genome,
                        parent1.fitness,
                        parent2.fitness,
                    );
                    c.mutate(
                        &self.config.mutation_config,
                        self.config.controller_mutation_rate,
                    );
                    c
                } else if let Some(parent) = self.grid.sample_elite() {
                    // Mutation only
                    let mut c = parent.genome.clone();
                    c.mutate(
                        &self.config.mutation_config,
                        self.config.controller_mutation_rate,
                    );
                    c
                } else {
                    // Random (shouldn't happen after initialization)
                    let mut genome = CreatureGenome::test_biped();
                    genome.mutate(
                        &self.config.mutation_config,
                        self.config.controller_mutation_rate,
                    );
                    genome
                };

                // Check viability if using simple morphology
                if !self.config.use_simple_morphology || self.is_viable(&candidate) {
                    child = Some(candidate);
                    break;
                }
            }

            // Use fallback if no viable child found
            offspring.push(child.unwrap_or_else(|| self.generate_viable_genome()));
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

        // Spawn creature using the configured morphology
        let spawn_pos = self.scenario.config.spawn_position;
        let creature_id = if self.scenario.config.name == "Parcour" {
            // Start with 50% hunger for parcour - creates survival pressure
            creature_manager.spawn_creature_with_hunger_and_config(
                genome.clone(),
                spawn_pos,
                0.5,
                &mut physics_world,
                &self.morphology_config,
            )
        } else {
            creature_manager.spawn_creature_with_config(
                genome.clone(),
                spawn_pos,
                &mut physics_world,
                &self.morphology_config,
            )
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

        let (fitness, behavior, displacement) = if let Some(creature) = creature {
            let displacement = (creature.position - spawn_pos).length();
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
            (fitness, behavior, displacement)
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
                0.0,
            )
        };

        EvalResult {
            genome,
            fitness,
            behavior,
            displacement,
        }
    }

    /// Update grid with evaluation results
    fn update_grid(&mut self, results: Vec<EvalResult>) -> TrainingStats {
        let mut new_elites = 0;
        let mut total_fitness = 0.0;
        let mut total_displacement = 0.0;
        let mut max_displacement = 0.0f32;

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
            total_displacement += result.displacement;
            max_displacement = max_displacement.max(result.displacement);
        }

        let stats = self.grid.stats();
        let n = results.len() as f32;

        TrainingStats {
            generation: self.generation,
            best_fitness: stats.best_fitness,
            avg_fitness: if results.is_empty() {
                0.0
            } else {
                total_fitness / n
            },
            grid_coverage: stats.coverage,
            new_elites,
            max_displacement,
            avg_displacement: if results.is_empty() {
                0.0
            } else {
                total_displacement / n
            },
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
            creature_manager.spawn_creature_with_hunger_and_config(
                genome.clone(),
                spawn_pos,
                0.5,
                &mut physics_world,
                &self.morphology_config,
            )
        } else {
            creature_manager.spawn_creature_with_config(
                genome.clone(),
                spawn_pos,
                &mut physics_world,
                &self.morphology_config,
            )
        };

        // Simulation with frame capture
        let dt = 1.0 / 60.0;
        let fps = self.config.gif_fps as usize;
        let capture_interval = if fps > 0 { 60 / fps } else { 6 }; // frames between captures

        // Sensory update frequency: every 6 frames (10Hz instead of 60Hz)
        const SENSORY_SKIP: usize = 6;

        // Capture for full evaluation duration (up to 30 seconds)
        let gif_duration = self.config.eval_duration.min(30.0);
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
