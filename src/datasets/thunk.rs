use std::prelude::v1::*;

use crate::error::PondError;

pub type Thunk<T> = Box<dyn FnOnce() -> Result<T, PondError> + Send>;

/// Adapts a loaded partition element into a [`Thunk`].
///
/// Implemented for `T` itself (eager datasets, whose `LoadItem` is the element)
/// and for a `Lazy` thunk (lazy datasets). A failure to satisfy it in a
/// [`PartitionedNode`] means the input element type and the function's parameter
/// type disagree.
///
/// [`PartitionedNode`]: crate::pipeline::PartitionedNode
#[diagnostic::on_unimplemented(
    message = "the input partition yields `{Self}`, but the node function takes `{T}`",
    label = "yields `{Self}`",
    note = "a partitioned node's function maps one element of the input partition to one element of the output partition"
)]
pub trait IntoThunk<T> {
    fn into_thunk(self) -> Thunk<T>;
}

impl<T: Send + 'static> IntoThunk<T> for T {
    fn into_thunk(self) -> Thunk<T> {
        Box::new(move || Ok(self))
    }
}

impl<T: Send + 'static, E: Into<PondError> + Send + 'static> IntoThunk<T>
    for Box<dyn FnOnce() -> Result<T, E> + Send>
{
    fn into_thunk(self) -> Thunk<T> {
        Box::new(move || self().map_err(Into::into))
    }
}

/// Converts a [`Thunk`] into the item an output partition saves.
///
/// The mirror of [`IntoThunk`]: implemented for `T` itself (eager datasets) and
/// for a `Lazy` thunk (lazy datasets).
#[diagnostic::on_unimplemented(
    message = "the node function returns `{T}`, but the output partition stores `{Self}`",
    label = "stores `{Self}`",
    note = "a partitioned node's function maps one element of the input partition to one element of the output partition"
)]
pub trait FromThunk<T>: Sized {
    fn from_thunk(thunk: Thunk<T>) -> Result<Self, PondError>;
}

impl<T: Send + 'static> FromThunk<T> for T {
    fn from_thunk(thunk: Thunk<T>) -> Result<Self, PondError> {
        thunk()
    }
}

impl<T: Send + 'static, E: From<PondError> + Send + 'static> FromThunk<T>
    for Box<dyn FnOnce() -> Result<T, E> + Send>
{
    fn from_thunk(thunk: Thunk<T>) -> Result<Self, PondError> {
        Ok(Box::new(move || thunk().map_err(Into::into)))
    }
}
