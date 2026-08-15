//! Node struct - a single computation unit in the pipeline.

use crate::error::PondError;

use super::into_result::IntoNodeResult;
use super::stable::{StableFn, StableTuple};
use crate::hooks::{HookAbort, HookControl};
use super::traits::{DatasetEvent, DatasetRef, NodeInput, NodeOutput, StepMeta, Leaf, Step, StepKind};

/// Marker trait asserting that a return type is structurally compatible
/// with an output tuple `O`.
///
/// Implemented for `O` itself (bare tuple return) and `Result<O, E>`
/// (fallible return). This allows [`Node`] to catch output type mismatches
/// at construction time, before the pipeline error type `E` is known.
#[diagnostic::on_unimplemented(
    message = "node function returns `{Self}`, but `output` expects `{O}`",
    label = "returns `{Self}`",
    note = "a node function returns the `SaveItem` of each output dataset, in order",
    note = "return `{O}`, or `Result<{O}, E>` where the pipeline error type implements `From<E>`"
)]
pub trait CompatibleOutput<O: StableTuple> {}

impl<O: StableTuple> CompatibleOutput<O> for O {}
impl<O: StableTuple, E> CompatibleOutput<O> for Result<O, E> {}

/// A single computation unit: loads inputs, calls a function, saves outputs.
///
/// # Field order
///
/// Declare the fields as `name`, `input`, `output`, `func`:
///
/// ```rust,ignore
/// Node {
///     name: "scale",
///     input: (&cat.raw, &params.factor),
///     output: (&cat.scaled,),
///     func: |raw: Vec<f64>, factor: f64| (raw.iter().map(|v| v * factor).collect(),),
/// }
/// ```
///
/// Struct fields are type-checked in the order they are written, so listing
/// `input` and `output` first means the loaded argument types and the expected
/// return type are already known when the compiler checks `func`. Mistakes then
/// surface as closure-signature errors pointing at the closure ("expected closure
/// signature `fn(f64) -> _`") instead of associated-type mismatches pointing at
/// `Node {`. The order has no effect on behaviour — it only changes the
/// diagnostics you get when something does not line up.
///
/// A node function takes the `LoadItem` of each input, in order, and returns a
/// tuple of the `SaveItem` of each output, in order — or a `Result` of that tuple
/// when it can fail. A single output is a one-element tuple: `(value,)`.
pub struct Node<F, Input: NodeInput, Output: NodeOutput>
where
    F: StableFn<Input::Args>,
    F::Output: CompatibleOutput<Output::Output>,
{
    pub name: &'static str,
    pub func: F,
    pub input: Input,
    pub output: Output,
}

impl<F, Input, Output> StepMeta for Node<F, Input, Output>
where
    Input: NodeInput + Send + Sync,
    Output: NodeOutput + Send + Sync,
    F: StableFn<Input::Args> + Send + Sync,
    F::Output: CompatibleOutput<Output::Output>,
{
    fn name(&self) -> &'static str {
        self.name
    }

    fn is_leaf(&self) -> bool {
        true
    }

    fn type_string(&self) -> &'static str {
        core::any::type_name::<F>()
    }

    fn for_each_child<'a>(&'a self, _f: &mut dyn FnMut(&'a dyn StepMeta)) {}

    fn for_each_input<'s>(&'s self, f: &mut dyn FnMut(&DatasetRef<'s>)) {
        self.input.for_each_input(f);
    }

    fn for_each_output<'s>(&'s self, f: &mut dyn FnMut(&DatasetRef<'s>)) {
        self.output.for_each_output(f);
    }
}

impl<F, Input, Output, E, R> Leaf<E> for Node<F, Input, Output>
where
    Input: NodeInput + Send + Sync,
    Output: NodeOutput + Send + Sync,
    F: StableFn<Input::Args, Output = R> + Send + Sync,
    R: IntoNodeResult<Output::Output, E>,
    E: From<PondError>,
{
    fn call(&self, on_event: &mut dyn FnMut(&DatasetRef<'_>, DatasetEvent<'_>) -> Result<HookControl, HookAbort>) -> Result<(), E> {
        let args = self.input.load_data(on_event).map_err(E::from)?;
        let result = StableFn::call(&self.func, args);
        let output = result.into_node_result()?;
        self.output.save_data(output, on_event).map_err(E::from)?;
        Ok(())
    }
}

// Deliberately *not* `#[diagnostic::do_not_recommend]`: suppressing this impl
// replaces the precise `IntoNodeResult` / `From<PondError>` diagnostics with a
// generic "`Node<...>` is not a pipeline step" that spells out the whole closure
// type. The chain through this impl is what makes those messages readable.
impl<F, Input, Output, E, R> Step<E> for Node<F, Input, Output>
where
    Input: NodeInput + Send + Sync,
    Output: NodeOutput + Send + Sync,
    F: StableFn<Input::Args, Output = R> + Send + Sync,
    R: IntoNodeResult<Output::Output, E>,
    E: From<PondError>,
{
    fn kind(&self) -> StepKind<'_, E> { StepKind::Leaf(self) }
}
