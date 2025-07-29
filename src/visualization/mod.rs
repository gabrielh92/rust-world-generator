use crate::params::ParamGroup;
use crate::pipeline::StageDataMap;
use egui::{Painter, Ui};

pub mod points;
pub mod voronoi;
pub mod landmass;

pub trait VisualLayer {
    fn display_name(&self) -> &str;
    fn is_enabled(&self) -> bool;
    fn set_enabled(&mut self, enabled: bool);

    /// Draws controls in UI (optional, delegated from app shell)
    fn draw_controls(&mut self, ui: &mut Ui, params: Option<&mut ParamGroup>) -> bool;

    /// Draws onto canvas using outputs from pipeline
    fn draw_canvas(&self, painter: &Painter, params: Option<&ParamGroup>, data: &StageDataMap);
}
