use crate::{params::ParamGroup, util::{rgb_to_hsv, hsv_to_rgb}};
use crate::pipeline::StageDataMap;
use egui::{Color32, Painter, Rect, Ui};

pub mod points;
pub mod voronoi;
pub mod landmass;
pub mod elevation;

/// Linearly interpolate between two Color32 values.
pub fn lerp_color(a: Color32, b: Color32, t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    Color32::from_rgb(
        (a.r() as f32 + (b.r() as f32 - a.r() as f32) * t) as u8,
        (a.g() as f32 + (b.g() as f32 - a.g() as f32) * t) as u8,
        (a.b() as f32 + (b.b() as f32 - a.b() as f32) * t) as u8,
    )
}

/// Map an elevation value in [-1, 1] to a terrain color.
/// Negative = ocean, positive = land, with gradient steps.
pub fn elevation_color(elevation: f32) -> Color32 {
    const DEEP_OCEAN:    Color32 = Color32::from_rgb(10,  30,  80);
    const SHALLOW_OCEAN: Color32 = Color32::from_rgb(50,  100, 180);
    const COAST_WATER:   Color32 = Color32::from_rgb(90,  150, 210);
    const BEACH:         Color32 = Color32::from_rgb(220, 200, 140);
    const LOWLAND:       Color32 = Color32::from_rgb(90,  160, 60);
    const HIGHLAND:      Color32 = Color32::from_rgb(65,  120, 45);
    const MOUNTAIN:      Color32 = Color32::from_rgb(120, 100, 80);
    const PEAK:          Color32 = Color32::from_rgb(230, 230, 230);

    if elevation < -0.5 {
        lerp_color(DEEP_OCEAN, SHALLOW_OCEAN, (elevation + 1.0) / 0.5)
    } else if elevation < -0.1 {
        lerp_color(SHALLOW_OCEAN, COAST_WATER, (elevation + 0.5) / 0.4)
    } else if elevation < 0.0 {
        lerp_color(COAST_WATER, BEACH, (elevation + 0.1) / 0.1)
    } else if elevation < 0.15 {
        lerp_color(BEACH, LOWLAND, elevation / 0.15)
    } else if elevation < 0.5 {
        lerp_color(LOWLAND, HIGHLAND, (elevation - 0.15) / 0.35)
    } else if elevation < 0.8 {
        lerp_color(HIGHLAND, MOUNTAIN, (elevation - 0.5) / 0.3)
    } else {
        lerp_color(MOUNTAIN, PEAK, (elevation - 0.8) / 0.2)
    }
}

pub enum ColorKey {
    Canvas,
    CanvasBackdrop,
    DelaunayLines,
    VoronoiLines,
    Point,
    PointAlt,
    // Terrain
    Land,
    Coast,
    Ocean,
    Border,
}

impl ColorKey {
    pub fn egui32(self) -> Color32 {
        match self {
            ColorKey::Canvas         => Color32::from_rgb(240, 240, 240),
            ColorKey::CanvasBackdrop => Color32::from_rgb(20, 20, 40),
            ColorKey::DelaunayLines  => Color32::from_rgb(30, 80, 160),
            ColorKey::VoronoiLines   => Color32::from_rgb(160, 160, 160),
            ColorKey::Point          => Color32::BLACK,
            ColorKey::PointAlt       => Color32::from_rgb(220, 60, 60),
            ColorKey::Land           => Color32::from_rgb(90, 160, 60),
            ColorKey::Coast          => Color32::from_rgb(220, 200, 140),
            ColorKey::Ocean          => Color32::from_rgb(50, 100, 180),
            ColorKey::Border         => Color32::from_rgb(40, 40, 60),
        }
    }

    pub fn egui32_value_scaled_by(self, factor: f32) -> Color32 {
        let color = self.egui32();
        let [r, g, b, a] = color.to_array();
        let (h, s, mut v) = rgb_to_hsv(r, g, b);
        v = (v * factor).clamp(0., 1.);
        let (r2, g2, b2) = hsv_to_rgb(h, s, v);
        Color32::from_rgba_premultiplied(r2, g2, b2, a)
    }
}

pub trait VisualLayer {
    fn display_name(&self) -> &str;
    fn is_enabled(&self) -> bool;

    #[allow(dead_code)]
    fn set_enabled(&mut self, enabled: bool);

    /// Draws controls in the sidebar UI. Returns true if any value changed.
    fn draw_controls(&mut self, ui: &mut Ui, params: Option<&mut ParamGroup>) -> bool;

    /// Draws onto canvas using pipeline outputs.
    fn draw_canvas(&self, painter: &Painter, rect: &Rect, params: Option<&ParamGroup>, data: &StageDataMap);
}
