use rand::Rng;

use crate::canvas::CanvasData;
use crate::mathlib::vec::Vec2;
use crate::params::util::build_default_point_params;
use crate::params::ParamGroup;
use crate::pipeline::{PipelineStage, PipelineStageExecutor};
use crate::pipeline::{StageData, StageDataMap};
use crate::visualization::points::PointsVisualLayer;

pub fn make_points_stage() -> PipelineStage {
    PipelineStage {
        executor: Box::new(PointsStage::default()),
        params: Some(build_default_point_params()),
        visual_layer: Box::new(PointsVisualLayer::default()),
    }
}

#[derive(Clone, Debug)]
pub struct PointsOutput {
    pub points: Vec<Vec2>,
}

impl StageData for PointsOutput {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

pub struct PointsStage;

impl Default for PointsStage {
    fn default() -> Self {
        Self
    }
}

impl PipelineStageExecutor for PointsStage {
    fn name(&self) -> &str {
        "points"
    }
    fn rank(&self) -> u8 {
        1
    }

    fn run(&mut self, params: Option<&ParamGroup>, data: &StageDataMap) -> Box<dyn StageData> {
        let canvas = data
            .get("canvas")
            .and_then(|d| d.as_any().downcast_ref::<CanvasData>())
            .unwrap();
        let count = params
            .and_then(|pg| pg.get_param("Point Count"))
            .and_then(|p| p.as_int())
            .unwrap();

        let distribution_name = params
            .and_then(|pg| pg.get_param("Distribution"))
            .map(|p| p.value.as_str())
            .unwrap();

        let points = match distribution_name {
            "Uniform Grid" => {
                UniformGridDistribution.generate_points(count, canvas.width, canvas.height)
            }
            "Random Uniform" | _ => {
                RandomUniformDistribution.generate_points(count, canvas.width, canvas.height)
            }
        }
        .into_iter()
        .map(|p| Vec2::new(p.x, p.y))
        .collect();

        Box::new(PointsOutput { points })
    }
}

// todo: could take in an <R: Rng> parameter in the future
pub trait PointsDistribution {
    fn generate_points(&mut self, count: usize, width: f32, height: f32) -> Vec<egui::Pos2>;
}

// todo: add more complex distributions, such as poisson disc, gaussian, blue noise
#[derive(Default)]
pub struct RandomUniformDistribution;

impl PointsDistribution for RandomUniformDistribution {
    fn generate_points(&mut self, count: usize, width: f32, height: f32) -> Vec<egui::Pos2> {
        let mut rng = rand::rng();
        (0..count)
            .map(|_| egui::Pos2::new(rng.random_range(0.0..width), rng.random_range(0.0..height)))
            .collect()
    }
}

#[derive(Default)]
pub struct UniformGridDistribution;

impl PointsDistribution for UniformGridDistribution {
    fn generate_points(&mut self, count: usize, width: f32, height: f32) -> Vec<egui::Pos2> {
        // todo: placeholder: generate uniform grid with some spacing
        let mut points = Vec::new();
        let spacing = ((width * height) / count as f32).sqrt();

        let mut y = spacing / 2.0;
        while y < height {
            let mut x = spacing / 2.0;
            while x < width {
                points.push(egui::Pos2::new(x, y));
                x += spacing;
            }
            y += spacing;
        }

        points
    }
}
