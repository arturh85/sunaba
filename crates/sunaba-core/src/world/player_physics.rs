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
