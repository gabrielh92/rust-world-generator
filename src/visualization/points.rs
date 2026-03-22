use crate::canvas::CanvasData;
use crate::{params::ParamGroup, visualization::ColorKey};
use crate::pipeline::points::{PointsOutput, STAGE_DATA_KEY as POINTS_KEY};
use crate::pipeline::StageDataMap;
use crate::visualization::VisualLayer;
use egui::{Painter, Rect, Ui};

pub struct PointsVisualLayer {
    enabled: bool,
}

impl Default for PointsVisualLayer {
    fn default() -> Self { Self { enabled: false } }
}

impl VisualLayer for PointsVisualLayer {
    fn display_name(&self) -> &str { "Points" }
    fn is_enabled(&self) -> bool { self.enabled }
    fn set_enabled(&mut self, enabled: bool) { self.enabled = enabled; }

    fn draw_controls(&mut self, ui: &mut Ui, params: Option<&mut ParamGroup>) -> bool {
        let mut changed = false;
        let name = self.display_name().to_string();
        let enabled = &mut self.enabled;
        ui.collapsing(name, |ui| {
            changed |= ui.checkbox(enabled, "Enabled").changed();
            if *enabled {
                if let Some(p) = params {
                    for param in &mut p.params {
                        changed |= param.draw(ui);
                    }
                }
            }
        });
        changed
    }

    fn draw_canvas(&self, painter: &Painter, rect: &Rect, params: Option<&ParamGroup>, data: &StageDataMap) {
        if !self.enabled { return; }

        let canvas = data
            .get("canvas")
            .and_then(|c| c.as_any().downcast_ref::<CanvasData>())
            .expect("Canvas data required for points drawing");

        let ox = rect.center().x - canvas.width / 2.0;
        let oy = rect.center().y - canvas.height / 2.0;

        let radius = params
            .and_then(|p| p.get_param("Point Radius"))
            .and_then(|p| p.as_float())
            .unwrap_or(2.0);

        if let Some(output) = data
            .get(POINTS_KEY)
            .and_then(|d| d.as_any().downcast_ref::<PointsOutput>())
        {
            for pos in &output.points {
                painter.circle_filled(
                    egui::pos2(ox + pos.x, oy + pos.y),
                    radius,
                    ColorKey::Point.egui32(),
                );
            }
        }
    }
}
