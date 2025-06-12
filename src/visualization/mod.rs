use crate::canvas::CanvasLayer;
use crate::params::ParamGroup;
use crate::pipeline::StageOutputs;
use egui::{Painter, Ui};

pub mod points;

pub trait VisualLayer {
    fn name(&self) -> &str;
    fn is_enabled(&self) -> bool;
    fn set_enabled(&mut self, enabled: bool);

    fn params(&mut self) -> Option<&mut ParamGroup>;

    /// Draws controls in UI (optional, delegated from app shell)
    fn draw_controls(&mut self, ui: &mut Ui);

    /// Draws onto canvas using outputs from pipeline
    fn draw_canvas(&self, painter: &Painter, canvas: &CanvasLayer, data: &StageOutputs);
}

impl Default for Box<dyn VisualLayer> {
    fn default() -> Box<dyn VisualLayer> {
        Box::new(CanvasLayer::default())
    }
}
