use egui::Rect;

use crate::mathlib::vec::Vec2;
use crate::params::{FloatParam, IntParam, Param, ParamGroup};
use crate::pipeline::{StageData, StageDataMap};
use crate::visualization::{ColorKey, VisualLayer};

#[derive(Clone, Debug)]
pub struct CanvasData {
    pub width: f32,
    pub height: f32,
    pub center: Vec2,
    /// World seed used by all pipeline stages for deterministic generation.
    pub seed: u64,
}

impl StageData for CanvasData {
    fn as_any(&self) -> &dyn std::any::Any { self }
}

pub struct CanvasLayer {
    pub config: ParamGroup,
}

impl Default for CanvasLayer {
    fn default() -> Self {
        Self {
            config: ParamGroup {
                title: "Canvas".into(),
                enabled: true,
                params: vec![
                    Param {
                        name: "Width".into(),
                        tooltip: Some("Canvas width in pixels".into()),
                        visual_only: false,
                        is_divider: false,
                        value: Box::new(FloatParam { val: 800., min: 400., max: 2400., step: 100. }),
                    },
                    Param {
                        name: "Height".into(),
                        tooltip: Some("Canvas height in pixels".into()),
                        visual_only: false,
                        is_divider: false,
                        value: Box::new(FloatParam { val: 600., min: 200., max: 1200., step: 100. }),
                    },
                    Param {
                        name: "Seed".into(),
                        tooltip: Some("World seed — same seed always produces identical output".into()),
                        visual_only: false,
                        is_divider: false,
                        value: Box::new(IntParam { val: 42, min: 0, max: 99999, step: 1 }),
                    },
                ],
            },
        }
    }
}

impl CanvasLayer {
    pub fn width(&self) -> f32 {
        self.config.get_param("Width").and_then(|p| p.as_float()).unwrap_or(800.)
    }
    pub fn height(&self) -> f32 {
        self.config.get_param("Height").and_then(|p| p.as_float()).unwrap_or(600.)
    }
    pub fn seed(&self) -> u64 {
        self.config.get_param("Seed").and_then(|p| p.as_int()).unwrap_or(0) as u64
    }
}

impl VisualLayer for CanvasLayer {
    fn display_name(&self) -> &str { "Canvas" }
    fn is_enabled(&self) -> bool { self.config.enabled }
    fn set_enabled(&mut self, enabled: bool) { self.config.enabled = enabled; }

    fn draw_controls(&mut self, ui: &mut egui::Ui, _params: Option<&mut ParamGroup>) -> bool {
        let mut changed = false;
        let name = self.display_name().to_string();
        ui.collapsing(name, |ui| {
            changed |= ui.checkbox(&mut self.config.enabled, "Enabled").changed();
            if self.config.enabled {
                for param in &mut self.config.params {
                    changed |= param.draw(ui);
                }
            }
        });
        changed
    }

    fn draw_canvas(&self, painter: &egui::Painter, rect: &Rect, _params: Option<&ParamGroup>, _data: &StageDataMap) {
        let canvas_size = egui::Vec2 { x: self.width(), y: self.height() };
        let canvas_rect = egui::Rect::from_center_size(rect.center(), canvas_size);
        let backdrop_size = canvas_size * 1.15;
        let backdrop_rect = Rect::from_center_size(rect.center(), backdrop_size);
        painter.rect_filled(backdrop_rect, 4.0, ColorKey::CanvasBackdrop.egui32());
        painter.rect_filled(canvas_rect, 0.0, ColorKey::Canvas.egui32());
    }
}
