use std::prelude::v1::*;

/// A deferred computation carrying the pipeline error type `E`.
///
/// Generic over `E` rather than pinned to
/// [`PondError`](crate::error::PondError) so that a [`PartitionedNode`]'s
/// function may return a custom error type, exactly as a plain
/// [`Node`](crate::pipeline::Node)'s may.
///
/// [`PartitionedNode`]: crate::pipeline::PartitionedNode
pub type Thunk<T, E> = Box<dyn FnOnce() -> Result<T, E> + Send>;

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
pub trait IntoThunk<T, E> {
    fn into_thunk(self) -> Thunk<T, E>;
}

impl<T: Send + 'static, E> IntoThunk<T, E> for T {
    fn into_thunk(self) -> Thunk<T, E> {
        Box::new(move || Ok(self))
    }
}

impl<T: Send + 'static, E: From<E2>, E2: Send + 'static> IntoThunk<T, E>
    for Box<dyn FnOnce() -> Result<T, E2> + Send>
{
    fn into_thunk(self) -> Thunk<T, E> {
        Box::new(move || self().map_err(E::from))
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
pub trait FromThunk<T, E>: Sized {
    fn from_thunk(thunk: Thunk<T, E>) -> Result<Self, E>;
}

impl<T: Send + 'static, E> FromThunk<T, E> for T {
    fn from_thunk(thunk: Thunk<T, E>) -> Result<Self, E> {
        thunk()
    }
}

impl<T: Send + 'static, E: Send + 'static, E2: From<E> + Send + 'static> FromThunk<T, E>
    for Box<dyn FnOnce() -> Result<T, E2> + Send>
{
    fn from_thunk(thunk: Thunk<T, E>) -> Result<Self, E> {
        Ok(Box::new(move || thunk().map_err(E2::from)))
    }
}
