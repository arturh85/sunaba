//! Visual-only particle system for effects like flight exhaust, explosions, etc.
//! Particles are rendered directly to the pixel buffer and don't participate in simulation.

use glam::Vec2;
use rand::Rng as _;

/// A single visual particle
#[derive(Debug, Clone)]
pub struct Particle {
    pub position: Vec2,
    pub velocity: Vec2,
    pub color: [u8; 4],
    pub lifetime: f32,
    pub max_lifetime: f32,
}

impl Particle {
    pub fn new(position: Vec2, velocity: Vec2, color: [u8; 4], lifetime: f32) -> Self {
        Self {
            position,
            velocity,
            color,
            lifetime,
            max_lifetime: lifetime,
        }
    }

    /// Get the current alpha based on remaining lifetime (fades out)
    pub fn alpha(&self) -> u8 {
        ((self.lifetime / self.max_lifetime) * 255.0) as u8
    }

    /// Check if particle is still alive
    pub fn is_alive(&self) -> bool {
        self.lifetime > 0.0
    }
}

/// Manages a collection of visual particles
pub struct ParticleSystem {
    particles: Vec<Particle>,
    max_particles: usize,
}

impl Default for ParticleSystem {
    fn default() -> Self {
        Self::new()
    }
}

impl ParticleSystem {
    const DEFAULT_MAX_PARTICLES: usize = 500;

    pub fn new() -> Self {
        Self {
            particles: Vec::with_capacity(Self::DEFAULT_MAX_PARTICLES),
            max_particles: Self::DEFAULT_MAX_PARTICLES,
        }
    }

    /// Update all particles: move them, decrease lifetime, remove dead ones
    pub fn update(&mut self, dt: f32) {
        for particle in &mut self.particles {
            particle.position += particle.velocity * dt;
            particle.lifetime -= dt;
        }

        // Remove dead particles
        self.particles.retain(|p| p.is_alive());
    }

    /// Spawn a single particle
    pub fn spawn(&mut self, position: Vec2, velocity: Vec2, color: [u8; 4], lifetime: f32) {
        // If at capacity, remove oldest particle
        if self.particles.len() >= self.max_particles {
            self.particles.remove(0);
        }

        self.particles
            .push(Particle::new(position, velocity, color, lifetime));
    }

    /// Spawn a burst of flight exhaust particles (Noita-style)
    pub fn spawn_flight_burst(&mut self, player_pos: Vec2, player_height: f32) {
        let mut rng = rand::rng();
        let foot_y = player_pos.y - player_height / 2.0;

        // Spawn 2-4 particles per frame
        let count = rng.random_range(2..=4);
        for _ in 0..count {
            let offset_x = rng.random_range(-3.0..3.0);
            let position = Vec2::new(player_pos.x + offset_x, foot_y - 1.0);

            // Velocity: mostly down with some horizontal spread (exhaust effect)
            let velocity = Vec2::new(
                rng.random_range(-20.0..20.0),
                rng.random_range(-80.0..-40.0), // Downward
            );

            // Warm color palette (yellow → orange → white-yellow)
            let colors: [[u8; 4]; 3] = [
                [255, 220, 100, 255], // Yellow
                [255, 180, 80, 255],  // Orange
                [255, 255, 200, 255], // White-yellow
            ];
            let color = colors[rng.random_range(0..colors.len())];

            let lifetime = rng.random_range(0.15..0.35);
            self.spawn(position, velocity, color, lifetime);
        }
    }

    /// Spawn impact particles when placing a material
    pub fn spawn_impact_burst(&mut self, position: Vec2, color: [u8; 4]) {
        let mut rng = rand::rng();

        // Spawn 3-6 particles in a radial burst
        let count = rng.random_range(3..=6);
        for _ in 0..count {
            let angle = rng.random_range(0.0..std::f32::consts::TAU);
            let speed = rng.random_range(30.0..80.0);
            let velocity = Vec2::new(angle.cos() * speed, angle.sin() * speed);

            // Slightly vary the color
            let varied_color = [
                (color[0] as i16 + rng.random_range(-20..20)).clamp(0, 255) as u8,
                (color[1] as i16 + rng.random_range(-20..20)).clamp(0, 255) as u8,
                (color[2] as i16 + rng.random_range(-20..20)).clamp(0, 255) as u8,
                color[3],
            ];

            let lifetime = rng.random_range(0.15..0.3);
            self.spawn(position, velocity, varied_color, lifetime);
        }
    }

    /// Spawn splash particles for liquid placement
    pub fn spawn_liquid_splash(&mut self, position: Vec2, color: [u8; 4]) {
        let mut rng = rand::rng();

        // Spawn 4-8 droplets arcing upward then falling
        let count = rng.random_range(4..=8);
        for _ in 0..count {
            let angle: f32 = rng.random_range(-2.5..-0.6); // Mostly upward arc
            let speed = rng.random_range(40.0..100.0);
            let velocity = Vec2::new(
                rng.random_range(-40.0..40.0),
                speed * angle.sin(), // Upward
            );

            // Make slightly translucent
            let splash_color = [color[0], color[1], color[2], 200];

            let lifetime = rng.random_range(0.2..0.5);
            self.spawn(position, velocity, splash_color, lifetime);
        }
    }

    /// Spawn dust cloud particles for mining/digging
    pub fn spawn_dust_cloud(&mut self, position: Vec2, color: [u8; 4]) {
        let mut rng = rand::rng();

        // Spawn 5-10 dust particles drifting upward
        let count = rng.random_range(5..=10);
        for _ in 0..count {
            let offset = Vec2::new(rng.random_range(-4.0..4.0), rng.random_range(-4.0..4.0));
            let pos = position + offset;

            // Slow, upward drift with some horizontal spread
            let velocity = Vec2::new(
                rng.random_range(-15.0..15.0),
                rng.random_range(-30.0..-10.0), // Upward (negative Y)
            );

            // Dusty brown-gray color mixed with material color
            let dust_color = [
                ((color[0] as u16 + 120) / 2).min(255) as u8,
                ((color[1] as u16 + 110) / 2).min(255) as u8,
                ((color[2] as u16 + 100) / 2).min(255) as u8,
                180,
            ];

            let lifetime = rng.random_range(0.3..0.6);
            self.spawn(pos, velocity, dust_color, lifetime);
        }
    }

    /// Spawn spark particles for metal/stone impacts
    pub fn spawn_sparks(&mut self, position: Vec2) {
        let mut rng = rand::rng();

        // Spawn 2-5 bright sparks
        let count = rng.random_range(2..=5);
        for _ in 0..count {
            let angle = rng.random_range(0.0..std::f32::consts::TAU);
            let speed = rng.random_range(80.0..150.0);
            let velocity = Vec2::new(angle.cos() * speed, angle.sin() * speed);

            // Bright yellow-white sparks
            let colors: [[u8; 4]; 3] = [
                [255, 255, 200, 255], // White-yellow
                [255, 220, 100, 255], // Yellow
                [255, 200, 50, 255],  // Orange-yellow
            ];
            let color = colors[rng.random_range(0..colors.len())];

            let lifetime = rng.random_range(0.1..0.25);
            self.spawn(position, velocity, color, lifetime);
        }
    }

    /// Iterate over all active particles
    pub fn iter(&self) -> impl Iterator<Item = &Particle> {
        self.particles.iter()
    }

    /// Get particle count (for debug stats)
    pub fn count(&self) -> usize {
        self.particles.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_particle_lifecycle() {
        let mut particle = Particle::new(
            Vec2::new(0.0, 0.0),
            Vec2::new(10.0, -5.0),
            [255, 255, 255, 255],
            1.0,
        );

        assert!(particle.is_alive());
        assert_eq!(particle.alpha(), 255);

        // Simulate half lifetime
        particle.lifetime = 0.5;
        assert!(particle.is_alive());
        assert_eq!(particle.alpha(), 127);

        // Simulate death
        particle.lifetime = 0.0;
        assert!(!particle.is_alive());
        assert_eq!(particle.alpha(), 0);
    }

    #[test]
    fn test_particle_system_update() {
        let mut system = ParticleSystem::new();

        system.spawn(Vec2::ZERO, Vec2::new(10.0, 0.0), [255, 255, 255, 255], 0.5);
        assert_eq!(system.count(), 1);

        // Update for 0.25s
        system.update(0.25);
        assert_eq!(system.count(), 1);

        // Update for another 0.3s (should die)
        system.update(0.3);
        assert_eq!(system.count(), 0);
    }

    #[test]
    fn test_particle_system_max_capacity() {
        let mut system = ParticleSystem::new();

        // Spawn more than max
        for i in 0..600 {
            system.spawn(
                Vec2::new(i as f32, 0.0),
                Vec2::ZERO,
                [255, 255, 255, 255],
                1.0,
            );
        }

        assert!(system.count() <= ParticleSystem::DEFAULT_MAX_PARTICLES);
    }
}
