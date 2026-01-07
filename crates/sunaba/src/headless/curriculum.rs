//! Curriculum learning system for progressive training difficulty
//!
//! Enables creatures to learn incrementally by progressing through stages of
//! increasing environmental difficulty, improving sample efficiency and final
//! performance.

use anyhow::Result;
use serde::{Deserialize, Serialize};

use super::env_distribution::EnvironmentDistribution;
use super::terrain_config::DifficultyConfig;

/// Curriculum learning configuration with progressive difficulty stages
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurriculumConfig {
    /// Ordered list of training stages (easier → harder)
    pub stages: Vec<CurriculumStage>,

    /// Current stage index (starts at 0)
    #[serde(skip)]
    current_stage: usize,
}

/// Single stage in the curriculum
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurriculumStage {
    /// Human-readable stage name
    pub name: String,

    /// Environment distribution for this stage
    pub distribution: EnvironmentDistribution,

    /// Minimum generations before advancing to next stage
    pub min_generations: usize,

    /// Criteria for advancing to next stage (checked after min_generations)
    pub advancement: AdvancementCriteria,
}

/// Criteria for advancing to the next curriculum stage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AdvancementCriteria {
    /// Advance automatically after min_generations
    Automatic,

    /// Advance when best fitness exceeds threshold
    FitnessThreshold { target: f32 },

    /// Advance when MAP-Elites coverage exceeds threshold (0.0-1.0)
    CoverageThreshold { target: f32 },

    /// Advance when both fitness and coverage thresholds are met
    Combined {
        fitness_target: f32,
        coverage_target: f32,
    },
}

impl CurriculumConfig {
    /// Create a new curriculum with given stages
    pub fn new(stages: Vec<CurriculumStage>) -> Result<Self> {
        if stages.is_empty() {
            anyhow::bail!("Curriculum must have at least one stage");
        }

        Ok(Self {
            stages,
            current_stage: 0,
        })
    }

    /// Get the current curriculum stage
    pub fn current_stage(&self) -> &CurriculumStage {
        &self.stages[self.current_stage]
    }

    /// Get the current stage index
    pub fn current_stage_index(&self) -> usize {
        self.current_stage
    }

    /// Get total number of stages
    pub fn num_stages(&self) -> usize {
        self.stages.len()
    }

    /// Check if curriculum is complete (on final stage)
    pub fn is_complete(&self) -> bool {
        self.current_stage >= self.stages.len() - 1
    }

    /// Check if we should advance to the next stage
    ///
    /// Returns (should_advance, reason_message)
    pub fn should_advance(
        &self,
        generations_in_stage: usize,
        best_fitness: f32,
        avg_coverage: f32,
    ) -> (bool, Option<String>) {
        // Already on final stage
        if self.is_complete() {
            return (false, None);
        }

        let stage = self.current_stage();

        // Must complete minimum generations first
        if generations_in_stage < stage.min_generations {
            return (false, None);
        }

        // Check advancement criteria
        match &stage.advancement {
            AdvancementCriteria::Automatic => {
                (true, Some("minimum generations completed".to_string()))
            }
            AdvancementCriteria::FitnessThreshold { target } => {
                if best_fitness >= *target {
                    (
                        true,
                        Some(format!("fitness {:.2} >= target {:.2}", best_fitness, target)),
                    )
                } else {
                    (false, None)
                }
            }
            AdvancementCriteria::CoverageThreshold { target } => {
                if avg_coverage >= *target {
                    (
                        true,
                        Some(format!(
                            "coverage {:.1}% >= target {:.1}%",
                            avg_coverage * 100.0,
                            target * 100.0
                        )),
                    )
                } else {
                    (false, None)
                }
            }
            AdvancementCriteria::Combined {
                fitness_target,
                coverage_target,
            } => {
                if best_fitness >= *fitness_target && avg_coverage >= *coverage_target {
                    (
                        true,
                        Some(format!(
                            "fitness {:.2} >= {:.2} and coverage {:.1}% >= {:.1}%",
                            best_fitness,
                            fitness_target,
                            avg_coverage * 100.0,
                            coverage_target * 100.0
                        )),
                    )
                } else {
                    (false, None)
                }
            }
        }
    }

    /// Advance to the next stage (returns true if advanced, false if already on final stage)
    pub fn advance(&mut self) -> bool {
        if !self.is_complete() {
            self.current_stage += 1;
            true
        } else {
            false
        }
    }

    /// Reset curriculum to first stage (useful for restarting training)
    pub fn reset(&mut self) {
        self.current_stage = 0;
    }

    /// Standard 5-stage curriculum: Flat → Hills → Obstacles → Hazards → Random
    pub fn standard() -> Self {
        let stages = vec![
            CurriculumStage {
                name: "Stage 1: Flat Ground".to_string(),
                distribution: EnvironmentDistribution::discrete(vec![DifficultyConfig::flat()]),
                min_generations: 20,
                advancement: AdvancementCriteria::Automatic,
            },
            CurriculumStage {
                name: "Stage 2: Gentle Hills".to_string(),
                distribution: EnvironmentDistribution::discrete(vec![
                    DifficultyConfig::gentle_hills(),
                ]),
                min_generations: 30,
                advancement: AdvancementCriteria::FitnessThreshold { target: 5.0 },
            },
            CurriculumStage {
                name: "Stage 3: Obstacles".to_string(),
                distribution: EnvironmentDistribution::discrete(vec![
                    DifficultyConfig::gentle_hills(),
                    DifficultyConfig::obstacles(),
                ]),
                min_generations: 40,
                advancement: AdvancementCriteria::Combined {
                    fitness_target: 10.0,
                    coverage_target: 0.1,
                },
            },
            CurriculumStage {
                name: "Stage 4: Hazards".to_string(),
                distribution: EnvironmentDistribution::discrete(vec![
                    DifficultyConfig::obstacles(),
                    DifficultyConfig::hazards(),
                ]),
                min_generations: 50,
                advancement: AdvancementCriteria::Combined {
                    fitness_target: 15.0,
                    coverage_target: 0.15,
                },
            },
            CurriculumStage {
                name: "Stage 5: Full Random".to_string(),
                distribution: EnvironmentDistribution::uniform(
                    DifficultyConfig::flat(),
                    DifficultyConfig::random(),
                ),
                min_generations: 0, // Final stage - no advancement
                advancement: AdvancementCriteria::Automatic,
            },
        ];

        Self {
            stages,
            current_stage: 0,
        }
    }

    /// Fast curriculum for quick testing (3 stages, fewer generations)
    pub fn fast() -> Self {
        let stages = vec![
            CurriculumStage {
                name: "Stage 1: Flat Ground".to_string(),
                distribution: EnvironmentDistribution::discrete(vec![DifficultyConfig::flat()]),
                min_generations: 5,
                advancement: AdvancementCriteria::Automatic,
            },
            CurriculumStage {
                name: "Stage 2: Gentle Hills".to_string(),
                distribution: EnvironmentDistribution::discrete(vec![
                    DifficultyConfig::gentle_hills(),
                ]),
                min_generations: 10,
                advancement: AdvancementCriteria::Automatic,
            },
            CurriculumStage {
                name: "Stage 3: Full Random".to_string(),
                distribution: EnvironmentDistribution::uniform(
                    DifficultyConfig::flat(),
                    DifficultyConfig::random(),
                ),
                min_generations: 0,
                advancement: AdvancementCriteria::Automatic,
            },
        ];

        Self {
            stages,
            current_stage: 0,
        }
    }

    /// Aggressive curriculum focused on quick mastery (fitness-gated progression)
    pub fn aggressive() -> Self {
        let stages = vec![
            CurriculumStage {
                name: "Stage 1: Flat Ground".to_string(),
                distribution: EnvironmentDistribution::discrete(vec![DifficultyConfig::flat()]),
                min_generations: 10,
                advancement: AdvancementCriteria::FitnessThreshold { target: 8.0 },
            },
            CurriculumStage {
                name: "Stage 2: Mixed Easy".to_string(),
                distribution: EnvironmentDistribution::discrete(vec![
                    DifficultyConfig::flat(),
                    DifficultyConfig::gentle_hills(),
                ]),
                min_generations: 15,
                advancement: AdvancementCriteria::FitnessThreshold { target: 12.0 },
            },
            CurriculumStage {
                name: "Stage 3: Full Random".to_string(),
                distribution: EnvironmentDistribution::uniform(
                    DifficultyConfig::flat(),
                    DifficultyConfig::random(),
                ),
                min_generations: 0,
                advancement: AdvancementCriteria::Automatic,
            },
        ];

        Self {
            stages,
            current_stage: 0,
        }
    }
}

/// Tracks curriculum progress during training
#[derive(Debug, Clone)]
pub struct CurriculumTracker {
    /// Generation when current stage started
    stage_start_generation: usize,
}

impl CurriculumTracker {
    pub fn new() -> Self {
        Self {
            stage_start_generation: 0,
        }
    }

    /// Get number of generations spent in current stage
    pub fn generations_in_stage(&self, current_generation: usize) -> usize {
        current_generation.saturating_sub(self.stage_start_generation)
    }

    /// Mark that we've advanced to a new stage
    pub fn on_stage_advance(&mut self, current_generation: usize) {
        self.stage_start_generation = current_generation;
    }

    /// Reset tracker (useful when curriculum is reset)
    pub fn reset(&mut self) {
        self.stage_start_generation = 0;
    }
}

impl Default for CurriculumTracker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_curriculum_creation() {
        let curriculum = CurriculumConfig::standard();
        assert_eq!(curriculum.num_stages(), 5);
        assert_eq!(curriculum.current_stage_index(), 0);
        assert!(!curriculum.is_complete());
    }

    #[test]
    fn test_curriculum_advancement_automatic() {
        let mut curriculum = CurriculumConfig::new(vec![
            CurriculumStage {
                name: "Stage 1".to_string(),
                distribution: EnvironmentDistribution::default(),
                min_generations: 10,
                advancement: AdvancementCriteria::Automatic,
            },
            CurriculumStage {
                name: "Stage 2".to_string(),
                distribution: EnvironmentDistribution::default(),
                min_generations: 0,
                advancement: AdvancementCriteria::Automatic,
            },
        ])
        .unwrap();

        // Should not advance before min_generations
        let (should_advance, _) = curriculum.should_advance(5, 100.0, 1.0);
        assert!(!should_advance);

        // Should advance after min_generations (automatic)
        let (should_advance, reason) = curriculum.should_advance(10, 100.0, 1.0);
        assert!(should_advance);
        assert!(reason.is_some());

        // Actually advance
        assert!(curriculum.advance());
        assert_eq!(curriculum.current_stage_index(), 1);
        assert!(curriculum.is_complete());

        // Cannot advance past final stage
        assert!(!curriculum.advance());
        assert_eq!(curriculum.current_stage_index(), 1);
    }

    #[test]
    fn test_curriculum_advancement_fitness_threshold() {
        let mut curriculum = CurriculumConfig::new(vec![
            CurriculumStage {
                name: "Stage 1".to_string(),
                distribution: EnvironmentDistribution::default(),
                min_generations: 10,
                advancement: AdvancementCriteria::FitnessThreshold { target: 15.0 },
            },
            CurriculumStage {
                name: "Stage 2".to_string(),
                distribution: EnvironmentDistribution::default(),
                min_generations: 0,
                advancement: AdvancementCriteria::Automatic,
            },
        ])
        .unwrap();

        // Should not advance before min_generations
        let (should_advance, _) = curriculum.should_advance(5, 20.0, 0.5);
        assert!(!should_advance);

        // Should not advance if fitness too low
        let (should_advance, _) = curriculum.should_advance(15, 10.0, 0.5);
        assert!(!should_advance);

        // Should advance when both min_generations and fitness met
        let (should_advance, reason) = curriculum.should_advance(15, 20.0, 0.5);
        assert!(should_advance);
        assert!(reason.unwrap().contains("fitness"));
    }

    #[test]
    fn test_curriculum_advancement_coverage_threshold() {
        let curriculum = CurriculumConfig::new(vec![CurriculumStage {
            name: "Stage 1".to_string(),
            distribution: EnvironmentDistribution::default(),
            min_generations: 5,
            advancement: AdvancementCriteria::CoverageThreshold { target: 0.2 },
        }])
        .unwrap();

        // Should not advance if coverage too low
        let (should_advance, _) = curriculum.should_advance(10, 100.0, 0.1);
        assert!(!should_advance);

        // Should advance when coverage threshold met
        let (should_advance, reason) = curriculum.should_advance(10, 100.0, 0.25);
        assert!(should_advance);
        assert!(reason.unwrap().contains("coverage"));
    }

    #[test]
    fn test_curriculum_advancement_combined() {
        let curriculum = CurriculumConfig::new(vec![CurriculumStage {
            name: "Stage 1".to_string(),
            distribution: EnvironmentDistribution::default(),
            min_generations: 5,
            advancement: AdvancementCriteria::Combined {
                fitness_target: 10.0,
                coverage_target: 0.15,
            },
        }])
        .unwrap();

        // Should not advance if only fitness met
        let (should_advance, _) = curriculum.should_advance(10, 15.0, 0.1);
        assert!(!should_advance);

        // Should not advance if only coverage met
        let (should_advance, _) = curriculum.should_advance(10, 5.0, 0.2);
        assert!(!should_advance);

        // Should advance when both met
        let (should_advance, reason) = curriculum.should_advance(10, 15.0, 0.2);
        assert!(should_advance);
        let msg = reason.unwrap();
        assert!(msg.contains("fitness"));
        assert!(msg.contains("coverage"));
    }

    #[test]
    fn test_curriculum_reset() {
        let mut curriculum = CurriculumConfig::standard();

        // Advance to stage 2
        curriculum.advance();
        assert_eq!(curriculum.current_stage_index(), 1);

        // Reset should go back to stage 0
        curriculum.reset();
        assert_eq!(curriculum.current_stage_index(), 0);
    }

    #[test]
    fn test_curriculum_tracker() {
        let mut tracker = CurriculumTracker::new();

        // Initial state
        assert_eq!(tracker.generations_in_stage(0), 0);
        assert_eq!(tracker.generations_in_stage(10), 10);

        // Advance to new stage at generation 15
        tracker.on_stage_advance(15);
        assert_eq!(tracker.generations_in_stage(15), 0);
        assert_eq!(tracker.generations_in_stage(20), 5);
        assert_eq!(tracker.generations_in_stage(30), 15);

        // Reset
        tracker.reset();
        assert_eq!(tracker.generations_in_stage(30), 30);
    }

    #[test]
    fn test_standard_curriculum_structure() {
        let curriculum = CurriculumConfig::standard();

        assert_eq!(curriculum.num_stages(), 5);
        assert_eq!(curriculum.stages[0].name, "Stage 1: Flat Ground");
        assert_eq!(curriculum.stages[4].name, "Stage 5: Full Random");

        // Check min_generations progression
        assert_eq!(curriculum.stages[0].min_generations, 20);
        assert_eq!(curriculum.stages[1].min_generations, 30);
        assert_eq!(curriculum.stages[2].min_generations, 40);
        assert_eq!(curriculum.stages[3].min_generations, 50);
    }

    #[test]
    fn test_fast_curriculum() {
        let curriculum = CurriculumConfig::fast();

        assert_eq!(curriculum.num_stages(), 3);
        assert_eq!(curriculum.stages[0].min_generations, 5);
        assert_eq!(curriculum.stages[1].min_generations, 10);

        // All stages should use automatic advancement
        for stage in &curriculum.stages {
            assert!(matches!(
                stage.advancement,
                AdvancementCriteria::Automatic
            ));
        }
    }

    #[test]
    fn test_aggressive_curriculum() {
        let curriculum = CurriculumConfig::aggressive();

        assert_eq!(curriculum.num_stages(), 3);

        // Should use fitness thresholds for early stages
        assert!(matches!(
            curriculum.stages[0].advancement,
            AdvancementCriteria::FitnessThreshold { .. }
        ));
        assert!(matches!(
            curriculum.stages[1].advancement,
            AdvancementCriteria::FitnessThreshold { .. }
        ));
    }

    #[test]
    fn test_empty_curriculum_rejected() {
        let result = CurriculumConfig::new(vec![]);
        assert!(result.is_err());
    }

    #[test]
    fn test_current_stage() {
        let mut curriculum = CurriculumConfig::standard();

        let stage1 = curriculum.current_stage();
        assert_eq!(stage1.name, "Stage 1: Flat Ground");

        curriculum.advance();
        let stage2 = curriculum.current_stage();
        assert_eq!(stage2.name, "Stage 2: Gentle Hills");
    }
}
