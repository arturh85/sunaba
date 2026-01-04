//! Integration tests for creature-world interactions
//!
//! These tests require both the World implementation and creature functionality,
//! so they live in sunaba-core which has access to both.

use glam::Vec2;
use sunaba_core::world::World;
use sunaba_creature::{
    genome::CreatureGenome,
    morphology::{CreatureMorphology, MorphologyConfig},
    neural::extract_body_part_features_simple,
    sensors::{SensorConfig, SensoryInput},
    simple_physics::CreaturePhysicsState,
    types::EntityId,
    world_interaction::{consume_edible_material, get_nutritional_value, is_edible},
};
use sunaba_simulation::MaterialId;

// ============================================================================
// World Interaction Tests
// ============================================================================

#[test]
fn test_is_edible() {
    let world = World::new(false);

    // Fruit is edible
    assert!(is_edible(&world, MaterialId::FRUIT));

    // Stone is not edible
    assert!(!is_edible(&world, MaterialId::STONE));
    assert!(!is_edible(&world, MaterialId::AIR));
    assert!(!is_edible(&world, MaterialId::BEDROCK));
}

#[test]
fn test_get_nutritional_value() {
    let world = World::new(false);

    // Fruit should provide nutrition
    let fruit_nutrition = get_nutritional_value(&world, MaterialId::FRUIT);
    assert!(fruit_nutrition > 0.0);

    // Inedible materials should provide no nutrition
    assert_eq!(get_nutritional_value(&world, MaterialId::STONE), 0.0);
    assert_eq!(get_nutritional_value(&world, MaterialId::AIR), 0.0);
}

#[test]
fn test_consume_edible_material() {
    let mut world = World::new(false);
    world.ensure_chunks_for_area(0, 0, 100, 100);

    // Place fruit at (50, 50)
    world.set_pixel(50, 50, MaterialId::FRUIT);
    assert_eq!(
        world.get_pixel(50, 50).unwrap().material_id,
        MaterialId::FRUIT
    );

    // Consume it
    let entity_id = EntityId::from_raw(1);
    let nutrition = consume_edible_material(&mut world, Vec2::new(50.0, 50.0), &entity_id);

    assert!(nutrition.is_some());
    assert!(nutrition.unwrap() > 0.0);

    // Should be replaced with air
    assert_eq!(
        world.get_pixel(50, 50).unwrap().material_id,
        MaterialId::AIR
    );
}

#[test]
fn test_consume_inedible_returns_none() {
    let mut world = World::new(false);
    world.ensure_chunks_for_area(0, 0, 100, 100);

    // Place stone at (50, 50)
    world.set_pixel(50, 50, MaterialId::STONE);

    // Try to consume it
    let entity_id = EntityId::from_raw(1);
    let nutrition = consume_edible_material(&mut world, Vec2::new(50.0, 50.0), &entity_id);

    assert!(nutrition.is_none());

    // Stone should still be there
    assert_eq!(
        world.get_pixel(50, 50).unwrap().material_id,
        MaterialId::STONE
    );
}

#[test]
fn test_consume_air_returns_none() {
    let mut world = World::new(false);
    world.ensure_chunks_for_area(0, 0, 100, 100);

    // Position (50, 50) should be air
    assert_eq!(
        world.get_pixel(50, 50).unwrap().material_id,
        MaterialId::AIR
    );

    // Try to consume it
    let entity_id = EntityId::from_raw(1);
    let nutrition = consume_edible_material(&mut world, Vec2::new(50.0, 50.0), &entity_id);

    assert!(nutrition.is_none());
}

// ============================================================================
// Sensor Tests
// ============================================================================

#[test]
fn test_sensory_input_gather_in_empty_world() {
    let mut world = World::new(false);
    world.ensure_chunks_for_area(0, 0, 200, 200);

    let config = SensorConfig::default();
    let sensory = SensoryInput::gather(&world, Vec2::new(100.0, 100.0), &config);

    // Should have the correct number of raycasts
    assert_eq!(sensory.raycasts.len(), config.num_raycasts);

    // All raycasts should hit air (no obstacles)
    for raycast in &sensory.raycasts {
        assert_eq!(raycast.material_id, MaterialId::AIR);
    }

    // No food or threats in empty world
    assert!(sensory.nearest_food.is_none());
    assert!(sensory.nearest_threat.is_none());
}

#[test]
fn test_sensory_input_detect_nearby_food() {
    let mut world = World::new(false);
    world.ensure_chunks_for_area(0, 0, 200, 200);

    // Place fruit nearby at (110, 100)
    world.set_pixel(110, 100, MaterialId::FRUIT);

    let config = SensorConfig::default();
    let sensory = SensoryInput::gather(&world, Vec2::new(100.0, 100.0), &config);

    // Should detect food
    assert!(sensory.nearest_food.is_some());

    let food_pos = sensory.nearest_food.unwrap();
    // Food should be close to (110, 100)
    assert!((food_pos - Vec2::new(110.0, 100.0)).length() < 5.0);
}

#[test]
fn test_sensory_input_raycast_hits_obstacle() {
    let mut world = World::new(false);
    world.ensure_chunks_for_area(0, 0, 200, 200);

    // Place a wall at x=120
    for y in 90..110 {
        world.set_pixel(120, y, MaterialId::STONE);
    }

    let config = SensorConfig {
        num_raycasts: 8,
        raycast_distance: 50.0,
        food_detection_radius: 50.0,
        threat_detection_radius: 50.0,
        food_compass_radius: 100.0,
    };

    let sensory = SensoryInput::gather(&world, Vec2::new(100.0, 100.0), &config);

    // At least one raycast should hit the wall (distance < max)
    let hit_wall = sensory
        .raycasts
        .iter()
        .any(|r| r.distance < config.raycast_distance);
    assert!(hit_wall);

    // At least one raycast should detect stone
    let detected_stone = sensory
        .raycasts
        .iter()
        .any(|r| r.material_id == MaterialId::STONE);
    assert!(detected_stone);
}

#[test]
fn test_sensory_input_raycast_count_matches_config() {
    let mut world = World::new(false);
    world.ensure_chunks_for_area(0, 0, 200, 200);

    let config = SensorConfig {
        num_raycasts: 12,
        raycast_distance: 40.0,
        food_detection_radius: 30.0,
        threat_detection_radius: 30.0,
        food_compass_radius: 100.0,
    };

    let sensory = SensoryInput::gather(&world, Vec2::new(100.0, 100.0), &config);

    assert_eq!(sensory.raycasts.len(), 12);
}

#[test]
fn test_chemical_gradients_normalized() {
    let mut world = World::new(false);
    world.ensure_chunks_for_area(0, 0, 200, 200);

    // Place multiple food sources
    world.set_pixel(110, 100, MaterialId::FRUIT);
    world.set_pixel(105, 105, MaterialId::FRUIT);
    world.set_pixel(108, 98, MaterialId::FRUIT);

    let config = SensorConfig::default();
    let sensory = SensoryInput::gather(&world, Vec2::new(100.0, 100.0), &config);

    // Gradients should be in [0, 1] range
    assert!(sensory.gradients.food >= 0.0 && sensory.gradients.food <= 1.0);
    assert!(sensory.gradients.danger >= 0.0 && sensory.gradients.danger <= 1.0);
    assert!(sensory.gradients.mate >= 0.0 && sensory.gradients.mate <= 1.0);
}

// ============================================================================
// Neural Feature Extraction Test
// ============================================================================

#[test]
fn test_extract_body_part_features() {
    let genome = CreatureGenome::test_biped();
    let morphology_config = MorphologyConfig::default();
    let morphology = CreatureMorphology::from_genome(&genome, &morphology_config);

    let mut world = World::new(false);
    world.ensure_chunks_for_area(0, 0, 200, 200);

    // Place some ground
    for x in 0..200 {
        for y in 0..20 {
            world.set_pixel(x, y, MaterialId::STONE);
        }
    }

    let config = SensorConfig::default();
    let sensory_input = SensoryInput::gather(&world, Vec2::new(100.0, 50.0), &config);
    let physics_state = CreaturePhysicsState::new(&morphology, Vec2::new(100.0, 50.0));

    let features =
        extract_body_part_features_simple(&morphology, &physics_state, &sensory_input, &world);

    // Should have features for each body part
    assert_eq!(features.len(), morphology.body_parts.len());

    // Each feature should have raycast data and contact materials
    for feature in &features {
        assert_eq!(feature.raycast_distances.len(), config.num_raycasts);
        assert_eq!(feature.contact_materials.len(), 5); // Top 5 contact materials
    }
}

// ============================================================================
// Environmental Damage Test
// ============================================================================

#[test]
fn test_environmental_damage_from_lava() {
    use sunaba_creature::types::Health;
    use sunaba_creature::world_interaction::apply_environmental_damage;

    let mut world = World::new(false);
    world.ensure_chunks_for_area(0, 0, 200, 200);

    // Place lava at (100, 100)
    world.set_pixel(100, 100, MaterialId::LAVA);

    // Apply environmental damage at lava position
    let mut health = Health::new(100.0);
    let initial_health = health.current;
    apply_environmental_damage(&world, Vec2::new(100.0, 100.0), &mut health);

    // Lava should cause damage
    assert!(health.current < initial_health, "Lava should cause damage");
}

#[test]
fn test_environmental_damage_safe_in_air() {
    use sunaba_creature::types::Health;
    use sunaba_creature::world_interaction::apply_environmental_damage;

    let mut world = World::new(false);
    world.ensure_chunks_for_area(0, 0, 200, 200);

    // Check damage in air
    let mut health = Health::new(100.0);
    apply_environmental_damage(&world, Vec2::new(100.0, 100.0), &mut health);

    assert_eq!(health.current, 100.0, "Air should not cause damage");
}
