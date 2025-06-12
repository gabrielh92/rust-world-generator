use crate::canvas::CanvasLayer;
use crate::params::{BoolParam, FloatParam, IntParam, Param, ParamGroup};
use crate::pipeline::StageOutput;
use crate::pipeline::StageOutputs;
use crate::visualization::VisualLayer;
use egui::{Color32, Painter, Ui};

pub struct PointsVisualLayer {
    params: ParamGroup,
}

impl PointsVisualLayer {
    pub fn new() -> Self {
        Self {
            params: ParamGroup::new(
                "Point Distribution",
                vec![
                    Param {
                        name: "Point Count".into(),
                        tooltip: Some("How many points to generate".into()),
                        value: Box::new(IntParam {
                            val: 1000,
                            min: 10,
                            max: 5000,
                            step: 50,
                        }),
                    },
                    Param {
                        name: "Show Points".into(),
                        tooltip: Some("Toggle point visibility".into()),
                        value: Box::new(BoolParam { val: true }),
                    },
                    Param {
                        name: "Point Radius".into(),
                        tooltip: Some("Circle size of each point".into()),
                        value: Box::new(FloatParam {
                            val: 2.0,
                            min: 0.5,
                            max: 10.0,
                            step: 0.5,
                        }),
                    },
                ],
            ),
        }
    }
}

impl VisualLayer for PointsVisualLayer {
    fn name(&self) -> &str {
        "Points"
    }

    fn is_enabled(&self) -> bool {
        self.params.is_enabled()
    }

    fn set_enabled(&mut self, value: bool) {
        self.params.enabled = value;
    }

    fn params(&mut self) -> Option<&mut ParamGroup> {
        Some(&mut self.params)
    }

    fn draw_controls(&mut self, ui: &mut Ui) {
        self.params.draw_controls(ui);
    }

    fn draw_canvas(&self, painter: &Painter, canvas: &CanvasLayer, data: &StageOutputs) {
        if !self.is_enabled() {
            return;
        }

        if let Some(StageOutput::Points(points)) = data.get("points") {
            let radius = self
                .params
                .get_param("Point Radius")
                .and_then(|p| p.as_float())
                .unwrap_or(2.0);
            let show = self
                .params
                .get_param("Show Points")
                .and_then(|p| p.as_bool())
                .unwrap_or(true);

            if show {
                for &pos in points {
                    painter.circle_filled(pos, radius, Color32::BLACK);
                }
            }
        }
    }
}
