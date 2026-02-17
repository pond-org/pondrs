//! Dataset types for loading and saving data.

mod memory;
mod param;
mod polars;
mod yaml;

pub use memory::MemoryDataset;
pub use param::Param;
pub use polars::PolarsDataset;
pub use yaml::YamlDataset;

/// Trait for datasets that can load and save data.
pub trait Dataset {
    type LoadItem;
    type SaveItem;

    fn load(&self) -> Option<Self::LoadItem>;
    fn save(&self, output: Self::SaveItem);
}
