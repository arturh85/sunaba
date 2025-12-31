use serde::{Deserialize, Serialize};

/// Health component for entities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Health {
    pub current: f32,
    pub max: f32,
}

impl Health {
    /// Create a new health component with the specified max health
    pub fn new(max: f32) -> Self {
        Health {
            current: max,
            max,
        }
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
    pub drain_rate: f32,       // Units per second
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_health_basic() {
        let mut health = Health::new(100.0);
        assert_eq!(health.current, 100.0);
        assert!(!health.is_dead());
        assert!(health.is_full());

        health.take_damage(30.0);
        assert_eq!(health.current, 70.0);
        assert!(!health.is_dead());
        assert!(!health.is_full());

        health.heal(20.0);
        assert_eq!(health.current, 90.0);

        health.heal(50.0); // Overheal
        assert_eq!(health.current, 100.0);
        assert!(health.is_full());
    }

    #[test]
    fn test_health_death() {
        let mut health = Health::new(50.0);
        let died = health.take_damage(60.0);
        assert!(died);
        assert!(health.is_dead());
        assert_eq!(health.current, 0.0);
    }

    #[test]
    fn test_health_percentage() {
        let mut health = Health::new(100.0);
        assert_eq!(health.percentage(), 1.0);

        health.take_damage(50.0);
        assert_eq!(health.percentage(), 0.5);

        health.take_damage(50.0);
        assert_eq!(health.percentage(), 0.0);
    }

    #[test]
    fn test_health_set_max() {
        let mut health = Health::new(100.0);
        health.take_damage(50.0); // 50/100 = 50%

        health.set_max(200.0, true); // Maintain percentage
        assert_eq!(health.current, 100.0); // 50% of 200
        assert_eq!(health.max, 200.0);

        health.set(150.0);
        health.set_max(100.0, false); // Don't maintain percentage
        assert_eq!(health.current, 100.0); // Clamped to new max
    }

    #[test]
    fn test_hunger_basic() {
        let mut hunger = Hunger::new(100.0, 1.0, 5.0);
        assert!(!hunger.is_starving());
        assert!(hunger.is_full());

        let damage = hunger.update(10.0); // 10 seconds
        assert_eq!(hunger.current, 90.0);
        assert_eq!(damage, 0.0); // Not starving yet

        hunger.eat(5.0);
        assert_eq!(hunger.current, 95.0);
    }

    #[test]
    fn test_hunger_starvation() {
        let mut hunger = Hunger::new(10.0, 1.0, 5.0);

        // Drain until starving
        hunger.update(11.0);
        assert!(hunger.is_starving());
        assert_eq!(hunger.current, 0.0);

        // Should cause starvation damage
        let damage = hunger.update(2.0);
        assert_eq!(damage, 10.0); // 5 damage/sec * 2 sec
    }

    #[test]
    fn test_hunger_time_until_starvation() {
        let hunger = Hunger::new(100.0, 0.5, 1.0);
        let time = hunger.time_until_starvation();
        assert_eq!(time, Some(200.0)); // 100 / 0.5 = 200 seconds

        let starving = Hunger::new(0.0, 0.5, 1.0);
        assert_eq!(starving.time_until_starvation(), None);
    }

    #[test]
    fn test_hunger_percentage() {
        let mut hunger = Hunger::new(100.0, 1.0, 1.0);
        assert_eq!(hunger.percentage(), 1.0);

        hunger.update(50.0);
        assert_eq!(hunger.percentage(), 0.5);

        hunger.update(50.0);
        assert_eq!(hunger.percentage(), 0.0);
    }
}
