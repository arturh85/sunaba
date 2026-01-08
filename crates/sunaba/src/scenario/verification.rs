//! Verification conditions and state checks for scenarios

use serde::{Deserialize, Serialize};
use sunaba_core::entity::inventory::ItemStack;
use sunaba_core::simulation::MaterialId;
use sunaba_core::world::World;

/// Conditions that can be verified against world state
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum VerificationCondition {
    // === MATERIAL CHECKS ===
    /// Assert exact material count in region
    MaterialCount {
        material: u16,
        region: Region,
        expected: usize,
        tolerance: Option<usize>, // Allow ±N variance
    },

    /// Assert material count within range
    MaterialCountRange {
        material: u16,
        region: Region,
        min: usize,
        max: usize,
    },

    /// Assert material exists at specific pixel
    MaterialAt { x: i32, y: i32, expected: u16 },

    /// Assert no material in region (all air)
    RegionEmpty { region: Region },

    /// Assert region is filled (no air)
    RegionFilled { region: Region },

    // === PLAYER STATE CHECKS ===
    /// Assert player position (with tolerance)
    PlayerPosition { x: f32, y: f32, tolerance: f32 },

    /// Assert player position in region
    PlayerInRegion { region: Region },

    /// Assert player health
    PlayerHealth {
        expected: f32,
        tolerance: Option<f32>,
    },

    /// Assert player velocity
    PlayerVelocity { vx: f32, vy: f32, tolerance: f32 },

    /// Assert player is grounded
    PlayerGrounded { expected: bool },

    /// Assert player inventory slot
    InventorySlot {
        slot: usize,
        expected: Option<ItemStack>,
    },

    /// Assert player has item (any slot)
    HasItem {
        material: Option<u16>,
        tool: Option<u16>,
        min_count: usize,
    },

    // === PHYSICS CHECKS ===
    /// Assert temperature in region
    TemperatureRange { region: Region, min: f32, max: f32 },

    /// Assert structural integrity (no debris)
    NoDebrisIn { region: Region },

    /// Assert light level in region
    LightLevel { region: Region, min: u8, max: u8 },

    // === CREATURE CHECKS ===
    /// Assert creature count
    CreatureCount {
        expected: usize,
        tolerance: Option<usize>,
    },

    /// Assert creature exists in region
    CreatureInRegion { region: Region },

    // === LOGICAL OPERATORS ===
    /// All conditions must pass
    All {
        conditions: Vec<VerificationCondition>,
    },

    /// Any condition must pass
    Any {
        conditions: Vec<VerificationCondition>,
    },

    /// Condition must NOT pass
    Not {
        condition: Box<VerificationCondition>,
    },
}

/// Spatial region for verification
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Region {
    /// Rectangular region
    Rect {
        min_x: i32,
        min_y: i32,
        max_x: i32,
        max_y: i32,
    },

    /// Circular region
    Circle {
        center_x: i32,
        center_y: i32,
        radius: u32,
    },

    /// Entire world (all loaded chunks)
    Whole,

    /// Active chunks only
    ActiveChunks,
}

/// Result of a verification check
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationResult {
    pub passed: bool,
    pub message: String,
    pub actual_value: Option<String>, // For debugging
}

impl VerificationCondition {
    /// Evaluate condition against world state
    pub fn evaluate(&self, world: &World) -> VerificationResult {
        match self {
            VerificationCondition::MaterialCount {
                material,
                region,
                expected,
                tolerance,
            } => {
                let actual = count_material_in_region(world, *material, region);
                let tol = tolerance.unwrap_or(0);
                let passed = actual >= expected.saturating_sub(tol) && actual <= expected + tol;

                VerificationResult {
                    passed,
                    message: format!(
                        "Material {:?} count in {:?}: expected {}±{}, got {}",
                        material, region, expected, tol, actual
                    ),
                    actual_value: Some(actual.to_string()),
                }
            }

            VerificationCondition::MaterialCountRange {
                material,
                region,
                min,
                max,
            } => {
                let actual = count_material_in_region(world, *material, region);
                let passed = actual >= *min && actual <= *max;

                VerificationResult {
                    passed,
                    message: format!(
                        "Material {:?} count in {:?}: expected {}-{}, got {}",
                        material, region, min, max, actual
                    ),
                    actual_value: Some(actual.to_string()),
                }
            }

            VerificationCondition::MaterialAt { x, y, expected } => {
                let actual = world
                    .get_pixel(*x, *y)
                    .map(|p| p.material_id)
                    .unwrap_or(MaterialId::AIR);
                let passed = actual == *expected;

                VerificationResult {
                    passed,
                    message: format!(
                        "Material at ({}, {}): expected {:?}, got {:?}",
                        x, y, expected, actual
                    ),
                    actual_value: Some(format!("{:?}", actual)),
                }
            }

            VerificationCondition::RegionEmpty { region } => {
                let air_count = count_material_in_region(world, MaterialId::AIR, region);
                let total = count_pixels_in_region(world, region);
                let passed = air_count == total;

                VerificationResult {
                    passed,
                    message: format!(
                        "Region {:?} empty: {} air / {} total pixels",
                        region, air_count, total
                    ),
                    actual_value: Some(format!("{}/{}", air_count, total)),
                }
            }

            VerificationCondition::RegionFilled { region } => {
                let air_count = count_material_in_region(world, MaterialId::AIR, region);
                let passed = air_count == 0;

                VerificationResult {
                    passed,
                    message: format!("Region {:?} filled: {} air pixels", region, air_count),
                    actual_value: Some(air_count.to_string()),
                }
            }

            VerificationCondition::PlayerPosition { x, y, tolerance } => {
                let pos = world.player.position;
                let dist = ((pos.x - x).powi(2) + (pos.y - y).powi(2)).sqrt();
                let passed = dist <= *tolerance;

                VerificationResult {
                    passed,
                    message: format!(
                        "Player position: expected ({}, {}) ±{}, got ({:.1}, {:.1}), distance {:.1}",
                        x, y, tolerance, pos.x, pos.y, dist
                    ),
                    actual_value: Some(format!("({:.1}, {:.1})", pos.x, pos.y)),
                }
            }

            VerificationCondition::PlayerInRegion { region } => {
                let pos = world.player.position;
                let in_region = match region {
                    Region::Rect {
                        min_x,
                        min_y,
                        max_x,
                        max_y,
                    } => {
                        pos.x >= *min_x as f32
                            && pos.x <= *max_x as f32
                            && pos.y >= *min_y as f32
                            && pos.y <= *max_y as f32
                    }
                    Region::Circle {
                        center_x,
                        center_y,
                        radius,
                    } => {
                        let dx = pos.x - *center_x as f32;
                        let dy = pos.y - *center_y as f32;
                        (dx * dx + dy * dy).sqrt() <= *radius as f32
                    }
                    Region::Whole | Region::ActiveChunks => true,
                };

                VerificationResult {
                    passed: in_region,
                    message: format!(
                        "Player in region {:?}: ({:.1}, {:.1}) {}",
                        region,
                        pos.x,
                        pos.y,
                        if in_region { "inside" } else { "outside" }
                    ),
                    actual_value: Some(format!("({:.1}, {:.1})", pos.x, pos.y)),
                }
            }

            VerificationCondition::PlayerHealth {
                expected,
                tolerance,
            } => {
                let actual = world.player.health.current;
                let tol = tolerance.unwrap_or(0.0);
                let passed = (actual - expected).abs() <= tol;

                VerificationResult {
                    passed,
                    message: format!(
                        "Player health: expected {}±{}, got {:.1}",
                        expected, tol, actual
                    ),
                    actual_value: Some(format!("{:.1}", actual)),
                }
            }

            VerificationCondition::PlayerGrounded { expected } => {
                let actual = world.player.grounded;
                let passed = actual == *expected;

                VerificationResult {
                    passed,
                    message: format!("Player grounded: expected {}, got {}", expected, actual),
                    actual_value: Some(actual.to_string()),
                }
            }

            VerificationCondition::PlayerVelocity { vx, vy, tolerance } => {
                let vel = world.player.velocity;
                let dist = ((vel.x - vx).powi(2) + (vel.y - vy).powi(2)).sqrt();
                let passed = dist <= *tolerance;

                VerificationResult {
                    passed,
                    message: format!(
                        "Player velocity: expected ({}, {}) ±{}, got ({:.1}, {:.1}), distance {:.1}",
                        vx, vy, tolerance, vel.x, vel.y, dist
                    ),
                    actual_value: Some(format!("({:.1}, {:.1})", vel.x, vel.y)),
                }
            }

            VerificationCondition::All { conditions } => {
                let mut all_passed = true;
                let mut messages = Vec::new();

                for cond in conditions {
                    let result = cond.evaluate(world);
                    if !result.passed {
                        all_passed = false;
                    }
                    messages.push(format!("  - {}", result.message));
                }

                VerificationResult {
                    passed: all_passed,
                    message: format!("All conditions:\n{}", messages.join("\n")),
                    actual_value: None,
                }
            }

            VerificationCondition::Any { conditions } => {
                let mut any_passed = false;
                let mut messages = Vec::new();

                for cond in conditions {
                    let result = cond.evaluate(world);
                    if result.passed {
                        any_passed = true;
                    }
                    messages.push(format!("  - {}", result.message));
                }

                VerificationResult {
                    passed: any_passed,
                    message: format!("Any condition:\n{}", messages.join("\n")),
                    actual_value: None,
                }
            }

            VerificationCondition::Not { condition } => {
                let result = condition.evaluate(world);
                VerificationResult {
                    passed: !result.passed,
                    message: format!("NOT ({})", result.message),
                    actual_value: result.actual_value,
                }
            }

            VerificationCondition::InventorySlot { slot, expected } => {
                // get_slot returns Option<&Option<ItemStack>>, flatten to Option<ItemStack>
                let actual = world
                    .player
                    .inventory
                    .get_slot(*slot)
                    .and_then(|opt| opt.clone());

                // Compare Option<ItemStack>
                let passed = actual == *expected;

                VerificationResult {
                    passed,
                    message: format!(
                        "Inventory slot {}: expected {:?}, got {:?}",
                        slot, expected, actual
                    ),
                    actual_value: actual.as_ref().map(|item| format!("{:?}", item)),
                }
            }

            VerificationCondition::HasItem {
                material,
                tool,
                min_count,
            } => {
                let count = if let Some(mat_id) = material {
                    world.player.inventory.count_item(*mat_id) as usize
                } else if let Some(tool_id) = tool {
                    if world
                        .player
                        .inventory
                        .get_tool_durability(*tool_id)
                        .is_some()
                    {
                        1
                    } else {
                        0
                    }
                } else {
                    0
                };

                let passed = count >= *min_count;

                VerificationResult {
                    passed,
                    message: format!(
                        "Has item (material: {:?}, tool: {:?}): expected {}, got {}",
                        material, tool, min_count, count
                    ),
                    actual_value: Some(count.to_string()),
                }
            }

            VerificationCondition::CreatureCount {
                expected,
                tolerance,
            } => {
                let actual = world.creature_manager.count();
                let tol = tolerance.unwrap_or(0);
                let passed = actual >= expected.saturating_sub(tol) && actual <= expected + tol;

                VerificationResult {
                    passed,
                    message: format!(
                        "Creature count: expected {}±{}, got {}",
                        expected, tol, actual
                    ),
                    actual_value: Some(actual.to_string()),
                }
            }

            VerificationCondition::CreatureInRegion { region } => {
                // Get all creature positions
                let positions = world.creature_manager.get_positions();

                // Check if any creature is in region
                let in_region = positions.iter().any(|pos| match region {
                    Region::Rect {
                        min_x,
                        min_y,
                        max_x,
                        max_y,
                    } => {
                        pos.x >= *min_x as f32
                            && pos.x <= *max_x as f32
                            && pos.y >= *min_y as f32
                            && pos.y <= *max_y as f32
                    }
                    Region::Circle {
                        center_x,
                        center_y,
                        radius,
                    } => {
                        let dx = pos.x - *center_x as f32;
                        let dy = pos.y - *center_y as f32;
                        (dx * dx + dy * dy).sqrt() <= *radius as f32
                    }
                    Region::Whole | Region::ActiveChunks => true,
                });

                VerificationResult {
                    passed: in_region,
                    message: format!(
                        "Creature in region {:?}: {} found",
                        region,
                        if in_region { "creature" } else { "no creature" }
                    ),
                    actual_value: Some(format!("{} creatures total", positions.len())),
                }
            }

            // Stub implementations for less critical verifications
            _ => VerificationResult {
                passed: false,
                message: format!("Verification not yet implemented: {:?}", self),
                actual_value: None,
            },
        }
    }
}

// Helper functions

/// Count material in region
fn count_material_in_region(world: &World, material: u16, region: &Region) -> usize {
    match region {
        Region::Rect {
            min_x,
            min_y,
            max_x,
            max_y,
        } => {
            let mut count = 0;
            for y in *min_y..=*max_y {
                for x in *min_x..=*max_x {
                    if let Some(pixel) = world.get_pixel(x, y) {
                        if pixel.material_id == material {
                            count += 1;
                        }
                    }
                }
            }
            count
        }
        Region::Circle {
            center_x,
            center_y,
            radius,
        } => {
            let mut count = 0;
            let r = *radius as i32;
            for y in (center_y - r)..=(center_y + r) {
                for x in (center_x - r)..=(center_x + r) {
                    let dx = x - center_x;
                    let dy = y - center_y;
                    if (dx * dx + dy * dy) <= (r * r) {
                        if let Some(pixel) = world.get_pixel(x, y) {
                            if pixel.material_id == material {
                                count += 1;
                            }
                        }
                    }
                }
            }
            count
        }
        Region::Whole => {
            // Count across all loaded chunks
            world
                .chunks()
                .values()
                .flat_map(|chunk| chunk.pixels().iter())
                .filter(|p| p.material_id == material)
                .count()
        }
        Region::ActiveChunks => {
            // Count only in active chunks
            world
                .active_chunk_positions()
                .iter()
                .filter_map(|pos| world.get_chunk(pos.x, pos.y))
                .flat_map(|chunk| chunk.pixels().iter())
                .filter(|p| p.material_id == material)
                .count()
        }
    }
}

/// Count total pixels in region
fn count_pixels_in_region(world: &World, region: &Region) -> usize {
    match region {
        Region::Rect {
            min_x,
            min_y,
            max_x,
            max_y,
        } => {
            let mut count = 0;
            for y in *min_y..=*max_y {
                for x in *min_x..=*max_x {
                    if world.get_pixel(x, y).is_some() {
                        count += 1;
                    }
                }
            }
            count
        }
        Region::Circle {
            center_x,
            center_y,
            radius,
        } => {
            let mut count = 0;
            let r = *radius as i32;
            for y in (center_y - r)..=(center_y + r) {
                for x in (center_x - r)..=(center_x + r) {
                    let dx = x - center_x;
                    let dy = y - center_y;
                    if (dx * dx + dy * dy) <= (r * r) {
                        if world.get_pixel(x, y).is_some() {
                            count += 1;
                        }
                    }
                }
            }
            count
        }
        Region::Whole => world
            .chunks()
            .values()
            .flat_map(|chunk| chunk.pixels().iter())
            .count(),
        Region::ActiveChunks => world
            .active_chunk_positions()
            .iter()
            .filter_map(|pos| world.get_chunk(pos.x, pos.y))
            .flat_map(|chunk| chunk.pixels().iter())
            .count(),
    }
}
