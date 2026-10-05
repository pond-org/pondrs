//! Plain text file dataset.

use std::prelude::v1::*;
use serde::{Deserialize, Serialize};

use crate::error::PondError;
use super::{Dataset, FileDataset};

/// Plain text file dataset. Loads/saves the entire file as a `String`.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TextDataset {
    path: String,
}

impl TextDataset {
    pub fn new(path: impl Into<String>) -> Self {
        Self { path: path.into() }
    }
}

impl Dataset for TextDataset {
    type LoadItem = String;
    type SaveItem = String;
    type Error = PondError;

    fn save(&self, text: Self::SaveItem) -> Result<(), PondError> {
        self.ensure_parent_dir()?;
        std::fs::write(&self.path, text)?;
        Ok(())
    }

    fn load(&self) -> Result<Self::LoadItem, PondError> {
        Ok(std::fs::read_to_string(&self.path)?)
    }

    fn content_hash(&self) -> Option<u64> { self.file_content_hash() }
    fn is_persistent(&self) -> bool { true }
    fn remover(&self) -> Option<super::Remover> { self.file_remover() }

    fn html(&self) -> Option<String> {
        let contents = std::fs::read_to_string(&self.path).ok()?;
        Some(format!(
            "<pre style=\"font-family:monospace;font-size:13px;padding:8px;margin:0;overflow:auto\">{}</pre>",
            super::html_escape(&contents)
        ))
    }
}

impl FileDataset for TextDataset {
    fn path(&self) -> &str {
        &self.path
    }
    fn set_path(&mut self, path: &str) {
        self.path = path.to_string();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::datasets::DatasetMeta;
    use tempfile::tempdir;

    #[test]
    fn round_trip() {
        let dir = tempdir().unwrap();
        let ds = TextDataset::new(dir.path().join("out.txt").to_str().unwrap());
        ds.save("hello world".to_string()).unwrap();
        assert_eq!(ds.load().unwrap(), "hello world");
    }

    #[test]
    fn html_is_none_before_save() {
        let dir = tempdir().unwrap();
        let ds = TextDataset::new(dir.path().join("out.txt").to_str().unwrap());
        let meta: &dyn DatasetMeta = &ds;
        assert!(meta.html().is_none());
    }

    #[test]
    fn html_contains_content_after_save() {
        let dir = tempdir().unwrap();
        let ds = TextDataset::new(dir.path().join("out.txt").to_str().unwrap());
        ds.save("hello <world>".to_string()).unwrap();
        let meta: &dyn DatasetMeta = &ds;
        let html = meta.html().unwrap();
        assert!(html.contains("<pre"));
        assert!(html.contains("hello &lt;world&gt;"));
    }

    #[test]
    fn hash_changes_with_size_at_equal_mtime() {
        let dir = tempdir().unwrap();
        let ds = TextDataset::new(dir.path().join("out.txt").to_str().unwrap());
        ds.save("1.0".to_string()).unwrap();
        let mtime = std::fs::metadata(&ds.path).unwrap().modified().unwrap();
        let before = Dataset::content_hash(&ds).unwrap();

        // A copy tool that preserves mtimes (`rsync -t`, `cp -p`).
        ds.save("1.25".to_string()).unwrap();
        std::fs::File::options().write(true).open(&ds.path).unwrap().set_modified(mtime).unwrap();
        assert_ne!(Dataset::content_hash(&ds).unwrap(), before);
    }

    #[test]
    fn remover_deletes_the_file_and_tolerates_a_missing_one() {
        let dir = tempdir().unwrap();
        let ds = TextDataset::new(dir.path().join("out.txt").to_str().unwrap());
        ds.save("x".to_string()).unwrap();
        Dataset::remover(&ds).unwrap()().unwrap();
        assert!(!std::path::Path::new(&ds.path).exists());
        Dataset::remover(&ds).unwrap()().unwrap();
    }
}
