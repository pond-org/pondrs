//! Caching wrapper around any dataset.
//!
//! Stores a copy in memory after every load/save, so subsequent loads
//! return the cached value without hitting the underlying dataset.

use std::prelude::v1::*;
use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};

use crate::error::PondError;
use super::Dataset;

/// Caching wrapper that stores a copy in memory after every load/save.
///
/// Subsequent loads return the cached value without hitting the
/// underlying dataset.
#[derive(Debug, Serialize, Deserialize)]
pub struct CacheDataset<D: Dataset> {
    pub dataset: D,
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
            cache: Arc::new(Mutex::new(None)),
        }
    }
}

impl<D: Dataset> Dataset for CacheDataset<D>
where
    D::LoadItem: Clone,
    D::SaveItem: Clone + Into<D::LoadItem>,
    PondError: From<D::Error>,
{
    type LoadItem = D::LoadItem;
    type SaveItem = D::SaveItem;
    type Error = PondError;

    fn load(&self) -> Result<Self::LoadItem, PondError> {
        let guard = self.cache.lock().map_err(|e| PondError::LockPoisoned(e.to_string()))?;
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

    fn html(&self) -> Option<String> {
        self.dataset.html()
    }
}
