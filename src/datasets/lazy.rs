//! Lazy dataset wrapper — defers load and save to call time.

use std::prelude::v1::*;

use serde::{Deserialize, Serialize};

use super::{Dataset, FileDataset};

/// A deferred computation that produces a value on demand.
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
