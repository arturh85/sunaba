//! Sensory systems for creatures
//!
//! Implements raycasting vision, material detection, and chemical gradients.

use glam::Vec2;
use serde::{Deserialize, Serialize};
use sunaba_simulation::MaterialId;

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

/// Terrain-aware sensory input for adaptive locomotion
#[derive(Debug, Clone)]
pub struct TerrainSensoryInput {
    /// Ground slope: -1.0 (steep downhill) to 1.0 (steep uphill)
    pub ground_slope: f32,
    /// Vertical clearance: 0.0 (blocked) to 1.0 (fully clear)
    pub vertical_clearance: f32,
    /// Gap distance: 0.0 (immediate) to 1.0 (far/none), normalized
    pub gap_distance: f32,
    /// Gap width: 0.0 (no gap) to 1.0 (unjumpable), normalized
    pub gap_width: f32,
    /// Material ID underfoot (0 = air/no ground)
    pub surface_material: u16,
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
    /// Distance ahead to sample for slope detection (pixels)
    pub slope_sense_distance: f32,
    /// Maximum height to scan for clearance detection (pixels)
    pub clearance_sense_height: f32,
    /// Maximum lookahead distance for gap detection (pixels)
    pub gap_sense_distance: f32,
    /// Gap width threshold for unjumpable classification (pixels)
    pub max_gap_width: f32,
}

impl Default for SensorConfig {
    fn default() -> Self {
        Self {
            num_raycasts: 8,
            raycast_distance: 50.0,
            food_detection_radius: 30.0,
            threat_detection_radius: 40.0,
            food_compass_radius: 150.0, // Reduced from 500 for performance (used as fallback)
            slope_sense_distance: 5.0,
            clearance_sense_height: 25.0,
            gap_sense_distance: 40.0,
            max_gap_width: 30.0,
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

/// Bresenham raycasting - exact pixel traversal for creature sensors
fn raycast_dda(
    world: &impl crate::WorldAccess,
    origin: Vec2,
    direction: Vec2,
    max_distance: f32,
) -> RaycastHit {
    use bresenham::Bresenham;

    // Calculate start and end points for Bresenham (uses isize)
    let from_i = (origin.x.round() as isize, origin.y.round() as isize);
    let to = origin + direction * max_distance;
    let to_i = (to.x.round() as isize, to.y.round() as isize);

    // Calculate max distance for normalization (in pixels)
    let max_dist = ((to_i.0 - from_i.0).pow(2) + (to_i.1 - from_i.1).pow(2)) as f32;
    let max_dist_sqrt = max_dist.sqrt().max(1.0); // Avoid division by zero

    // Use Bresenham line algorithm for exact pixel traversal
    let mut pixel_count = 0.0;
    for (x, y) in Bresenham::new(from_i, to_i) {
        pixel_count += 1.0;
        let pixel_x = x as i32;
        let pixel_y = y as i32;

        if let Some(pixel) = world.get_pixel(pixel_x, pixel_y) {
            let material_id = pixel.material_id;

            // Hit solid material (not air)
            if material_id != 0 {
                // Get additional information about the hit
                let temperature = world.get_temperature_at_pixel(pixel_x, pixel_y);
                let light_level = world.get_light_at(pixel_x, pixel_y).unwrap_or(0);

                return RaycastHit {
                    distance: (pixel_count / max_dist_sqrt).min(1.0), // Normalize to 0-1
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

    for y in min_y..=max_y {
        for x in min_x..=max_x {
            if let Some(pixel) = world.get_pixel(x, y) {
                let material_id = pixel.material_id;

                // Check if material is dangerous
                if material_id == MaterialId::FIRE
                    || material_id == MaterialId::LAVA
                    || material_id == MaterialId::ACID
                {
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

/// Helper function to find ground below a position
/// Returns the Y coordinate of the first solid pixel, or None if no ground found
fn find_ground_below(
    world: &impl crate::WorldAccess,
    position: Vec2,
    max_depth: i32,
) -> Option<f32> {
    let px = position.x as i32;
    let start_y = position.y as i32;

    for dy in 0..max_depth {
        let check_y = start_y - dy; // Scan downward
        if let Some(pixel) = world.get_pixel(px, check_y)
            && pixel.material_id != 0
        {
            return Some(check_y as f32);
        }
    }
    None
}

/// Sense ground slope for gait adaptation
/// Returns -1.0 (downhill) to 1.0 (uphill), or 0.0 if no ground/flat
pub fn sense_ground_slope(
    world: &impl crate::WorldAccess,
    position: Vec2,
    facing_direction: f32,
    config: &SensorConfig,
) -> f32 {
    let sense_distance = config.slope_sense_distance;

    // Find ground at current position
    let height_current = find_ground_below(world, position, 20);

    // Find ground ahead
    let ahead_pos = position + Vec2::new(facing_direction * sense_distance, 0.0);
    let height_ahead = find_ground_below(world, ahead_pos, 20);

    match (height_current, height_ahead) {
        (Some(h_curr), Some(h_ahead)) => {
            let delta_y = h_ahead - h_curr; // Positive = uphill (ground ahead is higher)
            (delta_y / sense_distance).clamp(-1.0, 1.0)
        }
        _ => 0.0, // No ground = treat as flat
    }
}

/// Sense vertical clearance for jump height modulation
/// Returns 0.0 (blocked) to 1.0 (fully clear)
pub fn sense_vertical_clearance(
    world: &impl crate::WorldAccess,
    position: Vec2,
    config: &SensorConfig,
) -> f32 {
    let max_height = config.clearance_sense_height;
    let px = position.x as i32;
    let start_y = position.y as i32;

    let mut air_count = 0.0;
    for dy in 1..=(max_height as i32) {
        let check_y = start_y + dy; // Scan upward
        if let Some(pixel) = world.get_pixel(px, check_y) {
            if pixel.material_id == 0 {
                air_count += 1.0;
            } else {
                break; // Hit ceiling
            }
        } else {
            // Outside world bounds = open sky
            air_count = max_height;
            break;
        }
    }

    (air_count / max_height).min(1.0)
}

/// Sense gap distance and width for navigation
/// Returns (gap_distance, gap_width), both normalized 0.0-1.0
/// No gap: (1.0, 0.0), Gap too wide: (distance, 1.0)
pub fn sense_gap_info(
    world: &impl crate::WorldAccess,
    position: Vec2,
    facing_direction: f32,
    config: &SensorConfig,
) -> (f32, f32) {
    let max_distance = config.gap_sense_distance;
    let max_width = config.max_gap_width;
    let px_start = position.x as i32;
    let py = (position.y - 5.0) as i32; // Scan at foot level
    let dir = facing_direction.signum() as i32;

    let mut gap_start: Option<i32> = None;
    let mut on_solid = world.is_solid_at(px_start, py);

    for step in 1..=(max_distance as i32) {
        let check_x = px_start + (dir * step);
        let is_solid = world.is_solid_at(check_x, py);

        // Detect gap start (solid → air)
        if on_solid && !is_solid && gap_start.is_none() {
            gap_start = Some(step);
        }

        // Detect gap end (air → solid)
        if !on_solid
            && is_solid
            && let Some(gap_start_val) = gap_start
        {
            let gap_distance = (gap_start_val as f32 / max_distance).min(1.0);
            let gap_width = ((step - gap_start_val) as f32 / max_width).min(1.0);
            return (gap_distance, gap_width);
        }

        on_solid = is_solid;
    }

    // No gap found, or gap extends beyond range
    match gap_start {
        Some(start) => {
            let gap_distance = (start as f32 / max_distance).min(1.0);
            (gap_distance, 1.0) // Gap too wide (unjumpable)
        }
        None => (1.0, 0.0), // No gap detected
    }
}

/// Sense surface material underfoot
/// Returns material ID (0 = air/no ground)
pub fn sense_surface_material(
    world: &impl crate::WorldAccess,
    position: Vec2,
    body_part_radius: f32,
) -> u16 {
    let px = position.x as i32;
    let start_y = position.y as i32;
    let bottom_y = start_y - (body_part_radius as i32);

    // Check downward from bottom of body part
    for dy in 0..15 {
        let check_y = bottom_y - dy;
        if let Some(pixel) = world.get_pixel(px, check_y)
            && pixel.material_id != 0
        {
            return pixel.material_id;
        }
    }

    0 // No ground (air)
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
        assert_eq!(config.slope_sense_distance, 5.0);
        assert_eq!(config.clearance_sense_height, 25.0);
        assert_eq!(config.gap_sense_distance, 40.0);
        assert_eq!(config.max_gap_width, 30.0);
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
}
