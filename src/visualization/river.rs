use crate::canvas::CanvasData;
use crate::params::ParamGroup;
use crate::pipeline::landmass::{LandmassOutput, STAGE_DATA_KEY as LANDMASS_KEY};
use crate::pipeline::river::{RiverOutput, STAGE_DATA_KEY as RIVER_KEY};
use crate::pipeline::StageDataMap;
use crate::visualization::VisualLayer;
use egui::{Painter, Rect, Ui};

#[derive(Default)]
pub struct RiverVisualLayer {
    pub enabled: bool,
}

impl VisualLayer for RiverVisualLayer {
    fn display_name(&self) -> &str { "Rivers" }
    fn is_enabled(&self) -> bool { self.enabled }
    fn set_enabled(&mut self, e: bool) { self.enabled = e; }

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

        let canvas = match data.get("canvas")
            .and_then(|d| d.as_any().downcast_ref::<CanvasData>())
        {
            Some(c) => c,
            None => return,
        };
        let ox = rect.center().x - canvas.width / 2.0;
        let oy = rect.center().y - canvas.height / 2.0;

        let landmass = match data.get(LANDMASS_KEY)
            .and_then(|d| d.as_any().downcast_ref::<LandmassOutput>())
        {
            Some(l) => l,
            None => return,
        };

        let river_out = match data.get(RIVER_KEY)
            .and_then(|d| d.as_any().downcast_ref::<RiverOutput>())
        {
            Some(r) => r,
            None => return,
        };

        let river_scale = params
            .and_then(|p| p.get_param("River Scale"))
            .and_then(|p| p.as_float())
            .unwrap_or(1.0);

        crate::visualization::draw_rivers(painter, ox, oy, river_scale, landmass, river_out);
    }
}
