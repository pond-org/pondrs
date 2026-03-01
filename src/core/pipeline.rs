//! Pipeline struct - a container for multiple steps.

use super::steps::{StepInfo, Steps};
use super::traits::{DatasetEvent, DatasetRef, NodeInput, NodeOutput, PipelineInfo, PipelineItem};

pub struct Pipeline<S: StepInfo, Input: NodeInput, Output: NodeOutput> {
    pub name: &'static str,
    pub steps: S,
    pub input: Input,
    pub output: Output,
}

impl<S: StepInfo + Send + Sync, Input: NodeInput + Send + Sync, Output: NodeOutput + Send + Sync>
    PipelineInfo for Pipeline<S, Input, Output>
{
    fn get_name(&self) -> &'static str {
        self.name
    }

    fn is_leaf(&self) -> bool {
        false
    }

    fn for_each_child<'a>(&'a self, f: &mut dyn FnMut(&'a dyn PipelineInfo)) {
        self.steps.for_each_info(f);
    }

    fn for_each_input_id(&self, f: &mut dyn FnMut(&DatasetRef)) {
        self.input.for_each_input_id(f);
    }

    fn for_each_output_id(&self, f: &mut dyn FnMut(&DatasetRef)) {
        self.output.for_each_output_id(f);
    }
}

impl<E, S, Input: NodeInput + Send + Sync, Output: NodeOutput + Send + Sync>
    PipelineItem<E> for Pipeline<S, Input, Output>
where
    S: Steps<E> + Send + Sync,
{
    fn call(&self, _on_event: &mut dyn FnMut(&DatasetRef, DatasetEvent)) -> Result<(), E> {
        Ok(())
    }

    fn for_each_child_item<'a>(&'a self, f: &mut dyn FnMut(&'a dyn PipelineItem<E>)) {
        self.steps.for_each_item(f);
    }
}
