use crate::canvas::CanvasData;
use crate::params::ParamGroup;
use crate::pipeline::feature::{Feature, FeatureOutput, STAGE_DATA_KEY as FEATURE_KEY};
use crate::pipeline::landmass::{LandmassOutput, STAGE_DATA_KEY as LANDMASS_KEY};
use crate::pipeline::StageDataMap;
use crate::visualization::VisualLayer;
use egui::{Color32, Painter, Pos2, Rect, Ui};

#[derive(Default)]
pub struct FeatureVisualLayer {
    pub enabled: bool,
}

impl VisualLayer for FeatureVisualLayer {
    fn display_name(&self) -> &str { "Features" }
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

        let feature_out = match data.get(FEATURE_KEY)
            .and_then(|d| d.as_any().downcast_ref::<FeatureOutput>())
        {
            Some(f) => f,
            None => return,
        };

        let icon_size = params
            .and_then(|p| p.get_param("Icon Size"))
            .and_then(|p| p.as_float())
            .unwrap_or(4.0);

        let show_river_mouths = params
            .and_then(|p| p.get_param("Show River Mouths"))
            .and_then(|p| p.as_bool())
            .unwrap_or(true);
        let show_harbor_candidates = params
            .and_then(|p| p.get_param("Show Harbor Candidates"))
            .and_then(|p| p.as_bool())
            .unwrap_or(true);
        let show_mountain_passes = params
            .and_then(|p| p.get_param("Show Mountain Passes"))
            .and_then(|p| p.as_bool())
            .unwrap_or(true);
        let show_fertile_valleys = params
            .and_then(|p| p.get_param("Show Fertile Valleys"))
            .and_then(|p| p.as_bool())
            .unwrap_or(true);
        let show_resource_nodes = params
            .and_then(|p| p.get_param("Show Resource Nodes"))
            .and_then(|p| p.as_bool())
            .unwrap_or(true);

        for (i, cell) in landmass.cells.iter().enumerate() {
            let features = match feature_out.cell_features.get(i) {
                Some(f) => f,
                None => continue,
            };
            if features.is_empty() { continue; }

            let cx = ox + cell.center.x;
            let cy = oy + cell.center.y;
            let center = Pos2::new(cx, cy);

            for feature in features {
                let (color, size_factor, enabled) = match feature {
                    Feature::RiverMouth      => (Color32::from_rgb(30, 100, 220),  1.00, show_river_mouths),
                    Feature::HarborCandidate => (Color32::from_rgb(0, 180, 180),   0.90, show_harbor_candidates),
                    Feature::MountainPass    => (Color32::from_rgb(180, 120, 40),  0.85, show_mountain_passes),
                    Feature::FertileValley   => (Color32::from_rgb(50, 180, 60),   0.80, show_fertile_valleys),
                    Feature::ResourceNode    => (Color32::from_rgb(220, 190, 30),  0.70, show_resource_nodes),
                };
                if !enabled { continue; }
                painter.circle_filled(center, icon_size * size_factor, color);
            }
        }
    }
}
