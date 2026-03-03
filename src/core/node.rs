//! Node struct - a single computation unit in the pipeline.

use crate::error::PondError;

use super::into_result::IntoNodeResult;
use super::traits::{DatasetEvent, DatasetRef, NodeInput, NodeOutput, PipelineInfo, PipelineItem};

pub struct Node<F, Input: NodeInput, Output: NodeOutput>
where
    F: Fn<Input::Args>,
{
    pub name: &'static str,
    pub func: F,
    pub input: Input,
    pub output: Output,
}

impl<F, Input, Output> PipelineInfo for Node<F, Input, Output>
where
    Input: NodeInput + Send + Sync,
    Output: NodeOutput + Send + Sync,
    F: Fn<Input::Args> + Send + Sync,
{
    fn get_name(&self) -> &'static str {
        self.name
    }

    fn is_leaf(&self) -> bool {
        true
    }

    fn for_each_child<'a>(&'a self, _f: &mut dyn FnMut(&'a dyn PipelineInfo)) {}

    fn for_each_input_id<'s>(&'s self, f: &mut dyn FnMut(&DatasetRef<'s>)) {
        self.input.for_each_input_id(f);
    }

    fn for_each_output_id<'s>(&'s self, f: &mut dyn FnMut(&DatasetRef<'s>)) {
        self.output.for_each_output_id(f);
    }
}

impl<F, Input, Output, E, R> PipelineItem<E> for Node<F, Input, Output>
where
    Input: NodeInput + Send + Sync,
    Output: NodeOutput + Send + Sync,
    F: Fn<Input::Args, Output = R> + Send + Sync,
    R: IntoNodeResult<Output::Output, E>,
    E: From<PondError>,
{
    fn call(&self, on_event: &mut dyn FnMut(&DatasetRef<'_>, DatasetEvent)) -> Result<(), E> {
        let args = self.input.load_data(on_event).map_err(E::from)?;
        let result = Fn::call(&self.func, args);
        let output = result.into_node_result()?;
        self.output.save_data(output, on_event).map_err(E::from)?;
        Ok(())
    }

    fn for_each_child_item<'a>(&'a self, _f: &mut dyn FnMut(&'a dyn PipelineItem<E>)) {}
}
