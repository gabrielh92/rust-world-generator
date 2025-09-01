use voronoice::{BoundingBox, VoronoiBuilder};

use crate::{
    canvas::CanvasData,
    mathlib::vec::Vec2,
    params::{util::build_default_voronoi_params, ParamGroup},
    pipeline::{
        points::PointsOutput, PipelineStage, PipelineStageExecutor, StageData, StageDataMap,
    },
    util::make_stage_data_key,
    visualization::voronoi::VoronoiVisualLayer,
};

pub fn make_voronoi_stage() -> PipelineStage {
    PipelineStage {
        executor: Box::new(VoronoiStage::default()),
        params: Some(build_default_voronoi_params()),
        visual_layer: Box::new(VoronoiVisualLayer::default()),
    }
}

#[derive(Clone, Debug)]
pub struct VoronoiCell {
    pub vertices: Vec<Vec2>,
	pub is_border: bool,
}

#[derive(Clone, Debug)]
pub struct VoronoiOutput {
    pub cells: Vec<VoronoiCell>,
}

impl StageData for VoronoiOutput {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

#[derive(Default)]
pub struct VoronoiStage;

impl PipelineStageExecutor for VoronoiStage {
    fn name(&self) -> &str {
        "voronoi"
    }
    fn rank(&self) -> u8 {
        2
    }

    fn run(&mut self, params: Option<&ParamGroup>, data: &StageDataMap) -> Box<dyn StageData> {
		let iterations = params
			.expect("Expected voronoi params")
			.get_param("Lloyd Relaxations")
			.and_then(|p| p.as_int())
			.unwrap();

		let canvas = data
            .get("canvas")
            .and_then(|d| d.as_any().downcast_ref::<CanvasData>())
            .unwrap();

        let points = data
            .get(make_stage_data_key("points", 1).as_str())
            .and_then(|d| d.as_any().downcast_ref::<PointsOutput>())
            .expect("Points stage must run before Voronoi stage");

        let input_points = points
            .points
            .iter()
            .map(|p| voronoice::Point {
                x: p.x as f64,
                y: p.y as f64,
            })
            .collect();

        let diagram = VoronoiBuilder::default()
            .set_sites(input_points)
            .set_bounding_box(BoundingBox::new(
                voronoice::Point { x: (canvas.width / 2.).into(), y: (canvas.height / 2.).into() },
                canvas.width.into(),
                canvas.height.into(),
            ))
			.set_lloyd_relaxation_iterations(iterations)
            .build()
            .expect("Failed to build Voronoi diagram");

        let mut cells = Vec::new();
        diagram.iter_cells().for_each(|cell| {
            let vertices: Vec<Vec2> = cell
                .iter_vertices()
                .map(|vp| Vec2::new(vp.x as f32, vp.y as f32))
                .collect();
			let is_border = vertices.iter().any(|v| v.x <= 0.0 || v.x >= canvas.width || v.y <= 0.0 || v.y >= canvas.height);
            cells.push(VoronoiCell { vertices: vertices, is_border: is_border })
        });

        Box::new(VoronoiOutput { cells })
    }
}
