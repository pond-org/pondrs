//! Polars DataFrame dataset.

use std::collections::HashMap;
use std::fs;

use serde::{Deserialize, Serialize, de::DeserializeOwned};

use super::Dataset;

pub struct Lazy<D: Dataset> {
    dataset: D,
}

impl<D: Dataset> Lazy<D> {
    pub fn load(self) -> Option<D::LoadItem> {
        self.dataset.load()
    }
}

#[derive(Serialize, Deserialize)]
#[serde(bound(serialize = "D: Serialize", deserialize = "D: DeserializeOwned"))]
pub struct PartitionedDataset<D: Dataset + Serialize + DeserializeOwned> {
    path: String,
    ext: &'static str,
    dataset: D,
}

impl<D: Dataset + Serialize + DeserializeOwned> Dataset for PartitionedDataset<D> {
    type LoadItem = HashMap<String, Lazy<D>>;
    type SaveItem = HashMap<String, Lazy<D>>;

    fn save(&self, d: Self::SaveItem) {}

    fn load(&self) -> Option<Self::LoadItem> {
        let Ok(paths) = fs::read_dir(&self.path) else {
            return None;
        };
        let mut dataset = Self::LoadItem::new();
        for entry in paths {
            let Ok(entry) = entry else { continue };
            let file_name = entry.file_name();
            if !file_name.to_string_lossy().ends_with(self.ext) {
                continue;
            }
            let file_stem = entry
                .path()
                .file_stem()
                .map(|s| s.to_string_lossy().into_owned());
            // let d = D
            // dataset[file_stem] =
        }
        Some(dataset)
    }
}
