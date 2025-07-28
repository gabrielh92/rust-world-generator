use voronoice::{BoundingBox, VoronoiBuilder};

use crate::{
    canvas::CanvasData,
    mathlib::vec::Vec2,
    params::ParamGroup,
    pipeline::{
        points::PointsOutput, PipelineStage, PipelineStageExecutor, StageData, StageDataMap,
    },
    util::make_stage_data_key,
    visualization::voronoi::VoronoiVisualLayer,
};

pub fn make_voronoi_stage() -> PipelineStage {
    PipelineStage {
        executor: Box::new(VoronoiStage::default()),
        params: None,
        visual_layer: Box::new(VoronoiVisualLayer::default()),
    }
}

#[derive(Clone, Debug)]
pub struct VoronoiCell {
    pub vertices: Vec<Vec2>,
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

    fn run(&mut self, _params: Option<&ParamGroup>, data: &StageDataMap) -> Box<dyn StageData> {
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
                voronoice::Point {
                    x: (canvas.width / 2.).into(),
                    y: (canvas.height / 2.).into(),
                },
                canvas.width.into(),
                canvas.height.into(),
            ))
            .build()
            .expect("Failed to build Voronoi diagram");

        let mut cells = Vec::new();
        diagram.iter_cells().for_each(|cell| {
            let vertices: Vec<Vec2> = cell
                .iter_vertices()
                .map(|vp| Vec2::new(vp.x as f32, vp.y as f32))
                .collect();
            println!("Voronoi vertices: {:?}", vertices);
            cells.push(VoronoiCell { vertices: vertices })
        });

        Box::new(VoronoiOutput { cells })
    }
}
