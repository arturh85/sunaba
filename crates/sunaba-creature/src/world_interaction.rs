//! Creature-world interaction
//!
//! Handles creature interactions with the pixel world: eating, mining, building, damage.

use glam::Vec2;

use sunaba_simulation::{Pixel, pixel_flags};

/// Consume edible material at position
/// Returns nutritional value gained
pub fn consume_edible_material(
    world: &mut impl crate::WorldMutAccess,
    position: Vec2,
    _creature_id: &crate::EntityId,
) -> Option<f32> {
    let pixel_x = position.x.round() as i32;
    let pixel_y = position.y.round() as i32;

    if let Some(pixel) = world.get_pixel(pixel_x, pixel_y) {
        let material_id = pixel.material_id;
        let material = world.materials().get(material_id);

        // Check if edible
        if let Some(nutrition) = material.nutritional_value
            && nutrition > 0.0
        {
            // Remove the pixel (eat it)
            world.set_pixel(pixel_x, pixel_y, 0); // Set to air
            return Some(nutrition);
        }
    }

    None
}

/// Mine world pixel at position
/// Returns the material_id of the mined pixel, or None if can't mine
pub fn mine_world_pixel(
    world: &mut impl crate::WorldMutAccess,
    position: Vec2,
    _creature_id: &crate::EntityId,
) -> Option<u16> {
    let pixel_x = position.x.round() as i32;
    let pixel_y = position.y.round() as i32;

    if let Some(pixel) = world.get_pixel(pixel_x, pixel_y) {
        let material_id = pixel.material_id;

        // Can't mine air or bedrock
        if material_id == 0 || material_id == 14 {
            return None;
        }

        // Mine the pixel (remove it)
        world.set_pixel(pixel_x, pixel_y, 0); // Set to air
        return Some(material_id);
    }

    None
}

/// Place material in world
/// Returns true if placement succeeded
/// Sets PLAYER_PLACED flag so structural integrity applies to creature-built structures
pub fn place_material(
    world: &mut impl crate::WorldMutAccess,
    position: Vec2,
    material_id: u16,
    _creature_id: &crate::EntityId,
) -> bool {
    let pixel_x = position.x.round() as i32;
    let pixel_y = position.y.round() as i32;

    // Check if position is valid and currently air
    if let Some(pixel) = world.get_pixel(pixel_x, pixel_y)
        && pixel.material_id == 0
    {
        // Only place in air - set PLAYER_PLACED flag for structural integrity
        let mut new_pixel = Pixel::new(material_id);
        new_pixel.flags |= pixel_flags::PLAYER_PLACED;
        world.set_pixel_full(pixel_x, pixel_y, new_pixel);
        return true;
    }

    false
}

/// Apply environmental damage to creature
/// Returns true if damage was applied
pub fn apply_environmental_damage(
    world: &impl crate::WorldAccess,
    position: Vec2,
    health: &mut crate::Health,
) -> bool {
    let pixel_x = position.x.round() as i32;
    let pixel_y = position.y.round() as i32;

    if let Some(pixel) = world.get_pixel(pixel_x, pixel_y) {
        let material_id = pixel.material_id;

        // Dangerous materials (fire, lava, acid)
        const FIRE: u16 = 6;
        const LAVA: u16 = 9;
        const ACID: u16 = 11;

        let damage = match material_id {
            FIRE => 5.0,  // Fire damage per frame
            LAVA => 20.0, // High lava damage
            ACID => 10.0, // Acid damage
            _ => 0.0,
        };

        if damage > 0.0 {
            health.take_damage(damage);
            return true;
        }

        // Temperature damage
        let temperature = world.get_temperature_at_pixel(pixel_x, pixel_y);
        if temperature > 100.0 {
            // Extreme heat
            health.take_damage((temperature - 100.0) * 0.1);
            return true;
        } else if temperature < -10.0 {
            // Extreme cold
            health.take_damage((-10.0 - temperature) * 0.05);
            return true;
        }
    }

    false
}

/// Check if material is edible
pub fn is_edible(world: &impl crate::WorldAccess, material_id: u16) -> bool {
    let material = world.materials().get(material_id);
    material
        .nutritional_value
        .map(|nv| nv > 0.0)
        .unwrap_or(false)
}

/// Get nutritional value of material
pub fn get_nutritional_value(world: &impl crate::WorldAccess, material_id: u16) -> f32 {
    let material = world.materials().get(material_id);
    material.nutritional_value.unwrap_or(0.0)
}

#[cfg(test)]
mod tests {
    // All tests in this module require World::new() from sunaba-core.
    // These tests are moved to sunaba-core as integration tests.

    #[test]
    #[ignore] // Requires concrete World implementation from sunaba-core
    fn test_is_edible() {
        // This test requires World::new() from sunaba-core
    }

    #[test]
    #[ignore] // Requires concrete World implementation from sunaba-core
    fn test_get_nutritional_value() {
        // This test requires World::new() from sunaba-core
    }

    #[test]
    #[ignore] // Requires concrete World implementation from sunaba-core
    fn test_consume_edible_material() {
        // This test requires World::new() from sunaba-core
    }

    #[test]
    #[ignore] // Requires concrete World implementation from sunaba-core
    fn test_mine_air_fails() {
        // This test requires World::new() from sunaba-core
    }

    #[test]
    #[ignore] // Requires concrete World implementation from sunaba-core
    fn test_place_material() {
        // This test requires World::new() from sunaba-core
    }

    #[test]
    #[ignore] // Requires concrete World implementation from sunaba-core
    fn test_environmental_damage_safe_area() {
        // This test requires World::new() from sunaba-core
    }

    #[test]
    #[ignore] // Requires concrete World implementation from sunaba-core
    fn test_mine_and_place_cycle() {
        // This test requires World::new() from sunaba-core
    }

    #[test]
    #[ignore] // Requires concrete World implementation from sunaba-core
    fn test_cannot_mine_bedrock() {
        // This test requires World::new() from sunaba-core
    }
}
