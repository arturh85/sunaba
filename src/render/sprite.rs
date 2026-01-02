//! Sprite and animation system for player and entity rendering

use anyhow::{Context, Result};

/// A spritesheet containing multiple animation frames in a grid layout
pub struct SpriteSheet {
    /// Total width of the spritesheet in pixels
    pub width: u32,
    /// Total height of the spritesheet in pixels
    pub height: u32,
    /// Width of each frame in pixels
    pub frame_width: u32,
    /// Height of each frame in pixels
    pub frame_height: u32,
    /// Number of columns in the grid
    pub cols: u32,
    /// Number of rows in the grid
    pub rows: u32,
    /// Raw RGBA pixel data
    pub data: Vec<u8>,
}

impl SpriteSheet {
    /// Load a spritesheet from PNG bytes with the specified grid dimensions
    pub fn from_png_bytes(bytes: &[u8], cols: u32, rows: u32) -> Result<Self> {
        let img = image::load_from_memory(bytes)
            .context("Failed to decode PNG image")?
            .to_rgba8();

        let width = img.width();
        let height = img.height();
        let frame_width = width / cols;
        let frame_height = height / rows;

        Ok(SpriteSheet {
            width,
            height,
            frame_width,
            frame_height,
            cols,
            rows,
            data: img.into_raw(),
        })
    }

    /// Get a pixel from the spritesheet (returns RGBA)
    #[inline]
    pub fn get_pixel(&self, x: u32, y: u32) -> [u8; 4] {
        if x >= self.width || y >= self.height {
            return [0, 0, 0, 0];
        }
        let idx = ((y * self.width + x) * 4) as usize;
        [
            self.data[idx],
            self.data[idx + 1],
            self.data[idx + 2],
            self.data[idx + 3],
        ]
    }

    /// Get the pixel coordinates for a specific frame in the grid
    #[inline]
    pub fn frame_origin(&self, col: u32, row: u32) -> (u32, u32) {
        (col * self.frame_width, row * self.frame_height)
    }
}

/// Definition of an animation sequence within a spritesheet
#[derive(Debug, Clone, Copy)]
pub struct AnimationDef {
    /// Row in the spritesheet
    pub row: u8,
    /// Starting column
    pub start_col: u8,
    /// Number of frames in this animation
    pub frame_count: u8,
    /// Duration of each frame in seconds
    pub frame_time: f32,
    /// Whether the animation should loop
    pub looping: bool,
}

impl AnimationDef {
    pub const fn new(row: u8, start_col: u8, frame_count: u8, frame_time: f32) -> Self {
        Self {
            row,
            start_col,
            frame_count,
            frame_time,
            looping: true,
        }
    }
}

/// Player animation states
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerAnimState {
    IdleFront,
    WalkRight,
    WalkLeft,
    Jump,
    Fall,
    MineRight,
    MineLeft,
    MineDown,
}

/// Player animation definitions based on the spritesheet layout
pub struct PlayerAnimations;

impl PlayerAnimations {
    // Row 0: Idle (6 frames)
    pub const IDLE_FRONT: AnimationDef = AnimationDef::new(0, 0, 6, 0.2);

    // Row 1: Walk right (6 frames)
    pub const WALK_RIGHT: AnimationDef = AnimationDef::new(1, 0, 6, 0.1);

    // Row 2: Walk left (6 frames)
    pub const WALK_LEFT: AnimationDef = AnimationDef::new(2, 0, 6, 0.1);

    // Row 3: Jump up (2 frames, cols 0-1)
    pub const JUMP: AnimationDef = AnimationDef::new(3, 0, 2, 0.15);

    // Row 4: Fall/jump down (2 frames, cols 0-1)
    pub const FALL: AnimationDef = AnimationDef::new(4, 0, 2, 0.15);

    // Row 4: Mining right (3 frames, cols 3-5)
    pub const MINE_RIGHT: AnimationDef = AnimationDef::new(4, 3, 3, 0.1);

    // Row 5: Mining down (3 frames, cols 0-2)
    pub const MINE_DOWN: AnimationDef = AnimationDef::new(5, 0, 3, 0.1);

    pub fn get(state: PlayerAnimState) -> AnimationDef {
        match state {
            PlayerAnimState::IdleFront => Self::IDLE_FRONT,
            PlayerAnimState::WalkRight => Self::WALK_RIGHT,
            PlayerAnimState::WalkLeft => Self::WALK_LEFT,
            PlayerAnimState::Jump => Self::JUMP,
            PlayerAnimState::Fall => Self::FALL,
            PlayerAnimState::MineRight => Self::MINE_RIGHT,
            PlayerAnimState::MineLeft => Self::MINE_RIGHT, // Will flip horizontally
            PlayerAnimState::MineDown => Self::MINE_DOWN,
        }
    }
}

/// Player sprite state tracking animation and rendering
pub struct PlayerSprite {
    /// The loaded spritesheet
    pub sheet: SpriteSheet,
    /// Current animation state
    pub current_state: PlayerAnimState,
    /// Current frame within the animation
    pub current_frame: u8,
    /// Time accumulator for frame timing
    pub frame_timer: f32,
    /// Whether the player is facing right
    pub facing_right: bool,
    /// Display width after scaling
    pub display_width: u32,
    /// Display height after scaling
    pub display_height: u32,
}

impl PlayerSprite {
    /// Target display size for the sprite (scaled down from source, square to match source)
    pub const DISPLAY_WIDTH: u32 = 32;
    pub const DISPLAY_HEIGHT: u32 = 32;

    /// Create a new player sprite from PNG bytes
    pub fn new(png_bytes: &[u8]) -> Result<Self> {
        let sheet = SpriteSheet::from_png_bytes(png_bytes, 6, 6)?;

        Ok(Self {
            sheet,
            current_state: PlayerAnimState::IdleFront,
            current_frame: 0,
            frame_timer: 0.0,
            facing_right: true,
            display_width: Self::DISPLAY_WIDTH,
            display_height: Self::DISPLAY_HEIGHT,
        })
    }

    /// Update animation based on player velocity and state
    pub fn update_state(&mut self, velocity_x: f32, velocity_y: f32, is_mining: bool) {
        let new_state = if is_mining {
            if velocity_x > 0.1 {
                PlayerAnimState::MineRight
            } else if velocity_x < -0.1 {
                PlayerAnimState::MineLeft
            } else {
                PlayerAnimState::MineDown
            }
        } else if velocity_y < -10.0 {
            // Going up (negative Y is up in this coordinate system)
            PlayerAnimState::Jump
        } else if velocity_y > 10.0 {
            // Falling
            PlayerAnimState::Fall
        } else if velocity_x.abs() > 0.1 {
            if velocity_x > 0.0 {
                self.facing_right = true;
                PlayerAnimState::WalkRight
            } else {
                self.facing_right = false;
                PlayerAnimState::WalkLeft
            }
        } else {
            PlayerAnimState::IdleFront
        };

        // Reset animation frame when state changes
        if new_state != self.current_state {
            self.current_state = new_state;
            self.current_frame = 0;
            self.frame_timer = 0.0;
        }
    }

    /// Update animation timing
    pub fn update(&mut self, delta_time: f32) {
        let anim = PlayerAnimations::get(self.current_state);
        self.frame_timer += delta_time;

        if self.frame_timer >= anim.frame_time {
            self.frame_timer -= anim.frame_time;
            self.current_frame += 1;

            if self.current_frame >= anim.frame_count {
                if anim.looping {
                    self.current_frame = 0;
                } else {
                    self.current_frame = anim.frame_count - 1;
                }
            }
        }
    }

    /// Get the current frame's source rectangle (col, row) in the spritesheet
    pub fn current_frame_coords(&self) -> (u32, u32) {
        let anim = PlayerAnimations::get(self.current_state);
        let col = anim.start_col as u32 + self.current_frame as u32;
        let row = anim.row as u32;
        (col, row)
    }

    /// Sample a pixel from the current animation frame with scaling
    /// Returns RGBA, or None if the pixel is transparent
    /// `local_x` and `local_y` are in display coordinates (0..display_width, 0..display_height)
    #[inline]
    pub fn sample_pixel(&self, local_x: i32, local_y: i32, flip_h: bool) -> Option<[u8; 4]> {
        if local_x < 0
            || local_y < 0
            || local_x >= self.display_width as i32
            || local_y >= self.display_height as i32
        {
            return None;
        }

        let (col, row) = self.current_frame_coords();
        let (frame_x, frame_y) = self.sheet.frame_origin(col, row);

        // Scale from display coordinates to source coordinates (nearest neighbor)
        let scale_x = self.sheet.frame_width as f32 / self.display_width as f32;
        let scale_y = self.sheet.frame_height as f32 / self.display_height as f32;

        let src_local_x = if flip_h {
            ((self.display_width as i32 - 1 - local_x) as f32 * scale_x) as u32
        } else {
            (local_x as f32 * scale_x) as u32
        };
        // Flip Y to correct upside-down rendering (sprite Y=0 is top, world Y=0 is bottom)
        let flipped_local_y = self.display_height as i32 - 1 - local_y;
        let src_local_y = (flipped_local_y as f32 * scale_y) as u32;

        let src_x = frame_x + src_local_x;
        let src_y = frame_y + src_local_y;

        let pixel = self.sheet.get_pixel(src_x, src_y);

        // Skip transparent pixels
        if pixel[3] < 128 {
            return None;
        }

        Some(pixel)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_animation_def() {
        let anim = AnimationDef::new(0, 0, 4, 0.25);
        assert_eq!(anim.row, 0);
        assert_eq!(anim.frame_count, 4);
        assert!(anim.looping);
    }

    #[test]
    fn test_player_animations() {
        let idle = PlayerAnimations::get(PlayerAnimState::IdleFront);
        assert_eq!(idle.row, 0);
        assert_eq!(idle.frame_count, 6);

        let walk = PlayerAnimations::get(PlayerAnimState::WalkRight);
        assert_eq!(walk.row, 1);
        assert_eq!(walk.frame_count, 6);

        let fall = PlayerAnimations::get(PlayerAnimState::Fall);
        assert_eq!(fall.row, 4);
        assert_eq!(fall.frame_count, 2);
    }
}
