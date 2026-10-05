use std::prelude::v1::*;
use std::collections::BTreeMap;

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
    type LoadItem = BTreeMap<String, D::LoadItem>;
    type SaveItem = BTreeMap<String, D::SaveItem>;
    type Error = PondError;

    fn load(&self) -> Result<Self::LoadItem, PondError> {
        let mut items = BTreeMap::new();
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

    fn is_persistent(&self) -> bool { true }

    /// Removes every entry through the inner dataset's own remover, then the
    /// directory if that left it empty. Files that are not entries are kept.
    fn remover(&self) -> Option<super::Remover> {
        let (path, ext, template) = (self.path.clone(), self.ext.clone(), self.dataset.clone());
        Some(Box::new(move || {
            let entries = match template.list_entries(&path, &ext) {
                Ok(entries) => entries,
                Err(PondError::Io(e)) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
                Err(e) => return Err(std::io::Error::other(e.to_string())),
            };
            for name in entries {
                let mut ds = template.clone();
                ds.set_path(&format!("{path}/{name}.{ext}"));
                if let Some(remove) = ds.remover() {
                    remove()?;
                }
            }
            // Fails when other files remain, which is the intent.
            let _ = std::fs::remove_dir(&path);
            Ok(())
        }))
    }

    fn content_hash(&self) -> Option<u64> {
        use core::hash::{Hash, Hasher};
        let mut hasher = std::hash::DefaultHasher::new();
        let canonical = std::fs::canonicalize(&self.path).ok()?;
        canonical.hash(&mut hasher);
        let entries = self.dataset.list_entries(&self.path, &self.ext).ok()?;
        for name in &entries {
            let file_path = format!("{}/{name}.{}", self.path, self.ext);
            let mut ds = self.dataset.clone();
            ds.set_path(&file_path);
            let entry_hash = ds.file_content_hash()?;
            entry_hash.hash(&mut hasher);
        }
        Some(hasher.finish())
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::datasets::{LazyDataset, TextDataset};
    use tempfile::tempdir;

    #[test]
    fn remover_deletes_entries_and_keeps_other_files() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("parts");
        let ds = PartitionedDataset {
            path: path.to_str().unwrap().into(),
            ext: "txt".into(),
            dataset: LazyDataset { dataset: TextDataset::new("") },
        };
        let entries: BTreeMap<String, crate::datasets::Lazy<String, PondError>> = ["a", "b"]
            .into_iter()
            .map(|k| (k.to_string(), Box::new(move || Ok(k.to_string())) as _))
            .collect();
        ds.save(entries).unwrap();
        std::fs::write(path.join("notes.md"), "keep me").unwrap();

        ds.remover().unwrap()().unwrap();
        assert!(!path.join("a.txt").exists() && !path.join("b.txt").exists());
        assert!(path.join("notes.md").exists());

        std::fs::remove_file(path.join("notes.md")).unwrap();
        ds.remover().unwrap()().unwrap();
        assert!(!path.exists());
        // Nothing left to remove is not an error.
        ds.remover().unwrap()().unwrap();
    }
}
