//! HUD overlay for Powder Game demo

use egui::{Align2, Color32};

/// Stats for the HUD display
pub struct PowderStats {
    pub fps: f32,
    pub particle_count: usize,
    pub brush_size: u32,
    pub paused: bool,
}

/// Show the HUD overlay
pub fn show_hud(ctx: &egui::Context, stats: &PowderStats) {
    egui::Area::new(egui::Id::new("powder_hud"))
        .anchor(Align2::RIGHT_TOP, [-10.0, 10.0])
        .show(ctx, |ui| {
            egui::Frame::new()
                .fill(Color32::from_rgba_unmultiplied(0, 0, 0, 180))
                .inner_margin(8.0)
                .outer_margin(0.0)
                .corner_radius(4.0)
                .show(ui, |ui| {
                    ui.label(format!("FPS: {:.0}", stats.fps));
                    ui.label(format!("Particles: {}", stats.particle_count));
                    ui.label(format!("Brush: {}", stats.brush_size));
                    if stats.paused {
                        ui.colored_label(Color32::YELLOW, "PAUSED");
                    }
                });
        });
}
