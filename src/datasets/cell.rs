//! Cell-based dataset for no_std single-threaded use.

use core::cell::Cell;

use serde::ser::{Serialize, Serializer};

use crate::error::PondError;
use super::Dataset;

/// Stack-friendly dataset using `Cell` for `no_std` / single-threaded pipelines.
///
/// Only works with `Copy` types. Use `MemoryDataset` for `std` with thread safety.
#[derive(Debug)]
pub struct CellDataset<T: Copy> {
    value: Cell<Option<T>>,
}

// SAFETY: CellDataset is intended for single-threaded use only (e.g. SequentialRunner).
// The Sync impl is required because RunnableStep: Send + Sync, but no concurrent
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
    type Error = PondError;

    fn load(&self) -> Result<Self::LoadItem, PondError> {
        self.value.get().ok_or(PondError::DatasetNotLoaded)
    }

    fn save(&self, output: Self::SaveItem) -> Result<(), PondError> {
        self.value.set(Some(output));
        Ok(())
    }
}

impl<T: Copy> Serialize for CellDataset<T> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_unit()
    }
}
