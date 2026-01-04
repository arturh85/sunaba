//! Admin-only reducers for world management

use crate::tables::{AdminUser, WorldConfig, admin_user, chunk_data, world_config};
use spacetimedb::rand::Rng;
use spacetimedb::{ReducerContext, Table};

/// Claim admin status (client sends their email from parsed JWT)
/// Server checks if email is in whitelist from SUNABA_ADMIN_EMAILS environment variable
#[spacetimedb::reducer]
pub fn claim_admin(ctx: &ReducerContext, email: String) {
    log::info!(
        "Admin claim request from {:?} with email: {}",
        ctx.sender,
        email
    );

    // Check if email is in admin whitelist (from environment)
    let admin_whitelist = std::env::var("SUNABA_ADMIN_EMAILS").unwrap_or_default();

    let is_whitelisted = admin_whitelist
        .split(',')
        .map(|s| s.trim())
        .any(|e| e == email);

    if !is_whitelisted {
        log::warn!("Email not in admin whitelist: {}", email);
        return;
    }

    // Check if already admin
    if ctx.db.admin_user().identity().find(ctx.sender).is_some() {
        log::info!("User already has admin status: {}", email);
        return;
    }

    // Grant admin status
    ctx.db.admin_user().insert(AdminUser {
        identity: ctx.sender,
        email: email.clone(),
        granted_at: ctx.timestamp,
        last_seen: ctx.timestamp,
    });

    log::info!("Admin status granted: {}", email);
}

/// Rebuild world: Clear all chunks and reset world state
/// Admin only - players remain connected but world is regenerated
#[spacetimedb::reducer]
pub fn rebuild_world(ctx: &ReducerContext) {
    // Require admin
    crate::require_admin!(ctx);

    log::info!("Admin {:?} rebuilding world...", ctx.sender);

    // Clear all chunk data
    let chunks: Vec<_> = ctx.db.chunk_data().iter().collect();
    let chunk_count = chunks.len();
    for chunk in &chunks {
        ctx.db.chunk_data().id().delete(chunk.id);
    }

    log::info!("Cleared {} chunks", chunk_count);

    // Reset world config (optionally change seed)
    if let Some(config) = ctx.db.world_config().id().find(0) {
        let new_seed = ctx.rng().r#gen::<u64>();

        ctx.db.world_config().id().update(WorldConfig {
            seed: new_seed,
            tick_count: 0,
            settlement_progress: 0,
            settlement_complete: false,
            ..config
        });

        log::info!("World reset with new seed: {}", new_seed);
    }

    log::info!("World rebuild complete");
}
