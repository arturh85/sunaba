//! Mining knockback calculations
//!
//! Provides realistic knockback feedback when mining materials based on:
//! - Material hardness (harder materials = stronger knockback)
//! - Tool efficiency (better tools = more kickback)
//! - Distance from mining point (closer = stronger)

use glam::Vec2;

/// Calculate knockback velocity impulse for mining a material
///
/// # Formula
/// ```text
/// velocity = base_impulse × hardness_factor × tool_mult × distance_falloff
/// ```
///
/// # Parameters
/// - `player_pos` - Player's current position
/// - `mining_pos` - World position being mined (as Vec2)
/// - `material_hardness` - Material hardness on 1-8 scale (None = air, no knockback)
/// - `tool_speed_multiplier` - Tool tier speed (1.0 = wood, 4.0 = steel)
///
/// # Returns
/// Velocity impulse to apply to player (pixels/second)
pub fn calculate_mining_knockback(
    player_pos: Vec2,
    mining_pos: Vec2,
    material_hardness: Option<u8>,
    tool_speed_multiplier: f32,
) -> Vec2 {
    // No knockback for air or missing material
    let hardness = match material_hardness {
        Some(h) if h > 0 => h,
        _ => return Vec2::ZERO,
    };

    // Base knockback (px/s) - baseline for medium-hardness stone
    const BASE_IMPULSE: f32 = 20.0;

    // Hardness factor: scale by material hardness (1-8 scale → 0.2x to 1.6x)
    // Harder materials give more kickback
    let hardness_factor = hardness as f32 / 5.0;

    // Tool multiplier: better tools hit harder, more recoil
    // Formula: 1.0 + (tool_speed - 1.0) × 0.33
    // Wood (1.0x) → 1.0x knockback
    // Stone (1.5x) → 1.165x knockback
    // Iron (2.0x) → 1.33x knockback
    // Steel (4.0x) → 2.0x knockback
    let tool_mult = 1.0 + (tool_speed_multiplier - 1.0) * 0.33;

    // Distance falloff: closer mining = stronger knockback
    // Formula: 1.0 / (1.0 + distance / 16.0)
    // At 0px: 1.0x
    // At 16px: 0.5x
    // At 48px: 0.25x
    let distance = player_pos.distance(mining_pos);
    let distance_falloff = 1.0 / (1.0 + distance / 16.0);

    // Direction: away from mining point
    let direction = (player_pos - mining_pos).normalize_or_zero();

    // Calculate final knockback velocity
    let magnitude = BASE_IMPULSE * hardness_factor * tool_mult * distance_falloff;
    direction * magnitude
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_no_knockback_for_air() {
        let knockback = calculate_mining_knockback(
            Vec2::new(0.0, 0.0),
            Vec2::new(10.0, 0.0),
            None, // Air
            1.0,
        );
        assert_eq!(knockback, Vec2::ZERO);
    }

    #[test]
    fn test_no_knockback_for_zero_hardness() {
        let knockback =
            calculate_mining_knockback(Vec2::new(0.0, 0.0), Vec2::new(10.0, 0.0), Some(0), 1.0);
        assert_eq!(knockback, Vec2::ZERO);
    }

    #[test]
    fn test_knockback_direction() {
        // Mining to the right should push player left
        let knockback = calculate_mining_knockback(
            Vec2::new(0.0, 0.0),
            Vec2::new(10.0, 0.0),
            Some(5), // Medium hardness
            1.0,     // Wood tool
        );
        assert!(knockback.x < 0.0, "Should push player left (negative X)");
        assert_eq!(knockback.y, 0.0, "No Y component for horizontal mining");
    }

    #[test]
    fn test_hardness_scaling() {
        let player_pos = Vec2::new(0.0, 0.0);
        let mining_pos = Vec2::new(10.0, 0.0);
        let tool_speed = 1.0;

        // Soft material (hardness 1)
        let soft = calculate_mining_knockback(player_pos, mining_pos, Some(1), tool_speed);

        // Hard material (hardness 8)
        let hard = calculate_mining_knockback(player_pos, mining_pos, Some(8), tool_speed);

        // Hard material should give 8x more knockback
        assert!(
            hard.length() > soft.length() * 7.5,
            "Hard materials should give much stronger knockback"
        );
    }

    #[test]
    fn test_tool_multiplier() {
        let player_pos = Vec2::new(0.0, 0.0);
        let mining_pos = Vec2::new(10.0, 0.0);
        let hardness = Some(5);

        // Wood tool (1.0x)
        let wood = calculate_mining_knockback(player_pos, mining_pos, hardness, 1.0);

        // Steel tool (4.0x)
        let steel = calculate_mining_knockback(player_pos, mining_pos, hardness, 4.0);

        // Steel should give ~2x more knockback (tool_mult = 2.0)
        assert!(
            steel.length() > wood.length() * 1.9,
            "Steel tools should give ~2x knockback"
        );
        assert!(
            steel.length() < wood.length() * 2.1,
            "Steel tools should give ~2x knockback"
        );
    }

    #[test]
    fn test_distance_falloff() {
        let player_pos = Vec2::new(0.0, 0.0);
        let hardness = Some(5);
        let tool_speed = 1.0;

        // Close mining (10px away)
        let close =
            calculate_mining_knockback(player_pos, Vec2::new(10.0, 0.0), hardness, tool_speed);

        // Far mining (50px away)
        let far =
            calculate_mining_knockback(player_pos, Vec2::new(50.0, 0.0), hardness, tool_speed);

        assert!(
            close.length() > far.length(),
            "Closer mining should give stronger knockback"
        );
    }

    #[test]
    fn test_realistic_values() {
        // Typical scenario: mining stone (hardness 5) with wood tool (1.0x) at 10px
        let knockback = calculate_mining_knockback(
            Vec2::new(0.0, 0.0),
            Vec2::new(10.0, 0.0),
            Some(5), // Stone
            1.0,     // Wood tool
        );

        // Expected: ~15-25 px/s knockback (noticeable but not overwhelming)
        let magnitude = knockback.length();
        assert!(magnitude > 10.0, "Knockback should be noticeable");
        assert!(magnitude < 30.0, "Knockback should not be overwhelming");
    }

    #[test]
    fn test_zero_distance_safe() {
        // Edge case: mining exactly at player position
        let knockback =
            calculate_mining_knockback(Vec2::new(0.0, 0.0), Vec2::new(0.0, 0.0), Some(5), 1.0);
        // normalize_or_zero() should handle this gracefully
        assert!(knockback.is_finite());
    }
}
