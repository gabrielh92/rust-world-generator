use std::collections::HashMap;

use rand::SeedableRng;
use rand_pcg::Pcg64;

use crate::canvas::CanvasData;
use crate::mathlib::vec::Vec2;
use crate::params::util::build_default_moisture_params;
use crate::pipeline::terrain::{TerrainOutput, STAGE_DATA_KEY as TERRAIN_KEY};
use crate::pipeline::{PipelineStage, PipelineStageExecutor, StageData, StageDataMap};
use crate::visualization::moisture::MoistureVisualLayer;

pub const STAGE_DATA_KEY: &str = "stage:moisture";

pub fn make_moisture_stage() -> PipelineStage {
    PipelineStage {
        executor: Box::new(MoistureStage),
        params: Some(build_default_moisture_params()),
        visual_layer: Box::new(MoistureVisualLayer::default()),
    }
}

// ---------------------------------------------------------------------------
// Output
// ---------------------------------------------------------------------------

/// Moisture for each cell, produced by Stage 5.
/// Indexed parallel to LandmassOutput::cells. Range: [0, 1].
#[derive(Debug, Clone)]
pub struct MoistureOutput {
    pub cell_moisture: Vec<f32>,
}

impl StageData for MoistureOutput {
    fn as_any(&self) -> &dyn std::any::Any { self }
}

// ---------------------------------------------------------------------------
// Stage executor
// ---------------------------------------------------------------------------

pub struct MoistureStage;

impl PipelineStageExecutor for MoistureStage {
    fn name(&self) -> &str { "moisture" }
    fn rank(&self) -> u8 { 5 }
    fn data_key(&self) -> &'static str { STAGE_DATA_KEY }

    fn run(&mut self, params: Option<&crate::params::ParamGroup>, data: &StageDataMap) -> Box<dyn StageData> {
        let pg = params.expect("Moisture params required");

        let wind_pattern    = pg.get_param("Wind Pattern").map(|p| p.value.as_str()).unwrap_or("Earth-like");
        let wind_angle_deg  = pg.get_param("Wind Angle").and_then(|p| p.as_float()).unwrap_or(270.0);
        let decay           = pg.get_param("Moisture Decay").and_then(|p| p.as_float()).unwrap_or(0.88);
        let orog_strength   = pg.get_param("Orographic Strength").and_then(|p| p.as_float()).unwrap_or(0.75);
        let mountain_thresh = pg.get_param("Mountain Threshold").and_then(|p| p.as_float()).unwrap_or(0.55);

        let canvas = data.get("canvas")
            .and_then(|d| d.as_any().downcast_ref::<CanvasData>())
            .expect("Canvas data required for moisture stage");

        let terrain = data.get(TERRAIN_KEY)
            .and_then(|d| d.as_any().downcast_ref::<TerrainOutput>())
            .expect("Terrain stage must run before Moisture stage");

        // Seed derived deterministically (unused directly but keeps the contract clear)
        let _rng = Pcg64::seed_from_u64(canvas.seed ^ 0x4D4F_4953_5455u64); // "MOISTU"

        let cells = &terrain.cells;
        let n = cells.len();

        // Scale iterations to mesh size so deep-interior cells are always reached.
        // With n=300: ~17 passes. With n=1000: ~32 passes. With n=5000: ~71 passes.
        let iterations = ((n as f32).sqrt() as usize).max(20);

        let id_to_idx: HashMap<usize, usize> =
            cells.iter().enumerate().map(|(i, c)| (c.id, i)).collect();

        // --- Step 1: Per-cell wind vectors ---
        let wind_vectors: Vec<Vec2> = cells.iter().map(|cell| {
            match wind_pattern {
                "Uniform" => {
                    let rad = wind_angle_deg.to_radians();
                    Vec2::new(rad.cos(), rad.sin())
                }
                "None" => Vec2::new(0.0, 0.0),
                _ => earth_like_wind(cell.center.y, canvas.height),
            }
        }).collect();

        // --- Step 2: Initialise ---
        // Ocean/coast-adjacent cells are moisture sources (1.0). Land starts at 0.
        let mut moisture: Vec<f32> = cells.iter().map(|c| {
            if !c.is_land { 1.0 } else { 0.0 }
        }).collect();

        // --- Step 3: Iterative directional propagation ---
        // Each pass advances moisture one graph-hop downwind.
        // Iterations scale with sqrt(n) so all interior cells are reachable
        // regardless of mesh density.
        for _ in 0..iterations {
            let prev = moisture.clone();
            for i in 0..n {
                let cell = &cells[i];
                if !cell.is_land || cell.is_border { continue; }

                let wind = wind_vectors[i];

                if wind.x == 0.0 && wind.y == 0.0 {
                    // No-wind mode: take max from any neighbour
                    let best = cell.neighbor_ids.iter()
                        .filter_map(|&nid| id_to_idx.get(&nid).map(|&ni| prev[ni]))
                        .fold(0.0_f32, f32::max);
                    moisture[i] = moisture[i].max(best * decay);
                    continue;
                }

                // Upwind neighbours: those where the vector neighbor→cell aligns with wind
                let mut best: f32 = 0.0;
                for &nid in &cell.neighbor_ids {
                    let Some(&ni) = id_to_idx.get(&nid) else { continue };
                    let neighbor = &cells[ni];

                    // Positive dot product means neighbor is upwind of current cell
                    let to_cell = cell.center - neighbor.center;
                    if to_cell.dot(wind) <= 0.0 { continue; }

                    let incoming = prev[ni];

                    // Orographic shadow: mountains on the upwind side block moisture.
                    // The higher above threshold, the stronger the block.
                    let neighbor_elev = neighbor.elevation;
                    let shadow = if neighbor_elev > mountain_thresh {
                        let excess = ((neighbor_elev - mountain_thresh) / (1.0 - mountain_thresh)).min(1.0);
                        1.0 - orog_strength * excess
                    } else {
                        1.0
                    };

                    best = best.max(incoming * decay * shadow);
                }
                moisture[i] = moisture[i].max(best);
            }
        }

        // --- Step 4: Smooth to kill isolated zero-moisture speckle ---
        // Cells with poor graph connectivity may never receive propagated moisture
        // and stay at 0. Two neighbour-averaging passes blend them out without
        // flattening the overall gradient.
        for _ in 0..2 {
            let prev = moisture.clone();
            for i in 0..n {
                if !cells[i].is_land || cells[i].is_border { continue; }
                let neighbors: Vec<f32> = cells[i].neighbor_ids.iter()
                    .filter_map(|&nid| id_to_idx.get(&nid).map(|&ni| prev[ni]))
                    .collect();
                if neighbors.is_empty() { continue; }
                let avg = neighbors.iter().sum::<f32>() / neighbors.len() as f32;
                moisture[i] = prev[i] * 0.7 + avg * 0.3;
            }
        }

        // --- Step 5: Power-curve compression then restore ocean/border ---
        // Apply a mild exponent > 1 to the land moisture values. This compresses the
        // high-humidity cluster (most cells end up near the coast and score 0.5–0.8)
        // and spreads the lower end, producing visible arid/desert zones without
        // requiring continent sizes larger than the canvas.
        // Exponent 1.5: 0.8 → 0.72, 0.5 → 0.35, 0.3 → 0.16, 0.15 → 0.058.
        for i in 0..n {
            if cells[i].is_border     { moisture[i] = 0.0; }
            else if !cells[i].is_land { moisture[i] = 1.0; }
            else                      { moisture[i] = moisture[i].clamp(0.0, 1.0).powf(1.5); }
        }

        Box::new(MoistureOutput { cell_moisture: moisture })
    }
}

// ---------------------------------------------------------------------------
// Wind model
// ---------------------------------------------------------------------------

/// Earth-like three-band prevailing wind.
/// Returns the direction air is *travelling* (not where it comes from).
/// y=0 = north pole, y=height = south pole.
fn earth_like_wind(y: f32, height: f32) -> Vec2 {
    let lat = y / height; // 0 (north) → 1 (south)
    if lat < 0.15 || lat > 0.85 {
        // Polar easterlies — blow westward
        Vec2::new(-1.0, 0.0)
    } else if (0.15..0.4).contains(&lat) || (0.6..0.85).contains(&lat) {
        // Mid-latitude westerlies — blow eastward
        Vec2::new(1.0, 0.0)
    } else {
        // Tropical trade winds — blow westward and toward equator
        let toward_eq = if lat < 0.5 { 1.0_f32 } else { -1.0 };
        Vec2::new(-0.7, toward_eq * 0.7).normalized()
    }
}
