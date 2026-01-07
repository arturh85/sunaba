//! MAP-Elites implementation for maintaining behavioral diversity
//!
//! MAP-Elites maintains a grid of elite individuals, where each cell
//! represents a distinct behavioral niche.

use std::collections::HashMap;

use rand::Rng;
use serde::{Deserialize, Serialize};

use crate::creature::genome::CreatureGenome;
use crate::creature::morphology::CreatureArchetype;

use super::fitness::BehaviorDescriptor;

/// Selection method for parent sampling
#[derive(Debug, Clone, Copy, Default)]
pub enum SelectionMethod {
    /// Uniform random selection (original behavior)
    Uniform,
    /// Tournament selection - pick best from k random candidates
    #[default]
    Tournament,
}

/// An elite individual in the MAP-Elites grid
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Elite {
    /// The elite's genome
    pub genome: CreatureGenome,
    /// Fitness score
    pub fitness: f32,
    /// Behavioral descriptor values
    pub behavior: Vec<f32>,
    /// Generation when this elite was discovered
    pub generation: usize,
    /// Archetype of this creature (for multi-archetype training)
    #[serde(default)]
    pub archetype: CreatureArchetype,
}

/// MAP-Elites grid for maintaining diverse populations
#[derive(Debug, Clone)]
pub struct MapElitesGrid {
    /// Grid cells indexed by (dim0, dim1) coordinates
    cells: HashMap<(usize, usize), Elite>,
    /// Resolution of each dimension (e.g., 10 = 10x10 grid)
    resolution: usize,
    /// Dimension 0 name (for reporting)
    pub dim0_name: String,
    /// Dimension 1 name (for reporting)
    pub dim1_name: String,
    /// Dimension 0 index in BehaviorDescriptor
    dim0_idx: usize,
    /// Dimension 1 index in BehaviorDescriptor
    dim1_idx: usize,
    /// Min/max range for dimension 0
    dim0_range: (f32, f32),
    /// Min/max range for dimension 1
    dim1_range: (f32, f32),
    /// Selection method for parent sampling
    selection_method: SelectionMethod,
    /// Tournament size (how many candidates to compare)
    tournament_size: usize,
}

impl MapElitesGrid {
    /// Create a new MAP-Elites grid
    ///
    /// # Arguments
    /// * `resolution` - Grid resolution (e.g., 10 for 10x10)
    /// * `dim0_name` - Name of first behavioral dimension
    /// * `dim0_idx` - Index in BehaviorDescriptor
    /// * `dim0_range` - Expected range for dimension 0
    /// * `dim1_name` - Name of second behavioral dimension
    /// * `dim1_idx` - Index in BehaviorDescriptor
    /// * `dim1_range` - Expected range for dimension 1
    pub fn new(
        resolution: usize,
        dim0_name: &str,
        dim0_idx: usize,
        dim0_range: (f32, f32),
        dim1_name: &str,
        dim1_idx: usize,
        dim1_range: (f32, f32),
    ) -> Self {
        Self {
            cells: HashMap::new(),
            resolution,
            dim0_name: dim0_name.to_string(),
            dim1_name: dim1_name.to_string(),
            dim0_idx,
            dim1_idx,
            dim0_range,
            dim1_range,
            selection_method: SelectionMethod::Tournament,
            tournament_size: 3, // Default tournament size
        }
    }

    /// Set the selection method
    pub fn set_selection_method(&mut self, method: SelectionMethod) {
        self.selection_method = method;
    }

    /// Set the tournament size (only affects Tournament selection)
    pub fn set_tournament_size(&mut self, size: usize) {
        self.tournament_size = size.max(2); // Minimum of 2
    }

    /// Create default grid using locomotion and foraging efficiency
    pub fn default_grid() -> Self {
        Self::new(
            10, // 10x10 grid
            "Locomotion",
            0, // locomotion_efficiency
            (0.0, 10.0),
            "Foraging",
            1, // foraging_efficiency
            (0.0, 5.0),
        )
    }

    /// Convert a behavior value to a grid index
    fn to_cell_idx(&self, value: f32, range: (f32, f32)) -> usize {
        let normalized = (value - range.0) / (range.1 - range.0);
        let idx = (normalized * self.resolution as f32).floor() as usize;
        idx.min(self.resolution - 1)
    }

    /// Get the cell coordinates for a behavior descriptor
    fn get_cell(&self, behavior: &BehaviorDescriptor) -> (usize, usize) {
        let dim0 = behavior.get_dimension(self.dim0_idx);
        let dim1 = behavior.get_dimension(self.dim1_idx);

        (
            self.to_cell_idx(dim0, self.dim0_range),
            self.to_cell_idx(dim1, self.dim1_range),
        )
    }

    /// Try to insert an individual into the grid
    /// Returns true if the individual was added (either new cell or better than existing)
    pub fn try_insert(
        &mut self,
        genome: CreatureGenome,
        fitness: f32,
        behavior: &BehaviorDescriptor,
        generation: usize,
        archetype: CreatureArchetype,
    ) -> bool {
        let cell = self.get_cell(behavior);

        let behavior_vec = vec![
            behavior.locomotion_efficiency,
            behavior.foraging_efficiency,
            behavior.exploration,
            behavior.activity,
        ];

        let elite = Elite {
            genome,
            fitness,
            behavior: behavior_vec,
            generation,
            archetype,
        };

        match self.cells.get(&cell) {
            None => {
                // Empty cell - insert
                self.cells.insert(cell, elite);
                true
            }
            Some(existing) if fitness > existing.fitness => {
                // Better fitness - replace
                self.cells.insert(cell, elite);
                true
            }
            _ => false, // Existing elite is better
        }
    }

    /// Get the number of occupied cells
    pub fn cell_count(&self) -> usize {
        self.cells.len()
    }

    /// Get the total possible cells
    pub fn total_cells(&self) -> usize {
        self.resolution * self.resolution
    }

    /// Get coverage ratio (occupied / total)
    pub fn coverage(&self) -> f32 {
        self.cell_count() as f32 / self.total_cells() as f32
    }

    /// Get the best elite by fitness
    pub fn best_elite(&self) -> Option<&Elite> {
        self.cells.values().max_by(|a, b| {
            a.fitness
                .partial_cmp(&b.fitness)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
    }

    /// Get all elites
    pub fn elites(&self) -> impl Iterator<Item = &Elite> {
        self.cells.values()
    }

    /// Sample an elite for reproduction using configured selection method
    pub fn sample_elite(&self) -> Option<&Elite> {
        if self.cells.is_empty() {
            return None;
        }

        match self.selection_method {
            SelectionMethod::Uniform => self.sample_uniform(),
            SelectionMethod::Tournament => self.sample_tournament(),
        }
    }

    /// Uniform random selection (original behavior)
    fn sample_uniform(&self) -> Option<&Elite> {
        let mut rng = rand::thread_rng();
        let keys: Vec<_> = self.cells.keys().collect();
        let idx = rng.gen_range(0..keys.len());
        self.cells.get(keys[idx])
    }

    /// Tournament selection - pick best from k random candidates
    /// This creates selection pressure toward higher fitness elites
    fn sample_tournament(&self) -> Option<&Elite> {
        let mut rng = rand::thread_rng();
        let keys: Vec<_> = self.cells.keys().collect();

        let mut best: Option<&Elite> = None;
        let mut best_fitness = f32::NEG_INFINITY;

        // Sample tournament_size random candidates
        let actual_size = self.tournament_size.min(keys.len());
        for _ in 0..actual_size {
            let idx = rng.gen_range(0..keys.len());
            if let Some(elite) = self.cells.get(keys[idx])
                && elite.fitness > best_fitness {
                    best_fitness = elite.fitness;
                    best = Some(elite);
                }
        }

        best
    }

    /// Sample two different elites for crossover using configured selection method
    pub fn sample_parents(&self) -> Option<(&Elite, &Elite)> {
        if self.cells.len() < 2 {
            return None;
        }

        // Sample first parent using selection method
        let parent1 = self.sample_elite()?;

        // Sample second parent, ensuring it's different
        // Try a few times with selection method, then fallback to random
        let mut attempts = 0;
        loop {
            if let Some(parent2) = self.sample_elite()
                && !std::ptr::eq(parent1, parent2) {
                    return Some((parent1, parent2));
                }
            attempts += 1;
            if attempts >= 10 {
                break;
            }
        }

        // Fallback: uniform random different parent
        let mut rng = rand::thread_rng();
        let keys: Vec<_> = self.cells.keys().collect();
        for _ in 0..keys.len() {
            let idx = rng.gen_range(0..keys.len());
            if let Some(parent2) = self.cells.get(keys[idx])
                && !std::ptr::eq(parent1, parent2) {
                    return Some((parent1, parent2));
                }
        }

        None
    }

    /// Get statistics about the grid
    pub fn stats(&self) -> GridStats {
        let fitnesses: Vec<f32> = self.cells.values().map(|e| e.fitness).collect();

        GridStats {
            cell_count: self.cell_count(),
            total_cells: self.total_cells(),
            coverage: self.coverage(),
            best_fitness: fitnesses.iter().copied().fold(f32::NEG_INFINITY, f32::max),
            avg_fitness: if fitnesses.is_empty() {
                0.0
            } else {
                fitnesses.iter().sum::<f32>() / fitnesses.len() as f32
            },
            min_fitness: fitnesses.iter().copied().fold(f32::INFINITY, f32::min),
        }
    }

    /// Clear the grid
    pub fn clear(&mut self) {
        self.cells.clear();
    }

    /// Get grid as 2D array for visualization (fitness values, -1 for empty)
    pub fn as_fitness_grid(&self) -> Vec<Vec<f32>> {
        let mut grid = vec![vec![-1.0; self.resolution]; self.resolution];

        for ((x, y), elite) in &self.cells {
            if *x < self.resolution && *y < self.resolution {
                grid[*y][*x] = elite.fitness;
            }
        }

        grid
    }

    /// Sample diverse elites from different regions of the grid
    ///
    /// Returns up to 4 elites representing different behavioral strategies:
    /// 1. High dim0 (e.g., high locomotion)
    /// 2. High dim1 (e.g., high foraging)
    /// 3. Balanced (center region)
    /// 4. Random alternative
    pub fn sample_diverse_elites(&self) -> Vec<DiverseElite> {
        let mut results = Vec::new();
        let mid = self.resolution / 2;
        let high = self.resolution - 1;

        // Strategy 1: High dim0 (top-right quadrant for dim0)
        if let Some(elite) = self.find_elite_in_region(mid..=high, 0..=high) {
            results.push(DiverseElite {
                elite: elite.clone(),
                label: format!("High {}", self.dim0_name),
            });
        }

        // Strategy 2: High dim1 (top region for dim1)
        if let Some(elite) = self.find_elite_in_region(0..=high, mid..=high) {
            // Avoid duplicates
            if !results.iter().any(|r| {
                (r.elite.behavior[0] - elite.behavior[0]).abs() < 0.01
                    && (r.elite.behavior[1] - elite.behavior[1]).abs() < 0.01
            }) {
                results.push(DiverseElite {
                    elite: elite.clone(),
                    label: format!("High {}", self.dim1_name),
                });
            }
        }

        // Strategy 3: Balanced (center region)
        let center_start = mid.saturating_sub(1);
        let center_end = (mid + 1).min(high);
        if let Some(elite) =
            self.find_elite_in_region(center_start..=center_end, center_start..=center_end)
            && !results.iter().any(|r| {
                (r.elite.behavior[0] - elite.behavior[0]).abs() < 0.01
                    && (r.elite.behavior[1] - elite.behavior[1]).abs() < 0.01
            }) {
                results.push(DiverseElite {
                    elite: elite.clone(),
                    label: "Balanced".to_string(),
                });
            }

        // Strategy 4: Random alternative (any cell not already picked)
        let stats = self.stats();
        let existing_behaviors: Vec<_> = results.iter().map(|r| &r.elite.behavior).collect();

        for elite in self.cells.values() {
            if elite.fitness > stats.avg_fitness {
                let dominated = existing_behaviors.iter().any(|b| {
                    (b[0] - elite.behavior[0]).abs() < 1.0 && (b[1] - elite.behavior[1]).abs() < 1.0
                });
                if !dominated {
                    results.push(DiverseElite {
                        elite: elite.clone(),
                        label: "Alternative".to_string(),
                    });
                    break;
                }
            }
        }

        results
    }

    /// Find the best elite in a specific region of the grid
    fn find_elite_in_region(
        &self,
        x_range: std::ops::RangeInclusive<usize>,
        y_range: std::ops::RangeInclusive<usize>,
    ) -> Option<&Elite> {
        self.cells
            .iter()
            .filter(|((x, y), _)| x_range.contains(x) && y_range.contains(y))
            .map(|(_, elite)| elite)
            .max_by(|a, b| {
                a.fitness
                    .partial_cmp(&b.fitness)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
    }
}

/// A diverse elite with a descriptive label
#[derive(Debug, Clone)]
pub struct DiverseElite {
    /// The elite creature
    pub elite: Elite,
    /// Label describing this elite's strategy
    pub label: String,
}

/// Statistics about the MAP-Elites grid
#[derive(Debug, Clone)]
pub struct GridStats {
    pub cell_count: usize,
    pub total_cells: usize,
    pub coverage: f32,
    pub best_fitness: f32,
    pub avg_fitness: f32,
    pub min_fitness: f32,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_genome() -> CreatureGenome {
        CreatureGenome::test_biped()
    }

    fn make_behavior(loco: f32, forage: f32) -> BehaviorDescriptor {
        BehaviorDescriptor {
            locomotion_efficiency: loco,
            foraging_efficiency: forage,
            exploration: 0.5,
            activity: 1.0,
        }
    }

    #[test]
    fn test_grid_creation() {
        let grid = MapElitesGrid::default_grid();
        assert_eq!(grid.resolution, 10);
        assert_eq!(grid.total_cells(), 100);
        assert_eq!(grid.cell_count(), 0);
    }

    #[test]
    fn test_insert_elite() {
        let mut grid = MapElitesGrid::default_grid();
        let genome = make_test_genome();
        let behavior = make_behavior(5.0, 2.5); // Mid-range

        let inserted = grid.try_insert(genome, 10.0, &behavior, 0, CreatureArchetype::default());
        assert!(inserted);
        assert_eq!(grid.cell_count(), 1);
    }

    #[test]
    fn test_replace_worse_elite() {
        let mut grid = MapElitesGrid::default_grid();
        let behavior = make_behavior(5.0, 2.5);

        // Insert first elite
        grid.try_insert(
            make_test_genome(),
            10.0,
            &behavior,
            0,
            CreatureArchetype::default(),
        );

        // Try to insert worse elite - should fail
        let replaced = grid.try_insert(
            make_test_genome(),
            5.0,
            &behavior,
            1,
            CreatureArchetype::default(),
        );
        assert!(!replaced);

        // Try to insert better elite - should succeed
        let replaced = grid.try_insert(
            make_test_genome(),
            15.0,
            &behavior,
            2,
            CreatureArchetype::default(),
        );
        assert!(replaced);
        assert_eq!(grid.cell_count(), 1);
        assert!((grid.best_elite().unwrap().fitness - 15.0).abs() < 0.01);
    }

    #[test]
    fn test_different_cells() {
        let mut grid = MapElitesGrid::default_grid();

        // Insert elites with different behaviors
        grid.try_insert(
            make_test_genome(),
            10.0,
            &make_behavior(1.0, 1.0),
            0,
            CreatureArchetype::default(),
        );
        grid.try_insert(
            make_test_genome(),
            15.0,
            &make_behavior(8.0, 4.0),
            0,
            CreatureArchetype::default(),
        );

        assert_eq!(grid.cell_count(), 2);
    }

    #[test]
    fn test_sample_parents() {
        let mut grid = MapElitesGrid::default_grid();

        // Need at least 2 elites
        grid.try_insert(
            make_test_genome(),
            10.0,
            &make_behavior(1.0, 1.0),
            0,
            CreatureArchetype::default(),
        );
        assert!(grid.sample_parents().is_none());

        grid.try_insert(
            make_test_genome(),
            15.0,
            &make_behavior(8.0, 4.0),
            0,
            CreatureArchetype::default(),
        );
        assert!(grid.sample_parents().is_some());
    }

    #[test]
    fn test_grid_stats() {
        let mut grid = MapElitesGrid::default_grid();
        grid.try_insert(
            make_test_genome(),
            10.0,
            &make_behavior(1.0, 1.0),
            0,
            CreatureArchetype::default(),
        );
        grid.try_insert(
            make_test_genome(),
            20.0,
            &make_behavior(8.0, 4.0),
            0,
            CreatureArchetype::default(),
        );

        let stats = grid.stats();
        assert_eq!(stats.cell_count, 2);
        assert!((stats.best_fitness - 20.0).abs() < 0.01);
        assert!((stats.avg_fitness - 15.0).abs() < 0.01);
    }
}
