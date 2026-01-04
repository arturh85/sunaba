//! Scheduled tick reducers for world simulation, creature AI, and settlement

use glam::Vec2;
use spacetimedb::{ReducerContext, Table};
use std::time::Duration;

use sunaba_creature::{
    CreatureMorphology, CreaturePhysicsState, DeepNeuralController, SensorConfig, SensoryInput,
};

use crate::encoding;
use crate::helpers::{
    get_chunks_at_radius, load_or_create_chunk, sync_dirty_chunks_to_db, update_player_physics,
};
use crate::state::{NoOpStats, SERVER_WORLD};
use crate::tables::{
    ChunkData, CreatureData, CreatureTickTimer, Player, ServerMetrics, SettleTickTimer,
    WorldConfig, WorldTickTimer, chunk_data, creature_data, creature_tick_timer, player,
    server_metrics, settle_tick_timer, world_config, world_tick_timer,
};
use crate::world_access::SpacetimeWorldAccess;

// ============================================================================
// Manual Tick Reducers (called by clients or scheduled externally)
// ============================================================================

/// World simulation tick - scheduled at 60fps
#[spacetimedb::reducer]
pub fn world_tick(ctx: &ReducerContext, _arg: WorldTickTimer) {
    let Some(config) = ctx.db.world_config().id().find(0) else {
        log::error!("World config not found");
        return;
    };

    if config.simulation_paused {
        return;
    }

    let new_tick_count = config.tick_count + 1;

    // Update tick count
    ctx.db.world_config().id().update(WorldConfig {
        tick_count: new_tick_count,
        ..config
    });

    // Initialize or get World instance
    let mut world_guard = SERVER_WORLD.lock().unwrap();
    if world_guard.is_none() {
        log::info!("Initializing server World with seed {}", config.seed);
        let mut world = sunaba_core::world::World::new(true);
        world.set_generator(config.seed);
        log::info!("Server world initialized with terrain generation");
        *world_guard = Some(world);
    }

    let Some(world) = world_guard.as_mut() else {
        log::error!("Failed to get World");
        return;
    };

    // Get online players
    let online_players: Vec<Player> = ctx.db.player().iter().filter(|p| p.online).collect();

    // Load 7x7 chunks around each player
    for player in &online_players {
        let chunk_x = (player.x as i32).div_euclid(64);
        let chunk_y = (player.y as i32).div_euclid(64);

        for dy in -3..=3 {
            for dx in -3..=3 {
                load_or_create_chunk(ctx, world, chunk_x + dx, chunk_y + dy);
            }
        }
    }

    // Run simulation (World::update uses dirty chunk optimization internally)
    let delta_time = 0.016;
    let mut stats = NoOpStats;
    let mut rng = ctx.rng();
    world.update(delta_time, &mut stats, &mut rng, true); // Server: skip creatures (not implemented server-side)

    // Sync ONLY dirty chunks to database
    sync_dirty_chunks_to_db(ctx, world, config.tick_count);

    // Update players
    for player in online_players {
        update_player_physics(ctx, player, delta_time);
    }

    // Collect server metrics every 10th tick (6fps sampling)
    if new_tick_count % 10 == 0 {
        // Count active chunks (loaded in world)
        let active_chunks = world.active_chunks().count() as u32;

        // Count online players
        let online_players_count = ctx.db.player().iter().filter(|p| p.online).count() as u32;

        // Count alive creatures
        let creatures_alive = ctx.db.creature_data().iter().filter(|c| c.alive).count() as u32;

        // Insert metrics sample
        ctx.db.server_metrics().insert(ServerMetrics {
            id: 0,
            tick: new_tick_count,
            timestamp_ms: (new_tick_count * 16), // Approximate timestamp (16ms per tick)
            world_tick_time_ms: 0.0,             // TODO: Add timing once SpacetimeDB supports it
            creature_tick_time_ms: 0.0,          // TODO: Get from creature_tick
            active_chunks,
            dirty_chunks_synced: 0, // TODO: Track in sync_dirty_chunks_to_db
            online_players: online_players_count,
            creatures_alive,
        });
    }

    // Cleanup old metrics every 600 ticks (10 seconds at 60fps)
    if new_tick_count % 600 == 0 {
        cleanup_old_metrics(ctx);
    }

    // Schedule next tick (16ms = 60fps)
    ctx.db.world_tick_timer().insert(WorldTickTimer {
        id: 0,
        scheduled_at: Duration::from_millis(16).into(),
    });
}

/// Creature AI tick - scheduled at 30fps
#[spacetimedb::reducer]
pub fn creature_tick(ctx: &ReducerContext, _arg: CreatureTickTimer) {
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

    // Schedule next tick (33ms = 30fps)
    ctx.db.creature_tick_timer().insert(CreatureTickTimer {
        id: 0,
        scheduled_at: Duration::from_millis(33).into(),
    });
}

/// World settlement tick - scheduled at 10fps (low priority)
/// Pre-simulates chunks in expanding rings from spawn to prevent falling sand during exploration
#[spacetimedb::reducer]
pub fn settle_world_tick(ctx: &ReducerContext, _arg: SettleTickTimer) {
    let Some(config) = ctx.db.world_config().id().find(0) else {
        return;
    };

    if config.settlement_complete {
        return; // Already settled
    }

    let mut world_guard = SERVER_WORLD.lock().unwrap();
    let Some(world) = world_guard.as_mut() else {
        return;
    };

    // Settle chunks in expanding ring around spawn (0, 0)
    let r = config.settlement_progress;
    let chunks_to_settle = get_chunks_at_radius(0, 0, r);

    for (chunk_x, chunk_y) in chunks_to_settle {
        // Load chunk if not already loaded
        load_or_create_chunk(ctx, world, chunk_x, chunk_y);

        // Simulate chunk for 60 ticks (1 second worth of settling)
        let mut rng = ctx.rng();
        for _ in 0..60 {
            world.update_chunk_settle(chunk_x, chunk_y, &mut rng);
        }

        // Save settled chunk to DB
        if let Some(chunk) = world.get_chunk(chunk_x, chunk_y) {
            let Ok(pixel_data) = encoding::encode_chunk(chunk) else {
                continue;
            };

            ctx.db.chunk_data().insert(ChunkData {
                id: 0,
                x: chunk_x,
                y: chunk_y,
                pixel_data,
                dirty: false, // Settled chunks start clean
                last_modified_tick: 0,
            });
        }
    }

    // Update progress
    let new_progress = r + 1;
    let complete = new_progress > config.settlement_radius;

    ctx.db.world_config().id().update(WorldConfig {
        settlement_progress: new_progress,
        settlement_complete: complete,
        ..config
    });

    if complete {
        log::info!(
            "World settlement complete! Settled {} chunks from spawn",
            config.settlement_radius
        );
    }

    // Schedule next tick (100ms = 10fps)
    ctx.db.settle_tick_timer().insert(SettleTickTimer {
        id: 0,
        scheduled_at: Duration::from_millis(100).into(),
    });
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Cleanup old server metrics (keep last 3600 samples = 10 minutes at 6fps)
fn cleanup_old_metrics(ctx: &ReducerContext) {
    const MAX_METRICS: usize = 3600;

    let mut metrics: Vec<ServerMetrics> = ctx.db.server_metrics().iter().collect();

    if metrics.len() > MAX_METRICS {
        metrics.sort_by_key(|m| m.tick);
        let to_remove = metrics.len() - MAX_METRICS;

        for metric in metrics.iter().take(to_remove) {
            ctx.db.server_metrics().id().delete(metric.id);
        }

        log::info!(
            "Cleaned up {} old server metrics (kept {})",
            to_remove,
            MAX_METRICS
        );
    }
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
        let padding_count = 8_usize.saturating_sub(sensory_input.raycasts.len());
        features.extend(std::iter::repeat_n(1.0, padding_count));

        // Contact materials (5 slots)
        features.extend(std::iter::repeat_n(0.0, 5));
    }

    features
}
