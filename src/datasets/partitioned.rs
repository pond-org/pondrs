//! Partitioned dataset types.

use std::prelude::v1::*;
use std::collections::HashMap;
use std::fs;

use serde::{Deserialize, Serialize, de::DeserializeOwned};

use super::{Dataset, FileDataset};

pub struct Lazy<T> {
    loader: Box<dyn Fn() -> Option<T>>,
}

impl<T> Lazy<T> {
    pub fn new(loader: impl Fn() -> Option<T> + 'static) -> Self {
        Self {
            loader: Box::new(loader),
        }
    }

    pub fn load(&self) -> Option<T> {
        (self.loader)()
    }
}

#[derive(Serialize, Deserialize)]
#[serde(bound(serialize = "D: Serialize", deserialize = "D: DeserializeOwned"))]
pub struct LazyPartitionedDataset<D: FileDataset + Serialize + DeserializeOwned> {
    pub path: String,
    pub ext: &'static str,
    pub dataset: D,
}

impl<D: FileDataset + Serialize + DeserializeOwned + 'static> Dataset
    for LazyPartitionedDataset<D>
{
    type LoadItem = HashMap<String, Lazy<D::LoadItem>>;
    type SaveItem = HashMap<String, D::SaveItem>;

    fn save(&self, datasets: Self::SaveItem) {
        std::fs::create_dir_all(&self.path).unwrap();
        let dir = std::path::Path::new(&self.path);
        for (name, data) in datasets {
            let ext = self.ext;
            let path = dir.join(format!("{name}.{ext}"));
            let mut dataset = self.dataset.clone();
            dataset.set_path(path.to_str().unwrap());
            dataset.save(data);
        }
    }

    fn load(&self) -> Option<Self::LoadItem> {
        let Ok(paths) = fs::read_dir(&self.path) else {
            return None;
        };
        let mut datasets = Self::LoadItem::new();
        for entry in paths {
            let Ok(entry) = entry else { continue };
            let file_name = entry.file_name();
            if !file_name.to_string_lossy().ends_with(self.ext) {
                continue;
            }
            let file_stem = entry
                .path()
                .file_stem()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap();
            let mut dataset = self.dataset.clone();
            dataset.set_path(entry.path().to_str().unwrap());
            datasets.insert(file_stem, Lazy::new(move || dataset.load()));
        }
        Some(datasets)
    }
}

#[derive(Serialize, Deserialize)]
#[serde(bound(serialize = "D: Serialize", deserialize = "D: DeserializeOwned"))]
pub struct PartitionedDataset<D: FileDataset + Serialize + DeserializeOwned> {
    pub path: String,
    pub ext: &'static str,
    pub dataset: D,
}

impl<D: FileDataset + Serialize + DeserializeOwned + 'static> Dataset for PartitionedDataset<D> {
    type LoadItem = HashMap<String, D::LoadItem>;
    type SaveItem = HashMap<String, D::SaveItem>;

    fn save(&self, datasets: Self::SaveItem) {
        std::fs::create_dir_all(&self.path).unwrap();
        let dir = std::path::Path::new(&self.path);
        for (name, data) in datasets {
            let ext = self.ext;
            let path = dir.join(format!("{name}.{ext}"));
            let mut dataset = self.dataset.clone();
            dataset.set_path(path.to_str().unwrap());
            dataset.save(data);
        }
    }

    fn load(&self) -> Option<Self::LoadItem> {
        let Ok(paths) = fs::read_dir(&self.path) else {
            return None;
        };
        let mut datasets = Self::LoadItem::new();
        for entry in paths {
            let Ok(entry) = entry else { continue };
            let file_name = entry.file_name();
            if !file_name.to_string_lossy().ends_with(self.ext) {
                continue;
            }
            let file_stem = entry
                .path()
                .file_stem()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap();
            let mut dataset = self.dataset.clone();
            dataset.set_path(entry.path().to_str().unwrap());
            if let Some(loaded) = dataset.load() {
                datasets.insert(file_stem, loaded);
            }
        }
        Some(datasets)
    }
}
