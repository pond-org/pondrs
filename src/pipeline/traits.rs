//! Core traits for pipeline items and data flow.

use crate::datasets::{Dataset, DatasetMeta};
use crate::hooks::{HookAbort, HookControl};

use super::stable::StableTuple;
use crate::error::PondError;

/// Convert a reference to a unique ID based on its pointer address.
/// Uses the data pointer only (ignores vtable for trait objects).
pub(crate) fn ptr_to_id<T: ?Sized>(r: &T) -> usize {
    r as *const T as *const () as usize
}

/// A reference to a dataset, carrying its pointer ID, object-safe metadata,
/// and an optionally resolved human-readable name.
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

impl Clone for DatasetRef<'_> {
    fn clone(&self) -> Self { *self }
}

impl Copy for DatasetRef<'_> {}

/// Events fired during dataset load/save operations.
pub enum DatasetEvent<'v> {
    BeforeLoad,
    AfterLoad(&'v dyn core::any::Any),
    BeforeSave(&'v dyn core::any::Any),
    AfterSave,
}

/// Non-generic, object-safe metadata for pipeline items.
///
/// Used by hooks, graph building, and validation. Leaf items are nodes;
/// non-leaf items are pipelines (containers with children).
pub trait StepInfo: Send + Sync {
    /// Human-readable name for this item.
    fn name(&self) -> &'static str;
    /// `true` for nodes, `false` for pipelines.
    fn is_leaf(&self) -> bool;
    /// The Rust type name of the underlying function or `"pipeline"`.
    fn type_string(&self) -> &'static str;
    /// Iterate over child items (empty for leaf nodes).
    fn for_each_child<'a>(&'a self, f: &mut dyn FnMut(&'a dyn StepInfo));
    /// Iterate over input dataset references.
    fn for_each_input<'s>(&'s self, f: &mut dyn FnMut(&DatasetRef<'s>));
    /// Iterate over output dataset references.
    fn for_each_output<'s>(&'s self, f: &mut dyn FnMut(&DatasetRef<'s>));
}

/// Executable leaf step (node). Has a `call()` method for actual computation.
pub trait LeafStep<E>: StepInfo {
    fn call(&self, on_event: &mut dyn FnMut(&DatasetRef<'_>, DatasetEvent<'_>) -> Result<HookControl, HookAbort>) -> Result<(), E>;
}

/// Container step (pipeline). Has children that are themselves `RunnableStep`s.
pub trait GroupStep<E>: StepInfo {
    fn for_each_child_step<'a>(&'a self, f: &mut dyn FnMut(&'a dyn RunnableStep<E>));
}

/// Discriminated union of leaf and group steps.
pub enum StepKind<'a, E> {
    Leaf(&'a dyn LeafStep<E>),
    Group(&'a dyn GroupStep<E>),
}

/// Generic execution trait, parameterized by the pipeline error type `E`.
///
/// Implementors are either leaves ([`LeafStep`]) or groups ([`GroupStep`]).
/// Use [`kind()`](RunnableStep::kind) to match and access the appropriate interface.
pub trait RunnableStep<E>: StepInfo {
    /// Returns whether this step is a leaf or a group, with access to the
    /// appropriate trait object for calling `call()` or iterating children.
    fn kind(&self) -> StepKind<'_, E>;

    /// Upcast to `&dyn StepInfo`.
    fn as_pipeline_info(&self) -> &dyn StepInfo;

    /// Box this step for use in a [`StepVec`](crate::StepVec).
    #[cfg(feature = "std")]
    fn boxed<'a>(self) -> std::boxed::Box<dyn RunnableStep<E> + Send + Sync + 'a>
    where
        Self: Sized + Send + Sync + 'a,
    {
        std::boxed::Box::new(self)
    }
}

// --- Blanket impls for references ---
// These allow `&'a dyn RunnableStep<E>` to be boxed into a `StepVec<'a, E>` directly.

impl<T: StepInfo + ?Sized> StepInfo for &T {
    fn name(&self) -> &'static str { (**self).name() }
    fn is_leaf(&self) -> bool { (**self).is_leaf() }
    fn type_string(&self) -> &'static str { (**self).type_string() }
    fn for_each_child<'a>(&'a self, f: &mut dyn FnMut(&'a dyn StepInfo)) {
        (**self).for_each_child(f);
    }
    fn for_each_input<'s>(&'s self, f: &mut dyn FnMut(&DatasetRef<'s>)) {
        (**self).for_each_input(f);
    }
    fn for_each_output<'s>(&'s self, f: &mut dyn FnMut(&DatasetRef<'s>)) {
        (**self).for_each_output(f);
    }
}

impl<E, T: RunnableStep<E> + ?Sized> RunnableStep<E> for &T {
    fn kind(&self) -> StepKind<'_, E> { (**self).kind() }
    fn as_pipeline_info(&self) -> &dyn StepInfo { (**self).as_pipeline_info() }
}

/// A single input port in a node's input tuple.
///
/// Each element of a `NodeInput` tuple implements this trait. The blanket impl
/// for `&T where T: Dataset` covers plain dataset references; custom impls
/// (e.g. `EachField`) support fan-in patterns.
pub trait InputPort {
    type Item: 'static;
    fn load_port(&self, on_event: &mut dyn FnMut(&DatasetRef<'_>, DatasetEvent<'_>) -> Result<HookControl, HookAbort>) -> Result<Self::Item, PondError>;
    fn for_each_ref<'s>(&'s self, f: &mut dyn FnMut(&DatasetRef<'s>));
}

/// A single output port in a node's output tuple.
///
/// Each element of a `NodeOutput` tuple implements this trait. The blanket impl
/// for `&T where T: Dataset` covers plain dataset references; custom impls
/// (e.g. `EachField`) support fan-out patterns.
pub trait OutputPort {
    type Item: 'static;
    fn save_port(&self, value: Self::Item, on_event: &mut dyn FnMut(&DatasetRef<'_>, DatasetEvent<'_>) -> Result<HookControl, HookAbort>) -> Result<(), PondError>;
    fn for_each_ref<'s>(&'s self, f: &mut dyn FnMut(&DatasetRef<'s>));
}

impl<T: Dataset + Send + Sync> InputPort for &T
where
    PondError: From<T::Error>,
{
    type Item = T::LoadItem;
    fn load_port(&self, on_event: &mut dyn FnMut(&DatasetRef<'_>, DatasetEvent<'_>) -> Result<HookControl, HookAbort>) -> Result<Self::Item, PondError> {
        let ds = DatasetRef::from_ref(*self);
        on_event(&ds, DatasetEvent::BeforeLoad)?;
        let value = (*self).load()?;
        on_event(&ds, DatasetEvent::AfterLoad(&value))?;
        Ok(value)
    }
    fn for_each_ref<'s>(&'s self, f: &mut dyn FnMut(&DatasetRef<'s>)) {
        f(&DatasetRef::from_ref(*self));
    }
}

impl<T: Dataset + Send + Sync> OutputPort for &T
where
    PondError: From<T::Error>,
{
    type Item = T::SaveItem;
    fn save_port(&self, value: Self::Item, on_event: &mut dyn FnMut(&DatasetRef<'_>, DatasetEvent<'_>) -> Result<HookControl, HookAbort>) -> Result<(), PondError> {
        let ds = DatasetRef::from_ref(*self);
        let control = on_event(&ds, DatasetEvent::BeforeSave(&value))?;
        if control != HookControl::Skip {
            (*self).save(value)?;
            on_event(&ds, DatasetEvent::AfterSave)?;
        }
        Ok(())
    }
    fn for_each_ref<'s>(&'s self, f: &mut dyn FnMut(&DatasetRef<'s>)) {
        f(&DatasetRef::from_ref(*self));
    }
}

/// Trait for loading data from input datasets.
pub trait NodeInput: StableTuple {
    type Args: StableTuple;
    fn load_data(&self, on_event: &mut dyn FnMut(&DatasetRef<'_>, DatasetEvent<'_>) -> Result<HookControl, HookAbort>) -> Result<Self::Args, PondError>;
    fn for_each_input<'s>(&'s self, f: &mut dyn FnMut(&DatasetRef<'s>));
}

impl NodeInput for () {
    type Args = ();
    fn load_data(&self, _on_event: &mut dyn FnMut(&DatasetRef<'_>, DatasetEvent<'_>) -> Result<HookControl, HookAbort>) -> Result<Self::Args, PondError> {
        Ok(())
    }
    fn for_each_input<'s>(&'s self, _f: &mut dyn FnMut(&DatasetRef<'s>)) {}
}

macro_rules! impl_node_input {
    ($($P:ident $idx:tt),+) => {
        impl<$($P: InputPort + Send + Sync),+> NodeInput for ($($P,)+) {
            type Args = ($($P::Item,)+);
            #[allow(non_snake_case)]
            fn load_data(&self, on_event: &mut dyn FnMut(&DatasetRef<'_>, DatasetEvent<'_>) -> Result<HookControl, HookAbort>) -> Result<Self::Args, PondError> {
                $(
                    let $P = self.$idx.load_port(on_event)?;
                )+
                Ok(($($P,)+))
            }
            fn for_each_input<'s>(&'s self, f: &mut dyn FnMut(&DatasetRef<'s>)) {
                $(self.$idx.for_each_ref(f);)+
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

/// Trait for saving data to output datasets.
pub trait NodeOutput: StableTuple {
    type Output: StableTuple;
    fn save_data(&self, output: Self::Output, on_event: &mut dyn FnMut(&DatasetRef<'_>, DatasetEvent<'_>) -> Result<HookControl, HookAbort>) -> Result<(), PondError>;
    fn for_each_output<'s>(&'s self, f: &mut dyn FnMut(&DatasetRef<'s>));
}

impl NodeOutput for () {
    type Output = ();
    fn save_data(&self, _output: Self::Output, _on_event: &mut dyn FnMut(&DatasetRef<'_>, DatasetEvent<'_>) -> Result<HookControl, HookAbort>) -> Result<(), PondError> {
        Ok(())
    }
    fn for_each_output<'s>(&'s self, _f: &mut dyn FnMut(&DatasetRef<'s>)) {}
}

macro_rules! impl_node_output {
    ($($P:ident $idx:tt),+) => {
        impl<$($P: OutputPort + Send + Sync),+> NodeOutput for ($($P,)+) {
            type Output = ($($P::Item,)+);
            fn save_data(&self, output: Self::Output, on_event: &mut dyn FnMut(&DatasetRef<'_>, DatasetEvent<'_>) -> Result<HookControl, HookAbort>) -> Result<(), PondError> {
                $({
                    self.$idx.save_port(output.$idx, on_event)?;
                })+
                Ok(())
            }
            fn for_each_output<'s>(&'s self, f: &mut dyn FnMut(&DatasetRef<'s>)) {
                $(self.$idx.for_each_ref(f);)+
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
