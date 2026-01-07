//! Multi-environment evaluation system for creature training
//!
//! Evaluates creatures across multiple environments to promote generalization
//! and prevent overfitting to a single terrain configuration.

use anyhow::Result;
use serde::{Deserialize, Serialize};

use super::env_distribution::EnvironmentDistribution;
use super::terrain_config::TrainingTerrainConfig;

/// Evaluates creatures across multiple environments
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultiEnvironmentEvaluator {
    /// Number of environments to evaluate each creature on
    pub num_environments: usize,

    /// How environment configs are sampled
    pub distribution: EnvironmentDistribution,

    /// How to aggregate fitness across environments
    pub aggregation: FitnessAggregation,
}

/// Strategy for aggregating fitness across multiple environments
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum FitnessAggregation {
    /// Average performance (encourages general competence)
    Mean,

    /// Worst-case performance (encourages robustness)
    Min,

    /// Median performance (robust to outliers)
    Median,

    /// Percentile performance (e.g., 25th = bottom quartile)
    Percentile(f32),

    /// Harmonic mean (punishes poor performance more than arithmetic mean)
    HarmonicMean,
}

/// Result of multi-environment evaluation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultiEnvFitness {
    /// Individual fitness scores per environment
    pub individual_scores: Vec<f32>,

    /// Aggregated fitness (what's used for selection)
    pub aggregated: f32,

    /// Statistics for reporting
    pub stats: FitnessStats,
}

/// Fitness statistics across multiple environments
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FitnessStats {
    pub mean: f32,
    pub median: f32,
    pub min: f32,
    pub max: f32,
    pub std_dev: f32,
}

impl MultiEnvironmentEvaluator {
    /// Sample terrain configurations for this evaluation
    ///
    /// eval_id should be unique per evaluation (e.g., generation * pop_size + creature_idx)
    /// to ensure different creatures see different environment samples
    pub fn sample_terrains(&self, eval_id: u64) -> Result<Vec<TrainingTerrainConfig>> {
        self.distribution
            .sample_batch(eval_id, self.num_environments)
    }

    /// Aggregate fitness scores from multiple environments
    pub fn aggregate_fitness(&self, scores: &[f32]) -> f32 {
        if scores.is_empty() {
            return 0.0;
        }

        match self.aggregation {
            FitnessAggregation::Mean => scores.iter().sum::<f32>() / scores.len() as f32,
            FitnessAggregation::Min => scores.iter().copied().fold(f32::INFINITY, f32::min),
            FitnessAggregation::Median => {
                let mut sorted = scores.to_vec();
                sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
                let mid = sorted.len() / 2;
                if sorted.len().is_multiple_of(2) {
                    (sorted[mid - 1] + sorted[mid]) / 2.0
                } else {
                    sorted[mid]
                }
            }
            FitnessAggregation::Percentile(p) => {
                let mut sorted = scores.to_vec();
                sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
                let idx = ((p / 100.0) * (sorted.len() - 1) as f32) as usize;
                sorted[idx.min(sorted.len() - 1)]
            }
            FitnessAggregation::HarmonicMean => {
                let n = scores.len() as f32;
                let sum_reciprocals: f32 = scores.iter().map(|&x| 1.0 / x.max(1e-6)).sum();
                n / sum_reciprocals
            }
        }
    }

    /// Compute fitness statistics from multiple environment scores
    pub fn compute_stats(&self, scores: &[f32]) -> FitnessStats {
        if scores.is_empty() {
            return FitnessStats {
                mean: 0.0,
                median: 0.0,
                min: 0.0,
                max: 0.0,
                std_dev: 0.0,
            };
        }

        let mean = scores.iter().sum::<f32>() / scores.len() as f32;
        let variance =
            scores.iter().map(|&x| (x - mean).powi(2)).sum::<f32>() / scores.len() as f32;
        let std_dev = variance.sqrt();

        let min = scores.iter().copied().fold(f32::INFINITY, f32::min);
        let max = scores.iter().copied().fold(f32::NEG_INFINITY, f32::max);

        let mut sorted = scores.to_vec();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let median = if sorted.len().is_multiple_of(2) {
            (sorted[sorted.len() / 2 - 1] + sorted[sorted.len() / 2]) / 2.0
        } else {
            sorted[sorted.len() / 2]
        };

        FitnessStats {
            mean,
            median,
            min,
            max,
            std_dev,
        }
    }

    /// Create multi-environment fitness result from individual scores
    pub fn create_result(&self, individual_scores: Vec<f32>) -> MultiEnvFitness {
        let aggregated = self.aggregate_fitness(&individual_scores);
        let stats = self.compute_stats(&individual_scores);

        MultiEnvFitness {
            individual_scores,
            aggregated,
            stats,
        }
    }
}

impl Default for MultiEnvironmentEvaluator {
    fn default() -> Self {
        Self {
            num_environments: 1, // Backward compatible - single environment
            distribution: EnvironmentDistribution::default(),
            aggregation: FitnessAggregation::Mean,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fitness_aggregation_mean() {
        let evaluator = MultiEnvironmentEvaluator {
            num_environments: 5,
            distribution: EnvironmentDistribution::default(),
            aggregation: FitnessAggregation::Mean,
        };

        let scores = vec![10.0, 20.0, 30.0, 40.0, 50.0];
        assert_eq!(evaluator.aggregate_fitness(&scores), 30.0);
    }

    #[test]
    fn test_fitness_aggregation_min() {
        let evaluator = MultiEnvironmentEvaluator {
            num_environments: 5,
            distribution: EnvironmentDistribution::default(),
            aggregation: FitnessAggregation::Min,
        };

        let scores = vec![10.0, 20.0, 5.0, 40.0, 50.0];
        assert_eq!(evaluator.aggregate_fitness(&scores), 5.0);
    }

    #[test]
    fn test_fitness_aggregation_median() {
        let evaluator = MultiEnvironmentEvaluator {
            num_environments: 5,
            distribution: EnvironmentDistribution::default(),
            aggregation: FitnessAggregation::Median,
        };

        let scores = vec![10.0, 20.0, 30.0, 40.0, 50.0];
        assert_eq!(evaluator.aggregate_fitness(&scores), 30.0);

        // Even number of scores
        let scores_even = vec![10.0, 20.0, 30.0, 40.0];
        assert_eq!(evaluator.aggregate_fitness(&scores_even), 25.0);
    }

    #[test]
    fn test_fitness_aggregation_percentile() {
        let evaluator = MultiEnvironmentEvaluator {
            num_environments: 5,
            distribution: EnvironmentDistribution::default(),
            aggregation: FitnessAggregation::Percentile(25.0),
        };

        let scores = vec![10.0, 20.0, 30.0, 40.0, 50.0];
        // 25th percentile should be around 20.0
        let result = evaluator.aggregate_fitness(&scores);
        assert!((result - 20.0).abs() < 1.0);
    }

    #[test]
    fn test_fitness_aggregation_harmonic_mean() {
        let evaluator = MultiEnvironmentEvaluator {
            num_environments: 3,
            distribution: EnvironmentDistribution::default(),
            aggregation: FitnessAggregation::HarmonicMean,
        };

        let scores = vec![10.0, 20.0, 30.0];
        // Harmonic mean = 3 / (1/10 + 1/20 + 1/30) ≈ 16.36
        let result = evaluator.aggregate_fitness(&scores);
        assert!((result - 16.36).abs() < 0.1);
    }

    #[test]
    fn test_compute_stats() {
        let evaluator = MultiEnvironmentEvaluator::default();

        let scores = vec![10.0, 20.0, 30.0, 40.0, 50.0];
        let stats = evaluator.compute_stats(&scores);

        assert_eq!(stats.mean, 30.0);
        assert_eq!(stats.median, 30.0);
        assert_eq!(stats.min, 10.0);
        assert_eq!(stats.max, 50.0);
        // Std dev of [10, 20, 30, 40, 50] with mean 30
        // variance = ((20^2 + 10^2 + 0 + 10^2 + 20^2) / 5) = 200
        // std_dev = sqrt(200) ≈ 14.14
        assert!((stats.std_dev - 14.14).abs() < 0.1);
    }

    #[test]
    fn test_compute_stats_empty() {
        let evaluator = MultiEnvironmentEvaluator::default();

        let scores: Vec<f32> = vec![];
        let stats = evaluator.compute_stats(&scores);

        assert_eq!(stats.mean, 0.0);
        assert_eq!(stats.median, 0.0);
        assert_eq!(stats.min, 0.0);
        assert_eq!(stats.max, 0.0);
        assert_eq!(stats.std_dev, 0.0);
    }

    #[test]
    fn test_create_result() {
        let evaluator = MultiEnvironmentEvaluator {
            num_environments: 3,
            distribution: EnvironmentDistribution::default(),
            aggregation: FitnessAggregation::Mean,
        };

        let scores = vec![10.0, 20.0, 30.0];
        let result = evaluator.create_result(scores.clone());

        assert_eq!(result.individual_scores, scores);
        assert_eq!(result.aggregated, 20.0); // mean
        assert_eq!(result.stats.mean, 20.0);
        assert_eq!(result.stats.median, 20.0);
    }

    #[test]
    fn test_sample_terrains_deterministic() {
        let evaluator = MultiEnvironmentEvaluator {
            num_environments: 3,
            distribution: EnvironmentDistribution::default(),
            aggregation: FitnessAggregation::Mean,
        };

        // Same eval_id should produce same terrains
        let terrains1 = evaluator.sample_terrains(12345).unwrap();
        let terrains2 = evaluator.sample_terrains(12345).unwrap();

        assert_eq!(terrains1.len(), 3);
        assert_eq!(terrains2.len(), 3);

        for (t1, t2) in terrains1.iter().zip(terrains2.iter()) {
            assert_eq!(t1.base_seed, t2.base_seed);
        }
    }

    #[test]
    fn test_sample_terrains_different_creatures() {
        let evaluator = MultiEnvironmentEvaluator {
            num_environments: 3,
            distribution: EnvironmentDistribution::default(),
            aggregation: FitnessAggregation::Mean,
        };

        // Different eval_id should produce different terrains
        let terrains1 = evaluator.sample_terrains(100).unwrap();
        let terrains2 = evaluator.sample_terrains(200).unwrap();

        // At least one terrain should be different
        let any_different = terrains1
            .iter()
            .zip(terrains2.iter())
            .any(|(t1, t2)| t1.base_seed != t2.base_seed);
        assert!(any_different);
    }
}
