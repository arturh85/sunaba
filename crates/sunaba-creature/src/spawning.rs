//! Creature spawning and management
//!
//! Manages creature population, spawning, and removal.

use std::collections::HashMap;

use glam::Vec2;

use crate::EntityId;
use crate::PhysicsWorld;
// Tests need concrete World implementation

use super::creature::Creature;
use super::genome::CreatureGenome;

/// Manages creature population
pub struct CreatureManager {
    creatures: HashMap<EntityId, Creature>,
    max_creatures: usize,
}

impl CreatureManager {
    /// Create new creature manager
    pub fn new(max_creatures: usize) -> Self {
        Self {
            creatures: HashMap::new(),
            max_creatures,
        }
    }

    /// Spawn creature from genome
    pub fn spawn_creature(
        &mut self,
        genome: CreatureGenome,
        position: Vec2,
        physics_world: &mut PhysicsWorld,
    ) -> EntityId {
        // Check if we can spawn
        if !self.can_spawn() {
            log::warn!(
                "Cannot spawn creature: max population reached ({})",
                self.max_creatures
            );
            return EntityId::new(); // Return dummy ID
        }

        // Create creature from genome
        let mut creature = Creature::from_genome(genome, position);
        let id = creature.id;

        // Build physics body
        creature.rebuild_physics(physics_world);

        // Add to manager
        self.creatures.insert(id, creature);

        log::info!(
            "Spawned creature {} at ({:.1}, {:.1}). Population: {}/{}",
            id,
            position.x,
            position.y,
            self.count(),
            self.max_creatures
        );

        id
    }

    /// Spawn creature from genome with specified initial hunger level
    ///
    /// # Arguments
    /// * `initial_hunger_percent` - 0.0 to 1.0, where 1.0 is full and 0.0 is starving
    pub fn spawn_creature_with_hunger(
        &mut self,
        genome: CreatureGenome,
        position: Vec2,
        initial_hunger_percent: f32,
        physics_world: &mut PhysicsWorld,
    ) -> EntityId {
        // Check if we can spawn
        if !self.can_spawn() {
            log::warn!(
                "Cannot spawn creature: max population reached ({})",
                self.max_creatures
            );
            return EntityId::new(); // Return dummy ID
        }

        // Create creature from genome
        let mut creature = Creature::from_genome(genome, position);
        let id = creature.id;

        // Set initial hunger level
        let max_hunger = creature.hunger.max;
        creature
            .hunger
            .set(max_hunger * initial_hunger_percent.clamp(0.0, 1.0));

        // Build physics body
        creature.rebuild_physics(physics_world);

        // Add to manager
        self.creatures.insert(id, creature);

        log::info!(
            "Spawned creature {} at ({:.1}, {:.1}) with {:.0}% hunger. Population: {}/{}",
            id,
            position.x,
            position.y,
            initial_hunger_percent * 100.0,
            self.count(),
            self.max_creatures
        );

        id
    }

    /// Remove creature
    pub fn remove_creature(&mut self, id: EntityId, physics_world: &mut PhysicsWorld) {
        if let Some(creature) = self.creatures.remove(&id) {
            // Clean up physics
            if let Some(physics) = creature.physics {
                for body_handle in &physics.link_handles {
                    physics_world.remove_rigid_body(*body_handle);
                }
            }

            log::info!(
                "Removed creature {}. Population: {}/{}",
                id,
                self.count(),
                self.max_creatures
            );
        }
    }

    /// Update all creatures
    pub fn update(
        &mut self,
        delta_time: f32,
        world: &impl crate::WorldAccess,
        physics_world: &mut PhysicsWorld,
    ) {
        use super::sensors::SensoryInput;

        let mut dead_creatures = Vec::new();

        // Collect creature IDs to iterate over (to avoid borrow issues)
        let creature_ids: Vec<EntityId> = self.creatures.keys().copied().collect();

        // Update each creature
        for id in creature_ids {
            let Some(creature) = self.creatures.get_mut(&id) else {
                continue;
            };

            // Gather sensory input
            let sensory_input =
                SensoryInput::gather(world, creature.position, &creature.sensor_config);

            // Update creature state (hunger, needs, planning, neural control)
            let died = creature.update(delta_time, &sensory_input, physics_world, world);

            if died {
                dead_creatures.push(id);
                continue;
            }

            // Apply movement physics (gravity, wandering, collision)
            creature.apply_movement(world, physics_world, delta_time);
        }

        // Remove dead creatures
        for id in dead_creatures {
            self.remove_creature(id, physics_world);
            log::info!("Creature {} died", id);
        }
    }

    /// Update all creatures with cached food positions (optimized for training)
    ///
    /// Uses pre-computed food positions instead of scanning all pixels,
    /// reducing food detection from O(r²) to O(n_food).
    pub fn update_with_cache(
        &mut self,
        delta_time: f32,
        world: &impl crate::WorldAccess,
        physics_world: &mut PhysicsWorld,
        food_positions: &[glam::Vec2],
    ) {
        use super::sensors::SensoryInput;

        let mut dead_creatures = Vec::new();

        // Collect creature IDs to iterate over (to avoid borrow issues)
        let creature_ids: Vec<EntityId> = self.creatures.keys().copied().collect();

        // Update each creature
        for id in creature_ids {
            let Some(creature) = self.creatures.get_mut(&id) else {
                continue;
            };

            // Gather sensory input using cached food positions
            let sensory_input = SensoryInput::gather_with_cache(
                world,
                creature.position,
                &creature.sensor_config,
                food_positions,
            );

            // Update creature state (hunger, needs, planning, neural control)
            let died = creature.update(delta_time, &sensory_input, physics_world, world);

            if died {
                dead_creatures.push(id);
                continue;
            }

            // Apply movement physics (gravity, wandering, collision)
            creature.apply_movement(world, physics_world, delta_time);
        }

        // Remove dead creatures
        for id in dead_creatures {
            self.remove_creature(id, physics_world);
            log::info!("Creature {} died", id);
        }
    }

    /// Execute creature actions (requires mutable world)
    pub fn execute_actions(&mut self, world: &mut impl crate::WorldMutAccess, delta_time: f32) {
        for creature in self.creatures.values_mut() {
            creature.execute_action(world, delta_time);
        }
    }

    /// Get number of active creatures
    pub fn count(&self) -> usize {
        self.creatures.len()
    }

    /// Check if can spawn more creatures
    pub fn can_spawn(&self) -> bool {
        self.creatures.len() < self.max_creatures
    }

    /// Get all creature positions (for rendering)
    pub fn get_positions(&self) -> Vec<Vec2> {
        self.creatures.values().map(|c| c.position).collect()
    }

    /// Get render data for all creatures (for rendering)
    pub fn get_render_data(&self, physics_world: &PhysicsWorld) -> Vec<super::CreatureRenderData> {
        self.creatures
            .values()
            .filter_map(|creature| creature.get_render_data(physics_world))
            .collect()
    }

    /// Find creature at or near world position (within radius)
    /// Used for mouse hover detection
    pub fn get_creature_at_position(&self, pos: Vec2, radius: f32) -> Option<&Creature> {
        for creature in self.creatures.values() {
            let dist = (creature.position - pos).length();
            if dist <= radius {
                return Some(creature);
            }
        }
        None
    }

    /// Get creature by ID
    pub fn get(&self, id: EntityId) -> Option<&Creature> {
        self.creatures.get(&id)
    }

    /// Get mutable creature by ID
    pub fn get_mut(&mut self, id: EntityId) -> Option<&mut Creature> {
        self.creatures.get_mut(&id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_creature_manager_creation() {
        let manager = CreatureManager::new(100);
        assert_eq!(manager.count(), 0);
        assert_eq!(manager.max_creatures, 100);
        assert!(manager.can_spawn());
    }

    #[test]
    fn test_spawn_creature() {
        let mut manager = CreatureManager::new(10);
        let mut physics_world = PhysicsWorld::new();

        let genome = CreatureGenome::test_biped();
        let position = Vec2::new(100.0, 100.0);

        let id = manager.spawn_creature(genome, position, &mut physics_world);

        assert_eq!(manager.count(), 1);
        assert!(manager.get(id).is_some());

        // Creature should have physics built
        let creature = manager.get(id).unwrap();
        assert!(creature.physics.is_some());
    }

    #[test]
    fn test_spawn_multiple_creatures() {
        let mut manager = CreatureManager::new(10);
        let mut physics_world = PhysicsWorld::new();

        for i in 0..5 {
            let genome = CreatureGenome::test_biped();
            let position = Vec2::new(100.0 + i as f32 * 10.0, 100.0);
            manager.spawn_creature(genome, position, &mut physics_world);
        }

        assert_eq!(manager.count(), 5);
        assert!(manager.can_spawn());
    }

    #[test]
    fn test_max_population_limit() {
        let mut manager = CreatureManager::new(3);
        let mut physics_world = PhysicsWorld::new();

        // Spawn up to max
        for i in 0..3 {
            let genome = CreatureGenome::test_biped();
            let position = Vec2::new(100.0 + i as f32 * 10.0, 100.0);
            manager.spawn_creature(genome, position, &mut physics_world);
        }

        assert_eq!(manager.count(), 3);
        assert!(!manager.can_spawn());

        // Try to spawn one more (should fail gracefully)
        let genome = CreatureGenome::test_biped();
        manager.spawn_creature(genome, Vec2::ZERO, &mut physics_world);

        // Count should still be 3
        assert_eq!(manager.count(), 3);
    }

    #[test]
    fn test_remove_creature() {
        let mut manager = CreatureManager::new(10);
        let mut physics_world = PhysicsWorld::new();

        let genome = CreatureGenome::test_biped();
        let id = manager.spawn_creature(genome, Vec2::ZERO, &mut physics_world);

        assert_eq!(manager.count(), 1);

        manager.remove_creature(id, &mut physics_world);

        assert_eq!(manager.count(), 0);
        assert!(manager.get(id).is_none());
    }

    #[test]
    fn test_get_positions() {
        let mut manager = CreatureManager::new(10);
        let mut physics_world = PhysicsWorld::new();

        let positions = vec![
            Vec2::new(100.0, 100.0),
            Vec2::new(200.0, 200.0),
            Vec2::new(300.0, 300.0),
        ];

        for pos in &positions {
            let genome = CreatureGenome::test_biped();
            manager.spawn_creature(genome, *pos, &mut physics_world);
        }

        let creature_positions = manager.get_positions();
        assert_eq!(creature_positions.len(), 3);

        // All positions should be present (order may differ)
        for pos in positions {
            assert!(creature_positions.contains(&pos));
        }
    }

    #[test]
    #[ignore] // Requires concrete World implementation from sunaba-core
    fn test_update_creatures() {
        // This test requires World::new() from sunaba-core
    }

    #[test]
    #[ignore] // Requires concrete World implementation from sunaba-core
    fn test_creature_death_removal() {
        // This test requires World::new() from sunaba-core
    }

    #[test]
    fn test_spawn_creature_with_hunger() {
        let mut manager = CreatureManager::new(10);
        let mut physics_world = PhysicsWorld::new();

        let genome = CreatureGenome::test_biped();
        let position = Vec2::new(100.0, 100.0);

        // Spawn with 50% hunger
        let id = manager.spawn_creature_with_hunger(genome, position, 0.5, &mut physics_world);

        let creature = manager.get(id).unwrap();

        // Hunger should be at 50%
        let hunger_percent = creature.hunger.percentage();
        assert!((hunger_percent - 0.5).abs() < 0.01);
    }
}
