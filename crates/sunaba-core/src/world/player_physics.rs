//! Player physics system - movement, gravity, jumping, collision

use crate::entity::input::InputState;
use crate::entity::player::Player;
use glam::Vec2;

/// Player physics system - handles movement, jumping, gravity, collision
pub struct PlayerPhysicsSystem;

impl PlayerPhysicsSystem {
    /// Update player physics for one frame
    ///
    /// Callbacks:
    /// - `is_grounded`: Check if player is on ground (queries world collision)
    /// - `check_collision`: Check if position+size collides with solid (AABB check)
    ///
    /// # Arguments
    /// * `player` - Mutable reference to player state
    /// * `input` - Player input state (keyboard/controller)
    /// * `dt` - Delta time in seconds
    /// * `player_speed` - Horizontal movement speed in pixels/second
    /// * `is_grounded` - Callback to check if player is on ground
    /// * `check_collision` - Callback to check AABB collision with world
    pub fn update<F, G>(
        player: &mut Player,
        input: &InputState,
        dt: f32,
        player_speed: f32,
        is_grounded: F,
        check_collision: G,
    ) where
        F: Fn() -> bool,
        G: Fn(f32, f32, f32, f32) -> bool,
    {
        // 1. Check if grounded
        player.grounded = is_grounded();

        // 2. Update coyote time (grace period for jumping after leaving ground)
        if player.grounded {
            player.coyote_time = Player::COYOTE_TIME;
        } else {
            player.coyote_time = (player.coyote_time - dt).max(0.0);
        }

        // 3. Update jump buffer (allows jump input slightly before landing)
        if input.jump_pressed {
            player.jump_buffer = Player::JUMP_BUFFER;
        } else {
            player.jump_buffer = (player.jump_buffer - dt).max(0.0);
        }

        // 4. Horizontal movement (A/D keys) with friction
        const PLAYER_DECELERATION: f32 = 800.0; // px/s² (friction when no input)

        let mut horizontal_input = 0.0;
        if input.a_pressed {
            horizontal_input -= 1.0;
        }
        if input.d_pressed {
            horizontal_input += 1.0;
        }

        if horizontal_input != 0.0 {
            // Apply movement input
            player.velocity.x = horizontal_input * player_speed;
        } else if player.grounded {
            // Apply friction when grounded and no input
            let friction = PLAYER_DECELERATION * dt;
            if player.velocity.x.abs() < friction {
                player.velocity.x = 0.0;
            } else {
                player.velocity.x -= player.velocity.x.signum() * friction;
            }
        }
        // Note: No friction in air - preserve momentum for better jump control

        // 5. Vertical movement (gravity + jump + flight)
        if player.jump_buffer > 0.0 && player.coyote_time > 0.0 {
            // Jump!
            player.velocity.y = Player::JUMP_VELOCITY;
            player.jump_buffer = 0.0;
            player.coyote_time = 0.0;
            log::debug!("Player jumped!");
        } else if !player.grounded {
            // Apply flight thrust if W pressed (Noita-style levitation)
            if input.w_pressed {
                player.velocity.y += Player::FLIGHT_THRUST * dt;
            }
            // Apply gravity when airborne
            player.velocity.y -= Player::GRAVITY * dt;
            // Clamp to terminal velocity (both up and down)
            player.velocity.y = player
                .velocity
                .y
                .clamp(-Player::MAX_FALL_SPEED, Player::MAX_FALL_SPEED);
        } else {
            // Grounded and not jumping - reset vertical velocity
            player.velocity.y = 0.0;
        }

        // 6. Integrate velocity into position with collision
        let movement = player.velocity * dt;

        // Check collision separately for X and Y
        let new_x = player.position.x + movement.x;
        let new_y = player.position.y + movement.y;

        let can_move_x = !check_collision(new_x, player.position.y, Player::WIDTH, Player::HEIGHT);

        let can_move_y = !check_collision(player.position.x, new_y, Player::WIDTH, Player::HEIGHT);

        // Apply movement only on non-colliding axes
        let final_movement = Vec2::new(
            if can_move_x { movement.x } else { 0.0 },
            if can_move_y { movement.y } else { 0.0 },
        );

        // Stop vertical velocity if hit ceiling/floor
        if !can_move_y {
            player.velocity.y = 0.0;
        }

        // 7. Automatic unstuck mechanic - nudge player out of tight spaces if completely stuck
        if !can_move_x
            && !can_move_y
            && (input.a_pressed || input.d_pressed || input.w_pressed || input.s_pressed)
        {
            // Try small position adjustments to unstuck player
            const UNSTUCK_OFFSET: f32 = 0.5;
            let unstuck_attempts = [
                (UNSTUCK_OFFSET, 0.0),
                (-UNSTUCK_OFFSET, 0.0),
                (0.0, UNSTUCK_OFFSET),
                (0.0, -UNSTUCK_OFFSET),
            ];

            for (dx, dy) in unstuck_attempts {
                let test_x = player.position.x + dx;
                let test_y = player.position.y + dy;
                if !check_collision(test_x, test_y, Player::WIDTH, Player::HEIGHT) {
                    player.position.x = test_x;
                    player.position.y = test_y;
                    log::debug!("Player unstuck: nudged ({}, {})", dx, dy);
                    return; // Exit early after unstucking
                }
            }
        }

        if final_movement.length() > 0.0 {
            log::trace!(
                "Player: {:?} → {:?} (vel: {:?}, grounded: {})",
                player.position,
                player.position + final_movement,
                player.velocity,
                player.grounded
            );
        }

        player.move_by(final_movement);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entity::player::Player;
    use glam::Vec2;

    fn make_test_player() -> Player {
        Player::new(Vec2::new(100.0, 100.0))
    }

    fn make_test_input() -> InputState {
        InputState::new()
    }

    #[test]
    fn test_player_grounded_detection() {
        let mut player = make_test_player();
        let input = make_test_input();
        let dt = 1.0 / 60.0;

        // Test grounded player
        PlayerPhysicsSystem::update(
            &mut player,
            &input,
            dt,
            200.0,
            || true, // is_grounded
            |_, _, _, _| false, // no collision
        );

        assert!(player.grounded);
        assert_eq!(player.coyote_time, Player::COYOTE_TIME);
    }

    #[test]
    fn test_player_not_grounded() {
        let mut player = make_test_player();
        let input = make_test_input();
        let dt = 1.0 / 60.0;

        PlayerPhysicsSystem::update(
            &mut player,
            &input,
            dt,
            200.0,
            || false, // not grounded
            |_, _, _, _| false,
        );

        assert!(!player.grounded);
    }

    #[test]
    fn test_coyote_time_decreases() {
        let mut player = make_test_player();
        let input = make_test_input();
        let dt = 0.05; // 50ms

        // First frame on ground
        PlayerPhysicsSystem::update(
            &mut player,
            &input,
            dt,
            200.0,
            || true,
            |_, _, _, _| false,
        );
        assert_eq!(player.coyote_time, Player::COYOTE_TIME);

        // Now in air, coyote time should decrease
        PlayerPhysicsSystem::update(
            &mut player,
            &input,
            dt,
            200.0,
            || false,
            |_, _, _, _| false,
        );
        assert!(player.coyote_time < Player::COYOTE_TIME);
        assert!(player.coyote_time > 0.0);
    }

    #[test]
    fn test_jump_buffer() {
        let mut player = make_test_player();
        let mut input = make_test_input();
        input.jump_pressed = true;
        let dt = 1.0 / 60.0;

        PlayerPhysicsSystem::update(
            &mut player,
            &input,
            dt,
            200.0,
            || false,
            |_, _, _, _| false,
        );

        // Jump buffer should be set when jump is pressed
        // (but used immediately if coyote time > 0, so we start in air)
    }

    #[test]
    fn test_horizontal_movement_right() {
        let mut player = make_test_player();
        let mut input = make_test_input();
        input.d_pressed = true;
        let dt = 1.0 / 60.0;
        let player_speed = 200.0;

        let initial_x = player.position.x;

        PlayerPhysicsSystem::update(
            &mut player,
            &input,
            dt,
            player_speed,
            || true,
            |_, _, _, _| false,
        );

        assert!(player.position.x > initial_x);
        assert_eq!(player.velocity.x, player_speed);
    }

    #[test]
    fn test_horizontal_movement_left() {
        let mut player = make_test_player();
        let mut input = make_test_input();
        input.a_pressed = true;
        let dt = 1.0 / 60.0;
        let player_speed = 200.0;

        let initial_x = player.position.x;

        PlayerPhysicsSystem::update(
            &mut player,
            &input,
            dt,
            player_speed,
            || true,
            |_, _, _, _| false,
        );

        assert!(player.position.x < initial_x);
        assert_eq!(player.velocity.x, -player_speed);
    }

    #[test]
    fn test_friction_when_no_input() {
        let mut player = make_test_player();
        player.velocity.x = 200.0; // Moving right
        let input = make_test_input(); // No keys pressed
        let dt = 1.0 / 60.0;

        PlayerPhysicsSystem::update(
            &mut player,
            &input,
            dt,
            200.0,
            || true, // grounded
            |_, _, _, _| false,
        );

        // Velocity should have decreased due to friction
        assert!(player.velocity.x < 200.0);
    }

    #[test]
    fn test_gravity_when_airborne() {
        let mut player = make_test_player();
        player.velocity.y = 0.0;
        let input = make_test_input();
        let dt = 1.0 / 60.0;

        PlayerPhysicsSystem::update(
            &mut player,
            &input,
            dt,
            200.0,
            || false, // not grounded
            |_, _, _, _| false,
        );

        // Gravity should have been applied (velocity decreases since gravity is downward)
        assert!(player.velocity.y < 0.0);
    }

    #[test]
    fn test_jump_execution() {
        let mut player = make_test_player();
        player.coyote_time = Player::COYOTE_TIME;
        let mut input = make_test_input();
        input.jump_pressed = true;
        let dt = 1.0 / 60.0;

        PlayerPhysicsSystem::update(
            &mut player,
            &input,
            dt,
            200.0,
            || true,
            |_, _, _, _| false,
        );

        // After jumping, coyote time should be consumed
        assert_eq!(player.coyote_time, 0.0);
    }

    #[test]
    fn test_flight_thrust() {
        let mut player = make_test_player();
        player.velocity.y = 0.0;
        let mut input = make_test_input();
        input.w_pressed = true;
        let dt = 1.0 / 60.0;

        PlayerPhysicsSystem::update(
            &mut player,
            &input,
            dt,
            200.0,
            || false, // not grounded
            |_, _, _, _| false,
        );

        // Flight thrust should counter gravity somewhat
        // Net effect depends on thrust vs gravity over dt
        // FLIGHT_THRUST = 1200, GRAVITY = 800
        // So velocity should increase (thrust > gravity for first frame)
    }

    #[test]
    fn test_collision_blocks_x_movement() {
        let mut player = make_test_player();
        let mut input = make_test_input();
        input.d_pressed = true;
        let dt = 1.0 / 60.0;
        let initial_x = player.position.x;

        PlayerPhysicsSystem::update(
            &mut player,
            &input,
            dt,
            200.0,
            || true,
            |x, _, _, _| x > initial_x, // Block movement to the right
        );

        // Player should not have moved
        assert_eq!(player.position.x, initial_x);
    }

    #[test]
    fn test_collision_blocks_y_movement() {
        let mut player = make_test_player();
        player.velocity.y = -100.0; // Falling
        let input = make_test_input();
        let dt = 1.0 / 60.0;
        let initial_y = player.position.y;

        PlayerPhysicsSystem::update(
            &mut player,
            &input,
            dt,
            200.0,
            || false,
            |_, y, _, _| y < initial_y, // Block downward movement
        );

        // Y position should not have changed, velocity should be zeroed
        assert_eq!(player.position.y, initial_y);
        assert_eq!(player.velocity.y, 0.0);
    }

    #[test]
    fn test_terminal_velocity() {
        let mut player = make_test_player();
        player.velocity.y = -10000.0; // Way beyond terminal
        let input = make_test_input();
        let dt = 1.0 / 60.0;

        PlayerPhysicsSystem::update(
            &mut player,
            &input,
            dt,
            200.0,
            || false,
            |_, _, _, _| false,
        );

        // Velocity should be clamped to max fall speed
        assert!(player.velocity.y >= -Player::MAX_FALL_SPEED);
        assert!(player.velocity.y <= Player::MAX_FALL_SPEED);
    }

    #[test]
    fn test_grounded_resets_vertical_velocity() {
        let mut player = make_test_player();
        player.velocity.y = -50.0; // Some downward velocity
        let input = make_test_input();
        let dt = 1.0 / 60.0;

        PlayerPhysicsSystem::update(
            &mut player,
            &input,
            dt,
            200.0,
            || true, // grounded
            |_, _, _, _| false,
        );

        // Grounded player (not jumping) should have zero vertical velocity
        assert_eq!(player.velocity.y, 0.0);
    }

    #[test]
    fn test_unstuck_mechanic() {
        let mut player = make_test_player();
        let mut input = make_test_input();
        input.d_pressed = true; // Try to move
        let dt = 1.0 / 60.0;
        let initial_pos = player.position;

        PlayerPhysicsSystem::update(
            &mut player,
            &input,
            dt,
            200.0,
            || false,
            |x, y, _, _| {
                // Block regular movement, but allow small unstuck offsets
                let dx = (x - initial_pos.x).abs();
                let dy = (y - initial_pos.y).abs();
                dx > 0.5 || dy > 0.5
            },
        );

        // Player may have been nudged by unstuck mechanic
        // This tests that the code path is exercised
    }
}
