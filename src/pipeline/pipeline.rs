//! Pipeline struct - a container for multiple steps.

use super::steps::{PipelineInfo, Steps};
use super::traits::{DatasetRef, NodeInput, NodeOutput, StepInfo, GroupStep, RunnableStep, StepKind};

/// A named group of steps with declared input/output dataset contracts.
///
/// Pipelines are containers — they delegate execution to their child steps
/// and are never called directly by runners.
pub struct Pipeline<S: PipelineInfo, Input: NodeInput, Output: NodeOutput> {
    pub name: &'static str,
    pub steps: S,
    pub input: Input,
    pub output: Output,
}

impl<S: PipelineInfo + Send + Sync, Input: NodeInput + Send + Sync, Output: NodeOutput + Send + Sync>
    StepInfo for Pipeline<S, Input, Output>
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

    fn for_each_child<'a>(&'a self, f: &mut dyn FnMut(&'a dyn StepInfo)) {
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
    GroupStep<E> for Pipeline<S, Input, Output>
where
    S: Steps<E> + Send + Sync,
{
    fn for_each_child_step<'a>(&'a self, f: &mut dyn FnMut(&'a dyn RunnableStep<E>)) {
        self.steps.for_each_item(f);
    }
}

impl<E, S, Input: NodeInput + Send + Sync, Output: NodeOutput + Send + Sync>
    RunnableStep<E> for Pipeline<S, Input, Output>
where
    S: Steps<E> + Send + Sync,
{
    fn kind(&self) -> StepKind<'_, E> { StepKind::Group(self) }
    fn as_pipeline_info(&self) -> &dyn StepInfo { self }
}
