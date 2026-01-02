//! Sensory systems for creatures
//!
//! Implements raycasting vision, material detection, and chemical gradients.

use glam::Vec2;
use serde::{Deserialize, Serialize};

/// Raycast vision result
#[derive(Debug, Clone)]
pub struct RaycastHit {
    pub distance: f32, // Normalized by max_distance
    pub material_id: u16,
    pub temperature: f32,
    pub light_level: u8,
}

/// Chemical gradient detection
#[derive(Debug, Clone)]
pub struct ChemicalGradient {
    pub food: f32,   // 0.0 - 1.0
    pub danger: f32, // 0.0 - 1.0
    pub mate: f32,   // 0.0 - 1.0
}

/// Complete sensory input
#[derive(Debug, Clone)]
pub struct SensoryInput {
    pub raycasts: Vec<RaycastHit>,   // Typically 8 directions
    pub contact_materials: Vec<u16>, // Materials in contact
    pub gradients: ChemicalGradient,
    pub nearest_food: Option<Vec2>,
    pub nearest_threat: Option<Vec2>,
    /// Normalized direction vector pointing toward nearest food (long-range compass)
    pub food_direction: Option<Vec2>,
    /// Distance to nearest food (normalized 0-1 based on compass_radius)
    pub food_distance: f32,
}

impl SensoryInput {
    /// Gather all sensory input for creature at position
    pub fn gather(world: &impl crate::WorldAccess, position: Vec2, config: &SensorConfig) -> Self {
        // Raycast vision in multiple directions
        let raycasts = raycast_vision(
            world,
            position,
            config.num_raycasts,
            config.raycast_distance,
        );

        // Detect nearby food and threats
        let nearest_food = detect_nearby_food(world, position, config.food_detection_radius);
        let nearest_threat = detect_nearby_threats(world, position, config.threat_detection_radius);

        // Long-range food compass (directional sensing)
        let (food_direction, food_distance) =
            detect_food_direction(world, position, config.food_compass_radius);

        // Calculate chemical gradients
        let gradients = calculate_gradients(
            world,
            position,
            config
                .food_detection_radius
                .max(config.threat_detection_radius),
        );

        // For now, contact materials is empty (would need physics integration)
        let contact_materials = Vec::new();

        Self {
            raycasts,
            contact_materials,
            gradients,
            nearest_food,
            nearest_threat,
            food_direction,
            food_distance,
        }
    }

    /// Gather sensory input with cached food positions (optimized for training)
    ///
    /// Uses pre-computed food positions instead of scanning all pixels,
    /// reducing food detection from O(r²) to O(n_food).
    pub fn gather_with_cache(
        world: &impl crate::WorldAccess,
        position: Vec2,
        config: &SensorConfig,
        food_positions: &[Vec2],
    ) -> Self {
        // Raycast vision in multiple directions
        let raycasts = raycast_vision(
            world,
            position,
            config.num_raycasts,
            config.raycast_distance,
        );

        // Detect nearby threats (still need to scan, but small radius)
        let nearest_threat = detect_nearby_threats(world, position, config.threat_detection_radius);

        // Use cached food positions for food direction (O(n) instead of O(r²))
        let (food_direction, food_distance) =
            detect_food_direction_cached(position, food_positions, config.food_compass_radius);

        // Find nearest food from cached positions
        let nearest_food =
            find_nearest_food_from_cache(position, food_positions, config.food_detection_radius);

        // Simple gradients based on food distance
        let food_gradient = if food_distance < 1.0 {
            1.0 - food_distance
        } else {
            0.0
        };
        let gradients = ChemicalGradient {
            food: food_gradient,
            danger: 0.0, // Would need threat scan
            mate: 0.0,
        };

        let contact_materials = Vec::new();

        Self {
            raycasts,
            contact_materials,
            gradients,
            nearest_food,
            nearest_threat,
            food_direction,
            food_distance,
        }
    }
}

/// Sensor configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SensorConfig {
    pub num_raycasts: usize,
    pub raycast_distance: f32,
    pub food_detection_radius: f32,
    pub threat_detection_radius: f32,
    /// Long-range food compass radius (for directional sensing)
    pub food_compass_radius: f32,
}

impl Default for SensorConfig {
    fn default() -> Self {
        Self {
            num_raycasts: 8,
            raycast_distance: 50.0,
            food_detection_radius: 30.0,
            threat_detection_radius: 40.0,
            food_compass_radius: 150.0, // Reduced from 500 for performance (used as fallback)
        }
    }
}

/// Raycast through pixel world using DDA algorithm
pub fn raycast_vision(
    world: &impl crate::WorldAccess,
    origin: Vec2,
    num_rays: usize,
    max_distance: f32,
) -> Vec<RaycastHit> {
    use std::f32::consts::PI;

    let mut hits = Vec::with_capacity(num_rays);

    for i in 0..num_rays {
        // Calculate ray direction
        let angle = (i as f32 / num_rays as f32) * 2.0 * PI;
        let dir = Vec2::new(angle.cos(), angle.sin());

        // DDA raycasting
        let hit = raycast_dda(world, origin, dir, max_distance);
        hits.push(hit);
    }

    hits
}

/// DDA (Digital Differential Analyzer) raycasting
fn raycast_dda(
    world: &impl crate::WorldAccess,
    origin: Vec2,
    direction: Vec2,
    max_distance: f32,
) -> RaycastHit {
    let mut current_pos = origin;
    let step_size = 1.0; // Step one pixel at a time

    let mut distance = 0.0;

    while distance < max_distance {
        // Step forward
        current_pos += direction * step_size;
        distance += step_size;

        // Check pixel at current position
        let pixel_x = current_pos.x.round() as i32;
        let pixel_y = current_pos.y.round() as i32;

        if let Some(pixel) = world.get_pixel(pixel_x, pixel_y) {
            let material_id = pixel.material_id;

            // Hit solid material (not air)
            if material_id != 0 {
                // Get additional information about the hit
                let temperature = world.get_temperature_at_pixel(pixel_x, pixel_y);

                let light_level = world.get_light_at(pixel_x, pixel_y).unwrap_or(0);

                return RaycastHit {
                    distance: distance / max_distance, // Normalize
                    material_id,
                    temperature,
                    light_level,
                };
            }
        } else {
            // Hit world boundary
            break;
        }
    }

    // No hit - return air at max distance
    RaycastHit {
        distance: 1.0,  // Max distance (normalized)
        material_id: 0, // Air
        temperature: 20.0,
        light_level: 15, // Max light (outdoor)
    }
}

/// Detect nearby edible materials
pub fn detect_nearby_food(
    world: &impl crate::WorldAccess,
    position: Vec2,
    radius: f32,
) -> Option<Vec2> {
    use sunaba_simulation::MaterialTag;

    let mut nearest_food: Option<(Vec2, f32)> = None;
    let radius_sq = radius * radius;

    // Search in a square around the position
    let min_x = (position.x - radius).floor() as i32;
    let max_x = (position.x + radius).ceil() as i32;
    let min_y = (position.y - radius).floor() as i32;
    let max_y = (position.y + radius).ceil() as i32;

    for y in min_y..=max_y {
        for x in min_x..=max_x {
            if let Some(pixel) = world.get_pixel(x, y) {
                let material_id = pixel.material_id;
                let material = world.materials().get(material_id);

                // Check if material is edible
                if material.tags.contains(&MaterialTag::Edible) {
                    let pixel_pos = Vec2::new(x as f32, y as f32);
                    let dist_sq = position.distance_squared(pixel_pos);

                    if dist_sq <= radius_sq {
                        match nearest_food {
                            None => nearest_food = Some((pixel_pos, dist_sq)),
                            Some((_, current_dist_sq)) => {
                                if dist_sq < current_dist_sq {
                                    nearest_food = Some((pixel_pos, dist_sq));
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    nearest_food.map(|(pos, _)| pos)
}

/// Detect nearby threats (fire, lava, acid)
pub fn detect_nearby_threats(
    world: &impl crate::WorldAccess,
    position: Vec2,
    radius: f32,
) -> Option<Vec2> {
    let mut nearest_threat: Option<(Vec2, f32)> = None;
    let radius_sq = radius * radius;

    // Search in a square around the position
    let min_x = (position.x - radius).floor() as i32;
    let max_x = (position.x + radius).ceil() as i32;
    let min_y = (position.y - radius).floor() as i32;
    let max_y = (position.y + radius).ceil() as i32;

    // Dangerous material IDs (based on materials.rs)
    const FIRE: u16 = 6;
    const LAVA: u16 = 9;
    const ACID: u16 = 11;

    for y in min_y..=max_y {
        for x in min_x..=max_x {
            if let Some(pixel) = world.get_pixel(x, y) {
                let material_id = pixel.material_id;

                // Check if material is dangerous
                if material_id == FIRE || material_id == LAVA || material_id == ACID {
                    let pixel_pos = Vec2::new(x as f32, y as f32);
                    let dist_sq = position.distance_squared(pixel_pos);

                    if dist_sq <= radius_sq {
                        match nearest_threat {
                            None => nearest_threat = Some((pixel_pos, dist_sq)),
                            Some((_, current_dist_sq)) => {
                                if dist_sq < current_dist_sq {
                                    nearest_threat = Some((pixel_pos, dist_sq));
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    nearest_threat.map(|(pos, _)| pos)
}

/// Detect direction to nearest food (long-range compass)
/// Returns (normalized_direction, normalized_distance)
pub fn detect_food_direction(
    world: &impl crate::WorldAccess,
    position: Vec2,
    radius: f32,
) -> (Option<Vec2>, f32) {
    use sunaba_simulation::MaterialTag;

    let mut nearest_food: Option<(Vec2, f32)> = None;
    let radius_sq = radius * radius;

    // Search in a square around the position
    let min_x = (position.x - radius).floor() as i32;
    let max_x = (position.x + radius).ceil() as i32;
    let min_y = (position.y - radius).floor() as i32;
    let max_y = (position.y + radius).ceil() as i32;

    for y in min_y..=max_y {
        for x in min_x..=max_x {
            if let Some(pixel) = world.get_pixel(x, y) {
                let material_id = pixel.material_id;
                let material = world.materials().get(material_id);

                // Check if material is edible
                if material.tags.contains(&MaterialTag::Edible) {
                    let pixel_pos = Vec2::new(x as f32, y as f32);
                    let dist_sq = position.distance_squared(pixel_pos);

                    if dist_sq <= radius_sq {
                        match nearest_food {
                            None => nearest_food = Some((pixel_pos, dist_sq)),
                            Some((_, current_dist_sq)) => {
                                if dist_sq < current_dist_sq {
                                    nearest_food = Some((pixel_pos, dist_sq));
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    match nearest_food {
        Some((food_pos, dist_sq)) => {
            let direction = food_pos - position;
            let distance = dist_sq.sqrt();
            let normalized_dir = if distance > 0.001 {
                direction / distance
            } else {
                Vec2::ZERO
            };
            let normalized_dist = (distance / radius).min(1.0);
            (Some(normalized_dir), normalized_dist)
        }
        None => (None, 1.0), // No food found, max distance
    }
}

/// Detect direction to nearest food using cached positions (O(n) instead of O(r²))
/// Returns (normalized_direction, normalized_distance)
pub fn detect_food_direction_cached(
    position: Vec2,
    food_positions: &[Vec2],
    max_distance: f32,
) -> (Option<Vec2>, f32) {
    let mut nearest: Option<(Vec2, f32)> = None;

    for &food_pos in food_positions {
        let dist_sq = position.distance_squared(food_pos);
        match nearest {
            None => nearest = Some((food_pos, dist_sq)),
            Some((_, curr_dist_sq)) if dist_sq < curr_dist_sq => {
                nearest = Some((food_pos, dist_sq));
            }
            _ => {}
        }
    }

    match nearest {
        Some((food_pos, dist_sq)) => {
            let distance = dist_sq.sqrt();
            let direction = food_pos - position;
            let normalized_dir = if distance > 0.001 {
                direction / distance
            } else {
                Vec2::ZERO
            };
            let normalized_dist = (distance / max_distance).min(1.0);
            (Some(normalized_dir), normalized_dist)
        }
        None => (None, 1.0),
    }
}

/// Find nearest food from cached positions within radius
pub fn find_nearest_food_from_cache(
    position: Vec2,
    food_positions: &[Vec2],
    radius: f32,
) -> Option<Vec2> {
    let radius_sq = radius * radius;
    let mut nearest: Option<(Vec2, f32)> = None;

    for &food_pos in food_positions {
        let dist_sq = position.distance_squared(food_pos);
        if dist_sq <= radius_sq {
            match nearest {
                None => nearest = Some((food_pos, dist_sq)),
                Some((_, curr_dist_sq)) if dist_sq < curr_dist_sq => {
                    nearest = Some((food_pos, dist_sq));
                }
                _ => {}
            }
        }
    }

    nearest.map(|(pos, _)| pos)
}

/// Calculate chemical gradients (scent following)
pub fn calculate_gradients(
    world: &impl crate::WorldAccess,
    position: Vec2,
    radius: f32,
) -> ChemicalGradient {
    use sunaba_simulation::MaterialTag;

    let mut food_count = 0;
    let mut danger_count = 0;
    let radius_sq = radius * radius;

    // Search in a square around the position
    let min_x = (position.x - radius).floor() as i32;
    let max_x = (position.x + radius).ceil() as i32;
    let min_y = (position.y - radius).floor() as i32;
    let max_y = (position.y + radius).ceil() as i32;

    const FIRE: u16 = 6;
    const LAVA: u16 = 9;
    const ACID: u16 = 11;

    for y in min_y..=max_y {
        for x in min_x..=max_x {
            if let Some(pixel) = world.get_pixel(x, y) {
                let pixel_pos = Vec2::new(x as f32, y as f32);
                let dist_sq = position.distance_squared(pixel_pos);

                if dist_sq <= radius_sq {
                    let material_id = pixel.material_id;
                    let material = world.materials().get(material_id);

                    // Count food
                    if material.tags.contains(&MaterialTag::Edible) {
                        food_count += 1;
                    }

                    // Count dangers
                    if material_id == FIRE || material_id == LAVA || material_id == ACID {
                        danger_count += 1;
                    }
                }
            }
        }
    }

    // Normalize to 0-1 range (arbitrarily using 100 pixels as saturation point)
    let food_gradient = (food_count as f32 / 100.0).min(1.0);
    let danger_gradient = (danger_count as f32 / 100.0).min(1.0);

    ChemicalGradient {
        food: food_gradient,
        danger: danger_gradient,
        mate: 0.0, // Mate detection not implemented yet
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sensor_config_defaults() {
        let config = SensorConfig::default();
        assert_eq!(config.num_raycasts, 8);
        assert_eq!(config.raycast_distance, 50.0);
        assert_eq!(config.food_detection_radius, 30.0);
        assert_eq!(config.threat_detection_radius, 40.0);
        assert_eq!(config.food_compass_radius, 150.0);
    }

    #[test]
    fn test_raycast_hit_creation() {
        let hit = RaycastHit {
            distance: 0.5,
            material_id: 1,
            temperature: 25.0,
            light_level: 10,
        };

        assert_eq!(hit.distance, 0.5);
        assert_eq!(hit.material_id, 1);
        assert_eq!(hit.temperature, 25.0);
        assert_eq!(hit.light_level, 10);
    }

    #[test]
    fn test_chemical_gradient_creation() {
        let gradient = ChemicalGradient {
            food: 0.5,
            danger: 0.3,
            mate: 0.0,
        };

        assert_eq!(gradient.food, 0.5);
        assert_eq!(gradient.danger, 0.3);
        assert_eq!(gradient.mate, 0.0);
    }

    // The following tests require World::new() which is in sunaba-core.
    // These tests are moved to sunaba-core as integration tests.
    // See sunaba-core/tests/creature_sensors_test.rs

    #[test]
    #[ignore] // Requires concrete World implementation from sunaba-core
    fn test_raycast_vision_creates_correct_number() {
        // This test requires World::new() from sunaba-core
    }

    #[test]
    #[ignore] // Requires concrete World implementation from sunaba-core
    fn test_raycast_dda_air_returns_max_distance() {
        // This test requires World::new() from sunaba-core
    }

    #[test]
    #[ignore] // Requires concrete World implementation from sunaba-core
    fn test_detect_nearby_food_none_when_empty() {
        // This test requires World::new() from sunaba-core
    }

    #[test]
    #[ignore] // Requires concrete World implementation from sunaba-core
    fn test_detect_nearby_threats_none_when_safe() {
        // This test requires World::new() from sunaba-core
    }

    #[test]
    #[ignore] // Requires concrete World implementation from sunaba-core
    fn test_calculate_gradients_returns_normalized() {
        // This test requires World::new() from sunaba-core
    }

    #[test]
    #[ignore] // Requires concrete World implementation from sunaba-core
    fn test_sensory_input_gather_complete() {
        // This test requires World::new() from sunaba-core
    }
}
