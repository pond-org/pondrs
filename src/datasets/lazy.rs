//! Lazy dataset wrapper — defers load and save to call time.

use std::prelude::v1::*;
use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::error::PondError;
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

    fn save_partitioned(
        &self,
        entries: HashMap<String, Self::SaveItem>,
        dir: &std::path::Path,
        ext: &str,
    ) -> Result<(), PondError>
    where
        PondError: From<Self::Error>,
        Self: Send + Sync,
        Self::SaveItem: Send,
        Self::Error: Send,
    {
        if rayon::current_thread_index().is_some() {
            use rayon::iter::{IntoParallelIterator, ParallelIterator};
            entries.into_par_iter().try_for_each(|(name, thunk)| {
                let value = thunk()?;
                let path = dir.join(format!("{name}.{ext}"));
                let mut ds = self.dataset.clone();
                ds.set_path(path.to_str().ok_or_else(|| PondError::Custom(format!("non-UTF-8 path: {}", path.display())))?);
                ds.save(value)?;
                Ok(())
            })
        } else {
            <Self as FileDataset>::default_save_partitioned(self, entries, dir, ext)
        }
    }
}

#[cfg(feature = "polars")]
pub type LazyPartitionedDataset<D> = super::PartitionedDataset<LazyDataset<D>>;
