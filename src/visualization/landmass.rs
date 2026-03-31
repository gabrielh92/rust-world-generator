use crate::canvas::CanvasData;
use crate::visualization::{elevation_color, lerp_color, ColorKey, VisualLayer};
use crate::params::ParamGroup;
use crate::pipeline::StageDataMap;
use crate::pipeline::terrain::{TerrainOutput, STAGE_DATA_KEY as TERRAIN_KEY};
use egui::{Painter, Pos2, Rect, Ui};

#[derive(Default)]
pub struct LandmassVisualLayer {
    pub enabled: bool,
}

impl VisualLayer for LandmassVisualLayer {
    fn display_name(&self) -> &str { "Landmass" }
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

        let pg = match params { Some(p) => p, None => return };

        let water_level = pg.get_param("Water Level").and_then(|p| p.as_float()).unwrap_or(0.0);
        let view_mode   = pg.get_param("View Mode").map(|p| p.value.as_str()).unwrap_or("Elevation");

        let canvas = data.get("canvas")
            .and_then(|d| d.as_any().downcast_ref::<CanvasData>())
            .unwrap();

        let ox = rect.center().x - canvas.width / 2.0;
        let oy = rect.center().y - canvas.height / 2.0;

        let output = match data
            .get(TERRAIN_KEY)
            .and_then(|d| d.as_any().downcast_ref::<TerrainOutput>())
        {
            Some(o) => o,
            None => return,
        };

        for cell in &output.cells {
            // Collect polygon vertices from corner positions
            let points: Vec<Pos2> = cell.corner_ids.iter()
                .map(|&cid| {
                    let p = output.corners[cid].position;
                    Pos2::new(ox + p.x, oy + p.y)
                })
                .collect();

            if points.len() < 3 { continue; }

            let fill = match view_mode {
                "Land Type" => {
                    if cell.is_border {
                        ColorKey::Border.egui32()
                    } else if cell.is_coast {
                        ColorKey::Coast.egui32()
                    } else if cell.is_land {
                        // Tint land slightly by elevation
                        lerp_color(
                            ColorKey::Land.egui32(),
                            egui::Color32::from_rgb(230, 230, 230),
                            (cell.elevation * 0.6).clamp(0.0, 1.0),
                        )
                    } else {
                        // Ocean: shade by depth
                        lerp_color(
                            ColorKey::Ocean.egui32(),
                            egui::Color32::from_rgb(10, 25, 70),
                            (-cell.elevation * 0.8).clamp(0.0, 1.0),
                        )
                    }
                }
                _ => {
                    // Elevation mode: use water_level as the land/ocean threshold
                    if cell.is_border {
                        ColorKey::Border.egui32()
                    } else {
                        let display_elev = if cell.elevation > water_level {
                            // Remap land part to [0, 1]
                            (cell.elevation - water_level) / (1.0 - water_level)
                        } else {
                            // Remap ocean part to [-1, 0]
                            (cell.elevation - water_level) / (water_level + 1.0)
                        };
                        elevation_color(display_elev)
                    }
                }
            };

            painter.add(egui::Shape::convex_polygon(
                points,
                fill,
                egui::Stroke::NONE,
            ));
        }

        // Canvas centre marker
        painter.circle_filled(
            Pos2::new(ox + canvas.center.x, oy + canvas.center.y),
            2.0,
            ColorKey::PointAlt.egui32(),
        );
    }
}
