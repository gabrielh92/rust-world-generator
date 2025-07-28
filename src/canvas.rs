use egui::{Color32, Painter, Rect, Sense, Ui, Vec2};

use crate::params::{FloatParam, Param, ParamGroup};
use crate::pipeline::{StageData, StageDataMap};
use crate::visualization::VisualLayer;

#[derive(Clone, Debug)]
pub struct CanvasData {
	pub width: f32,
	pub height: f32,
}

impl StageData for CanvasData {
	fn as_any(&self) -> &dyn std::any::Any {
		self
	}
}

pub struct CanvasLayer {
    pub config: ParamGroup,
}

impl Default for CanvasLayer {
    fn default() -> Self {
        Self {
            config: ParamGroup {
                title: "Canvas Settings".into(),
                enabled: true,
                params: vec![
                    Param {
                        name: "width".into(),
                        tooltip: Some("Width in px".into()),
                        value: Box::new(FloatParam {
                            val: 800.,
                            min: 400.,
                            max: 2400.,
                            step: 100.,
                        }),
                    },
                    Param {
                        name: "height".into(),
                        tooltip: Some("Height in px".into()),
                        value: Box::new(FloatParam {
                            val: 600.,
                            min: 200.,
                            max: 1200.,
                            step: 100.,
                        }),
                    },
                ],
            },
        }
    }
}

impl CanvasLayer {
    pub fn width(&self) -> f32 {
        self.config
            .get_param("width")
            .and_then(|p| p.as_float())
            .unwrap()
    }

    pub fn height(&self) -> f32 {
        self.config
            .get_param("height")
            .and_then(|p| p.as_float())
            .unwrap()
    }
}

impl VisualLayer for CanvasLayer {
    fn display_name(&self) -> &str {
        &self.config.title
    }

    fn is_enabled(&self) -> bool {
        self.config.enabled
    }

    fn set_enabled(&mut self, enabled: bool) {
        self.config.enabled = enabled;
    }

    fn draw_controls(&mut self, ui: &mut egui::Ui, _params: Option<&mut ParamGroup>) -> bool {
		let (mut changed_width, mut changed_height) = (false, false);
        let name = self.display_name().to_string();
        ui.collapsing(name, |ui| {
            ui.checkbox(&mut self.is_enabled(), "Enabled");
            if self.is_enabled() {
                changed_width = self.config.get_param_mut("width").map(|p| p.draw(ui)).unwrap_or(false);
                changed_height = self.config.get_param_mut("height").map(|p| p.draw(ui)).unwrap_or(false);
            }
        });
		changed_width | changed_height
    }

    fn draw_canvas(&self, painter: &egui::Painter, _params: Option<&ParamGroup>, _data: &StageDataMap) {
        let width = self.width();
        let height = self.height();
        painter.rect_filled(painter.clip_rect(), 0.0, egui::Color32::WHITE);
        painter.text(
            egui::pos2(10.0, 10.0),
            egui::Align2::LEFT_TOP,
            format!("Canvas: {width} x {height}"),
            egui::FontId::monospace(14.0),
            Color32::DARK_GRAY,
        );
    }
}

pub fn show_canvas(ui: &mut Ui, canvas_layer: &CanvasLayer) -> (Rect, Painter) {
    let canvas_size = Vec2::new(canvas_layer.width(), canvas_layer.height());

    // Allocate painter space in the central panel
    let (response, painter) = ui.allocate_painter(canvas_size, Sense::hover());

    // Optional: Draw a dark background for visibility
    painter.rect_filled(response.rect, 0.0, Color32::WHITE);
	painter.rect_stroke(response.rect, 0.0, egui::Stroke::new(1.0, Color32::RED));

    (response.rect, painter)
}
