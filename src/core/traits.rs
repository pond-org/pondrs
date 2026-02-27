//! Core traits for pipeline items and data flow.

use core::marker::Tuple;

use crate::datasets::Dataset;
use crate::error::PondError;

/// Convert a reference to a unique ID based on its pointer address.
/// Uses the data pointer only (ignores vtable for trait objects).
pub fn ptr_to_id<T: ?Sized>(r: &T) -> usize {
    r as *const T as *const () as usize
}

/// A reference to a dataset, carrying its pointer ID and whether it's a parameter.
#[derive(Debug, Clone)]
pub struct DatasetRef {
    pub id: usize,
    pub is_param: bool,
}

/// Non-generic metadata trait -- used by hooks, graph building, object-safe.
pub trait PipelineInfo: Send + Sync {
    fn get_name(&self) -> &'static str;
    fn is_leaf(&self) -> bool;
    fn for_each_child<'a>(&'a self, f: &mut dyn FnMut(&'a dyn PipelineInfo));
    fn for_each_input_id(&self, f: &mut dyn FnMut(&DatasetRef));
    fn for_each_output_id(&self, f: &mut dyn FnMut(&DatasetRef));
}

/// Generic execution trait -- parameterized by error type E.
pub trait PipelineItem<E>: PipelineInfo {
    fn call(&self) -> Result<(), E>;
    fn for_each_child_item<'a>(&'a self, f: &mut dyn FnMut(&'a dyn PipelineItem<E>));
}

/// Trait for loading data from input datasets.
pub trait NodeInput: Tuple {
    type Args: Tuple;
    fn load_data(&self) -> Result<Self::Args, PondError>;
    fn for_each_input_id(&self, f: &mut dyn FnMut(&DatasetRef));
}

impl NodeInput for () {
    type Args = ();
    fn load_data(&self) -> Result<Self::Args, PondError> {
        Ok(())
    }
    fn for_each_input_id(&self, _f: &mut dyn FnMut(&DatasetRef)) {}
}

impl<T: Dataset> NodeInput for (&T,)
where
    PondError: From<T::Error>,
{
    type Args = (T::LoadItem,);
    fn load_data(&self) -> Result<Self::Args, PondError> {
        Ok((self.0.load()?,))
    }
    fn for_each_input_id(&self, f: &mut dyn FnMut(&DatasetRef)) {
        f(&DatasetRef { id: ptr_to_id(self.0), is_param: self.0.is_param() });
    }
}

impl<T1: Dataset, T2: Dataset> NodeInput for (&T1, &T2)
where
    PondError: From<T1::Error>,
    PondError: From<T2::Error>,
{
    type Args = (T1::LoadItem, T2::LoadItem);
    fn load_data(&self) -> Result<Self::Args, PondError> {
        Ok((self.0.load()?, self.1.load()?))
    }
    fn for_each_input_id(&self, f: &mut dyn FnMut(&DatasetRef)) {
        f(&DatasetRef { id: ptr_to_id(self.0), is_param: self.0.is_param() });
        f(&DatasetRef { id: ptr_to_id(self.1), is_param: self.1.is_param() });
    }
}

/// Trait for saving data to output datasets.
pub trait NodeOutput: Tuple {
    type Output: Tuple;
    fn save_data(&self, output: Self::Output) -> Result<(), PondError>;
    fn for_each_output_id(&self, f: &mut dyn FnMut(&DatasetRef));
}

impl NodeOutput for () {
    type Output = ();
    fn save_data(&self, _output: Self::Output) -> Result<(), PondError> {
        Ok(())
    }
    fn for_each_output_id(&self, _f: &mut dyn FnMut(&DatasetRef)) {}
}

impl<T: Dataset> NodeOutput for (&T,)
where
    PondError: From<T::Error>,
{
    type Output = (T::SaveItem,);
    fn save_data(&self, output: Self::Output) -> Result<(), PondError> {
        self.0.save(output.0)?;
        Ok(())
    }
    fn for_each_output_id(&self, f: &mut dyn FnMut(&DatasetRef)) {
        f(&DatasetRef { id: ptr_to_id(self.0), is_param: self.0.is_param() });
    }
}

impl<T1: Dataset, T2: Dataset> NodeOutput for (&T1, &T2)
where
    PondError: From<T1::Error>,
    PondError: From<T2::Error>,
{
    type Output = (T1::SaveItem, T2::SaveItem);
    fn save_data(&self, output: Self::Output) -> Result<(), PondError> {
        self.0.save(output.0)?;
        self.1.save(output.1)?;
        Ok(())
    }
    fn for_each_output_id(&self, f: &mut dyn FnMut(&DatasetRef)) {
        f(&DatasetRef { id: ptr_to_id(self.0), is_param: self.0.is_param() });
        f(&DatasetRef { id: ptr_to_id(self.1), is_param: self.1.is_param() });
    }
}
