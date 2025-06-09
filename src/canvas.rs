use egui::{Color32, Painter, Rect, Sense, Ui, Vec2};

use crate::params::{FloatParam, Param};

pub fn show_canvas(ui: &mut Ui, canvas_config: &CanvasConfig) -> (Rect, Painter) {
    // Assume you've extracted f32 values from your FloatParams
    let width = canvas_config.width.as_float().unwrap_or(800.0);
    let height = canvas_config.height.as_float().unwrap_or(600.0);
    let canvas_size = Vec2::new(width, height);

    // Allocate painter space in the central panel
    let (response, painter) = ui.allocate_painter(canvas_size, Sense::hover());

    // Optional: Draw a dark background for visibility
    painter.rect_filled(response.rect, 0.0, Color32::WHITE);

    (response.rect, painter)
}

// todo: refactor into standalone struct `ConfigPanel` and have CanvasPanel implement it
pub struct CanvasPanel {
	pub title: String,
	pub enabled: bool,
	pub config: CanvasConfig,
}

impl Default for CanvasPanel {
	fn default() -> Self {
		Self { title: "Canvas Settings".into(), enabled: true, config: CanvasConfig::default() }
	}
}

impl CanvasPanel {
	pub fn draw_controls(&mut self, ui: &mut Ui) {
		ui.collapsing(&self.title, |ui| {
			ui.checkbox(&mut self.enabled, "Enabled");
			if self.enabled {
                self.config.width.draw(ui);
                self.config.height.draw(ui);
            }
		});
	}

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }
}

pub struct CanvasConfig {
	pub width: Param,
	pub height: Param,
}

impl Default for CanvasConfig {
	fn default() -> Self {
		Self {
			width: Param {
				name: "Canvas Width".into(),
				tooltip: Some("Canvas width in pixels".into()),
				value: Box::new(FloatParam {
					val: 800.,
					min: 400.,
					max: 2400.,
					step: 100.,
				}),
			},
			height: Param {
				name: "Canvas Height".into(),
				tooltip: Some("Canvas height in pixels".into()),
				value: Box::new(FloatParam {
					val: 600.,
					min: 200.,
					max: 1200.,
					step: 100.,
				})
			}
		}
	}
}
