use crate::canvas::CanvasData;
use crate::params::ParamGroup;
use crate::pipeline::biome::{Biome, BiomeOutput, STAGE_DATA_KEY as BIOME_KEY};
use crate::pipeline::landmass::{LandmassOutput, STAGE_DATA_KEY as LANDMASS_KEY};
use crate::pipeline::river::{RiverOutput, STAGE_DATA_KEY as RIVER_KEY};
use crate::pipeline::StageDataMap;
use crate::visualization::VisualLayer;
use egui::{Color32, Painter, Pos2, Rect, Ui};

#[derive(Default)]
pub struct BiomeVisualLayer {
    pub enabled: bool,
    pub show_coastline: bool,
    pub show_rivers: bool,
}

impl VisualLayer for BiomeVisualLayer {
    fn display_name(&self) -> &str { "Biome" }
    fn is_enabled(&self) -> bool { self.enabled }
    fn set_enabled(&mut self, e: bool) { self.enabled = e; }

    fn draw_controls(&mut self, ui: &mut Ui, params: Option<&mut ParamGroup>) -> bool {
        let mut changed = false;
        let name = self.display_name().to_string();
        let enabled = &mut self.enabled;
        let show_coastline = &mut self.show_coastline;
        ui.collapsing(name, |ui| {
            changed |= ui.checkbox(enabled, "Enabled").changed();
            if *enabled {
                changed |= ui.checkbox(show_coastline, "Show Coastline").changed();
                changed |= ui.checkbox(&mut self.show_rivers, "Show Rivers").changed();
                if let Some(p) = params {
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
        let ox = rect.center().x - canvas.width / 2.0;
        let oy = rect.center().y - canvas.height / 2.0;

        let landmass = match data.get(LANDMASS_KEY)
            .and_then(|d| d.as_any().downcast_ref::<LandmassOutput>())
        {
            Some(l) => l,
            None => return,
        };

        let biome_out = match data.get(BIOME_KEY)
            .and_then(|d| d.as_any().downcast_ref::<BiomeOutput>())
        {
            Some(b) => b,
            None => return,
        };

        for (i, cell) in landmass.cells.iter().enumerate() {
            let biome = biome_out.cell_biomes.get(i).copied().unwrap_or(Biome::ShallowOcean);

            let points: Vec<Pos2> = cell.corner_ids.iter()
                .map(|&cid| {
                    let p = landmass.corners[cid].position;
                    Pos2::new(ox + p.x, oy + p.y)
                })
                .collect();
            if points.len() < 3 { continue; }

            painter.add(egui::Shape::convex_polygon(
                points,
                biome_color(biome),
                egui::Stroke::NONE,
            ));
        }

        if self.show_coastline {
            crate::visualization::draw_coastline(painter, ox, oy, landmass);
        }

        if self.show_rivers {
            if let Some(river_out) = data.get(RIVER_KEY)
                .and_then(|d| d.as_any().downcast_ref::<RiverOutput>())
            {
                crate::visualization::draw_rivers(painter, ox, oy, 1.0, landmass, river_out);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Color palette — vegetation structure tier
// ---------------------------------------------------------------------------

pub fn biome_color(biome: Biome) -> Color32 {
    match biome {
        // Water
        Biome::DeepOcean    => Color32::from_rgb(8,   20,  60),
        Biome::ShallowOcean => Color32::from_rgb(40,  100, 170),
        Biome::Lake         => Color32::from_rgb(70,  140, 190),
        // Coastal land
        Biome::Beach        => Color32::from_rgb(220, 205, 150),
        Biome::Wetland      => Color32::from_rgb(80,  130, 100),
        // Land vegetation structures
        Biome::Alpine       => Color32::from_rgb(210, 215, 225),
        Biome::Sparse       => Color32::from_rgb(200, 175, 110),
        Biome::Open         => Color32::from_rgb(160, 190, 85),
        Biome::Dense        => Color32::from_rgb(45,  115, 50),
        Biome::Riparian     => Color32::from_rgb(55,  150, 115),
        // System
        Biome::Border       => Color32::from_rgb(25,  25,  45),
    }
}
