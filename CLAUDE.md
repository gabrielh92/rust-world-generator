# Fantasy Football Empire — World Generator
# CLAUDE.md — AI Context File
#
# USAGE: This file is read automatically by Claude Code on startup.
# It is organized in modular sections. The General section always applies.
# Append stage-specific sections as you work on them.
# Future sections for PROM, ATHN, HERC will be added as those stages are designed.

<!--
  ╔══════════════════════════════════════════════════════╗
  ║  SECTION 1 — GENERAL PROJECT CONTEXT                ║
  ║  Always active. Never remove this section.           ║
  ╚══════════════════════════════════════════════════════╝
-->

## What This Project Is

This is the `worldgen` codebase for **Fantasy Football Empire (FFE)**, a football management
game with a fully procedurally generated world. The generator produces the physical world,
population, geopolitics, and football federation structures the game runs on top of.

The generator is a sequential pipeline of named stages, collectively called the **Orchestrator**.
Each stage is codenamed after a Greek figure:

| Codename | Full Name  | Responsibility                                      |
|----------|------------|-----------------------------------------------------|
| GAIA     | Gaia       | Physical world — landmasses, elevation, biomes, rivers |
| PROM     | Prometheus | Population — races, individuals, traits, stats      |
| ATHN     | Athena     | Geopolitics — countries, cities, borders            |
| HERC     | Hercules   | Football federations — leagues, tournaments         |

Each stage depends on the output of the previous.
**Current focus is GAIA only. Do not touch PROM / ATHN / HERC.**

---

## Tech Stack & Architecture Decisions

### Rust — Crate Structure

- **`gaia-core`** — headless library crate with zero rendering dependencies.
  All generation logic lives here. Must be fully testable without a window.
- **`gaia-viewer`** — thin binary crate that depends on `gaia-core` for visualization.
  Currently uses `egui`. Will be replaced by Godot integration later.

**Hard rule:** generation logic never imports rendering code. If a PR mixes them, revert it.

### RNG Contract

- Use `rand_pcg::Pcg64` seeded from a `u64` for all RNG. No `thread_rng()` in `gaia-core`.
- Sub-stage seeds are derived deterministically: `parent_seed ^ STAGE_CONSTANT`.
- Snapshot tests enforce that a given `WorldSeed` produces **bit-identical output** across runs.

### Godot Integration (Future)

- The viewer will eventually be replaced by Godot via `gdext` (GDExtension Rust bindings),
  so Godot calls into compiled Rust natively — no subprocess boundary.
- `gaia-core` must remain a pure Rust library so this swap is drop-in.
- **No Godot-specific code yet.** Keep `gaia-core` clean.

---

## Generative Philosophy

- **Seed-deterministic:** A `WorldSeed: u64` must always produce identical output.
- **Pipeline architecture:** Each stage consumes the output of the previous.
  Stages do not reach into each other's internals.
- **Modular & portable:** No FFE-specific game logic belongs in `gaia-core`.
  The generator should be reusable in other projects.
- **Interesting over realistic:** The goal is varied, believable, interesting worlds —
  not physical simulation. Good heuristics beat accurate models here.
- **Template-driven:** Generation parameters are config structs, not hardcoded values.
  Worlds are tunable without recompilation.

### Key Reference

Polygon map generation article (primary reference for GAIA approach):
https://www-cs-students.stanford.edu/~amitp/game-programming/polygon-map-generation/

The **Voronoi/Delaunay dual graph** is the foundational data structure.
All map data is stored on `Cell`, `Corner`, and `Edge` nodes — not on a pixel grid.

---

<!--
  ╔══════════════════════════════════════════════════════╗
  ║  SECTION 2 — GAIA PIPELINE                          ║
  ║  Add when working on world generation.               ║
  ╚══════════════════════════════════════════════════════╝
-->

## GAIA: World Generation Pipeline

GAIA is a sequential pipeline. Each stage annotates the shared graph and passes it forward.
The final output is a `WorldMap` struct, which is the sole input to PROM and ATHN.

**If PROM or ATHN needs a geographic property, add it as a Stage 8 annotation —
not inline logic inside those stages.**

---

### Core Data Model

```rust
struct Cell {
    id: u32,
    center: Vec2,
    corners: Vec<CornerId>,
    neighbors: Vec<CellId>,
    is_land: bool,
    is_coast: bool,
    elevation: f32,         // 0.0 = sea level, 1.0 = max peak
    moisture: f32,          // 0.0 = arid, 1.0 = saturated
    biome: Biome,
    river_volume: f32,
    features: Vec<Feature>, // harbor_candidate, river_mouth, mountain_pass, etc.
}

struct Corner {
    id: u32,
    position: Vec2,
    elevation: f32,
    river_flow: f32,        // downhill flow accumulation
    adjacent_cells: Vec<CellId>,
    adjacent_corners: Vec<CornerId>,
}
```

The graph topology (cells, corners, edges) is **immutable after Stage 2**.
All later stages annotate; they never rewire.

---

### Stage 0 — Landmass Orchestration & Stitching

Drives the whole process. Spawns parallel landmass generation jobs.

**Stopping condition:** `target_land_ratio ± ratio_tolerance` — NOT a fixed continent count.
This is what produces natural variety (2 big continents one seed, 1 big + archipelago another).

- Each landmass job runs on its own thread, generating in local normalized space `[0,1]²`.
- Sub-seeds: `WorldSeed ^ (job_index * LANDMASS_PRIME)`
- The **Stitcher** arranges placed landmasses via:
  1. Minimum ocean gap enforcement
  2. Force-directed repulsion spreading (a few iterations)
  3. Tectonic-flavored placement heuristics (implicit plate centers drive convergent/divergent rules)
- Island clusters (volcanic arcs, shelf fragments) are spawned as budget gap-fillers.
- After placement: all landmass subgraphs + a coarse ocean fill mesh are merged into `WorldGraph`.

```rust
struct LandmassConfig {
    target_land_ratio: f32,      // 0.33 default
    ratio_tolerance: f32,        // 0.06 default
    min_landmass_area: f32,      // fraction of world area, e.g. 0.002
    max_landmass_area: f32,      // fraction of world area, e.g. 0.18
    min_ocean_gap: f32,
    n_plate_centers: u32,        // 6–10, drives tectonic heuristics
    island_arc_probability: f32,
    thread_pool_size: u32,
}
```

---

### Stage 1 — Point Sampling

- **Poisson disc sampling** over world bounds (not uniform random — avoids clustering).
- 2–3 iterations of Lloyd relaxation to regularize without over-uniformizing.

---

### Stage 2 — Voronoi/Delaunay Graph Construction

- Compute Voronoi diagram + dual Delaunay triangulation simultaneously.
- Recommended crate: `spade`
- Clip to world bounds. Boundary cells are permanent ocean.
- Graph is immutable after this stage.

---

### Stage 3 — Island/Continent Shaping

- Finalizes `is_land`, `is_coast`, `is_ocean` per cell.
- For FFE: continent-seed flood fill with noise perturbation (not pure noise threshold).
- Target: 4–7 distinct continents of varied size (driven by Stage 0 config, not hardcoded).

---

### Stage 4 — Elevation

- Ocean cells: negative elevation proportional to distance from coast (shelf effect).
- Land cells: base noise + distance-from-coast boost + mountain ridge injection.
- Redistribute final values via histogram equalization — prevents all-mountains or all-flatlands.
- Stored on both cells (average) and corners (precise — required for river flow calculation).

#### Hydraulic Erosion (feature-flagged, deferred to pre-Stage 7)

Optional post-processing step at the end of Stage 4, toggled by `erosion_enabled: bool`.
Simulates water droplets flowing downhill over N iterations, carrying and depositing sediment:

- **Carves** natural river valley channels — Stage 7 traces these rather than inventing paths.
- **Deposits** sediment in lowland floodplains (flattens and enriches them).
- **Identifies** lake sinks — local minima where water pools before reaching the ocean.

When disabled, Stage 7 places rivers heuristically on the raw elevation field.
Implement this just before Stage 7, once elevation pipeline is stable.

**Reference:** Sebastian Lague — Hydraulic Erosion
https://www.youtube.com/watch?v=eaXk97ujbPQ

---

### Stage 5 — Moisture & Wind

- Global prevailing wind direction per latitude band.
- Upwind-of-ocean cells = high moisture. Moisture drops as air moves inland.
- **Orographic shadow:** lee side of mountain ranges = drastically reduced moisture (rain shadow deserts).
- No physical accuracy required. Calibrate to target distribution (e.g. ≤20% desert globally).

---

### Stage 6 — Biome Classification

Discretized Whittaker diagram: `elevation × moisture → Biome`.

| | Low Moisture | Mid Moisture | High Moisture |
|---|---|---|---|
| **High Elevation** | Alpine Tundra | Alpine Meadow | Alpine Forest |
| **Mid Elevation** | Shrubland | Temperate Forest | Rainforest |
| **Low Elevation** | Desert | Grassland / Savanna | Tropical Rainforest |
| **Coast** | Beach | Wetland | Mangrove |

Ocean: Shallow Coast, Open Ocean, Deep Trench.

Biomes inform PROM's population density: desert/tundra = very low; river grassland/temperate = high.

---

### Stage 7 — River Generation

- Rivers flow along **corner edges** (not cell edges) downhill.
- Spring candidates: high-elevation corners above moisture threshold.
- Steepest descent from spring → ocean/sink. Merge paths, accumulate flow volume.
- Discard rivers below minimum length or volume. Mark river-adjacent cells.

---

### Stage 8 — Feature Annotation

Final pass. Tags cells for ATHN and PROM to query without re-deriving geography:

- `river_mouth` — where a river path reaches a coast cell
- `harbor_candidate` — sheltered concave coastline geometry
- `mountain_pass` — low-elevation land cell surrounded by high-elevation neighbors
- `fertile_valley` — low elevation, high moisture, non-coast, river-adjacent
- `resource_node` — placeholder for future resource system

**This is a query index.** If a downstream stage needs geographic data not listed here,
add it to Stage 8 — don't compute it inline in PROM or ATHN.

---

### Output Contract

```rust
struct WorldMap {
    seed: u64,
    bounds: Rect,
    cells: Vec<Cell>,
    corners: Vec<Corner>,
    continents: Vec<Continent>,
    rivers: Vec<RiverPath>,
    metadata: GenerationMetadata,
}
```

This is the **sole handoff to PROM and ATHN.** Neither stage re-derives geography.

---

### What NOT To Do

- Do not use `thread_rng()` in `gaia-core`. Always use seeded RNG.
- Do not mix rendering code into generation logic.
- Do not hardcode continent count. Use `target_land_ratio`.
- Do not simulate plate tectonics. Use placement heuristics only.
- Do not let ATHN or PROM re-derive geography. Add Stage 8 annotations instead.

---

<!--
  ╔══════════════════════════════════════════════════════╗
  ║  SECTION 3 — PROM (placeholder)                     ║
  ║  Add when working on population generation.          ║
  ╚══════════════════════════════════════════════════════╝
-->

<!-- PROM section to be added once GAIA is stable. -->

<!--
  ╔══════════════════════════════════════════════════════╗
  ║  SECTION 4 — ATHN (placeholder)                     ║
  ║  Add when working on geopolitics.                    ║
  ╚══════════════════════════════════════════════════════╝
-->

<!-- ATHN section to be added after PROM. -->

<!--
  ╔══════════════════════════════════════════════════════╗
  ║  SECTION 5 — HERC (placeholder)                     ║
  ║  Add when working on football federation structure.  ║
  ╚══════════════════════════════════════════════════════╝
-->

<!-- HERC section to be added after ATHN. -->
