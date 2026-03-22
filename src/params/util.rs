use crate::params::builder::ParamGroupBuilder;
use crate::params::ParamGroup;

pub fn build_default_biome_params() -> ParamGroup {
    ParamGroupBuilder::new("Biome")
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
        .float_param("Wind Angle", 270.0, 0.0, 360.0, 5.0)
        .with_tooltip("[Uniform] Travel direction in degrees (0=east, 90=south, 180=west, 270=north)")
        .float_param("Moisture Decay", 0.72, 0.5, 1.0, 0.01)
        .with_tooltip("Fraction of moisture retained per graph-hop inland (lower = drier interiors)")
        .float_param("Orographic Strength", 0.75, 0.0, 1.0, 0.05)
        .with_tooltip("How strongly mountain ridges block moisture on their leeward side")
        .float_param("Mountain Threshold", 0.55, 0.3, 0.9, 0.05)
        .with_tooltip("Cell elevation above which orographic shadow kicks in")
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
        .int_param("Point Count", 300, 10, 5000, 10)
        .with_tooltip("Number of points (used by Random Uniform and Uniform Grid)")
        .float_param("Min Distance", 35.0, 5.0, 120.0, 1.0)
        .with_tooltip("Minimum spacing between points (used by Poisson Disc — controls point density)")
        .float_param("Point Radius", 2.0, 0.5, 8.0, 0.5)
        .with_tooltip("Visual display radius of each point (rendering only)")
        .visual_only()
        .build()
}

pub fn build_default_voronoi_params() -> ParamGroup {
    ParamGroupBuilder::new("Voronoi Graph")
        .int_param("Lloyd Relaxations", 2, 0, 6, 1)
        .with_tooltip("Iterations of Lloyd relaxation to regularise the mesh (2–3 recommended)")
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
        .int_param("Num Ridges", 4, 0, 12, 1)
        .with_tooltip("Number of mountain ridge lines to inject")
        .float_param("Ridge Intensity", 0.5, 0.0, 1.0, 0.02)
        .with_tooltip("Peak height added at the ridge centre")
        .float_param("Ridge Width", 0.12, 0.02, 0.5, 0.01)
        .with_tooltip("Falloff width of each ridge (fraction of canvas diagonal)")
        // Redistribution
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
        .float_param("Stage 3 Blend", 0.5, 0.0, 1.0, 0.02)
        .with_tooltip(
            "Weight given to Stage 3 rough elevation as a starting point. \
             0 = ignore Stage 3 entirely and recompute from scratch.",
        )
        // Visual-only
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
        .build()
}

pub fn build_default_landmass_params() -> ParamGroup {
    ParamGroupBuilder::new("Landmass")
        // --- shaping mode ---
        .enum_param(
            "Shaping Mode",
            vec!["Continent Seeds".into(), "Radial Gradient".into(), "Noise Threshold".into()],
            0,
        )
        .with_tooltip("Algorithm used to decide which cells are land vs ocean")
        // --- radial params ---
        .float_param("Falloff", 0.35, 0.05, 1.5, 0.01)
        .with_tooltip("[Radial] Controls how steeply land falls off toward the border")
        .float_param("X Scale", 0.8, 0.1, 2.0, 0.05)
        .with_tooltip("[Radial] Horizontal stretch of the landmass")
        .float_param("Y Scale", 0.8, 0.1, 2.0, 0.05)
        .with_tooltip("[Radial] Vertical stretch of the landmass")
        // --- noise threshold params ---
        .float_param("Noise Scale", 0.004, 0.001, 0.02, 0.001)
        .with_tooltip("[Noise / Seeds] Frequency of Perlin noise sampling")
        .float_param("Land Threshold", 0.05, -0.5, 0.5, 0.01)
        .with_tooltip("[Noise] Cells with noise > threshold become land")
        // --- continent seeds params ---
        .int_param("Num Seeds", 5, 1, 20, 1)
        .with_tooltip("[Seeds] Number of continent anchor points")
        .float_param("Spread Probability", 0.72, 0.1, 1.0, 0.01)
        .with_tooltip("[Seeds] Base probability of land spreading to a neighbour")
        .float_param("Noise Influence", 0.25, 0.0, 1.0, 0.01)
        .with_tooltip("[Seeds] How much Perlin noise perturbs the spread probability")
        // --- shared elevation noise ---
        .float_param("Elevation Noise Intensity", 0.4, 0.0, 1.0, 0.01)
        .with_tooltip("Amplitude of Perlin noise applied on top of distance-based elevation")
        // --- visual-only ---
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
