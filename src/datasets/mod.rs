//! Dataset types for loading and saving data.

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

/// Trait for datasets that can load and save data.
pub trait Dataset {
    type LoadItem;
    type SaveItem;

    fn load(&self) -> Option<Self::LoadItem>;
    fn save(&self, output: Self::SaveItem);
    fn is_param(&self) -> bool { false }
}

#[cfg(feature = "std")]
pub trait FileDataset: Dataset + Clone {
    fn get_path(&self) -> &str;
    fn set_path(&mut self, path: &str);
}
