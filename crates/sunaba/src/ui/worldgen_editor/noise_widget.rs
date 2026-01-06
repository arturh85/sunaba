//! Reusable noise layer editor widget

use sunaba_core::world::worldgen_config::{FractalTypeConfig, NoiseLayerConfig, NoiseTypeConfig};

/// Render a collapsible noise layer editor
///
/// Returns true if any value changed.
pub fn noise_layer_editor(ui: &mut egui::Ui, config: &mut NoiseLayerConfig, id_salt: &str) -> bool {
    let mut changed = false;

    ui.push_id(id_salt, |ui| {
        // Frequency slider (logarithmic for wide range)
        changed |= ui
            .add(
                egui::Slider::new(&mut config.frequency, 0.0001..=0.1)
                    .logarithmic(true)
                    .text("Frequency"),
            )
            .on_hover_text("Lower = larger features, Higher = smaller features")
            .changed();

        // Noise type dropdown
        egui::ComboBox::from_label("Noise Type")
            .selected_text(format!("{:?}", config.noise_type))
            .show_ui(ui, |ui| {
                for noise_type in [
                    NoiseTypeConfig::OpenSimplex2,
                    NoiseTypeConfig::OpenSimplex2S,
                    NoiseTypeConfig::Perlin,
                    NoiseTypeConfig::Value,
                    NoiseTypeConfig::ValueCubic,
                    NoiseTypeConfig::Cellular,
                ] {
                    if ui
                        .selectable_value(
                            &mut config.noise_type,
                            noise_type,
                            format!("{:?}", noise_type),
                        )
                        .changed()
                    {
                        changed = true;
                    }
                }
            });

        // Fractal settings in collapsible section
        ui.collapsing("Fractal Settings", |ui| {
            // Fractal type dropdown
            egui::ComboBox::from_label("Fractal Type")
                .selected_text(format!("{:?}", config.fractal_type))
                .show_ui(ui, |ui| {
                    for fractal_type in [
                        FractalTypeConfig::None,
                        FractalTypeConfig::FBm,
                        FractalTypeConfig::Ridged,
                        FractalTypeConfig::PingPong,
                    ] {
                        if ui
                            .selectable_value(
                                &mut config.fractal_type,
                                fractal_type,
                                format!("{:?}", fractal_type),
                            )
                            .changed()
                        {
                            changed = true;
                        }
                    }
                });

            if config.fractal_type != FractalTypeConfig::None {
                changed |= ui
                    .add(egui::Slider::new(&mut config.octaves, 1..=8).text("Octaves"))
                    .on_hover_text("More octaves = more detail, but slower")
                    .changed();

                changed |= ui
                    .add(egui::Slider::new(&mut config.lacunarity, 1.5..=3.0).text("Lacunarity"))
                    .on_hover_text("Frequency multiplier per octave")
                    .changed();

                changed |= ui
                    .add(egui::Slider::new(&mut config.gain, 0.2..=0.8).text("Gain (Persistence)"))
                    .on_hover_text("Amplitude multiplier per octave")
                    .changed();
            }
        });
    });

    changed
}
