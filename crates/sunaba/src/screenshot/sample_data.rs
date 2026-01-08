//! Sample data generators for UI screenshot testing
//!
//! Provides realistic test data to populate UI panels for screenshot capture.
//! Sample data includes diverse inventory items, tools with varying durability,
//! and partial health/hunger to demonstrate all UI features.

use glam::Vec2;
use sunaba_core::{
    entity::{inventory::ItemStack, player::Player},
    simulation::MaterialId,
};

/// Create a sample player with populated inventory for screenshot testing
///
/// Returns a player with:
/// - Diverse material types (ores, processed materials, natural resources)
/// - Multiple tools with varying durability
/// - Partial health and hunger to show bars
/// - Selected slot 0 with wood pickaxe equipped
pub fn create_sample_player_with_inventory() -> Player {
    let mut player = Player::new(Vec2::ZERO);

    // Clear default inventory (Player::new() gives starting materials)
    // We'll repopulate with diverse items for screenshot demonstration
    while let Some(_) = player.inventory.slots.iter_mut().find(|s| s.is_some()) {
        for slot in player.inventory.slots.iter_mut() {
            *slot = None;
        }
    }

    // Add diverse materials using add_item (automatically stacks and fills slots)
    let _ = player.inventory.add_item(MaterialId::SAND, 500);
    let _ = player.inventory.add_item(MaterialId::WOOD, 150);
    let _ = player.inventory.add_item(MaterialId::STONE, 250);
    let _ = player.inventory.add_item(MaterialId::COAL_ORE, 100);
    let _ = player.inventory.add_item(MaterialId::IRON_ORE, 50);
    let _ = player.inventory.add_item(MaterialId::COPPER_ORE, 35);
    let _ = player.inventory.add_item(MaterialId::GOLD_ORE, 20);
    let _ = player.inventory.add_item(MaterialId::PLANT_MATTER, 75);
    let _ = player.inventory.add_item(MaterialId::ASH, 30);
    let _ = player.inventory.add_item(MaterialId::FERTILIZER, 15);

    // Add tools manually to specific slots (tools don't stack)
    // Find first empty slot and add tools
    let mut next_slot = 0;
    for (i, slot) in player.inventory.slots.iter().enumerate() {
        if slot.is_none() {
            next_slot = i;
            break;
        }
    }

    // Add Wood Pickaxe (50% durability)
    if next_slot < player.inventory.max_slots {
        player.inventory.slots[next_slot] = Some(ItemStack::new_tool(1000, 25)); // 25/50 durability
        next_slot += 1;
    }

    // Add Stone Pickaxe (80% durability)
    if next_slot < player.inventory.max_slots {
        player.inventory.slots[next_slot] = Some(ItemStack::new_tool(1001, 80)); // 80/100 durability
        next_slot += 1;
    }

    // Add Iron Pickaxe (100% durability - brand new)
    if next_slot < player.inventory.max_slots {
        player.inventory.slots[next_slot] = Some(ItemStack::new_tool(1002, 400)); // 400/400 durability
    }

    // Set selected slot and equipped tool
    player.selected_slot = 0;
    player.equipped_tool = Some(1000); // Wood pickaxe equipped

    // Set partial health and hunger for visual interest
    player.health.set(75.0); // 75/100
    player.hunger.set(60.0); // 60/100

    player
}
