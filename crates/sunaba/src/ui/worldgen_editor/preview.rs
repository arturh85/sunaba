//! Live preview system for world generation

use crate::simulation::Materials;
use sunaba_core::world::{CHUNK_SIZE, WorldGenConfig, WorldGenerator};

/// Preview chunk radius (7x7 = 49 chunks visible)
const PREVIEW_RADIUS: i32 = 3;

/// Preview center Y position (show surface area)
const PREVIEW_CENTER_Y: i32 = 0;

/// Preview state for live world generation preview
pub struct PreviewState {
    /// Preview generator (separate from world generator)
    generator: Option<WorldGenerator>,

    /// Current seed
    current_seed: u64,

    /// Cached preview image data
    preview_pixels: Vec<u8>,

    /// Preview dimensions
    preview_width: usize,
    preview_height: usize,

    /// Texture handle for egui
    texture_handle: Option<egui::TextureHandle>,

    /// Camera offset (for panning)
    pub camera_x: i32,
    pub camera_y: i32,
}

impl PreviewState {
    pub fn new() -> Self {
        let size = (PREVIEW_RADIUS * 2 + 1) as usize * CHUNK_SIZE;
        Self {
            generator: None,
            current_seed: 0,
            preview_pixels: vec![0u8; size * size * 4],
            preview_width: size,
            preview_height: size,
            texture_handle: None,
            camera_x: 0,
            camera_y: PREVIEW_CENTER_Y,
        }
    }

    /// Update preview with new config/seed
    pub fn update(&mut self, config: &WorldGenConfig, seed: u64, materials: &Materials) {
        // Create/update generator
        if self.generator.is_none() || self.current_seed != seed {
            self.generator = Some(WorldGenerator::from_config(seed, config.clone()));
            self.current_seed = seed;
        } else {
            self.generator
                .as_mut()
                .unwrap()
                .update_config(config.clone());
        }

        // Generate preview chunks and build texture
        self.generate_preview(materials);

        // Invalidate texture so it gets recreated
        self.texture_handle = None;
    }

    /// Generate preview image from chunks
    fn generate_preview(&mut self, materials: &Materials) {
        let generator_ref = match &self.generator {
            Some(g) => g,
            None => return,
        };

        let chunk_start_x = (self.camera_x / CHUNK_SIZE as i32) - PREVIEW_RADIUS;
        let chunk_start_y = (self.camera_y / CHUNK_SIZE as i32) - PREVIEW_RADIUS;

        // Generate each chunk and copy to preview buffer
        for cy_offset in 0..=(PREVIEW_RADIUS * 2) {
            for cx_offset in 0..=(PREVIEW_RADIUS * 2) {
                let chunk_x = chunk_start_x + cx_offset;
                let chunk_y = chunk_start_y + cy_offset;

                let chunk = generator_ref.generate_chunk(chunk_x, chunk_y);

                // Copy chunk pixels to preview buffer
                let base_px = cx_offset as usize * CHUNK_SIZE;
                // Flip Y for display (higher Y = higher on screen)
                let base_py = (PREVIEW_RADIUS * 2 - cy_offset) as usize * CHUNK_SIZE;

                for ly in 0..CHUNK_SIZE {
                    for lx in 0..CHUNK_SIZE {
                        let material_id = chunk.get_material(lx, ly);
                        let color = materials.get(material_id).color;

                        let px = base_px + lx;
                        let py = base_py + (CHUNK_SIZE - 1 - ly); // Flip Y within chunk too
                        let idx = (py * self.preview_width + px) * 4;

                        if idx + 3 < self.preview_pixels.len() {
                            self.preview_pixels[idx] = color[0];
                            self.preview_pixels[idx + 1] = color[1];
                            self.preview_pixels[idx + 2] = color[2];
                            self.preview_pixels[idx + 3] = 255;
                        }
                    }
                }
            }
        }
    }

    /// Render the preview in egui
    pub fn render(&mut self, ui: &mut egui::Ui, _materials: &Materials) {
        // Create texture if needed
        if self.texture_handle.is_none() {
            let color_image = egui::ColorImage::from_rgba_unmultiplied(
                [self.preview_width, self.preview_height],
                &self.preview_pixels,
            );

            self.texture_handle = Some(ui.ctx().load_texture(
                "worldgen_preview",
                color_image,
                egui::TextureOptions::NEAREST,
            ));
        }

        // Preview controls
        ui.horizontal(|ui| {
            ui.label("Pan:");
            if ui.button("◀").clicked() {
                self.camera_x -= CHUNK_SIZE as i32;
            }
            if ui.button("▶").clicked() {
                self.camera_x += CHUNK_SIZE as i32;
            }
            if ui.button("▲").clicked() {
                self.camera_y += CHUNK_SIZE as i32;
            }
            if ui.button("▼").clicked() {
                self.camera_y -= CHUNK_SIZE as i32;
            }
            if ui.button("⟳ Reset").clicked() {
                self.camera_x = 0;
                self.camera_y = PREVIEW_CENTER_Y;
            }
        });

        ui.label(format!("Center: ({}, {})", self.camera_x, self.camera_y));

        // Render preview image
        if let Some(tex) = &self.texture_handle {
            let available = ui.available_size();
            let size = available.min(egui::vec2(400.0, 400.0));

            ui.add(
                egui::Image::new(tex)
                    .fit_to_exact_size(size)
                    .sense(egui::Sense::click_and_drag()),
            );
        } else {
            ui.label("Generating preview...");
        }

        // Legend
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            ui.label("Legend:");
            ui.colored_label(egui::Color32::from_rgb(139, 90, 43), "● Dirt");
            ui.colored_label(egui::Color32::from_rgb(194, 178, 128), "● Sand");
            ui.colored_label(egui::Color32::from_rgb(128, 128, 128), "● Stone");
            ui.colored_label(egui::Color32::from_rgb(64, 164, 223), "● Water");
        });
    }
}

impl Default for PreviewState {
    fn default() -> Self {
        Self::new()
    }
}
