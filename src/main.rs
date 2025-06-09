mod canvas;
mod params;
mod ui;

static DEBUG_APP_NAME: &str = "WorldGen Debug UI";

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
		viewport: egui::ViewportBuilder::default().with_title(DEBUG_APP_NAME),
		..Default::default()
	};

	eframe::run_native(DEBUG_APP_NAME, options, Box::new(|_cc|  Box::<crate::ui::DebugVisualizer>::default()))
}
