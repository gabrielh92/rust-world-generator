use std::collections::HashSet;

use crate::canvas::CanvasData;
use crate::pipeline::voronoi::VoronoiOutput;
use crate::pipeline::StageDataMap;
use crate::util::make_stage_data_key;
use crate::{params::ParamGroup, visualization::VisualLayer};
use crate::visualization::ColorKey;
use egui::{FontId, Painter, Pos2, Rect, Stroke, Ui};

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
				let mut visited_triangles: HashSet<&Vec<usize>> = HashSet::new();
                for cell in &voronoi.cells {
                    let region = cell
                        .vertices
                        .iter()
                        .map(|v| Pos2::new(x_center_offset + v.x, y_center_offset + v.y))
                        .collect::<Vec<_>>();

					// Draw voronoi polygon
                    for i in 0..region.len() {
                        let a = region[i];
                        let b = region[(i + 1) % region.len()];
                        painter.line_segment([a, b], Stroke::new(1.0, ColorKey::VoronoiLines.egui32()));
                    }

					// Draw delauney triangle
					for triangle in &cell.triangles {
						if visited_triangles.insert(triangle) {
							let [a, b, c]: [usize; 3] = [triangle[0], triangle[1], triangle[2]];

							for &(i, j) in &[(a, b), (b, c), (c, a)] {
								let edge = (i, j);
								let pa = voronoi.cells.iter().find(|it| it.index == edge.0).unwrap();
								let pb = voronoi.cells.iter().find(|it| it.index == edge.1).unwrap();

								let p1 = egui::Pos2 { x: x_center_offset + pa.centroid.x, y: y_center_offset + pa.centroid.y };
								let p2 = egui::Pos2 { x: x_center_offset + pb.centroid.x, y: y_center_offset + pb.centroid.y };
								let dir = (p2 - p1).normalized();
								let offset = 10.; // recession offset from site position
								let p1_recede = p1 + dir * offset;
								let p2_recede = p2 - dir * offset;
								painter.line_segment([p1_recede, p2_recede], Stroke::new(0.5, ColorKey::DelauneyLines.egui32()));
							}
						}
					}

					// Draw cell centroid with site index
					painter.text(
						egui::Pos2{ x: x_center_offset + cell.centroid.x, y: y_center_offset + cell.centroid.y},
						egui::Align2::CENTER_CENTER,
						format!("{}", cell.index),
						FontId::proportional(12.),
						ColorKey::PointAlt.egui32()
					);
				}
            }
        }
    }
}
