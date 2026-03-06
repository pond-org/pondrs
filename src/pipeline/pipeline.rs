//! Pipeline struct - a container for multiple steps.

use super::steps::{StepInfo, Steps};
use super::traits::{DatasetEvent, DatasetRef, NodeInput, NodeOutput, PipelineInfo, RunnableStep};

pub struct Pipeline<S: StepInfo, Input: NodeInput, Output: NodeOutput> {
    pub name: &'static str,
    pub steps: S,
    pub input: Input,
    pub output: Output,
}

impl<S: StepInfo + Send + Sync, Input: NodeInput + Send + Sync, Output: NodeOutput + Send + Sync>
    PipelineInfo for Pipeline<S, Input, Output>
{
    fn name(&self) -> &'static str {
        self.name
    }

    fn is_leaf(&self) -> bool {
        false
    }

    fn type_string(&self) -> &'static str {
        "pipeline"
    }

    fn for_each_child<'a>(&'a self, f: &mut dyn FnMut(&'a dyn PipelineInfo)) {
        self.steps.for_each_info(f);
    }

    fn for_each_input<'s>(&'s self, f: &mut dyn FnMut(&DatasetRef<'s>)) {
        self.input.for_each_input(f);
    }

    fn for_each_output<'s>(&'s self, f: &mut dyn FnMut(&DatasetRef<'s>)) {
        self.output.for_each_output(f);
    }
}

impl<E, S, Input: NodeInput + Send + Sync, Output: NodeOutput + Send + Sync>
    RunnableStep<E> for Pipeline<S, Input, Output>
where
    S: Steps<E> + Send + Sync,
{
    /// Pipeline is a container — execution happens via `for_each_child_step`.
    /// Runners should never call this directly; they check `is_leaf()` first.
    fn call(&self, _on_event: &mut dyn FnMut(&DatasetRef<'_>, DatasetEvent)) -> Result<(), E> {
        unreachable!("Pipeline::call() should not be invoked directly — use for_each_child_step")
    }

    fn for_each_child_step<'a>(&'a self, f: &mut dyn FnMut(&'a dyn RunnableStep<E>)) {
        self.steps.for_each_item(f);
    }
}
