//! Dataset types for loading and saving data.

mod memory;
mod param;
mod partitioned;
mod polars;
mod yaml;

pub use memory::MemoryDataset;
pub use param::Param;
pub use partitioned::Lazy;
pub use partitioned::{LazyPartitionedDataset, PartitionedDataset};
pub use polars::{PolarsCsvDataset, PolarsParquetDataset};
pub use yaml::YamlDataset;

/// Trait for datasets that can load and save data.
pub trait Dataset {
    type LoadItem;
    type SaveItem;

    fn load(&self) -> Option<Self::LoadItem>;
    fn save(&self, output: Self::SaveItem);
}

pub trait FileDataset: Dataset + Clone {
    fn get_path(&self) -> &str;
    fn set_path(&mut self, path: &str);
}
