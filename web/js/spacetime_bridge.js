/**
 * SpacetimeDB TypeScript SDK bridge for Sunaba WASM
 *
 * This module provides JavaScript functions that can be called from Rust WASM
 * using wasm-bindgen. It wraps the SpacetimeDB TypeScript SDK.
 */

// Global state
let spacetimeClient = null;
let isConnectedFlag = false;

/**
 * Connect to SpacetimeDB server
 * @param {string} host - Server URL (e.g., "http://localhost:3000")
 * @param {string} dbName - Database/module name
 * @returns {Promise<void>}
 */
export async function connectSpacetime(host, dbName) {
    try {
        console.log(`[SpacetimeDB] Connecting to ${host}/${dbName}...`);

        // Import SpacetimeDB SDK dynamically
        // NOTE: You need to install @clockworklabs/spacetimedb-sdk via npm
        // and bundle it, or use a CDN link in index.html
        const { SpacetimeDBClient } = await import('@clockworklabs/spacetimedb-sdk');

        // Create client instance
        spacetimeClient = new SpacetimeDBClient(host, dbName);

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
            'SELECT * FROM creature_data'
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
 */
export function updatePlayerPosition(x, y) {
    if (!spacetimeClient) {
        throw new Error('Not connected to SpacetimeDB');
    }

    try {
        spacetimeClient.call('update_player_position', x, y);
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
        spacetimeClient.call('place_material', x, y, materialId);
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
        spacetimeClient.call('mine', x, y);
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
