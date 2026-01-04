/**
 * SpacetimeDB TypeScript SDK bridge for Sunaba WASM
 *
 * This module provides JavaScript functions that can be called from Rust WASM
 * using wasm-bindgen. It wraps the SpacetimeDB TypeScript SDK.
 */

// Global state
let spacetimeClient = null;
let isConnectedFlag = false;
let latestServerMetrics = null;

/**
 * Connect to SpacetimeDB server
 * @param {string} host - Server URL (e.g., "http://localhost:3000")
 * @param {string} dbName - Database/module name
 * @returns {Promise<void>}
 */
export async function connectSpacetime(host, dbName, authToken = null) {
    try {
        console.log(`[SpacetimeDB] Connecting to ${host}/${dbName}...`);

        // Import SpacetimeDB SDK dynamically
        // NOTE: You need to install @clockworklabs/spacetimedb-sdk via npm
        // and bundle it, or use a CDN link in index.html
        const { SpacetimeDBClient } = await import('@clockworklabs/spacetimedb-sdk');

        // Create client with optional token
        if (authToken) {
            console.log('[SpacetimeDB] Connecting with OAuth token');
            spacetimeClient = new SpacetimeDBClient(host, dbName, authToken);
        } else {
            spacetimeClient = new SpacetimeDBClient(host, dbName);
        }

        // Connect to the server
        await spacetimeClient.connect();

        isConnectedFlag = true;
        console.log('[SpacetimeDB] Connected successfully!');

        // Set up event handlers
        spacetimeClient.on('connected', () => {
            console.log('[SpacetimeDB] Connection established');
            isConnectedFlag = true;
        });

        spacetimeClient.on('disconnected', () => {
            console.log('[SpacetimeDB] Disconnected');
            isConnectedFlag = false;
        });

        spacetimeClient.on('error', (error) => {
            console.error('[SpacetimeDB] Error:', error);
        });

    } catch (error) {
        console.error('[SpacetimeDB] Connection failed:', error);
        isConnectedFlag = false;
        throw error;
    }
}

/**
 * Subscribe to world state tables
 * @returns {Promise<void>}
 */
export async function subscribeWorld() {
    if (!spacetimeClient) {
        throw new Error('Not connected to SpacetimeDB');
    }

    try {
        console.log('[SpacetimeDB] Subscribing to world state...');

        // Subscribe to all relevant tables
        await spacetimeClient.subscribe([
            'SELECT * FROM world_config',
            'SELECT * FROM chunk_data',
            'SELECT * FROM player',
            'SELECT * FROM creature_data',
            'SELECT * FROM server_metrics'
        ]);

        // Set up table update handlers
        spacetimeClient.on('world_config', (table, operation, row) => {
            console.log('[SpacetimeDB] World config update:', row);
        });

        spacetimeClient.on('chunk_data', (table, operation, row) => {
            // console.log('[SpacetimeDB] Chunk data update:', row.x, row.y);
            // TODO: Update local chunk cache for rendering
        });

        spacetimeClient.on('player', (table, operation, row) => {
            console.log('[SpacetimeDB] Player update:', row);
        });

        spacetimeClient.on('creature_data', (table, operation, row) => {
            // console.log('[SpacetimeDB] Creature update:', row.id);
        });

        spacetimeClient.on('server_metrics', (table, operation, row) => {
            // Cache latest metrics (by tick number)
            if (!latestServerMetrics || row.tick > latestServerMetrics.tick) {
                latestServerMetrics = {
                    tick: row.tick,
                    timestamp_ms: row.timestamp_ms,
                    world_tick_time_ms: row.world_tick_time_ms,
                    creature_tick_time_ms: row.creature_tick_time_ms,
                    active_chunks: row.active_chunks,
                    dirty_chunks_synced: row.dirty_chunks_synced,
                    online_players: row.online_players,
                    creatures_alive: row.creatures_alive
                };
                // console.log('[SpacetimeDB] Metrics updated:', row.tick);
            }
        });

        console.log('[SpacetimeDB] Subscribed to world state');

    } catch (error) {
        console.error('[SpacetimeDB] Subscription failed:', error);
        throw error;
    }
}

/**
 * Send player position update to server
 * @param {number} x - Player X position
 * @param {number} y - Player Y position
 * @param {number} velX - Player X velocity
 * @param {number} velY - Player Y velocity
 */
export function updatePlayerPosition(x, y, velX, velY) {
    if (!spacetimeClient) {
        throw new Error('Not connected to SpacetimeDB');
    }

    try {
        spacetimeClient.call('player_update_position', x, y, velX, velY);
    } catch (error) {
        console.error('[SpacetimeDB] Failed to update player position:', error);
        throw error;
    }
}

/**
 * Send material placement request to server
 * @param {number} x - Pixel X coordinate
 * @param {number} y - Pixel Y coordinate
 * @param {number} materialId - Material ID
 */
export function placeMaterial(x, y, materialId) {
    if (!spacetimeClient) {
        throw new Error('Not connected to SpacetimeDB');
    }

    try {
        spacetimeClient.call('player_place_material', x, y, materialId);
    } catch (error) {
        console.error('[SpacetimeDB] Failed to place material:', error);
        throw error;
    }
}

/**
 * Send mining request to server
 * @param {number} x - Pixel X coordinate
 * @param {number} y - Pixel Y coordinate
 */
export function mineMaterial(x, y) {
    if (!spacetimeClient) {
        throw new Error('Not connected to SpacetimeDB');
    }

    try {
        spacetimeClient.call('player_mine', x, y);
    } catch (error) {
        console.error('[SpacetimeDB] Failed to mine:', error);
        throw error;
    }
}

/**
 * Check if connected to server
 * @returns {boolean}
 */
export function isConnected() {
    return isConnectedFlag && spacetimeClient !== null;
}

/**
 * Send ping request to server for latency measurement
 * @param {number} timestampMs - Client timestamp in milliseconds
 * @returns {Promise<void>}
 */
export async function requestPing(timestampMs) {
    if (!spacetimeClient) {
        throw new Error('Not connected to SpacetimeDB');
    }

    try {
        // Record send time for RTT calculation
        const sentTime = Date.now();

        // Call request_ping reducer (it echoes the timestamp)
        await spacetimeClient.call('request_ping', timestampMs);

        // Calculate round-trip time
        const rtt = Date.now() - sentTime;
        // console.log(`[SpacetimeDB] Ping RTT: ${rtt}ms`);

    } catch (error) {
        console.error('[SpacetimeDB] Ping failed:', error);
        throw error;
    }
}

/**
 * Get latest cached server metrics
 * @returns {Object|null} Server metrics object or null if none available
 */
export function getLatestServerMetrics() {
    return latestServerMetrics;
}

/**
 * Claim admin status (sends email to server for whitelist validation)
 * @param {string} email - User's email address from OAuth token
 * @returns {Promise<void>}
 */
export async function claimAdmin(email) {
    if (!spacetimeClient) {
        throw new Error('Not connected to SpacetimeDB');
    }

    try {
        console.log('[SpacetimeDB] Claiming admin status:', email);
        await spacetimeClient.call('claim_admin', email);
    } catch (error) {
        console.error('[SpacetimeDB] Failed to claim admin:', error);
        throw error;
    }
}

/**
 * Rebuild world (admin only - clears all chunks and resets world)
 * @returns {Promise<void>}
 */
export async function rebuildWorld() {
    if (!spacetimeClient) {
        throw new Error('Not connected to SpacetimeDB');
    }

    try {
        console.log('[SpacetimeDB] Requesting world rebuild...');
        await spacetimeClient.call('rebuild_world');
        console.log('[SpacetimeDB] World rebuild complete');
    } catch (error) {
        console.error('[SpacetimeDB] Failed to rebuild world:', error);
        throw error;
    }
}
