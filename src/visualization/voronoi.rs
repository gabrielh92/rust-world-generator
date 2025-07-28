use crate::pipeline::voronoi::VoronoiOutput;
use crate::pipeline::StageDataMap;
use crate::util::make_stage_data_key;
use crate::{params::ParamGroup, visualization::VisualLayer};
use egui::{Color32, Painter, Pos2, Rect, Stroke, TextBuffer, Ui};

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
        ui.checkbox(&mut self.enabled, "Enabled").changed()
    }

    fn draw_canvas(&self, painter: &Painter, _params: Option<&ParamGroup>, data: &StageDataMap) {
        if !self.enabled {
            return;
        }

        if let Some(output) = data.get(&make_stage_data_key("voronoi", 2)) {
            if let Some(voronoi) = output.as_any().downcast_ref::<VoronoiOutput>() {
                for cell in &voronoi.cells {
                    let region = cell
                        .vertices
                        .iter()
                        .map(|v| Pos2::new(v.x, v.y))
                        .collect::<Vec<_>>();

                    for i in 0..region.len() {
                        let a = region[i];
                        let b = region[(i + 1) % region.len()];
                        painter.line_segment([a, b], Stroke::new(1.0, Color32::LIGHT_GRAY));
                    }
                }
            }
        }
    }
}
