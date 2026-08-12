//! Alias struct - a no-op step declaring that two datasets hold the same data.

use crate::error::PondError;

use super::traits::{DatasetEvent, DatasetRef, StepMeta, Leaf, Step, StepKind};
use crate::datasets::Dataset;

/// Declares that two datasets hold the same data.
///
/// Creates an edge in the pipeline graph — so `output` counts as produced
/// and downstream steps may consume it — without performing any computation
/// or data transfer. Typically used when the same bytes are read back through
/// a different dataset type (e.g. text written, then read as a DataFrame).
pub struct Alias<'a, Input: Dataset + Send + Sync, Output: Dataset + Send + Sync> {
    pub name: &'static str,
    pub input: &'a Input,
    pub output: &'a Output,
}

impl<Input, Output> StepMeta for Alias<'_, Input, Output>
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

    fn for_each_child<'a>(&'a self, _f: &mut dyn FnMut(&'a dyn StepMeta)) {}

    fn for_each_input<'s>(&'s self, f: &mut dyn FnMut(&DatasetRef<'s>)) {
        f(&DatasetRef::from_ref(self.input));
    }

    fn for_each_output<'s>(&'s self, f: &mut dyn FnMut(&DatasetRef<'s>)) {
        f(&DatasetRef::from_ref(self.output));
    }
}

impl<Input, Output, E> Leaf<E> for Alias<'_, Input, Output>
where
    Input: Dataset + Send + Sync,
    Output: Dataset + Send + Sync,
    E: From<PondError>,
{
    fn call(&self, _on_event: &mut dyn FnMut(&DatasetRef<'_>, DatasetEvent<'_>) -> Result<crate::hooks::HookControl, crate::hooks::HookAbort>) -> Result<(), E> {
        Ok(())
    }
}

impl<Input, Output, E> Step<E> for Alias<'_, Input, Output>
where
    Input: Dataset + Send + Sync,
    Output: Dataset + Send + Sync,
    E: From<PondError>,
{
    fn kind(&self) -> StepKind<'_, E> { StepKind::Leaf(self) }
}
