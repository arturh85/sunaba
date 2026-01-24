//! Player action reducers (movement, placement, mining, name setting)

use spacetimedb::ReducerContext;
use sunaba_simulation::{CHUNK_SIZE, Pixel};

use crate::encoding;
use crate::helpers::find_chunk_at;
use crate::tables::{ChunkData, Player, chunk_data, player};

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
    // Get or create chunk
    let chunk_x = world_x.div_euclid(CHUNK_SIZE as i32);
    let chunk_y = world_y.div_euclid(CHUNK_SIZE as i32);
    let local_x = world_x.rem_euclid(CHUNK_SIZE as i32) as usize;
    let local_y = world_y.rem_euclid(CHUNK_SIZE as i32) as usize;

    if let Some(chunk) = find_chunk_at(ctx, chunk_x, chunk_y)
        && let Ok(mut pixels) = encoding::decode_chunk_pixels(&chunk.pixel_data)
    {
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
    let chunk_x = world_x.div_euclid(CHUNK_SIZE as i32);
    let chunk_y = world_y.div_euclid(CHUNK_SIZE as i32);
    let local_x = world_x.rem_euclid(CHUNK_SIZE as i32) as usize;
    let local_y = world_y.rem_euclid(CHUNK_SIZE as i32) as usize;

    if let Some(chunk) = find_chunk_at(ctx, chunk_x, chunk_y)
        && let Ok(mut pixels) = encoding::decode_chunk_pixels(&chunk.pixel_data)
    {
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

/// Respawn dead player at server-determined spawn point
#[spacetimedb::reducer]
pub fn player_respawn(ctx: &ReducerContext) {
    let Some(player) = ctx.db.player().identity().find(ctx.sender) else {
        log::error!("Player respawn failed: player not found");
        return;
    };

    // Server-determined spawn point (can be customized based on world state)
    let spawn_x = 0.0;
    let spawn_y = 100.0;

    // Get player name before update (to avoid borrow after move)
    let player_name = player.name.clone();

    // Update player state (reset position, health, hunger)
    ctx.db.player().identity().update(Player {
        x: spawn_x,
        y: spawn_y,
        vel_x: 0.0,
        vel_y: 0.0,
        health: 100.0,
        hunger: 100.0,
        ..player
    });

    log::info!(
        "Player {} respawned at ({}, {})",
        player_name.as_deref().unwrap_or("Unknown"),
        spawn_x,
        spawn_y
    );
}
