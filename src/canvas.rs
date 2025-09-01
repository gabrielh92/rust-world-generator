use egui::{Painter, Rect, Sense, Ui};

use crate::mathlib::vec::Vec2;
use crate::params::{FloatParam, Param, ParamGroup};
use crate::pipeline::{StageData, StageDataMap};
use crate::visualization::{ColorKey, VisualLayer};

#[derive(Clone, Debug)]
pub struct CanvasData {
    pub width: f32,
    pub height: f32,
	pub center: Vec2,
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
                changed_width = self
                    .config
                    .get_param_mut("width")
                    .map(|p| p.draw(ui))
                    .unwrap_or(false);
                changed_height = self
                    .config
                    .get_param_mut("height")
                    .map(|p| p.draw(ui))
                    .unwrap_or(false);
            }
        });
        changed_width | changed_height
    }

    fn draw_canvas(&self, painter: &egui::Painter, rect: &Rect, _params: Option<&ParamGroup>, data: &StageDataMap) {
		let canvas_size = egui::Vec2 { x: self.width(), y: self.height() };
		let canvas_rect = egui::Rect::from_center_size(rect.center(), canvas_size);

		let backdrop_size = canvas_size * 1.25;
		let backdrop_rect = Rect::from_center_size(rect.center(), backdrop_size);

		// === Draw backdrop frame ===
		painter.rect_filled(backdrop_rect, 0.0, ColorKey::CanvasBackdrop.egui32());

		// === Draw main canvas background ===
		painter.rect_filled(canvas_rect, 0.0, ColorKey::Canvas.egui32());
	}
}
