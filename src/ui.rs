use egui::{CentralPanel, Context, SidePanel};

use crate::canvas::{show_canvas, CanvasData, CanvasLayer};
use crate::pipeline::{Pipeline, PipelineStage};
use crate::visualization::VisualLayer;

pub struct DebugVisualizer {
	canvas_layer: CanvasLayer,
	pipeline: Pipeline,
}

impl DebugVisualizer {
    pub fn new(mut pipeline: Pipeline) -> Self {
        let canvas_layer = CanvasLayer::default();

        // Insert canvas data as the first pipeline data entry
        pipeline
            .data
            .insert("canvas".into(), Box::new(CanvasData {
                width: canvas_layer.width(),
                height: canvas_layer.height(),
            }));

        Self {
            canvas_layer,
            pipeline,
        }
    }
}

impl eframe::App for DebugVisualizer {
    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
		let mut param_changed = false;
		let mut canvas_changed = false;

        // Left control panel
        SidePanel::left("control_panel").show(ctx, |ui| {
            canvas_changed |= self.canvas_layer.draw_controls(ui, None);

            for stage in self.pipeline.stages.iter_mut() {
                let changed = stage
                    .visual_layer
                    .draw_controls(ui, stage.params.as_mut());

                if changed {
                    println!("Params changed for stage: {}", stage.executor.name());
                }

                param_changed |= changed;
            }
        });

        // Center canvas
        CentralPanel::default().show(ctx, |ui| {
            if self.canvas_layer.is_enabled() {
                let (rect, painter) = show_canvas(ui, &self.canvas_layer);
				self.canvas_layer.draw_canvas(&painter, None, &self.pipeline.data);

				// todo: doesn't run at start-up as well
				if param_changed || canvas_changed {
					let canvas_data = CanvasData {
						width: self.canvas_layer.width() + rect.min.x,
						height: self.canvas_layer.height() + rect.min.y,
					};
					self.pipeline.data.insert("canvas".into(), Box::new(canvas_data));
					self.pipeline.run();
				}

                for stage in self.pipeline.stages.iter() {
                    if stage.visual_layer.is_enabled() {
                        stage.visual_layer.draw_canvas(&painter, stage.params.as_ref(), &self.pipeline.data);
                    }
                }
            }
        });
    }
}
