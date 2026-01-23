//! Rendering module for Powder Game demo

mod renderer;
mod visualization;

pub use renderer::Renderer;
pub use visualization::{
    get_visualization_overlay, light_to_color, pressure_to_color, temperature_to_color,
};
