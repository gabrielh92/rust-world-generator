use std::collections::{HashMap, VecDeque};

use noise::{NoiseFn, Perlin};

use crate::canvas::CanvasData;
use crate::mathlib::vec::Vec2;
use crate::params::util::build_default_terrain_params;
use crate::pipeline::landmass::LandEdge;
use crate::pipeline::voronoi::{Corner, GraphOutput, STAGE_DATA_KEY as VORONOI_KEY};
use crate::pipeline::{PipelineStage, PipelineStageExecutor, StageData};
use crate::visualization::terrain::TerrainVisualLayer;

pub const STAGE_DATA_KEY: &str = "stage:terrain";

pub fn make_terrain_stage() -> PipelineStage {
    PipelineStage {
        executor: Box::new(TerrainStage),
        params: Some(build_default_terrain_params()),
        visual_layer: Box::new(TerrainVisualLayer::default()),
    }
}

// ---------------------------------------------------------------------------
// Output data
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct TerrainCell {
    pub id: usize,
    pub center: Vec2,
    pub corner_ids: Vec<usize>,
    pub neighbor_ids: Vec<usize>,
    pub is_border: bool,
    pub is_land: bool,
    pub is_coast: bool,
    pub is_lake: bool,
    pub is_ocean: bool,
    pub elevation: f32,
    pub continentalness: f32,
    pub erosion: f32,
    pub peaks_valleys: f32,
}

#[derive(Debug, Clone)]
pub struct TerrainOutput {
    pub cells: Vec<TerrainCell>,
    pub corners: Vec<Corner>,
    pub edges: Vec<LandEdge>,
    pub corner_elevations: Vec<f32>,
    pub continentalness_map: Vec<f32>,
    pub erosion_map: Vec<f32>,
    pub peaks_valleys_map: Vec<f32>,
}

impl StageData for TerrainOutput {
    fn as_any(&self) -> &dyn std::any::Any { self }
}

// ---------------------------------------------------------------------------
// Stage executor
// ---------------------------------------------------------------------------

pub struct TerrainStage;

impl PipelineStageExecutor for TerrainStage {
    fn name(&self) -> &str { "terrain" }
    fn rank(&self) -> u8 { 3 }
    fn data_key(&self) -> &'static str { STAGE_DATA_KEY }

    fn run(&mut self, params: Option<&crate::params::ParamGroup>, data: &super::StageDataMap) -> Box<dyn StageData> {
        let pg = params.expect("Terrain params required");

        // --- Read continentalness noise params ---
        let c_scale       = pg.get_param("C Scale").and_then(|p| p.as_float()).unwrap_or(0.003) as f64;
        let c_octaves     = pg.get_param("C Octaves").and_then(|p| p.as_int()).unwrap_or(5);
        let c_lacunarity  = pg.get_param("C Lacunarity").and_then(|p| p.as_float()).unwrap_or(2.0) as f64;
        let c_persistence = pg.get_param("C Persistence").and_then(|p| p.as_float()).unwrap_or(0.5) as f64;

        // --- Read erosion noise params ---
        let e_scale       = pg.get_param("E Scale").and_then(|p| p.as_float()).unwrap_or(0.006) as f64;
        let e_octaves     = pg.get_param("E Octaves").and_then(|p| p.as_int()).unwrap_or(4);
        let e_lacunarity  = pg.get_param("E Lacunarity").and_then(|p| p.as_float()).unwrap_or(2.0) as f64;
        let e_persistence = pg.get_param("E Persistence").and_then(|p| p.as_float()).unwrap_or(0.5) as f64;

        // --- Read peaks & valleys noise params ---
        let pv_scale       = pg.get_param("PV Scale").and_then(|p| p.as_float()).unwrap_or(0.008) as f64;
        let pv_octaves     = pg.get_param("PV Octaves").and_then(|p| p.as_int()).unwrap_or(4);
        let pv_lacunarity  = pg.get_param("PV Lacunarity").and_then(|p| p.as_float()).unwrap_or(2.0) as f64;
        let pv_persistence = pg.get_param("PV Persistence").and_then(|p| p.as_float()).unwrap_or(0.5) as f64;

        // --- Read spline anchor params ---
        let c_ocean_depth = pg.get_param("C Ocean Depth").and_then(|p| p.as_float()).unwrap_or(-0.5);
        let c_shoreline   = pg.get_param("C Shoreline").and_then(|p| p.as_float()).unwrap_or(0.02);
        let c_interior    = pg.get_param("C Interior").and_then(|p| p.as_float()).unwrap_or(0.5);

        let e_craggy_amp  = pg.get_param("E Craggy Amp").and_then(|p| p.as_float()).unwrap_or(1.0);
        let e_flat_amp    = pg.get_param("E Flat Amp").and_then(|p| p.as_float()).unwrap_or(0.1);

        let pv_valley_offset = pg.get_param("PV Valley Offset").and_then(|p| p.as_float()).unwrap_or(-0.3);
        let pv_peak_offset   = pg.get_param("PV Peak Offset").and_then(|p| p.as_float()).unwrap_or(0.4);

        // --- Build splines ---
        let continentalness_spline: Vec<(f32, f32)> = vec![
            (-1.0, c_ocean_depth),
            (-0.2, c_ocean_depth * 0.2),
            (0.0,  c_shoreline),
            (0.3,  c_shoreline + (c_interior - c_shoreline) * 0.4),
            (1.0,  c_interior),
        ];
        let erosion_spline: Vec<(f32, f32)> = vec![
            (0.0, e_craggy_amp),
            (0.5, (e_craggy_amp + e_flat_amp) * 0.5),
            (1.0, e_flat_amp),
        ];
        let pv_spline: Vec<(f32, f32)> = vec![
            (-1.0, pv_valley_offset),
            (0.0,  0.0),
            (1.0,  pv_peak_offset),
        ];

        // --- Fetch upstream data ---
        let canvas = data
            .get("canvas")
            .and_then(|c| c.as_any().downcast_ref::<CanvasData>())
            .expect("Canvas data required for terrain stage");

        let graph = data
            .get(VORONOI_KEY)
            .and_then(|d| d.as_any().downcast_ref::<GraphOutput>())
            .expect("Voronoi stage must run before terrain stage");

        let cells = &graph.cells;
        let n = cells.len();

        let seed = canvas.seed;

        // --- Create noise generators with derived seeds ---
        let cont_seed  = (seed ^ 0x434F4E54494E454Eu64) as u32;
        let eros_seed  = (seed ^ 0x45524F53494F4E00u64) as u32;
        let pv_seed    = (seed ^ 0x504B5356414C4C45u64) as u32;

        let perlin_c  = Perlin::new(cont_seed);
        let perlin_e  = Perlin::new(eros_seed);
        let perlin_pv = Perlin::new(pv_seed);
        let perlin_corner = Perlin::new(seed.wrapping_add(7) as u32);

        // --- Build id→index map ---
        let id_to_idx: HashMap<usize, usize> =
            cells.iter().enumerate().map(|(i, c)| (c.id, i)).collect();

        // --- Step 1: Sample three noise channels per cell ---
        let mut continentalness_map = vec![0.0_f32; n];
        let mut erosion_map         = vec![0.0_f32; n];
        let mut peaks_valleys_map   = vec![0.0_f32; n];

        for (i, cell) in cells.iter().enumerate() {
            if cell.is_border {
                // Border cells are always deep ocean
                continentalness_map[i] = -1.0;
                erosion_map[i]         = 1.0;
                peaks_valleys_map[i]   = -1.0;
                continue;
            }

            let x = cell.center.x as f64;
            let y = cell.center.y as f64;

            let c_raw = fbm(&perlin_c, x * c_scale, y * c_scale, c_octaves, c_lacunarity, c_persistence);
            continentalness_map[i] = c_raw as f32;

            let e_raw = fbm(&perlin_e, x * e_scale, y * e_scale, e_octaves, e_lacunarity, e_persistence);
            // remap [-1, 1] → [0, 1]
            erosion_map[i] = ((e_raw as f32) * 0.5 + 0.5).clamp(0.0, 1.0);

            let pv_raw = ridged_fbm(&perlin_pv, x * pv_scale, y * pv_scale, pv_octaves, pv_lacunarity, pv_persistence);
            peaks_valleys_map[i] = pv_raw as f32;
        }

        // --- Step 2: Determine is_land from continentalness ---
        let mut is_land: Vec<bool> = continentalness_map.iter().enumerate()
            .map(|(i, &c)| !cells[i].is_border && c > 0.0)
            .collect();

        // --- Step 3: Morphological closing (copied from landmass.rs) ---
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

        // Fill small enclosed ocean components
        const MIN_LAKE_CELLS: usize = 8;
        {
            let mut visited = vec![false; n];
            for start in 0..n {
                if is_land[start] || cells[start].is_border || visited[start] { continue; }
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

        // --- Step 4: BFS to distinguish outer ocean from lakes ---
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

        // --- Step 5: Annotate edges with is_coast ---
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

        // --- Step 6: Derive per-cell coast flag ---
        let mut cell_has_coast_edge = vec![false; n];
        for edge in &land_edges {
            if !edge.is_coast { continue; }
            if let Some(ia) = id_to_idx.get(&edge.cell_a) { cell_has_coast_edge[*ia] = true; }
            if let Some(cb) = edge.cell_b {
                if let Some(ib) = id_to_idx.get(&cb) { cell_has_coast_edge[*ib] = true; }
            }
        }

        // --- Step 7: Compute elevation via splines ---
        let mut cell_elevations = vec![0.0_f32; n];
        for i in 0..n {
            if cells[i].is_border {
                cell_elevations[i] = -1.0;
                continue;
            }
            let c  = continentalness_map[i];
            let e  = erosion_map[i];
            let pv = peaks_valleys_map[i];

            let c_elev  = eval_spline(&continentalness_spline, c);
            let e_scale = eval_spline(&erosion_spline, e);
            let pv_off  = eval_spline(&pv_spline, pv);

            cell_elevations[i] = (c_elev + e_scale * pv_off).clamp(-1.0, 1.0);
        }

        // --- Step 8: Corner elevations (average of adjacent cells + small noise) ---
        let corner_noise_scale = 0.01_f64;
        let corner_elevations: Vec<f32> = graph.corners.iter().map(|corner| {
            if corner.adjacent_cell_ids.is_empty() {
                return -1.0;
            }
            let avg: f32 = corner.adjacent_cell_ids.iter()
                .filter_map(|&cid| id_to_idx.get(&cid).map(|&i| cell_elevations[i]))
                .sum::<f32>()
                / corner.adjacent_cell_ids.len() as f32;
            let noise = perlin_corner.get([
                corner.position.x as f64 * corner_noise_scale,
                corner.position.y as f64 * corner_noise_scale,
            ]) as f32;
            (avg + noise * 0.05).clamp(-1.0, 1.0)
        }).collect();

        // --- Step 9: Assemble TerrainCell output ---
        let terrain_cells: Vec<TerrainCell> = cells.iter().enumerate().map(|(i, gc)| {
            let is_lake = !is_land[i] && !is_outer_ocean[i] && !gc.is_border;
            TerrainCell {
                id: gc.id,
                center: gc.center,
                corner_ids: gc.corner_ids.clone(),
                neighbor_ids: gc.neighbor_ids.clone(),
                is_border: gc.is_border,
                is_land: is_land[i],
                is_coast: cell_has_coast_edge[i],
                is_lake,
                is_ocean: !is_land[i],
                elevation: cell_elevations[i],
                continentalness: continentalness_map[i],
                erosion: erosion_map[i],
                peaks_valleys: peaks_valleys_map[i],
            }
        }).collect();

        Box::new(TerrainOutput {
            cells: terrain_cells,
            corners: graph.corners.clone(),
            edges: land_edges,
            corner_elevations,
            continentalness_map,
            erosion_map,
            peaks_valleys_map,
        })
    }
}

// ---------------------------------------------------------------------------
// Noise functions
// ---------------------------------------------------------------------------

fn fbm(perlin: &Perlin, x: f64, y: f64, octaves: usize, lacunarity: f64, persistence: f64) -> f64 {
    let (mut val, mut amp, mut freq, mut max_val) = (0.0, 1.0, 1.0, 0.0);
    for _ in 0..octaves {
        val     += perlin.get([x * freq, y * freq]) * amp;
        max_val += amp;
        amp     *= persistence;
        freq    *= lacunarity;
    }
    val / max_val
}

fn ridged_fbm(perlin: &Perlin, x: f64, y: f64, octaves: usize, lacunarity: f64, persistence: f64) -> f64 {
    let raw = fbm(perlin, x, y, octaves, lacunarity, persistence);
    1.0 - 2.0 * raw.abs()
}

// ---------------------------------------------------------------------------
// Spline evaluation
// ---------------------------------------------------------------------------

pub fn eval_spline(points: &[(f32, f32)], x: f32) -> f32 {
    if points.is_empty() { return 0.0; }
    if x <= points[0].0 { return points[0].1; }
    if x >= points[points.len() - 1].0 { return points[points.len() - 1].1; }

    for i in 0..points.len() - 1 {
        let (x0, y0) = points[i];
        let (x1, y1) = points[i + 1];
        if x >= x0 && x <= x1 {
            let t = (x - x0) / (x1 - x0);
            return y0 + t * (y1 - y0);
        }
    }
    points[points.len() - 1].1
}
