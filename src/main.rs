mod canvas;
mod mathlib;
mod params;
mod pipeline;
mod ui;
mod util;
mod visualization;

use eframe::egui;
use pipeline::Pipeline;

use crate::{
    pipeline::{
        elevation::make_elevation_stage,
        landmass::make_landmass_stage,
        points::make_points_stage,
        voronoi::make_voronoi_stage,
    },
    ui::DebugVisualizer,
};

static DEBUG_APP_NAME: &str = "WorldGen Debug UI";
static WINDOW_WIDTH: f32 = 1800.;
static WINDOW_HEIGHT: f32 = 1200.;

fn main() -> eframe::Result<()> {
    let mut pipeline = Pipeline::new();

    // 1. todo: add brief description of pipeline stage
    pipeline.add_stage(make_points_stage());
    pipeline.add_stage(make_voronoi_stage());
    pipeline.add_stage(make_landmass_stage());
    pipeline.add_stage(make_elevation_stage());

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
		.with_title(DEBUG_APP_NAME)
		.with_inner_size(egui::Vec2 {x: WINDOW_WIDTH, y: WINDOW_HEIGHT}),

        ..Default::default()
    };

    let app = DebugVisualizer::new(pipeline);
    eframe::run_native(DEBUG_APP_NAME, options, Box::new(|_cc| Box::new(app)))
}
