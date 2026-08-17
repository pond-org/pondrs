//! Core traits for pipeline steps and data flow.

use crate::datasets::{Dataset, DatasetMeta};
use crate::hooks::{HookAbort, HookControl};

use super::stable::StableTuple;
use crate::error::PondError;

/// Convert a reference to a unique ID based on its pointer address.
/// Uses the data pointer only (ignores vtable for trait objects).
pub(crate) fn ptr_to_id<T: ?Sized>(r: &T) -> usize {
    core::ptr::from_ref::<T>(r).cast::<()>() as usize
}

/// A reference to a dataset, carrying its pointer ID, object-safe metadata,
/// and an optionally resolved human-readable name.
#[derive(Clone, Copy)]
pub struct DatasetRef<'a> {
    pub id: usize,
    pub meta: &'a dyn DatasetMeta,
    pub name: Option<&'a str>,
}

impl<'a> DatasetRef<'a> {
    pub fn from_ref<T: Dataset + Send + Sync>(ds: &'a T) -> Self {
        Self {
            id: ptr_to_id(ds),
            meta: ds,
            name: None,
        }
    }
}

impl core::fmt::Debug for DatasetRef<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("DatasetRef")
            .field("id", &self.id)
            .field("is_param", &self.meta.is_param())
            .field("name", &self.name)
            .finish()
    }
}

/// Events fired during dataset load/save operations.
pub enum DatasetEvent<'v> {
    BeforeLoad,
    AfterLoad(&'v dyn core::any::Any),
    BeforeSave(&'v dyn core::any::Any),
    AfterSave,
}

/// Non-generic, object-safe metadata for a single pipeline step.
///
/// The metadata companion to [`Step`], mirroring the [`Dataset`]/[`DatasetMeta`]
/// split: `StepMeta` carries everything hooks, graph building, and validation
/// need without knowing the pipeline error type. Leaf steps are nodes; non-leaf
/// steps are pipelines (containers with children).
pub trait StepMeta: Send + Sync {
    /// Human-readable name for this step.
    fn name(&self) -> &'static str;
    /// `true` for nodes, `false` for pipelines.
    fn is_leaf(&self) -> bool;
    /// The Rust type name of the underlying function or `"pipeline"`.
    fn type_string(&self) -> &'static str;
    /// Iterate over child steps (empty for leaf nodes).
    fn for_each_child<'a>(&'a self, f: &mut dyn FnMut(&'a dyn StepMeta));
    /// Iterate over input dataset references.
    fn for_each_input<'s>(&'s self, f: &mut dyn FnMut(&DatasetRef<'s>));
    /// Iterate over output dataset references.
    fn for_each_output<'s>(&'s self, f: &mut dyn FnMut(&DatasetRef<'s>));
}

/// Executable leaf step (node). Has a `call()` method for actual computation.
pub trait Leaf<E>: StepMeta {
    fn call(&self, on_event: &mut dyn FnMut(&DatasetRef<'_>, DatasetEvent<'_>) -> Result<HookControl, HookAbort>) -> Result<(), E>;
}

/// Container step (pipeline). Has children that are themselves `Step`s.
pub trait Group<E>: StepMeta {
    fn for_each_child_step<'a>(&'a self, f: &mut dyn FnMut(&'a dyn Step<E>));
}

/// Discriminated union of leaf and group steps.
pub enum StepKind<'a, E> {
    Leaf(&'a dyn Leaf<E>),
    Group(&'a dyn Group<E>),
}

/// Generic execution trait, parameterized by the pipeline error type `E`.
///
/// Implementors are either leaves ([`Leaf`]) or groups ([`Group`]).
/// Use [`kind()`](Step::kind) to match and access the appropriate interface.
#[diagnostic::on_unimplemented(
    message = "`{Self}` is not a pipeline step",
    label = "not a step",
    note = "steps are `Node`, `Pipeline`, `Alias`, `PartitionedNode`, or a boxed step in a `DynSteps`",
    note = "if `{Self}` is a step, check that the pipeline error type `{E}` implements `From<PondError>`"
)]
pub trait Step<E>: StepMeta {
    /// Returns whether this step is a leaf or a group, with access to the
    /// appropriate trait object for calling `call()` or iterating children.
    fn kind(&self) -> StepKind<'_, E>;

    /// Box this step for use in a [`DynSteps`](crate::DynSteps).
    #[cfg(feature = "std")]
    fn boxed<'a>(self) -> std::boxed::Box<dyn Step<E> + Send + Sync + 'a>
    where
        Self: Sized + Send + Sync + 'a,
    {
        std::boxed::Box::new(self)
    }
}

// --- Blanket impls for references ---
// These allow `&'a dyn Step<E>` to be boxed into a `DynSteps<'a, E>` directly.

impl<T: StepMeta + ?Sized> StepMeta for &T {
    fn name(&self) -> &'static str { (**self).name() }
    fn is_leaf(&self) -> bool { (**self).is_leaf() }
    fn type_string(&self) -> &'static str { (**self).type_string() }
    fn for_each_child<'a>(&'a self, f: &mut dyn FnMut(&'a dyn StepMeta)) {
        (**self).for_each_child(f);
    }
    fn for_each_input<'s>(&'s self, f: &mut dyn FnMut(&DatasetRef<'s>)) {
        (**self).for_each_input(f);
    }
    fn for_each_output<'s>(&'s self, f: &mut dyn FnMut(&DatasetRef<'s>)) {
        (**self).for_each_output(f);
    }
}

impl<E, T: Step<E> + ?Sized> Step<E> for &T {
    fn kind(&self) -> StepKind<'_, E> { (**self).kind() }
}

/// A single input slot in a node's input tuple — a generalized [`Dataset`] for loading.
///
/// The blanket impl for `&T where T: Dataset` covers plain dataset references.
/// Custom impls (e.g. [`EachField`](super::EachField)) support fan-in patterns
/// (loading from many datasets into one value).
///
/// `load_input` is generic over the pipeline error type rather than returning
/// [`PondError`]: a slot's [`Error`](Self::Error) converts straight into `E`, so
/// a custom dataset error reaches the user's error enum with its type and
/// `source()` chain intact. `E: From<PondError>` remains as the floor for
/// framework errors (hook aborts, and adapter errors such as
/// [`PondError::KeyMismatch`]) — a bound every pipeline already owes the runner.
#[diagnostic::on_unimplemented(
    message = "`{Self}` cannot be used as a node input",
    label = "not a node input",
    note = "input tuples hold dataset *references*: write `(&catalog.field,)`, not `(catalog.field,)`",
    note = "adapters such as `EachField` are also valid input slots"
)]
pub trait DatasetInput {
    type Item: 'static;
    /// The error this slot can raise — for a plain dataset reference, the
    /// dataset's own `Dataset::Error`.
    type Error;
    fn load_input<E>(&self, on_event: &mut dyn FnMut(&DatasetRef<'_>, DatasetEvent<'_>) -> Result<HookControl, HookAbort>) -> Result<Self::Item, E>
    where
        E: From<Self::Error> + From<PondError>;
    fn for_each_ref<'s>(&'s self, f: &mut dyn FnMut(&DatasetRef<'s>));
}

/// A single output slot in a node's output tuple — a generalized [`Dataset`] for saving.
///
/// The blanket impl for `&T where T: Dataset` covers plain dataset references.
/// Custom impls (e.g. [`EachField`](super::EachField)) support fan-out patterns
/// (distributing one value across many datasets).
///
/// See [`DatasetInput`] for why `save_output` is generic over `E`.
#[diagnostic::on_unimplemented(
    message = "`{Self}` cannot be used as a node output",
    label = "not a node output",
    note = "output tuples hold dataset *references*: write `(&catalog.field,)`, not `(catalog.field,)`",
    note = "adapters such as `EachField` are also valid output slots"
)]
pub trait DatasetOutput {
    type Item: 'static;
    /// The error this slot can raise — for a plain dataset reference, the
    /// dataset's own `Dataset::Error`.
    type Error;
    fn save_output<E>(&self, value: Self::Item, on_event: &mut dyn FnMut(&DatasetRef<'_>, DatasetEvent<'_>) -> Result<HookControl, HookAbort>) -> Result<(), E>
    where
        E: From<Self::Error> + From<PondError>;
    fn for_each_ref<'s>(&'s self, f: &mut dyn FnMut(&DatasetRef<'s>));
}

// Deliberately *not* `#[diagnostic::do_not_recommend]`: suppressing this impl
// makes a dataset used in a node input report the `DatasetInput` message
// instead, which tells the user to pass a reference when they already did.
impl<T: Dataset + Send + Sync> DatasetInput for &T {
    type Item = T::LoadItem;
    type Error = T::Error;
    fn load_input<E>(&self, on_event: &mut dyn FnMut(&DatasetRef<'_>, DatasetEvent<'_>) -> Result<HookControl, HookAbort>) -> Result<Self::Item, E>
    where
        E: From<Self::Error> + From<PondError>,
    {
        let ds = DatasetRef::from_ref(*self);
        on_event(&ds, DatasetEvent::BeforeLoad).map_err(|e| E::from(PondError::from(e)))?;
        let value = (*self).load().map_err(E::from)?;
        on_event(&ds, DatasetEvent::AfterLoad(&value)).map_err(|e| E::from(PondError::from(e)))?;
        Ok(value)
    }
    fn for_each_ref<'s>(&'s self, f: &mut dyn FnMut(&DatasetRef<'s>)) {
        f(&DatasetRef::from_ref(*self));
    }
}

// See the note on the `DatasetInput` impl above.
impl<T: Dataset + Send + Sync> DatasetOutput for &T {
    type Item = T::SaveItem;
    type Error = T::Error;
    fn save_output<E>(&self, value: Self::Item, on_event: &mut dyn FnMut(&DatasetRef<'_>, DatasetEvent<'_>) -> Result<HookControl, HookAbort>) -> Result<(), E>
    where
        E: From<Self::Error> + From<PondError>,
    {
        let ds = DatasetRef::from_ref(*self);
        let control = on_event(&ds, DatasetEvent::BeforeSave(&value))
            .map_err(|e| E::from(PondError::from(e)))?;
        if control != HookControl::Skip {
            (*self).save(value).map_err(E::from)?;
            on_event(&ds, DatasetEvent::AfterSave).map_err(|e| E::from(PondError::from(e)))?;
        }
        Ok(())
    }
    fn for_each_ref<'s>(&'s self, f: &mut dyn FnMut(&DatasetRef<'s>)) {
        f(&DatasetRef::from_ref(*self));
    }
}

/// Non-generic metadata for a node's `input` tuple: what it loads, and which
/// datasets it names.
///
/// The metadata companion to [`NodeInput`], mirroring the [`StepMeta`]/[`Step`]
/// split. [`Node`](super::Node) bounds its `Input` on this trait so that
/// [`Args`](Self::Args) — and therefore the closure's expected signature —
/// resolves at the `Node { .. }` literal, before the pipeline error type is
/// known.
#[diagnostic::on_unimplemented(
    message = "`{Self}` is not a valid `input` for a node",
    label = "not an input tuple",
    note = "`input` is a tuple of dataset references, e.g. `(&cat.raw, &params.factor)`, or `()` for no inputs"
)]
pub trait NodeInputMeta: StableTuple {
    type Args: StableTuple;
    fn for_each_input<'s>(&'s self, f: &mut dyn FnMut(&DatasetRef<'s>));
}

/// Loads a node's `input` tuple into its `Args`, converting each slot's error
/// into the pipeline error type `E`.
#[diagnostic::on_unimplemented(
    message = "`{Self}` cannot be loaded into the pipeline error type `{E}`",
    label = "input errors do not convert into `{E}`",
    note = "`{E}` must implement `From<..>` for the `Error` of every input dataset, plus `From<PondError>`",
    note = "the usual fix is a `#[derive(thiserror::Error)]` enum with an `#[error(transparent)] #[from]` variant per dataset error type"
)]
pub trait NodeInput<E>: NodeInputMeta {
    fn load_data(&self, on_event: &mut dyn FnMut(&DatasetRef<'_>, DatasetEvent<'_>) -> Result<HookControl, HookAbort>) -> Result<Self::Args, E>;
}

impl NodeInputMeta for () {
    type Args = ();
    fn for_each_input<'s>(&'s self, _f: &mut dyn FnMut(&DatasetRef<'s>)) {}
}

impl<E> NodeInput<E> for () {
    fn load_data(&self, _on_event: &mut dyn FnMut(&DatasetRef<'_>, DatasetEvent<'_>) -> Result<HookControl, HookAbort>) -> Result<Self::Args, E> {
        Ok(())
    }
}

macro_rules! impl_node_input {
    ($($P:ident $idx:tt),+) => {
        impl<$($P: DatasetInput + Send + Sync),+> NodeInputMeta for ($($P,)+) {
            type Args = ($($P::Item,)+);
            fn for_each_input<'s>(&'s self, f: &mut dyn FnMut(&DatasetRef<'s>)) {
                $(self.$idx.for_each_ref(f);)+
            }
        }

        impl<E, $($P: DatasetInput + Send + Sync),+> NodeInput<E> for ($($P,)+)
        where
            E: $(From<$P::Error> +)+ From<PondError>,
        {
            #[allow(non_snake_case)]
            fn load_data(&self, on_event: &mut dyn FnMut(&DatasetRef<'_>, DatasetEvent<'_>) -> Result<HookControl, HookAbort>) -> Result<Self::Args, E> {
                $(
                    let $P = self.$idx.load_input::<E>(on_event)?;
                )+
                Ok(($($P,)+))
            }
        }
    };
}

impl_node_input!(T0 0);
impl_node_input!(T0 0, T1 1);
impl_node_input!(T0 0, T1 1, T2 2);
impl_node_input!(T0 0, T1 1, T2 2, T3 3);
impl_node_input!(T0 0, T1 1, T2 2, T3 3, T4 4);
impl_node_input!(T0 0, T1 1, T2 2, T3 3, T4 4, T5 5);
impl_node_input!(T0 0, T1 1, T2 2, T3 3, T4 4, T5 5, T6 6);
impl_node_input!(T0 0, T1 1, T2 2, T3 3, T4 4, T5 5, T6 6, T7 7);
impl_node_input!(T0 0, T1 1, T2 2, T3 3, T4 4, T5 5, T6 6, T7 7, T8 8);
impl_node_input!(T0 0, T1 1, T2 2, T3 3, T4 4, T5 5, T6 6, T7 7, T8 8, T9 9);

/// Non-generic metadata for a node's `output` tuple: what it saves, and which
/// datasets it names.
///
/// The metadata companion to [`NodeOutput`]; see [`NodeInputMeta`] for why the
/// split exists.
#[diagnostic::on_unimplemented(
    message = "`{Self}` is not a valid `output` for a node",
    label = "not an output tuple",
    note = "`output` is a tuple of dataset references, e.g. `(&cat.scaled,)`, or `()` for no outputs"
)]
pub trait NodeOutputMeta: StableTuple {
    type Output: StableTuple;
    fn for_each_output<'s>(&'s self, f: &mut dyn FnMut(&DatasetRef<'s>));
}

/// Saves a node's return value into its `output` tuple, converting each slot's
/// error into the pipeline error type `E`.
#[diagnostic::on_unimplemented(
    message = "`{Self}` cannot be saved into the pipeline error type `{E}`",
    label = "output errors do not convert into `{E}`",
    note = "`{E}` must implement `From<..>` for the `Error` of every output dataset, plus `From<PondError>`",
    note = "the usual fix is a `#[derive(thiserror::Error)]` enum with an `#[error(transparent)] #[from]` variant per dataset error type"
)]
pub trait NodeOutput<E>: NodeOutputMeta {
    fn save_data(&self, output: Self::Output, on_event: &mut dyn FnMut(&DatasetRef<'_>, DatasetEvent<'_>) -> Result<HookControl, HookAbort>) -> Result<(), E>;
}

impl NodeOutputMeta for () {
    type Output = ();
    fn for_each_output<'s>(&'s self, _f: &mut dyn FnMut(&DatasetRef<'s>)) {}
}

impl<E> NodeOutput<E> for () {
    fn save_data(&self, _output: Self::Output, _on_event: &mut dyn FnMut(&DatasetRef<'_>, DatasetEvent<'_>) -> Result<HookControl, HookAbort>) -> Result<(), E> {
        Ok(())
    }
}

macro_rules! impl_node_output {
    ($($P:ident $idx:tt),+) => {
        impl<$($P: DatasetOutput + Send + Sync),+> NodeOutputMeta for ($($P,)+) {
            type Output = ($($P::Item,)+);
            fn for_each_output<'s>(&'s self, f: &mut dyn FnMut(&DatasetRef<'s>)) {
                $(self.$idx.for_each_ref(f);)+
            }
        }

        impl<E, $($P: DatasetOutput + Send + Sync),+> NodeOutput<E> for ($($P,)+)
        where
            E: $(From<$P::Error> +)+ From<PondError>,
        {
            fn save_data(&self, output: Self::Output, on_event: &mut dyn FnMut(&DatasetRef<'_>, DatasetEvent<'_>) -> Result<HookControl, HookAbort>) -> Result<(), E> {
                $({
                    self.$idx.save_output::<E>(output.$idx, on_event)?;
                })+
                Ok(())
            }
        }
    };
}

impl_node_output!(T0 0);
impl_node_output!(T0 0, T1 1);
impl_node_output!(T0 0, T1 1, T2 2);
impl_node_output!(T0 0, T1 1, T2 2, T3 3);
impl_node_output!(T0 0, T1 1, T2 2, T3 3, T4 4);
impl_node_output!(T0 0, T1 1, T2 2, T3 3, T4 4, T5 5);
impl_node_output!(T0 0, T1 1, T2 2, T3 3, T4 4, T5 5, T6 6);
impl_node_output!(T0 0, T1 1, T2 2, T3 3, T4 4, T5 5, T6 6, T7 7);
impl_node_output!(T0 0, T1 1, T2 2, T3 3, T4 4, T5 5, T6 6, T7 7, T8 8);
impl_node_output!(T0 0, T1 1, T2 2, T3 3, T4 4, T5 5, T6 6, T7 7, T8 8, T9 9);
