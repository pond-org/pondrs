//! Cell-based dataset for no_std single-threaded use.

use core::cell::Cell;

use serde::ser::{Serialize, Serializer};

use super::Dataset;

pub struct CellDataset<T: Copy> {
    value: Cell<Option<T>>,
}

// SAFETY: CellDataset is intended for single-threaded use only (e.g. SequentialRunner).
// The Sync impl is required because PipelineItem: Send + Sync, but no concurrent
// access occurs in single-threaded runners.
unsafe impl<T: Copy + Send> Sync for CellDataset<T> {}

impl<T: Copy> CellDataset<T> {
    pub const fn new() -> Self {
        Self {
            value: Cell::new(None),
        }
    }
}

impl<T: Copy> Default for CellDataset<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Copy> Dataset for CellDataset<T> {
    type LoadItem = T;
    type SaveItem = T;

    fn load(&self) -> Option<Self::LoadItem> {
        self.value.get()
    }

    fn save(&self, output: Self::SaveItem) {
        self.value.set(Some(output));
    }
}

impl<T: Copy> Serialize for CellDataset<T> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_unit()
    }
}
