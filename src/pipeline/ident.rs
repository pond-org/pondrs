//! Ident struct - a no-op identifying two datasets with each other using essentially a node.

use crate::error::PondError;

use super::traits::{DatasetEvent, DatasetRef, PipelineInfo, RunnableStep};
use crate::datasets::Dataset;

pub struct Ident<'a, Input: Dataset + Send + Sync, Output: Dataset + Send + Sync> {
    pub name: &'static str,
    pub input: &'a Input,
    pub output: &'a Output,
}

impl<Input, Output> PipelineInfo for Ident<'_, Input, Output>
where
    Input: Dataset + Send + Sync,
    Output: Dataset + Send + Sync,
{
    fn name(&self) -> &'static str {
        self.name
    }

    fn is_leaf(&self) -> bool {
        true
    }

    fn type_string(&self) -> &'static str {
        core::any::type_name::<Self>()
    }

    fn for_each_child<'a>(&'a self, _f: &mut dyn FnMut(&'a dyn PipelineInfo)) {}

    fn for_each_input<'s>(&'s self, f: &mut dyn FnMut(&DatasetRef<'s>)) {
        f(&DatasetRef::from_ref(self.input));
    }

    fn for_each_output<'s>(&'s self, f: &mut dyn FnMut(&DatasetRef<'s>)) {
        f(&DatasetRef::from_ref(self.output));
    }
}

impl<Input, Output, E> RunnableStep<E> for Ident<'_, Input, Output>
where
    Input: Dataset + Send + Sync,
    Output: Dataset + Send + Sync,
    E: From<PondError>,
{
    fn call(&self, _on_event: &mut dyn FnMut(&DatasetRef<'_>, DatasetEvent)) -> Result<(), E> {
        Ok(())
    }

    fn for_each_child_step<'a>(&'a self, _f: &mut dyn FnMut(&'a dyn RunnableStep<E>)) {}
}
