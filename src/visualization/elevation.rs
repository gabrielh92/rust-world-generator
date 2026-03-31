use crate::canvas::CanvasData;
use crate::params::ParamGroup;
use crate::pipeline::terrain::{TerrainOutput, STAGE_DATA_KEY as TERRAIN_KEY};
use crate::pipeline::StageDataMap;
use crate::visualization::{elevation_color, lerp_color, ColorKey, VisualLayer};
use egui::{Align2, FontId, Painter, Pos2, Rect, Ui};

#[derive(Default)]
pub struct ElevationVisualLayer {
    pub enabled: bool,
    pub show_coastline: bool,
}

impl VisualLayer for ElevationVisualLayer {
    fn display_name(&self) -> &str { "Elevation" }
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

    fn draw_canvas(&self, painter: &Painter, rect: &Rect, params: Option<&ParamGroup>, data: &StageDataMap) {
        if !self.enabled { return; }

        let pg = match params { Some(p) => p, None => return };
        let water_level  = pg.get_param("Water Level").and_then(|p| p.as_float()).unwrap_or(0.0);
        let view_mode    = pg.get_param("View Mode").map(|p| p.value.as_str()).unwrap_or("Elevation");
        let show_labels  = pg.get_param("Show Labels").and_then(|p| p.as_bool()).unwrap_or(false);
        let label_scale  = pg.get_param("Label Scale").map(|p| p.value.as_str()).unwrap_or("Earth-like");

        let canvas = data.get("canvas")
            .and_then(|d| d.as_any().downcast_ref::<CanvasData>())
            .unwrap();
        let ox = rect.center().x - canvas.width / 2.0;
        let oy = rect.center().y - canvas.height / 2.0;

        let landmass = match data.get(TERRAIN_KEY)
            .and_then(|d| d.as_any().downcast_ref::<TerrainOutput>())
        {
            Some(l) => l,
            None => return,
        };

        for (_i, cell) in landmass.cells.iter().enumerate() {
            let elevation = cell.elevation;

            let points: Vec<Pos2> = cell.corner_ids.iter()
                .map(|&cid| {
                    let p = landmass.corners[cid].position;
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
                        lerp_color(
                            ColorKey::Land.egui32(),
                            egui::Color32::from_rgb(230, 230, 230),
                            (elevation * 0.6).clamp(0.0, 1.0),
                        )
                    } else {
                        lerp_color(
                            ColorKey::Ocean.egui32(),
                            egui::Color32::from_rgb(10, 25, 70),
                            (-elevation * 0.8).clamp(0.0, 1.0),
                        )
                    }
                }
                _ => {
                    if cell.is_border {
                        ColorKey::Border.egui32()
                    } else {
                        let display_elev = if elevation > water_level {
                            (elevation - water_level) / (1.0 - water_level)
                        } else {
                            (elevation - water_level) / (water_level + 1.0)
                        };
                        elevation_color(display_elev)
                    }
                }
            };

            painter.add(egui::Shape::convex_polygon(points, fill, egui::Stroke::NONE));

            if show_labels && !cell.is_border {
                let meters = elevation_to_meters(elevation, label_scale);
                let label = if meters.abs() >= 100.0 {
                    format!("{:.0}m", meters)
                } else {
                    format!("{:.1}m", meters)
                };
                let center = Pos2::new(ox + cell.center.x, oy + cell.center.y);
                // Dark shadow then bright text for readability on any background
                painter.text(
                    center + egui::vec2(1.0, 1.0),
                    Align2::CENTER_CENTER,
                    &label,
                    FontId::proportional(9.0),
                    egui::Color32::from_black_alpha(160),
                );
                painter.text(
                    center,
                    Align2::CENTER_CENTER,
                    &label,
                    FontId::proportional(9.0),
                    egui::Color32::WHITE,
                );
            }
        }

        painter.circle_filled(
            Pos2::new(ox + canvas.center.x, oy + canvas.center.y),
            2.0,
            ColorKey::PointAlt.egui32(),
        );

        if self.show_coastline {
            crate::visualization::draw_coastline_terrain(painter, ox, oy, landmass);
        }
    }
}

/// Convert a normalised elevation value [-1, 1] to metres using the chosen scale mode.
///
/// Earth-like uses separate power-law exponents for land and ocean that approximate
/// the real hypsometric curve: land elevations are right-skewed (most terrain is low,
/// peaks are rare), while ocean floor quickly reaches deep basins.
fn elevation_to_meters(v: f32, scale_mode: &str) -> f32 {
    match scale_mode {
        "Linear"     => v * 8000.0,
        "Compressed" => v * 2000.0,
        _ => {
            // Earth-like: asymmetric power law
            if v >= 0.0 {
                v.powf(1.5) * 8850.0   // land: 0 → 8850m (Everest), skewed toward lowlands
            } else {
                -((-v).powf(0.7) * 10994.0) // ocean: 0 → -10994m (Mariana), deep basins common
            }
        }
    }
}
