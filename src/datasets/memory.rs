//! In-memory dataset for intermediate values.

use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};

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

impl<T: Copy> Dataset for MemoryDataset<T> {
    type LoadItem = T;
    type SaveItem = T;

    fn load(&self) -> Option<Self::LoadItem> {
        *self.value.lock().unwrap()
    }

    fn save(&self, output: Self::SaveItem) {
        let mut value = self.value.lock().unwrap();
        *value = Some(output);
    }
}
