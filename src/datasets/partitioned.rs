//! Partitioned dataset types.

use std::prelude::v1::*;
use std::collections::HashMap;
use std::fs;

use serde::{Deserialize, Serialize, de::DeserializeOwned};

use crate::error::PondError;
use super::{Dataset, FileDataset};

/// A deferred loader that produces a value on demand.
pub struct Lazy<T> {
    loader: Box<dyn Fn() -> Result<T, PondError>>,
}

impl<T> Lazy<T> {
    pub fn new(loader: impl Fn() -> Result<T, PondError> + 'static) -> Self {
        Self {
            loader: Box::new(loader),
        }
    }

    pub fn load(&self) -> Result<T, PondError> {
        (self.loader)()
    }
}

/// A directory of files where each file is loaded lazily on demand.
///
/// On load, returns a `HashMap<filename_stem, Lazy<D::LoadItem>>`.
/// On save, writes each entry as `{name}.{ext}` in the directory.
#[derive(Serialize, Deserialize)]
#[serde(bound(serialize = "D: Serialize", deserialize = "D: DeserializeOwned"))]
pub struct LazyPartitionedDataset<D: FileDataset + Serialize + DeserializeOwned> {
    pub path: String,
    pub ext: String,
    pub dataset: D,
}

fn list_files(path: &str, ext: &str) -> Option<Vec<String>> {
    let mut names: Vec<String> = fs::read_dir(path)
        .ok()?
        .filter_map(|e| e.ok())
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().into_owned();
            if name.ends_with(ext) { Some(name) } else { None }
        })
        .collect();
    names.sort();
    Some(names)
}

fn files_html(path: &str, ext: &str) -> Option<String> {
    let files = list_files(path, ext)?;
    if files.is_empty() { return None; }
    let items: Vec<String> = files.iter().map(|f| format!("<li>{f}</li>")).collect();
    Some(format!(
        "<ul style=\"font-family:monospace;font-size:13px;padding:8px 8px 8px 28px;margin:0\">{}</ul>",
        items.join("")
    ))
}

impl<D: FileDataset + Serialize + DeserializeOwned + 'static> Dataset
    for LazyPartitionedDataset<D>
where
    PondError: From<D::Error>,
{
    type LoadItem = HashMap<String, Lazy<D::LoadItem>>;
    type SaveItem = HashMap<String, D::SaveItem>;
    type Error = PondError;

    fn save(&self, datasets: Self::SaveItem) -> Result<(), PondError> {
        std::fs::create_dir_all(&self.path)?;
        let dir = std::path::Path::new(&self.path);
        for (name, data) in datasets {
            let ext = &self.ext;
            let path = dir.join(format!("{name}.{ext}"));
            let mut dataset = self.dataset.clone();
            dataset.set_path(path.to_str().unwrap());
            dataset.save(data)?;
        }
        Ok(())
    }

    fn load(&self) -> Result<Self::LoadItem, PondError> {
        let paths = fs::read_dir(&self.path)?;
        let mut datasets = Self::LoadItem::new();
        for entry in paths {
            let entry = entry?;
            let file_name = entry.file_name();
            if !file_name.to_string_lossy().ends_with(&*self.ext) {
                continue;
            }
            let file_stem = entry
                .path()
                .file_stem()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap();
            let mut dataset = self.dataset.clone();
            dataset.set_path(entry.path().to_str().unwrap());
            datasets.insert(file_stem, Lazy::new(move || Ok(dataset.load()?)));
        }
        Ok(datasets)
    }

    #[cfg(feature = "std")]
    fn html(&self) -> Option<String> {
        files_html(&self.path, &self.ext)
    }
}

/// A directory of files where each file is eagerly loaded into memory.
///
/// On load, returns a `HashMap<filename_stem, D::LoadItem>`.
/// On save, writes each entry as `{name}.{ext}` in the directory.
#[derive(Serialize, Deserialize)]
#[serde(bound(serialize = "D: Serialize", deserialize = "D: DeserializeOwned"))]
pub struct PartitionedDataset<D: FileDataset + Serialize + DeserializeOwned> {
    pub path: String,
    pub ext: String,
    pub dataset: D,
}

impl<D: FileDataset + Serialize + DeserializeOwned + 'static> Dataset for PartitionedDataset<D>
where
    PondError: From<D::Error>,
{
    type LoadItem = HashMap<String, D::LoadItem>;
    type SaveItem = HashMap<String, D::SaveItem>;
    type Error = PondError;

    fn save(&self, datasets: Self::SaveItem) -> Result<(), PondError> {
        std::fs::create_dir_all(&self.path)?;
        let dir = std::path::Path::new(&self.path);
        for (name, data) in datasets {
            let ext = &self.ext;
            let path = dir.join(format!("{name}.{ext}"));
            let mut dataset = self.dataset.clone();
            dataset.set_path(path.to_str().unwrap());
            dataset.save(data)?;
        }
        Ok(())
    }

    fn load(&self) -> Result<Self::LoadItem, PondError> {
        let paths = fs::read_dir(&self.path)?;
        let mut datasets = Self::LoadItem::new();
        for entry in paths {
            let entry = entry?;
            let file_name = entry.file_name();
            if !file_name.to_string_lossy().ends_with(&*self.ext) {
                continue;
            }
            let file_stem = entry
                .path()
                .file_stem()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap();
            let mut dataset = self.dataset.clone();
            dataset.set_path(entry.path().to_str().unwrap());
            datasets.insert(file_stem, dataset.load()?);
        }
        Ok(datasets)
    }

    #[cfg(feature = "std")]
    fn html(&self) -> Option<String> {
        files_html(&self.path, &self.ext)
    }
}
