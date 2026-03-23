use crate::params::util::build_default_biome_params;
use crate::pipeline::elevation::{ElevationOutput, STAGE_DATA_KEY as ELEVATION_KEY};
use crate::pipeline::landmass::{LandmassOutput, STAGE_DATA_KEY as LANDMASS_KEY};
use crate::pipeline::moisture::{MoistureOutput, STAGE_DATA_KEY as MOISTURE_KEY};
use crate::pipeline::river::{RiverOutput, STAGE_DATA_KEY as RIVER_KEY};
use crate::pipeline::{PipelineStage, PipelineStageExecutor, StageData, StageDataMap};
use crate::visualization::biome::BiomeVisualLayer;

pub const STAGE_DATA_KEY: &str = "stage:biome";

pub fn make_biome_stage() -> PipelineStage {
    PipelineStage {
        executor: Box::new(BiomeStage),
        params: Some(build_default_biome_params()),
        visual_layer: Box::new(BiomeVisualLayer::default()),
    }
}

// ---------------------------------------------------------------------------
// Vegetation structure enum
// ---------------------------------------------------------------------------

/// Tier-1 classification: local vegetation structure derived from elevation,
/// moisture, and water systems — without global climate context.
/// Named biomes (Tropical Rainforest, Tundra, etc.) are assigned post-stitch
/// once latitude and continental position are known.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Biome {
    // Water
    DeepOcean,
    ShallowOcean,
    Lake,
    // Coastal land
    Beach,
    Wetland,
    // Land vegetation structures
    Alpine,
    Sparse,
    Open,
    Dense,
    Riparian,
    // System
    Border,
}

impl Biome {
    pub fn label(self) -> &'static str {
        match self {
            Biome::DeepOcean   => "Deep Ocean",
            Biome::ShallowOcean => "Shallow Ocean",
            Biome::Lake        => "Lake",
            Biome::Beach       => "Beach",
            Biome::Wetland     => "Wetland",
            Biome::Alpine      => "Alpine",
            Biome::Sparse      => "Sparse",
            Biome::Open        => "Open",
            Biome::Dense       => "Dense",
            Biome::Riparian    => "Riparian",
            Biome::Border      => "Border",
        }
    }
}

// ---------------------------------------------------------------------------
// Output
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct BiomeOutput {
    /// One biome per cell, same order as LandmassOutput::cells.
    pub cell_biomes: Vec<Biome>,
    /// Fraction of non-border cells covered by each biome (0.0–1.0).
    pub biome_fractions: std::collections::HashMap<Biome, f32>,
}

impl StageData for BiomeOutput {
    fn as_any(&self) -> &dyn std::any::Any { self }
}

// ---------------------------------------------------------------------------
// Stage executor
// ---------------------------------------------------------------------------

pub struct BiomeStage;

impl PipelineStageExecutor for BiomeStage {
    fn name(&self) -> &str { "biome" }
    fn rank(&self) -> u8 { 7 }
    fn data_key(&self) -> &'static str { STAGE_DATA_KEY }

    fn run(&mut self, params: Option<&crate::params::ParamGroup>, data: &StageDataMap) -> Box<dyn StageData> {
        let smoothing_passes = params
            .and_then(|p| p.get_param("Smoothing Passes"))
            .and_then(|p| p.as_int())
            .unwrap_or(2);

        let landmass = data.get(LANDMASS_KEY)
            .and_then(|d| d.as_any().downcast_ref::<LandmassOutput>())
            .expect("Landmass stage must run before Biome stage");

        let elevation_out = data.get(ELEVATION_KEY)
            .and_then(|d| d.as_any().downcast_ref::<ElevationOutput>());

        let moisture_out = data.get(MOISTURE_KEY)
            .and_then(|d| d.as_any().downcast_ref::<MoistureOutput>());

        let river_out = data.get(RIVER_KEY)
            .and_then(|d| d.as_any().downcast_ref::<RiverOutput>());

        let cells = &landmass.cells;
        let n = cells.len();

        // Build cell id → index map for neighbor lookups during smoothing
        let id_to_idx: std::collections::HashMap<usize, usize> =
            cells.iter().enumerate().map(|(i, c)| (c.id, i)).collect();

        // Compute alpine elevation threshold: 85th percentile of land elevations,
        // with a floor of 0.60 so flat continents never get alpine.
        let alpine_threshold = {
            let mut land_elevs: Vec<f32> = cells.iter().enumerate()
                .filter(|(_, c)| c.is_land && !c.is_border)
                .map(|(i, _)| {
                    elevation_out
                        .and_then(|e| e.cell_elevations.get(i).copied())
                        .unwrap_or(0.5)
                })
                .collect();
            land_elevs.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
            let n_land = land_elevs.len();
            if n_land < 2 {
                0.70_f32
            } else {
                land_elevs[(n_land as f32 * 0.85) as usize].max(0.60)
            }
        };

        // --- Initial per-cell classification ---
        let mut cell_biomes: Vec<Biome> = cells.iter().enumerate().map(|(i, cell)| {
            if cell.is_border { return Biome::Border; }

            let elev = elevation_out
                .and_then(|e| e.cell_elevations.get(i).copied())
                .unwrap_or(cell.elevation);

            let moist = moisture_out
                .and_then(|m| m.cell_moisture.get(i).copied())
                .unwrap_or(0.5);

            // Water: landmass lakes + river sink-depression lakes
            let is_lake = cell.is_lake
                || river_out.and_then(|r| r.lake_cells.get(i).copied()).unwrap_or(false);
            if is_lake { return Biome::Lake; }

            // Ocean
            if !cell.is_land {
                return if elev < -0.4 { Biome::DeepOcean } else { Biome::ShallowOcean };
            }

            // Alpine (checked before river adjacency — alpine rivers stay Alpine)
            if elev >= alpine_threshold { return Biome::Alpine; }

            // Coastal land (beach / wetland)
            if cell.is_coast {
                return if moist > 0.50 { Biome::Wetland } else { Biome::Beach };
            }

            // River corridor
            let river_adj = river_out
                .and_then(|r| r.cell_has_river.get(i).copied())
                .unwrap_or(false);
            if river_adj { return Biome::Riparian; }

            // Interior land — vegetation density by moisture
            if moist > 0.55      { Biome::Dense }
            else if moist > 0.28 { Biome::Open  }
            else                 { Biome::Sparse }
        }).collect();

        // --- Majority-vote spatial smoothing ---
        // Only applied to land vegetation structures (not water or coast tiles).
        for _ in 0..smoothing_passes {
            let prev = cell_biomes.clone();
            for i in 0..n {
                if !is_smoothable(prev[i]) { continue; }

                let mut counts: std::collections::HashMap<Biome, usize> = std::collections::HashMap::new();
                for &nid in &cells[i].neighbor_ids {
                    if let Some(&ni) = id_to_idx.get(&nid) {
                        if is_smoothable(prev[ni]) {
                            *counts.entry(prev[ni]).or_insert(0) += 1;
                        }
                    }
                }
                if let Some((&majority, &maj_count)) = counts.iter().max_by_key(|(_, &c)| c) {
                    let neighbor_count = counts.values().sum::<usize>();
                    // Override if majority holds strictly more than half of smoothable neighbors
                    if maj_count * 2 > neighbor_count && majority != prev[i] {
                        cell_biomes[i] = majority;
                    }
                }
            }
        }

        // --- Biome fraction counts ---
        let total = cell_biomes.iter().filter(|&&b| b != Biome::Border).count() as f32;
        let mut biome_fractions = std::collections::HashMap::new();
        if total > 0.0 {
            for &b in &cell_biomes {
                if b == Biome::Border { continue; }
                *biome_fractions.entry(b).or_insert(0.0) += 1.0 / total;
            }
        }

        Box::new(BiomeOutput { cell_biomes, biome_fractions })
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Returns true for biome variants that should participate in smoothing.
/// Water and coast tiles are topologically determined — don't smooth them.
fn is_smoothable(b: Biome) -> bool {
    matches!(b, Biome::Alpine | Biome::Sparse | Biome::Open | Biome::Dense | Biome::Riparian)
}
