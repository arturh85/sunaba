//! Training environment for creature evolution
//!
//! Main training loop with parallel evaluation and checkpointing.

use std::collections::HashMap;

use anyhow::{Context, Result};
use indicatif::{ProgressBar, ProgressStyle};
use rayon::prelude::*;

use crate::creature::genome::{CreatureGenome, MutationConfig, crossover_genome};
use crate::creature::morphology::{CreatureArchetype, CreatureMorphology, MorphologyConfig};
use crate::creature::spawning::CreatureManager;
use crate::creature::viability::analyze_viability;
use crate::simulation::Materials;

use super::curriculum::{CurriculumConfig, CurriculumTracker};
use super::fitness::BehaviorDescriptor;
use super::gif_capture::GifCapture;
use super::map_elites::MapElitesGrid;
use super::multi_env_eval::MultiEnvironmentEvaluator;
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
    /// Single creature archetype (legacy, overridden by archetypes if set)
    pub archetype: CreatureArchetype,
    /// Multiple creature archetypes to train together (empty = use single archetype)
    pub archetypes: Vec<CreatureArchetype>,
    /// Multi-environment evaluation (None = single environment, backward compatible)
    pub multi_env: Option<MultiEnvironmentEvaluator>,
    /// Curriculum learning (None = no curriculum, backward compatible)
    pub curriculum: Option<CurriculumConfig>,
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
            gif_size: 368,
            gif_fps: 10,
            output_dir: "training_output".to_string(),
            use_simple_morphology: false,
            min_viability: 0.3,
            archetype: CreatureArchetype::default(),
            archetypes: Vec::new(), // Empty = use single archetype field
            multi_env: None,        // None = single environment (backward compatible)
            curriculum: None,       // None = no curriculum (backward compatible)
        }
    }
}

impl TrainingConfig {
    /// Get the effective list of archetypes to train
    /// If archetypes is non-empty, use that; otherwise use single archetype
    pub fn effective_archetypes(&self) -> Vec<CreatureArchetype> {
        if self.archetypes.is_empty() {
            vec![self.archetype]
        } else {
            self.archetypes.clone()
        }
    }
}

/// Statistics from a training run
#[derive(Debug, Clone)]
pub struct TrainingStats {
    /// Current generation
    pub generation: usize,
    /// Best fitness so far (across all archetypes)
    pub best_fitness: f32,
    /// Average fitness this generation
    pub avg_fitness: f32,
    /// Average grid coverage across all archetypes
    pub grid_coverage: f32,
    /// New elites discovered this generation (across all archetypes)
    pub new_elites: usize,
    /// Maximum displacement this generation (for debugging)
    pub max_displacement: f32,
    /// Average displacement this generation
    pub avg_displacement: f32,
    /// Best fitness per archetype
    pub best_per_archetype: HashMap<CreatureArchetype, f32>,
}

/// Single creature evaluation result
struct EvalResult {
    archetype: CreatureArchetype,
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
    /// MAP-Elites grids (one per archetype)
    pub grids: HashMap<CreatureArchetype, MapElitesGrid>,
    /// Current generation
    generation: usize,
    /// Statistics history
    pub stats_history: Vec<TrainingStats>,
    /// Report generator
    report_gen: ReportGenerator,
    /// Morphology configuration (simple or default)
    morphology_config: MorphologyConfig,
    /// Archetypes being trained
    archetypes: Vec<CreatureArchetype>,
    /// Curriculum tracker (tracks progress through stages)
    curriculum_tracker: Option<CurriculumTracker>,
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
        let archetypes = config.effective_archetypes();

        // Create a separate MAP-Elites grid for each archetype
        let mut grids = HashMap::new();
        for archetype in &archetypes {
            grids.insert(*archetype, MapElitesGrid::default_grid());
        }

        // Initialize curriculum tracker if curriculum is enabled
        let curriculum_tracker = if config.curriculum.is_some() {
            Some(CurriculumTracker::new())
        } else {
            None
        };

        Self {
            config,
            scenario,
            grids,
            generation: 0,
            stats_history: Vec::new(),
            report_gen,
            morphology_config,
            archetypes,
            curriculum_tracker,
        }
    }

    /// Helper to get best elite across all archetypes
    pub fn best_elite(&self) -> Option<(&CreatureArchetype, &super::map_elites::Elite)> {
        self.grids
            .iter()
            .filter_map(|(arch, grid)| grid.best_elite().map(|e| (arch, e)))
            .max_by(|(_, a), (_, b)| {
                a.fitness
                    .partial_cmp(&b.fitness)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
    }

    /// Helper to get best elite for a specific archetype
    pub fn best_elite_for(
        &self,
        archetype: &CreatureArchetype,
    ) -> Option<&super::map_elites::Elite> {
        self.grids.get(archetype).and_then(|g| g.best_elite())
    }

    /// Check if a genome produces a viable morphology for a given archetype
    fn is_viable(&self, genome: &CreatureGenome, archetype: CreatureArchetype) -> bool {
        // For fixed archetypes, morphology is always viable (predetermined structure)
        if archetype != CreatureArchetype::Evolved {
            return true;
        }
        let morphology = CreatureMorphology::from_genome(genome, &self.morphology_config);
        let viability = analyze_viability(&morphology);
        viability.overall >= self.config.min_viability && viability.has_locomotion
    }

    /// Create the base genome for a given archetype
    fn base_genome_for(&self, archetype: CreatureArchetype) -> CreatureGenome {
        match archetype {
            CreatureArchetype::Evolved => CreatureGenome::test_biped(),
            CreatureArchetype::Spider => CreatureGenome::archetype_spider(),
            CreatureArchetype::Snake => CreatureGenome::archetype_snake(),
            CreatureArchetype::Worm => CreatureGenome::archetype_worm(),
            CreatureArchetype::Flyer => CreatureGenome::archetype_flyer(),
        }
    }

    /// Generate a viable genome for a given archetype (retries until viability threshold met)
    fn generate_viable_genome(&self, archetype: CreatureArchetype) -> CreatureGenome {
        const MAX_ATTEMPTS: usize = 100;

        for _ in 0..MAX_ATTEMPTS {
            let mut genome = self.base_genome_for(archetype);
            genome.mutate(
                &self.config.mutation_config,
                self.config.controller_mutation_rate,
            );

            if self.is_viable(&genome, archetype) {
                return genome;
            }
        }

        // Fallback: return base genome without mutation
        self.base_genome_for(archetype)
    }

    /// Create a progress bar style
    fn progress_style() -> ProgressStyle {
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta}) {msg}")
            .unwrap()
            .progress_chars("█▓░")
    }

    /// Check and advance curriculum stage if conditions are met
    ///
    /// Returns true if stage was advanced, along with optional message
    fn check_curriculum_advancement(
        &mut self,
        stats: &TrainingStats,
        pb: &ProgressBar,
    ) -> (bool, Option<String>) {
        // No curriculum = no advancement
        let Some(ref mut curriculum) = self.config.curriculum else {
            return (false, None);
        };

        let Some(ref mut tracker) = self.curriculum_tracker else {
            return (false, None);
        };

        // Check if we should advance
        let generations_in_stage = tracker.generations_in_stage(self.generation);
        let (should_advance, reason) = curriculum.should_advance(
            generations_in_stage,
            stats.best_fitness,
            stats.grid_coverage,
        );

        if should_advance {
            let old_stage = curriculum.current_stage().name.clone();

            // Advance curriculum
            if curriculum.advance() {
                tracker.on_stage_advance(self.generation);
                let new_stage = curriculum.current_stage().name.clone();

                let msg = format!(
                    "🎓 Curriculum advanced: {} → {} ({})",
                    old_stage,
                    new_stage,
                    reason.unwrap_or_else(|| "criteria met".to_string())
                );
                pb.println(&msg);

                // Update multi_env distribution if enabled
                if let Some(ref mut multi_env) = self.config.multi_env {
                    multi_env.distribution = curriculum.current_stage().distribution.clone();
                    pb.println(format!(
                        "   Updated environment distribution for {}",
                        new_stage
                    ));
                }

                return (true, Some(msg));
            }
        }

        (false, None)
    }

    /// Run the full training loop
    pub fn run(&mut self) -> Result<()> {
        // Calculate total evaluations: init population + generations * population
        let total_evals = (self.config.generations as u64 + 1) * self.config.population_size as u64;

        // Create main progress bar for entire training
        let pb = ProgressBar::new(total_evals);
        pb.set_style(Self::progress_style());
        pb.enable_steady_tick(std::time::Duration::from_millis(100));

        let archetype_names: Vec<_> = self.archetypes.iter().map(|a| a.name()).collect();
        pb.println(format!(
            "Starting training: {} generations, {} population, {} archetypes: {:?}",
            self.config.generations,
            self.config.population_size,
            self.archetypes.len(),
            archetype_names
        ));

        // Log curriculum information if enabled
        if let Some(ref curriculum) = self.config.curriculum {
            pb.println(format!(
                "📚 Curriculum learning enabled: {} stages",
                curriculum.num_stages()
            ));
            pb.println(format!(
                "   Starting with: {}",
                curriculum.current_stage().name
            ));

            // Initialize multi_env with curriculum's first stage distribution
            if let Some(ref mut multi_env) = self.config.multi_env {
                multi_env.distribution = curriculum.current_stage().distribution.clone();
                pb.println("   Multi-environment evaluation configured from curriculum");
            }
        }

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

            // Generate offspring population (archetype, genome pairs)
            let offspring = self.generate_offspring();

            // Evaluate offspring in parallel
            let results = self.evaluate_population_with_archetypes(&offspring, &pb)?;

            // Update grid and collect stats
            let stats = self.update_grid(results);
            self.stats_history.push(stats.clone());

            // Check curriculum advancement
            self.check_curriculum_advancement(&stats, &pb);

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
        let champion_displacement = if let Some((champion_archetype, best)) = self.best_elite() {
            let champion_archetype = *champion_archetype;
            // Set up world and evaluate champion with position tracking
            let (mut world, food_positions) = self.scenario.setup_world();
            let mut creature_manager = CreatureManager::new(1);
            let spawn_pos = self.scenario.config.spawn_position;

            let creature_id = creature_manager.spawn_creature_with_archetype_and_hunger(
                best.genome.clone(),
                spawn_pos,
                1.0, // Full hunger
                &self.morphology_config,
                champion_archetype,
            );

            // Log initial state
            pb.println(format!(
                "=== Champion Debug ({}) ===",
                champion_archetype.name()
            ));
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
                        &mut world,
                        &food_positions,
                    );
                }

                // Sample position every 5 seconds
                if step % (60 * 5) == 0
                    && let Some(creature) = creature_manager.get(creature_id) {
                        pos_samples.push(creature.position);
                        min_x = min_x.min(creature.position.x);
                        max_x = max_x.max(creature.position.x);
                    }
            }

            // Get final position
            let final_displacement = if let Some(creature) = creature_manager.get(creature_id) {
                let disp = (creature.position - spawn_pos).length();
                let debug_info = format!(
                    "=== Champion Debug ({}) ===\n\
                     Spawn position: ({:.1}, {:.1})\n\
                     Final position: ({:.1}, {:.1})\n\
                     Displacement: {:.1}px\n\
                     X range: {:.1} to {:.1} (width: {:.1})\n\
                     Position samples: {:?}",
                    champion_archetype.name(),
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

            // Evaluate champion (use idx=0 for deterministic multi-env sampling)
            let result = self.evaluate_single(best.genome.clone(), champion_archetype, 0);
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

        // Final report with GIFs - pass all grids
        self.report_gen
            .generate_final_report_multi(&self.grids, &self.stats_history, &gifs)?;

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
            "Initializing population with {} creatures ({}) across {} archetypes",
            self.config.population_size,
            mode,
            self.archetypes.len()
        ));

        // Distribute population evenly across archetypes
        let num_archetypes = self.archetypes.len();
        let per_archetype = self.config.population_size / num_archetypes;
        let remainder = self.config.population_size % num_archetypes;

        // Generate (archetype, genome) pairs
        let mut archetype_genomes: Vec<(CreatureArchetype, CreatureGenome)> = Vec::new();

        for (idx, &archetype) in self.archetypes.iter().enumerate() {
            // Give remainder creatures to first archetypes
            let count = per_archetype + if idx < remainder { 1 } else { 0 };

            for _ in 0..count {
                let genome = if self.config.use_simple_morphology {
                    self.generate_viable_genome(archetype)
                } else {
                    let mut g = self.base_genome_for(archetype);
                    g.mutate(
                        &self.config.mutation_config,
                        self.config.controller_mutation_rate,
                    );
                    g
                };
                archetype_genomes.push((archetype, genome));
            }
        }

        let results = self.evaluate_population_with_archetypes(&archetype_genomes, pb)?;

        for result in results {
            if let Some(grid) = self.grids.get_mut(&result.archetype) {
                grid.try_insert(
                    result.genome,
                    result.fitness,
                    &result.behavior,
                    0,
                    result.archetype,
                );
            }
        }

        // Calculate average coverage across all grids
        let total_coverage: f32 = self.grids.values().map(|g| g.coverage()).sum();
        let avg_coverage = total_coverage / self.grids.len() as f32;

        pb.println(format!(
            "Initial grid coverage: {:.1}% average across {} archetypes",
            avg_coverage * 100.0,
            self.grids.len()
        ));
        Ok(())
    }

    /// Generate offspring from current grids (balanced across archetypes)
    fn generate_offspring(&self) -> Vec<(CreatureArchetype, CreatureGenome)> {
        let mut offspring = Vec::with_capacity(self.config.population_size);

        // Distribute offspring evenly across archetypes
        let num_archetypes = self.archetypes.len();
        let per_archetype = self.config.population_size / num_archetypes;
        let remainder = self.config.population_size % num_archetypes;

        for (idx, &archetype) in self.archetypes.iter().enumerate() {
            let count = per_archetype + if idx < remainder { 1 } else { 0 };
            let grid = self.grids.get(&archetype);

            for _ in 0..count {
                // Try to generate a viable child (up to 10 attempts if using simple morphology)
                let max_attempts = if self.config.use_simple_morphology {
                    10
                } else {
                    1
                };

                let mut child = None;
                for _ in 0..max_attempts {
                    let candidate = if let Some(grid) = grid {
                        if let Some((parent1, parent2)) = grid.sample_parents() {
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
                        } else if let Some(parent) = grid.sample_elite() {
                            // Mutation only
                            let mut c = parent.genome.clone();
                            c.mutate(
                                &self.config.mutation_config,
                                self.config.controller_mutation_rate,
                            );
                            c
                        } else {
                            // Random (shouldn't happen after initialization)
                            let mut genome = self.base_genome_for(archetype);
                            genome.mutate(
                                &self.config.mutation_config,
                                self.config.controller_mutation_rate,
                            );
                            genome
                        }
                    } else {
                        // No grid for this archetype - random
                        let mut genome = self.base_genome_for(archetype);
                        genome.mutate(
                            &self.config.mutation_config,
                            self.config.controller_mutation_rate,
                        );
                        genome
                    };

                    // Check viability if using simple morphology
                    if !self.config.use_simple_morphology || self.is_viable(&candidate, archetype) {
                        child = Some(candidate);
                        break;
                    }
                }

                // Use fallback if no viable child found
                let genome = child.unwrap_or_else(|| self.generate_viable_genome(archetype));
                offspring.push((archetype, genome));
            }
        }

        offspring
    }

    /// Evaluate a population of genomes in parallel (with archetypes)
    fn evaluate_population_with_archetypes(
        &self,
        archetype_genomes: &[(CreatureArchetype, CreatureGenome)],
        pb: &ProgressBar,
    ) -> Result<Vec<EvalResult>> {
        let results: Vec<EvalResult> = archetype_genomes
            .par_iter()
            .enumerate()
            .map(|(idx, (archetype, genome))| {
                let result = self.evaluate_single(genome.clone(), *archetype, idx);
                pb.inc(1);
                result
            })
            .collect();

        Ok(results)
    }

    /// Evaluate a single creature (with optional multi-environment support)
    fn evaluate_single(
        &self,
        genome: CreatureGenome,
        archetype: CreatureArchetype,
        creature_idx: usize,
    ) -> EvalResult {
        // Check if multi-environment evaluation is enabled
        if let Some(ref multi_env) = self.config.multi_env {
            return self.evaluate_single_multi_env(genome, archetype, creature_idx, multi_env);
        }

        // Legacy single-environment evaluation
        self.evaluate_single_legacy(genome, archetype)
    }

    /// Legacy single-environment evaluation (backward compatible)
    fn evaluate_single_legacy(
        &self,
        genome: CreatureGenome,
        archetype: CreatureArchetype,
    ) -> EvalResult {
        // Set up world with cached food positions
        let (mut world, food_positions) = self.scenario.setup_world();
        let mut creature_manager = CreatureManager::new(1);

        // Spawn creature using the configured morphology and archetype
        let spawn_pos = self.scenario.config.spawn_position;
        let initial_hunger = if self.scenario.config.name == "Parcour" {
            0.5 // Start with 50% hunger for parcour - creates survival pressure
        } else {
            1.0 // Full hunger for other scenarios
        };
        let creature_id = creature_manager.spawn_creature_with_archetype_and_hunger(
            genome.clone(),
            spawn_pos,
            initial_hunger,
            &self.morphology_config,
            archetype,
        );

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
                    &mut world,
                    &food_positions,
                );
            }
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
            archetype,
            genome,
            fitness,
            behavior,
            displacement,
        }
    }

    /// Multi-environment evaluation (NEW)
    fn evaluate_single_multi_env(
        &self,
        genome: CreatureGenome,
        archetype: CreatureArchetype,
        creature_idx: usize,
        multi_env: &MultiEnvironmentEvaluator,
    ) -> EvalResult {
        // Compute deterministic eval_id for this creature
        let eval_id =
            (self.generation as u64) * (self.config.population_size as u64) + (creature_idx as u64);

        // Sample terrain configurations for this evaluation
        let terrains = multi_env
            .sample_terrains(eval_id)
            .expect("Failed to sample terrains");

        // Evaluate on each environment
        let mut individual_scores = Vec::new();
        let mut behavior_sum = BehaviorDescriptor {
            locomotion_efficiency: 0.0,
            foraging_efficiency: 0.0,
            exploration: 0.0,
            activity: 0.0,
        };
        let mut displacement_sum = 0.0;

        for terrain in &terrains {
            let result = self.evaluate_on_terrain(&genome, archetype, terrain);
            individual_scores.push(result.fitness);
            behavior_sum.locomotion_efficiency += result.behavior.locomotion_efficiency;
            behavior_sum.foraging_efficiency += result.behavior.foraging_efficiency;
            behavior_sum.exploration += result.behavior.exploration;
            behavior_sum.activity += result.behavior.activity;
            displacement_sum += result.displacement;
        }

        // Aggregate fitness across environments
        let aggregated_fitness = multi_env.aggregate_fitness(&individual_scores);

        // Average behavior across environments
        let n = terrains.len() as f32;
        let avg_behavior = BehaviorDescriptor {
            locomotion_efficiency: behavior_sum.locomotion_efficiency / n,
            foraging_efficiency: behavior_sum.foraging_efficiency / n,
            exploration: behavior_sum.exploration / n,
            activity: behavior_sum.activity / n,
        };
        let avg_displacement = displacement_sum / n;

        EvalResult {
            archetype,
            genome,
            fitness: aggregated_fitness,
            behavior: avg_behavior,
            displacement: avg_displacement,
        }
    }

    /// Evaluate creature on a specific terrain (helper for multi-env)
    fn evaluate_on_terrain(
        &self,
        genome: &CreatureGenome,
        archetype: CreatureArchetype,
        terrain: &super::terrain_config::TrainingTerrainConfig,
    ) -> EvalResult {
        // Set up world with custom terrain
        let (mut world, food_positions) = self.scenario.setup_world_with_terrain(terrain);
        let mut creature_manager = CreatureManager::new(1);

        // Spawn creature
        let spawn_pos = self.scenario.config.spawn_position;
        let initial_hunger = if self.scenario.config.name == "Parcour" {
            0.5
        } else {
            1.0
        };
        let creature_id = creature_manager.spawn_creature_with_archetype_and_hunger(
            genome.clone(),
            spawn_pos,
            initial_hunger,
            &self.morphology_config,
            archetype,
        );

        // Run simulation
        let dt = 1.0 / 60.0;
        let steps = (self.config.eval_duration / dt) as usize;
        const SENSORY_SKIP: usize = 6;

        for step in 0..steps {
            if step % SENSORY_SKIP == 0 {
                creature_manager.update_with_cache(
                    dt * SENSORY_SKIP as f32,
                    &mut world,
                    &food_positions,
                );
            }
        }

        // Evaluate creature
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
            archetype,
            genome: genome.clone(),
            fitness,
            behavior,
            displacement,
        }
    }

    /// Update grids with evaluation results
    fn update_grid(&mut self, results: Vec<EvalResult>) -> TrainingStats {
        let mut new_elites = 0;
        let mut total_fitness = 0.0;
        let mut total_displacement = 0.0;
        let mut max_displacement = 0.0f32;

        for result in &results {
            if let Some(grid) = self.grids.get_mut(&result.archetype)
                && grid.try_insert(
                    result.genome.clone(),
                    result.fitness,
                    &result.behavior,
                    self.generation,
                    result.archetype,
                ) {
                    new_elites += 1;
                }
            total_fitness += result.fitness;
            total_displacement += result.displacement;
            max_displacement = max_displacement.max(result.displacement);
        }

        // Compute aggregate stats across all grids
        let mut best_fitness = f32::NEG_INFINITY;
        let mut total_coverage = 0.0;
        let mut best_per_archetype = HashMap::new();

        for (archetype, grid) in &self.grids {
            let stats = grid.stats();
            best_fitness = best_fitness.max(stats.best_fitness);
            total_coverage += stats.coverage;
            best_per_archetype.insert(*archetype, stats.best_fitness);
        }

        let avg_coverage = if self.grids.is_empty() {
            0.0
        } else {
            total_coverage / self.grids.len() as f32
        };

        let n = results.len() as f32;

        TrainingStats {
            generation: self.generation,
            best_fitness,
            avg_fitness: if results.is_empty() {
                0.0
            } else {
                total_fitness / n
            },
            grid_coverage: avg_coverage,
            new_elites,
            max_displacement,
            avg_displacement: if results.is_empty() {
                0.0
            } else {
                total_displacement / n
            },
            best_per_archetype,
        }
    }

    /// Save a checkpoint
    fn save_checkpoint(&self, pb: &ProgressBar) -> Result<()> {
        let checkpoint_dir = format!("{}/checkpoints", self.config.output_dir);
        std::fs::create_dir_all(&checkpoint_dir)
            .context("Failed to create checkpoint directory")?;

        // Save best genome (overall champion)
        if let Some((archetype, best)) = self.best_elite() {
            let path = format!(
                "{}/gen_{:04}_best_{}.genome",
                checkpoint_dir,
                self.generation,
                archetype.name().to_lowercase()
            );
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
        archetype: CreatureArchetype,
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
        let (mut world, food_positions) = self.scenario.setup_world();
        let mut creature_manager = CreatureManager::new(1);
        let spawn_pos = self.scenario.config.spawn_position;
        let initial_hunger = if self.scenario.config.name == "Parcour" {
            0.5
        } else {
            1.0
        };
        let creature_id = creature_manager.spawn_creature_with_archetype_and_hunger(
            genome.clone(),
            spawn_pos,
            initial_hunger,
            &self.morphology_config,
            archetype,
        );

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
                    &mut world,
                    &food_positions,
                );
            }

            // Capture frame at intervals
            if step % capture_interval == 0
                && let Some(creature) = creature_manager.get(creature_id) {
                    let render_data = creature.get_render_data();
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

        // Encode GIF to bytes
        let data = gif.to_bytes().context("Failed to encode GIF")?;

        Ok(CapturedGif {
            label: label.to_string(),
            fitness,
            behavior: behavior.to_vec(),
            data,
        })
    }

    /// Capture GIFs for the best creature from each archetype
    fn capture_all_gifs(&self, pb: &ProgressBar) -> Vec<CapturedGif> {
        let mut gifs = Vec::new();

        pb.println(format!(
            "Capturing GIFs of best creatures from {} archetypes...",
            self.archetypes.len()
        ));

        // Capture best elite from each archetype
        for &archetype in &self.archetypes {
            if let Some(grid) = self.grids.get(&archetype)
                && let Some(best) = grid.best_elite() {
                    let label = format!("Best {}", archetype.name());
                    pb.println(format!(
                        "  Capturing: {} (fitness: {:.2})",
                        label, best.fitness
                    ));
                    match self.capture_elite_gif(
                        &best.genome,
                        archetype,
                        &label,
                        best.fitness,
                        &best.behavior,
                    ) {
                        Ok(gif) => gifs.push(gif),
                        Err(e) => log::warn!("Failed to capture {} GIF: {}", label, e),
                    }
                }
        }

        // Also capture overall champion if there are multiple archetypes
        if self.archetypes.len() > 1
            && let Some((champion_archetype, best)) = self.best_elite() {
                // Only add if not already captured (i.e., if it's different from individual archetype bests)
                let overall_label = format!("Champion ({})", champion_archetype.name());
                let already_have_champion = gifs.iter().any(|g| {
                    g.label == format!("Best {}", champion_archetype.name())
                        && (g.fitness - best.fitness).abs() < 0.01
                });

                if !already_have_champion {
                    pb.println(format!(
                        "  Capturing: {} (fitness: {:.2})",
                        overall_label, best.fitness
                    ));
                    match self.capture_elite_gif(
                        &best.genome,
                        *champion_archetype,
                        &overall_label,
                        best.fitness,
                        &best.behavior,
                    ) {
                        Ok(gif) => gifs.push(gif),
                        Err(e) => log::warn!("Failed to capture overall champion GIF: {}", e),
                    }
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
