//! SpacetimeDB table definitions for Sunaba multiplayer server

use spacetimedb::{Identity, ScheduleAt, Timestamp};

// Import reducer functions for scheduled tables
use crate::reducers::{creature_tick, settle_world_tick, world_tick};

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
    /// Settlement radius (chunks from spawn to pre-simulate)
    pub settlement_radius: i32,
    /// Current settlement progress (chunks settled so far)
    pub settlement_progress: i32,
    /// Whether settlement is complete
    pub settlement_complete: bool,
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

/// Server performance metrics (rolling history)
#[spacetimedb::table(name = server_metrics, public)]
pub struct ServerMetrics {
    #[primary_key]
    #[auto_inc]
    pub id: u64,
    /// Tick number when metric was recorded
    pub tick: u64,
    /// Timestamp (milliseconds since server start)
    pub timestamp_ms: u64,
    /// World tick processing time (ms)
    pub world_tick_time_ms: f32,
    /// Creature tick processing time (ms)
    pub creature_tick_time_ms: f32,
    /// Active chunks loaded in memory
    pub active_chunks: u32,
    /// Dirty chunks synced this tick
    pub dirty_chunks_synced: u32,
    /// Online players count
    pub online_players: u32,
    /// Total creatures alive
    pub creatures_alive: u32,
}

/// Timer table for world simulation ticks (60fps)
#[spacetimedb::table(name = world_tick_timer, scheduled(world_tick))]
pub struct WorldTickTimer {
    #[primary_key]
    #[auto_inc]
    pub id: u64,
    pub scheduled_at: ScheduleAt,
}

/// Timer table for creature AI ticks (30fps)
#[spacetimedb::table(name = creature_tick_timer, scheduled(creature_tick))]
pub struct CreatureTickTimer {
    #[primary_key]
    #[auto_inc]
    pub id: u64,
    pub scheduled_at: ScheduleAt,
}

/// Timer table for world settlement (10fps, low priority)
#[spacetimedb::table(name = settle_tick_timer, scheduled(settle_world_tick))]
pub struct SettleTickTimer {
    #[primary_key]
    #[auto_inc]
    pub id: u64,
    pub scheduled_at: ScheduleAt,
}

// ============================================================================
// Admin Tables
// ============================================================================

/// Admin users (granted based on email whitelist from environment variable)
#[spacetimedb::table(name = admin_user, public)]
pub struct AdminUser {
    #[primary_key]
    pub identity: Identity,
    pub email: String,
    pub granted_at: Timestamp,
    pub last_seen: Timestamp,
}
