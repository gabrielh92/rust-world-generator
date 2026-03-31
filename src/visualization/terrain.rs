use crate::canvas::CanvasData;
use crate::params::ParamGroup;
use crate::pipeline::terrain::{TerrainOutput, STAGE_DATA_KEY};
use crate::pipeline::StageDataMap;
use crate::visualization::{elevation_color, lerp_color, ColorKey, VisualLayer};
use egui::{Color32, Painter, Pos2, Rect, Ui};

#[derive(Default)]
pub struct TerrainVisualLayer {
    pub enabled: bool,
}

impl VisualLayer for TerrainVisualLayer {
    fn display_name(&self) -> &str { "Terrain" }
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

        let view_mode = params
            .and_then(|p| p.get_param("View Mode"))
            .map(|p| p.value.as_str())
            .unwrap_or("Elevation");

        let canvas = match data.get("canvas")
            .and_then(|d| d.as_any().downcast_ref::<CanvasData>())
        {
            Some(c) => c,
            None => return,
        };
        let ox = rect.center().x - canvas.width / 2.0;
        let oy = rect.center().y - canvas.height / 2.0;

        let terrain = match data
            .get(STAGE_DATA_KEY)
            .and_then(|d| d.as_any().downcast_ref::<TerrainOutput>())
        {
            Some(t) => t,
            None => return,
        };

        for cell in &terrain.cells {
            let points: Vec<Pos2> = cell.corner_ids.iter()
                .map(|&cid| {
                    let p = terrain.corners[cid].position;
                    Pos2::new(ox + p.x, oy + p.y)
                })
                .collect();

            if points.len() < 3 { continue; }

            let fill = match view_mode {
                "Land Type" => {
                    if cell.is_border {
                        ColorKey::Border.egui32()
                    } else if cell.is_lake {
                        Color32::from_rgb(50, 140, 160) // teal for lakes
                    } else if cell.is_coast {
                        ColorKey::Coast.egui32()
                    } else if cell.is_land {
                        lerp_color(
                            ColorKey::Land.egui32(),
                            Color32::from_rgb(230, 230, 230),
                            (cell.elevation * 0.6).clamp(0.0, 1.0),
                        )
                    } else {
                        lerp_color(
                            ColorKey::Ocean.egui32(),
                            Color32::from_rgb(10, 25, 70),
                            (-cell.elevation * 0.8).clamp(0.0, 1.0),
                        )
                    }
                }
                "Continentalness" => {
                    if cell.is_border {
                        ColorKey::Border.egui32()
                    } else {
                        // Map [-1, 1] → dark blue to light gray
                        let t = (cell.continentalness * 0.5 + 0.5).clamp(0.0, 1.0);
                        lerp_color(
                            Color32::from_rgb(10, 20, 80),
                            Color32::from_rgb(220, 220, 220),
                            t,
                        )
                    }
                }
                "Erosion" => {
                    if cell.is_border {
                        ColorKey::Border.egui32()
                    } else {
                        // Map [0, 1] → brown (craggy) to white (flat)
                        let t = cell.erosion.clamp(0.0, 1.0);
                        lerp_color(
                            Color32::from_rgb(100, 60, 20),
                            Color32::from_rgb(230, 220, 200),
                            t,
                        )
                    }
                }
                "Peaks & Valleys" => {
                    if cell.is_border {
                        ColorKey::Border.egui32()
                    } else {
                        let pv = cell.peaks_valleys;
                        if pv >= 0.0 {
                            // Positive = peaks → white
                            lerp_color(
                                Color32::from_rgb(80, 120, 100),
                                Color32::from_rgb(230, 230, 230),
                                pv.clamp(0.0, 1.0),
                            )
                        } else {
                            // Negative = valleys → dark teal
                            lerp_color(
                                Color32::from_rgb(20, 60, 70),
                                Color32::from_rgb(80, 120, 100),
                                (pv + 1.0).clamp(0.0, 1.0),
                            )
                        }
                    }
                }
                _ => {
                    // Elevation (default)
                    if cell.is_border {
                        ColorKey::Border.egui32()
                    } else {
                        elevation_color(cell.elevation)
                    }
                }
            };

            painter.add(egui::Shape::convex_polygon(points, fill, egui::Stroke::NONE));
        }

        // Draw coastline on top
        draw_terrain_coastline(painter, ox, oy, terrain);
    }
}

/// Draw coast edges from TerrainOutput.
fn draw_terrain_coastline(painter: &Painter, ox: f32, oy: f32, terrain: &TerrainOutput) {
    let stroke = egui::Stroke::new(1.5, Color32::BLACK);
    for edge in &terrain.edges {
        if !edge.is_coast { continue; }
        let pa = terrain.corners[edge.corner_a].position;
        let pb = terrain.corners[edge.corner_b].position;
        painter.line_segment(
            [Pos2::new(ox + pa.x, oy + pa.y), Pos2::new(ox + pb.x, oy + pb.y)],
            stroke,
        );
    }
}
