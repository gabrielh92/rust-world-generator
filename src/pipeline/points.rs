use rand::Rng;

use crate::canvas::CanvasData;
use crate::params::ParamGroup;
use crate::params::util::build_default_point_params;
use crate::pipeline::{PipelineStage, PipelineStageExecutor};
use crate::pipeline::{StageData, StageDataMap};
use crate::util::make_stage_data_key;
use crate::visualization::points::PointsVisualLayer;

pub fn make_points_stage() -> PipelineStage {
	PipelineStage {
		executor: Box::new(PointsStage::<RandomUniformDistribution>::default()),
		params: Some(build_default_point_params()),
		visual_layer: Box::new(PointsVisualLayer::default()),
	}
}

#[derive(Clone, Debug)]
pub struct PointsOutput {
	pub points: Vec<egui::Pos2>,
}

impl StageData for PointsOutput {
	fn as_any(&self) -> &dyn std::any::Any {
		self
	}
}

pub struct PointsStage<D: PointsDistribution> {
	pub distribution: D,
}

impl<D: PointsDistribution + Default> Default for PointsStage<D> {
	fn default() -> Self {
		Self {
			distribution: D::default(),
		}
	}
}

impl<D: PointsDistribution> PipelineStageExecutor for PointsStage<D> {
	fn name(&self) -> &str { "points" }
	fn rank(&self) -> u8 { 1 }

	fn run(&mut self, params: Option<&ParamGroup>, data: &StageDataMap) -> Box<dyn StageData> {
		let canvas = data.get("canvas")
			.and_then(|d| d.as_any().downcast_ref::<CanvasData>())
			.unwrap();
		let count = params
			.and_then(|pg| pg.get_param("Point Count"))
			.and_then(|p| p.as_int())
			.unwrap();

		// let distribution_type = params
		// 	.and_then(|pg| pg.get_param("Distribution"))
		// 	.and_then(|p| p.as_enum())
		// 	.and_then(|index| PointDistributionType::all_variants().get(*index))
		// 	.and_then(|label| PointDistributionType::from_str(label))
		// 	.unwrap();

		let points = match PointDistributionType::RandomUniformDistribution {
			PointDistributionType::RandomUniformDistribution => {
				RandomUniformDistribution.generate_points(count, canvas.width, canvas.height)
			}
			PointDistributionType::PoissonDiscDistribution => {
				PoissonDiscDistribution.generate_points(count, canvas.width, canvas.height)
			}
		};
        Box::new(PointsOutput { points })
	}
}

// todo: could take in an <R: Rng> parameter in the future
pub trait PointsDistribution {
	fn generate_points(&mut self, count: usize, width: f32, height: f32) -> Vec<egui::Pos2>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PointDistributionType {
	RandomUniformDistribution,
	PoissonDiscDistribution,
}

impl PointDistributionType {
    pub fn all_variants() -> &'static [&'static str] {
        &["Random Uniform", "Poisson Disc"]
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "Random Uniform" => Some(Self::RandomUniformDistribution),
            "Poisson Disc" => Some(Self::PoissonDiscDistribution),
            _ => None,
        }
    }

    pub fn to_str(&self) -> &'static str {
        match self {
            Self::RandomUniformDistribution => "Random Uniform",
            Self::PoissonDiscDistribution => "Poisson Disc",
        }
    }
}

#[derive(Default)]
pub struct RandomUniformDistribution;

impl PointsDistribution for RandomUniformDistribution {
	fn generate_points(&mut self, count: usize, width: f32, height: f32) -> Vec<egui::Pos2> {
		let mut rng = rand::thread_rng();
		(0..count)
			.map(|_| egui::Pos2::new(
				rng.gen_range(0.0..width),
				rng.gen_range(0.0..height),
			))
			.collect()
	}
}

#[derive(Default)]
pub struct PoissonDiscDistribution;

impl PointsDistribution for PoissonDiscDistribution {
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