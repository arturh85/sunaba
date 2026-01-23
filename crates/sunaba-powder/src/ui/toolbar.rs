//! Material toolbar for Powder Game demo
//!
//! Provides a classic Powder Game-style material selector grid.

use egui::{Color32, CornerRadius, Stroke, StrokeKind, Vec2};
use sunaba_core::simulation::{MaterialId, Materials};

/// Cached material info for toolbar display
#[derive(Clone)]
pub struct MaterialInfo {
    pub id: u16,
    pub name: String,
    pub color: Color32,
}

/// Currently active drawing tool
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ActiveTool {
    #[default]
    Pen,
    Eraser,
    Wind,
    Drag,
}

/// Visualization mode for background overlays
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum VisualizationMode {
    #[default]
    None,
    /// Pressure heatmap (Air mode in Powder Game)
    Pressure,
    /// Thermography (TG mode)
    Temperature,
    /// Light level visualization
    Light,
}

/// State for the material toolbar
pub struct ToolbarState {
    /// Material selected for left mouse button
    pub left_material: u16,
    /// Material selected for right mouse button
    pub right_material: u16,
    /// Current brush size (1-10)
    pub brush_size: u32,
    /// Simulation speed multiplier (0.25-4.0)
    pub sim_speed: f32,
    /// Whether simulation is paused
    pub paused: bool,
    /// Currently active tool
    pub active_tool: ActiveTool,
    /// Visualization mode
    pub visualization_mode: VisualizationMode,
}

impl Default for ToolbarState {
    fn default() -> Self {
        Self {
            left_material: MaterialId::SAND,
            right_material: MaterialId::WATER,
            brush_size: 3,
            sim_speed: 1.0,
            paused: false,
            active_tool: ActiveTool::default(),
            visualization_mode: VisualizationMode::default(),
        }
    }
}

/// Material toolbar widget
pub struct MaterialToolbar {
    materials: Vec<MaterialInfo>,
}

impl MaterialToolbar {
    /// Create a new toolbar with materials from the registry
    pub fn new(materials: &Materials) -> Self {
        // Collect all non-AIR materials
        let material_infos: Vec<MaterialInfo> = (1..=MaterialId::BUBBLE)
            .map(|id| {
                let mat = materials.get(id);
                let color = mat.color;
                MaterialInfo {
                    id,
                    name: mat.name.clone(),
                    color: Color32::from_rgba_unmultiplied(color[0], color[1], color[2], color[3]),
                }
            })
            .collect();

        Self {
            materials: material_infos,
        }
    }

    /// Show the toolbar panel
    pub fn show(&self, ctx: &egui::Context, state: &mut ToolbarState) {
        egui::SidePanel::left("material_toolbar")
            .default_width(320.0)
            .resizable(true)
            .show(ctx, |ui| {
                ui.heading("Sunaba Powder");
                ui.separator();

                // Brush controls
                ui.horizontal(|ui| {
                    ui.label("Brush size:");
                    ui.add(egui::Slider::new(&mut state.brush_size, 1..=10));
                });

                ui.horizontal(|ui| {
                    ui.label("Speed:");
                    ui.add(egui::Slider::new(&mut state.sim_speed, 0.25..=4.0).step_by(0.25));
                });

                ui.horizontal(|ui| {
                    if ui
                        .button(if state.paused { "Resume" } else { "Pause" })
                        .clicked()
                    {
                        state.paused = !state.paused;
                    }
                    if ui.button("Step").clicked() && state.paused {
                        // Will be handled by app
                    }
                });

                ui.separator();

                // Tool selection
                ui.label("Tool:");
                ui.horizontal(|ui| {
                    ui.selectable_value(&mut state.active_tool, ActiveTool::Pen, "Pen");
                    ui.selectable_value(&mut state.active_tool, ActiveTool::Eraser, "Eraser");
                });
                ui.horizontal(|ui| {
                    ui.selectable_value(&mut state.active_tool, ActiveTool::Wind, "Wind");
                    ui.selectable_value(&mut state.active_tool, ActiveTool::Drag, "Drag");
                });

                ui.separator();

                // Visualization mode
                ui.label("Display Mode:");
                ui.horizontal(|ui| {
                    ui.selectable_value(
                        &mut state.visualization_mode,
                        VisualizationMode::None,
                        "None",
                    );
                    ui.selectable_value(
                        &mut state.visualization_mode,
                        VisualizationMode::Pressure,
                        "Air",
                    );
                });
                ui.horizontal(|ui| {
                    ui.selectable_value(
                        &mut state.visualization_mode,
                        VisualizationMode::Temperature,
                        "TG",
                    );
                    ui.selectable_value(
                        &mut state.visualization_mode,
                        VisualizationMode::Light,
                        "Light",
                    );
                });

                ui.separator();

                // Current selection display
                ui.horizontal(|ui| {
                    ui.label("Left click:");
                    let left_mat = self.materials.iter().find(|m| m.id == state.left_material);
                    if let Some(mat) = left_mat {
                        self.show_material_chip(ui, mat);
                    }
                });

                ui.horizontal(|ui| {
                    ui.label("Right click:");
                    let right_mat = self.materials.iter().find(|m| m.id == state.right_material);
                    if let Some(mat) = right_mat {
                        self.show_material_chip(ui, mat);
                    }
                });

                ui.separator();
                ui.label("Materials (click = left, right-click = right):");
                ui.separator();

                // Material grid (scrollable)
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        self.show_material_grid(ui, state);
                    });
            });
    }

    /// Show material grid
    fn show_material_grid(&self, ui: &mut egui::Ui, state: &mut ToolbarState) {
        let columns = 6;
        let button_size = Vec2::new(44.0, 44.0);

        egui::Grid::new("material_grid")
            .num_columns(columns)
            .spacing([4.0, 4.0])
            .show(ui, |ui| {
                for (i, mat) in self.materials.iter().enumerate() {
                    let is_left_selected = state.left_material == mat.id;
                    let is_right_selected = state.right_material == mat.id;

                    // Create button with material color
                    let response = self.material_button(
                        ui,
                        mat,
                        button_size,
                        is_left_selected,
                        is_right_selected,
                    );

                    if response.clicked() {
                        state.left_material = mat.id;
                    }
                    if response.secondary_clicked() {
                        state.right_material = mat.id;
                    }

                    response.on_hover_text(&mat.name);

                    if (i + 1) % columns == 0 {
                        ui.end_row();
                    }
                }
            });
    }

    /// Render a material button
    fn material_button(
        &self,
        ui: &mut egui::Ui,
        mat: &MaterialInfo,
        size: Vec2,
        is_left_selected: bool,
        is_right_selected: bool,
    ) -> egui::Response {
        let (response, painter) = ui.allocate_painter(size, egui::Sense::click());

        let rect = response.rect;
        let visuals = ui.style().interact(&response);

        // Background (material color)
        painter.rect_filled(rect.shrink(2.0), CornerRadius::same(4), mat.color);

        // Border based on selection
        let stroke = if is_left_selected && is_right_selected {
            Stroke::new(3.0, Color32::YELLOW)
        } else if is_left_selected {
            Stroke::new(3.0, Color32::WHITE)
        } else if is_right_selected {
            Stroke::new(3.0, Color32::from_rgb(100, 100, 255))
        } else if response.hovered() {
            Stroke::new(2.0, visuals.fg_stroke.color)
        } else {
            Stroke::new(1.0, Color32::from_gray(60))
        };

        painter.rect_stroke(
            rect.shrink(1.0),
            CornerRadius::same(4),
            stroke,
            StrokeKind::Outside,
        );

        response
    }

    /// Show a small material chip with color and name
    fn show_material_chip(&self, ui: &mut egui::Ui, mat: &MaterialInfo) {
        let (response, painter) = ui.allocate_painter(Vec2::new(20.0, 20.0), egui::Sense::hover());
        painter.rect_filled(response.rect, CornerRadius::same(3), mat.color);
        painter.rect_stroke(
            response.rect,
            CornerRadius::same(3),
            Stroke::new(1.0, Color32::WHITE),
            StrokeKind::Outside,
        );
        ui.label(&mat.name);
    }
}
