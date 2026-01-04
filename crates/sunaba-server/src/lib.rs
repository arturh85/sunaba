//! SpacetimeDB multiplayer server module for Sunaba
//!
//! This module provides multiplayer support with:
//! - World simulation (falling sand cellular automata)
//! - Server-side creature AI (neural network inference)
//! - Player synchronization
//!
//! Note: This is a minimal implementation to establish the structure.
//! Full functionality will be added incrementally.

mod encoding;
mod world_access;

use spacetimedb::{Identity, ReducerContext, Table};

use glam::Vec2;
use sunaba_creature::{
    CreatureArchetype, CreatureGenome, CreatureMorphology, CreaturePhysicsState,
    DeepNeuralController, MorphologyConfig, SensorConfig, SensoryInput,
};
use sunaba_simulation::{Pixel, CHUNK_SIZE};

use crate::encoding::decode_chunk_pixels;
use crate::world_access::SpacetimeWorldAccess;

// Note: WorldRng is automatically implemented for any rand::Rng via blanket impl in sunaba-core
// This includes SpacetimeDB's ctx.rng() which implements rand::Rng

// ============================================================================
// Tables
// ============================================================================

/// Global world configuration (singleton, id=0)
#[spacetimedb::table(name = world_config, public)]
pub struct WorldConfig {
    #[primary_key]
    pub id: u64,
    /// World generation seed
    pub seed: u64,
    /// Current simulation tick
    pub tick_count: u64,
    /// Whether simulation is paused
    pub simulation_paused: bool,
    /// Maximum creatures allowed
    pub max_creatures: u32,
}

/// Chunk pixel data
#[spacetimedb::table(name = chunk_data, public)]
pub struct ChunkData {
    #[primary_key]
    #[auto_inc]
    pub id: u64,
    /// Chunk X coordinate (chunk space)
    pub x: i32,
    /// Chunk Y coordinate (chunk space)
    pub y: i32,
    /// Serialized pixel data (bincode)
    pub pixel_data: Vec<u8>,
    /// Whether chunk needs re-simulation
    pub dirty: bool,
    /// Last modification tick
    pub last_modified_tick: u64,
}

/// Player state
#[spacetimedb::table(name = player, public)]
pub struct Player {
    #[primary_key]
    pub identity: Identity,
    /// Player display name
    pub name: Option<String>,
    /// Whether player is currently connected
    pub online: bool,
    /// Position X
    pub x: f32,
    /// Position Y
    pub y: f32,
    /// Velocity X
    pub vel_x: f32,
    /// Velocity Y
    pub vel_y: f32,
    /// Selected material for placement
    pub selected_material: u16,
    /// Current health
    pub health: f32,
    /// Current hunger
    pub hunger: f32,
}

/// Creature state (server-side AI)
#[spacetimedb::table(name = creature_data, public)]
pub struct CreatureData {
    #[primary_key]
    #[auto_inc]
    pub id: u64,
    /// Entity ID (unique identifier)
    pub entity_id: u64,
    /// Position X
    pub x: f32,
    /// Position Y
    pub y: f32,
    /// Chunk X (for spatial queries)
    pub chunk_x: i32,
    /// Chunk Y (for spatial queries)
    pub chunk_y: i32,
    /// Velocity X
    pub vel_x: f32,
    /// Velocity Y
    pub vel_y: f32,
    /// Archetype name
    pub archetype: String,
    /// Serialized CreatureGenome (bincode)
    pub genome_data: Vec<u8>,
    /// Serialized CreatureMorphology (bincode)
    pub morphology_data: Vec<u8>,
    /// Serialized CreaturePhysicsState (bincode)
    pub physics_state_data: Vec<u8>,
    /// Current health
    pub health: f32,
    /// Maximum health
    pub max_health: f32,
    /// Current hunger
    pub hunger: f32,
    /// Maximum hunger
    pub max_hunger: f32,
    /// Generation number
    pub generation: u64,
    /// Food eaten count
    pub food_eaten: u32,
    /// Blocks mined count
    pub blocks_mined: u32,
    /// Whether creature is alive
    pub alive: bool,
}

// ============================================================================
// Lifecycle Reducers
// ============================================================================

/// Initialize the server module
#[spacetimedb::reducer(init)]
pub fn init(ctx: &ReducerContext) {
    log::info!("Initializing Sunaba server module");

    // Create world config singleton
    ctx.db.world_config().insert(WorldConfig {
        id: 0,
        seed: 12345,
        tick_count: 0,
        simulation_paused: false,
        max_creatures: 50,
    });

    log::info!("Server initialized");
}

/// Handle client connection
#[spacetimedb::reducer(client_connected)]
pub fn client_connected(ctx: &ReducerContext) {
    log::info!("Client connected: {:?}", ctx.sender);

    // Check if player already exists
    if let Some(player) = ctx.db.player().identity().find(ctx.sender) {
        // Mark existing player as online
        ctx.db.player().identity().update(Player {
            online: true,
            ..player
        });
        log::info!("Returning player reconnected");
    } else {
        // Create new player at spawn point
        ctx.db.player().insert(Player {
            identity: ctx.sender,
            name: None,
            online: true,
            x: 500.0,
            y: 200.0,
            vel_x: 0.0,
            vel_y: 0.0,
            selected_material: 2, // Sand
            health: 100.0,
            hunger: 100.0,
        });
        log::info!("New player created");
    }
}

/// Handle client disconnection
#[spacetimedb::reducer(client_disconnected)]
pub fn client_disconnected(ctx: &ReducerContext) {
    log::info!("Client disconnected: {:?}", ctx.sender);

    if let Some(player) = ctx.db.player().identity().find(ctx.sender) {
        ctx.db.player().identity().update(Player {
            online: false,
            ..player
        });
    }
}

// ============================================================================
// Manual Tick Reducers (called by clients or scheduled externally)
// ============================================================================

/// World simulation tick - call this periodically for simulation
#[spacetimedb::reducer]
pub fn world_tick(ctx: &ReducerContext) {
    // Get world config
    let Some(config) = ctx.db.world_config().id().find(0) else {
        log::error!("World config not found");
        return;
    };

    if config.simulation_paused {
        return;
    }

    // Update tick count
    ctx.db.world_config().id().update(WorldConfig {
        tick_count: config.tick_count + 1,
        ..config
    });

    // Get all online players
    let online_players: Vec<Player> = ctx.db.player().iter().filter(|p| p.online).collect();

    // Update player physics
    let delta_time = 0.016; // ~60fps
    for player in online_players {
        update_player_physics(ctx, player, delta_time);
    }
}

/// Creature AI tick - call this periodically for creature updates
#[spacetimedb::reducer]
pub fn creature_tick(ctx: &ReducerContext) {
    let delta_time = 0.033; // ~30fps

    // Get all living creatures
    let creatures: Vec<CreatureData> = ctx.db.creature_data().iter().filter(|c| c.alive).collect();

    for creature_row in creatures {
        // Deserialize creature state
        let Ok(genome) = encoding::decode_genome(&creature_row.genome_data) else {
            log::error!(
                "Failed to deserialize genome for creature {}",
                creature_row.id
            );
            continue;
        };
        let Ok(morphology) = encoding::decode_morphology(&creature_row.morphology_data) else {
            log::error!(
                "Failed to deserialize morphology for creature {}",
                creature_row.id
            );
            continue;
        };
        let Ok(mut physics_state) =
            encoding::decode_physics_state(&creature_row.physics_state_data)
        else {
            log::error!(
                "Failed to deserialize physics state for creature {}",
                creature_row.id
            );
            continue;
        };

        // Rebuild brain from genome (deterministic)
        let num_raycasts = 8;
        let num_materials = 5;
        let body_part_features = morphology.body_parts.len() * (9 + num_raycasts + num_materials);
        let output_dim = morphology.joints.len() + 1;

        let mut brain =
            DeepNeuralController::from_genome(&genome.controller, body_part_features, output_dim);

        // Create world access for sensing
        let world_access = SpacetimeWorldAccess::new(ctx);

        // Gather sensory input
        let position = Vec2::new(creature_row.x, creature_row.y);
        let sensor_config = SensorConfig::default();
        let sensory_input = SensoryInput::gather(&world_access, position, &sensor_config);

        // Extract body part features and run neural network
        let features = extract_creature_features(&morphology, &physics_state, &sensory_input);

        if features.len() == brain.input_dim() {
            let outputs = brain.forward(&features);

            // Apply motor commands
            let num_joints = morphology.joints.len();
            if outputs.len() > num_joints {
                let joint_commands: Vec<f32> = outputs[..num_joints].to_vec();
                physics_state.apply_all_motor_commands(&joint_commands, &morphology, delta_time);
            }
        }

        // Apply physics
        physics_state.apply_motor_rotations(&morphology, position);

        // Update hunger
        let mut new_hunger = creature_row.hunger - (genome.metabolic.hunger_rate * delta_time);
        let mut new_health = creature_row.health;

        // Starvation damage
        if new_hunger <= 0.0 {
            new_hunger = 0.0;
            new_health -= 5.0 * delta_time;
        }

        let alive = new_health > 0.0;

        // Serialize updated state
        let Ok(physics_state_data) = encoding::encode_physics_state(&physics_state) else {
            log::error!(
                "Failed to serialize physics state for creature {}",
                creature_row.id
            );
            continue;
        };

        // Update creature in database
        ctx.db.creature_data().id().update(CreatureData {
            physics_state_data,
            health: new_health,
            hunger: new_hunger,
            alive,
            ..creature_row
        });
    }
}

// ============================================================================
// Player Action Reducers
// ============================================================================

/// Update player position directly (client-authoritative for now)
#[spacetimedb::reducer]
pub fn player_update_position(ctx: &ReducerContext, x: f32, y: f32, vel_x: f32, vel_y: f32) {
    let Some(player) = ctx.db.player().identity().find(ctx.sender) else {
        log::warn!("Player not found: {:?}", ctx.sender);
        return;
    };

    ctx.db.player().identity().update(Player {
        x,
        y,
        vel_x,
        vel_y,
        ..player
    });
}

/// Place a material at world coordinates
#[spacetimedb::reducer]
pub fn player_place_material(ctx: &ReducerContext, world_x: i32, world_y: i32, material_id: u16) {
    let Some(player) = ctx.db.player().identity().find(ctx.sender) else {
        log::warn!("Player not found for place: {:?}", ctx.sender);
        return;
    };

    // Distance check
    let dx = world_x as f32 - player.x;
    let dy = world_y as f32 - player.y;
    let distance = (dx * dx + dy * dy).sqrt();

    if distance > 50.0 {
        log::warn!("Player tried to place too far away");
        return;
    }

    // Get or create chunk
    let chunk_x = world_x.div_euclid(CHUNK_SIZE as i32);
    let chunk_y = world_y.div_euclid(CHUNK_SIZE as i32);
    let local_x = world_x.rem_euclid(CHUNK_SIZE as i32) as usize;
    let local_y = world_y.rem_euclid(CHUNK_SIZE as i32) as usize;

    if let Some(chunk) = find_chunk_at(ctx, chunk_x, chunk_y)
        && let Ok(mut pixels) = decode_chunk_pixels(&chunk.pixel_data) {
            let idx = local_y * CHUNK_SIZE + local_x;
            if idx < pixels.len() {
                pixels[idx] = Pixel::new(material_id);

                if let Ok(pixel_data) = encoding::encode_chunk_pixels(&pixels) {
                    ctx.db.chunk_data().id().update(ChunkData {
                        pixel_data,
                        dirty: true,
                        ..chunk
                    });
                }
            }
        }
}

/// Mine a pixel at world coordinates
#[spacetimedb::reducer]
pub fn player_mine(ctx: &ReducerContext, world_x: i32, world_y: i32) {
    let Some(player) = ctx.db.player().identity().find(ctx.sender) else {
        log::warn!("Player not found for mine: {:?}", ctx.sender);
        return;
    };

    // Distance check
    let dx = world_x as f32 - player.x;
    let dy = world_y as f32 - player.y;
    let distance = (dx * dx + dy * dy).sqrt();

    if distance > 50.0 {
        log::warn!("Player tried to mine too far away");
        return;
    }

    let chunk_x = world_x.div_euclid(CHUNK_SIZE as i32);
    let chunk_y = world_y.div_euclid(CHUNK_SIZE as i32);
    let local_x = world_x.rem_euclid(CHUNK_SIZE as i32) as usize;
    let local_y = world_y.rem_euclid(CHUNK_SIZE as i32) as usize;

    if let Some(chunk) = find_chunk_at(ctx, chunk_x, chunk_y)
        && let Ok(mut pixels) = decode_chunk_pixels(&chunk.pixel_data) {
            let idx = local_y * CHUNK_SIZE + local_x;
            if idx < pixels.len() {
                pixels[idx] = Pixel::new(0); // Air

                if let Ok(pixel_data) = encoding::encode_chunk_pixels(&pixels) {
                    ctx.db.chunk_data().id().update(ChunkData {
                        pixel_data,
                        dirty: true,
                        ..chunk
                    });
                }
            }
        }
}

/// Set player name
#[spacetimedb::reducer]
pub fn set_player_name(ctx: &ReducerContext, name: String) {
    let Some(player) = ctx.db.player().identity().find(ctx.sender) else {
        log::warn!("Player not found for set_name: {:?}", ctx.sender);
        return;
    };

    ctx.db.player().identity().update(Player {
        name: Some(name),
        ..player
    });
}

// ============================================================================
// Creature Management Reducers
// ============================================================================

/// Spawn a creature from archetype
#[spacetimedb::reducer]
pub fn spawn_creature(ctx: &ReducerContext, archetype: String, x: f32, y: f32) {
    // Check creature limit
    let Some(config) = ctx.db.world_config().id().find(0) else {
        log::error!("World config not found");
        return;
    };

    let current_count = ctx.db.creature_data().iter().filter(|c| c.alive).count() as u32;
    if current_count >= config.max_creatures {
        log::warn!("Maximum creature limit reached");
        return;
    }

    // Parse archetype
    let archetype_enum = match archetype.to_lowercase().as_str() {
        "spider" => CreatureArchetype::Spider,
        "snake" => CreatureArchetype::Snake,
        "worm" => CreatureArchetype::Worm,
        "flyer" => CreatureArchetype::Flyer,
        _ => CreatureArchetype::Evolved,
    };

    // Create genome based on archetype
    let genome = match archetype_enum {
        CreatureArchetype::Spider => CreatureGenome::archetype_spider(),
        CreatureArchetype::Snake => CreatureGenome::archetype_snake(),
        CreatureArchetype::Worm => CreatureGenome::archetype_worm(),
        CreatureArchetype::Flyer => CreatureGenome::archetype_flyer(),
        CreatureArchetype::Evolved => CreatureGenome::archetype_spider(), // Default to spider for evolved
    };
    let morph_config = MorphologyConfig::default();
    let morphology = archetype_enum.create_morphology(&genome, &morph_config);
    let physics_state = CreaturePhysicsState::new(&morphology, Vec2::new(x, y));

    // Serialize
    let Ok(genome_data) = encoding::encode_genome(&genome) else {
        log::error!("Failed to serialize genome");
        return;
    };
    let Ok(morphology_data) = encoding::encode_morphology(&morphology) else {
        log::error!("Failed to serialize morphology");
        return;
    };
    let Ok(physics_state_data) = encoding::encode_physics_state(&physics_state) else {
        log::error!("Failed to serialize physics state");
        return;
    };

    // Insert creature
    ctx.db.creature_data().insert(CreatureData {
        id: 0, // auto_inc
        entity_id: config.tick_count, // Use tick as unique ID
        x,
        y,
        chunk_x: (x / CHUNK_SIZE as f32).floor() as i32,
        chunk_y: (y / CHUNK_SIZE as f32).floor() as i32,
        vel_x: 0.0,
        vel_y: 0.0,
        archetype,
        genome_data,
        morphology_data,
        physics_state_data,
        health: 100.0,
        max_health: 100.0,
        hunger: 100.0,
        max_hunger: 100.0,
        generation: genome.generation,
        food_eaten: 0,
        blocks_mined: 0,
        alive: true,
    });

    log::info!("Spawned creature at ({}, {})", x, y);
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Find chunk at given chunk coordinates
fn find_chunk_at(ctx: &ReducerContext, chunk_x: i32, chunk_y: i32) -> Option<ChunkData> {
    ctx.db
        .chunk_data()
        .iter()
        .find(|c| c.x == chunk_x && c.y == chunk_y)
}

/// Update player physics based on velocity
fn update_player_physics(ctx: &ReducerContext, player: Player, delta_time: f32) {
    const GRAVITY: f32 = 300.0;

    let vel_y = player.vel_y + GRAVITY * delta_time;
    let new_x = player.x + player.vel_x * delta_time;
    let new_y = player.y + vel_y * delta_time;

    ctx.db.player().identity().update(Player {
        x: new_x,
        y: new_y,
        vel_y,
        ..player
    });
}

/// Extract features from creature for neural network input
fn extract_creature_features(
    morphology: &CreatureMorphology,
    physics_state: &CreaturePhysicsState,
    sensory_input: &SensoryInput,
) -> Vec<f32> {
    let mut features = Vec::new();

    for (i, _part) in morphology.body_parts.iter().enumerate() {
        // Joint angle and velocity
        features.push(physics_state.get_motor_angle(i).unwrap_or(0.0));
        features.push(physics_state.get_motor_velocity(i).unwrap_or(0.0));

        // Position relative to root
        if let (Some(pos), Some(root_pos)) = (
            physics_state.part_positions.get(i),
            physics_state.part_positions.first(),
        ) {
            features.push((pos.x - root_pos.x) / 50.0);
            features.push((pos.y - root_pos.y) / 50.0);
        } else {
            features.push(0.0);
            features.push(0.0);
        }

        // Ground contact, food direction, etc.
        features.push(0.0); // ground contact
        // Food direction from food_direction field
        if let Some(dir) = sensory_input.food_direction {
            features.push(dir.x);
            features.push(dir.y);
        } else {
            features.push(0.0);
            features.push(0.0);
        }
        features.push(sensory_input.food_distance);
        features.push(sensory_input.gradients.food); // food gradient intensity

        // Raycast distances (8 rays)
        for ray in &sensory_input.raycasts {
            features.push(ray.distance);
        }
        for _ in sensory_input.raycasts.len()..8 {
            features.push(1.0);
        }

        // Contact materials (5 slots)
        for _ in 0..5 {
            features.push(0.0);
        }
    }

    features
}
