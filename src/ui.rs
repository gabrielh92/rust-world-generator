use egui::{CentralPanel, Context, Sense, SidePanel};

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
