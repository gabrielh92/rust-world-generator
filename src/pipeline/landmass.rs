use noise::{NoiseFn, Perlin};

use crate::canvas::CanvasData;
use crate::mathlib::vec::Vec2;
use crate::params::util::build_default_landmass_params;
use crate::pipeline::voronoi::VoronoiOutput;
use crate::pipeline::{PipelineStage, PipelineStageExecutor, StageData};
use crate::util::make_stage_data_key;
use crate::visualization::landmass::LandmassVisualLayer;


pub fn make_landmass_stage() -> PipelineStage {
	PipelineStage {
		executor: Box::new(LandmassStage::default()),
		params: Some(build_default_landmass_params()),
		visual_layer: Box::new(LandmassVisualLayer::default()),
	}
}

#[derive(Debug, Clone)]
pub struct LandmassCell {
	pub center: Vec2,
	pub vertices: Vec<Vec2>,
	pub elevation: f32,
}

#[derive(Debug, Clone)]
pub struct LandmassOutput {
	pub cells: Vec<LandmassCell>,
}

impl StageData for LandmassOutput {
	fn as_any(&self) -> &dyn std::any::Any {
		self
	}
}

pub struct LandmassStage;

impl Default for LandmassStage {
	fn default() -> Self {
		Self
	}
}

impl PipelineStageExecutor for LandmassStage {
	fn name(&self) -> &str {
		"landmass"
	}

	fn rank(&self) -> u8 {
		3
	}

	fn run(&mut self, params: Option<&crate::params::ParamGroup>, data: &super::StageDataMap) -> Box<dyn StageData> {
		let (scale, size_x, size_y, rotation) = if let Some(pg) = params {
			(
				pg.get_param("Scale").and_then(|p| p.as_float()).unwrap(),
				pg.get_param("X Size").and_then(|p| p.as_float()).unwrap(),
				pg.get_param("Y Size").and_then(|p| p.as_float()).unwrap(),
				pg.get_param("Rotation").and_then(|p| p.as_float()).unwrap(),

			)
		} else {
			panic!("Landmass parameters undefined")
		};

		let (noise_scale, noise_amplitude) = if let Some(pg) = params {
			(
				pg.get_param("Noise Scale").and_then(|p| p.as_float()).unwrap(),
				pg.get_param("Noise Amplitude").and_then(|p| p.as_float()).unwrap(),
			)
		} else {
			panic!("Landmass parameters undefined")
		};

		let voronoi = data
			.get(make_stage_data_key("voronoi", 2).as_str())
			.and_then(|d| d.as_any().downcast_ref::<VoronoiOutput>())
			.expect("Voronoi data must exist for landmass definition");

		let canvas = data
			.get("canvas")
			.and_then(|c| c.as_any().downcast_ref::<CanvasData>())
			.expect("Canvas data defined for landmass generation");

		// todo: fix rotation factor in param
		let theta = rotation.to_radians();
		let cos_t = theta.cos();
		let sin_t = theta.sin();


        // Compute elevation for each cell center
        let mut cells = Vec::new();
        for cell in &voronoi.cells {
            // Cell center = average of vertices
			let local_center = cell.centroid;

			// // todo: properly center landmass, this hack is wtf
            // let cx = local_center.x - (canvas.center.x);
            // let cy = local_center.y - (canvas.center.y);

            // // Rotate
            // let xr = cx * cos_t + cy * sin_t;
            // let yr = -cx * sin_t + cy * cos_t;

			// // Apply Scale
			// let sx = size_x * scale;
			// let sy = size_y * scale;

            // // Elliptical distance
			// 1 - e ^ -x
            // let d = ((xr / (sx / 2.0)).powi(2) + (yr / (sy / 2.0)).powi(2)).sqrt();

            // // Falloff elevation
            // let mut elevation = 1.0 - d;
			let mut elevation = 0.;
			// Add noise
			// todo: update into a better shape generation function
			let n = LandmassStage::perlin_noise(local_center.x, local_center.y, noise_scale);
			elevation += noise_amplitude * n;

			elevation = if cell.is_border { -1. } else {elevation};
			elevation = elevation.clamp(-1., 1.);
			cells.push(LandmassCell { center: Vec2 { x: local_center.x, y: local_center.y }, vertices: cell.vertices.clone(), elevation });
        }

		Box::new(LandmassOutput { cells })
	}
}

impl LandmassStage {
	fn simple_noise(x: f32, y: f32, scale: f32) -> f32 {
		let s = scale;
		((x * s).sin() * (y * s).cos()) as f32 // deterministic pseudo-noise
	}

	fn perlin_noise(x: f32, y: f32, scale: f32) -> f32 {
		let perlin = Perlin::new(0);
		let nx = x as f64 * scale as f64;
		let ny = y as f64 * scale as f64;
		perlin.get([nx, ny]) as f32 // [-1, 1]
	}
}