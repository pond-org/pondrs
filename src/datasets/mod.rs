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
    type LoadItem;
    type SaveItem;
    type Error;

    fn load(&self) -> Result<Self::LoadItem, Self::Error>;
    fn save(&self, output: Self::SaveItem) -> Result<(), Self::Error>;
    fn is_param(&self) -> bool { false }

    /// Returns the dataset's HTML representation, if available.
    /// Override in datasets that can produce HTML (e.g. `PlotlyDataset`).
    #[cfg(feature = "std")]
    fn html(&self) -> Option<String> { None }
}

/// Object-safe metadata trait for datasets. Blanket-implemented for all `Dataset` types.
/// Enables collecting `&dyn DatasetMeta` references without knowing concrete types.
pub trait DatasetMeta: Send + Sync {
    fn is_param(&self) -> bool;
    fn type_string(&self) -> &'static str;

    #[cfg(feature = "std")]
    fn html(&self) -> Option<String>;

    #[cfg(feature = "std")]
    fn yaml(&self) -> Option<String>;
}

impl<T: Dataset + Send + Sync> DatasetMeta for T {
    fn is_param(&self) -> bool { <T as Dataset>::is_param(self) }
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

    /// Creates parent directories for `self.path()` if they don't exist.
    fn ensure_parent_dir(&self) -> Result<(), std::io::Error> {
        if let Some(parent) = std::path::Path::new(self.path()).parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)?;
            }
        }
        Ok(())
    }

    /// Save multiple partitioned entries sequentially. Exposed so overrides
    /// can delegate back to it as a fallback.
    fn default_save_partitioned(
        &self,
        entries: std::collections::HashMap<String, Self::SaveItem>,
        dir: &std::path::Path,
        ext: &str,
    ) -> Result<(), crate::error::PondError>
    where
        crate::error::PondError: From<Self::Error>,
        Self: Send + Sync,
        Self::SaveItem: Send,
        Self::Error: Send,
    {
        for (name, data) in entries {
            let path = dir.join(format!("{name}.{ext}"));
            let mut ds = self.clone();
            ds.set_path(path.to_str().ok_or_else(|| crate::error::PondError::Custom(format!("non-UTF-8 path: {}", path.display())))?);
            ds.save(data)?;
        }
        Ok(())
    }

    /// Save multiple partitioned entries. Default delegates to the sequential
    /// [`default_save_partitioned`](FileDataset::default_save_partitioned);
    /// `LazyDataset` overrides with parallel save via rayon.
    fn save_partitioned(
        &self,
        entries: std::collections::HashMap<String, Self::SaveItem>,
        dir: &std::path::Path,
        ext: &str,
    ) -> Result<(), crate::error::PondError>
    where
        crate::error::PondError: From<Self::Error>,
        Self: Send + Sync,
        Self::SaveItem: Send,
        Self::Error: Send,
    {
        self.default_save_partitioned(entries, dir, ext)
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
}
