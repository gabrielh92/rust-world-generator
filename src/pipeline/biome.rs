use crate::params::util::build_default_biome_params;
use crate::pipeline::elevation::{ElevationOutput, STAGE_DATA_KEY as ELEVATION_KEY};
use crate::pipeline::landmass::{LandmassOutput, STAGE_DATA_KEY as LANDMASS_KEY};
use crate::pipeline::moisture::{MoistureOutput, STAGE_DATA_KEY as MOISTURE_KEY};
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
// Biome enum
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Biome {
    // Ocean (by depth)
    DeepTrench,
    OpenOcean,
    ShallowCoast,
    // Inland water
    Lake,
    // Special land
    Border,
    Beach,
    Wetland,
    Mangrove,
    // High elevation
    AlpineTundra,
    AlpineMeadow,
    AlpineForest,
    // Mid elevation
    Shrubland,
    TemperateForest,
    Rainforest,
    // Low elevation
    Desert,
    GrasslandSavanna,
    TropicalRainforest,
}

impl Biome {
    pub fn label(self) -> &'static str {
        match self {
            Biome::DeepTrench        => "Deep Trench",
            Biome::OpenOcean         => "Open Ocean",
            Biome::ShallowCoast      => "Shallow Coast",
            Biome::Lake              => "Lake",
            Biome::Border            => "Border",
            Biome::Beach             => "Beach",
            Biome::Wetland           => "Wetland",
            Biome::Mangrove          => "Mangrove",
            Biome::AlpineTundra      => "Alpine Tundra",
            Biome::AlpineMeadow      => "Alpine Meadow",
            Biome::AlpineForest      => "Alpine Forest",
            Biome::Shrubland         => "Shrubland",
            Biome::TemperateForest   => "Temperate Forest",
            Biome::Rainforest        => "Rainforest",
            Biome::Desert            => "Desert",
            Biome::GrasslandSavanna  => "Grassland / Savanna",
            Biome::TropicalRainforest => "Tropical Rainforest",
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
    fn rank(&self) -> u8 { 6 }
    fn data_key(&self) -> &'static str { STAGE_DATA_KEY }

    fn run(&mut self, _params: Option<&crate::params::ParamGroup>, data: &StageDataMap) -> Box<dyn StageData> {
        let landmass = data.get(LANDMASS_KEY)
            .and_then(|d| d.as_any().downcast_ref::<LandmassOutput>())
            .expect("Landmass stage must run before Biome stage");

        let elevation_out = data.get(ELEVATION_KEY)
            .and_then(|d| d.as_any().downcast_ref::<ElevationOutput>());

        let moisture_out = data.get(MOISTURE_KEY)
            .and_then(|d| d.as_any().downcast_ref::<MoistureOutput>());

        // Compute elevation percentiles across land cells to derive adaptive thresholds.
        // This ensures alpine biomes only dominate if the terrain is genuinely high,
        // and low/mid bands reflect the actual distribution of the map.
        // We target: bottom 40% → low, 40-82% → mid, top 18% → high.
        // An absolute floor of 0.55 for alpine means flat continents have no tundra.
        let (elev_lo_thresh, elev_hi_thresh) = {
            let mut land_elevs: Vec<f32> = landmass.cells.iter().enumerate()
                .filter(|(_, c)| c.is_land && !c.is_border)
                .map(|(i, c)| elevation_out.and_then(|e| e.cell_elevations.get(i).copied()).unwrap_or(c.elevation))
                .collect();
            land_elevs.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
            let n = land_elevs.len();
            if n < 2 {
                (0.35_f32, 0.65_f32)
            } else {
                let lo = land_elevs[(n as f32 * 0.40) as usize];
                let hi = land_elevs[(n as f32 * 0.82) as usize].max(0.55);
                (lo, hi)
            }
        };

        let cell_biomes: Vec<Biome> = landmass.cells.iter().enumerate().map(|(i, cell)| {
            if cell.is_border { return Biome::Border; }

            let elev = elevation_out
                .and_then(|e| e.cell_elevations.get(i).copied())
                .unwrap_or(cell.elevation);

            let moist = moisture_out
                .and_then(|m| m.cell_moisture.get(i).copied())
                .unwrap_or(0.5);

            if cell.is_lake { return Biome::Lake; }

            if !cell.is_land {
                return classify_ocean(elev);
            }

            if cell.is_coast {
                return classify_coast(moist);
            }

            classify_land(elev, moist, elev_lo_thresh, elev_hi_thresh)
        }).collect();

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
// Classification helpers — Whittaker diagram
// ---------------------------------------------------------------------------

fn classify_ocean(elev: f32) -> Biome {
    if elev < -0.6      { Biome::DeepTrench   }
    else if elev < -0.2 { Biome::OpenOcean    }
    else                { Biome::ShallowCoast }
}

fn classify_coast(moisture: f32) -> Biome {
    if moisture < 0.35      { Biome::Beach   }
    else if moisture < 0.65 { Biome::Wetland }
    else                    { Biome::Mangrove }
}

fn classify_land(elev: f32, moisture: f32, elev_lo: f32, elev_hi: f32) -> Biome {
    // Elevation bands derived from map-wide land percentiles (adaptive thresholds).
    // Moisture bands are fixed: Low < 0.33, Mid 0.33–0.66, High > 0.66
    let elev_band = if elev > elev_hi { 2 } else if elev > elev_lo { 1 } else { 0 };
    let moist_band = if moisture > 0.66 { 2 } else if moisture > 0.33 { 1 } else { 0 };

    match (elev_band, moist_band) {
        // High elevation
        (2, 0) => Biome::AlpineTundra,
        (2, 1) => Biome::AlpineMeadow,
        (2, 2) => Biome::AlpineForest,
        // Mid elevation
        (1, 0) => Biome::Shrubland,
        (1, 1) => Biome::TemperateForest,
        (1, 2) => Biome::Rainforest,
        // Low elevation
        (0, 0) => Biome::Desert,
        (0, 1) => Biome::GrasslandSavanna,
        (0, 2) => Biome::TropicalRainforest,
        _      => Biome::GrasslandSavanna, // unreachable
    }
}
