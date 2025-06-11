use egui::{Color32, Painter, Rect, Sense, Ui, Vec2};

use crate::params::{FloatParam, Param, ParamGroup};
use crate::pipeline::StageOutputs;
use crate::visualization::VisualLayer;

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
            .unwrap_or(800.0)
    }

    pub fn height(&self) -> f32 {
        self.config
            .get_param("height")
            .and_then(|p| p.as_float())
            .unwrap_or(600.0)
    }
}

impl VisualLayer for CanvasLayer {
    fn name(&self) -> &str {
        &self.config.title
    }

    fn is_enabled(&self) -> bool {
        self.config.enabled
    }

    fn set_enabled(&mut self, enabled: bool) {
        self.config.enabled = enabled;
    }

    fn params(&mut self) -> Option<&mut ParamGroup> {
        None // Canvas config is global; no dynamic params here
    }

    fn draw_controls(&mut self, ui: &mut egui::Ui) {
        let name = self.name().to_string();
        ui.collapsing(name, |ui| {
            ui.checkbox(&mut self.is_enabled(), "Enabled");
            if self.is_enabled() {
                self.config.get_param_mut("width").map(|p| p.draw(ui));
                self.config.get_param_mut("height").map(|p| p.draw(ui));
            }
        });
    }

    fn draw_canvas(&self, painter: &egui::Painter, canvas: &CanvasLayer, _data: &StageOutputs) {
        let width = canvas.width();
        let height = canvas.height();
        painter.rect_filled(painter.clip_rect(), 0.0, egui::Color32::WHITE);
        painter.text(
            egui::pos2(10.0, 10.0),
            egui::Align2::LEFT_TOP,
            format!("Canvas: {width} x {height}"),
            egui::FontId::monospace(14.0),
            egui::Color32::DARK_GRAY,
        );
    }
}

pub fn show_canvas(ui: &mut Ui, canvas_layer: &CanvasLayer) -> (Rect, Painter) {
    let canvas_size = Vec2::new(canvas_layer.width(), canvas_layer.height());

    // Allocate painter space in the central panel
    let (response, painter) = ui.allocate_painter(canvas_size, Sense::hover());

    // Optional: Draw a dark background for visibility
    painter.rect_filled(response.rect, 0.0, Color32::WHITE);

    (response.rect, painter)
}
