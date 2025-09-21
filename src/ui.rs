use egui::{CentralPanel, Context, Sense, SidePanel};

use crate::canvas::{CanvasData, CanvasLayer};
use crate::mathlib::vec::Vec2;
use crate::pipeline::Pipeline;
use crate::visualization::VisualLayer;

pub struct DebugVisualizer {
    canvas_layer: CanvasLayer,
    pipeline: Pipeline,
}

impl DebugVisualizer {
    pub fn new(pipeline: Pipeline) -> Self {
        let canvas_layer = CanvasLayer::default();

        Self {
            canvas_layer,
            pipeline,
        }
    }
}

impl eframe::App for DebugVisualizer {
    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        let mut params_changed = vec![false; self.pipeline.stages.len()];
        let mut canvas_changed = false;

        // Left control panel
        SidePanel::left("control_panel").show(ctx, |ui| {
            canvas_changed |= self.canvas_layer.draw_controls(ui, None);

            for stage in self.pipeline.stages.iter_mut() {
                let changed = stage.visual_layer.draw_controls(ui, stage.params.as_mut());

                if changed {
                    println!("Params changed for stage: {}", stage.executor.name());
                }

				params_changed[(stage.executor.rank() - 1) as usize] = changed;
            }
        });

        // Center canvas
        CentralPanel::default().show(ctx, |ui| {
            if !self.canvas_layer.is_enabled() {
				return;
			}

			let panel_size = ui.available_size();
			let (response, painter) = ui.allocate_painter(panel_size, Sense::hover());
			let panel_rect = response.rect;

			self.canvas_layer.draw_canvas(&painter, &panel_rect, None, &self.pipeline.data);

			// Add canvas data into pipeline data
			let w = self.canvas_layer.width();
			let h = self.canvas_layer.height();

			// use half width and half height because it's relative
			let c = Vec2 { x: w / 2., y: h / 2. };
			self.pipeline.data.insert(
				"canvas".into(),
				Box::new(CanvasData { width: w, height: h, center: c })
			);

			// todo: doesn't run at start-up as well
			// todo: fix logic to run pipeline progressively on param changed instead of the entire thing
			if params_changed.iter().any(|it| *it) || canvas_changed {
				self.pipeline.run();
			}

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
