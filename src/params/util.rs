use crate::params::builder::ParamGroupBuilder;
use crate::params::ParamGroup;

pub fn build_default_biome_params() -> ParamGroup {
    ParamGroupBuilder::new("Biome")
        .int_param("Smoothing Passes", 2, 0, 6, 1)
        .with_tooltip("Majority-vote smoothing passes after classification — higher values merge microclimates into larger blobs")
        .build()
}

pub fn build_default_moisture_params() -> ParamGroup {
    ParamGroupBuilder::new("Moisture & Wind")
        .enum_param(
            "Wind Pattern",
            vec!["Earth-like".into(), "Uniform".into(), "None".into()],
            0,
        )
        .with_tooltip(
            "Earth-like: 3-band model (polar easterlies, westerlies, trade winds). \
             Uniform: single direction angle. None: moisture propagates uniformly from all coasts.",
        )
        .with_section("Wind")
        .float_param("Wind Angle", 270.0, 0.0, 360.0, 5.0)
        .with_tooltip("[Uniform] Travel direction in degrees (0=east, 90=south, 180=west, 270=north)")
        .with_section("Propagation")
        .float_param("Moisture Decay", 0.72, 0.5, 1.0, 0.01)
        .with_tooltip("Fraction of moisture retained per graph-hop inland (lower = drier interiors)")
        .float_param("Orographic Strength", 0.75, 0.0, 1.0, 0.05)
        .with_tooltip("How strongly mountain ridges block moisture on their leeward side")
        .float_param("Mountain Threshold", 0.55, 0.3, 0.9, 0.05)
        .with_tooltip("Cell elevation above which orographic shadow kicks in")
        .with_section("Display")
        .enum_param(
            "View Mode",
            vec!["Moisture".into(), "Land Type".into()],
            0,
        )
        .with_tooltip("Moisture: continuous arid→wet gradient. Land Type: discrete zone colours.")
        .visual_only()
        .build()
}

pub fn build_default_point_params() -> ParamGroup {
    ParamGroupBuilder::new("Point Sampling")
        .enum_param(
            "Distribution",
            vec!["Poisson Disc".into(), "Random Uniform".into(), "Uniform Grid".into()],
            0,
        )
        .with_tooltip("Point distribution algorithm. Poisson Disc gives the most natural-looking mesh.")
        .with_section("Sampling")
        .int_param("Point Count", 500, 10, 5000, 10)
        .with_tooltip("Number of points (used by Random Uniform and Uniform Grid)")
        .float_param("Min Distance", 15.0, 5.0, 120.0, 1.0)
        .with_tooltip("Minimum spacing between points (used by Poisson Disc — controls point density)")
        .with_section("Display")
        .float_param("Point Radius", 2.0, 0.5, 8.0, 0.5)
        .with_tooltip("Visual display radius of each point (rendering only)")
        .visual_only()
        .build()
}

pub fn build_default_voronoi_params() -> ParamGroup {
    ParamGroupBuilder::new("Voronoi Graph")
        .int_param("Lloyd Relaxations", 2, 0, 6, 1)
        .with_tooltip("Iterations of Lloyd relaxation to regularise the mesh (2–3 recommended)")
        .with_section("Display")
        .bool_param("Show Cell Labels", false)
        .with_tooltip("Render cell index at each centroid (disable for large meshes)")
        .visual_only()
        .bool_param("Show Delaunay", true)
        .with_tooltip("Render Delaunay triangulation edges")
        .visual_only()
        .build()
}

pub fn build_default_elevation_params() -> ParamGroup {
    ParamGroupBuilder::new("Elevation")
        // Ridge injection
        .enum_param(
            "Ridge Mode",
            vec!["Follow Terrain".into(), "Random".into(), "None".into()],
            0,
        )
        .with_tooltip(
            "Follow Terrain: anchors ridges near Stage 3 elevation maxima. \
             Random: picks anchors anywhere on land.",
        )
        .with_section("Ridge Settings")
        .int_param("Num Ridges", 4, 0, 12, 1)
        .with_tooltip("Number of mountain ridge lines to inject")
        .float_param("Ridge Intensity", 0.5, 0.0, 1.0, 0.02)
        .with_tooltip("Peak height added at the ridge centre")
        .float_param("Ridge Width", 0.12, 0.02, 0.5, 0.01)
        .with_tooltip("Falloff width of each ridge (fraction of canvas diagonal)")
        // Redistribution
        .with_section("Redistribution")
        .enum_param(
            "Distribution",
            vec!["Natural".into(), "Flat".into(), "Mountainous".into()],
            0,
        )
        .with_tooltip(
            "Target elevation distribution after ridge injection. \
             Natural ≈ Earth-like; Flat = mostly lowlands; Mountainous = high terrain.",
        )
        .float_param("Redistribution Strength", 0.6, 0.0, 1.0, 0.02)
        .with_tooltip(
            "How strongly to remap elevation to the target distribution (0 = off, 1 = full)",
        )
        // Stage 3 blend
        .with_section("Blending")
        .float_param("Stage 3 Blend", 0.5, 0.0, 1.0, 0.02)
        .with_tooltip(
            "Weight given to Stage 3 rough elevation as a starting point. \
             0 = ignore Stage 3 entirely and recompute from scratch.",
        )
        // Display
        .with_section("Display")
        .float_param("Water Level", 0.0, -1.0, 1.0, 0.01)
        .with_tooltip("Rendering threshold — cells below render as water")
        .visual_only()
        .enum_param(
            "View Mode",
            vec!["Elevation".into(), "Land Type".into()],
            0,
        )
        .with_tooltip("Elevation: gradient by height.  Land Type: flat colours per cell type")
        .visual_only()
        .bool_param("Show Labels", false)
        .with_tooltip("Render elevation value at each cell centre")
        .visual_only()
        .enum_param(
            "Label Scale",
            vec!["Earth-like".into(), "Linear".into(), "Compressed".into()],
            0,
        )
        .with_tooltip(
            "Earth-like: power-law mapping to real elevation ranges (land ^1.5 ×8850m, ocean ^0.7 ×-10994m). \
             Linear: v×8000m. Compressed: v×2000m.",
        )
        .visual_only()
        .build()
}

pub fn build_default_river_params() -> ParamGroup {
    ParamGroupBuilder::new("River")
        .enum_param("Erosion", vec!["None".into(), "Flow Weighted".into()], 0)
        .with_tooltip("Flow Weighted: single-pass channel carving proportional to sqrt(accumulated flow)")
        .with_section("Erosion Settings")
        .float_param("Carve Strength", 0.08, 0.0, 0.3, 0.005)
        .with_tooltip("Erosion depth multiplier (k: elev -= k * sqrt(flow)); active when Erosion = Flow Weighted")
        .with_section("Filtering")
        .float_param("Min Flow", 0.04, 0.001, 0.2, 0.001)
        .with_tooltip("Minimum normalized flow at river mouth to keep a path")
        .int_param("Min Length", 4, 2, 20, 1)
        .with_tooltip("Minimum corner count for a river path to be kept")
        .with_section("Lake Detection")
        .float_param("Lake Depth Threshold", 0.05, 0.0, 0.3, 0.005)
        .with_tooltip("depth = spill_elevation - depression_elevation: >= threshold → lake, < threshold → overflow")
        .with_section("Display")
        .float_param("River Scale", 1.0, 0.1, 5.0, 0.1)
        .with_tooltip("Visual width multiplier for river rendering")
        .visual_only()
        .build()
}

pub fn build_default_feature_params() -> ParamGroup {
    ParamGroupBuilder::new("Features")
        .with_section("Detection")
        .float_param("Pass Max Elevation", 0.45, 0.1, 0.7, 0.02)
        .with_tooltip("Land cells below this elevation qualify as mountain pass candidates")
        .float_param("Pass Min Neighbor Elevation", 0.60, 0.3, 1.0, 0.02)
        .with_tooltip("Neighbor cells above this elevation count as 'mountain' for pass detection")
        .int_param("Pass Min Mountain Neighbors", 2, 1, 4, 1)
        .with_tooltip("Minimum number of mountain neighbors required to tag a cell as a mountain pass")
        .float_param("Fertile Max Elevation", 0.30, 0.05, 0.6, 0.02)
        .with_tooltip("Land cells below this elevation qualify as fertile valley candidates")
        .float_param("Fertile Min Moisture", 0.55, 0.3, 1.0, 0.05)
        .with_tooltip("Minimum moisture for fertile valley classification")
        .with_section("Display")
        .bool_param("Show River Mouths", true)
        .with_tooltip("Mark cells where rivers reach the coast")
        .visual_only()
        .bool_param("Show Harbor Candidates", true)
        .with_tooltip("Mark coast cells sheltered by land on multiple sides")
        .visual_only()
        .bool_param("Show Mountain Passes", true)
        .with_tooltip("Mark low-elevation cells flanked by high-elevation neighbors")
        .visual_only()
        .bool_param("Show Fertile Valleys", true)
        .with_tooltip("Mark low-elevation, moist, river-adjacent land cells")
        .visual_only()
        .bool_param("Show Resource Nodes", true)
        .with_tooltip("Mark noise-distributed resource placeholder locations")
        .visual_only()
        .float_param("Icon Size", 4.0, 1.5, 10.0, 0.5)
        .with_tooltip("Radius of feature indicator circles")
        .visual_only()
        .build()
}

pub fn build_default_terrain_params() -> ParamGroup {
    ParamGroupBuilder::new("Terrain Shaping")
        .with_section("Continentalness Noise")
        .float_param("C Scale", 0.003, 0.0005, 0.015, 0.0005)
        .with_tooltip("Sampling frequency of the continentalness noise field")
        .int_param("C Octaves", 5, 1, 8, 1)
        .with_tooltip("FBM octave count — more octaves add fine coastal detail")
        .float_param("C Lacunarity", 2.0, 1.5, 3.0, 0.1)
        .with_tooltip("Frequency multiplier per FBM octave")
        .float_param("C Persistence", 0.5, 0.2, 0.8, 0.05)
        .with_tooltip("Amplitude decay per FBM octave")
        .with_section("Erosion Noise")
        .float_param("E Scale", 0.006, 0.001, 0.03, 0.001)
        .with_tooltip("Sampling frequency of the erosion noise field")
        .int_param("E Octaves", 4, 1, 8, 1)
        .with_tooltip("FBM octave count")
        .float_param("E Lacunarity", 2.0, 1.5, 3.0, 0.1)
        .float_param("E Persistence", 0.5, 0.2, 0.8, 0.05)
        .with_section("Peaks & Valleys Noise")
        .float_param("PV Scale", 0.008, 0.001, 0.04, 0.001)
        .with_tooltip("Sampling frequency of the ridged peaks & valleys noise")
        .int_param("PV Octaves", 4, 1, 8, 1)
        .float_param("PV Lacunarity", 2.0, 1.5, 3.0, 0.1)
        .float_param("PV Persistence", 0.5, 0.2, 0.8, 0.05)
        .with_section("Continentalness Spline")
        .float_param("C Ocean Depth", -0.5, -1.0, -0.1, 0.05)
        .with_tooltip("Elevation at continentalness=-1 (deep ocean floor)")
        .float_param("C Shoreline", 0.02, -0.2, 0.2, 0.01)
        .with_tooltip("Elevation at continentalness=0 (shoreline)")
        .float_param("C Interior", 0.5, 0.1, 0.9, 0.05)
        .with_tooltip("Elevation at continentalness=1 (deep continental interior)")
        .with_section("Erosion Spline")
        .float_param("E Craggy Amp", 1.0, 0.3, 1.5, 0.05)
        .with_tooltip("PV amplitude scale when erosion=0 (craggy, un-eroded terrain)")
        .float_param("E Flat Amp", 0.1, 0.0, 0.5, 0.05)
        .with_tooltip("PV amplitude scale when erosion=1 (worn flat plains)")
        .with_section("Peaks & Valleys Spline")
        .float_param("PV Valley Offset", -0.3, -0.8, 0.0, 0.05)
        .with_tooltip("Elevation offset at PV=-1 (valley floors)")
        .float_param("PV Peak Offset", 0.4, 0.0, 0.8, 0.05)
        .with_tooltip("Elevation offset at PV=1 (mountain peaks)")
        .with_section("Display")
        .enum_param("View Mode", vec!["Elevation".into(), "Land Type".into(), "Continentalness".into(), "Erosion".into(), "Peaks & Valleys".into()], 0)
        .with_tooltip("Rendering mode — noise channel views are useful for tuning")
        .visual_only()
        .build()
}

pub fn build_default_landmass_params() -> ParamGroup {
    ParamGroupBuilder::new("Landmass")
        .enum_param(
            "Shaping Mode",
            vec!["Continent Seeds".into(), "Radial Gradient".into(), "Noise Threshold".into()],
            0,
        )
        .with_tooltip("Algorithm used to decide which cells are land vs ocean")
        // Radial params
        .with_section("Radial Gradient")
        .float_param("Falloff", 0.35, 0.05, 1.5, 0.01)
        .with_tooltip("[Radial] Controls how steeply land falls off toward the border")
        .float_param("X Scale", 0.8, 0.1, 2.0, 0.05)
        .with_tooltip("[Radial] Horizontal stretch of the landmass")
        .float_param("Y Scale", 0.8, 0.1, 2.0, 0.05)
        .with_tooltip("[Radial] Vertical stretch of the landmass")
        // Noise threshold params
        .with_section("Noise Threshold")
        .float_param("Noise Scale", 0.004, 0.001, 0.02, 0.001)
        .with_tooltip("[Noise / Seeds] Frequency of Perlin noise sampling")
        .float_param("Land Threshold", 0.05, -0.5, 0.5, 0.01)
        .with_tooltip("[Noise] Cells with noise > threshold become land")
        // Continent seeds params
        .with_section("Continent Seeds")
        .int_param("Num Seeds", 5, 1, 20, 1)
        .with_tooltip("[Seeds] Number of continent anchor points")
        .float_param("Spread Probability", 0.4, 0.2, 0.6, 0.01)
        .with_tooltip("[Seeds] Base probability of land spreading to a neighbour")
        .float_param("Noise Influence", 0.25, 0.0, 1.0, 0.01)
        .with_tooltip("[Seeds] How much Perlin noise perturbs the spread probability")
        // Shared elevation noise
        .with_section("Elevation Noise")
        .float_param("Elevation Noise Intensity", 0.4, 0.0, 1.0, 0.01)
        .with_tooltip("Amplitude of Perlin noise applied on top of distance-based elevation")
        // Display
        .with_section("Display")
        .float_param("Water Level", 0.0, -1.0, 1.0, 0.01)
        .with_tooltip("Visual threshold: cells below this elevation render as water (rendering only)")
        .visual_only()
        .enum_param(
            "View Mode",
            vec!["Elevation".into(), "Land Type".into()],
            0,
        )
        .with_tooltip("Elevation: gradient by height value.  Land Type: flat colours per cell type")
        .visual_only()
        .build()
}
