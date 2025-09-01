use crate::canvas::CanvasData;
use crate::{params::ParamGroup, visualization::ColorKey};
use crate::pipeline::points::PointsOutput;
use crate::pipeline::StageDataMap;
use crate::util::make_stage_data_key;
use crate::visualization::VisualLayer;
use egui::{Painter, Rect, Ui};

pub struct PointsVisualLayer {
    enabled: bool,
}

impl Default for PointsVisualLayer {
    fn default() -> Self {
        Self { enabled: false }
    }
}

impl VisualLayer for PointsVisualLayer {
    fn display_name(&self) -> &str {
        "Points"
    }

    fn is_enabled(&self) -> bool {
        self.enabled
    }

    fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    fn draw_controls(&mut self, ui: &mut Ui, params: Option<&mut ParamGroup>) -> bool {
        let mut changed = false;

        let name = self.display_name().to_string();
        let enabled = &mut self.enabled;

        ui.collapsing(name, |ui| {
            changed |= ui.checkbox(enabled, "Enabled").changed();

            if *enabled {
                if let Some(p) = params {
                    // Don't call p.draw_controls(ui), because it includes its own collapsible.
                    // Instead, draw just the inner fields manually:
                    for param in &mut p.params {
                        changed |= param.draw(ui);
                    }
                }
            }
        });

        changed
    }

    fn draw_canvas(&self, painter: &Painter, rect: &Rect, params: Option<&ParamGroup>, data: &StageDataMap) {
        if !self.enabled {
            return;
        }

		let canvas = data
			.get("canvas")
			.and_then(|c| c.as_any().downcast_ref::<CanvasData>())
			.expect("Canvas data defined for points drawing");
		let x_center_offset = rect.center().x - (canvas.width / 2.);
		let y_center_offset = rect.center().y - (canvas.height / 2.);

		let params = params.unwrap();
        let radius = params
            .get_param("Point Radius")
            .and_then(|p| p.as_float())
            .unwrap();


        if let Some(output) = data.get(make_stage_data_key("points", 1).as_str()) {
            if let Some(points) = output.as_any().downcast_ref::<PointsOutput>() {
                for pos in &points.points {
                    let canvas_pos = egui::pos2(x_center_offset + pos.x, y_center_offset + pos.y);
                    painter.circle_filled(canvas_pos, radius, ColorKey::Point.egui32());
                }
            }
        }
    }
}
