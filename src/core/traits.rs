//! Core traits for pipeline items and data flow.

use core::marker::Tuple;

use crate::datasets::{Dataset, DatasetMeta};
use crate::error::PondError;

/// Convert a reference to a unique ID based on its pointer address.
/// Uses the data pointer only (ignores vtable for trait objects).
pub fn ptr_to_id<T: ?Sized>(r: &T) -> usize {
    r as *const T as *const () as usize
}

/// A reference to a dataset, carrying its pointer ID and object-safe metadata.
pub struct DatasetRef<'a> {
    pub id: usize,
    pub meta: &'a dyn DatasetMeta,
}

impl<'a> DatasetRef<'a> {
    pub fn new<T: Dataset + Send + Sync>(ds: &'a T) -> Self {
        Self {
            id: ptr_to_id(ds),
            meta: ds,
        }
    }
}

impl core::fmt::Debug for DatasetRef<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("DatasetRef")
            .field("id", &self.id)
            .field("is_param", &self.meta.is_param())
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

/// Dataset metadata passed to hooks, with an optionally resolved name.
#[derive(Debug, Clone)]
pub struct DatasetInfo<'a> {
    pub id: usize,
    pub is_param: bool,
    pub name: Option<&'a str>,
}

/// Non-generic metadata trait -- used by hooks, graph building, object-safe.
pub trait PipelineInfo: Send + Sync {
    fn get_name(&self) -> &'static str;
    fn is_leaf(&self) -> bool;
    fn get_type_string(&self) -> &'static str;
    fn for_each_child<'a>(&'a self, f: &mut dyn FnMut(&'a dyn PipelineInfo));
    fn for_each_input_id<'s>(&'s self, f: &mut dyn FnMut(&DatasetRef<'s>));
    fn for_each_output_id<'s>(&'s self, f: &mut dyn FnMut(&DatasetRef<'s>));
}

/// Generic execution trait -- parameterized by error type E.
pub trait PipelineItem<E>: PipelineInfo {
    fn call(&self, on_event: &mut dyn FnMut(&DatasetRef<'_>, DatasetEvent)) -> Result<(), E>;
    fn for_each_child_item<'a>(&'a self, f: &mut dyn FnMut(&'a dyn PipelineItem<E>));
}

/// Trait for loading data from input datasets.
pub trait NodeInput: Tuple {
    type Args: Tuple;
    fn load_data(&self, on_event: &mut dyn FnMut(&DatasetRef<'_>, DatasetEvent)) -> Result<Self::Args, PondError>;
    fn for_each_input_id<'s>(&'s self, f: &mut dyn FnMut(&DatasetRef<'s>));
}

impl NodeInput for () {
    type Args = ();
    fn load_data(&self, _on_event: &mut dyn FnMut(&DatasetRef<'_>, DatasetEvent)) -> Result<Self::Args, PondError> {
        Ok(())
    }
    fn for_each_input_id<'s>(&'s self, _f: &mut dyn FnMut(&DatasetRef<'s>)) {}
}

impl<T: Dataset + Send + Sync> NodeInput for (&T,)
where
    PondError: From<T::Error>,
{
    type Args = (T::LoadItem,);
    fn load_data(&self, on_event: &mut dyn FnMut(&DatasetRef<'_>, DatasetEvent)) -> Result<Self::Args, PondError> {
        let ds = DatasetRef::new(self.0);
        on_event(&ds, DatasetEvent::BeforeLoad);
        let val = self.0.load()?;
        on_event(&ds, DatasetEvent::AfterLoad);
        Ok((val,))
    }
    fn for_each_input_id<'s>(&'s self, f: &mut dyn FnMut(&DatasetRef<'s>)) {
        f(&DatasetRef::new(self.0));
    }
}

impl<T1: Dataset + Send + Sync, T2: Dataset + Send + Sync> NodeInput for (&T1, &T2)
where
    PondError: From<T1::Error>,
    PondError: From<T2::Error>,
{
    type Args = (T1::LoadItem, T2::LoadItem);
    fn load_data(&self, on_event: &mut dyn FnMut(&DatasetRef<'_>, DatasetEvent)) -> Result<Self::Args, PondError> {
        let ds0 = DatasetRef::new(self.0);
        on_event(&ds0, DatasetEvent::BeforeLoad);
        let val0 = self.0.load()?;
        on_event(&ds0, DatasetEvent::AfterLoad);
        let ds1 = DatasetRef::new(self.1);
        on_event(&ds1, DatasetEvent::BeforeLoad);
        let val1 = self.1.load()?;
        on_event(&ds1, DatasetEvent::AfterLoad);
        Ok((val0, val1))
    }
    fn for_each_input_id<'s>(&'s self, f: &mut dyn FnMut(&DatasetRef<'s>)) {
        f(&DatasetRef::new(self.0));
        f(&DatasetRef::new(self.1));
    }
}

impl<T1: Dataset + Send + Sync, T2: Dataset + Send + Sync, T3: Dataset + Send + Sync> NodeInput for (&T1, &T2, &T3)
where
    PondError: From<T1::Error>,
    PondError: From<T2::Error>,
    PondError: From<T3::Error>,
{
    type Args = (T1::LoadItem, T2::LoadItem, T3::LoadItem);
    fn load_data(&self, on_event: &mut dyn FnMut(&DatasetRef<'_>, DatasetEvent)) -> Result<Self::Args, PondError> {
        let ds0 = DatasetRef::new(self.0);
        on_event(&ds0, DatasetEvent::BeforeLoad);
        let val0 = self.0.load()?;
        on_event(&ds0, DatasetEvent::AfterLoad);
        let ds1 = DatasetRef::new(self.1);
        on_event(&ds1, DatasetEvent::BeforeLoad);
        let val1 = self.1.load()?;
        on_event(&ds1, DatasetEvent::AfterLoad);
        let ds2 = DatasetRef::new(self.2);
        on_event(&ds2, DatasetEvent::BeforeLoad);
        let val2 = self.2.load()?;
        on_event(&ds2, DatasetEvent::AfterLoad);
        Ok((val0, val1, val2))
    }
    fn for_each_input_id<'s>(&'s self, f: &mut dyn FnMut(&DatasetRef<'s>)) {
        f(&DatasetRef::new(self.0));
        f(&DatasetRef::new(self.1));
        f(&DatasetRef::new(self.2));
    }
}

/// Trait for saving data to output datasets.
pub trait NodeOutput: Tuple {
    type Output: Tuple;
    fn save_data(&self, output: Self::Output, on_event: &mut dyn FnMut(&DatasetRef<'_>, DatasetEvent)) -> Result<(), PondError>;
    fn for_each_output_id<'s>(&'s self, f: &mut dyn FnMut(&DatasetRef<'s>));
}

impl NodeOutput for () {
    type Output = ();
    fn save_data(&self, _output: Self::Output, _on_event: &mut dyn FnMut(&DatasetRef<'_>, DatasetEvent)) -> Result<(), PondError> {
        Ok(())
    }
    fn for_each_output_id<'s>(&'s self, _f: &mut dyn FnMut(&DatasetRef<'s>)) {}
}

impl<T: Dataset + Send + Sync> NodeOutput for (&T,)
where
    PondError: From<T::Error>,
{
    type Output = (T::SaveItem,);
    fn save_data(&self, output: Self::Output, on_event: &mut dyn FnMut(&DatasetRef<'_>, DatasetEvent)) -> Result<(), PondError> {
        let ds = DatasetRef::new(self.0);
        on_event(&ds, DatasetEvent::BeforeSave);
        self.0.save(output.0)?;
        on_event(&ds, DatasetEvent::AfterSave);
        Ok(())
    }
    fn for_each_output_id<'s>(&'s self, f: &mut dyn FnMut(&DatasetRef<'s>)) {
        f(&DatasetRef::new(self.0));
    }
}

impl<T1: Dataset + Send + Sync, T2: Dataset + Send + Sync> NodeOutput for (&T1, &T2)
where
    PondError: From<T1::Error>,
    PondError: From<T2::Error>,
{
    type Output = (T1::SaveItem, T2::SaveItem);
    fn save_data(&self, output: Self::Output, on_event: &mut dyn FnMut(&DatasetRef<'_>, DatasetEvent)) -> Result<(), PondError> {
        let ds0 = DatasetRef::new(self.0);
        on_event(&ds0, DatasetEvent::BeforeSave);
        self.0.save(output.0)?;
        on_event(&ds0, DatasetEvent::AfterSave);
        let ds1 = DatasetRef::new(self.1);
        on_event(&ds1, DatasetEvent::BeforeSave);
        self.1.save(output.1)?;
        on_event(&ds1, DatasetEvent::AfterSave);
        Ok(())
    }
    fn for_each_output_id<'s>(&'s self, f: &mut dyn FnMut(&DatasetRef<'s>)) {
        f(&DatasetRef::new(self.0));
        f(&DatasetRef::new(self.1));
    }
}

impl<T1: Dataset + Send + Sync, T2: Dataset + Send + Sync, T3: Dataset + Send + Sync> NodeOutput for (&T1, &T2, &T3)
where
    PondError: From<T1::Error>,
    PondError: From<T2::Error>,
    PondError: From<T3::Error>,
{
    type Output = (T1::SaveItem, T2::SaveItem, T3::SaveItem);
    fn save_data(&self, output: Self::Output, on_event: &mut dyn FnMut(&DatasetRef<'_>, DatasetEvent)) -> Result<(), PondError> {
        let ds0 = DatasetRef::new(self.0);
        on_event(&ds0, DatasetEvent::BeforeSave);
        self.0.save(output.0)?;
        on_event(&ds0, DatasetEvent::AfterSave);
        let ds1 = DatasetRef::new(self.1);
        on_event(&ds1, DatasetEvent::BeforeSave);
        self.1.save(output.1)?;
        on_event(&ds1, DatasetEvent::AfterSave);
        let ds2 = DatasetRef::new(self.2);
        on_event(&ds2, DatasetEvent::BeforeSave);
        self.2.save(output.2)?;
        on_event(&ds2, DatasetEvent::AfterSave);
        Ok(())
    }
    fn for_each_output_id<'s>(&'s self, f: &mut dyn FnMut(&DatasetRef<'s>)) {
        f(&DatasetRef::new(self.0));
        f(&DatasetRef::new(self.1));
        f(&DatasetRef::new(self.2));
    }
}
