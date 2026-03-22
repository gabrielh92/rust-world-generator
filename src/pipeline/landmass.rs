use std::collections::{HashMap, HashSet, VecDeque};

use noise::{NoiseFn, Perlin};
use rand::{Rng, SeedableRng};
use rand_pcg::Pcg64;

use crate::canvas::CanvasData;
use crate::mathlib::vec::Vec2;
use crate::params::util::build_default_landmass_params;
use crate::pipeline::voronoi::{Corner, GraphCell, GraphOutput, STAGE_DATA_KEY as VORONOI_KEY};
use crate::pipeline::{PipelineStage, PipelineStageExecutor, StageData};
use crate::visualization::landmass::LandmassVisualLayer;

pub fn make_landmass_stage() -> PipelineStage {
    PipelineStage {
        executor: Box::new(LandmassStage),
        params: Some(build_default_landmass_params()),
        visual_layer: Box::new(LandmassVisualLayer::default()),
    }
}

// ---------------------------------------------------------------------------
// Output data
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct LandCell {
    pub id: usize,
    pub center: Vec2,
    /// Indices into `LandmassOutput::corners`.
    pub corner_ids: Vec<usize>,
    /// Neighbouring cell IDs (Delaunay adjacency, carried from GraphOutput).
    pub neighbor_ids: Vec<usize>,
    pub is_border: bool,
    pub is_land: bool,
    /// Land cell that has at least one coast edge (borders outer ocean, not a lake).
    pub is_coast: bool,
    /// Ocean cell not reachable from the canvas border — enclosed by land.
    pub is_lake: bool,
    pub is_ocean: bool,
    /// -1.0 (deep ocean) … 0.0 (sea level) … 1.0 (peak).
    pub elevation: f32,
}

/// A Voronoi edge annotated with coast status.
/// `is_coast` is true when one side is land and the other is outer ocean (not a lake).
#[derive(Debug, Clone)]
pub struct LandEdge {
    pub id: usize,
    pub corner_a: usize,
    pub corner_b: usize,
    pub cell_a: usize,
    pub cell_b: Option<usize>,
    pub is_coast: bool,
}

#[derive(Debug, Clone)]
pub struct LandmassOutput {
    pub cells: Vec<LandCell>,
    /// Corners carried forward from GraphOutput (positions unchanged).
    pub corners: Vec<Corner>,
    /// Edges annotated with coast status.
    pub edges: Vec<LandEdge>,
}

impl StageData for LandmassOutput {
    fn as_any(&self) -> &dyn std::any::Any { self }
}

// ---------------------------------------------------------------------------
// Stage executor
// ---------------------------------------------------------------------------

pub const STAGE_DATA_KEY: &str = "stage:landmass";

pub struct LandmassStage;

impl PipelineStageExecutor for LandmassStage {
    fn name(&self) -> &str { "landmass" }
    fn rank(&self) -> u8 { 3 }
    fn data_key(&self) -> &'static str { STAGE_DATA_KEY }

    fn run(&mut self, params: Option<&crate::params::ParamGroup>, data: &super::StageDataMap) -> Box<dyn StageData> {
        let pg = params.expect("Landmass params required");

        let mode = pg.get_param("Shaping Mode").map(|p| p.value.as_str()).unwrap_or("Continent Seeds");

        // Radial params
        let falloff       = pg.get_param("Falloff").and_then(|p| p.as_float()).unwrap_or(0.35);
        let x_scale       = pg.get_param("X Scale").and_then(|p| p.as_float()).unwrap_or(0.8);
        let y_scale       = pg.get_param("Y Scale").and_then(|p| p.as_float()).unwrap_or(0.8);
        // Shared noise
        let noise_scale   = pg.get_param("Noise Scale").and_then(|p| p.as_float()).unwrap_or(0.004);
        let elev_noise    = pg.get_param("Elevation Noise Intensity").and_then(|p| p.as_float()).unwrap_or(0.4);
        // Noise threshold params
        let land_thresh   = pg.get_param("Land Threshold").and_then(|p| p.as_float()).unwrap_or(0.05);
        // Continent seed params
        let num_seeds     = pg.get_param("Num Seeds").and_then(|p| p.as_int()).unwrap_or(5);
        let spread_prob   = pg.get_param("Spread Probability").and_then(|p| p.as_float()).unwrap_or(0.72);
        let noise_influence = pg.get_param("Noise Influence").and_then(|p| p.as_float()).unwrap_or(0.25);

        let canvas = data
            .get("canvas")
            .and_then(|c| c.as_any().downcast_ref::<CanvasData>())
            .expect("Canvas data required for landmass stage");

        let graph = data
            .get(VORONOI_KEY)
            .and_then(|d| d.as_any().downcast_ref::<GraphOutput>())
            .expect("Graph (voronoi) stage must run before landmass stage");

        let cells = &graph.cells;
        let n = cells.len();

        // Seed derived from world seed + stage constant
        let seed = canvas.seed ^ 0x4C414E44_4D415353u64; // "LANDMASS"
        let perlin = Perlin::new(canvas.seed as u32);

        // --- Step 1: Determine is_land per cell ---
        let is_land: Vec<bool> = match mode {
            "Radial Gradient" => radial_land(cells, canvas.center, x_scale, y_scale, falloff, noise_scale, land_thresh, &perlin),
            "Noise Threshold" => noise_threshold_land(cells, noise_scale, land_thresh, &perlin),
            _                 => continent_seeds_land(cells, num_seeds, spread_prob, noise_influence, noise_scale, seed, &perlin),
        };

        // --- Step 2: Build id→index map for neighbour lookups ---
        let id_to_idx: HashMap<usize, usize> =
            cells.iter().enumerate().map(|(i, c)| (c.id, i)).collect();

        // --- Step 2b: Morphological closing — fill thin ocean channels within the landmass ---
        // Pass 1-4: any non-border ocean cell where the majority of its neighbours are land
        // gets converted to land. Removes hairline channels that make the coastline fractal.
        // This runs BEFORE the outer-ocean BFS so the BFS sees the cleaned-up mask.
        let mut is_land: Vec<bool> = is_land;
        for _ in 0..4 {
            let prev = is_land.clone();
            for i in 0..n {
                if prev[i] || cells[i].is_border { continue; }
                let neighbors: Vec<usize> = cells[i].neighbor_ids.iter()
                    .filter_map(|&nid| id_to_idx.get(&nid).copied())
                    .collect();
                if neighbors.is_empty() { continue; }
                let land_neighbors = neighbors.iter().filter(|&&ni| prev[ni]).count();
                if land_neighbors * 2 > neighbors.len() {
                    is_land[i] = true;
                }
            }
        }

        // Pass 5: fill small enclosed ocean components entirely.
        // After the initial closing, small disconnected ocean pockets remain.
        // Any connected ocean component of < MIN_LAKE_CELLS that doesn't touch the
        // canvas border is converted to land, giving a smoother coastline.
        const MIN_LAKE_CELLS: usize = 8;
        {
            let mut visited = vec![false; n];
            for start in 0..n {
                if is_land[start] || cells[start].is_border || visited[start] { continue; }
                // BFS to find the connected ocean component
                let mut component = Vec::new();
                let mut touches_border = false;
                let mut queue = VecDeque::new();
                queue.push_back(start);
                visited[start] = true;
                while let Some(idx) = queue.pop_front() {
                    component.push(idx);
                    if cells[idx].is_border { touches_border = true; }
                    for &nid in &cells[idx].neighbor_ids {
                        if let Some(&ni) = id_to_idx.get(&nid) {
                            if !visited[ni] && !is_land[ni] {
                                visited[ni] = true;
                                queue.push_back(ni);
                            }
                        }
                    }
                }
                if !touches_border && component.len() < MIN_LAKE_CELLS {
                    for idx in component { is_land[idx] = true; }
                }
            }
        }

        // --- Step 3: BFS from border ocean cells to distinguish outer ocean from enclosed lakes ---
        let mut is_outer_ocean = vec![false; n];
        {
            let mut queue: VecDeque<usize> = VecDeque::new();
            for (i, cell) in cells.iter().enumerate() {
                if cell.is_border && !is_land[i] {
                    is_outer_ocean[i] = true;
                    queue.push_back(i);
                }
            }
            while let Some(idx) = queue.pop_front() {
                for &nid in &cells[idx].neighbor_ids {
                    if let Some(&ni) = id_to_idx.get(&nid) {
                        if !is_outer_ocean[ni] && !is_land[ni] {
                            is_outer_ocean[ni] = true;
                            queue.push_back(ni);
                        }
                    }
                }
            }
        }

        // --- Step 4: Annotate edges with is_coast (land ↔ outer ocean boundary) ---
        let land_edges: Vec<LandEdge> = graph.edges.iter().map(|e| {
            let is_coast = if let Some(cb) = e.cell_b {
                let ia = id_to_idx.get(&e.cell_a).copied();
                let ib = id_to_idx.get(&cb).copied();
                match (ia, ib) {
                    (Some(a), Some(b)) => {
                        (is_land[a] && is_outer_ocean[b]) || (is_land[b] && is_outer_ocean[a])
                    }
                    _ => false,
                }
            } else {
                false
            };
            LandEdge { id: e.id, corner_a: e.corner_a, corner_b: e.corner_b, cell_a: e.cell_a, cell_b: e.cell_b, is_coast }
        }).collect();

        // --- Step 5: Derive per-cell coast flag from coast edges ---
        // A land cell is coastal only if it has at least one coast edge.
        // This naturally excludes lake-adjacent land cells.
        let mut cell_has_coast_edge = vec![false; n];
        for edge in &land_edges {
            if !edge.is_coast { continue; }
            if let Some(ia) = id_to_idx.get(&edge.cell_a) { cell_has_coast_edge[*ia] = true; }
            if let Some(cb) = edge.cell_b {
                if let Some(ib) = id_to_idx.get(&cb) { cell_has_coast_edge[*ib] = true; }
            }
        }

        // --- Step 6: BFS distance from land/ocean boundary for elevation ---
        let elevation = compute_elevation(cells, &id_to_idx, &is_land, noise_scale, elev_noise, &perlin);

        // --- Step 7: Assemble LandCell output ---
        let land_cells: Vec<LandCell> = cells.iter().enumerate().map(|(i, gc)| {
            let is_lake = !is_land[i] && !is_outer_ocean[i] && !gc.is_border;
            LandCell {
                id: gc.id,
                center: gc.center,
                corner_ids: gc.corner_ids.clone(),
                neighbor_ids: gc.neighbor_ids.clone(),
                is_border: gc.is_border,
                is_land: is_land[i],
                is_coast: cell_has_coast_edge[i],
                is_lake,
                is_ocean: !is_land[i],
                elevation: elevation[i],
            }
        }).collect();

        Box::new(LandmassOutput {
            cells: land_cells,
            corners: graph.corners.clone(),
            edges: land_edges,
        })
    }
}

// ---------------------------------------------------------------------------
// Shaping modes
// ---------------------------------------------------------------------------

/// Radial gradient: cells near the canvas centre tend to be land.
/// base_elevation = -tanh((normalised_distance - 1) / falloff)
/// Noise is added on top; `land_thresh` acts as the sea-level offset.
fn radial_land(
    cells: &[GraphCell],
    center: Vec2,
    x_scale: f32,
    y_scale: f32,
    falloff: f32,
    noise_scale: f32,
    _land_thresh: f32,
    perlin: &Perlin,
) -> Vec<bool> {
    let ref_radius = compute_ref_radius(cells, center);

    cells.iter().map(|cell| {
        if cell.is_border { return false; }

        let dx = (cell.center.x - center.x) / (ref_radius * x_scale);
        let dy = (cell.center.y - center.y) / (ref_radius * y_scale);
        let d  = (dx * dx + dy * dy).sqrt();

        // Fixed formula (no 0. * bug): -tanh yields +1 at centre, -1 at edges
        let base = -((d - 1.0) / falloff).tanh() as f64;
        let noise = perlin.get([
            cell.center.x as f64 * noise_scale as f64,
            cell.center.y as f64 * noise_scale as f64,
        ]);
        (base + noise * 0.3) > 0.0
    }).collect()
}

/// Pure Perlin noise threshold: cells where noise(pos) > threshold become land.
fn noise_threshold_land(
    cells: &[GraphCell],
    noise_scale: f32,
    threshold: f32,
    perlin: &Perlin,
) -> Vec<bool> {
    cells.iter().map(|cell| {
        if cell.is_border { return false; }
        let val = perlin.get([
            cell.center.x as f64 * noise_scale as f64,
            cell.center.y as f64 * noise_scale as f64,
        ]) as f32;
        val > threshold
    }).collect()
}

/// Continent seeds: place N seed cells, BFS flood-fill outward with noise-perturbed probability.
/// Produces distinct separated landmasses — the preferred mode for FFE.
fn continent_seeds_land(
    cells: &[GraphCell],
    num_seeds: usize,
    spread_prob: f32,
    noise_influence: f32,
    noise_scale: f32,
    seed: u64,
    perlin: &Perlin,
) -> Vec<bool> {
    let mut rng = Pcg64::seed_from_u64(seed);
    let id_to_idx: HashMap<usize, usize> =
        cells.iter().enumerate().map(|(i, c)| (c.id, i)).collect();

    // Pick non-border cells as continent anchors
    let candidates: Vec<usize> = cells.iter()
        .enumerate()
        .filter(|(_, c)| !c.is_border)
        .map(|(i, _)| i)
        .collect();

    let mut is_land = vec![false; cells.len()];
    let mut queue: VecDeque<usize> = VecDeque::new();
    let mut visited: HashSet<usize> = HashSet::new();

    // Seed cells — attempt to space them apart for better continent separation
    let actual_seeds = num_seeds.min(candidates.len());
    let mut seeds_placed = 0;
    let mut attempts = 0;
    let min_sep = if cells.len() > 0 {
        let avg_x = cells.iter().map(|c| c.center.x).sum::<f32>() / cells.len() as f32;
        avg_x * 0.5 // rough minimum separation
    } else { 0.0 };

    let mut seed_positions: Vec<Vec2> = Vec::new();

    while seeds_placed < actual_seeds && attempts < candidates.len() * 4 {
        let pick = candidates[rng.random_range(0..candidates.len())];
        let pos = cells[pick].center;
        // Ensure seeds aren't too clustered
        let too_close = seed_positions.iter().any(|&sp| pos.distance(sp) < min_sep);
        if !too_close || seed_positions.is_empty() {
            seed_positions.push(pos);
            is_land[pick] = true;
            visited.insert(pick);
            queue.push_back(pick);
            seeds_placed += 1;
        }
        attempts += 1;
    }
    // Fallback: fill remaining seeds without spacing constraint
    while seeds_placed < actual_seeds {
        let pick = candidates[rng.random_range(0..candidates.len())];
        if !visited.contains(&pick) {
            is_land[pick] = true;
            visited.insert(pick);
            queue.push_back(pick);
            seeds_placed += 1;
        }
    }

    // BFS flood fill
    while let Some(idx) = queue.pop_front() {
        for &nid in &cells[idx].neighbor_ids {
            let ni = match id_to_idx.get(&nid) { Some(&i) => i, None => continue };
            if visited.contains(&ni) { continue; }
            if cells[ni].is_border {
                visited.insert(ni);
                continue;
            }
            visited.insert(ni);

            let noise_val = perlin.get([
                cells[ni].center.x as f64 * noise_scale as f64,
                cells[ni].center.y as f64 * noise_scale as f64,
            ]) as f32;
            let prob = (spread_prob + noise_val * noise_influence).clamp(0.0, 1.0);

            if rng.random::<f32>() < prob {
                is_land[ni] = true;
                queue.push_back(ni);
            }
        }
    }

    is_land
}

// ---------------------------------------------------------------------------
// Elevation
// ---------------------------------------------------------------------------

/// BFS distance from the land/ocean boundary, normalised and combined with noise.
/// Land cells: elevation in [0.0, 1.0] (coasts near 0, interiors near 1).
/// Ocean cells: elevation in [-1.0, 0.0] (coast-adjacent near 0, far ocean near -1).
fn compute_elevation(
    cells: &[GraphCell],
    id_to_idx: &HashMap<usize, usize>,
    is_land: &[bool],
    noise_scale: f32,
    noise_intensity: f32,
    perlin: &Perlin,
) -> Vec<f32> {
    let n = cells.len();

    // Find boundary cells: those with at least one neighbour on the other side
    let mut bfs_dist: Vec<i32> = vec![-1; n];
    let mut queue: VecDeque<usize> = VecDeque::new();

    for i in 0..n {
        let at_boundary = cells[i].neighbor_ids.iter().any(|&nid| {
            id_to_idx.get(&nid).map_or(false, |&ni| is_land[ni] != is_land[i])
        });
        if at_boundary {
            bfs_dist[i] = 0;
            queue.push_back(i);
        }
    }
    // Border cells are always ocean with zero elevation
    for i in 0..n {
        if cells[i].is_border && bfs_dist[i] < 0 {
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

    let max_land = bfs_dist.iter().zip(is_land.iter())
        .filter_map(|(&d, &l)| if l && d > 0 { Some(d) } else { None })
        .max().unwrap_or(1) as f32;
    let max_ocean = bfs_dist.iter().zip(is_land.iter())
        .filter_map(|(&d, &l)| if !l && d > 0 { Some(d) } else { None })
        .max().unwrap_or(1) as f32;

    (0..n).map(|i| {
        let d = bfs_dist[i].max(0) as f32;
        let noise_val = perlin.get([
            cells[i].center.x as f64 * noise_scale as f64 * 2.0,
            cells[i].center.y as f64 * noise_scale as f64 * 2.0,
        ]) as f32;

        if cells[i].is_border {
            -1.0
        } else if is_land[i] {
            let base = d / max_land;
            (base + noise_val * noise_intensity).clamp(0.02, 1.0)
        } else {
            let base = -(d / max_ocean);
            (base + noise_val * noise_intensity * 0.3).clamp(-1.0, -0.02)
        }
    }).collect()
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn compute_ref_radius(cells: &[GraphCell], center: Vec2) -> f32 {
    if cells.is_empty() { return 1.0; }
    let sum: f32 = cells.iter().map(|c| c.center.distance(center)).sum();
    (sum / cells.len() as f32).max(1e-4)
}
