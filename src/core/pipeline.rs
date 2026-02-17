//! Pipeline struct - a container for multiple steps.

use super::steps::Steps;
use super::traits::{NodeInput, NodeOutput, PipelineItem};

pub struct Pipeline<S: Steps, Input: NodeInput, Output: NodeOutput> {
    pub steps: S,
    pub input: Input,
    pub output: Output,
}

impl<S: Steps + Send + Sync, Input: NodeInput + Send + Sync, Output: NodeOutput + Send + Sync>
    PipelineItem for Pipeline<S, Input, Output>
{
    fn call(&self) {}

    fn get_name(&self) -> &'static str {
        "pipeline"
    }

    fn is_leaf(&self) -> bool {
        false
    }

    fn for_each_child<'a>(&'a self, f: &mut dyn FnMut(&'a dyn PipelineItem)) {
        self.steps.for_each_item(f);
    }
}
