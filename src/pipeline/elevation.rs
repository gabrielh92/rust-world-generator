use std::collections::{HashMap, VecDeque};

use noise::{NoiseFn, Perlin};
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

use crate::canvas::CanvasData;
use crate::mathlib::vec::Vec2;
use crate::params::util::build_default_elevation_params;
use crate::pipeline::landmass::{LandCell, LandmassOutput, STAGE_DATA_KEY as LANDMASS_KEY};
use crate::pipeline::{PipelineStage, PipelineStageExecutor, StageData, StageDataMap};
use crate::visualization::elevation::ElevationVisualLayer;

pub const STAGE_DATA_KEY: &str = "stage:elevation";

pub fn make_elevation_stage() -> PipelineStage {
    PipelineStage {
        executor: Box::new(ElevationStage),
        params: Some(build_default_elevation_params()),
        visual_layer: Box::new(ElevationVisualLayer::default()),
    }
}

// ---------------------------------------------------------------------------
// Output
// ---------------------------------------------------------------------------

/// Refined elevation for each cell and corner, produced by Stage 4.
/// Indexed by position in LandmassOutput::cells / ::corners — not by ID.
/// The visualization layer joins this with LandmassOutput for geometry.
#[derive(Debug, Clone)]
pub struct ElevationOutput {
    /// One value per cell (same order as LandmassOutput::cells). Range: [-1, 1].
    pub cell_elevations: Vec<f32>,
    /// One value per corner (same order as LandmassOutput::corners). Range: [-1, 1].
    pub corner_elevations: Vec<f32>,
}

impl StageData for ElevationOutput {
    fn as_any(&self) -> &dyn std::any::Any { self }
}

// ---------------------------------------------------------------------------
// Stage executor
// ---------------------------------------------------------------------------

pub struct ElevationStage;

impl PipelineStageExecutor for ElevationStage {
    fn name(&self) -> &str { "elevation" }
    fn rank(&self) -> u8 { 4 }
    fn data_key(&self) -> &'static str { STAGE_DATA_KEY }

    fn run(&mut self, params: Option<&crate::params::ParamGroup>, data: &StageDataMap) -> Box<dyn StageData> {
        let pg = params.expect("Elevation params required");

        let ridge_mode     = pg.get_param("Ridge Mode").map(|p| p.value.as_str()).unwrap_or("Follow Terrain");
        let num_ridges     = pg.get_param("Num Ridges").and_then(|p| p.as_int()).unwrap_or(4);
        let ridge_intensity = pg.get_param("Ridge Intensity").and_then(|p| p.as_float()).unwrap_or(0.5);
        let ridge_width    = pg.get_param("Ridge Width").and_then(|p| p.as_float()).unwrap_or(0.15);
        let distribution   = pg.get_param("Distribution").map(|p| p.value.as_str()).unwrap_or("Natural");
        let redist_strength = pg.get_param("Redistribution Strength").and_then(|p| p.as_float()).unwrap_or(0.6);
        let stage3_blend   = pg.get_param("Stage 3 Blend").and_then(|p| p.as_float()).unwrap_or(0.5);

        let canvas = data.get("canvas")
            .and_then(|d| d.as_any().downcast_ref::<CanvasData>())
            .expect("Canvas data required for elevation stage");

        let landmass = data.get(LANDMASS_KEY)
            .and_then(|d| d.as_any().downcast_ref::<LandmassOutput>())
            .expect("Landmass stage must run before Elevation stage");

        let cells = &landmass.cells;
        let n = cells.len();

        let seed = canvas.seed as u64 ^ 0x454C455641_u64; // "ELEVA"
        let mut rng = StdRng::seed_from_u64(seed);
        let perlin = Perlin::new(canvas.seed.wrapping_add(1));

        let id_to_idx: HashMap<usize, usize> =
            cells.iter().enumerate().map(|(i, c)| (c.id, i)).collect();

        // --- Step 1: Start from Stage 3 elevation or compute fresh BFS ---
        // Stage 3 elevation is always available here (required dep), but we
        // respect the blend param — at 0.0 the stage3 signal is ignored.
        let mut elevation: Vec<f32> = if stage3_blend > 0.0 {
            cells.iter().map(|c| c.elevation * stage3_blend).collect()
        } else {
            bfs_base_elevation(cells, &id_to_idx)
        };

        // --- Step 2: Mountain ridge injection ---
        if num_ridges > 0 && ridge_mode != "None" {
            let canvas_diag = (canvas.width * canvas.width + canvas.height * canvas.height).sqrt();
            let abs_width = ridge_width * canvas_diag;

            let ridges = build_ridges(
                cells,
                num_ridges,
                ridge_mode,
                &elevation,
                &id_to_idx,
                &mut rng,
            );

            for cell in cells.iter() {
                if !cell.is_land { continue; }
                let idx = id_to_idx[&cell.id];

                let mut max_boost: f32 = 0.0;
                for &(a, b) in &ridges {
                    let dist = point_to_segment_distance(cell.center, a, b);
                    // Gaussian falloff from ridge line
                    let boost = ridge_intensity * (-dist * dist / (2.0 * abs_width * abs_width)).exp();
                    max_boost = max_boost.max(boost);
                }
                elevation[idx] = (elevation[idx] + max_boost).clamp(-1.0, 1.0);
            }
        }

        // --- Step 3: Pre-redistribution smooth (removes noise spikes that corrupt ranking) ---
        for _ in 0..2 {
            let prev = elevation.clone();
            for (i, cell) in cells.iter().enumerate() {
                if cell.is_border { continue; }
                let neighbors: Vec<f32> = cell.neighbor_ids.iter()
                    .filter_map(|&nid| id_to_idx.get(&nid).map(|&ni| prev[ni]))
                    .collect();
                if neighbors.is_empty() { continue; }
                let avg = neighbors.iter().sum::<f32>() / neighbors.len() as f32;
                elevation[i] = prev[i] * 0.6 + avg * 0.4;
            }
        }

        // --- Step 4: Histogram redistribution (land cells only) ---
        if redist_strength > 0.0 {
            let exponent = match distribution {
                "Flat"        => 2.5_f32,
                "Mountainous" => 0.35,
                _             => 0.65, // Natural
            };

            let mut land_indices: Vec<usize> = (0..n)
                .filter(|&i| cells[i].is_land && !cells[i].is_border)
                .collect();
            land_indices.sort_by(|&a, &b| elevation[a].partial_cmp(&elevation[b]).unwrap_or(std::cmp::Ordering::Equal));

            let count = land_indices.len();
            if count > 1 {
                for (rank, &idx) in land_indices.iter().enumerate() {
                    let t = rank as f32 / (count - 1) as f32;
                    let target = t.powf(exponent);
                    elevation[idx] = elevation[idx] * (1.0 - redist_strength) + target * redist_strength;
                    elevation[idx] = elevation[idx].clamp(0.01, 1.0);
                }
            }
        }

        // --- Step 5: Add fine-detail noise (after redistribution, so it doesn't corrupt ranking) ---
        let noise_scale = 0.008_f64;
        for (i, cell) in cells.iter().enumerate() {
            if cell.is_border { continue; }
            let noise = perlin.get([cell.center.x as f64 * noise_scale, cell.center.y as f64 * noise_scale]) as f32;
            let intensity = if cells[i].is_land { 0.06 } else { 0.03 };
            elevation[i] = (elevation[i] + noise * intensity).clamp(-1.0, 1.0);
        }

        // --- Step 6: Post-redistribution smooth (kills any remaining speckle) ---
        for _ in 0..2 {
            let prev = elevation.clone();
            for (i, cell) in cells.iter().enumerate() {
                if !cell.is_land || cell.is_border { continue; }
                let neighbors: Vec<f32> = cell.neighbor_ids.iter()
                    .filter_map(|&nid| id_to_idx.get(&nid).map(|&ni| prev[ni]))
                    .collect();
                if neighbors.is_empty() { continue; }
                let avg = neighbors.iter().sum::<f32>() / neighbors.len() as f32;
                elevation[i] = prev[i] * 0.65 + avg * 0.35;
            }
        }

        // --- Step 7: Corner elevation (average of adjacent cells + small noise) ---
        let corner_noise_scale = 0.01_f64;
        let corner_elevations: Vec<f32> = landmass.corners.iter().map(|corner| {
            if corner.adjacent_cell_ids.is_empty() {
                return -1.0;
            }
            let avg: f32 = corner.adjacent_cell_ids.iter()
                .filter_map(|&cid| id_to_idx.get(&cid).map(|&i| elevation[i]))
                .sum::<f32>()
                / corner.adjacent_cell_ids.len() as f32;

            let noise = perlin.get([
                corner.position.x as f64 * corner_noise_scale,
                corner.position.y as f64 * corner_noise_scale,
            ]) as f32;
            (avg + noise * 0.05).clamp(-1.0, 1.0)
        }).collect();

        Box::new(ElevationOutput { cell_elevations: elevation, corner_elevations })
    }
}

// ---------------------------------------------------------------------------
// Ridge building
// ---------------------------------------------------------------------------

/// Build a list of ridge line segments (start, end) as world-space Vec2 pairs.
fn build_ridges(
    cells: &[LandCell],
    num_ridges: usize,
    mode: &str,
    elevation: &[f32],
    id_to_idx: &HashMap<usize, usize>,
    rng: &mut StdRng,
) -> Vec<(Vec2, Vec2)> {
    let land_cells: Vec<usize> = cells.iter().enumerate()
        .filter(|(_, c)| c.is_land && !c.is_border && !c.is_coast)
        .map(|(i, _)| i)
        .collect();

    if land_cells.len() < 2 { return vec![]; }

    // Candidate anchor pool — in "Follow Terrain" mode prefer local maxima
    let anchors: Vec<usize> = if mode == "Follow Terrain" {
        let mut maxima: Vec<usize> = land_cells.iter().cloned().filter(|&i| {
            let cell = &cells[i];
            // Local maximum: elevation higher than all land neighbours
            cell.neighbor_ids.iter().all(|&nid| {
                id_to_idx.get(&nid)
                    .map_or(true, |&ni| elevation[ni] <= elevation[i])
            })
        }).collect();

        // If too few maxima fall back to top-quartile land cells
        if maxima.len() < num_ridges * 2 {
            let mut top = land_cells.clone();
            top.sort_by(|&a, &b| elevation[b].partial_cmp(&elevation[a]).unwrap_or(std::cmp::Ordering::Equal));
            maxima = top.into_iter().take((land_cells.len() / 4).max(num_ridges * 2)).collect();
        }
        maxima
    } else {
        land_cells.clone()
    };

    let mut ridges: Vec<(Vec2, Vec2)> = Vec::new();

    for _ in 0..num_ridges {
        if anchors.len() < 2 { break; }

        // Pick two anchors with minimum separation to avoid degenerate ridges
        let mut attempts = 0;
        loop {
            let ai = rng.random_range(0..anchors.len());
            let bi = rng.random_range(0..anchors.len());
            if ai == bi { attempts += 1; if attempts > 20 { break; } continue; }

            let a = cells[anchors[ai]].center;
            let b = cells[anchors[bi]].center;
            let separation = a.distance(b);

            // Require meaningful separation (avoid micro-ridges)
            if separation > 30.0 {
                ridges.push((a, b));
                break;
            }
            attempts += 1;
            if attempts > 30 { break; }
        }
    }

    ridges
}

// ---------------------------------------------------------------------------
// Fallback BFS base elevation (used when stage3_blend == 0)
// ---------------------------------------------------------------------------

fn bfs_base_elevation(cells: &[LandCell], id_to_idx: &HashMap<usize, usize>) -> Vec<f32> {
    let n = cells.len();
    let mut bfs_dist: Vec<i32> = vec![-1; n];
    let mut queue: VecDeque<usize> = VecDeque::new();

    for i in 0..n {
        let at_boundary = cells[i].neighbor_ids.iter().any(|&nid| {
            id_to_idx.get(&nid).map_or(false, |&ni| cells[ni].is_land != cells[i].is_land)
        });
        if at_boundary || cells[i].is_border {
            bfs_dist[i] = 0;
            queue.push_back(i);
        }
    }

    while let Some(idx) = queue.pop_front() {
        let d = bfs_dist[idx];
        for &nid in &cells[idx].neighbor_ids {
            if let Some(&ni) = id_to_idx.get(&nid) {
                if bfs_dist[ni] < 0 {
                    bfs_dist[ni] = d + 1;
                    queue.push_back(ni);
                }
            }
        }
    }

    let max_land = bfs_dist.iter().zip(cells.iter())
        .filter_map(|(&d, c)| if c.is_land && d > 0 { Some(d) } else { None })
        .max().unwrap_or(1) as f32;
    let max_ocean = bfs_dist.iter().zip(cells.iter())
        .filter_map(|(&d, c)| if !c.is_land && d > 0 { Some(d) } else { None })
        .max().unwrap_or(1) as f32;

    (0..n).map(|i| {
        let d = bfs_dist[i].max(0) as f32;
        if cells[i].is_border { -1.0 }
        else if cells[i].is_land { (d / max_land).clamp(0.01, 1.0) }
        else { -(d / max_ocean).clamp(0.01, 1.0) }
    }).collect()
}

// ---------------------------------------------------------------------------
// Geometry helper
// ---------------------------------------------------------------------------

fn point_to_segment_distance(p: Vec2, a: Vec2, b: Vec2) -> f32 {
    let ab = b - a;
    let len_sq = ab.dot(ab);
    if len_sq < 1e-8 { return p.distance(a); }
    let t = ((p - a).dot(ab) / len_sq).clamp(0.0, 1.0);
    p.distance(a + ab * t)
}
