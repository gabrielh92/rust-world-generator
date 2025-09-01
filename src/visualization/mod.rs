use crate::params::ParamGroup;
use crate::pipeline::StageDataMap;
use egui::{Color32, Painter, Rect, Ui};

pub mod points;
pub mod voronoi;
pub mod landmass;

pub enum ColorKey {
	Base,
	Canvas,
	CanvasBackdrop,
	Point,
	Text,
	VoronoiLines,
	Water,
}

impl ColorKey {
	pub fn egui32(self) -> Color32 {
		match self {
			ColorKey::Base => Color32::from_rgb(34, 139, 34),
			ColorKey::Canvas => Color32::WHITE,
			ColorKey::CanvasBackdrop => Color32::DARK_BLUE,
			ColorKey::Point => Color32::BLACK,
			ColorKey::Text => Color32::DARK_GRAY,
			ColorKey::VoronoiLines => Color32::LIGHT_GRAY,
			ColorKey::Water => Color32::from_rgb(65, 105, 225),
		}
	}
}

pub trait VisualLayer {
    fn display_name(&self) -> &str;
    fn is_enabled(&self) -> bool;
    fn set_enabled(&mut self, enabled: bool);

    /// Draws controls in UI (optional, delegated from app shell)
    fn draw_controls(&mut self, ui: &mut Ui, params: Option<&mut ParamGroup>) -> bool;

    /// Draws onto canvas using outputs from pipeline
    fn draw_canvas(&self, painter: &Painter, rect: &Rect, params: Option<&ParamGroup>, data: &StageDataMap);
}
