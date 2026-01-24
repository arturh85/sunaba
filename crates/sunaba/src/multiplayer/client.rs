//! SpacetimeDB Rust SDK client wrapper for native multiplayer integration

use anyhow::Context;
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

// Import generated SpacetimeDB client bindings
use super::generated::{self, DbConnection};
use generated::chunk_data_table::ChunkDataTableAccess;
use generated::claim_admin_reducer::claim_admin;
use generated::creature_data_table::CreatureDataTableAccess;
use generated::player_respawn_reducer::player_respawn;
use generated::player_table::PlayerTableAccess;
use generated::rebuild_world_reducer::rebuild_world;
use generated::request_ping_reducer::request_ping;
use generated::server_metrics_table::ServerMetricsTableAccess;
use generated::set_player_name_reducer::set_player_name;
use generated::{player_mine, player_place_material, player_update_position};
use spacetimedb_sdk::{DbContext, Table, TableWithPrimaryKey}; // Trait for connection and table methods

// Re-export traits needed by app.rs for player table access
pub use generated::player_table::PlayerTableAccess as PlayerTableAccessTrait;
pub use spacetimedb_sdk::{DbContext as DbContextTrait, Table as TableTrait};

// OAuth imports (native only)
#[cfg(all(not(target_arch = "wasm32"), feature = "multiplayer_native"))]
use crate::multiplayer::oauth_native::{
    OAuthClaims, delete_oauth_token as native_delete_token, load_oauth_token as native_load_token,
    oauth_login as native_oauth_login, parse_jwt_claims, save_oauth_token as native_save_token,
};

/// SpacetimeDB client wrapper for native multiplayer integration
#[derive(Clone)]
pub struct MultiplayerClient {
    /// SpacetimeDB connection (wrapped in Arc<Mutex> for interior mutability)
    connection: Option<Arc<Mutex<DbConnection>>>,

    /// Server host URL
    host: String,

    /// Database name
    db_name: String,

    /// Background thread that processes WebSocket messages (keeps connection alive)
    /// Note: JoinHandle doesn't implement Clone, so we wrap in Arc<Mutex<Option<...>>>
    /// The thread is spawned on connect and dropped on disconnect
    #[allow(dead_code)]
    message_thread: Arc<Mutex<Option<std::thread::JoinHandle<()>>>>,

    /// Index mapping (x, y) coordinates to chunk IDs for O(1) lookup
    /// This avoids O(n) linear search with 29KB clones per iteration
    chunk_coord_index: Arc<Mutex<HashMap<(i32, i32), u64>>>,

    /// Disconnect event flag (set by on_disconnected callback)
    /// Used to detect when WebSocket dies so UI can update state
    disconnect_detected: Arc<AtomicBool>,

    /// Flag set by background thread when subscription data is received
    /// This bypasses the frame_tick() requirement for detection
    subscription_data_received: Arc<AtomicBool>,

    /// Chunks that have been updated on the server and need reloading
    /// Set by on_update callback (background thread), consumed by sync_chunks_progressive
    chunks_needing_reload: Arc<Mutex<HashSet<(i32, i32)>>>,
}

/// Generate default nickname from Identity (format: "Player_abc123" using last 6 hex chars)
fn generate_default_nickname(identity: &spacetimedb_sdk::Identity) -> String {
    let identity_str = identity.to_string();
    // Take last 6 characters of hex identity
    let suffix = if identity_str.len() >= 6 {
        &identity_str[identity_str.len() - 6..]
    } else {
        &identity_str
    };
    format!("Player_{}", suffix)
}

impl MultiplayerClient {
    /// Create a new multiplayer client (not yet connected)
    pub fn new() -> Self {
        Self {
            connection: None,
            host: String::new(),
            db_name: String::new(),
            message_thread: Arc::new(Mutex::new(None)),
            chunk_coord_index: Arc::new(Mutex::new(HashMap::new())),
            disconnect_detected: Arc::new(AtomicBool::new(false)),
            subscription_data_received: Arc::new(AtomicBool::new(false)),
            chunks_needing_reload: Arc::new(Mutex::new(HashSet::new())),
        }
    }

    /// Connect to SpacetimeDB server
    ///
    /// If `fresh_identity` is true, skips loading stored OAuth token and uses a fresh
    /// anonymous identity. This allows multiple client instances to connect simultaneously.
    pub async fn connect(
        &mut self,
        host: impl Into<String>,
        db_name: impl Into<String>,
        fresh_identity: bool,
    ) -> anyhow::Result<()> {
        self.host = host.into();
        self.db_name = db_name.into();

        // Reset flags for new connection
        self.disconnect_detected.store(false, Ordering::SeqCst);
        self.subscription_data_received
            .store(false, Ordering::SeqCst);

        log::info!(
            "Connecting to SpacetimeDB at {}/{}",
            self.host,
            self.db_name
        );

        // Load OAuth token if available (native only), skip if fresh_identity
        #[cfg(all(not(target_arch = "wasm32"), feature = "multiplayer_native"))]
        let token = if fresh_identity {
            log::info!("[SpacetimeDB] Using fresh anonymous identity (--fresh-identity)");
            None
        } else {
            native_load_token()
        };

        #[cfg(not(all(not(target_arch = "wasm32"), feature = "multiplayer_native")))]
        let token: Option<String> = {
            let _ = fresh_identity; // suppress unused warning
            None
        };

        if let Some(ref token) = token {
            log::info!("[SpacetimeDB] Connecting with OAuth token");
            // Extract email for logging
            #[cfg(all(not(target_arch = "wasm32"), feature = "multiplayer_native"))]
            if let Ok(claims) = parse_jwt_claims(token) {
                log::info!("[SpacetimeDB] Authenticated as: {:?}", claims.email);
            }
        } else {
            log::info!("[SpacetimeDB] Connecting anonymously (no OAuth token)");
        }

        // Build connection using generated DbConnection
        // Use closure for on_disconnect to capture the disconnect flag
        let disconnect_flag = Arc::clone(&self.disconnect_detected);
        let mut builder = DbConnection::builder()
            .on_connect(Self::on_connected)
            .on_connect_error(Self::on_connect_error)
            .on_disconnect(move |_ctx, err| {
                disconnect_flag.store(true, Ordering::SeqCst);
                if let Some(err) = err {
                    log::warn!("[SpacetimeDB] Disconnected with error: {}", err);
                } else {
                    log::info!("[SpacetimeDB] Disconnected");
                }
            })
            .with_uri(&self.host)
            .with_module_name(&self.db_name);

        // Add token if available
        if let Some(token) = token {
            builder = builder.with_token(Some(token));
        }

        let conn = builder
            .build()
            .context("Failed to build SpacetimeDB connection")?;

        // Spawn background thread to process WebSocket messages (keeps connection alive)
        // This is required - without it, the connection will timeout and disconnect
        let thread_handle = conn.run_threaded();
        log::info!("Started SpacetimeDB message processing thread");

        self.connection = Some(Arc::new(Mutex::new(conn)));
        *self.message_thread.lock().unwrap() = Some(thread_handle);

        log::info!("Connected to SpacetimeDB successfully");

        Ok(())
    }

    /// Subscribe to world state (chunks, players, creatures)
    pub async fn subscribe_world(&mut self) -> anyhow::Result<()> {
        let conn = self
            .connection
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Not connected to server"))?;

        log::info!("Subscribing to world state tables");

        let mut conn_guard = conn.lock().unwrap();

        // Subscribe to world config
        let _config_sub = conn_guard
            .subscription_builder()
            .on_applied(|_ctx| {
                log::debug!("World config subscription applied");
            })
            .subscribe("SELECT * FROM world_config");

        // Subscribe to chunk data with update callbacks
        // Initial subscription: small radius (3 chunks) for fast spawn load
        // This gives us a 7x7 grid (49 chunks) instead of larger area
        // Will be expanded to larger radius after spawn chunks are loaded
        // Note: Use BETWEEN instead of ABS() - SpacetimeDB doesn't support functions in WHERE
        // Diagnostic: Try SELECT * without WHERE to rule out BETWEEN clause issues
        // TODO: Restore filtered query after confirming subscription works
        let chunk_index_for_applied = Arc::clone(&self.chunk_coord_index);
        let chunk_index_for_insert = Arc::clone(&self.chunk_coord_index);
        let chunk_index_for_delete = Arc::clone(&self.chunk_coord_index);
        let subscription_flag = Arc::clone(&self.subscription_data_received);

        // Register table callbacks for incremental index updates
        conn_guard.db.chunk_data().on_insert(move |_ctx, chunk| {
            // Update index when new chunk is inserted
            if let Ok(mut index) = chunk_index_for_insert.lock() {
                index.insert((chunk.x, chunk.y), chunk.id);
            }
        });
        conn_guard.db.chunk_data().on_delete(move |_ctx, chunk| {
            // Remove from index when chunk is deleted
            if let Ok(mut index) = chunk_index_for_delete.lock() {
                index.remove(&(chunk.x, chunk.y));
            }
        });

        // Register on_update callback to detect when other players modify chunks
        let chunks_for_update = Arc::clone(&self.chunks_needing_reload);
        conn_guard
            .db
            .chunk_data()
            .on_update(move |_ctx, _old_chunk, new_chunk| {
                // Mark this chunk as needing reload (another player modified it)
                if let Ok(mut chunks) = chunks_for_update.lock() {
                    chunks.insert((new_chunk.x, new_chunk.y));
                }
            });

        let _chunk_sub = conn_guard
            .subscription_builder()
            .on_applied(move |ctx| {
                // Set flag immediately - main thread can detect without frame_tick()
                subscription_flag.store(true, Ordering::SeqCst);

                // Build coordinate index for O(1) chunk lookup
                let mut index = chunk_index_for_applied.lock().unwrap();
                index.clear();
                let mut chunk_coords = Vec::new();
                for chunk in ctx.db.chunk_data().iter() {
                    index.insert((chunk.x, chunk.y), chunk.id);
                    chunk_coords.push((chunk.x, chunk.y));
                }
                let index_size = index.len();
                drop(index); // Release lock

                // Check spawn chunks (0,0 and immediate neighbors)
                let spawn_positions = [(0, 0), (-1, 0), (1, 0), (0, -1), (0, 1)];
                let spawn_present: Vec<_> = spawn_positions
                    .iter()
                    .filter(|(x, y)| chunk_coords.contains(&(*x, *y)))
                    .collect();

                log::info!(
                    "Chunk data subscription applied - {} chunks received, spawn chunks present: {}/5 {:?}",
                    chunk_coords.len(),
                    spawn_present.len(),
                    spawn_present
                );
                log::info!("Built chunk coordinate index ({} entries)", index_size);

                // Log chunk coordinate range for diagnostic
                if let (Some(min_x), Some(max_x), Some(min_y), Some(max_y)) = (
                    chunk_coords.iter().map(|(x, _)| *x).min(),
                    chunk_coords.iter().map(|(x, _)| *x).max(),
                    chunk_coords.iter().map(|(_, y)| *y).min(),
                    chunk_coords.iter().map(|(_, y)| *y).max(),
                ) {
                    log::info!(
                        "Initial chunk cache range: x=[{}, {}], y=[{}, {}]",
                        min_x,
                        max_x,
                        min_y,
                        max_y
                    );
                }
            })
            .on_error(|_ctx, err| {
                log::error!("Chunk subscription error: {:?}", err);
            })
            .subscribe("SELECT * FROM chunk_data");

        // Subscribe to players
        let _player_sub = conn_guard
            .subscription_builder()
            .on_applied(|ctx| {
                log::debug!(
                    "Player subscription applied - {} players",
                    ctx.db.player().iter().count()
                );
            })
            .subscribe("SELECT * FROM player");

        // Set default nickname if not already set (identity may not be available yet)
        if let Some(identity) = conn_guard.try_identity() {
            // Check if player has a name
            if let Some(player) = conn_guard.db.player().identity().find(&identity) {
                if player.name.is_none() {
                    let default_name = generate_default_nickname(&identity);
                    drop(conn_guard); // Release lock before calling reducer
                    if let Err(e) = self.set_nickname(default_name.clone()) {
                        log::warn!("Failed to set default nickname: {}", e);
                    } else {
                        log::info!("Set default nickname: {}", default_name);
                    }
                    // Re-acquire lock for remaining subscriptions
                    conn_guard = conn.lock().unwrap();
                }
            }
        } else {
            log::debug!("Identity not yet available, will set nickname later");
        }

        // Subscribe to creatures
        let _creature_sub = conn_guard
            .subscription_builder()
            .on_applied(|ctx| {
                log::debug!(
                    "Creature subscription applied - {} creatures",
                    ctx.db.creature_data().iter().count()
                );
            })
            .subscribe("SELECT * FROM creature_data");

        // Subscribe to server metrics
        let _metrics_sub = conn_guard
            .subscription_builder()
            .on_applied(|ctx| {
                log::debug!(
                    "Server metrics subscription applied - {} samples",
                    ctx.db.server_metrics().iter().count()
                );
            })
            .subscribe("SELECT * FROM server_metrics");

        log::info!("Subscribed to world state successfully");

        Ok(())
    }

    /// Process incoming messages (call this in your game loop)
    pub fn frame_tick(&self) {
        if let Some(ref conn) = self.connection {
            let conn_guard = conn.lock().unwrap();
            // Only tick if connection is active
            if !conn_guard.is_active() {
                return;
            }
            if let Err(e) = conn_guard.frame_tick() {
                // Filter out "already disconnected" spam
                let msg = e.to_string();
                if !msg.contains("already disconnected") && !msg.contains("terminated normally") {
                    log::error!("Error processing SpacetimeDB messages: {}", e);
                }
            }
        }
    }

    /// Send player position update to server
    pub fn update_player_position(&self, x: f32, y: f32, vx: f32, vy: f32) -> anyhow::Result<()> {
        let conn = self
            .connection
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Not connected to server"))?;

        let conn_guard = conn.lock().unwrap();
        conn_guard
            .reducers
            .player_update_position(x, y, vx, vy)
            .context("Failed to call player_update_position reducer")?;

        Ok(())
    }

    /// Request material placement at position
    pub fn place_material(&self, x: i32, y: i32, material_id: u16) -> anyhow::Result<()> {
        let conn = self
            .connection
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Not connected to server"))?;

        let conn_guard = conn.lock().unwrap();
        conn_guard
            .reducers
            .player_place_material(x, y, material_id)
            .context("Failed to call player_place_material reducer")?;

        Ok(())
    }

    /// Request mining at position
    pub fn mine(&self, x: i32, y: i32) -> anyhow::Result<()> {
        let conn = self
            .connection
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Not connected to server"))?;

        let conn_guard = conn.lock().unwrap();
        conn_guard
            .reducers
            .player_mine(x, y)
            .context("Failed to call player_mine reducer")?;

        Ok(())
    }

    /// Claim admin status on the server (requires OAuth email)
    pub async fn claim_admin(&self, email: String) -> anyhow::Result<()> {
        let conn = self
            .connection
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Not connected to server"))?;

        let conn_guard = conn.lock().unwrap();
        conn_guard
            .reducers
            .claim_admin(email)
            .context("Failed to call claim_admin reducer")?;

        Ok(())
    }

    /// Request server to rebuild the world
    pub async fn rebuild_world(&self) -> anyhow::Result<()> {
        let conn = self
            .connection
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Not connected to server"))?;

        let conn_guard = conn.lock().unwrap();
        conn_guard
            .reducers
            .rebuild_world()
            .context("Failed to call rebuild_world reducer")?;

        Ok(())
    }

    /// Set player nickname (calls set_player_name reducer)
    pub fn set_nickname(&self, name: String) -> anyhow::Result<()> {
        let conn = self
            .connection
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Not connected to server"))?;

        let conn_guard = conn.lock().unwrap();
        conn_guard
            .reducers
            .set_player_name(name.clone())
            .context("Failed to call set_player_name reducer")?;

        log::info!("Nickname set to: {}", name);
        Ok(())
    }

    /// Get the SpacetimeDB connection (for reading player data)
    pub fn get_connection(&self) -> Option<&Arc<Mutex<DbConnection>>> {
        self.connection.as_ref()
    }

    /// Request player respawn from server
    pub fn request_respawn(&self) -> anyhow::Result<()> {
        let conn = self
            .connection
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Not connected to server"))?;

        let conn_guard = conn.lock().unwrap();
        conn_guard
            .reducers
            .player_respawn()
            .context("Failed to call player_respawn reducer")?;

        log::info!("Respawn request sent to server");
        Ok(())
    }

    /// Sync chunks from server cache to local world (progressive loading)
    ///
    /// Uses a chunk load queue to rate-limit chunk loading (2-3 chunks per frame).
    /// Call this in your game loop for progressive, non-blocking chunk streaming.
    ///
    /// Performance: Uses O(1) coordinate index lookup instead of O(n) linear search
    /// to avoid cloning 29KB ChunkData per iteration (was causing 34+ second load times).
    pub fn sync_chunks_progressive(
        &self,
        world: &mut sunaba_core::world::World,
        load_queue: &mut crate::multiplayer::chunk_loader::ChunkLoadQueue,
    ) -> anyhow::Result<usize> {
        let conn = self
            .connection
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Not connected to server"))?;

        let conn_guard = conn.lock().unwrap();
        let mut synced_count = 0;

        // Lock index FIRST (needed for both pending updates and batch loading)
        // Uses try_lock semantics via .ok() - returns None if lock is held by background thread
        let index = self.chunk_coord_index.lock().ok();
        if index.is_none() {
            log::warn!("chunk_coord_index lock contention - skipping sync this frame");
            return Ok(0);
        }

        // Check for chunks updated by other players (on_update callback)
        // Process these IMMEDIATELY - don't wait for spiral iterator
        if let Ok(mut pending) = self.chunks_needing_reload.lock() {
            for (x, y) in pending.drain() {
                let pos = glam::IVec2::new(x, y);

                // Remove from world so it gets reloaded with new data
                if world.has_chunk(pos) {
                    world.chunks_mut().remove(&pos);
                }

                // Clear from load queue's "loaded" tracking
                load_queue.mark_needs_reload(pos);

                // IMMEDIATELY reload from server cache (don't wait for spiral)
                let chunk_row = index
                    .as_ref()
                    .and_then(|idx| idx.get(&(x, y)).copied())
                    .and_then(|id| conn_guard.db.chunk_data().id().find(&id));

                if let Some(chunk_row) = chunk_row {
                    if let Ok(chunk) = crate::encoding::decode_chunk(&chunk_row.pixel_data) {
                        world.insert_chunk(pos, chunk);
                        load_queue.mark_loaded(pos);
                        synced_count += 1;
                        log::info!("Chunk ({}, {}) reloaded from server update", x, y);
                    }
                }
            }
        }

        // Get next batch from spiral queue for progressive loading
        let batch = load_queue.next_batch();

        for pos in batch {
            // Skip if already loaded in world
            if world.has_chunk(pos) {
                load_queue.mark_loaded(pos);
                continue;
            }

            // O(1) lookup via coordinate index instead of O(n) linear search
            // This avoids cloning 29KB ChunkData per iteration
            let chunk_row = index
                .as_ref()
                .and_then(|idx| idx.get(&(pos.x, pos.y)).copied())
                .and_then(|id| conn_guard.db.chunk_data().id().find(&id));

            // Diagnostic logging for spawn chunks
            let is_spawn_chunk =
                matches!((pos.x, pos.y), (0, 0) | (-1, 0) | (1, 0) | (0, -1) | (0, 1));
            if is_spawn_chunk {
                if chunk_row.is_some() {
                    log::info!("Spawn chunk ({}, {}) found in cache, loading", pos.x, pos.y);
                } else {
                    log::debug!("Spawn chunk ({}, {}) NOT in cache yet", pos.x, pos.y);
                }
            }

            if let Some(chunk_row) = chunk_row {
                // Decode and insert
                let Ok(chunk) = crate::encoding::decode_chunk(&chunk_row.pixel_data) else {
                    log::warn!("Failed to decode chunk ({}, {})", pos.x, pos.y);
                    continue;
                };

                world.insert_chunk(pos, chunk);
                load_queue.mark_loaded(pos);
                synced_count += 1;
            }
            // Note: Don't mark as loaded if not found - chunk may arrive later
        }

        if synced_count > 0 {
            log::debug!("Synced {} chunks from server (progressive)", synced_count);
        }

        Ok(synced_count)
    }

    /// Sync chunks from server cache to local world (legacy method, loads all at once)
    ///
    /// For progressive loading, use `sync_chunks_progressive()` instead.
    pub fn sync_chunks_to_world(
        &self,
        world: &mut sunaba_core::world::World,
    ) -> anyhow::Result<usize> {
        use glam::IVec2;

        let conn = self
            .connection
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Not connected to server"))?;

        let conn_guard = conn.lock().unwrap();

        let mut synced_count = 0;

        // Iterate all cached chunks from server
        for chunk_row in conn_guard.db.chunk_data().iter() {
            let pos = IVec2::new(chunk_row.x, chunk_row.y);

            // Skip if already loaded
            if world.has_chunk(pos) {
                continue;
            }

            // Decode chunk data
            let Ok(chunk) = crate::encoding::decode_chunk(&chunk_row.pixel_data) else {
                log::warn!("Failed to decode chunk ({}, {})", chunk_row.x, chunk_row.y);
                continue;
            };

            // Insert into world
            world.insert_chunk(pos, chunk);
            synced_count += 1;
        }

        if synced_count > 0 {
            log::info!("Synced {} chunks from server", synced_count);
        }

        Ok(synced_count)
    }

    /// Get chunk data from local cache (for rendering)
    ///
    /// Uses O(1) coordinate index lookup instead of O(n) linear search.
    pub fn get_chunk(&self, x: i32, y: i32) -> Option<Vec<u8>> {
        let conn = self.connection.as_ref()?;
        let conn_guard = conn.lock().unwrap();

        // O(1) lookup via coordinate index
        let chunk_id = self.chunk_coord_index.lock().ok()?.get(&(x, y)).copied()?;
        conn_guard
            .db
            .chunk_data()
            .id()
            .find(&chunk_id)
            .map(|chunk| chunk.pixel_data.clone())
    }

    /// Send ping request to server for latency measurement
    pub fn request_ping(&self, timestamp_ms: u64) -> anyhow::Result<()> {
        let conn = self
            .connection
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Not connected to server"))?;

        let conn_guard = conn.lock().unwrap();
        conn_guard
            .reducers
            .request_ping(timestamp_ms)
            .context("Failed to call request_ping reducer")?;

        Ok(())
    }

    /// Get latest server metrics from subscribed table
    pub fn get_latest_server_metrics(&self) -> Option<generated::ServerMetrics> {
        let conn = self.connection.as_ref()?;
        let conn_guard = conn.lock().unwrap();

        // Get the most recent metric by tick number
        conn_guard
            .db
            .server_metrics()
            .iter()
            .max_by_key(|m| m.tick)
            .map(|m| m.clone())
    }

    /// Check if connected to server
    pub fn is_connected(&self) -> bool {
        self.connection
            .as_ref()
            .map(|conn| {
                let conn_guard = conn.lock().unwrap();
                conn_guard.is_active()
            })
            .unwrap_or(false)
    }

    /// Check if subscription data has been received (proves WebSocket is connected)
    ///
    /// Uses atomic flag for instant detection (set by background thread in on_applied),
    /// bypassing the need for frame_tick() to populate the local cache.
    pub fn has_received_subscription_data(&self) -> bool {
        // Check atomic flag first (set by background thread, instant detection)
        if self.subscription_data_received.load(Ordering::SeqCst) {
            return true;
        }

        // Fallback to cache check (requires frame_tick to have processed)
        self.connection
            .as_ref()
            .map(|conn| {
                let conn_guard = conn.lock().unwrap();
                // If we have ANY chunk data, the WebSocket is connected and subscription works
                conn_guard.db.chunk_data().iter().next().is_some()
            })
            .unwrap_or(false)
    }

    /// Disconnect from server
    pub async fn disconnect(&mut self) -> anyhow::Result<()> {
        log::info!("Disconnecting from SpacetimeDB");

        // Reset flags for next connection
        self.subscription_data_received
            .store(false, Ordering::SeqCst);
        self.disconnect_detected.store(false, Ordering::SeqCst);

        if let Some(conn) = self.connection.take() {
            let conn_guard = conn.lock().unwrap();
            conn_guard
                .disconnect()
                .context("Failed to disconnect from SpacetimeDB")?;
        }

        // Clear the message thread handle (thread will exit when connection is dropped)
        *self.message_thread.lock().unwrap() = None;

        Ok(())
    }

    // ===== Progressive Chunk Loading Methods =====

    /// Expand chunk subscription from initial small radius to larger radius
    ///
    /// Called after initial spawn chunks are loaded to expand the subscription area.
    /// Note: SpacetimeDB SDK manages subscription lifecycle automatically.
    pub fn expand_chunk_subscription(
        &mut self,
        center: glam::IVec2,
        radius: i32,
    ) -> anyhow::Result<()> {
        log::info!(
            "Expanding chunk subscription to radius {} around {:?}",
            radius,
            center
        );

        let conn = self
            .connection
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Not connected to server"))?;

        let conn_guard = conn.lock().unwrap();

        // Subscribe to larger area using BETWEEN (SpacetimeDB doesn't support ABS())
        // Previous subscription will be automatically managed by SDK
        let query = format!(
            "SELECT * FROM chunk_data WHERE x BETWEEN {} AND {} AND y BETWEEN {} AND {}",
            center.x - radius,
            center.x + radius,
            center.y - radius,
            center.y + radius
        );

        let _new_sub = conn_guard
            .subscription_builder()
            .on_applied(|ctx| {
                log::info!(
                    "Expanded chunk subscription applied - {} chunks received",
                    ctx.db.chunk_data().iter().count()
                );
            })
            .subscribe(query);

        log::info!("Expanded chunk subscription successfully");

        Ok(())
    }

    /// Re-subscribe to chunks centered around new position (when player moves far)
    ///
    /// Called when player moves >8 chunks from subscription center.
    /// Note: SpacetimeDB SDK manages subscription lifecycle automatically.
    pub fn resubscribe_chunks(&mut self, center: glam::IVec2, radius: i32) -> anyhow::Result<()> {
        log::info!(
            "Re-subscribing chunks around {:?} with radius {}",
            center,
            radius
        );

        let conn = self
            .connection
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Not connected to server"))?;

        let conn_guard = conn.lock().unwrap();

        // Subscribe to new range centered on player using BETWEEN
        // Previous subscription will be automatically managed by SDK
        let query = format!(
            "SELECT * FROM chunk_data WHERE x BETWEEN {} AND {} AND y BETWEEN {} AND {}",
            center.x - radius,
            center.x + radius,
            center.y - radius,
            center.y + radius
        );

        let _new_sub = conn_guard
            .subscription_builder()
            .on_applied(|ctx| {
                log::info!(
                    "Re-subscribed chunks - {} chunks received",
                    ctx.db.chunk_data().iter().count()
                );
            })
            .subscribe(query);

        log::info!("Re-subscribed to chunks successfully");

        Ok(())
    }

    // ===== OAuth Methods (Native Only) =====

    /// Initiate OAuth login flow (native only)
    /// Opens browser automatically, starts local HTTP server for callback
    #[cfg(all(not(target_arch = "wasm32"), feature = "multiplayer_native"))]
    pub fn oauth_login(&self) -> anyhow::Result<String> {
        native_oauth_login()
    }

    /// Save OAuth token to file (native only)
    #[cfg(all(not(target_arch = "wasm32"), feature = "multiplayer_native"))]
    pub fn save_oauth_token(&self, token: &str) -> anyhow::Result<()> {
        native_save_token(token)
    }

    /// Load saved OAuth token from file (native only)
    #[cfg(all(not(target_arch = "wasm32"), feature = "multiplayer_native"))]
    pub fn load_oauth_token(&self) -> Option<String> {
        native_load_token()
    }

    /// Get OAuth claims from saved token (native only)
    #[cfg(all(not(target_arch = "wasm32"), feature = "multiplayer_native"))]
    pub fn get_oauth_claims(&self) -> Option<OAuthClaims> {
        let token = self.load_oauth_token()?;
        parse_jwt_claims(&token).ok()
    }

    /// Delete OAuth token (logout, native only)
    #[cfg(all(not(target_arch = "wasm32"), feature = "multiplayer_native"))]
    pub fn oauth_logout(&self) {
        if let Err(e) = native_delete_token() {
            log::error!("Failed to delete OAuth token: {}", e);
        }
    }

    // Connection lifecycle callbacks
    fn on_connected(_conn: &DbConnection, identity: spacetimedb_sdk::Identity, token: &str) {
        log::info!(
            "[SpacetimeDB] Connected successfully (identity: {}, token: {})",
            identity,
            token
        );
    }

    fn on_connect_error(_ctx: &generated::ErrorContext, err: spacetimedb_sdk::Error) {
        log::error!("[SpacetimeDB] Connection error: {}", err);
    }

    /// Check if a disconnect was detected since last connect
    ///
    /// This flag is set by the on_disconnect callback when the WebSocket dies.
    /// The game loop should check this to update manager state.
    pub fn was_disconnected(&self) -> bool {
        self.disconnect_detected.load(Ordering::SeqCst)
    }

    /// Reset the disconnect flag (call after handling disconnect)
    pub fn clear_disconnect_flag(&self) {
        self.disconnect_detected.store(false, Ordering::SeqCst);
    }
}

impl Default for MultiplayerClient {
    fn default() -> Self {
        Self::new()
    }
}
