//! Dataset types for loading and saving data.

#[cfg(feature = "std")]
use std::prelude::v1::*;

mod cell;
mod gpio;
#[cfg(feature = "std")]
mod cache;
#[cfg(feature = "std")]
mod memory;
mod param;
mod register;
#[cfg(feature = "std")]
mod lazy;
#[cfg(feature = "std")]
mod partitioned;
#[cfg(feature = "std")]
mod thunk;
#[cfg(feature = "polars")]
mod polars;
#[cfg(feature = "json")]
mod json;
#[cfg(feature = "std")]
mod text;
#[cfg(feature = "yaml")]
mod yaml;
#[cfg(feature = "plotly")]
mod plotly_dataset;
#[cfg(feature = "image")]
mod image_dataset;
#[cfg(feature = "std")]
mod templated;

pub use cell::CellDataset;
pub use gpio::GpioDataset;
#[cfg(feature = "std")]
pub use cache::CacheDataset;
#[cfg(feature = "std")]
pub use memory::MemoryDataset;
pub use param::Param;
pub use register::RegisterDataset;
#[cfg(feature = "std")]
pub use lazy::{Lazy, LazyDataset};
#[cfg(feature = "std")]
pub use lazy::LazyPartitionedDataset;
#[cfg(feature = "std")]
pub use partitioned::PartitionedDataset;
#[cfg(feature = "std")]
pub use thunk::{Thunk, IntoThunk, FromThunk};
#[cfg(feature = "polars")]
pub use polars::{PolarsCsvDataset, PolarsExcelDataset, PolarsParquetDataset};
#[cfg(feature = "json")]
pub use json::JsonDataset;
#[cfg(feature = "std")]
pub use text::TextDataset;
#[cfg(feature = "yaml")]
pub use yaml::YamlDataset;
#[cfg(feature = "plotly")]
pub use plotly_dataset::PlotlyDataset;
#[cfg(feature = "image")]
pub use image_dataset::ImageDataset;
#[cfg(feature = "std")]
pub use templated::TemplatedCatalog;

/// Trait for datasets that can load and save data.
///
/// Each dataset declares its own `Error` type. Infallible datasets (like `Param`)
/// use `core::convert::Infallible`. The framework converts dataset errors to the
/// pipeline's error type via `PondError: From<Self::Error>`.
///
/// `Serialize` is a supertrait so that `DatasetMeta::yaml()` can automatically
/// serialize any dataset's configuration to YAML.
pub trait Dataset: serde::Serialize {
    type LoadItem: 'static;
    type SaveItem: 'static;
    type Error;

    fn load(&self) -> Result<Self::LoadItem, Self::Error>;
    fn save(&self, output: Self::SaveItem) -> Result<(), Self::Error>;
    fn is_param(&self) -> bool { false }
    fn content_hash(&self) -> Option<u64> { None }
    fn is_persistent(&self) -> bool { false }

    /// Returns the dataset's HTML representation, if available.
    /// Override in datasets that can produce HTML (e.g. `PlotlyDataset`).
    #[cfg(feature = "std")]
    fn html(&self) -> Option<String> { None }
}

/// Object-safe metadata trait for datasets. Blanket-implemented for all `Dataset` types.
/// Enables collecting `&dyn DatasetMeta` references without knowing concrete types.
pub trait DatasetMeta: Send + Sync {
    fn is_param(&self) -> bool;
    fn content_hash(&self) -> Option<u64>;
    fn is_persistent(&self) -> bool;
    fn type_string(&self) -> &'static str;

    #[cfg(feature = "std")]
    fn html(&self) -> Option<String>;

    #[cfg(feature = "std")]
    fn yaml(&self) -> Option<String>;
}

impl<T: Dataset + Send + Sync> DatasetMeta for T {
    fn is_param(&self) -> bool { <T as Dataset>::is_param(self) }
    fn content_hash(&self) -> Option<u64> { <T as Dataset>::content_hash(self) }
    fn is_persistent(&self) -> bool { <T as Dataset>::is_persistent(self) }
    fn type_string(&self) -> &'static str { core::any::type_name::<T>() }

    #[cfg(feature = "std")]
    fn html(&self) -> Option<String> { <T as Dataset>::html(self) }

    #[cfg(feature = "std")]
    fn yaml(&self) -> Option<String> { serde_yaml::to_string(self).ok() }
}

/// A dataset backed by a file on disk.
///
/// Used by `PartitionedDataset` to clone a template dataset and
/// point each partition at a different file path.
#[cfg(feature = "std")]
pub trait FileDataset: Dataset + Clone {
    /// The file path this dataset reads from / writes to.
    fn path(&self) -> &str;
    /// Redirect this dataset to a different file path.
    fn set_path(&mut self, path: &str);

    /// Whether `PartitionedDataset` should use rayon for parallel save.
    /// Default: `false`. `LazyDataset` overrides to `true`.
    fn prefer_parallel(&self) -> bool { false }

    fn file_content_hash(&self) -> Option<u64> {
        let meta = std::fs::metadata(self.path()).ok()?;
        let mtime = meta.modified().ok()?
            .duration_since(std::time::UNIX_EPOCH).ok()?
            .as_nanos();
        use core::hash::{Hash, Hasher};
        let mut hasher = std::hash::DefaultHasher::new();
        let canonical = std::fs::canonicalize(self.path()).ok()?;
        canonical.hash(&mut hasher);
        mtime.hash(&mut hasher);
        Some(hasher.finish())
    }

    /// Creates parent directories for `self.path()` if they don't exist.
    fn ensure_parent_dir(&self) -> Result<(), std::io::Error> {
        if let Some(parent) = std::path::Path::new(self.path()).parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)?;
            }
        }
        Ok(())
    }

    /// List entry names in a partition. Default scans the directory at `path`
    /// for files matching `ext` and returns their stems, sorted.
    fn list_entries(
        &self,
        path: &str,
        ext: &str,
    ) -> Result<Vec<String>, crate::error::PondError> {
        let dir = std::path::Path::new(path);
        let mut names: Vec<String> = std::fs::read_dir(dir)?
            .filter_map(|e| e.ok())
            .filter_map(|e| {
                let file_name = e.file_name().to_string_lossy().into_owned();
                if file_name.ends_with(ext) {
                    let entry_path = e.path();
                    entry_path.file_stem().and_then(|s| s.to_str()).map(|s| s.to_string())
                } else {
                    None
                }
            })
            .collect();
        names.sort();
        Ok(names)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pipeline::{DatasetRef, Node, ptr_to_id};

    // ── blanket impl: is_param ───────────────────────────────────────────────

    #[test]
    fn param_is_param_via_meta() {
        let p = Param(42i32);
        let meta: &dyn DatasetMeta = &p;
        assert!(meta.is_param());
    }

    #[test]
    fn cell_dataset_not_param_via_meta() {
        let ds = CellDataset::<i32>::new();
        let meta: &dyn DatasetMeta = &ds;
        assert!(!meta.is_param());
    }

    #[cfg(feature = "std")]
    #[test]
    fn memory_dataset_not_param_via_meta() {
        let ds = MemoryDataset::<i32>::new();
        let meta: &dyn DatasetMeta = &ds;
        assert!(!meta.is_param());
    }

    // ── default html() returns None ──────────────────────────────────────────

    #[cfg(feature = "std")]
    #[test]
    fn cell_dataset_html_is_none() {
        let ds = CellDataset::<i32>::new();
        let meta: &dyn DatasetMeta = &ds;
        assert!(meta.html().is_none());
    }

    #[cfg(feature = "std")]
    #[test]
    fn param_html_is_some() {
        let p = Param(1i32);
        let meta: &dyn DatasetMeta = &p;
        assert!(meta.html().is_some());
    }

    #[cfg(feature = "std")]
    #[test]
    fn memory_dataset_html_is_none() {
        let ds = MemoryDataset::<i32>::new();
        let meta: &dyn DatasetMeta = &ds;
        assert!(meta.html().is_none());
    }

    // ── DatasetRef::from_ref ─────────────────────────────────────────────────

    #[test]
    fn dataset_ref_from_ref_id_matches_ptr() {
        let ds = CellDataset::<i32>::new();
        let r = DatasetRef::from_ref(&ds);
        assert_eq!(r.id, ptr_to_id(&ds));
        assert!(!r.meta.is_param());
    }

    #[test]
    fn dataset_ref_from_ref_param() {
        let p = Param(99i32);
        let r = DatasetRef::from_ref(&p);
        assert_eq!(r.id, ptr_to_id(&p));
        assert!(r.meta.is_param());
    }

    // ── pipeline walk collects &dyn DatasetMeta ──────────────────────────────

    #[cfg(feature = "std")]
    #[test]
    fn pipeline_walk_collects_meta_refs() {
        use crate::PipelineInfo;
        use std::collections::HashMap;

        let param = Param(1i32);
        let a = CellDataset::<i32>::new();
        let b = CellDataset::<i32>::new();

        let pipe = (
            Node { name: "n1", func: |v| (v,), input: (&param,), output: (&a,) },
            Node { name: "n2", func: |v| (v,), input: (&a,), output: (&b,) },
        );

        // Walk the pipeline and collect all DatasetRef ids → is_param
        let mut meta_map: HashMap<usize, bool> = HashMap::new();
        pipe.for_each_info(&mut |item: &dyn crate::StepInfo| {
            item.for_each_input(&mut |d| { meta_map.insert(d.id, d.meta.is_param()); });
            item.for_each_output(&mut |d| { meta_map.insert(d.id, d.meta.is_param()); });
        });

        // param, a, b should all be present
        assert_eq!(meta_map.len(), 3);
        assert_eq!(meta_map[&ptr_to_id(&param)], true);
        assert_eq!(meta_map[&ptr_to_id(&a)], false);
        assert_eq!(meta_map[&ptr_to_id(&b)], false);
    }

    // ── content_hash / is_persistent ────────────────────────────────────────

    #[cfg(feature = "std")]
    #[test]
    fn param_content_hash_deterministic() {
        let p = Param(42i32);
        let h1 = Dataset::content_hash(&p);
        let h2 = Dataset::content_hash(&p);
        assert!(h1.is_some());
        assert_eq!(h1, h2);
    }

    #[cfg(feature = "std")]
    #[test]
    fn param_content_hash_differs_for_different_values() {
        assert_ne!(Dataset::content_hash(&Param(42i32)), Dataset::content_hash(&Param(43i32)));
    }

    #[test]
    fn param_is_persistent() {
        assert!(Dataset::is_persistent(&Param(42i32)));
    }

    #[cfg(feature = "std")]
    #[test]
    fn memory_dataset_content_hash_is_none() {
        assert!(Dataset::content_hash(&MemoryDataset::<i32>::new()).is_none());
    }

    #[cfg(feature = "std")]
    #[test]
    fn memory_dataset_not_persistent() {
        assert!(!Dataset::is_persistent(&MemoryDataset::<i32>::new()));
    }

    #[test]
    fn cell_dataset_content_hash_is_none() {
        assert!(Dataset::content_hash(&CellDataset::<i32>::new()).is_none());
    }

    #[test]
    fn cell_dataset_not_persistent() {
        assert!(!Dataset::is_persistent(&CellDataset::<i32>::new()));
    }

    #[cfg(feature = "std")]
    #[test]
    fn text_dataset_content_hash_none_before_write() {
        let dir = tempfile::tempdir().unwrap();
        let ds = TextDataset::new(dir.path().join("x.txt").to_str().unwrap());
        assert!(Dataset::content_hash(&ds).is_none());
    }

    #[cfg(feature = "std")]
    #[test]
    fn text_dataset_content_hash_some_after_write() {
        let dir = tempfile::tempdir().unwrap();
        let ds = TextDataset::new(dir.path().join("x.txt").to_str().unwrap());
        ds.save("hello".into()).unwrap();
        assert!(Dataset::content_hash(&ds).is_some());
    }

    #[cfg(feature = "std")]
    #[test]
    fn text_dataset_is_persistent() {
        let ds = TextDataset::new("/tmp/nonexistent.txt");
        assert!(Dataset::is_persistent(&ds));
    }

    #[cfg(feature = "std")]
    #[test]
    fn dataset_meta_delegates_content_hash_and_persistent() {
        let p = Param(42i32);
        let meta: &dyn DatasetMeta = &p;
        assert_eq!(meta.content_hash(), Dataset::content_hash(&p));
        assert_eq!(meta.is_persistent(), Dataset::is_persistent(&p));

        let ds = TextDataset::new("/tmp/nonexistent.txt");
        let meta: &dyn DatasetMeta = &ds;
        assert_eq!(meta.content_hash(), Dataset::content_hash(&ds));
        assert_eq!(meta.is_persistent(), Dataset::is_persistent(&ds));

        let m = MemoryDataset::<i32>::new();
        let meta: &dyn DatasetMeta = &m;
        assert_eq!(meta.content_hash(), Dataset::content_hash(&m));
        assert_eq!(meta.is_persistent(), Dataset::is_persistent(&m));
    }
}
