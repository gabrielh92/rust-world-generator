use std::collections::{HashMap, HashSet};

use voronoice::{BoundingBox, VoronoiBuilder};

use crate::{
    canvas::CanvasData,
    mathlib::vec::Vec2,
    params::{util::build_default_voronoi_params, ParamGroup},
    pipeline::{
        points::{PointsOutput, STAGE_DATA_KEY as POINTS_KEY},
        PipelineStage, PipelineStageExecutor, StageData, StageDataMap,
    },
    visualization::voronoi::VoronoiVisualLayer,
};

pub fn make_voronoi_stage() -> PipelineStage {
    PipelineStage {
        executor: Box::new(VoronoiStage),
        params: Some(build_default_voronoi_params()),
        visual_layer: Box::new(VoronoiVisualLayer::default()),
    }
}

// ---------------------------------------------------------------------------
// Data model — matches the GDD Cell/Corner/Edge polygon graph
// ---------------------------------------------------------------------------

/// A shared vertex between multiple Voronoi cells (circumcenter of a Delaunay triangle).
/// This is the `Corner` from the GDD data model.
#[derive(Clone, Debug)]
pub struct Corner {
    pub id: usize,
    pub position: Vec2,
    /// IDs of all cells that share this corner.
    pub adjacent_cell_ids: Vec<usize>,
    /// IDs of corners directly connected to this one by a Voronoi edge.
    pub adjacent_corner_ids: Vec<usize>,
}

/// A Voronoi cell (polygon region) with full graph connectivity.
/// Topology is fixed after Stage 2; downstream stages enrich with elevation, biome, etc.
#[derive(Clone, Debug)]
pub struct GraphCell {
    pub id: usize,
    /// Voronoi site point (generator).
    pub center: Vec2,
    /// Ordered polygon vertices (Corner IDs, counter-clockwise).
    pub corner_ids: Vec<usize>,
    /// Adjacent cells from the Delaunay triangulation (i.e., cells sharing an edge).
    pub neighbor_ids: Vec<usize>,
    /// True if any corner touches the canvas boundary — always treated as ocean.
    pub is_border: bool,
}

/// A Voronoi edge — the shared boundary segment between two adjacent cells.
/// Endpoints are two adjacent corners; each side belongs to one cell.
/// `cell_b` is `None` for boundary edges that face the canvas edge rather than another cell.
#[derive(Clone, Debug)]
pub struct GraphEdge {
    pub id: usize,
    /// Indices into `GraphOutput::corners`.
    pub corner_a: usize,
    pub corner_b: usize,
    /// IDs of the cells on each side (matches `GraphCell::id`).
    pub cell_a: usize,
    pub cell_b: Option<usize>,
}

/// Output of the Voronoi/Graph stage: the immutable polygon graph.
#[derive(Clone, Debug)]
pub struct GraphOutput {
    pub cells: Vec<GraphCell>,
    pub corners: Vec<Corner>,
    pub edges: Vec<GraphEdge>,
}

impl StageData for GraphOutput {
    fn as_any(&self) -> &dyn std::any::Any { self }
}

// ---------------------------------------------------------------------------
// Stage executor
// ---------------------------------------------------------------------------

pub const STAGE_DATA_KEY: &str = "stage:voronoi";

#[derive(Default)]
pub struct VoronoiStage;

impl PipelineStageExecutor for VoronoiStage {
    fn name(&self) -> &str { "voronoi" }
    fn rank(&self) -> u8 { 2 }
    fn data_key(&self) -> &'static str { STAGE_DATA_KEY }

    fn run(&mut self, params: Option<&ParamGroup>, data: &StageDataMap) -> Box<dyn StageData> {
        let iterations = params
            .expect("Voronoi params required")
            .get_param("Lloyd Relaxations")
            .and_then(|p| p.as_int())
            .unwrap_or(2);

        let canvas = data
            .get("canvas")
            .and_then(|d| d.as_any().downcast_ref::<CanvasData>())
            .expect("Canvas data required for voronoi stage");

        let points = data
            .get(POINTS_KEY)
            .and_then(|d| d.as_any().downcast_ref::<PointsOutput>())
            .expect("Points stage must run before Voronoi stage");

        // Build the Voronoi diagram via voronoice
        let input_points = points.points.iter()
            .map(|p| voronoice::Point { x: p.x as f64, y: p.y as f64 })
            .collect();

        let diagram = VoronoiBuilder::default()
            .set_sites(input_points)
            .set_bounding_box(BoundingBox::new(
                voronoice::Point { x: (canvas.width / 2.) as f64, y: (canvas.height / 2.) as f64 },
                canvas.width as f64,
                canvas.height as f64,
            ))
            .set_lloyd_relaxation_iterations(iterations)
            .build()
            .expect("Failed to build Voronoi diagram");

        // --- Build the corner pool ---
        // Each unique Voronoi vertex becomes one Corner node.
        // We deduplicate by bit-exact position (voronoice stores each vertex once internally).
        let mut corners: Vec<Corner> = Vec::new();
        let mut corner_key_to_id: HashMap<(u32, u32), usize> = HashMap::new();

        let mut cells: Vec<GraphCell> = Vec::new();

        for voronoi_cell in diagram.iter_cells() {
            let site_id = voronoi_cell.site();
            let center = Vec2::new(
                voronoi_cell.site_position().x as f32,
                voronoi_cell.site_position().y as f32,
            );

            let vertex_positions: Vec<Vec2> = voronoi_cell
                .iter_vertices()
                .map(|v| Vec2::new(v.x as f32, v.y as f32))
                .collect();

            let is_border = vertex_positions.iter().any(|v| {
                v.x <= 0.0 || v.x >= canvas.width || v.y <= 0.0 || v.y >= canvas.height
            });

            // Map each vertex to a corner ID, creating new corners as needed
            let corner_ids: Vec<usize> = vertex_positions.iter().map(|&pos| {
                get_or_insert_corner(pos, &mut corners, &mut corner_key_to_id)
            }).collect();

            // Register: cell → corner adjacency
            for &cid in &corner_ids {
                if !corners[cid].adjacent_cell_ids.contains(&site_id) {
                    corners[cid].adjacent_cell_ids.push(site_id);
                }
            }

            // Register: corner → corner adjacency (consecutive vertices in the polygon ring)
            let n = corner_ids.len();
            for i in 0..n {
                let a = corner_ids[i];
                let b = corner_ids[(i + 1) % n];
                if !corners[a].adjacent_corner_ids.contains(&b) {
                    corners[a].adjacent_corner_ids.push(b);
                }
                if !corners[b].adjacent_corner_ids.contains(&a) {
                    corners[b].adjacent_corner_ids.push(a);
                }
            }

            cells.push(GraphCell {
                id: site_id,
                center,
                corner_ids,
                neighbor_ids: vec![], // filled in the Delaunay pass below
                is_border,
            });
        }

        // --- Build cell neighbour adjacency from shared corners ---
        // Two cells that share a Voronoi corner also share a Voronoi edge, making them neighbours.
        // This is more robust than scanning Delaunay triangles, which can miss cells that don't
        // appear as a source vertex in the triangulation output (degenerate Lloyd iterations, hull).
        {
            let id_to_idx: HashMap<usize, usize> =
                cells.iter().enumerate().map(|(i, c)| (c.id, i)).collect();

            let mut adj: HashMap<usize, HashSet<usize>> = HashMap::new();
            for corner in &corners {
                let ids = &corner.adjacent_cell_ids;
                for i in 0..ids.len() {
                    for j in (i + 1)..ids.len() {
                        adj.entry(ids[i]).or_default().insert(ids[j]);
                        adj.entry(ids[j]).or_default().insert(ids[i]);
                    }
                }
            }

            for (cell_id, neighbors) in &adj {
                if let Some(&idx) = id_to_idx.get(cell_id) {
                    cells[idx].neighbor_ids = neighbors.iter().cloned().collect();
                }
            }
        }

        // --- Build edges from consecutive corner pairs in each cell polygon ---
        // Each Voronoi edge is a segment between two adjacent corners, shared by exactly
        // two cells (or one cell for boundary edges). We deduplicate by sorted corner pair.
        let mut edges: Vec<GraphEdge> = Vec::new();
        let mut edge_key_to_id: HashMap<(usize, usize), usize> = HashMap::new();

        for cell in &cells {
            let n = cell.corner_ids.len();
            for i in 0..n {
                let ca = cell.corner_ids[i];
                let cb = cell.corner_ids[(i + 1) % n];
                let key = if ca < cb { (ca, cb) } else { (cb, ca) };

                if let Some(&edge_id) = edge_key_to_id.get(&key) {
                    // Second cell encountered for this edge — fill in cell_b
                    edges[edge_id].cell_b = Some(cell.id);
                } else {
                    let edge_id = edges.len();
                    edge_key_to_id.insert(key, edge_id);
                    edges.push(GraphEdge {
                        id: edge_id,
                        corner_a: key.0,
                        corner_b: key.1,
                        cell_a: cell.id,
                        cell_b: None,
                    });
                }
            }
        }

        Box::new(GraphOutput { cells, corners, edges })
    }
}

// ---------------------------------------------------------------------------
// Helper
// ---------------------------------------------------------------------------

fn get_or_insert_corner(
    pos: Vec2,
    corners: &mut Vec<Corner>,
    map: &mut HashMap<(u32, u32), usize>,
) -> usize {
    let key = (pos.x.to_bits(), pos.y.to_bits());
    if let Some(&id) = map.get(&key) {
        return id;
    }
    let id = corners.len();
    corners.push(Corner {
        id,
        position: pos,
        adjacent_cell_ids: vec![],
        adjacent_corner_ids: vec![],
    });
    map.insert(key, id);
    id
}
