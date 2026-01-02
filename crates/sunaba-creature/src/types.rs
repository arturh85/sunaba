//! Common types for creatures
//!
//! These types are duplicated from sunaba-core to avoid circular dependencies.

use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};

/// Unique identifier for entities in the world
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EntityId(u64);

static NEXT_ENTITY_ID: AtomicU64 = AtomicU64::new(1);

impl EntityId {
    /// Generate a new unique entity ID
    pub fn new() -> Self {
        EntityId(NEXT_ENTITY_ID.fetch_add(1, Ordering::Relaxed))
    }

    /// Get the raw u64 value (useful for debugging/serialization)
    pub fn raw(&self) -> u64 {
        self.0
    }

    /// Create an EntityId from a raw u64 (for deserialization)
    pub fn from_raw(id: u64) -> Self {
        // Update the counter if this ID is higher than current
        NEXT_ENTITY_ID.fetch_max(id + 1, Ordering::Relaxed);
        EntityId(id)
    }
}

impl Default for EntityId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for EntityId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Entity({})", self.0)
    }
}

/// Health component for entities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Health {
    pub current: f32,
    pub max: f32,
}

impl Health {
    /// Create a new health component with the specified max health
    pub fn new(max: f32) -> Self {
        Health { current: max, max }
    }

    /// Deal damage to this entity
    /// Returns true if the entity died (health <= 0)
    pub fn take_damage(&mut self, amount: f32) -> bool {
        self.current = (self.current - amount).max(0.0);
        self.is_dead()
    }

    /// Heal this entity
    pub fn heal(&mut self, amount: f32) {
        self.current = (self.current + amount).min(self.max);
    }

    /// Check if the entity is dead
    pub fn is_dead(&self) -> bool {
        self.current <= 0.0
    }

    /// Check if the entity is at full health
    pub fn is_full(&self) -> bool {
        self.current >= self.max
    }

    /// Get health as a percentage (0.0 - 1.0)
    pub fn percentage(&self) -> f32 {
        if self.max <= 0.0 {
            0.0
        } else {
            (self.current / self.max).clamp(0.0, 1.0)
        }
    }

    /// Set current health (clamped to 0..=max)
    pub fn set(&mut self, value: f32) {
        self.current = value.clamp(0.0, self.max);
    }

    /// Set max health and optionally adjust current health
    pub fn set_max(&mut self, new_max: f32, maintain_percentage: bool) {
        if maintain_percentage {
            let pct = self.percentage();
            self.max = new_max;
            self.current = (new_max * pct).min(new_max);
        } else {
            self.max = new_max;
            self.current = self.current.min(new_max);
        }
    }
}

impl Default for Health {
    fn default() -> Self {
        Self::new(100.0)
    }
}

/// Hunger component for entities
/// Hunger drains over time and causes starvation damage when depleted
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Hunger {
    pub current: f32,
    pub max: f32,
    pub drain_rate: f32,        // Units per second
    pub starvation_damage: f32, // Damage per second when starving
}

impl Hunger {
    /// Create a new hunger component
    pub fn new(max: f32, drain_rate: f32, starvation_damage: f32) -> Self {
        Hunger {
            current: max,
            max,
            drain_rate,
            starvation_damage,
        }
    }

    /// Update hunger (called each frame)
    /// Returns starvation damage to apply this frame (0.0 if not starving)
    pub fn update(&mut self, delta_time: f32) -> f32 {
        // Drain hunger
        self.current = (self.current - self.drain_rate * delta_time).max(0.0);

        // Apply starvation damage if starving
        if self.is_starving() {
            self.starvation_damage * delta_time
        } else {
            0.0
        }
    }

    /// Eat food to restore hunger
    pub fn eat(&mut self, amount: f32) {
        self.current = (self.current + amount).min(self.max);
    }

    /// Check if the entity is starving
    pub fn is_starving(&self) -> bool {
        self.current <= 0.0
    }

    /// Check if the entity is full
    pub fn is_full(&self) -> bool {
        self.current >= self.max
    }

    /// Get hunger as a percentage (0.0 - 1.0)
    pub fn percentage(&self) -> f32 {
        if self.max <= 0.0 {
            0.0
        } else {
            (self.current / self.max).clamp(0.0, 1.0)
        }
    }

    /// Set current hunger (clamped to 0..=max)
    pub fn set(&mut self, value: f32) {
        self.current = value.clamp(0.0, self.max);
    }

    /// Get time until starvation at current drain rate (in seconds)
    /// Returns None if already starving
    pub fn time_until_starvation(&self) -> Option<f32> {
        if self.is_starving() {
            None
        } else if self.drain_rate <= 0.0 {
            Some(f32::INFINITY)
        } else {
            Some(self.current / self.drain_rate)
        }
    }
}

impl Default for Hunger {
    fn default() -> Self {
        // Default: 100 max, 0.1/sec drain, 1.0/sec starvation damage
        Self::new(100.0, 0.1, 1.0)
    }
}
