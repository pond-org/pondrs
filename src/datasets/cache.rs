//! Caching wrapper around any dataset.
//!
//! Stores a copy in memory after every load/save, so subsequent loads
//! return the cached value without hitting the underlying dataset. With
//! `take`, the first load moves the value out of memory instead.

use std::prelude::v1::*;
use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};

use crate::error::PondError;
use super::Dataset;

/// Caching wrapper that stores a copy in memory after every load/save.
///
/// Subsequent loads return the cached value without hitting the
/// underlying dataset.
///
/// With `take: true`, the first load after a save moves the value out of
/// memory rather than cloning it, and later loads read the inner dataset
/// again. Use it for a value with a single consumer, such as one checkpoint
/// in an epoch chain, so that memory holds only the values not yet consumed.
#[derive(Debug, Serialize, Deserialize)]
pub struct CacheDataset<D: Dataset> {
    pub dataset: D,
    #[serde(default)]
    pub take: bool,
    #[serde(skip_serializing, skip_deserializing)]
    cache: Arc<Mutex<Option<D::LoadItem>>>,
}

impl<D: Dataset> CacheDataset<D>
where
    D::LoadItem: Clone,
{
    pub fn new(dataset: D) -> Self {
        Self {
            dataset,
            take: false,
            cache: Arc::new(Mutex::new(None)),
        }
    }

    /// Sets whether the first load moves the value out of memory.
    #[must_use]
    pub fn with_take(mut self, take: bool) -> Self {
        self.take = take;
        self
    }
}

impl<D: Dataset> Dataset for CacheDataset<D>
where
    // `Send` is already implied wherever the dataset is usable in a pipeline
    // (`DatasetMeta` needs `Self: Sync`); it lets `remover` move the cache out.
    D::LoadItem: Clone + Send,
    D::SaveItem: Clone + Into<D::LoadItem>,
    PondError: From<D::Error>,
{
    type LoadItem = D::LoadItem;
    type SaveItem = D::SaveItem;
    type Error = PondError;

    fn load(&self) -> Result<Self::LoadItem, PondError> {
        let mut guard = self.cache.lock().map_err(|e| PondError::LockPoisoned(e.to_string()))?;
        if self.take {
            if let Some(cached) = guard.take() {
                return Ok(cached);
            }
            drop(guard);
            return Ok(self.dataset.load()?);
        }
        if let Some(cached) = &*guard {
            return Ok(cached.clone());
        }
        drop(guard);

        let value = self.dataset.load()?;
        let mut guard = self.cache.lock().map_err(|e| PondError::LockPoisoned(e.to_string()))?;
        *guard = Some(value.clone());
        Ok(value)
    }

    fn save(&self, output: Self::SaveItem) -> Result<(), PondError> {
        self.dataset.save(output.clone())?;
        let mut guard = self.cache.lock().map_err(|e| PondError::LockPoisoned(e.to_string()))?;
        *guard = Some(output.into());
        Ok(())
    }

    fn content_hash(&self) -> Option<u64> { self.dataset.content_hash() }
    fn is_persistent(&self) -> bool { self.dataset.is_persistent() }

    /// Clears the in-memory copy as well as the inner dataset's stored value.
    /// Offered only when the inner dataset has a remover.
    fn remover(&self) -> Option<super::Remover> {
        let inner = self.dataset.remover()?;
        let cache = Arc::clone(&self.cache);
        Some(Box::new(move || {
            *cache.lock().unwrap_or_else(std::sync::PoisonError::into_inner) = None;
            inner()
        }))
    }

    fn html(&self) -> Option<String> {
        self.dataset.html()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::datasets::TextDataset;
    use tempfile::tempdir;

    fn cached(dir: &std::path::Path, take: bool) -> CacheDataset<TextDataset> {
        CacheDataset::new(TextDataset::new(dir.join("v.txt").to_str().unwrap())).with_take(take)
    }

    #[test]
    fn keeps_the_value_in_memory_by_default() {
        let dir = tempdir().unwrap();
        let ds = cached(dir.path(), false);
        ds.save("saved".into()).unwrap();
        std::fs::write(dir.path().join("v.txt"), "on disk").unwrap();
        assert_eq!(ds.load().unwrap(), "saved");
        assert_eq!(ds.load().unwrap(), "saved");
    }

    #[test]
    fn take_moves_the_value_out_then_reads_the_inner_dataset() {
        let dir = tempdir().unwrap();
        let ds = cached(dir.path(), true);
        ds.save("saved".into()).unwrap();
        std::fs::write(dir.path().join("v.txt"), "on disk").unwrap();
        assert_eq!(ds.load().unwrap(), "saved");
        assert_eq!(ds.load().unwrap(), "on disk");
        assert!(ds.cache.lock().unwrap().is_none());
    }

    #[test]
    fn take_deserializes_with_a_default() {
        let ds: CacheDataset<TextDataset> = serde_yaml::from_str("dataset:\n  path: a.txt\n").unwrap();
        assert!(!ds.take);
        let ds: CacheDataset<TextDataset> = serde_yaml::from_str("dataset:\n  path: a.txt\ntake: true\n").unwrap();
        assert!(ds.take);
    }

    #[test]
    fn remover_clears_memory_and_the_inner_file() {
        let dir = tempdir().unwrap();
        let ds = cached(dir.path(), false);
        ds.save("saved".into()).unwrap();
        ds.remover().unwrap()().unwrap();
        assert!(ds.cache.lock().unwrap().is_none());
        assert!(!dir.path().join("v.txt").exists());
        assert!(ds.load().is_err());
    }
}
