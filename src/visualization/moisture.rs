use crate::canvas::CanvasData;
use crate::params::ParamGroup;
use crate::pipeline::terrain::{TerrainOutput, STAGE_DATA_KEY as TERRAIN_KEY};
use crate::pipeline::moisture::{MoistureOutput, STAGE_DATA_KEY as MOISTURE_KEY};
use crate::pipeline::StageDataMap;
use crate::visualization::{lerp_color, ColorKey, VisualLayer};
use egui::{Color32, Painter, Pos2, Rect, Ui};

#[derive(Default)]
pub struct MoistureVisualLayer {
    pub enabled: bool,
}

impl VisualLayer for MoistureVisualLayer {
    fn display_name(&self) -> &str { "Moisture" }
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

        let moisture_out = match data.get(MOISTURE_KEY)
            .and_then(|d| d.as_any().downcast_ref::<MoistureOutput>())
        {
            Some(m) => m,
            None => return,
        };

        let view_mode = params
            .and_then(|p| p.get_param("View Mode"))
            .map(|p| p.value.as_str())
            .unwrap_or("Moisture");

        for (i, cell) in landmass.cells.iter().enumerate() {
            let m = moisture_out.cell_moisture.get(i).copied().unwrap_or(0.0);

            let points: Vec<Pos2> = cell.corner_ids.iter()
                .map(|&cid| {
                    let p = landmass.corners[cid].position;
                    Pos2::new(ox + p.x, oy + p.y)
                })
                .collect();
            if points.len() < 3 { continue; }

            let fill = if cell.is_border {
                ColorKey::Border.egui32()
            } else if !cell.is_land {
                ColorKey::Ocean.egui32()
            } else {
                match view_mode {
                    "Land Type" => moisture_type_color(m),
                    _           => moisture_gradient_color(m),
                }
            };

            painter.add(egui::Shape::convex_polygon(points, fill, egui::Stroke::NONE));
        }
    }
}

// ---------------------------------------------------------------------------
// Color helpers
// ---------------------------------------------------------------------------

/// Continuous moisture gradient: arid tan → humid dark teal.
pub fn moisture_gradient_color(m: f32) -> Color32 {
    const ARID:      Color32 = Color32::from_rgb(200, 170, 100);
    const SEMI_ARID: Color32 = Color32::from_rgb(160, 185, 95);
    const TEMPERATE: Color32 = Color32::from_rgb(80,  160, 80);
    const HUMID:     Color32 = Color32::from_rgb(40,  125, 100);
    const WET:       Color32 = Color32::from_rgb(20,  80,  110);

    let m = m.clamp(0.0, 1.0);
    if m < 0.25 {
        lerp_color(ARID,      SEMI_ARID, m / 0.25)
    } else if m < 0.5 {
        lerp_color(SEMI_ARID, TEMPERATE, (m - 0.25) / 0.25)
    } else if m < 0.75 {
        lerp_color(TEMPERATE, HUMID,     (m - 0.5)  / 0.25)
    } else {
        lerp_color(HUMID,     WET,       (m - 0.75) / 0.25)
    }
}

/// Discrete moisture zones for Land Type view.
pub fn moisture_type_color(m: f32) -> Color32 {
    if      m < 0.2 { Color32::from_rgb(210, 180, 110) } // Arid
    else if m < 0.4 { Color32::from_rgb(170, 185, 100) } // Semi-arid
    else if m < 0.6 { Color32::from_rgb(90,  160, 70)  } // Temperate
    else if m < 0.8 { Color32::from_rgb(50,  140, 90)  } // Humid
    else            { Color32::from_rgb(30,  100, 120)  } // Wet
}
