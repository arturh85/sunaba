//! Environment distribution system for multi-environment training evaluation
//!
//! Provides deterministic, reproducible sampling of training terrain configurations
//! to evaluate creatures across diverse environments.

use anyhow::Result;
use rand::{Rng, SeedableRng};
use rand_xoshiro::Xoshiro256PlusPlus;
use serde::{Deserialize, Serialize};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use super::terrain_config::{DifficultyConfig, TrainingTerrainConfig};

/// Defines how training environments are sampled during evaluation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvironmentDistribution {
    /// How to sample difficulty within range
    pub difficulty_sampling: DifficultySampling,

    /// Difficulty range to sample from (used by Uniform sampling)
    pub difficulty_range: DifficultyRange,
}

/// Range of difficulty parameters to sample from
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DifficultyRange {
    pub min: DifficultyConfig,
    pub max: DifficultyConfig,
}

impl Default for DifficultyRange {
    fn default() -> Self {
        Self {
            min: DifficultyConfig::flat(),
            max: DifficultyConfig::random(),
        }
    }
}

/// Strategy for sampling difficulty configurations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DifficultySampling {
    /// Uniform random sampling between min and max
    Uniform,

    /// Sample from fixed difficulty levels
    Discrete(Vec<DifficultyConfig>),

    /// Use preset terrain configs (backward compatible)
    Presets(Vec<TrainingTerrainConfig>),
}

impl EnvironmentDistribution {
    /// Sample a single environment configuration deterministically
    ///
    /// Uses eval_id and env_index to derive a deterministic seed:
    /// seed = hash(eval_id, env_index, distribution_hash)
    ///
    /// This ensures that the same creature (eval_id) always sees the same
    /// environments each evaluation, enabling fair comparison and reproducibility.
    pub fn sample(&self, eval_id: u64, env_index: usize) -> Result<TrainingTerrainConfig> {
        let seed = self.derive_seed(eval_id, env_index);
        let mut rng = Xoshiro256PlusPlus::seed_from_u64(seed);

        match &self.difficulty_sampling {
            DifficultySampling::Uniform => self.sample_uniform(&mut rng),
            DifficultySampling::Discrete(configs) => {
                let idx = rng.gen_range(0..configs.len());
                Ok(self.difficulty_to_terrain(&configs[idx], seed))
            }
            DifficultySampling::Presets(presets) => {
                let idx = rng.gen_range(0..presets.len());
                let mut config = presets[idx].clone();
                // Override seed to ensure different terrain with same preset
                config.base_seed = seed;
                Ok(config)
            }
        }
    }

    /// Sample multiple environments in batch (more efficient)
    pub fn sample_batch(&self, eval_id: u64, count: usize) -> Result<Vec<TrainingTerrainConfig>> {
        (0..count).map(|i| self.sample(eval_id, i)).collect()
    }

    /// Sample a single environment for a specific biome (biome specialist training)
    ///
    /// Forces the sampled terrain to use the specified biome, overriding any default
    /// biome distribution. Used for biome specialist training where creatures train
    /// exclusively on one biome type.
    ///
    /// # Arguments
    /// * `eval_id` - Evaluation ID (generation * pop_size + creature_idx)
    /// * `env_index` - Environment index within the batch
    /// * `biome` - Target biome type to force
    pub fn sample_for_biome(
        &self,
        eval_id: u64,
        env_index: usize,
        biome: sunaba_core::world::biome::BiomeType,
    ) -> Result<TrainingTerrainConfig> {
        let mut config = self.sample(eval_id, env_index)?;
        config.biome_type = Some(biome);
        Ok(config)
    }

    /// Sample multiple environments for a specific biome in batch
    ///
    /// # Arguments
    /// * `eval_id` - Evaluation ID
    /// * `count` - Number of environments to sample
    /// * `biome` - Target biome type to force for all environments
    pub fn sample_batch_for_biome(
        &self,
        eval_id: u64,
        count: usize,
        biome: sunaba_core::world::biome::BiomeType,
    ) -> Result<Vec<TrainingTerrainConfig>> {
        (0..count)
            .map(|i| self.sample_for_biome(eval_id, i, biome))
            .collect()
    }

    /// Derive deterministic seed from eval_id and environment index
    ///
    /// Combines:
    /// - eval_id: identifies the evaluation run (generation + creature)
    /// - env_index: which environment in the batch
    /// - config_hash: hash of distribution config for reproducibility
    fn derive_seed(&self, eval_id: u64, env_index: usize) -> u64 {
        let mut hasher = DefaultHasher::new();
        eval_id.hash(&mut hasher);
        env_index.hash(&mut hasher);

        // Include config in hash so changing distribution params changes seeds
        let config_hash = self.config_hash();
        config_hash.hash(&mut hasher);

        hasher.finish()
    }

    fn sample_uniform(&self, rng: &mut impl Rng) -> Result<TrainingTerrainConfig> {
        let diff = DifficultyConfig {
            terrain_roughness: rng.gen_range(
                self.difficulty_range.min.terrain_roughness
                    ..=self.difficulty_range.max.terrain_roughness,
            ),
            obstacle_density: rng.gen_range(
                self.difficulty_range.min.obstacle_density
                    ..=self.difficulty_range.max.obstacle_density,
            ),
            hazard_density: rng.gen_range(
                self.difficulty_range.min.hazard_density..=self.difficulty_range.max.hazard_density,
            ),
            cave_density: rng.gen_range(
                self.difficulty_range.min.cave_density..=self.difficulty_range.max.cave_density,
            ),
            gap_frequency: rng.gen_range(
                self.difficulty_range.min.gap_frequency..=self.difficulty_range.max.gap_frequency,
            ),
        };

        let seed = rng.r#gen();
        Ok(self.difficulty_to_terrain(&diff, seed))
    }

    fn difficulty_to_terrain(&self, diff: &DifficultyConfig, seed: u64) -> TrainingTerrainConfig {
        // Map difficulty params to TrainingTerrainConfig
        // Use default world dimensions
        TrainingTerrainConfig {
            base_seed: seed,
            width: 512,
            height: 128,
            difficulty: diff.clone(),
            worldgen_config: sunaba_core::world::worldgen_config::WorldGenConfig::default(),
            biome_type: None,
        }
    }

    fn config_hash(&self) -> u64 {
        // Hash the configuration for seed derivation
        // This ensures changing the distribution changes the environments
        let mut hasher = DefaultHasher::new();

        // Hash sampling strategy type
        match &self.difficulty_sampling {
            DifficultySampling::Uniform => {
                0u8.hash(&mut hasher);
                // Hash difficulty range (quantize floats to avoid precision issues)
                self.hash_difficulty_quantized(&self.difficulty_range.min, &mut hasher);
                self.hash_difficulty_quantized(&self.difficulty_range.max, &mut hasher);
            }
            DifficultySampling::Discrete(configs) => {
                1u8.hash(&mut hasher);
                configs.len().hash(&mut hasher);
                for config in configs {
                    self.hash_difficulty_quantized(config, &mut hasher);
                }
            }
            DifficultySampling::Presets(presets) => {
                2u8.hash(&mut hasher);
                presets.len().hash(&mut hasher);
                // Hash preset seeds and dimensions
                for preset in presets {
                    preset.base_seed.hash(&mut hasher);
                    preset.width.hash(&mut hasher);
                    preset.height.hash(&mut hasher);
                }
            }
        }

        hasher.finish()
    }

    fn hash_difficulty_quantized(&self, diff: &DifficultyConfig, hasher: &mut DefaultHasher) {
        // Quantize floats to 2 decimal places to avoid floating point precision issues
        let quantize = |f: f32| (f * 100.0).round() as i32;

        quantize(diff.terrain_roughness).hash(hasher);
        quantize(diff.obstacle_density).hash(hasher);
        quantize(diff.hazard_density).hash(hasher);
        quantize(diff.cave_density).hash(hasher);
        quantize(diff.gap_frequency).hash(hasher);
    }

    /// Create a simple uniform distribution (good default)
    pub fn uniform(min: DifficultyConfig, max: DifficultyConfig) -> Self {
        Self {
            difficulty_sampling: DifficultySampling::Uniform,
            difficulty_range: DifficultyRange { min, max },
        }
    }

    /// Create distribution from discrete difficulty levels
    pub fn discrete(configs: Vec<DifficultyConfig>) -> Self {
        Self {
            difficulty_sampling: DifficultySampling::Discrete(configs),
            difficulty_range: DifficultyRange::default(),
        }
    }

    /// Create distribution from preset configs (backward compatible)
    pub fn from_presets(presets: Vec<TrainingTerrainConfig>) -> Self {
        Self {
            difficulty_sampling: DifficultySampling::Presets(presets),
            difficulty_range: DifficultyRange::default(),
        }
    }
}

impl Default for EnvironmentDistribution {
    fn default() -> Self {
        // Backward compatible: use all 5 existing presets
        Self::from_presets(vec![
            TrainingTerrainConfig::flat(42, 512, 128),
            TrainingTerrainConfig::gentle_hills(43, 512, 128),
            TrainingTerrainConfig::obstacles(44, 512, 128),
            TrainingTerrainConfig::hazards(45, 512, 128),
            TrainingTerrainConfig::random(46, 512, 128),
        ])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deterministic_sampling() {
        let dist = EnvironmentDistribution::default();

        // Same eval_id and env_index should produce same result
        let config1 = dist.sample(12345, 0).unwrap();
        let config2 = dist.sample(12345, 0).unwrap();

        assert_eq!(config1.base_seed, config2.base_seed);
        assert_eq!(config1.width, config2.width);
        assert_eq!(config1.height, config2.height);
    }

    #[test]
    fn test_different_environments() {
        let dist = EnvironmentDistribution::default();

        // Different env_index should produce different results
        let config1 = dist.sample(12345, 0).unwrap();
        let config2 = dist.sample(12345, 1).unwrap();

        // Should get different presets or at least different seeds
        assert!(config1.base_seed != config2.base_seed || config1.difficulty != config2.difficulty);
    }

    #[test]
    fn test_different_creatures() {
        let dist = EnvironmentDistribution::default();

        // Different eval_id should produce different results
        let config1 = dist.sample(100, 0).unwrap();
        let config2 = dist.sample(200, 0).unwrap();

        assert_ne!(config1.base_seed, config2.base_seed);
    }

    #[test]
    fn test_batch_consistency() {
        let dist = EnvironmentDistribution::default();

        let batch = dist.sample_batch(12345, 5).unwrap();
        assert_eq!(batch.len(), 5);

        for (i, config) in batch.iter().enumerate() {
            let individual = dist.sample(12345, i).unwrap();
            assert_eq!(config.base_seed, individual.base_seed);
            assert_eq!(config.width, individual.width);
        }
    }

    #[test]
    fn test_uniform_sampling() {
        let dist = EnvironmentDistribution::uniform(
            DifficultyConfig::flat(),
            DifficultyConfig::gentle_hills(),
        );

        // Sample multiple times, all should be in range
        for i in 0..10 {
            let config = dist.sample(1000, i).unwrap();
            assert!(config.difficulty.terrain_roughness >= 0.0);
            assert!(config.difficulty.terrain_roughness <= 0.3);
        }
    }

    #[test]
    fn test_discrete_sampling() {
        let configs = vec![
            DifficultyConfig::flat(),
            DifficultyConfig::gentle_hills(),
            DifficultyConfig::obstacles(),
        ];
        let dist = EnvironmentDistribution::discrete(configs.clone());

        // Sample should always return one of the provided configs
        for i in 0..10 {
            let sampled = dist.sample(2000, i).unwrap();
            // Check that difficulty matches one of the input configs
            let matches = configs
                .iter()
                .any(|c| c.terrain_roughness == sampled.difficulty.terrain_roughness);
            assert!(matches, "Sampled config should match one of the inputs");
        }
    }

    #[test]
    fn test_config_hash_stability() {
        let dist = EnvironmentDistribution::default();

        // Same config should produce same hash
        let hash1 = dist.config_hash();
        let hash2 = dist.config_hash();
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_config_hash_uniqueness() {
        let dist1 = EnvironmentDistribution::uniform(
            DifficultyConfig::flat(),
            DifficultyConfig::gentle_hills(),
        );
        let dist2 = EnvironmentDistribution::uniform(
            DifficultyConfig::flat(),
            DifficultyConfig::obstacles(),
        );

        // Different configs should produce different hashes
        let hash1 = dist1.config_hash();
        let hash2 = dist2.config_hash();
        assert_ne!(hash1, hash2);
    }
}
