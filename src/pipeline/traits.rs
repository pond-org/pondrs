//! Core traits for pipeline items and data flow.

use crate::datasets::{Dataset, DatasetMeta};

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
#[derive(Debug, Clone, Copy)]
pub enum DatasetEvent {
    BeforeLoad,
    AfterLoad,
    BeforeSave,
    AfterSave,
}

/// Non-generic, object-safe metadata for pipeline items.
///
/// Used by hooks, graph building, and validation. Leaf items are nodes;
/// non-leaf items are pipelines (containers with children).
pub trait PipelineInfo: Send + Sync {
    /// Human-readable name for this item.
    fn name(&self) -> &'static str;
    /// `true` for nodes, `false` for pipelines.
    fn is_leaf(&self) -> bool;
    /// The Rust type name of the underlying function or `"pipeline"`.
    fn type_string(&self) -> &'static str;
    /// Iterate over child items (empty for leaf nodes).
    fn for_each_child<'a>(&'a self, f: &mut dyn FnMut(&'a dyn PipelineInfo));
    /// Iterate over input dataset references.
    fn for_each_input<'s>(&'s self, f: &mut dyn FnMut(&DatasetRef<'s>));
    /// Iterate over output dataset references.
    fn for_each_output<'s>(&'s self, f: &mut dyn FnMut(&DatasetRef<'s>));
}

/// Generic execution trait, parameterized by the pipeline error type `E`.
pub trait RunnableStep<E>: PipelineInfo {
    /// Execute this item, firing dataset events via the callback.
    fn call(&self, on_event: &mut dyn FnMut(&DatasetRef<'_>, DatasetEvent)) -> Result<(), E>;
    /// Iterate over child steps (empty for leaf nodes).
    fn for_each_child_step<'a>(&'a self, f: &mut dyn FnMut(&'a dyn RunnableStep<E>));

    /// Upcast to `&dyn PipelineInfo`.
    ///
    /// Rust 1.85 does not support automatic trait-object upcasting, so this
    /// method is required to obtain a `&dyn PipelineInfo` from a
    /// `&dyn RunnableStep<E>`. Implement as `fn as_pipeline_info(&self) -> &dyn PipelineInfo { self }`.
    fn as_pipeline_info(&self) -> &dyn PipelineInfo;

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

impl<T: PipelineInfo + ?Sized> PipelineInfo for &T {
    fn name(&self) -> &'static str { (**self).name() }
    fn is_leaf(&self) -> bool { (**self).is_leaf() }
    fn type_string(&self) -> &'static str { (**self).type_string() }
    fn for_each_child<'a>(&'a self, f: &mut dyn FnMut(&'a dyn PipelineInfo)) {
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
    fn call(&self, on_event: &mut dyn FnMut(&DatasetRef<'_>, DatasetEvent)) -> Result<(), E> {
        (**self).call(on_event)
    }
    fn for_each_child_step<'a>(&'a self, f: &mut dyn FnMut(&'a dyn RunnableStep<E>)) {
        (**self).for_each_child_step(f);
    }
    fn as_pipeline_info(&self) -> &dyn PipelineInfo { (**self).as_pipeline_info() }
}

/// Trait for loading data from input datasets.
pub trait NodeInput: StableTuple {
    type Args: StableTuple;
    fn load_data(&self, on_event: &mut dyn FnMut(&DatasetRef<'_>, DatasetEvent)) -> Result<Self::Args, PondError>;
    fn for_each_input<'s>(&'s self, f: &mut dyn FnMut(&DatasetRef<'s>));
}

impl NodeInput for () {
    type Args = ();
    fn load_data(&self, _on_event: &mut dyn FnMut(&DatasetRef<'_>, DatasetEvent)) -> Result<Self::Args, PondError> {
        Ok(())
    }
    fn for_each_input<'s>(&'s self, _f: &mut dyn FnMut(&DatasetRef<'s>)) {}
}

macro_rules! impl_node_input {
    ($($T:ident $idx:tt),+) => {
        impl<$($T: Dataset + Send + Sync),+> NodeInput for ($(&$T,)+)
        where
            $(PondError: From<$T::Error>,)+
        {
            type Args = ($($T::LoadItem,)+);
            #[allow(non_snake_case)]
            fn load_data(&self, on_event: &mut dyn FnMut(&DatasetRef<'_>, DatasetEvent)) -> Result<Self::Args, PondError> {
                $(
                    let ds = DatasetRef::from_ref(self.$idx);
                    on_event(&ds, DatasetEvent::BeforeLoad);
                    let $T = self.$idx.load()?;
                    on_event(&ds, DatasetEvent::AfterLoad);
                )+
                Ok(($($T,)+))
            }
            fn for_each_input<'s>(&'s self, f: &mut dyn FnMut(&DatasetRef<'s>)) {
                $(f(&DatasetRef::from_ref(self.$idx));)+
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
    fn save_data(&self, output: Self::Output, on_event: &mut dyn FnMut(&DatasetRef<'_>, DatasetEvent)) -> Result<(), PondError>;
    fn for_each_output<'s>(&'s self, f: &mut dyn FnMut(&DatasetRef<'s>));
}

impl NodeOutput for () {
    type Output = ();
    fn save_data(&self, _output: Self::Output, _on_event: &mut dyn FnMut(&DatasetRef<'_>, DatasetEvent)) -> Result<(), PondError> {
        Ok(())
    }
    fn for_each_output<'s>(&'s self, _f: &mut dyn FnMut(&DatasetRef<'s>)) {}
}

macro_rules! impl_node_output {
    ($($T:ident $idx:tt),+) => {
        impl<$($T: Dataset + Send + Sync),+> NodeOutput for ($(&$T,)+)
        where
            $(PondError: From<$T::Error>,)+
        {
            type Output = ($($T::SaveItem,)+);
            fn save_data(&self, output: Self::Output, on_event: &mut dyn FnMut(&DatasetRef<'_>, DatasetEvent)) -> Result<(), PondError> {
                $({
                    let ds = DatasetRef::from_ref(self.$idx);
                    on_event(&ds, DatasetEvent::BeforeSave);
                    self.$idx.save(output.$idx)?;
                    on_event(&ds, DatasetEvent::AfterSave);
                })+
                Ok(())
            }
            fn for_each_output<'s>(&'s self, f: &mut dyn FnMut(&DatasetRef<'s>)) {
                $(f(&DatasetRef::from_ref(self.$idx));)+
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
