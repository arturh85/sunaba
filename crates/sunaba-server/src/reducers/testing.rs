//! Test and debug reducers for development

use glam::IVec2;
use spacetimedb::{ReducerContext, Table};

use crate::encoding;
use crate::state::SERVER_WORLD;
use crate::tables::{ChunkData, chunk_data, world_config};

// ============================================================================
// Test/Debug Reducers
// ============================================================================

/// Test terrain generation by generating chunks at ground level
#[spacetimedb::reducer]
pub fn test_terrain_generation(ctx: &ReducerContext) {
    let Some(config) = ctx.db.world_config().id().find(0) else {
        log::error!("World config not found");
        return;
    };

    // Initialize World with proper seed
    let mut world_guard = SERVER_WORLD.lock().unwrap();
    if world_guard.is_none() {
        log::info!("Initializing new World with seed {}", config.seed);
        let mut world = sunaba_core::world::World::new(true);
        world.set_generator(config.seed);
        log::info!("World generator set with seed {}", config.seed);
        *world_guard = Some(world);
    }

    let Some(world) = world_guard.as_mut() else {
        log::error!("Failed to get World");
        return;
    };

    // Generate chunks at ground level (y = -1, 0, 1) where we should see terrain
    log::info!(
        "Testing terrain generation at ground level with seed {}...",
        config.seed
    );

    for chunk_y in -1..=1 {
        for chunk_x in -2..=2 {
            let pos = IVec2::new(chunk_x, chunk_y);

            // Generate chunk
            world.generate_chunk(pos);

            // Get the chunk and count non-air pixels
            if let Some(chunk) = world.get_chunk(chunk_x, chunk_y) {
                let non_air = chunk.count_non_air();
                log::info!(
                    "Generated chunk ({}, {}) - {} non-air pixels",
                    chunk_x,
                    chunk_y,
                    non_air
                );

                // Sample bottom-left corner pixels to see what materials are generated
                let mut sample_materials = Vec::new();
                for y in 0..4 {
                    for x in 0..4 {
                        let pixel = chunk.get_pixel(x, y);
                        sample_materials.push(pixel.material_id);
                    }
                }
                log::info!("  Bottom-left 4x4 material IDs: {:?}", sample_materials);

                // Save to database
                let Ok(pixel_data) = encoding::encode_chunk(chunk) else {
                    log::error!("Failed to encode chunk ({}, {})", chunk_x, chunk_y);
                    continue;
                };

                ctx.db.chunk_data().insert(ChunkData {
                    id: 0,
                    x: chunk_x,
                    y: chunk_y,
                    pixel_data,
                    dirty: true,
                    last_modified_tick: config.tick_count,
                });

                log::info!("Saved chunk ({}, {}) to database", chunk_x, chunk_y);
            }
        }
    }

    log::info!("Terrain generation test complete!");
}
