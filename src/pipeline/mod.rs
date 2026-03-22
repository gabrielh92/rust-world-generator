use std::fmt::Debug;

use crate::{params::ParamGroup, visualization::VisualLayer};

pub mod points;
pub mod voronoi;
pub mod landmass;

/// The central struct that manages and runs all computation stages in sequence.
/// Owns both the ordered list of stages and their shared data outputs.
pub struct Pipeline {
    pub stages: Vec<PipelineStage>,
    pub data: StageDataMap,
}

impl Pipeline {
    pub fn new() -> Self {
        Self {
            stages: Vec::new(), // guaranteed ordered at insert
            data: StageDataMap::new(),
        }
    }

    pub fn add_stage(&mut self, stage: PipelineStage) {
        self.stages.push(stage);
        self.stages.sort_by_key(|s| s.executor.rank());
    }

    /// Run all stages in rank order.
    pub fn run(&mut self) {
        self.run_from_stage(0);
    }

    /// Run stages from `idx` onward (inclusive), reusing upstream data already in the map.
    /// Safe to call after a partial param change — upstream stage outputs remain valid.
    pub fn run_from_stage(&mut self, idx: usize) {
        // Use an iterator (not index expressions) so the borrow checker can apply
        // field splitting: stage.executor (mut) and stage.params (ref) are disjoint,
        // and self.stages (via iterator) and self.data are disjoint fields of Pipeline.
        for stage in self.stages[idx..].iter_mut() {
            let key = stage.executor.data_key();
            let output = stage.executor.run(stage.params.as_ref(), &self.data);
            self.data.insert(key.to_string(), output);
        }
    }
}

/// A mapping from stage data-keys to their computed outputs.
/// Acts as shared memory across all pipeline stages.
pub type StageDataMap = std::collections::HashMap<String, Box<dyn StageData>>;

pub trait StageData: Debug + Send + Sync {
    fn as_any(&self) -> &dyn std::any::Any;
}

/// The execution logic for a pipeline stage.
pub trait PipelineStageExecutor {
    fn name(&self) -> &str;

    /// Rank determines execution order (lower = earlier).
    fn rank(&self) -> u8;

    /// Stable key used to store and retrieve this stage's output in `StageDataMap`.
    /// Must be unique across all stages. Prefer the module-level `STAGE_DATA_KEY` constant.
    fn data_key(&self) -> &'static str;

    /// Run computation, consuming upstream data and returning this stage's output.
    fn run(&mut self, params: Option<&ParamGroup>, pipeline_data: &StageDataMap) -> Box<dyn StageData>;
}

/// A full pipeline stage: executor + UI params + visual rendering layer.
pub struct PipelineStage {
    pub executor: Box<dyn PipelineStageExecutor>,
    pub params: Option<ParamGroup>,
    pub visual_layer: Box<dyn VisualLayer>,
}
