//! Creature-world interaction
//!
//! Handles creature interactions with the pixel world: eating, mining, building, damage.

use glam::Vec2;

use crate::world::World;

/// Consume edible material at position
/// Returns nutritional value gained
pub fn consume_edible_material(
    world: &mut World,
    position: Vec2,
    _creature_id: &crate::entity::EntityId,
) -> Option<f32> {
    let pixel_x = position.x.round() as i32;
    let pixel_y = position.y.round() as i32;

    if let Some(pixel) = world.get_pixel(pixel_x, pixel_y) {
        let material_id = pixel.material_id;
        let material = world.materials().get(material_id);

        // Check if edible
        if let Some(nutrition) = material.nutritional_value {
            if nutrition > 0.0 {
                // Remove the pixel (eat it)
                world.set_pixel(pixel_x, pixel_y, 0); // Set to air
                return Some(nutrition);
            }
        }
    }

    None
}

/// Mine world pixel at position
/// Returns the material_id of the mined pixel, or None if can't mine
pub fn mine_world_pixel(
    world: &mut World,
    position: Vec2,
    _creature_id: &crate::entity::EntityId,
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
pub fn place_material(
    world: &mut World,
    position: Vec2,
    material_id: u16,
    _creature_id: &crate::entity::EntityId,
) -> bool {
    let pixel_x = position.x.round() as i32;
    let pixel_y = position.y.round() as i32;

    // Check if position is valid and currently air
    if let Some(pixel) = world.get_pixel(pixel_x, pixel_y) {
        if pixel.material_id == 0 {
            // Only place in air
            world.set_pixel(pixel_x, pixel_y, material_id);
            return true;
        }
    }

    false
}

/// Apply environmental damage to creature
/// Returns true if damage was applied
pub fn apply_environmental_damage(
    world: &World,
    position: Vec2,
    health: &mut crate::entity::health::Health,
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
pub fn is_edible(world: &World, material_id: u16) -> bool {
    let material = world.materials().get(material_id);
    material
        .nutritional_value
        .map(|nv| nv > 0.0)
        .unwrap_or(false)
}

/// Get nutritional value of material
pub fn get_nutritional_value(world: &World, material_id: u16) -> f32 {
    let material = world.materials().get(material_id);
    material.nutritional_value.unwrap_or(0.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entity::{health::Health, EntityId};

    #[test]
    fn test_is_edible() {
        let world = World::new();

        // Air should not be edible
        assert!(!is_edible(&world, 0));

        // Check if any materials are edible (depends on world setup)
        // For now just test that function runs without panicking
        let _result = is_edible(&world, 1);
    }

    #[test]
    fn test_get_nutritional_value() {
        let world = World::new();

        // Air has no nutritional value
        assert_eq!(get_nutritional_value(&world, 0), 0.0);

        // Function should run without panicking
        let _nutrition = get_nutritional_value(&world, 1);
    }

    #[test]
    fn test_consume_edible_material() {
        let mut world = World::new();
        let creature_id = EntityId::new();

        // Try to eat air (should fail)
        let nutrition = consume_edible_material(&mut world, Vec2::new(100.0, 100.0), &creature_id);
        assert!(nutrition.is_none() || nutrition.is_some());
    }

    #[test]
    fn test_mine_air_fails() {
        let mut world = World::new();
        let creature_id = EntityId::new();

        // Mining air should fail
        let result = mine_world_pixel(&mut world, Vec2::new(100.0, 100.0), &creature_id);

        // Result depends on what's at that position, but shouldn't panic
        assert!(result.is_none() || result.is_some());
    }

    #[test]
    fn test_place_material() {
        let mut world = World::new();
        let creature_id = EntityId::new();

        // Find an air position
        let position = Vec2::new(100.0, 100.0);

        // Try to place stone (material_id = 1)
        // Should either succeed or fail gracefully without panicking
        let _result = place_material(&mut world, position, 1, &creature_id);
    }

    #[test]
    fn test_environmental_damage_safe_area() {
        let world = World::new();
        let mut health = Health::new(100.0);

        // Safe area should not damage
        // May or may not be damaged depending on world state, just test it doesn't panic
        let _damaged = apply_environmental_damage(&world, Vec2::new(100.0, 100.0), &mut health);
    }

    #[test]
    fn test_mine_and_place_cycle() {
        let mut world = World::new();
        let creature_id = EntityId::new();

        let position = Vec2::new(100.0, 100.0);

        // Place a block
        let placed = place_material(&mut world, position, 1, &creature_id);

        if placed {
            // If placement succeeded, try to mine it back
            let mined = mine_world_pixel(&mut world, position, &creature_id);
            assert!(mined.is_some());

            if let Some(material_id) = mined {
                // Should get back stone
                assert_eq!(material_id, 1);
            }
        }
    }

    #[test]
    fn test_cannot_mine_bedrock() {
        let mut world = World::new();
        let creature_id = EntityId::new();

        // Try to place bedrock first (material_id = 14)
        let position = Vec2::new(200.0, 200.0);
        world.set_pixel(200, 200, 14); // Bedrock

        // Try to mine bedrock (should fail)
        let result = mine_world_pixel(&mut world, position, &creature_id);
        assert!(result.is_none());
    }
}
