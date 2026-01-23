use glam::Vec2;
use serde::{Deserialize, Serialize};

use super::{
    EntityId,
    health::{Health, Hunger},
    inventory::Inventory,
};
use crate::simulation::mining::MiningProgress;
use sunaba_simulation::MaterialId;

/// The player entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Player {
    pub id: EntityId,
    pub position: Vec2,
    pub velocity: Vec2,
    pub grounded: bool,   // Is player standing on ground?
    pub coyote_time: f32, // Grace period after leaving ground (0.1s)
    pub jump_buffer: f32, // Jump input buffering (0.1s)
    pub inventory: Inventory,
    pub health: Health,
    pub hunger: Hunger,
    pub selected_slot: usize, // Currently selected inventory slot (for hotbar)
    pub equipped_tool: Option<u16>, // Currently equipped tool ID (1000+)
    pub mining_progress: MiningProgress, // Mining progress tracker
    pub is_dead: bool,        // Track death state explicitly

    /// Pending knockback impulse (accumulated this frame, cleared after physics)
    /// This allows multiple sources to add knockback (mining, explosions, etc.)
    #[serde(skip)]
    pub pending_knockback: Vec2,

    /// Dash mechanics
    #[serde(skip)]
    pub dash_direction: Vec2, // Direction of active dash (unit vector)

    #[serde(skip)]
    pub dash_timer: f32, // Remaining dash duration (0.15s → 0.0)

    #[serde(skip)]
    pub dash_cooldown: f32, // Cooldown until next dash allowed (0.5s → 0.0)

    #[serde(skip)]
    pub air_dash_used: bool, // Has air dash been used this jump?
}

impl Player {
    pub const WIDTH: f32 = 8.0; // pixels
    pub const HEIGHT: f32 = 12.0; // pixels (reduced from 16 for Noita-like proportions)

    // Physics constants
    pub const GRAVITY: f32 = 800.0; // px/s² (downward)
    pub const JUMP_VELOCITY: f32 = 300.0; // px/s (upward)
    pub const MAX_FALL_SPEED: f32 = 500.0; // Terminal velocity
    pub const COYOTE_TIME: f32 = 0.1; // Jump grace period (seconds)
    pub const JUMP_BUFFER: f32 = 0.1; // Jump input buffer (seconds)
    pub const FLIGHT_THRUST: f32 = 1200.0; // px/s² (upward, Noita-style levitation)
    pub const MAX_STEP_UP_HEIGHT: f32 = 4.0; // Max height player can auto-step up (pixels)

    // Dash mechanics
    pub const DASH_SPEED: f32 = 400.0; // px/s (2x walk speed)
    pub const DASH_DURATION: f32 = 0.15; // 0.15s = 9 frames at 60fps
    pub const DASH_COOLDOWN: f32 = 0.5; // 0.5s between dashes

    /// Create a new player at the specified position
    pub fn new(position: Vec2) -> Self {
        let mut player = Player {
            id: EntityId::new(),
            position,
            velocity: Vec2::ZERO,
            grounded: false, // Start in air
            coyote_time: 0.0,
            jump_buffer: 0.0,
            inventory: Inventory::new(50), // 50 slots
            health: Health::new(100.0),
            hunger: Hunger::new(100.0, 0.1, 1.0), // Drain 0.1/sec, 1.0 dmg/sec when starving
            selected_slot: 0,
            equipped_tool: None,
            mining_progress: MiningProgress::new(),
            is_dead: false,
            pending_knockback: Vec2::ZERO,
            dash_direction: Vec2::ZERO,
            dash_timer: 0.0,
            dash_cooldown: 0.0,
            air_dash_used: false,
        };

        // Give player some starting materials for testing
        player.inventory.add_item(MaterialId::SAND, 1000);
        player.inventory.add_item(MaterialId::WATER, 1000);
        player.inventory.add_item(MaterialId::WOOD, 1000);
        player.inventory.add_item(MaterialId::FIRE, 1000);

        player
    }

    /// Create a player from existing data (for deserialization)
    #[allow(clippy::too_many_arguments)]
    pub fn from_data(
        id: EntityId,
        position: Vec2,
        velocity: Vec2,
        inventory: Inventory,
        health: Health,
        hunger: Hunger,
        selected_slot: usize,
        equipped_tool: Option<u16>,
        mining_progress: MiningProgress,
        is_dead: bool,
    ) -> Self {
        Player {
            id,
            position,
            velocity,
            grounded: false,  // Runtime physics state
            coyote_time: 0.0, // Runtime physics state
            jump_buffer: 0.0, // Runtime physics state
            inventory,
            health,
            hunger,
            selected_slot,
            equipped_tool,
            mining_progress,
            is_dead,
            pending_knockback: Vec2::ZERO, // Runtime physics state
            dash_direction: Vec2::ZERO,    // Runtime dash state
            dash_timer: 0.0,               // Runtime dash state
            dash_cooldown: 0.0,            // Runtime dash state
            air_dash_used: false,          // Runtime dash state
        }
    }

    /// Update player state (hunger, health, etc.)
    /// Returns true if the player died this frame
    pub fn update(&mut self, delta_time: f32) -> bool {
        // Update hunger and get starvation damage
        let starvation_damage = self.hunger.update(delta_time);

        // Apply starvation damage
        if starvation_damage > 0.0 {
            self.health.take_damage(starvation_damage);
        }

        // Check if player died this frame
        if self.health.is_dead() && !self.is_dead {
            self.is_dead = true;
            return true; // Signal death to caller
        }

        false
    }

    /// Update mining progress
    /// Returns true if mining completed this frame
    pub fn update_mining(&mut self, delta_time: f32) -> bool {
        self.mining_progress.update(delta_time)
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
    /// Returns None if slot is empty or slot contains a tool
    pub fn get_selected_material(&self) -> Option<u16> {
        self.inventory
            .get_slot(self.selected_slot)
            .and_then(|slot| slot.as_ref())
            .and_then(|stack| stack.material_id())
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

    /// Equip a tool by ID
    pub fn equip_tool(&mut self, tool_id: u16) {
        self.equipped_tool = Some(tool_id);
    }

    /// Unequip the currently equipped tool
    pub fn unequip_tool(&mut self) {
        self.equipped_tool = None;
    }

    /// Get the currently equipped tool ID
    pub fn get_equipped_tool_id(&self) -> Option<u16> {
        self.equipped_tool
    }

    /// Get the currently equipped tool from the registry
    pub fn get_equipped_tool<'a>(
        &self,
        tool_registry: &'a crate::entity::tools::ToolRegistry,
    ) -> Option<&'a crate::entity::tools::ToolDef> {
        self.equipped_tool.and_then(|id| tool_registry.get(id))
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
        self.is_dead = false; // Clear death flag
        // Keep inventory on respawn (optional: can clear if you want)
    }

    /// Check if player can initiate dash
    pub fn can_dash(&self, grounded: bool) -> bool {
        // Cooldown must be expired
        if self.dash_cooldown > 0.0 {
            return false;
        }

        // Can always dash on ground
        if grounded {
            return true;
        }

        // Can dash once in air per jump
        !self.air_dash_used
    }

    /// Start a dash in the given direction
    pub fn start_dash(&mut self, direction: Vec2, grounded: bool) {
        if !self.can_dash(grounded) {
            return;
        }

        // Normalize direction (ensure unit vector)
        let dir = if direction.length() > 0.0 {
            direction.normalize()
        } else {
            Vec2::X // Default to right if no direction
        };

        self.dash_direction = dir;
        self.dash_timer = Self::DASH_DURATION;
        self.dash_cooldown = Self::DASH_COOLDOWN;

        // Mark air dash as used
        if !grounded {
            self.air_dash_used = true;
        }
    }

    /// Update dash timers (call every frame)
    pub fn update_dash(&mut self, dt: f32) {
        // Decrement dash timer
        self.dash_timer = (self.dash_timer - dt).max(0.0);

        // Decrement cooldown timer
        self.dash_cooldown = (self.dash_cooldown - dt).max(0.0);
    }

    /// Check if currently dashing
    pub fn is_dashing(&self) -> bool {
        self.dash_timer > 0.0
    }

    /// Get current dash velocity
    pub fn dash_velocity(&self) -> Vec2 {
        if self.is_dashing() {
            self.dash_direction * Self::DASH_SPEED
        } else {
            Vec2::ZERO
        }
    }

    /// Reset air dash on landing
    pub fn reset_air_dash(&mut self) {
        self.air_dash_used = false;
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

    #[test]
    fn test_dash_cooldown() {
        let mut player = Player::new(Vec2::ZERO);

        // Can dash initially
        assert!(player.can_dash(true));

        // Start dash
        player.start_dash(Vec2::X, true);
        assert!(player.is_dashing());
        assert_eq!(player.dash_timer, Player::DASH_DURATION);

        // Cannot dash again (cooldown)
        assert!(!player.can_dash(true));

        // Update until cooldown expires
        player.update_dash(Player::DASH_COOLDOWN + 0.01);
        assert!(player.can_dash(true));
    }

    #[test]
    fn test_air_dash_limit() {
        let mut player = Player::new(Vec2::ZERO);

        // Can air dash initially
        assert!(player.can_dash(false));

        // Use air dash
        player.start_dash(Vec2::X, false);
        assert!(player.air_dash_used);

        // Wait for cooldown
        player.update_dash(Player::DASH_COOLDOWN + 0.01);

        // Still can't dash in air (used air dash)
        assert!(!player.can_dash(false));

        // Reset on landing
        player.reset_air_dash();
        assert!(player.can_dash(false));
    }
}
