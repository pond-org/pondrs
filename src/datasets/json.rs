//! JSON file dataset.

use std::prelude::v1::*;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::PondError;
use super::{Dataset, FileDataset};

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

/// JSON file dataset. Loads/saves `serde_json::Value`.
#[derive(Serialize, Deserialize, Clone)]
pub struct JsonDataset {
    path: String,
}

impl JsonDataset {
    pub fn new(path: impl Into<String>) -> Self {
        Self { path: path.into() }
    }
}

impl Dataset for JsonDataset {
    type LoadItem = Value;
    type SaveItem = Value;
    type Error = PondError;

    fn load(&self) -> Result<Self::LoadItem, PondError> {
        let content = std::fs::read_to_string(&self.path)?;
        Ok(serde_json::from_str(&content)?)
    }

    fn save(&self, value: Self::SaveItem) -> Result<(), PondError> {
        std::fs::write(&self.path, serde_json::to_string_pretty(&value)?)?;
        Ok(())
    }

    fn html(&self) -> Option<String> {
        let content = std::fs::read_to_string(&self.path).ok()?;
        let value: Value = serde_json::from_str(&content).ok()?;
        let pretty = serde_json::to_string_pretty(&value).ok()?;
        Some(format!(
            "<pre style=\"font-family:monospace;font-size:13px;padding:8px;margin:0;overflow:auto\">{}</pre>",
            html_escape(&pretty)
        ))
    }
}

impl FileDataset for JsonDataset {
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
    use serde_json::json;
    use tempfile::tempdir;

    #[test]
    fn round_trip() {
        let dir = tempdir().unwrap();
        let ds = JsonDataset::new(dir.path().join("data.json").to_str().unwrap());
        let value = json!({"key": "value", "num": 42});
        ds.save(value.clone()).unwrap();
        assert_eq!(ds.load().unwrap(), value);
    }

    #[test]
    fn html_is_none_before_save() {
        let dir = tempdir().unwrap();
        let ds = JsonDataset::new(dir.path().join("data.json").to_str().unwrap());
        let meta: &dyn DatasetMeta = &ds;
        assert!(meta.html().is_none());
    }

    #[test]
    fn html_contains_pretty_json_after_save() {
        let dir = tempdir().unwrap();
        let ds = JsonDataset::new(dir.path().join("data.json").to_str().unwrap());
        ds.save(json!({"a": 1})).unwrap();
        let meta: &dyn DatasetMeta = &ds;
        let html = meta.html().unwrap();
        assert!(html.contains("<pre"));
        assert!(html.contains("\"a\""));
    }
}
