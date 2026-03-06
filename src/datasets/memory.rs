//! In-memory dataset for intermediate values.

use std::prelude::v1::*;
use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};

use crate::error::PondError;
use super::Dataset;

#[derive(Serialize, Deserialize)]
pub struct MemoryDataset<T: Clone> {
    #[serde(skip_serializing, skip_deserializing)]
    value: Arc<Mutex<Option<T>>>,
}

impl<T: Clone> MemoryDataset<T> {
    pub fn new() -> Self {
        Self {
            value: Arc::new(Mutex::new(None)),
        }
    }
}

impl<T: Clone> Default for MemoryDataset<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Clone> Dataset for MemoryDataset<T> {
    type LoadItem = T;
    type SaveItem = T;
    type Error = PondError;

    fn load(&self) -> Result<Self::LoadItem, PondError> {
        let guard = self.value.lock().map_err(|e| PondError::LockPoisoned(e.to_string()))?;
        guard.clone().ok_or(PondError::DatasetNotLoaded)
    }

    fn save(&self, output: Self::SaveItem) -> Result<(), PondError> {
        let mut value = self.value.lock().map_err(|e| PondError::LockPoisoned(e.to_string()))?;
        *value = Some(output);
        Ok(())
    }
}
