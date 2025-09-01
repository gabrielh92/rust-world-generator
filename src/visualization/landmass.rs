use crate::canvas::CanvasData;
use crate::{pipeline::voronoi::VoronoiOutput, util::make_stage_data_key};
use crate::visualization::{ColorKey, VisualLayer};
use crate::params::ParamGroup;
use crate::pipeline::StageDataMap;
use crate::pipeline::landmass::LandmassOutput;
use egui::{Painter, Pos2, Rect, Ui};

#[derive(Default)]
pub struct LandmassVisualLayer {
    pub enabled: bool,
}

impl VisualLayer for LandmassVisualLayer {
    fn display_name(&self) -> &str { "Landmass Layer" }
    fn is_enabled(&self) -> bool { self.enabled }
    fn set_enabled(&mut self, e: bool) { self.enabled = e }

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
        if !self.enabled { return; }

		let canvas = data.get("canvas")
			.and_then(|d| d.as_any().downcast_ref::<CanvasData>())
			.unwrap();
		let x_center_offset = rect.center().x - (canvas.width / 2.);
		let y_center_offset = rect.center().y - (canvas.height / 2.);

		let voronoi = data.get(make_stage_data_key("voronoi", 2).as_str())
			.and_then(|d| d.as_any().downcast_ref::<VoronoiOutput>())
			.unwrap();

        if let Some(output) = data.get(make_stage_data_key("landmass", 3).as_str()) {
            if let Some(mask) = output.as_any().downcast_ref::<LandmassOutput>() {
				for (cell, voronoi_cell) in mask.cells.iter().zip(&voronoi.cells) {
					let color = if cell.elevation > 0.0 { ColorKey::Base.egui32() } else { ColorKey::Water.egui32() };

					let points: Vec<Pos2> = voronoi_cell.vertices
						.iter()
						.map(|v| egui::pos2(x_center_offset + v.x, y_center_offset + v.y))
						.collect();

					painter.add(egui::Shape::convex_polygon(points, color, egui::Stroke::NONE));
				}
            }
        }
    }
}
