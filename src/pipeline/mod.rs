use std::fmt::Debug;

use crate::{params::ParamGroup, visualization::VisualLayer};
use crate::util::make_stage_data_key;

pub mod points;
pub mod voronoi;

/// The central struct that manages and runs all computation stages in sequence.
/// Owns both the ordered list of stages and their shared data outputs.
pub struct Pipeline {
	pub stages: Vec<PipelineStage>,
	pub data: StageDataMap,
}

impl Pipeline {
	pub fn new() -> Self {
		Self {
			stages: Vec::new(),
			data: StageDataMap::new(),
		}
	}

	pub fn add_stage(&mut self, stage: PipelineStage) {
		self.stages.push(stage);
		self.stages.sort_by_key(|s| s.executor.rank());
	}

	pub fn run(&mut self) {
		for stage in self.stages.iter_mut() {
			let output = stage.executor.run(stage.params.as_ref(), &self.data);
			self.data.insert(make_stage_data_key(stage.executor.name(), stage.executor.rank()), output);
		}
	}


}

/// A mapping from stage names to their computed output data.
/// Acts as the shared memory across all pipeline stages.
pub type StageDataMap = std::collections::HashMap<String, Box<dyn StageData>>;

pub trait StageData: Debug + Send + Sync {
    fn as_any(&self) -> &dyn std::any::Any;
}

/// The execution logic for a pipeline stage.
/// Takes in shared pipeline data and computes new output to be stored in the data map.
pub trait PipelineStageExecutor {
	fn name(&self) -> &str;

	/// Returns the rank of this stage in the whole pipeline
	fn rank(&self) -> u8;

    /// Run computation logic, using inputs and returning output
    fn run(&mut self, params: Option<&ParamGroup>, pipeline_data: &StageDataMap) -> Box<dyn StageData>;
}

/// Represents a full stage of computation, including its executor, UI params, and visual rendering layer.
/// Controls execution, parameters, and visualization for a specific part of the generation pipeline.
pub struct PipelineStage {
	pub executor: Box<dyn PipelineStageExecutor>,
	pub params: Option<ParamGroup>,
	pub visual_layer: Box<dyn VisualLayer>,
}