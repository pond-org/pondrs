use std::prelude::v1::*;
use std::collections::HashMap;

use serde::{Deserialize, Serialize, de::DeserializeOwned};

use crate::error::PondError;
use super::{Dataset, FileDataset};

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

    fn load(&self) -> Result<Self::LoadItem, PondError> {
        let mut items = HashMap::new();
        for name in self.dataset.list_entries(&self.path, &self.ext)? {
            let file_path = format!("{}/{name}.{}", self.path, self.ext);
            let mut ds = self.dataset.clone();
            ds.set_path(&file_path);
            items.insert(name, ds.load()?);
        }
        Ok(items)
    }

    fn save(&self, entries: Self::SaveItem) -> Result<(), PondError> {
        let mut ds = self.dataset.clone();
        ds.set_path(&format!("{}/_.{}", self.path, self.ext));
        ds.ensure_parent_dir()?;

        if self.dataset.prefer_parallel() && rayon::current_thread_index().is_some() {
            use rayon::iter::{IntoParallelIterator, ParallelIterator};
            entries.into_par_iter().try_for_each(|(name, value)| {
                let file_path = format!("{}/{name}.{}", self.path, self.ext);
                let mut ds = self.dataset.clone();
                ds.set_path(&file_path);
                ds.save(value)?;
                Ok(())
            })
        } else {
            for (name, value) in entries {
                let file_path = format!("{}/{name}.{}", self.path, self.ext);
                let mut ds = self.dataset.clone();
                ds.set_path(&file_path);
                ds.save(value)?;
            }
            Ok(())
        }
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
