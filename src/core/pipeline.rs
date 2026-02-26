//! Pipeline struct - a container for multiple steps.

use super::steps::Steps;
use super::traits::{DatasetRef, NodeInput, NodeOutput, PipelineItem};

pub struct Pipeline<S: Steps, Input: NodeInput, Output: NodeOutput> {
    pub name: &'static str,
    pub steps: S,
    pub input: Input,
    pub output: Output,
}

impl<S: Steps + Send + Sync, Input: NodeInput + Send + Sync, Output: NodeOutput + Send + Sync>
    PipelineItem for Pipeline<S, Input, Output>
{
    fn call(&self) {}

    fn get_name(&self) -> &'static str {
        self.name
    }

    fn is_leaf(&self) -> bool {
        false
    }

    fn for_each_child<'a>(&'a self, f: &mut dyn FnMut(&'a dyn PipelineItem)) {
        self.steps.for_each_item(f);
    }

    fn for_each_input_id(&self, f: &mut dyn FnMut(&DatasetRef)) {
        self.input.for_each_input_id(f);
    }

    fn for_each_output_id(&self, f: &mut dyn FnMut(&DatasetRef)) {
        self.output.for_each_output_id(f);
    }
}
