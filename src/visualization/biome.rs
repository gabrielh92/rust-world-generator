use crate::canvas::CanvasData;
use crate::params::ParamGroup;
use crate::pipeline::biome::{Biome, BiomeOutput, STAGE_DATA_KEY as BIOME_KEY};
use crate::pipeline::landmass::{LandmassOutput, STAGE_DATA_KEY as LANDMASS_KEY};
use crate::pipeline::StageDataMap;
use crate::visualization::VisualLayer;
use egui::{Color32, Painter, Pos2, Rect, Ui};

#[derive(Default)]
pub struct BiomeVisualLayer {
    pub enabled: bool,
    pub show_coastline: bool,
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
            let biome = biome_out.cell_biomes.get(i).copied().unwrap_or(Biome::OpenOcean);

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
    }
}

// ---------------------------------------------------------------------------
// Biome color palette
// ---------------------------------------------------------------------------

pub fn biome_color(biome: Biome) -> Color32 {
    match biome {
        // Ocean
        Biome::DeepTrench        => Color32::from_rgb(8,   20,  60),
        Biome::OpenOcean         => Color32::from_rgb(30,  80,  160),
        Biome::ShallowCoast      => Color32::from_rgb(70,  140, 200),
        Biome::Lake              => Color32::from_rgb(80,  120, 155),
        // Special
        Biome::Border            => Color32::from_rgb(30,  30,  50),
        Biome::Beach             => Color32::from_rgb(220, 205, 150),
        Biome::Wetland           => Color32::from_rgb(90,  140, 100),
        Biome::Mangrove          => Color32::from_rgb(50,  100, 70),
        // High elevation
        Biome::AlpineTundra      => Color32::from_rgb(200, 200, 210),
        Biome::AlpineMeadow      => Color32::from_rgb(160, 185, 150),
        Biome::AlpineForest      => Color32::from_rgb(80,  120, 90),
        // Mid elevation
        Biome::Shrubland         => Color32::from_rgb(180, 160, 100),
        Biome::TemperateForest   => Color32::from_rgb(60,  130, 60),
        Biome::Rainforest        => Color32::from_rgb(20,  100, 40),
        // Low elevation
        Biome::Desert            => Color32::from_rgb(210, 185, 120),
        Biome::GrasslandSavanna  => Color32::from_rgb(160, 185, 80),
        Biome::TropicalRainforest => Color32::from_rgb(10, 120, 50),
    }
}
