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
}

#[cfg(feature = "std")]
pub trait FileDataset: Dataset + Clone {
    fn get_path(&self) -> &str;
    fn set_path(&mut self, path: &str);
}
