mod canvas;
mod params;
mod pipeline;
mod ui;
mod util;
mod visualization;

use eframe::egui;
use pipeline::Pipeline;

use crate::{pipeline::points::make_points_stage, ui::DebugVisualizer};

static DEBUG_APP_NAME: &str = "WorldGen Debug UI";

fn main() -> eframe::Result<()> {
	let mut pipeline = Pipeline::new();

	// 1. todo: add brief description of pipeline stage
	pipeline.add_stage(make_points_stage());
	// todo: add similarly formatted comments for all planned pipeline stages

	let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_title(DEBUG_APP_NAME),
        ..Default::default()
    };

	let app = DebugVisualizer::new(pipeline);
    eframe::run_native(
        DEBUG_APP_NAME,
        options,
        Box::new(|_cc| {
			Box::new(app)
		}),
	)
}
