//! Helper functions for SpacetimeDB reducers

use glam::IVec2;
use spacetimedb::{ReducerContext, Table};

use crate::encoding;
use crate::tables::{ChunkData, Player, chunk_data, player};

// ============================================================================
// Helper Functions for World Simulation
// ============================================================================

/// Load chunk from DB or generate new one
pub fn load_or_create_chunk(
    ctx: &ReducerContext,
    world: &mut sunaba_core::world::World,
    chunk_x: i32,
    chunk_y: i32,
) {
    let pos = IVec2::new(chunk_x, chunk_y);

    if world.has_chunk(pos) {
        return;
    }

    // Try load from DB
    if let Some(data) = ctx
        .db
        .chunk_data()
        .iter()
        .find(|c| c.x == chunk_x && c.y == chunk_y)
        && let Ok(chunk) = encoding::decode_chunk(&data.pixel_data)
    {
        world.insert_chunk(pos, chunk);
        log::debug!("Loaded chunk ({}, {}) from DB", chunk_x, chunk_y);
        return;
    }

    // Generate new
    log::debug!("Generating chunk ({}, {})", chunk_x, chunk_y);
    world.generate_chunk(pos);

    // CRITICAL FIX: Save newly generated chunk to database immediately
    // Without this, chunks exist only in server memory and aren't synced to clients
    if let Some(chunk) = world.get_chunk(chunk_x, chunk_y) {
        let pixel_data =
            encoding::encode_chunk(chunk).expect("Failed to encode newly generated chunk");

        ctx.db.chunk_data().insert(ChunkData {
            id: 0, // auto_inc
            x: chunk_x,
            y: chunk_y,
            pixel_data,
            dirty: false,
            last_modified_tick: 0,
        });

        log::debug!(
            "Saved newly generated chunk ({}, {}) to DB",
            chunk_x,
            chunk_y
        );
    }
}

/// Sync dirty chunks from World to database
/// CRITICAL OPTIMIZATION: Only syncs chunks marked dirty by simulation
pub fn sync_dirty_chunks_to_db(ctx: &ReducerContext, world: &sunaba_core::world::World, tick: u64) {
    for (pos, chunk) in world.chunks_iter() {
        // Skip clean chunks (optimization)
        if !chunk.is_dirty() {
            continue;
        }

        let Ok(pixel_data) = encoding::encode_chunk(chunk) else {
            log::error!("Failed to encode chunk ({}, {})", pos.x, pos.y);
            continue;
        };

        // Update or insert
        if let Some(existing) = ctx
            .db
            .chunk_data()
            .iter()
            .find(|c| c.x == pos.x && c.y == pos.y)
        {
            ctx.db.chunk_data().id().update(ChunkData {
                pixel_data,
                dirty: true,
                last_modified_tick: tick,
                ..existing
            });
        } else {
            ctx.db.chunk_data().insert(ChunkData {
                id: 0,
                x: pos.x,
                y: pos.y,
                pixel_data,
                dirty: true,
                last_modified_tick: tick,
            });
        }
    }
}

/// Get chunks at radius r from center (for settlement system)
pub fn get_chunks_at_radius(center_x: i32, center_y: i32, r: i32) -> Vec<(i32, i32)> {
    let mut chunks = Vec::new();

    if r == 0 {
        return vec![(center_x, center_y)];
    }

    // Top and bottom edges
    for x in -r..=r {
        chunks.push((center_x + x, center_y - r));
        chunks.push((center_x + x, center_y + r));
    }

    // Left and right edges (excluding corners to avoid duplicates)
    for y in (-r + 1)..(r) {
        chunks.push((center_x - r, center_y + y));
        chunks.push((center_x + r, center_y + y));
    }

    chunks
}

// ============================================================================
// Legacy Helper Functions
// ============================================================================

/// Find chunk at given chunk coordinates
pub fn find_chunk_at(ctx: &ReducerContext, chunk_x: i32, chunk_y: i32) -> Option<ChunkData> {
    ctx.db
        .chunk_data()
        .iter()
        .find(|c| c.x == chunk_x && c.y == chunk_y)
}

/// Update player physics based on velocity
pub fn update_player_physics(ctx: &ReducerContext, player: Player, delta_time: f32) {
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

// ============================================================================
// Admin Authentication Helpers
// ============================================================================

/// Check if sender is an admin
pub fn is_admin(ctx: &ReducerContext) -> bool {
    use crate::tables::admin_user;
    ctx.db.admin_user().identity().find(ctx.sender).is_some()
}

/// Require admin status or return early (macro)
#[macro_export]
macro_rules! require_admin {
    ($ctx:expr) => {
        if !$crate::helpers::is_admin($ctx) {
            log::warn!("Admin action denied for {:?}", $ctx.sender);
            return;
        }
    };
}
