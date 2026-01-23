//! Visualization modes for Powder Game demo
//!
//! Provides color conversion functions for pressure, temperature, and light visualization.

use crate::ui::VisualizationMode;

/// Convert pressure (0-100) to RGBA color
/// Low pressure (0) = blue, high pressure (100) = red
pub fn pressure_to_color(pressure: f32) -> [u8; 4] {
    if pressure < 1.0 {
        // Very low pressure: dark blue
        let t = pressure;
        [0, 0, (128.0 + t * 127.0) as u8, 128]
    } else if pressure < 10.0 {
        // Mild pressure: blue to green gradient
        let t = (pressure - 1.0) / 9.0;
        [
            (t * 50.0) as u8,
            (100.0 + t * 155.0) as u8,
            (255.0 - t * 200.0) as u8,
            128,
        ]
    } else if pressure < 50.0 {
        // High pressure: green to yellow gradient
        let t = (pressure - 10.0) / 40.0;
        [(50.0 + t * 205.0) as u8, 255, (55.0 - t * 55.0) as u8, 128]
    } else {
        // Extreme pressure: yellow to red
        let t = ((pressure - 50.0) / 50.0).min(1.0);
        [255, (255.0 * (1.0 - t)) as u8, 0, 128]
    }
}

/// Convert temperature (Celsius, 0-1500) to RGBA color
/// Cold = blue, room temp = green, hot = red/white
pub fn temperature_to_color(temp: f32) -> [u8; 4] {
    if temp < 0.0 {
        // Frozen: deep blue
        let t = ((-temp).min(50.0)) / 50.0;
        [0, (50.0 * (1.0 - t)) as u8, (150.0 + 105.0 * t) as u8, 128]
    } else if temp < 20.0 {
        // Cold: blue to cyan
        let t = temp / 20.0;
        [0, (128.0 * t) as u8, (200.0 - t * 50.0) as u8, 128]
    } else if temp < 100.0 {
        // Room temp to warm: cyan to green
        let t = (temp - 20.0) / 80.0;
        [0, (128.0 + t * 127.0) as u8, (150.0 - t * 150.0) as u8, 128]
    } else if temp < 500.0 {
        // Hot: green to yellow
        let t = (temp - 100.0) / 400.0;
        [(t * 255.0) as u8, 255, 0, 128]
    } else if temp < 1000.0 {
        // Very hot: yellow to red
        let t = (temp - 500.0) / 500.0;
        [255, (255.0 * (1.0 - t)) as u8, 0, 128]
    } else {
        // Extreme: red to white
        let t = ((temp - 1000.0) / 500.0).min(1.0);
        [255, (t * 200.0) as u8, (t * 200.0) as u8, 128]
    }
}

/// Convert light level (0-15) to RGBA color
/// Dark = black, bright = white/yellow
pub fn light_to_color(level: u8) -> [u8; 4] {
    let t = level as f32 / 15.0;
    let brightness = (t * 255.0) as u8;
    // Slightly warm tint for light
    let warm = (t * 240.0) as u8;
    [brightness, brightness, warm, 128]
}

/// Get visualization overlay color for a pixel based on mode
/// Returns Some(color) if overlay should be applied, None otherwise
pub fn get_visualization_overlay(
    mode: VisualizationMode,
    material_id: u16,
    pressure: f32,
    temperature: f32,
    light_level: u8,
) -> Option<[u8; 4]> {
    use sunaba_core::simulation::MaterialId;

    match mode {
        VisualizationMode::None => None,
        VisualizationMode::Pressure => {
            // Show pressure for air and gas materials
            if material_id == MaterialId::AIR {
                Some(pressure_to_color(pressure))
            } else {
                None
            }
        }
        VisualizationMode::Temperature => {
            // Show temperature for all materials
            Some(temperature_to_color(temperature))
        }
        VisualizationMode::Light => {
            // Show light level for all materials
            Some(light_to_color(light_level))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pressure_colors() {
        let low = pressure_to_color(0.0);
        let high = pressure_to_color(100.0);

        // Low pressure should be blue-ish
        assert!(low[2] > low[0]);

        // High pressure should be red
        assert_eq!(high[0], 255);
        assert_eq!(high[1], 0);
    }

    #[test]
    fn test_temperature_colors() {
        let cold = temperature_to_color(-50.0);
        let hot = temperature_to_color(1000.0);

        // Cold should be blue-ish
        assert!(cold[2] > cold[0]);

        // Hot should be red
        assert_eq!(hot[0], 255);
    }

    #[test]
    fn test_light_colors() {
        let dark = light_to_color(0);
        let bright = light_to_color(15);

        // Dark should be black
        assert_eq!(dark[0], 0);
        assert_eq!(dark[1], 0);

        // Bright should be white-ish
        assert_eq!(bright[0], 255);
        assert_eq!(bright[1], 255);
    }
}
