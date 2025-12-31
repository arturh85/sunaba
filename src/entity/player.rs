use glam::Vec2;
use serde::{Deserialize, Serialize};

use super::{EntityId, inventory::Inventory, health::{Health, Hunger}};

/// The player entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Player {
    pub id: EntityId,
    pub position: Vec2,
    pub velocity: Vec2,
    pub inventory: Inventory,
    pub health: Health,
    pub hunger: Hunger,
    pub selected_slot: usize, // Currently selected inventory slot (for hotbar)
}

impl Player {
    /// Create a new player at the specified position
    pub fn new(position: Vec2) -> Self {
        let mut player = Player {
            id: EntityId::new(),
            position,
            velocity: Vec2::ZERO,
            inventory: Inventory::new(50), // 50 slots
            health: Health::new(100.0),
            hunger: Hunger::new(100.0, 0.1, 1.0), // Drain 0.1/sec, 1.0 dmg/sec when starving
            selected_slot: 0,
        };

        // Give player some starting materials for testing
        // Using material IDs from simulation/materials.rs
        player.inventory.add_item(2, 100); // Stone
        player.inventory.add_item(3, 100); // Sand
        player.inventory.add_item(4, 50);  // Water
        player.inventory.add_item(5, 50);  // Wood

        player
    }

    /// Create a player from existing data (for deserialization)
    pub fn from_data(
        id: EntityId,
        position: Vec2,
        velocity: Vec2,
        inventory: Inventory,
        health: Health,
        hunger: Hunger,
        selected_slot: usize,
    ) -> Self {
        Player {
            id,
            position,
            velocity,
            inventory,
            health,
            hunger,
            selected_slot,
        }
    }

    /// Update player state (hunger, health, etc.)
    /// Returns true if the player died
    pub fn update(&mut self, delta_time: f32) -> bool {
        // Update hunger and get starvation damage
        let starvation_damage = self.hunger.update(delta_time);

        // Apply starvation damage
        if starvation_damage > 0.0 {
            let died = self.health.take_damage(starvation_damage);
            if died {
                return true;
            }
        }

        false
    }

    /// Move the player by the specified velocity
    pub fn move_by(&mut self, delta: Vec2) {
        self.position += delta;
    }

    /// Set the player's velocity
    pub fn set_velocity(&mut self, velocity: Vec2) {
        self.velocity = velocity;
    }

    /// Try to mine a material and add it to inventory
    /// Returns true if the material was successfully added
    pub fn mine_material(&mut self, material_id: u16) -> bool {
        let remaining = self.inventory.add_item(material_id, 1);
        remaining == 0 // True if fully added
    }

    /// Try to place a material from inventory
    /// Returns true if the material was available and consumed
    pub fn consume_material(&mut self, material_id: u16) -> bool {
        let removed = self.inventory.remove_item(material_id, 1);
        removed > 0
    }

    /// Get the currently selected material from inventory hotbar
    /// Returns None if slot is empty
    pub fn get_selected_material(&self) -> Option<u16> {
        self.inventory
            .get_slot(self.selected_slot)
            .and_then(|slot| slot.as_ref())
            .map(|stack| stack.material_id)
    }

    /// Select the next inventory slot (cycles through 0-9 for hotbar)
    pub fn select_next_slot(&mut self) {
        self.selected_slot = (self.selected_slot + 1) % 10;
    }

    /// Select the previous inventory slot
    pub fn select_prev_slot(&mut self) {
        self.selected_slot = if self.selected_slot == 0 {
            9
        } else {
            self.selected_slot - 1
        };
    }

    /// Select a specific inventory slot
    pub fn select_slot(&mut self, slot: usize) {
        self.selected_slot = slot.min(49); // Clamp to max slots
    }

    /// Eat food from inventory
    /// material_id: The material to eat
    /// nutritional_value: How much hunger it restores
    /// Returns true if the food was consumed
    pub fn eat_food(&mut self, material_id: u16, nutritional_value: f32) -> bool {
        if self.inventory.has_item(material_id, 1) {
            let removed = self.inventory.remove_item(material_id, 1);
            if removed > 0 {
                self.hunger.eat(nutritional_value);
                return true;
            }
        }
        false
    }

    /// Check if the player is alive
    pub fn is_alive(&self) -> bool {
        !self.health.is_dead()
    }

    /// Check if the player is starving
    pub fn is_starving(&self) -> bool {
        self.hunger.is_starving()
    }

    /// Respawn the player at a new position
    pub fn respawn(&mut self, position: Vec2) {
        self.position = position;
        self.velocity = Vec2::ZERO;
        self.health = Health::new(100.0);
        self.hunger = Hunger::new(100.0, 0.1, 1.0);
        // Keep inventory on respawn (optional: can clear if you want)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_player_creation() {
        let player = Player::new(Vec2::new(100.0, 200.0));
        assert_eq!(player.position, Vec2::new(100.0, 200.0));
        assert_eq!(player.velocity, Vec2::ZERO);
        assert_eq!(player.inventory.max_slots, 50);
        assert_eq!(player.health.current, 100.0);
        assert_eq!(player.hunger.current, 100.0);
        assert!(player.is_alive());
        assert!(!player.is_starving());
    }

    #[test]
    fn test_player_movement() {
        let mut player = Player::new(Vec2::ZERO);
        player.move_by(Vec2::new(10.0, 20.0));
        assert_eq!(player.position, Vec2::new(10.0, 20.0));

        player.set_velocity(Vec2::new(5.0, 5.0));
        assert_eq!(player.velocity, Vec2::new(5.0, 5.0));
    }

    #[test]
    fn test_player_mining_and_placement() {
        let mut player = Player::new(Vec2::ZERO);

        // Mine some materials
        assert!(player.mine_material(1));
        assert_eq!(player.inventory.count_item(1), 1);

        assert!(player.mine_material(1));
        assert_eq!(player.inventory.count_item(1), 2);

        // Place a material
        assert!(player.consume_material(1));
        assert_eq!(player.inventory.count_item(1), 1);

        // Try to place material we don't have
        assert!(!player.consume_material(99));
    }

    #[test]
    fn test_player_starvation() {
        let mut player = Player::new(Vec2::ZERO);

        // Simulate 1000 seconds to fully deplete hunger
        player.update(1000.0);
        assert!(player.is_starving());

        // Simulate more time to cause starvation damage
        player.update(50.0);
        assert!(player.health.current < 100.0);

        // Continue until death
        let mut iterations = 0;
        while player.is_alive() && iterations < 1000 {
            player.update(1.0);
            iterations += 1;
        }
        assert!(!player.is_alive());
    }

    #[test]
    fn test_player_eating() {
        let mut player = Player::new(Vec2::ZERO);

        // Add food to inventory
        player.inventory.add_item(10, 5);

        // Deplete some hunger
        player.hunger.set(50.0);

        // Eat food
        assert!(player.eat_food(10, 30.0));
        assert_eq!(player.hunger.current, 80.0);
        assert_eq!(player.inventory.count_item(10), 4);

        // Try to eat food we don't have
        assert!(!player.eat_food(99, 10.0));
        assert_eq!(player.hunger.current, 80.0);
    }

    #[test]
    fn test_player_slot_selection() {
        let mut player = Player::new(Vec2::ZERO);
        assert_eq!(player.selected_slot, 0);

        player.select_next_slot();
        assert_eq!(player.selected_slot, 1);

        player.select_slot(5);
        assert_eq!(player.selected_slot, 5);

        player.select_prev_slot();
        assert_eq!(player.selected_slot, 4);

        // Test wrap-around
        player.select_slot(9);
        player.select_next_slot();
        assert_eq!(player.selected_slot, 0);

        player.select_prev_slot();
        assert_eq!(player.selected_slot, 9);
    }

    #[test]
    fn test_player_respawn() {
        let mut player = Player::new(Vec2::ZERO);
        player.inventory.add_item(1, 100);
        player.health.take_damage(90.0);
        player.hunger.set(10.0);

        player.respawn(Vec2::new(500.0, 500.0));

        assert_eq!(player.position, Vec2::new(500.0, 500.0));
        assert_eq!(player.velocity, Vec2::ZERO);
        assert_eq!(player.health.current, 100.0);
        assert_eq!(player.hunger.current, 100.0);
        assert_eq!(player.inventory.count_item(1), 100); // Inventory preserved
    }

    #[test]
    fn test_get_selected_material() {
        let mut player = Player::new(Vec2::ZERO);

        // Clear starting inventory for testing
        player.inventory.clear();

        // No material in slot 0
        assert_eq!(player.get_selected_material(), None);

        // Add material to slot 0
        player.inventory.add_item(5, 1);
        assert_eq!(player.get_selected_material(), Some(5));

        // Select slot 1 (empty)
        player.select_next_slot();
        assert_eq!(player.get_selected_material(), None);
    }
}
