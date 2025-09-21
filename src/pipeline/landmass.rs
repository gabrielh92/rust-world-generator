use noise::{NoiseFn, Perlin};

use crate::canvas::CanvasData;
use crate::mathlib::vec::Vec2;
use crate::params::util::build_default_landmass_params;
use crate::pipeline::voronoi::{VoronoiCell, VoronoiOutput};
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
	pub cell: VoronoiCell,
	pub elevation: f64,
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
		let (falloff_rate, size_x, size_y) = if let Some(pg) = params {
			(
				pg.get_param("Land Falloff").and_then(|p| p.as_float()).unwrap(),
				pg.get_param("X Size").and_then(|p| p.as_float()).unwrap(),
				pg.get_param("Y Size").and_then(|p| p.as_float()).unwrap(),
			)
		} else {
			panic!("Landmass parameters undefined")
		};

		let (land_noise_intensity, water_noise_intensity) = if let Some(pg) = params {
			(
				pg.get_param("Land Noise Intensity").and_then(|p| p.as_float()).unwrap() as f64,
				pg.get_param("Water Noise Intensity").and_then(|p| p.as_float()).unwrap() as f64,
			)
		} else {
			panic!("Landmass parameters undefined")
		};

		let voronoi = data
			.get(make_stage_data_key("voronoi", 2).as_str())
			.and_then(|d| d.as_any().downcast_ref::<VoronoiOutput>())
			.expect("Voronoi data must exist for landmass definition");

		// we want to start abstracting the shape of the canvas from the generation in case we do tiling with diff polygons
		// so we only get the center of the canvas and operate from there
		let canvas_center = data
			.get("canvas")
			.and_then(|c| c.as_any().downcast_ref::<CanvasData>())
			.expect("Canvas data defined for landmass generation").center;

		let reference_radius = LandmassStage::compute_ref_radius(&voronoi.cells, canvas_center);

        let mut cells = Vec::new();
        for cell in &voronoi.cells {
			if cell.is_border {
				cells.push(LandmassCell { cell: cell.clone(), elevation: -1. });
				continue;
			}

			// todo: do we want elevation per vertex rather than per cell?

			// Calculate Cartesian distance - d = √[(x₂ - x₁)² + (y₂ - y₁)²]
			// Scale by the reference_radius (mean distance of all cells from center) which acts as a normalizer
			let (cell_x, cell_y) = (cell.centroid.x, cell.centroid.y);
			let x_component = (cell_x - canvas_center.x) / (reference_radius * size_x);
			let y_component = (cell_y - canvas_center.y) / (reference_radius * size_y);
			let cell_d_from_center = f32::sqrt(x_component * x_component + y_component * y_component);

			// Calculate base elevation as a radial function with a falloff_rate
			let base_elevation = - ((cell_d_from_center - 1.) / falloff_rate).tanh() as f64;
			println!("cell: {} at pos {:?} - d {}", cell.index, cell.centroid, cell_d_from_center);
			println!("cell: {} base elevation {}", cell.index, base_elevation);

			// Apply noise for basic land and water features
			let sample = LandmassStage::perlin_noise(cell.centroid.x, cell.centroid.y);
			let noise_intensity = base_elevation.max(0.) * land_noise_intensity + (-base_elevation).max(0.) * water_noise_intensity;
			println!("cell: {} sample {} intensity {}", cell.index, sample, noise_intensity);

			let out_elevation = (base_elevation + sample * noise_intensity).clamp(-1., 1.);
			cells.push(LandmassCell { cell: cell.clone(), elevation: out_elevation });
        }

		println!("Center at {:?} - ref radius: {}", canvas_center, reference_radius);

		Box::new(LandmassOutput { cells })
	}
}

impl LandmassStage {
	fn compute_ref_radius(cells: &[VoronoiCell], center: Vec2) -> f32 {
		let mut sum = 0.0;
		for c in cells {
			sum += ((c.centroid.x - center.x).powi(2) + (c.centroid.y - center.y).powi(2)).sqrt();
		}
		(sum / cells.len() as f32).max(1e-4) // mean; median is even nicer if you want robustness
	}

	#[allow(dead_code)]
	fn simple_noise(x: f32, y: f32, scale: f32) -> f32 {
		let s = scale;
		((x * s).sin() * (y * s).cos()) as f32 // deterministic pseudo-noise
	}

	#[allow(dead_code)]
	fn perlin_noise(x: f32, y: f32) -> f64 {
		// todo: expose seed in canvas layer and use here
		let perlin = Perlin::new(0);
		let nx = x as f64;
		let ny = y as f64;
		perlin.get([nx, ny]) // [-1, 1]
	}
}