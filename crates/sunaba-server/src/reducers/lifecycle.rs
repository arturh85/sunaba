//! Lifecycle reducers for SpacetimeDB server initialization and client connections

use spacetimedb::{ReducerContext, Table};
use std::time::Duration;

use crate::tables::{
    AdminUser, CreatureTickTimer, Player, SettleTickTimer, WorldConfig, WorldTickTimer, admin_user,
    creature_tick_timer, player, settle_tick_timer, world_config, world_tick_timer,
};

// ============================================================================
// Lifecycle Reducers
// ============================================================================

/// Initialize the server module
#[spacetimedb::reducer(init)]
pub fn init(ctx: &ReducerContext) {
    log::info!("Initializing Sunaba multiplayer server");

    // Create world config singleton
    ctx.db.world_config().insert(WorldConfig {
        id: 0,
        seed: 12345,
        tick_count: 0,
        simulation_paused: false,
        max_creatures: 50,
        settlement_radius: 50, // Settle 50 chunks from spawn
        settlement_progress: 0,
        settlement_complete: false,
    });

    // Schedule world tick at 60fps (16ms)
    ctx.db.world_tick_timer().insert(WorldTickTimer {
        id: 0,
        scheduled_at: Duration::from_millis(16).into(),
    });

    // Schedule creature tick at 30fps (33ms)
    ctx.db.creature_tick_timer().insert(CreatureTickTimer {
        id: 0,
        scheduled_at: Duration::from_millis(33).into(),
    });

    // Schedule settlement tick at 10fps (100ms) - lower priority
    ctx.db.settle_tick_timer().insert(SettleTickTimer {
        id: 0,
        scheduled_at: Duration::from_millis(100).into(),
    });

    // Log admin whitelist from environment
    let admin_emails = std::env::var("SUNABA_ADMIN_EMAILS").unwrap_or_else(|_| String::new());

    if admin_emails.is_empty() {
        log::warn!("No admin emails configured (set SUNABA_ADMIN_EMAILS)");
    } else {
        for email in admin_emails.split(',') {
            let email = email.trim();
            if !email.is_empty() {
                log::info!("Admin whitelist: {}", email);
            }
        }
    }

    log::info!("Scheduled reducers initialized");
}

/// Handle client connection
#[spacetimedb::reducer(client_connected)]
pub fn client_connected(ctx: &ReducerContext) {
    log::info!("Client connected: {:?}", ctx.sender);

    // Update last seen for existing admin (if they are one)
    if let Some(existing) = ctx.db.admin_user().identity().find(ctx.sender) {
        let email = existing.email.clone();

        ctx.db.admin_user().identity().update(AdminUser {
            last_seen: ctx.timestamp,
            ..existing
        });
        log::info!("Admin reconnected: {}", email);
    }

    // Check if player already exists
    if let Some(player) = ctx.db.player().identity().find(ctx.sender) {
        // Mark existing player as online
        ctx.db.player().identity().update(Player {
            online: true,
            ..player
        });
        log::info!("Returning player reconnected");
    } else {
        // Create new player at spawn point (0, 100) - matches client spawn chunks
        ctx.db.player().insert(Player {
            identity: ctx.sender,
            name: None,
            online: true,
            x: 0.0,
            y: 100.0,
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
