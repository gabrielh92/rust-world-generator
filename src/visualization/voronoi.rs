use std::collections::HashSet;

use crate::canvas::CanvasData;
use crate::pipeline::voronoi::{GraphOutput, STAGE_DATA_KEY as VORONOI_KEY};
use crate::pipeline::StageDataMap;
use crate::{params::ParamGroup, visualization::VisualLayer};
use crate::visualization::ColorKey;
use egui::{FontId, Painter, Pos2, Rect, Stroke, Ui};

#[derive(Default)]
pub struct VoronoiVisualLayer {
    pub enabled: bool,
}

impl VisualLayer for VoronoiVisualLayer {
    fn display_name(&self) -> &str { "Voronoi Graph" }
    fn is_enabled(&self) -> bool { self.enabled }
    fn set_enabled(&mut self, e: bool) { self.enabled = e; }

    fn draw_controls(&mut self, ui: &mut Ui, params: Option<&mut ParamGroup>) -> bool {
        let mut changed = false;
        let name = self.display_name().to_string();
        ui.collapsing(name, |ui| {
            changed |= ui.checkbox(&mut self.enabled, "Enabled").changed();
            if self.enabled {
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
            .expect("Canvas data required for voronoi drawing");

        let ox = rect.center().x - canvas.width / 2.0;
        let oy = rect.center().y - canvas.height / 2.0;
        let offset = |v: egui::Pos2| Pos2::new(ox + v.x, oy + v.y);

        let show_labels   = params.and_then(|p| p.get_param("Show Cell Labels")).and_then(|p| p.as_bool()).unwrap_or(false);
        let show_delaunay = params.and_then(|p| p.get_param("Show Delaunay")).and_then(|p| p.as_bool()).unwrap_or(true);

        let graph = match data
            .get(VORONOI_KEY)
            .and_then(|d| d.as_any().downcast_ref::<GraphOutput>())
        {
            Some(g) => g,
            None => return,
        };

        let voronoi_stroke  = Stroke::new(0.8, ColorKey::VoronoiLines.egui32());
        let delaunay_stroke = Stroke::new(0.5, ColorKey::DelaunayLines.egui32());

        // Draw Voronoi polygon edges via corners
        for cell in &graph.cells {
            let n = cell.corner_ids.len();
            if n < 2 { continue; }
            for i in 0..n {
                let a = graph.corners[cell.corner_ids[i]].position;
                let b = graph.corners[cell.corner_ids[(i + 1) % n]].position;
                painter.line_segment(
                    [offset(a.into()), offset(b.into())],
                    voronoi_stroke,
                );
            }
        }

        // Draw Delaunay edges (each shared edge drawn once)
        if show_delaunay {
            let mut drawn: HashSet<(usize, usize)> = HashSet::new();
            for cell in &graph.cells {
                for &nid in &cell.neighbor_ids {
                    let edge = if cell.id < nid { (cell.id, nid) } else { (nid, cell.id) };
                    if drawn.insert(edge) {
                        if let Some(nbr) = graph.cells.iter().find(|c| c.id == nid) {
                            let p1: Pos2 = offset(cell.center.into());
                            let p2: Pos2 = offset(nbr.center.into());
                            // Slightly recede endpoints from site centres for clarity
                            let dir = (p2 - p1).normalized();
                            let recession = 8.0_f32;
                            painter.line_segment(
                                [p1 + dir * recession, p2 - dir * recession],
                                delaunay_stroke,
                            );
                        }
                    }
                }
            }
        }

        // Draw cell labels (optional — avoid with large meshes)
        if show_labels {
            for cell in &graph.cells {
                painter.text(
                    offset(cell.center.into()),
                    egui::Align2::CENTER_CENTER,
                    format!("{}", cell.id),
                    FontId::proportional(10.),
                    ColorKey::PointAlt.egui32(),
                );
            }
        }
    }
}
