//! Core traits for pipeline items and data flow.

use core::marker::Tuple;

use crate::datasets::Dataset;

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

/// Trait for items that can be part of a pipeline (nodes or nested pipelines).
pub trait PipelineItem: Send + Sync {
    fn call(&self);
    fn get_name(&self) -> &'static str;
    fn is_leaf(&self) -> bool;
    fn for_each_child<'a>(&'a self, f: &mut dyn FnMut(&'a dyn PipelineItem));
    fn for_each_input_id(&self, f: &mut dyn FnMut(&DatasetRef));
    fn for_each_output_id(&self, f: &mut dyn FnMut(&DatasetRef));
}

/// Trait for loading data from input datasets.
pub trait NodeInput: Tuple {
    type Args: Tuple;
    fn load_data(&self) -> Self::Args;
    fn for_each_input_id(&self, f: &mut dyn FnMut(&DatasetRef));
}

impl NodeInput for () {
    type Args = ();
    fn load_data(&self) -> Self::Args {
        ()
    }
    fn for_each_input_id(&self, _f: &mut dyn FnMut(&DatasetRef)) {}
}

impl<T: Dataset> NodeInput for (&T,) {
    type Args = (T::LoadItem,);
    fn load_data(&self) -> Self::Args {
        (self.0.load().unwrap(),)
    }
    fn for_each_input_id(&self, f: &mut dyn FnMut(&DatasetRef)) {
        f(&DatasetRef { id: ptr_to_id(self.0), is_param: self.0.is_param() });
    }
}

impl<T1: Dataset, T2: Dataset> NodeInput for (&T1, &T2) {
    type Args = (T1::LoadItem, T2::LoadItem);
    fn load_data(&self) -> Self::Args {
        (self.0.load().unwrap(), self.1.load().unwrap())
    }
    fn for_each_input_id(&self, f: &mut dyn FnMut(&DatasetRef)) {
        f(&DatasetRef { id: ptr_to_id(self.0), is_param: self.0.is_param() });
        f(&DatasetRef { id: ptr_to_id(self.1), is_param: self.1.is_param() });
    }
}

/// Trait for saving data to output datasets.
pub trait NodeOutput: Tuple {
    type Output: Tuple;
    fn save_data(&self, output: Self::Output);
    fn for_each_output_id(&self, f: &mut dyn FnMut(&DatasetRef));
}

impl NodeOutput for () {
    type Output = ();
    fn save_data(&self, _output: Self::Output) {}
    fn for_each_output_id(&self, _f: &mut dyn FnMut(&DatasetRef)) {}
}

impl<T: Dataset> NodeOutput for (&T,) {
    type Output = (T::SaveItem,);
    fn save_data(&self, output: Self::Output) {
        self.0.save(output.0);
    }
    fn for_each_output_id(&self, f: &mut dyn FnMut(&DatasetRef)) {
        f(&DatasetRef { id: ptr_to_id(self.0), is_param: self.0.is_param() });
    }
}

impl<T1: Dataset, T2: Dataset> NodeOutput for (&T1, &T2) {
    type Output = (T1::SaveItem, T2::SaveItem);
    fn save_data(&self, output: Self::Output) {
        self.0.save(output.0);
        self.1.save(output.1);
    }
    fn for_each_output_id(&self, f: &mut dyn FnMut(&DatasetRef)) {
        f(&DatasetRef { id: ptr_to_id(self.0), is_param: self.0.is_param() });
        f(&DatasetRef { id: ptr_to_id(self.1), is_param: self.1.is_param() });
    }
}
