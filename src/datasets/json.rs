//! JSON file dataset.

use std::prelude::v1::*;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::PondError;
use super::{Dataset, FileDataset};

/// JSON file dataset. Loads/saves `serde_json::Value`.
#[derive(Debug, Serialize, Deserialize, Clone)]
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
        self.ensure_parent_dir()?;
        std::fs::write(&self.path, serde_json::to_string_pretty(&value)?)?;
        Ok(())
    }

    fn content_hash(&self) -> Option<u64> { self.file_content_hash() }
    fn is_persistent(&self) -> bool { true }
    fn remover(&self) -> Option<super::Remover> { self.file_remover() }

    fn html(&self) -> Option<String> {
        json_html(&self.path, HTML_PREVIEW_LIMIT)
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

/// Files larger than this many bytes get a size notice instead of a
/// pretty-printed preview from `html()`, so viz doesn't stall reading them.
pub(crate) const HTML_PREVIEW_LIMIT: u64 = 16 * 1024 * 1024;

/// Arrays in an `html()` preview show at most this many items, followed by a
/// `"… N more items"` marker.
pub(crate) const HTML_PREVIEW_ITEMS: usize = 50;

/// Pretty-printed `<pre>` preview of the JSON file at `path`, or a short size
/// notice if the file is larger than `limit` bytes. Arrays at any depth are
/// cut to [`HTML_PREVIEW_ITEMS`]. `None` if the file is missing or not valid
/// JSON.
pub(crate) fn json_html(path: &str, limit: u64) -> Option<String> {
    const PRE: &str = "<pre style=\"font-family:monospace;font-size:13px;padding:8px;margin:0;overflow:auto\">";
    let size = std::fs::metadata(path).ok()?.len();
    if size > limit {
        return Some(format!(
            "{PRE}{}: {size} bytes, too large to preview (limit {limit} bytes)</pre>",
            super::html_escape(path)
        ));
    }
    let bytes = std::fs::read(path).ok()?;
    let mut value: Value = serde_json::from_slice(&bytes).ok()?;
    drop(bytes);
    truncate_arrays(&mut value, HTML_PREVIEW_ITEMS);
    let pretty = serde_json::to_string_pretty(&value).ok()?;
    Some(format!("{PRE}{}</pre>", super::html_escape(&pretty)))
}

/// Cuts every array in `value` to `max` items, appending a string marker that
/// counts the dropped ones.
fn truncate_arrays(value: &mut Value, max: usize) {
    match value {
        Value::Array(items) => {
            let dropped = items.len().saturating_sub(max);
            items.truncate(max);
            for v in items.iter_mut() {
                truncate_arrays(v, max);
            }
            if dropped > 0 {
                items.push(Value::String(format!("… {dropped} more items")));
            }
        }
        Value::Object(fields) => {
            for v in fields.values_mut() {
                truncate_arrays(v, max);
            }
        }
        _ => {}
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

    #[test]
    fn html_truncates_long_arrays_at_any_depth() {
        let dir = tempdir().unwrap();
        let ds = JsonDataset::new(dir.path().join("data.json").to_str().unwrap());
        let top: Vec<_> = (0..120).map(|i| json!({"id": i})).collect();
        let nested: Vec<_> = (1000..1060).collect();
        ds.save(json!({"top": top, "inner": {"nested": nested}, "short": [1, 2]})).unwrap();
        let html = Dataset::html(&ds).unwrap();
        assert!(html.contains("\"id\": 49"));
        assert!(!html.contains("\"id\": 50"));
        assert!(html.contains("… 70 more items"));
        assert!(html.contains("1049"));
        assert!(!html.contains("1050"));
        assert!(html.contains("… 10 more items"));
        assert_eq!(html.matches("more items").count(), 2);
    }
}
