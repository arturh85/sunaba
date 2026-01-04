//! Mining system - player mining and placement mechanics

use super::chunk_manager::ChunkManager;
use crate::entity::player::Player;
use crate::entity::tools::ToolRegistry;
use crate::simulation::{MaterialId, Materials, mining::calculate_mining_time};

/// Mining system - static utility methods for mining and placement
pub struct MiningSystem;

impl MiningSystem {
    /// Mine a single pixel and add it to player's inventory
    /// Returns true if successfully mined
    pub fn mine_pixel(
        player: &mut Player,
        chunk_manager: &mut ChunkManager,
        world_x: i32,
        world_y: i32,
        materials: &Materials,
    ) -> bool {
        let (chunk_pos, local_x, local_y) = ChunkManager::world_to_chunk_coords(world_x, world_y);

        if let Some(chunk) = chunk_manager.chunks.get_mut(&chunk_pos) {
            let pixel = chunk.get_pixel(local_x, local_y);
            let material_id = pixel.material_id;

            // Can't mine air or bedrock
            if material_id == MaterialId::AIR || material_id == MaterialId::BEDROCK {
                return false;
            }

            // Try to add to inventory
            if player.mine_material(material_id) {
                // Successfully added to inventory, remove the pixel
                chunk.set_material(local_x, local_y, MaterialId::AIR);
                chunk.dirty = true;

                let material_name = &materials.get(material_id).name;
                log::debug!(
                    "[MINE] Mined {} at ({}, {})",
                    material_name,
                    world_x,
                    world_y
                );
                true
            } else {
                log::debug!(
                    "[MINE] Inventory full, can't mine at ({}, {})",
                    world_x,
                    world_y
                );
                false
            }
        } else {
            false
        }
    }

    /// Place material from player's inventory at world coordinates with circular brush
    /// Returns number of pixels successfully placed
    pub fn place_material_from_inventory(
        player: &mut Player,
        chunk_manager: &ChunkManager,
        world_x: i32,
        world_y: i32,
        material_id: u16,
        materials: &Materials,
        brush_size: u32,
    ) -> Vec<(i32, i32)> {
        let material_name = materials.get(material_id).name.clone();
        let mut positions_to_place = Vec::new();

        let brush_radius = brush_size as i32;

        // Calculate positions where we want to place (circular brush)
        for dy in -brush_radius..=brush_radius {
            for dx in -brush_radius..=brush_radius {
                if dx * dx + dy * dy <= brush_radius * brush_radius {
                    let x = world_x + dx;
                    let y = world_y + dy;
                    // Only count if target pixel is air (can be replaced)
                    let (chunk_pos, local_x, local_y) = ChunkManager::world_to_chunk_coords(x, y);
                    if let Some(chunk) = chunk_manager.chunks.get(&chunk_pos) {
                        let pixel = chunk.get_pixel(local_x, local_y);
                        if pixel.material_id == MaterialId::AIR {
                            positions_to_place.push((x, y));
                        }
                    }
                }
            }
        }

        if positions_to_place.is_empty() {
            return Vec::new();
        }

        let pixels_needed = positions_to_place.len() as u32;

        // Check if player has enough material
        if !player.inventory.has_item(material_id, pixels_needed) {
            log::debug!(
                "[PLACE] Not enough {} in inventory (need {}, have {})",
                material_name,
                pixels_needed,
                player.inventory.count_item(material_id)
            );
            return Vec::new();
        }

        // Consume from inventory
        let consumed = player.inventory.remove_item(material_id, pixels_needed);

        log::debug!(
            "[PLACE] Ready to place {} {} pixels at ({}, {}), consumed {} from inventory",
            positions_to_place.len(),
            material_name,
            world_x,
            world_y,
            consumed
        );

        positions_to_place
    }

    /// Place material at world coordinates without consuming from inventory (debug mode)
    /// Returns positions where material should be placed
    pub fn place_material_debug(
        chunk_manager: &ChunkManager,
        world_x: i32,
        world_y: i32,
        _material_id: u16,
        brush_size: u32,
    ) -> Vec<(i32, i32)> {
        let mut positions_to_place = Vec::new();

        let brush_radius = brush_size as i32;

        for dy in -brush_radius..=brush_radius {
            for dx in -brush_radius..=brush_radius {
                if dx * dx + dy * dy <= brush_radius * brush_radius {
                    let x = world_x + dx;
                    let y = world_y + dy;
                    let (chunk_pos, local_x, local_y) = ChunkManager::world_to_chunk_coords(x, y);
                    if let Some(chunk) = chunk_manager.chunks.get(&chunk_pos) {
                        let pixel = chunk.get_pixel(local_x, local_y);
                        if pixel.material_id == MaterialId::AIR {
                            positions_to_place.push((x, y));
                        }
                    }
                }
            }
        }

        positions_to_place
    }

    /// Start mining a pixel (calculates required time based on material hardness and tool)
    pub fn start_mining(
        player: &mut Player,
        chunk_manager: &ChunkManager,
        world_x: i32,
        world_y: i32,
        materials: &Materials,
        tool_registry: &ToolRegistry,
    ) {
        // Get the pixel
        let (chunk_pos, local_x, local_y) = ChunkManager::world_to_chunk_coords(world_x, world_y);
        let pixel = match chunk_manager.chunks.get(&chunk_pos) {
            Some(chunk) => chunk.get_pixel(local_x, local_y),
            None => return, // Out of bounds
        };

        let material = materials.get(pixel.material_id);

        // Can't mine air or materials without hardness (bedrock)
        if material.hardness.is_none() {
            return;
        }

        // Get equipped tool
        let tool = player.get_equipped_tool(tool_registry);

        // Calculate mining time
        let required_time = calculate_mining_time(1.0, material, tool);

        // Start mining
        player
            .mining_progress
            .start((world_x, world_y), required_time);

        log::debug!(
            "[MINING] Started mining {} at ({}, {}) - required time: {:.2}s (tool: {:?})",
            material.name,
            world_x,
            world_y,
            required_time,
            tool.map(|t| t.name.as_str())
        );
    }

    /// Update mining progress (called each frame)
    /// Returns Some((x, y)) if mining completed this frame with the target pixel coordinates
    pub fn update_mining(player: &mut Player, delta_time: f32) -> Option<(i32, i32)> {
        if player.update_mining(delta_time) {
            // Mining completed
            player.mining_progress.target_pixel
        } else {
            None
        }
    }

    /// Complete mining at the specified position
    /// Returns Some(material_id) if successfully mined, None otherwise
    pub fn complete_mining(
        player: &mut Player,
        chunk_manager: &ChunkManager,
        world_x: i32,
        world_y: i32,
        materials: &Materials,
        tool_registry: &ToolRegistry,
    ) -> Option<u16> {
        // Get the pixel
        let (chunk_pos, local_x, local_y) = ChunkManager::world_to_chunk_coords(world_x, world_y);
        let pixel = match chunk_manager.chunks.get(&chunk_pos) {
            Some(chunk) => chunk.get_pixel(local_x, local_y),
            None => {
                log::warn!(
                    "[MINING] Complete mining failed: pixel at ({}, {}) not found",
                    world_x,
                    world_y
                );
                return None;
            }
        };

        let material_id = pixel.material_id;
        let material_name = materials.get(material_id).name.clone();

        // Add to inventory
        if player.mine_material(material_id) {
            // Damage tool durability
            if let Some(tool_id) = player.equipped_tool {
                let broke = player.inventory.damage_tool(tool_id, 1);
                if broke {
                    let tool_name = tool_registry
                        .get(tool_id)
                        .map(|t| t.name.as_str())
                        .unwrap_or("Unknown");
                    log::info!("[MINING] {} broke!", tool_name);
                    player.unequip_tool();
                }
            }

            log::debug!(
                "[MINING] Completed mining {} at ({}, {})",
                material_name,
                world_x,
                world_y
            );

            Some(material_id)
        } else {
            log::warn!(
                "[MINING] Failed to add {} to inventory (full?)",
                material_name
            );
            None
        }
    }

    /// DEBUG: Instantly mine all materials in a circle around position
    /// Used for quick world exploration during testing
    /// Returns positions to mine
    pub fn debug_mine_circle(
        player: &mut Player,
        chunk_manager: &ChunkManager,
        center_x: i32,
        center_y: i32,
        radius: i32,
        materials: &Materials,
    ) -> Vec<(i32, i32)> {
        let mut positions_to_mine = Vec::new();

        // Iterate over square containing circle
        for dy in -radius..=radius {
            for dx in -radius..=radius {
                // Check if point is inside circle (Euclidean distance)
                if dx * dx + dy * dy <= radius * radius {
                    let x = center_x + dx;
                    let y = center_y + dy;

                    // Get pixel material
                    let (chunk_pos, local_x, local_y) = ChunkManager::world_to_chunk_coords(x, y);
                    if let Some(chunk) = chunk_manager.chunks.get(&chunk_pos) {
                        let pixel = chunk.get_pixel(local_x, local_y);
                        let material_id = pixel.material_id;

                        // Skip air and bedrock
                        if material_id == MaterialId::AIR {
                            continue;
                        }

                        let material = materials.get(material_id);
                        if material.hardness.is_none() {
                            continue; // Bedrock/unmineable materials
                        }

                        // Add to inventory and mark for removal
                        if player.mine_material(material_id) {
                            positions_to_mine.push((x, y));
                        }
                    }
                }
            }
        }

        log::debug!(
            "[DEBUG MINING] Mined {} pixels at ({}, {}) with radius {}",
            positions_to_mine.len(),
            center_x,
            center_y,
            radius
        );

        positions_to_mine
    }
}
