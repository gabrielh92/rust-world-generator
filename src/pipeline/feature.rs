use std::collections::{HashMap, HashSet};

use noise::{NoiseFn, Perlin};

use crate::canvas::CanvasData;
use crate::params::util::build_default_feature_params;
use crate::pipeline::terrain::{TerrainOutput, STAGE_DATA_KEY as TERRAIN_KEY};
use crate::pipeline::moisture::{MoistureOutput, STAGE_DATA_KEY as MOISTURE_KEY};
use crate::pipeline::river::{RiverOutput, RiverTerminus, STAGE_DATA_KEY as RIVER_KEY};
use crate::pipeline::{PipelineStage, PipelineStageExecutor, StageData, StageDataMap};
use crate::visualization::feature::FeatureVisualLayer;

pub const STAGE_DATA_KEY: &str = "stage:feature";

pub fn make_feature_stage() -> PipelineStage {
    PipelineStage {
        executor: Box::new(FeatureStage),
        params: Some(build_default_feature_params()),
        visual_layer: Box::new(FeatureVisualLayer::default()),
    }
}

// ---------------------------------------------------------------------------
// Feature enum
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Feature {
    /// River path terminates at a coast cell.
    RiverMouth,
    /// Coast cell sheltered by land on multiple sides.
    HarborCandidate,
    /// Low-elevation cell flanked by high-elevation neighbors.
    MountainPass,
    /// Low elevation + high moisture + river-adjacent.
    FertileValley,
    /// Noise-weighted placeholder for future resource system.
    ResourceNode,
}

// ---------------------------------------------------------------------------
// Output
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct FeatureOutput {
    /// One set per cell, same order as landmass.cells.
    pub cell_features: Vec<HashSet<Feature>>,
}

impl StageData for FeatureOutput {
    fn as_any(&self) -> &dyn std::any::Any { self }
}

// ---------------------------------------------------------------------------
// Stage executor
// ---------------------------------------------------------------------------

pub struct FeatureStage;

impl PipelineStageExecutor for FeatureStage {
    fn name(&self) -> &str { "feature" }
    fn rank(&self) -> u8 { 8 }
    fn data_key(&self) -> &'static str { STAGE_DATA_KEY }

    fn run(&mut self, params: Option<&crate::params::ParamGroup>, data: &StageDataMap) -> Box<dyn StageData> {
        let pg = params.expect("Feature params required");

        // Read detection params
        let pass_max_elevation = pg.get_param("Pass Max Elevation")
            .and_then(|p| p.as_float()).unwrap_or(0.45);
        let pass_min_neighbor_elevation = pg.get_param("Pass Min Neighbor Elevation")
            .and_then(|p| p.as_float()).unwrap_or(0.60);
        let pass_min_mountain_neighbors = pg.get_param("Pass Min Mountain Neighbors")
            .and_then(|p| p.as_int()).unwrap_or(2) as usize;
        let fertile_max_elevation = pg.get_param("Fertile Max Elevation")
            .and_then(|p| p.as_float()).unwrap_or(0.30);
        let fertile_min_moisture = pg.get_param("Fertile Min Moisture")
            .and_then(|p| p.as_float()).unwrap_or(0.55);

        // Read required data
        let landmass = data.get(TERRAIN_KEY)
            .and_then(|d| d.as_any().downcast_ref::<TerrainOutput>())
            .expect("Terrain stage must run before Feature stage");

        let moisture_out = data.get(MOISTURE_KEY)
            .and_then(|d| d.as_any().downcast_ref::<MoistureOutput>());

        let river_out = data.get(RIVER_KEY)
            .and_then(|d| d.as_any().downcast_ref::<RiverOutput>());

        let canvas = data.get("canvas")
            .and_then(|d| d.as_any().downcast_ref::<CanvasData>());

        let cells = &landmass.cells;
        let corners = &landmass.corners;
        let n_cells = cells.len();

        // Build cell id → array index map
        let cell_id_to_idx: HashMap<usize, usize> =
            cells.iter().enumerate().map(|(i, c)| (c.id, i)).collect();

        let mut cell_features: Vec<HashSet<Feature>> = vec![HashSet::new(); n_cells];

        // -------------------------------------------------------------------
        // RiverMouth: river path with Ocean terminus → tag coast cells adjacent
        // to the last corner in the path.
        // -------------------------------------------------------------------
        if let Some(river) = river_out {
            for path in &river.rivers {
                if path.terminus != RiverTerminus::Ocean { continue; }
                let last_corner_idx = match path.corners.last() {
                    Some(&ci) => ci,
                    None => continue,
                };
                for &cid in &corners[last_corner_idx].adjacent_cell_ids {
                    if let Some(&cell_idx) = cell_id_to_idx.get(&cid) {
                        let cell = &cells[cell_idx];
                        if cell.is_coast && cell.is_land {
                            cell_features[cell_idx].insert(Feature::RiverMouth);
                        }
                    }
                }
            }
        }

        // -------------------------------------------------------------------
        // HarborCandidate: coast land cells sheltered by land on multiple sides.
        // Condition: outer_ocean_neighbors <= 2 AND non_coast_land_neighbors >= 2
        // -------------------------------------------------------------------
        for (i, cell) in cells.iter().enumerate() {
            if !cell.is_land || !cell.is_coast { continue; }

            let mut outer_ocean_neighbors = 0usize;
            let mut non_coast_land_neighbors = 0usize;

            for &nid in &cell.neighbor_ids {
                if let Some(&ni) = cell_id_to_idx.get(&nid) {
                    let neighbor = &cells[ni];
                    // outer ocean: not land, not lake, not border
                    if !neighbor.is_land && !neighbor.is_lake && !neighbor.is_border {
                        outer_ocean_neighbors += 1;
                    }
                    // non-coast land
                    if neighbor.is_land && !neighbor.is_coast {
                        non_coast_land_neighbors += 1;
                    }
                }
            }

            if outer_ocean_neighbors <= 2 && non_coast_land_neighbors >= 2 {
                cell_features[i].insert(Feature::HarborCandidate);
            }
        }

        // -------------------------------------------------------------------
        // MountainPass: non-coast, non-border land cell below pass_max_elevation
        // with enough high-elevation neighbors.
        // -------------------------------------------------------------------
        for (i, cell) in cells.iter().enumerate() {
            if !cell.is_land || cell.is_coast || cell.is_border { continue; }

            let cell_elev = cell.elevation;
            if cell_elev >= pass_max_elevation { continue; }

            let mut mountain_neighbor_count = 0usize;
            for &nid in &cell.neighbor_ids {
                if let Some(&ni) = cell_id_to_idx.get(&nid) {
                    let neighbor_elev = cells[ni].elevation;
                    if neighbor_elev > pass_min_neighbor_elevation {
                        mountain_neighbor_count += 1;
                    }
                }
            }

            if mountain_neighbor_count >= pass_min_mountain_neighbors {
                cell_features[i].insert(Feature::MountainPass);
            }
        }

        // -------------------------------------------------------------------
        // FertileValley: non-coast, non-border land cell with low elevation,
        // high moisture, and river adjacency.
        // -------------------------------------------------------------------
        if let Some(moisture) = moisture_out {
            if let Some(river) = river_out {
                for (i, cell) in cells.iter().enumerate() {
                    if !cell.is_land || cell.is_coast || cell.is_border { continue; }

                    let cell_elev = cell.elevation;
                    let cell_moisture = moisture.cell_moisture.get(i).copied().unwrap_or(0.0);
                    let has_river = river.cell_has_river.get(i).copied().unwrap_or(false);

                    if cell_elev < fertile_max_elevation
                        && cell_moisture > fertile_min_moisture
                        && has_river
                    {
                        cell_features[i].insert(Feature::FertileValley);
                    }
                }
            }
        }

        // -------------------------------------------------------------------
        // ResourceNode: noise-distributed placeholder (~25-30% of land cells).
        // -------------------------------------------------------------------
        let noise_seed = canvas.map(|c| c.seed.wrapping_add(3) as u32).unwrap_or(3);
        let perlin = Perlin::new(noise_seed);

        for (i, cell) in cells.iter().enumerate() {
            if !cell.is_land || cell.is_border { continue; }

            let nx = cell.center.x as f64 * 0.008;
            let ny = cell.center.y as f64 * 0.008;
            let noise_val = perlin.get([nx, ny]) as f32;

            if noise_val > 0.45 {
                cell_features[i].insert(Feature::ResourceNode);
            }
        }

        Box::new(FeatureOutput { cell_features })
    }
}
