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
		let voronoi = data
			.get(make_stage_data_key("voronoi", 2).as_str())
			.and_then(|d| d.as_any().downcast_ref::<VoronoiOutput>())
			.expect("Voronoi data must exist for landmass definition");

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

		// todo: rotation factor in param doesn't match what's happening- landmass rotates by way more
		let cos_t = rotation.cos();
		let sin_t = rotation.sin();


        // Compute elevation for each cell center
        let mut cells = Vec::new();
        for cell in &voronoi.cells {
            // Cell center = average of vertices
			let local_center = LandmassStage::average_point(&cell.vertices);

			// todo: properly center landmass, it's offset left for some reason
            let cx = local_center.x;
            let cy = local_center.y;

            // Rotate
            let xr = cx * cos_t + cy * sin_t;
            let yr = -cx * sin_t + cy * cos_t;

			// Apply Scale
			let sx = size_x * scale;
			let sy = size_y * scale;

            // Elliptical distance
            let d = ((xr / (sx / 2.0)).powi(2) + (yr / (sy / 2.0)).powi(2)).sqrt();

            // Falloff elevation
			// todo: maybe make this exponential s.t. we never reach the edge
            let mut elevation = 1.0 - d;

			// Add noise
			// todo: update into a better shape generation function
			let n = LandmassStage::simple_noise(local_center.x, local_center.y, noise_scale);
			elevation += noise_amplitude * n;

			// todo: fix edge detection so land doesn't touch the edge of the canvas
			//elevation = if LandmassStage::is_cell_at_edge(&local_center, canvas.width, canvas.height) { 0. } else {elevation};

			cells.push(LandmassCell { center: local_center, elevation });
        }

		Box::new(LandmassOutput { cells })
	}
}

impl LandmassStage {
	fn average_point(vertices: &Vec<Vec2>) -> Vec2 {
		let (sx, sy) = vertices.iter().fold((0.0, 0.0), |(ax, ay), v| (ax + v.x, ay + v.y));
		let n = vertices.len() as f32;
		Vec2::new(sx / n, sy / n)
	}

	fn simple_noise(x: f32, y: f32, scale: f32) -> f32 {
		let s = scale;
		((x * s).sin() * (y * s).cos()) as f32 // deterministic pseudo-noise
	}

	fn is_cell_at_edge(coord: &Vec2, width: f32, height: f32) -> bool {
		coord.x <= 0.0 || coord.x >= width || coord.y <= 0.0 || coord.y >= height
	}
}