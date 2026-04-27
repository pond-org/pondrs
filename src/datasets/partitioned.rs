//! Partitioned dataset types.

use std::prelude::v1::*;
use std::collections::HashMap;

use serde::{Deserialize, Serialize, de::DeserializeOwned};

use crate::error::PondError;
use super::{Dataset, FileDataset};

/// A directory of files where each file is eagerly loaded into memory.
///
/// On load, returns a `HashMap<filename_stem, D::LoadItem>`.
/// On save, writes each entry as `{name}.{ext}` in the directory.
#[derive(Debug, Serialize, Deserialize)]
#[serde(bound(serialize = "D: Serialize", deserialize = "D: DeserializeOwned"))]
pub struct PartitionedDataset<D: FileDataset + Serialize + DeserializeOwned> {
    pub path: String,
    pub ext: String,
    pub dataset: D,
}

impl<D: FileDataset + Serialize + DeserializeOwned + Send + Sync + 'static> Dataset for PartitionedDataset<D>
where
    PondError: From<D::Error>,
    D::SaveItem: Send,
    D::Error: Send,
{
    type LoadItem = HashMap<String, D::LoadItem>;
    type SaveItem = HashMap<String, D::SaveItem>;
    type Error = PondError;

    fn save(&self, datasets: Self::SaveItem) -> Result<(), PondError> {
        self.dataset.save_partitioned(datasets, &self.path, &self.ext)
    }

    fn load(&self) -> Result<Self::LoadItem, PondError> {
        self.dataset.load_partitioned(&self.path, &self.ext)
    }

    #[cfg(feature = "std")]
    fn html(&self) -> Option<String> {
        let entries = self.dataset.list_entries(&self.path, &self.ext).ok()?;
        if entries.is_empty() { return None; }
        let items: Vec<String> = entries.iter().map(|name| format!("<li>{name}.{}</li>", self.ext)).collect();
        Some(format!(
            "<ul style=\"font-family:monospace;font-size:13px;padding:8px 8px 8px 28px;margin:0\">{}</ul>",
            items.join("")
        ))
    }
}
