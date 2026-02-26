//! Node struct - a single computation unit in the pipeline.

use super::traits::{DatasetRef, NodeInput, NodeOutput, PipelineItem};

pub struct Node<F, Input: NodeInput, Output: NodeOutput>
where
    F: Fn<Input::Args, Output = Output::Output>,
{
    pub name: &'static str,
    pub func: F,
    pub input: Input,
    pub output: Output,
}

impl<F, Input: NodeInput, Output: NodeOutput> PipelineItem for Node<F, Input, Output>
where
    F: Fn<Input::Args, Output = Output::Output> + Send + Sync,
    Input: Send + Sync,
    Output: Send + Sync,
{
    fn call(&self) {
        let args = self.input.load_data();
        let outputs = Fn::call(&self.func, args);
        self.output.save_data(outputs);
    }

    fn get_name(&self) -> &'static str {
        self.name
    }

    fn is_leaf(&self) -> bool {
        true
    }

    fn for_each_child<'a>(&'a self, _f: &mut dyn FnMut(&'a dyn PipelineItem)) {
        // No children, do nothing
    }

    fn for_each_input_id(&self, f: &mut dyn FnMut(&DatasetRef)) {
        self.input.for_each_input_id(f);
    }

    fn for_each_output_id(&self, f: &mut dyn FnMut(&DatasetRef)) {
        self.output.for_each_output_id(f);
    }
}
