//! Training environment for creature evolution
//!
//! Main training loop with parallel evaluation and checkpointing.

use std::sync::atomic::{AtomicUsize, Ordering};

use anyhow::{Context, Result};
use rayon::prelude::*;

use crate::creature::genome::{crossover_genome, CreatureGenome, MutationConfig};
use crate::creature::spawning::CreatureManager;
use crate::physics::PhysicsWorld;
use crate::simulation::Materials;
use crate::ui::StatsCollector;

use super::fitness::BehaviorDescriptor;
use super::map_elites::MapElitesGrid;
use super::report::ReportGenerator;
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
    /// Materials registry
    materials: Materials,
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
        let materials = Materials::new();
        let report_gen = ReportGenerator::new(&config.output_dir, &scenario.config);

        Self {
            config,
            scenario,
            grid: MapElitesGrid::default_grid(),
            materials,
            generation: 0,
            stats_history: Vec::new(),
            report_gen,
        }
    }

    /// Run the full training loop
    pub fn run(&mut self) -> Result<()> {
        log::info!(
            "Starting training: {} generations, {} population",
            self.config.generations,
            self.config.population_size
        );

        // Initialize with random population
        self.initialize_population()?;

        // Main training loop
        for gen in 0..self.config.generations {
            self.generation = gen;

            // Generate offspring population
            let offspring = self.generate_offspring();

            // Evaluate offspring in parallel
            let results = self.evaluate_population(&offspring)?;

            // Update grid and collect stats
            let stats = self.update_grid(results);
            self.stats_history.push(stats.clone());

            // Log progress
            log::info!(
                "Gen {}: best={:.2}, avg={:.2}, coverage={:.1}%, new={}",
                gen,
                stats.best_fitness,
                stats.avg_fitness,
                stats.grid_coverage * 100.0,
                stats.new_elites
            );

            // Checkpoint
            if self.config.checkpoint_interval > 0 && gen % self.config.checkpoint_interval == 0 {
                self.save_checkpoint()?;
            }
        }

        // Final report
        self.report_gen
            .generate_final_report(&self.grid, &self.stats_history)?;

        log::info!("Training complete!");
        Ok(())
    }

    /// Initialize with random population
    fn initialize_population(&mut self) -> Result<()> {
        log::info!(
            "Initializing population with {} creatures",
            self.config.population_size
        );

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

        let results = self.evaluate_population(&genomes)?;

        for result in results {
            self.grid
                .try_insert(result.genome, result.fitness, &result.behavior, 0);
        }

        log::info!(
            "Initial grid coverage: {:.1}%",
            self.grid.coverage() * 100.0
        );
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
    fn evaluate_population(&self, genomes: &[CreatureGenome]) -> Result<Vec<EvalResult>> {
        let counter = AtomicUsize::new(0);
        let total = genomes.len();

        let results: Vec<EvalResult> = genomes
            .par_iter()
            .map(|genome| {
                let result = self.evaluate_single(genome.clone());

                let done = counter.fetch_add(1, Ordering::Relaxed) + 1;
                if done % 10 == 0 {
                    log::debug!("Evaluated {}/{} creatures", done, total);
                }

                result
            })
            .collect();

        Ok(results)
    }

    /// Evaluate a single creature
    fn evaluate_single(&self, genome: CreatureGenome) -> EvalResult {
        // Set up world
        let mut world = self.scenario.setup_world();
        let mut physics_world = PhysicsWorld::new();
        let mut creature_manager = CreatureManager::new(1);
        let mut stats_collector = StatsCollector::new();

        // Spawn creature
        let spawn_pos = self.scenario.config.spawn_position;
        let creature_id =
            creature_manager.spawn_creature(genome.clone(), spawn_pos, &mut physics_world);

        // Run simulation
        let dt = 1.0 / 60.0;
        let steps = (self.config.eval_duration / dt) as usize;

        for _step in 0..steps {
            // Update simulation
            world.update(dt, &mut stats_collector);
            creature_manager.update(dt, &world, &mut physics_world);
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
    fn save_checkpoint(&self) -> Result<()> {
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

        log::info!("Saved checkpoint at generation {}", self.generation);
        Ok(())
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
