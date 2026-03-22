use egui::{CentralPanel, Color32, Context, Sense, SidePanel};

use crate::canvas::{CanvasData, CanvasLayer};
use crate::mathlib::vec::Vec2;
use crate::pipeline::Pipeline;
use crate::visualization::VisualLayer;

pub struct DebugVisualizer {
    canvas_layer: CanvasLayer,
    pipeline: Pipeline,
    /// True on the first frame — ensures pipeline runs once before anything is drawn.
    needs_initial_run: bool,
}

impl DebugVisualizer {
    pub fn new(pipeline: Pipeline) -> Self {
        Self {
            canvas_layer: CanvasLayer::default(),
            pipeline,
            needs_initial_run: true,
        }
    }

    fn draw_legend(&self, ui: &mut egui::Ui) {
        ui.collapsing("Legend", |ui| {
            // Determine which view modes are active across enabled stages
            let mut show_elevation  = false;
            let mut show_land_type  = false;
            let mut show_moisture   = false;
            let mut show_moist_type = false;

            for stage in &self.pipeline.stages {
                if !stage.visual_layer.is_enabled() { continue; }
                let name      = stage.visual_layer.display_name();
                let view_mode = stage.params.as_ref()
                    .and_then(|p| p.get_param("View Mode"))
                    .map(|p| p.value.as_str())
                    .unwrap_or("");
                match (name, view_mode) {
                    ("Moisture", "Land Type") => show_moist_type = true,
                    ("Moisture", _)           => show_moisture   = true,
                    (_, "Land Type")          => show_land_type  = true,
                    _                         => show_elevation  = true,
                }
            }

            let mut any = false;

            if show_elevation {
                any = true;
                ui.label(egui::RichText::new("Elevation").strong().small());
                legend_swatch(ui, Color32::from_rgb(10,  30,  80),  "Deep Ocean     < -0.5");
                legend_swatch(ui, Color32::from_rgb(50,  100, 180), "Shallow Ocean  -0.5 → -0.1");
                legend_swatch(ui, Color32::from_rgb(90,  150, 210), "Coast Water    -0.1 → 0.0");
                legend_swatch(ui, Color32::from_rgb(220, 200, 140), "Beach           0.0 → 0.15");
                legend_swatch(ui, Color32::from_rgb(90,  160, 60),  "Lowland        0.15 → 0.5");
                legend_swatch(ui, Color32::from_rgb(65,  120, 45),  "Highland        0.5 → 0.8");
                legend_swatch(ui, Color32::from_rgb(120, 100, 80),  "Mountain        0.8 → 1.0");
                legend_swatch(ui, Color32::from_rgb(230, 230, 230), "Peak           → 1.0");
            }

            if show_land_type {
                if any { ui.add_space(4.0); }
                any = true;
                ui.label(egui::RichText::new("Land Type (Elevation)").strong().small());
                legend_swatch(ui, Color32::from_rgb(40,  40,  60),  "Border");
                legend_swatch(ui, Color32::from_rgb(50,  100, 180), "Ocean");
                legend_swatch(ui, Color32::from_rgb(220, 200, 140), "Coast");
                legend_swatch(ui, Color32::from_rgb(90,  160, 60),  "Land (low elev)");
                legend_swatch(ui, Color32::from_rgb(230, 230, 230), "Land (high elev)");
            }

            if show_moisture {
                if any { ui.add_space(4.0); }
                any = true;
                ui.label(egui::RichText::new("Moisture").strong().small());
                legend_swatch(ui, Color32::from_rgb(200, 170, 100), "Arid       0.0 → 0.25");
                legend_swatch(ui, Color32::from_rgb(160, 185, 95),  "Semi-arid  0.25 → 0.5");
                legend_swatch(ui, Color32::from_rgb(80,  160, 80),  "Temperate  0.5 → 0.75");
                legend_swatch(ui, Color32::from_rgb(40,  125, 100), "Humid      0.75 → 1.0");
                legend_swatch(ui, Color32::from_rgb(20,  80,  110), "Wet        → 1.0");
            }

            if show_moist_type {
                if any { ui.add_space(4.0); }
                any = true;
                ui.label(egui::RichText::new("Land Type (Moisture)").strong().small());
                legend_swatch(ui, Color32::from_rgb(210, 180, 110), "Arid");
                legend_swatch(ui, Color32::from_rgb(170, 185, 100), "Semi-arid");
                legend_swatch(ui, Color32::from_rgb(90,  160, 70),  "Temperate");
                legend_swatch(ui, Color32::from_rgb(50,  140, 90),  "Humid");
                legend_swatch(ui, Color32::from_rgb(30,  100, 120), "Wet");
            }

            if !any {
                ui.label(egui::RichText::new("Enable a stage to see legend").weak().small());
            }
        });
    }

    fn inject_canvas_data(&mut self) {
        let w = self.canvas_layer.width();
        let h = self.canvas_layer.height();
        let seed = self.canvas_layer.seed();
        self.pipeline.data.insert(
            "canvas".into(),
            Box::new(CanvasData { width: w, height: h, center: Vec2::new(w / 2., h / 2.), seed }),
        );
    }
}

fn legend_swatch(ui: &mut egui::Ui, color: Color32, label: &str) {
    ui.horizontal(|ui| {
        let (rect, _) = ui.allocate_exact_size(egui::vec2(14.0, 10.0), Sense::hover());
        ui.painter().rect_filled(rect, 2.0, color);
        ui.painter().rect_stroke(rect, 2.0, egui::Stroke::new(0.5, Color32::from_gray(80)));
        ui.label(egui::RichText::new(label).small());
    });
}

impl eframe::App for DebugVisualizer {
    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        let mut params_changed = false;
        let mut canvas_changed = false;

        // Index (into stages vec) of the earliest stage whose computation params changed.
        // Stages are sorted by rank, so the first dirty index is the one to run from.
        let mut first_dirty_stage: Option<usize> = None;

        // --- Sidebar controls ---
        SidePanel::left("control_panel")
            .min_width(220.0)
            .show(ctx, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    canvas_changed |= self.canvas_layer.draw_controls(ui, None);

                    ui.separator();

                    for (i, stage) in self.pipeline.stages.iter_mut().enumerate() {
                        let changed = stage.visual_layer.draw_controls(ui, stage.params.as_mut());
                        if changed && first_dirty_stage.is_none() {
                            first_dirty_stage = Some(i);
                        }
                        params_changed |= changed;
                    }

                    ui.separator();
                    self.draw_legend(ui);
                });
            });

        // --- Canvas panel ---
        CentralPanel::default().show(ctx, |ui| {
            if !self.canvas_layer.is_enabled() { return; }

            let panel_size = ui.available_size();
            let (response, painter) = ui.allocate_painter(panel_size, Sense::hover());
            let panel_rect = response.rect;

            // Always keep canvas data fresh (insertion is cheap)
            self.inject_canvas_data();

            // Run the pipeline when needed — selectively from the first dirty stage
            // so upstream outputs are reused when only downstream params change.
            if self.needs_initial_run || canvas_changed {
                self.pipeline.run();
                self.needs_initial_run = false;
            } else if let Some(idx) = first_dirty_stage {
                self.pipeline.run_from_stage(idx);
            }

            // Draw canvas background
            self.canvas_layer.draw_canvas(&painter, &panel_rect, None, &self.pipeline.data);

            // Draw each enabled stage layer
            for stage in self.pipeline.stages.iter() {
                if stage.visual_layer.is_enabled() {
                    stage.visual_layer.draw_canvas(
                        &painter,
                        &panel_rect,
                        stage.params.as_ref(),
                        &self.pipeline.data,
                    );
                }
            }
        });
    }
}
