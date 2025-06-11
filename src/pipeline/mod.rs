use crate::params::ParamGroup;

pub trait PipelineStage {
    fn name(&self) -> &str;

    /// Returns a mutable set of parameters (if any) to be exposed in UI
    fn params(&mut self) -> Option<&mut ParamGroup>;

    /// Run computation logic, using inputs and returning output
    fn run(&mut self, inputs: &StageOutputs) -> StageOutput;

    /// Returns cached output, if available
    fn output(&self) -> Option<&StageOutput>;

    /// Marks the stage as needing re-run (e.g., param changed)
    fn mark_dirty(&mut self);
    fn is_dirty(&self) -> bool;
}

pub type StageOutputs = std::collections::HashMap<String, StageOutput>;

pub enum StageOutput {
	// todo: replace with an agnostic vector type
    Points(Vec<egui::Pos2>),
    // Add other types as needed
}
