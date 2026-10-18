//! Typed JSON file dataset.

use std::prelude::v1::*;
use core::fmt;
use core::marker::PhantomData;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

use crate::error::PondError;
use super::{Dataset, FileDataset};
use super::json::{json_html, HTML_PREVIEW_LIMIT};

/// JSON file dataset that loads and saves a `T` through serde.
///
/// Unlike [`JsonDataset`](super::JsonDataset), which goes through
/// `serde_json::Value`, the file is parsed straight into `T`, so fields `T`
/// doesn't declare are skipped while parsing and never held in memory. Peak
/// memory during a load is the file size (the raw bytes, read in one go) plus
/// the parsed `T`.
///
/// The catalog form is just `{ path: ... }`; `T` need not be `Serialize`,
/// `Deserialize`, `Send`, `Sync` or `Clone` for the dataset itself to be.
#[derive(Serialize, Deserialize)]
#[serde(bound = "")]
pub struct TypedJsonDataset<T> {
    path: String,
    #[serde(skip)]
    _type: PhantomData<fn() -> T>,
}

impl<T> TypedJsonDataset<T> {
    pub fn new(path: impl Into<String>) -> Self {
        Self { path: path.into(), _type: PhantomData }
    }
}

impl<T> Clone for TypedJsonDataset<T> {
    fn clone(&self) -> Self {
        Self::new(self.path.clone())
    }
}

impl<T> fmt::Debug for TypedJsonDataset<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TypedJsonDataset").field("path", &self.path).finish()
    }
}

impl<T> Dataset for TypedJsonDataset<T>
where
    T: Serialize + DeserializeOwned + 'static,
{
    type LoadItem = T;
    type SaveItem = T;
    type Error = PondError;

    fn load(&self) -> Result<Self::LoadItem, PondError> {
        // `from_slice` over the whole file is much faster than `from_reader`,
        // even with a `BufReader`; the bytes are dropped on return.
        let bytes = std::fs::read(&self.path)?;
        Ok(serde_json::from_slice(&bytes)?)
    }

    fn save(&self, value: Self::SaveItem) -> Result<(), PondError> {
        self.ensure_parent_dir()?;
        std::fs::write(&self.path, serde_json::to_vec_pretty(&value)?)?;
        Ok(())
    }

    fn content_hash(&self) -> Option<u64> { self.file_content_hash() }
    fn is_persistent(&self) -> bool { true }
    fn remover(&self) -> Option<super::Remover> { self.file_remover() }

    fn html(&self) -> Option<String> {
        // Through `Value`, so the preview shows the whole file, not just `T`'s fields.
        json_html(&self.path, HTML_PREVIEW_LIMIT)
    }
}

impl<T> FileDataset for TypedJsonDataset<T>
where
    T: Serialize + DeserializeOwned + 'static,
{
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
    use std::collections::BTreeMap;
    use std::rc::Rc;
    use crate::datasets::{DatasetMeta, PartitionedDataset};
    use serde_json::json;
    use tempfile::tempdir;

    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    struct Point {
        x: i32,
        label: String,
    }

    fn path_in(dir: &tempfile::TempDir, name: &str) -> String {
        dir.path().join(name).to_str().unwrap().to_string()
    }

    #[test]
    fn round_trip() {
        let dir = tempdir().unwrap();
        let ds = TypedJsonDataset::<Point>::new(path_in(&dir, "p.json"));
        let p = Point { x: 3, label: "a".into() };
        ds.save(Point { x: 3, label: "a".into() }).unwrap();
        assert_eq!(ds.load().unwrap(), p);
    }

    #[test]
    fn unknown_fields_are_skipped() {
        let dir = tempdir().unwrap();
        let path = path_in(&dir, "p.json");
        let value = json!({
            "x": 7,
            "label": "b",
            "segmentation": [[1.0, 2.0, 3.0], [4.0, 5.0]],
            "extra": {"nested": {"deep": [1, 2, 3]}},
        });
        std::fs::write(&path, value.to_string()).unwrap();
        let ds = TypedJsonDataset::<Point>::new(path);
        assert_eq!(ds.load().unwrap(), Point { x: 7, label: "b".into() });
    }

    #[test]
    fn type_mismatch_is_json_error() {
        let dir = tempdir().unwrap();
        let path = path_in(&dir, "p.json");
        std::fs::write(&path, r#"{"x": "not a number", "label": "c"}"#).unwrap();
        let ds = TypedJsonDataset::<Point>::new(path);
        assert!(matches!(ds.load(), Err(PondError::Json(_))));
    }

    #[allow(dead_code, reason = "only the type matters: not Serialize, Deserialize, Send, Sync or Clone")]
    struct NotSerde(Rc<()>);

    #[test]
    fn deserializes_from_yaml_without_t_bounds() {
        let ds: TypedJsonDataset<NotSerde> = serde_yaml::from_str("path: a.json").unwrap();
        assert_eq!(ds.path, "a.json");
        assert_eq!(serde_yaml::to_string(&ds).unwrap().trim(), "path: a.json");
    }

    #[test]
    fn send_sync_clone_regardless_of_t() {
        fn assert_bounds<D: Send + Sync + Clone>() {}
        assert_bounds::<TypedJsonDataset<Rc<()>>>();
        assert_bounds::<TypedJsonDataset<NotSerde>>();
    }

    #[test]
    fn html_is_none_before_save() {
        let dir = tempdir().unwrap();
        let ds = TypedJsonDataset::<Point>::new(path_in(&dir, "p.json"));
        let meta: &dyn DatasetMeta = &ds;
        assert!(meta.html().is_none());
    }

    #[test]
    fn html_contains_pretty_json_after_save() {
        let dir = tempdir().unwrap();
        let ds = TypedJsonDataset::<Point>::new(path_in(&dir, "p.json"));
        ds.save(Point { x: 1, label: "<a>".into() }).unwrap();
        let meta: &dyn DatasetMeta = &ds;
        let html = meta.html().unwrap();
        assert!(html.contains("<pre"));
        assert!(html.contains("\"x\": 1"));
        assert!(html.contains("&lt;a&gt;"));
    }

    #[test]
    fn html_shows_fields_t_does_not_declare() {
        let dir = tempdir().unwrap();
        let path = path_in(&dir, "p.json");
        std::fs::write(&path, r#"{"x": 1, "label": "a", "extra": true}"#).unwrap();
        let ds = TypedJsonDataset::<Point>::new(path);
        assert!(Dataset::html(&ds).unwrap().contains("\"extra\""));
    }

    #[test]
    fn html_over_limit_shows_only_size() {
        let dir = tempdir().unwrap();
        let path = path_in(&dir, "p.json");
        let content = r#"{"x": 1, "label": "secret"}"#;
        std::fs::write(&path, content).unwrap();
        let html = json_html(&path, 10).unwrap();
        assert!(!html.contains("secret"));
        assert!(html.contains(&format!("{} bytes", content.len())));
        // Within the limit, the same file previews in full.
        assert!(json_html(&path, HTML_PREVIEW_LIMIT).unwrap().contains("secret"));
    }

    #[test]
    fn works_inside_partitioned_dataset() {
        let dir = tempdir().unwrap();
        let ds = PartitionedDataset {
            path: path_in(&dir, "parts"),
            ext: "json".into(),
            dataset: TypedJsonDataset::<Point>::new(""),
        };
        let entries: BTreeMap<String, Point> = [
            ("a".to_string(), Point { x: 1, label: "a".into() }),
            ("b".to_string(), Point { x: 2, label: "b".into() }),
        ]
        .into_iter()
        .collect();
        ds.save(entries).unwrap();
        let loaded = ds.load().unwrap();
        assert_eq!(loaded.len(), 2);
        assert_eq!(loaded["a"], Point { x: 1, label: "a".into() });
        assert_eq!(loaded["b"], Point { x: 2, label: "b".into() });
    }

    #[test]
    fn catalog_indexer_treats_it_as_leaf() {
        use crate::catalog_indexer::index_catalog;
        use crate::pipeline::ptr_to_id;

        #[derive(Serialize)]
        struct Catalog {
            annotations: TypedJsonDataset<NotSerde>,
        }
        let catalog = Catalog { annotations: TypedJsonDataset::new("a.json") };
        let index = index_catalog(&catalog);
        assert_eq!(index.get(ptr_to_id(&catalog.annotations)), Some("annotations"));
        assert_eq!(index.into_inner().len(), 1);
    }
}
