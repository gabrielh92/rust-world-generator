use egui::{CentralPanel, Context, SidePanel};

use crate::canvas::show_canvas;
use crate::canvas::CanvasLayer;
use crate::visualization::points::PointsVisualLayer;
use crate::visualization::VisualLayer;

pub struct DebugVisualizer {
    canvas_layer: CanvasLayer,
    visual_layers: Vec<Box<dyn VisualLayer>>,
}

impl eframe::App for DebugVisualizer {
    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        // Left control panel
        SidePanel::left("control_panel").show(ctx, |ui| {
            self.canvas_layer.draw_controls(ui);
            for layer in self.visual_layers.iter_mut() {
                layer.draw_controls(ui);
            }
        });

        // Center canvas
        CentralPanel::default().show(ctx, |ui| {
            if self.canvas_layer.is_enabled() {
                let (rect, painter) = show_canvas(ui, &self.canvas_layer);

                let temp = crate::pipeline::StageOutputs::new();
                for layer in self.visual_layers.iter() {
                    if layer.is_enabled() {
                        layer.draw_canvas(&painter, &self.canvas_layer, &temp);
                    }
                }
            }
        });
    }
}

impl Default for DebugVisualizer {
    fn default() -> Self {
        Self {
            canvas_layer: CanvasLayer::default(),
            visual_layers: vec![
                Box::new(PointsVisualLayer::new()),
                // add more layers here as needed
            ],
        }
    }
}
