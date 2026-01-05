//! Player input state

use crate::simulation::MaterialId;

/// Tracks current input state for player control
#[derive(Debug, Clone)]
pub struct InputState {
    // Movement keys
    pub w_pressed: bool,
    pub a_pressed: bool,
    pub s_pressed: bool,
    pub d_pressed: bool,
    pub jump_pressed: bool, // Space bar for jumping

    // Material selection (1-9 map to material IDs)
    pub selected_material: u16,

    // Mouse state
    pub mouse_world_pos: Option<(i32, i32)>, // Converted to world coords
    pub left_mouse_pressed: bool,
    pub right_mouse_pressed: bool,
    pub prev_right_mouse_pressed: bool, // Previous frame's right mouse state

    // Zoom control
    pub zoom_delta: f32, // Zoom change this frame (1.0 = no change)
}

impl InputState {
    pub fn new() -> Self {
        Self {
            w_pressed: false,
            a_pressed: false,
            s_pressed: false,
            d_pressed: false,
            jump_pressed: false,
            selected_material: MaterialId::SAND, // Start with sand
            mouse_world_pos: None,
            left_mouse_pressed: false,
            right_mouse_pressed: false,
            prev_right_mouse_pressed: false,
            zoom_delta: 1.0, // No change by default
        }
    }
}

impl Default for InputState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_input_state_new() {
        let input = InputState::new();

        // All movement keys should be unpressed
        assert!(!input.w_pressed);
        assert!(!input.a_pressed);
        assert!(!input.s_pressed);
        assert!(!input.d_pressed);
        assert!(!input.jump_pressed);

        // Default material should be sand
        assert_eq!(input.selected_material, MaterialId::SAND);

        // Mouse state should be reset
        assert!(input.mouse_world_pos.is_none());
        assert!(!input.left_mouse_pressed);
        assert!(!input.right_mouse_pressed);
        assert!(!input.prev_right_mouse_pressed);

        // Zoom should be 1.0 (no change)
        assert_eq!(input.zoom_delta, 1.0);
    }

    #[test]
    fn test_input_state_default() {
        let input = InputState::default();
        let new_input = InputState::new();

        assert_eq!(input.selected_material, new_input.selected_material);
        assert_eq!(input.zoom_delta, new_input.zoom_delta);
    }

    #[test]
    fn test_input_state_modifiable() {
        let mut input = InputState::new();

        // Simulate pressing movement keys
        input.w_pressed = true;
        input.a_pressed = true;
        assert!(input.w_pressed);
        assert!(input.a_pressed);

        // Simulate mouse input
        input.mouse_world_pos = Some((100, 200));
        input.left_mouse_pressed = true;
        assert_eq!(input.mouse_world_pos, Some((100, 200)));
        assert!(input.left_mouse_pressed);

        // Change material selection
        input.selected_material = MaterialId::WATER;
        assert_eq!(input.selected_material, MaterialId::WATER);

        // Zoom
        input.zoom_delta = 1.5;
        assert_eq!(input.zoom_delta, 1.5);
    }
}
