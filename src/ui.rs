use egui::{CentralPanel, Context, SidePanel};

use crate::canvas::show_canvas;
use crate::canvas::CanvasPanel;

#[derive(Default)]
pub struct DebugVisualizer {
	canvas_panel: CanvasPanel,
}

impl eframe::App for DebugVisualizer {
	fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        // Left control panel
        SidePanel::left("control_panel").show(ctx, |ui| {
            self.canvas_panel.draw_controls(ui);
        });

        // Center canvas
        CentralPanel::default().show(ctx, |ui| {
			if self.canvas_panel.is_enabled() {
				let (_rect, _painter) = show_canvas(ui, &self.canvas_panel.config);
			}
        });
	}
}