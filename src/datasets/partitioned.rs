//! Partitioned dataset types.

use std::prelude::v1::*;
use std::collections::HashMap;
use std::fs;

use serde::{Deserialize, Serialize, de::DeserializeOwned};

use crate::error::PondError;
use super::{Dataset, FileDataset};

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
        std::fs::create_dir_all(&self.path)?;
        let dir = std::path::Path::new(&self.path);
        self.dataset.save_partitioned(datasets, dir, &self.ext)
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
            let entry_path = entry.path();
            let file_stem = entry_path
                .file_stem()
                .and_then(|s| s.to_str())
                .ok_or_else(|| PondError::Custom(format!("non-UTF-8 path: {}", entry_path.display())))?
                .to_string();
            let mut dataset = self.dataset.clone();
            dataset.set_path(entry_path.to_str().ok_or_else(|| PondError::Custom(format!("non-UTF-8 path: {}", entry_path.display())))?);
            datasets.insert(file_stem, dataset.load()?);
        }
        Ok(datasets)
    }

    #[cfg(feature = "std")]
    fn html(&self) -> Option<String> {
        files_html(&self.path, &self.ext)
    }
}
