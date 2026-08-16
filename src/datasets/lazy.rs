//! Lazy dataset wrapper — defers load and save to call time.

use std::prelude::v1::*;

use serde::{Deserialize, Serialize};

use super::{Dataset, FileDataset};

/// A deferred computation that produces a value on demand.
///
/// Used in two positions. As [`LazyDataset`]'s `LoadItem`/`SaveItem`, `E` is the
/// inner dataset's error. Inside a [`PartitionedNode`], `E` is the pipeline error
/// type — generic rather than pinned to [`PondError`](crate::error::PondError) so
/// that a partitioned node's function may return a custom error type, exactly as a
/// plain [`Node`](crate::pipeline::Node)'s may. [`IntoLazy`] and [`FromLazy`]
/// bridge the two.
///
/// [`PartitionedNode`]: crate::pipeline::PartitionedNode
pub type Lazy<T, E> = Box<dyn FnOnce() -> Result<T, E> + Send>;

/// Lazy wrapper around any dataset — defers load and save to call time.
///
/// On load, returns a closure that loads from the inner dataset when called.
/// On save, accepts a closure that produces the value, calls it, then saves.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(transparent)]
pub struct LazyDataset<D> {
    pub dataset: D,
}

impl<D: Dataset + Clone + Send + 'static> Dataset for LazyDataset<D>
where
    D::Error: Send,
{
    type LoadItem = Lazy<D::LoadItem, D::Error>;
    type SaveItem = Lazy<D::SaveItem, D::Error>;
    type Error = D::Error;

    fn load(&self) -> Result<Self::LoadItem, D::Error> {
        let ds = self.dataset.clone();
        Ok(Box::new(move || ds.load()))
    }

    fn save(&self, thunk: Self::SaveItem) -> Result<(), D::Error> {
        let value = thunk()?;
        self.dataset.save(value)
    }

    fn is_param(&self) -> bool {
        self.dataset.is_param()
    }

    fn content_hash(&self) -> Option<u64> { self.dataset.content_hash() }
    fn is_persistent(&self) -> bool { self.dataset.is_persistent() }

    fn html(&self) -> Option<String> {
        self.dataset.html()
    }
}

impl<D: FileDataset + Send + Sync + 'static> FileDataset for LazyDataset<D>
where
    D::Error: Send,
    D::SaveItem: Send,
{
    fn path(&self) -> &str {
        self.dataset.path()
    }

    fn set_path(&mut self, path: &str) {
        self.dataset.set_path(path);
    }

    fn prefer_parallel(&self) -> bool { true }
}

pub type LazyPartitionedDataset<D> = super::PartitionedDataset<LazyDataset<D>>;

/// Adapts a loaded partition element into a [`Lazy`].
///
/// Implemented for `T` itself (eager datasets, whose `LoadItem` is the element)
/// and for a [`Lazy`] (lazy datasets). A failure to satisfy it in a
/// [`PartitionedNode`] means the input element type and the function's parameter
/// type disagree.
///
/// [`PartitionedNode`]: crate::pipeline::PartitionedNode
#[diagnostic::on_unimplemented(
    message = "the input partition yields `{Self}`, but the node function takes `{T}`",
    label = "yields `{Self}`",
    note = "a partitioned node's function maps one element of the input partition to one element of the output partition"
)]
pub trait IntoLazy<T, E> {
    fn into_lazy(self) -> Lazy<T, E>;
}

impl<T: Send + 'static, E> IntoLazy<T, E> for T {
    fn into_lazy(self) -> Lazy<T, E> {
        Box::new(move || Ok(self))
    }
}

impl<T: Send + 'static, E: From<E2>, E2: Send + 'static> IntoLazy<T, E> for Lazy<T, E2> {
    fn into_lazy(self) -> Lazy<T, E> {
        Box::new(move || self().map_err(E::from))
    }
}

/// Converts a [`Lazy`] into the item an output partition saves.
///
/// The mirror of [`IntoLazy`]: implemented for `T` itself (eager datasets) and
/// for a [`Lazy`] (lazy datasets).
#[diagnostic::on_unimplemented(
    message = "the node function returns `{T}`, but the output partition stores `{Self}`",
    label = "stores `{Self}`",
    note = "a partitioned node's function maps one element of the input partition to one element of the output partition"
)]
pub trait FromLazy<T, E>: Sized {
    fn from_lazy(lazy: Lazy<T, E>) -> Result<Self, E>;
}

impl<T: Send + 'static, E> FromLazy<T, E> for T {
    fn from_lazy(lazy: Lazy<T, E>) -> Result<Self, E> {
        lazy()
    }
}

impl<T: Send + 'static, E: Send + 'static, E2: From<E> + Send + 'static> FromLazy<T, E>
    for Lazy<T, E2>
{
    fn from_lazy(lazy: Lazy<T, E>) -> Result<Self, E> {
        Ok(Box::new(move || lazy().map_err(E2::from)))
    }
}
