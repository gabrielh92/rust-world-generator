use std::collections::HashMap;

use crate::params::util::build_default_river_params;
use crate::pipeline::elevation::{ElevationOutput, STAGE_DATA_KEY as ELEVATION_KEY};
use crate::pipeline::landmass::{LandmassOutput, STAGE_DATA_KEY as LANDMASS_KEY};
use crate::pipeline::moisture::{MoistureOutput, STAGE_DATA_KEY as MOISTURE_KEY};
use crate::pipeline::voronoi::Corner;
use crate::pipeline::{PipelineStage, PipelineStageExecutor, StageData, StageDataMap};
use crate::visualization::river::RiverVisualLayer;

pub const STAGE_DATA_KEY: &str = "stage:river";

pub fn make_river_stage() -> PipelineStage {
    PipelineStage {
        executor: Box::new(RiverStage),
        params: Some(build_default_river_params()),
        visual_layer: Box::new(RiverVisualLayer::default()),
    }
}

// ---------------------------------------------------------------------------
// Output
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RiverTerminus {
    Ocean,
    Lake,
    Sink,
}

#[derive(Debug, Clone)]
pub struct RiverPath {
    /// Corner array indices, source → mouth.
    pub corners: Vec<usize>,
    pub mouth_flow: f32,
    pub terminus: RiverTerminus,
}

#[derive(Debug, Clone)]
pub struct RiverOutput {
    pub rivers: Vec<RiverPath>,
    /// Normalized [0, 1], same length as landmass.corners.
    pub corner_flow: Vec<f32>,
    pub cell_has_river: Vec<bool>,
    pub cell_river_flow: Vec<f32>,
    /// Cells that are river-created lake basins (sink depressions above depth threshold).
    /// Same length as landmass.cells.
    pub lake_cells: Vec<bool>,
}

impl StageData for RiverOutput {
    fn as_any(&self) -> &dyn std::any::Any { self }
}

// ---------------------------------------------------------------------------
// Stage executor
// ---------------------------------------------------------------------------

pub struct RiverStage;

impl PipelineStageExecutor for RiverStage {
    fn name(&self) -> &str { "river" }
    fn rank(&self) -> u8 { 6 }
    fn data_key(&self) -> &'static str { STAGE_DATA_KEY }

    fn run(&mut self, params: Option<&crate::params::ParamGroup>, data: &StageDataMap) -> Box<dyn StageData> {
        let pg = params.expect("River params required");

        let erosion_mode    = pg.get_param("Erosion").map(|p| p.value.as_str()).unwrap_or("None");
        let carve_strength  = pg.get_param("Carve Strength").and_then(|p| p.as_float()).unwrap_or(0.08);
        let min_flow        = pg.get_param("Min Flow").and_then(|p| p.as_float()).unwrap_or(0.04);
        let min_length      = pg.get_param("Min Length").and_then(|p| p.as_int()).unwrap_or(4);
        let lake_depth_threshold = pg.get_param("Lake Depth Threshold").and_then(|p| p.as_float()).unwrap_or(0.05);

        let landmass = data.get(LANDMASS_KEY)
            .and_then(|d| d.as_any().downcast_ref::<LandmassOutput>())
            .expect("Landmass stage must run before River stage");

        let elevation_out = data.get(ELEVATION_KEY)
            .and_then(|d| d.as_any().downcast_ref::<ElevationOutput>())
            .expect("Elevation stage must run before River stage");

        let moisture_out = data.get(MOISTURE_KEY)
            .and_then(|d| d.as_any().downcast_ref::<MoistureOutput>());

        let cells = &landmass.cells;
        let corners = &landmass.corners;
        let n_corners = corners.len();
        let n_cells = cells.len();

        // Build cell id → array index map
        let cell_id_to_idx: HashMap<usize, usize> =
            cells.iter().enumerate().map(|(i, c)| (c.id, i)).collect();

        // --- Step 1: Clone corner elevations (mutable for erosion) ---
        let mut corner_elev = elevation_out.corner_elevations.clone();

        // --- Step 2: Classify corners ---
        let mut corner_is_land = vec![false; n_corners];
        let mut corner_is_ocean_adj = vec![false; n_corners];
        let mut corner_is_lake_adj = vec![false; n_corners];

        for ci in 0..n_corners {
            for &cid in &corners[ci].adjacent_cell_ids {
                if let Some(&cell_idx) = cell_id_to_idx.get(&cid) {
                    let cell = &cells[cell_idx];
                    if cell.is_land && !cell.is_border {
                        corner_is_land[ci] = true;
                    }
                    if !cell.is_land && !cell.is_lake && !cell.is_border {
                        corner_is_ocean_adj[ci] = true;
                    }
                    if cell.is_lake {
                        corner_is_lake_adj[ci] = true;
                    }
                }
            }
        }

        // --- Step 3: Assign rainfall per corner ---
        let corner_rainfall: Vec<f32> = (0..n_corners).map(|ci| {
            let mut total = 0.0_f32;
            let mut count = 0usize;
            for &cid in &corners[ci].adjacent_cell_ids {
                if let Some(&cell_idx) = cell_id_to_idx.get(&cid) {
                    let cell = &cells[cell_idx];
                    if cell.is_land && !cell.is_border {
                        let m = moisture_out
                            .and_then(|mo| mo.cell_moisture.get(cell_idx).copied())
                            .unwrap_or(0.5);
                        total += m;
                        count += 1;
                    }
                }
            }
            if count == 0 { 0.5 } else { total / count as f32 }
        }).collect();

        // --- Step 4: Accumulate flow ---
        let (mut corner_flow, mut flow_dir) = accumulate_flow(
            n_corners,
            &corner_elev,
            &corner_rainfall,
            &corner_is_land,
            corners,
        );

        // --- Step 5: Optional flow-weighted erosion ---
        if erosion_mode == "Flow Weighted" {
            for ci in 0..n_corners {
                if !corner_is_land[ci] { continue; }
                if corner_elev[ci] > 0.01 {
                    corner_elev[ci] = (corner_elev[ci] - corner_flow[ci].sqrt() * carve_strength).max(0.01);
                }
            }
            let (new_flow, new_dir) = accumulate_flow(
                n_corners,
                &corner_elev,
                &corner_rainfall,
                &corner_is_land,
                corners,
            );
            corner_flow = new_flow;
            flow_dir = new_dir;
        }

        // --- Step 6: Normalize corner_flow to [0, 1] ---
        let max_flow = corner_flow.iter().cloned().fold(0.0_f32, f32::max);
        if max_flow > 1e-9 {
            for f in &mut corner_flow {
                *f /= max_flow;
            }
        }

        // --- Step 7: Find headwater corners ---
        // A headwater corner is one that is NOT the flow_dir target of any other corner.
        let mut has_upstream = vec![false; n_corners];
        for ci in 0..n_corners {
            if !corner_is_land[ci] { continue; }
            if let Some(target) = flow_dir[ci] {
                has_upstream[target] = true;
            }
        }

        let mut headwaters: Vec<usize> = (0..n_corners)
            .filter(|&ci| corner_is_land[ci] && !has_upstream[ci])
            .collect();

        // Sort descending by elevation so we trace from highest peaks first
        headwaters.sort_by(|&a, &b| {
            corner_elev[b].partial_cmp(&corner_elev[a]).unwrap_or(std::cmp::Ordering::Equal)
        });

        // --- Step 8: River tracing ---
        let mut corner_in_river = vec![false; n_corners];
        let mut rivers: Vec<RiverPath> = Vec::new();
        // Sink corners where depth >= threshold — become lakes after path filtering
        let mut lake_sink_corners: Vec<usize> = Vec::new();

        for start in headwaters {
            let mut path: Vec<usize> = vec![start];
            let mut current = start;
            let mut sink_lake_corner: Option<usize> = None;

            let terminus = loop {
                if corner_is_ocean_adj[current] {
                    break RiverTerminus::Ocean;
                }
                if corner_is_lake_adj[current] {
                    break RiverTerminus::Lake;
                }
                if corner_in_river[current] && path.len() > 1 {
                    // Joined an existing river
                    break RiverTerminus::Ocean; // inherit — will be filtered by flow anyway
                }

                match flow_dir[current] {
                    None => {
                        // Sink — check if it's a lake depression
                        let spill_elev = corners[current].adjacent_corner_ids.iter()
                            .filter_map(|&adj| {
                                if corner_is_land[adj] { Some(corner_elev[adj]) } else { None }
                            })
                            .fold(f32::MAX, f32::min);

                        let depth = if spill_elev < f32::MAX {
                            spill_elev - corner_elev[current]
                        } else {
                            0.0
                        };

                        if depth >= lake_depth_threshold {
                            sink_lake_corner = Some(current);
                            break RiverTerminus::Lake;
                        } else {
                            // Try to spill over to lowest adjacent corner not already in path
                            let spill_candidate = corners[current].adjacent_corner_ids.iter()
                                .cloned()
                                .filter(|&adj| corner_is_land[adj] && !path.contains(&adj))
                                .min_by(|&a, &b| {
                                    corner_elev[a].partial_cmp(&corner_elev[b])
                                        .unwrap_or(std::cmp::Ordering::Equal)
                                });

                            if let Some(spill) = spill_candidate {
                                if path.len() < 1000 {
                                    path.push(spill);
                                    current = spill;
                                    continue;
                                }
                            }
                            break RiverTerminus::Sink;
                        }
                    }
                    Some(next) => {
                        if path.len() >= 1000 {
                            break RiverTerminus::Sink;
                        }
                        path.push(next);
                        current = next;
                    }
                }
            };

            let mouth = *path.last().unwrap();
            let mouth_flow = corner_flow.get(mouth).copied().unwrap_or(0.0);

            if path.len() >= min_length && mouth_flow >= min_flow {
                for &ci in &path {
                    corner_in_river[ci] = true;
                }
                if let Some(lc) = sink_lake_corner {
                    lake_sink_corners.push(lc);
                }
                rivers.push(RiverPath {
                    corners: path,
                    mouth_flow,
                    terminus,
                });
            }
        }

        // --- Step 9: Build cell_has_river and cell_river_flow ---
        let mut cell_has_river = vec![false; n_cells];
        let mut cell_river_flow = vec![0.0_f32; n_cells];

        for river in &rivers {
            for &ci in &river.corners {
                for &cid in &corners[ci].adjacent_cell_ids {
                    if let Some(&cell_idx) = cell_id_to_idx.get(&cid) {
                        cell_has_river[cell_idx] = true;
                        let f = corner_flow.get(ci).copied().unwrap_or(0.0);
                        if f > cell_river_flow[cell_idx] {
                            cell_river_flow[cell_idx] = f;
                        }
                    }
                }
            }
        }

        // --- Step 10: Build lake_cells from sink depressions ---
        let mut lake_cells = vec![false; n_cells];
        for lc in &lake_sink_corners {
            for &cid in &corners[*lc].adjacent_cell_ids {
                if let Some(&cell_idx) = cell_id_to_idx.get(&cid) {
                    if cells[cell_idx].is_land && !cells[cell_idx].is_border {
                        lake_cells[cell_idx] = true;
                    }
                }
            }
        }

        Box::new(RiverOutput {
            rivers,
            corner_flow,
            cell_has_river,
            cell_river_flow,
            lake_cells,
        })
    }
}

// ---------------------------------------------------------------------------
// Flow accumulation
// ---------------------------------------------------------------------------

fn accumulate_flow(
    n_corners: usize,
    corner_elev: &[f32],
    corner_rainfall: &[f32],
    corner_is_land: &[bool],
    corners: &[Corner],
) -> (Vec<f32>, Vec<Option<usize>>) {
    // Compute flow direction: steepest downhill adjacent land corner
    let mut flow_dir: Vec<Option<usize>> = vec![None; n_corners];
    for ci in 0..n_corners {
        if !corner_is_land[ci] { continue; }

        let mut best_elev = corner_elev[ci];
        let mut best_adj: Option<usize> = None;

        for &adj in &corners[ci].adjacent_corner_ids {
            if corner_is_land[adj] && corner_elev[adj] < best_elev {
                best_elev = corner_elev[adj];
                best_adj = Some(adj);
            }
        }
        flow_dir[ci] = best_adj;
    }

    // Topological sort: corners by elevation descending
    let mut order: Vec<usize> = (0..n_corners).filter(|&ci| corner_is_land[ci]).collect();
    order.sort_by(|&a, &b| {
        corner_elev[b].partial_cmp(&corner_elev[a]).unwrap_or(std::cmp::Ordering::Equal)
    });

    // Seed flow from rainfall
    let mut corner_flow: Vec<f32> = corner_rainfall.to_vec();
    // Zero out non-land corners
    for ci in 0..n_corners {
        if !corner_is_land[ci] {
            corner_flow[ci] = 0.0;
        }
    }

    // Propagate high → low
    for &ci in &order {
        if let Some(target) = flow_dir[ci] {
            corner_flow[target] += corner_flow[ci];
        }
    }

    (corner_flow, flow_dir)
}
