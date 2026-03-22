use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

use crate::canvas::CanvasData;
use crate::mathlib::vec::Vec2;
use crate::params::util::build_default_point_params;
use crate::params::ParamGroup;
use crate::pipeline::{PipelineStage, PipelineStageExecutor};
use crate::pipeline::{StageData, StageDataMap};
use crate::visualization::points::PointsVisualLayer;

pub fn make_points_stage() -> PipelineStage {
    PipelineStage {
        executor: Box::new(PointsStage),
        params: Some(build_default_point_params()),
        visual_layer: Box::new(PointsVisualLayer::default()),
    }
}

#[derive(Clone, Debug)]
pub struct PointsOutput {
    pub points: Vec<Vec2>,
}

impl StageData for PointsOutput {
    fn as_any(&self) -> &dyn std::any::Any { self }
}

pub const STAGE_DATA_KEY: &str = "stage:points";

pub struct PointsStage;

impl PipelineStageExecutor for PointsStage {
    fn name(&self) -> &str { "points" }
    fn rank(&self) -> u8 { 1 }
    fn data_key(&self) -> &'static str { STAGE_DATA_KEY }

    fn run(&mut self, params: Option<&ParamGroup>, data: &StageDataMap) -> Box<dyn StageData> {
        let canvas = data
            .get("canvas")
            .and_then(|d| d.as_any().downcast_ref::<CanvasData>())
            .expect("Canvas data required for points stage");

        let pg = params.expect("Points params required");
        let distribution_name = pg.get_param("Distribution").map(|p| p.value.as_str()).unwrap_or("Poisson Disc");
        let count = pg.get_param("Point Count").and_then(|p| p.as_int()).unwrap_or(300);
        let min_distance = pg.get_param("Min Distance").and_then(|p| p.as_float()).unwrap_or(35.0);

        // Seed derived deterministically from world seed + stage constant
        let seed = canvas.seed as u64 ^ 0x504F_494E_5453u64; // "POINTS"
        let mut rng = StdRng::seed_from_u64(seed);

        let points = match distribution_name {
            "Uniform Grid"   => UniformGridDistribution.generate_points(count, canvas.width, canvas.height, &mut rng),
            "Random Uniform" => RandomUniformDistribution.generate_points(count, canvas.width, canvas.height, &mut rng),
            _                => PoissonDiscDistribution { min_distance }.generate_points(count, canvas.width, canvas.height, &mut rng),
        };

        Box::new(PointsOutput { points })
    }
}

// ---------------------------------------------------------------------------
// Distribution trait
// ---------------------------------------------------------------------------

pub trait PointsDistribution {
    /// Generate site points. `count` is a hint; Poisson Disc ignores it in favour of `min_distance`.
    fn generate_points(&mut self, count: usize, width: f32, height: f32, rng: &mut StdRng) -> Vec<Vec2>;
}

// ---------------------------------------------------------------------------
// Random Uniform
// ---------------------------------------------------------------------------

#[derive(Default)]
pub struct RandomUniformDistribution;

impl PointsDistribution for RandomUniformDistribution {
    fn generate_points(&mut self, count: usize, width: f32, height: f32, rng: &mut StdRng) -> Vec<Vec2> {
        (0..count)
            .map(|_| Vec2::new(rng.random_range(0.0..width), rng.random_range(0.0..height)))
            .collect()
    }
}

// ---------------------------------------------------------------------------
// Uniform Grid
// ---------------------------------------------------------------------------

#[derive(Default)]
pub struct UniformGridDistribution;

impl PointsDistribution for UniformGridDistribution {
    fn generate_points(&mut self, count: usize, width: f32, height: f32, _rng: &mut StdRng) -> Vec<Vec2> {
        let mut points = Vec::new();
        let spacing = ((width * height) / count as f32).sqrt();
        let mut y = spacing / 2.0;
        while y < height {
            let mut x = spacing / 2.0;
            while x < width {
                points.push(Vec2::new(x, y));
                x += spacing;
            }
            y += spacing;
        }
        points
    }
}

// ---------------------------------------------------------------------------
// Poisson Disc (Bridson's algorithm)
// ---------------------------------------------------------------------------

pub struct PoissonDiscDistribution {
    pub min_distance: f32,
}

impl PointsDistribution for PoissonDiscDistribution {
    fn generate_points(&mut self, _count: usize, width: f32, height: f32, rng: &mut StdRng) -> Vec<Vec2> {
        let r = self.min_distance;
        let cell_size = r / 2.0_f32.sqrt();
        let grid_w = (width / cell_size).ceil() as usize + 1;
        let grid_h = (height / cell_size).ceil() as usize + 1;

        let mut grid: Vec<Option<usize>> = vec![None; grid_w * grid_h];
        let mut points: Vec<Vec2> = Vec::new();
        let mut active: Vec<usize> = Vec::new();

        let insert = |p: Vec2, points: &mut Vec<Vec2>, active: &mut Vec<usize>, grid: &mut Vec<Option<usize>>| {
            let idx = points.len();
            let gx = (p.x / cell_size) as usize;
            let gy = (p.y / cell_size) as usize;
            grid[gy * grid_w + gx] = Some(idx);
            points.push(p);
            active.push(idx);
        };

        let initial = Vec2::new(rng.random_range(0.0..width), rng.random_range(0.0..height));
        insert(initial, &mut points, &mut active, &mut grid);

        const K: usize = 30;

        while !active.is_empty() {
            let list_idx = rng.random_range(0..active.len());
            let origin = points[active[list_idx]];
            let mut found = false;

            for _ in 0..K {
                let angle: f32 = rng.random_range(0.0..std::f32::consts::TAU);
                let dist: f32 = rng.random_range(r..2.0 * r);
                let candidate = Vec2::new(origin.x + dist * angle.cos(), origin.y + dist * angle.sin());

                if candidate.x < 0.0 || candidate.x >= width || candidate.y < 0.0 || candidate.y >= height {
                    continue;
                }

                let cx = (candidate.x / cell_size) as usize;
                let cy = (candidate.y / cell_size) as usize;

                let mut too_close = false;
                'check: for dy in cy.saturating_sub(2)..=(cy + 2).min(grid_h - 1) {
                    for dx in cx.saturating_sub(2)..=(cx + 2).min(grid_w - 1) {
                        if let Some(pt_idx) = grid[dy * grid_w + dx] {
                            if candidate.distance(points[pt_idx]) < r {
                                too_close = true;
                                break 'check;
                            }
                        }
                    }
                }

                if !too_close {
                    insert(candidate, &mut points, &mut active, &mut grid);
                    found = true;
                    break;
                }
            }

            if !found {
                active.swap_remove(list_idx);
            }
        }

        points
    }
}
