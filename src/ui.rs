use egui::{CentralPanel, Context, SidePanel};

use crate::canvas::show_canvas;
use crate::canvas::CanvasLayer;
use crate::visualization::VisualLayer;

#[derive(Default)]
pub struct DebugVisualizer {
    canvas_layer: CanvasLayer,
}

impl eframe::App for DebugVisualizer {
    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        // Left control panel
        SidePanel::left("control_panel").show(ctx, |ui| {
            self.canvas_layer.draw_controls(ui);
        });

        // Center canvas
        CentralPanel::default().show(ctx, |ui| {
            if self.canvas_layer.is_enabled() {
                let (_rect, _painter) = show_canvas(ui, &self.canvas_layer);
            }
        });
    }
}
