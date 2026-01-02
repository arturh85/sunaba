//! Simple state-machine critters that act as food for evolved creatures
//!
//! Critters are non-evolved creatures with fixed behavior:
//! - Single circular body
//! - Simple state machine (wander, flee, idle)
//! - Respawn when eaten
//! - Provide nutrition to creatures that eat them

use glam::Vec2;
use serde::{Deserialize, Serialize};

/// Critter behavior state
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum CritterState {
    /// Randomly wandering around
    Wander { target: (f32, f32) },
    /// Fleeing from a threat
    Flee { from: (f32, f32) },
    /// Standing still
    Idle { timer: f32 },
}

impl Default for CritterState {
    fn default() -> Self {
        Self::Idle { timer: 1.0 }
    }
}

/// A simple non-evolved creature that acts as food
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Critter {
    /// Current position in world coordinates
    pub position: Vec2,
    /// Body radius
    pub radius: f32,
    /// Current behavior state
    pub state: CritterState,
    /// Movement speed
    pub speed: f32,
    /// Detection radius for threats
    pub threat_radius: f32,
    /// Nutrition value when eaten
    pub nutrition: f32,
    /// Whether this critter is alive
    pub alive: bool,
    /// Respawn timer (counts down when dead)
    pub respawn_timer: f32,
    /// Spawn position for respawning
    pub spawn_position: Vec2,
    /// Ground level (y coordinate)
    ground_level: f32,
}

impl Critter {
    /// Create a new critter at the given position
    pub fn new(position: Vec2, ground_level: f32) -> Self {
        Self {
            position,
            radius: 3.0,
            state: CritterState::Idle { timer: 1.0 },
            speed: 20.0,
            threat_radius: 30.0,
            nutrition: 20.0,
            alive: true,
            respawn_timer: 0.0,
            spawn_position: position,
            ground_level,
        }
    }

    /// Create a critter with custom parameters
    pub fn with_params(
        position: Vec2,
        radius: f32,
        speed: f32,
        nutrition: f32,
        ground_level: f32,
    ) -> Self {
        Self {
            position,
            radius,
            state: CritterState::Idle { timer: 1.0 },
            speed,
            threat_radius: 30.0,
            nutrition,
            alive: true,
            respawn_timer: 0.0,
            spawn_position: position,
            ground_level,
        }
    }

    /// Update critter state and position
    /// `threats` is a list of positions that the critter should flee from
    pub fn update(&mut self, dt: f32, threats: &[Vec2], world_bounds: (f32, f32)) {
        use rand::Rng;

        // Handle respawning
        if !self.alive {
            self.respawn_timer -= dt;
            if self.respawn_timer <= 0.0 {
                self.respawn();
            }
            return;
        }

        // Check for nearby threats
        let nearest_threat = threats
            .iter()
            .filter_map(|&t| {
                let dist = (t - self.position).length();
                if dist < self.threat_radius {
                    Some((t, dist))
                } else {
                    None
                }
            })
            .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(t, _)| t);

        // Update state based on threats
        if let Some(threat_pos) = nearest_threat {
            self.state = CritterState::Flee {
                from: (threat_pos.x, threat_pos.y),
            };
        }

        // Process current state
        let mut rng = rand::rng();
        match &mut self.state {
            CritterState::Idle { timer } => {
                *timer -= dt;
                if *timer <= 0.0 {
                    // Pick a random wander target
                    let angle = rng.random::<f32>() * std::f32::consts::TAU;
                    let dist = rng.random_range(20.0..50.0);
                    let target = (
                        (self.position.x + angle.cos() * dist).clamp(0.0, world_bounds.0),
                        self.ground_level + self.radius,
                    );
                    self.state = CritterState::Wander { target };
                }
            }
            CritterState::Wander { target } => {
                let target_pos = Vec2::new(target.0, target.1);
                let to_target = target_pos - self.position;
                let dist = to_target.length();

                if dist > 2.0 {
                    let dir = to_target.normalize();
                    self.position += dir * self.speed * dt;
                    // Clamp to bounds
                    self.position.x = self.position.x.clamp(0.0, world_bounds.0);
                    self.position.y = self.ground_level + self.radius;
                } else {
                    // Reached target, go idle
                    self.state = CritterState::Idle {
                        timer: rng.random_range(1.0..3.0),
                    };
                }
            }
            CritterState::Flee { from } => {
                let threat_pos = Vec2::new(from.0, from.1);
                let away = self.position - threat_pos;
                let dist_to_threat = away.length();

                if dist_to_threat > self.threat_radius * 1.5 {
                    // Safe, go idle
                    self.state = CritterState::Idle {
                        timer: rng.random_range(0.5..1.5),
                    };
                } else if away.length() > 0.1 {
                    let dir = away.normalize();
                    self.position += dir * self.speed * 1.5 * dt; // Flee faster
                    // Clamp to bounds
                    self.position.x = self.position.x.clamp(0.0, world_bounds.0);
                    self.position.y = self.ground_level + self.radius;
                }
            }
        }
    }

    /// Mark this critter as eaten
    /// Returns the nutrition value
    pub fn eat(&mut self) -> f32 {
        if self.alive {
            self.alive = false;
            self.respawn_timer = 5.0; // Respawn after 5 seconds
            self.nutrition
        } else {
            0.0
        }
    }

    /// Respawn at original position
    fn respawn(&mut self) {
        self.alive = true;
        self.position = self.spawn_position;
        self.state = CritterState::Idle { timer: 1.0 };
    }

    /// Check if a position overlaps with this critter
    pub fn overlaps(&self, pos: Vec2, other_radius: f32) -> bool {
        self.alive && (self.position - pos).length() < self.radius + other_radius
    }

    /// Get render data for this critter
    pub fn get_render_data(&self) -> CritterRenderData {
        CritterRenderData {
            position: self.position,
            radius: self.radius,
            color: if self.alive {
                // Green color for alive critters
                [50, 200, 50, 255]
            } else {
                // Gray for dead/respawning
                [100, 100, 100, 128]
            },
            state: self.state,
        }
    }
}

/// Render data for a critter
#[derive(Debug, Clone)]
pub struct CritterRenderData {
    pub position: Vec2,
    pub radius: f32,
    pub color: [u8; 4],
    pub state: CritterState,
}

/// Manager for a group of critters
#[derive(Debug, Clone, Default)]
pub struct CritterManager {
    critters: Vec<Critter>,
    ground_level: f32,
    world_bounds: (f32, f32),
}

impl CritterManager {
    /// Create a new critter manager
    pub fn new(ground_level: f32, world_width: f32, world_height: f32) -> Self {
        Self {
            critters: Vec::new(),
            ground_level,
            world_bounds: (world_width, world_height),
        }
    }

    /// Spawn critters at regular intervals
    pub fn spawn_critters(&mut self, count: usize, start_x: f32, spacing: f32) {
        for i in 0..count {
            let x = start_x + i as f32 * spacing;
            let y = self.ground_level + 3.0; // Just above ground
            let critter = Critter::new(Vec2::new(x, y), self.ground_level);
            self.critters.push(critter);
        }
    }

    /// Spawn a single critter at a specific position
    pub fn spawn_at(&mut self, position: Vec2) {
        let critter = Critter::new(position, self.ground_level);
        self.critters.push(critter);
    }

    /// Update all critters
    pub fn update(&mut self, dt: f32, threats: &[Vec2]) {
        for critter in &mut self.critters {
            critter.update(dt, threats, self.world_bounds);
        }
    }

    /// Check if any critter overlaps with the given position
    /// Returns the nutrition gained if eaten
    pub fn try_eat_at(&mut self, pos: Vec2, radius: f32) -> f32 {
        let mut nutrition = 0.0;
        for critter in &mut self.critters {
            if critter.overlaps(pos, radius) {
                nutrition += critter.eat();
            }
        }
        nutrition
    }

    /// Get all critter positions (for threat detection)
    pub fn get_positions(&self) -> Vec<Vec2> {
        self.critters
            .iter()
            .filter(|c| c.alive)
            .map(|c| c.position)
            .collect()
    }

    /// Get render data for all critters
    pub fn get_render_data(&self) -> Vec<CritterRenderData> {
        self.critters.iter().map(|c| c.get_render_data()).collect()
    }

    /// Get number of alive critters
    pub fn alive_count(&self) -> usize {
        self.critters.iter().filter(|c| c.alive).count()
    }

    /// Get total number of critters
    pub fn total_count(&self) -> usize {
        self.critters.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_critter_creation() {
        let critter = Critter::new(Vec2::new(100.0, 30.0), 20.0);
        assert!(critter.alive);
        assert_eq!(critter.position, Vec2::new(100.0, 30.0));
        assert_eq!(critter.radius, 3.0);
    }

    #[test]
    fn test_critter_eating() {
        let mut critter = Critter::new(Vec2::new(100.0, 30.0), 20.0);
        let nutrition = critter.eat();
        assert_eq!(nutrition, 20.0);
        assert!(!critter.alive);
        assert!(critter.respawn_timer > 0.0);
    }

    #[test]
    fn test_critter_overlap() {
        let critter = Critter::new(Vec2::new(100.0, 30.0), 20.0);
        assert!(critter.overlaps(Vec2::new(102.0, 30.0), 3.0)); // Close
        assert!(!critter.overlaps(Vec2::new(110.0, 30.0), 3.0)); // Too far
    }

    #[test]
    fn test_critter_manager() {
        let mut manager = CritterManager::new(20.0, 400.0, 100.0);
        manager.spawn_critters(5, 50.0, 30.0);
        assert_eq!(manager.total_count(), 5);
        assert_eq!(manager.alive_count(), 5);

        // Eat one
        let nutrition = manager.try_eat_at(Vec2::new(50.0, 23.0), 5.0);
        assert!(nutrition > 0.0);
        assert_eq!(manager.alive_count(), 4);
    }

    #[test]
    fn test_critter_flee() {
        let mut critter = Critter::new(Vec2::new(100.0, 30.0), 20.0);
        let threats = vec![Vec2::new(105.0, 30.0)]; // Threat nearby
        critter.update(0.1, &threats, (400.0, 100.0));

        // Should be fleeing
        matches!(critter.state, CritterState::Flee { .. });
    }
}
