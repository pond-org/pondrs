//! Dataset types for loading and saving data.

#[cfg(feature = "std")]
use std::prelude::v1::*;

mod cell;
#[cfg(feature = "std")]
mod memory;
mod param;
#[cfg(feature = "polars")]
mod partitioned;
#[cfg(feature = "polars")]
mod polars;
#[cfg(feature = "yaml")]
mod yaml;
#[cfg(feature = "plotly")]
mod plotly_dataset;

pub use cell::CellDataset;
#[cfg(feature = "std")]
pub use memory::MemoryDataset;
pub use param::Param;
#[cfg(feature = "polars")]
pub use partitioned::Lazy;
#[cfg(feature = "polars")]
pub use partitioned::{LazyPartitionedDataset, PartitionedDataset};
#[cfg(feature = "polars")]
pub use polars::{PolarsCsvDataset, PolarsParquetDataset};
#[cfg(feature = "yaml")]
pub use yaml::YamlDataset;
#[cfg(feature = "plotly")]
pub use plotly_dataset::PlotlyDataset;

/// Trait for datasets that can load and save data.
///
/// Each dataset declares its own `Error` type. Infallible datasets (like `Param`)
/// use `core::convert::Infallible`. The framework converts dataset errors to the
/// pipeline's error type via `PondError: From<Self::Error>`.
pub trait Dataset {
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

    #[cfg(feature = "std")]
    fn html(&self) -> Option<String>;
}

impl<T: Dataset + Send + Sync> DatasetMeta for T {
    fn is_param(&self) -> bool { <T as Dataset>::is_param(self) }

    #[cfg(feature = "std")]
    fn html(&self) -> Option<String> { <T as Dataset>::html(self) }
}

#[cfg(feature = "std")]
pub trait FileDataset: Dataset + Clone {
    fn get_path(&self) -> &str;
    fn set_path(&mut self, path: &str);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::{DatasetRef, Node, ptr_to_id};

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
    fn param_html_is_none() {
        let p = Param(1i32);
        let meta: &dyn DatasetMeta = &p;
        assert!(meta.html().is_none());
    }

    #[cfg(feature = "std")]
    #[test]
    fn memory_dataset_html_is_none() {
        let ds = MemoryDataset::<i32>::new();
        let meta: &dyn DatasetMeta = &ds;
        assert!(meta.html().is_none());
    }

    // ── DatasetRef::new ──────────────────────────────────────────────────────

    #[test]
    fn dataset_ref_new_id_matches_ptr() {
        let ds = CellDataset::<i32>::new();
        let r = DatasetRef::new(&ds);
        assert_eq!(r.id, ptr_to_id(&ds));
        assert!(!r.meta.is_param());
    }

    #[test]
    fn dataset_ref_new_param() {
        let p = Param(99i32);
        let r = DatasetRef::new(&p);
        assert_eq!(r.id, ptr_to_id(&p));
        assert!(r.meta.is_param());
    }

    // ── pipeline walk collects &dyn DatasetMeta ──────────────────────────────

    #[cfg(feature = "std")]
    #[test]
    fn pipeline_walk_collects_meta_refs() {
        use crate::StepInfo;
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
        pipe.for_each_info(&mut |item: &dyn crate::PipelineInfo| {
            item.for_each_input_id(&mut |d| { meta_map.insert(d.id, d.meta.is_param()); });
            item.for_each_output_id(&mut |d| { meta_map.insert(d.id, d.meta.is_param()); });
        });

        // param, a, b should all be present
        assert_eq!(meta_map.len(), 3);
        assert_eq!(meta_map[&ptr_to_id(&param)], true);
        assert_eq!(meta_map[&ptr_to_id(&a)], false);
        assert_eq!(meta_map[&ptr_to_id(&b)], false);
    }
}
