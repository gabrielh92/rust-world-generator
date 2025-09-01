use crate::canvas::CanvasData;
use crate::pipeline::voronoi::VoronoiOutput;
use crate::pipeline::StageDataMap;
use crate::util::make_stage_data_key;
use crate::{params::ParamGroup, visualization::VisualLayer};
use crate::visualization::ColorKey;
use egui::{Painter, Pos2, Rect, Stroke, Ui};

#[derive(Default)]
pub struct VoronoiVisualLayer {
    pub enabled: bool,
}

impl VisualLayer for VoronoiVisualLayer {
    fn display_name(&self) -> &str {
        "Voronoi Layer"
    }

    fn is_enabled(&self) -> bool {
        self.enabled
    }
    fn set_enabled(&mut self, e: bool) {
        self.enabled = e
    }

    fn draw_controls(&mut self, ui: &mut Ui, _params: Option<&mut ParamGroup>) -> bool {
		let mut changed = false;
		let name = self.display_name().to_string();

		ui.collapsing(name, |ui| {
			changed |= ui.checkbox(&mut self.enabled, "Enabled").changed();
		});

		false // toggling voronoi mask should not re-run pipeline
    }

    fn draw_canvas(&self, painter: &Painter, rect: &Rect, _params: Option<&ParamGroup>, data: &StageDataMap) {
        if !self.enabled {
            return;
        }

		let canvas = data
			.get("canvas")
			.and_then(|c| c.as_any().downcast_ref::<CanvasData>())
			.expect("Canvas data defined for voronoi drawing");
		let x_center_offset = rect.center().x - (canvas.width / 2.);
		let y_center_offset = rect.center().y - (canvas.height / 2.);

        if let Some(output) = data.get(&make_stage_data_key("voronoi", 2)) {
            if let Some(voronoi) = output.as_any().downcast_ref::<VoronoiOutput>() {
                for cell in &voronoi.cells {
                    let region = cell
                        .vertices
                        .iter()
                        .map(|v| Pos2::new(x_center_offset + v.x, y_center_offset + v.y))
                        .collect::<Vec<_>>();

                    for i in 0..region.len() {
                        let a = region[i];
                        let b = region[(i + 1) % region.len()];
                        painter.line_segment([a, b], Stroke::new(1.0, ColorKey::VoronoiLines.egui32()));
                    }
                }
            }
        }
    }
}
