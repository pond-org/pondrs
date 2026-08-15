//! Pipeline struct - a container for multiple steps.

use super::steps::{StepsMeta, Steps};
use super::traits::{DatasetRef, NodeInputMeta, NodeOutputMeta, StepMeta, Group, Step, StepKind};

/// A named group of steps with declared input/output dataset contracts.
///
/// Pipelines are containers — they delegate execution to their child steps
/// and are never called directly by runners.
pub struct Pipeline<S: StepsMeta, Input: NodeInputMeta, Output: NodeOutputMeta> {
    pub name: &'static str,
    pub steps: S,
    pub input: Input,
    pub output: Output,
}

impl<S: StepsMeta + Send + Sync, Input: NodeInputMeta + Send + Sync, Output: NodeOutputMeta + Send + Sync>
    StepMeta for Pipeline<S, Input, Output>
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

    fn for_each_child<'a>(&'a self, f: &mut dyn FnMut(&'a dyn StepMeta)) {
        self.steps.for_each_meta(f);
    }

    fn for_each_input<'s>(&'s self, f: &mut dyn FnMut(&DatasetRef<'s>)) {
        self.input.for_each_input(f);
    }

    fn for_each_output<'s>(&'s self, f: &mut dyn FnMut(&DatasetRef<'s>)) {
        self.output.for_each_output(f);
    }
}

impl<E, S, Input: NodeInputMeta + Send + Sync, Output: NodeOutputMeta + Send + Sync>
    Group<E> for Pipeline<S, Input, Output>
where
    S: Steps<E> + Send + Sync,
{
    fn for_each_child_step<'a>(&'a self, f: &mut dyn FnMut(&'a dyn Step<E>)) {
        self.steps.for_each_step(f);
    }
}

impl<E, S, Input: NodeInputMeta + Send + Sync, Output: NodeOutputMeta + Send + Sync>
    Step<E> for Pipeline<S, Input, Output>
where
    S: Steps<E> + Send + Sync,
{
    fn kind(&self) -> StepKind<'_, E> { StepKind::Group(self) }
}
