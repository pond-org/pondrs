# List of Datasets

All built-in dataset types, their feature flags, and typical use cases.

## Core datasets (no feature flags)

| Type | Load/Save types | Description |
|------|----------------|-------------|
| `Param<T>` | `T` / `()` | Read-only parameter. Error: `Infallible`. |
| `CellDataset<T>` | `T` / `T` | `no_std` intermediate storage. `T: Copy` only. |

## `std` datasets

| Type | Feature | Load/Save types | Description |
|------|---------|----------------|-------------|
| `MemoryDataset<T>` | `std` | `T` / `T` | Thread-safe in-memory storage via `Arc<Mutex<_>>`. |
| `TextDataset` | `std` | `String` / `String` | Reads/writes plain text files. |
| `CacheDataset<D>` | `std` | `D::LoadItem` / `D::SaveItem` | Caching wrapper for any dataset. |

## File format datasets

| Type | Feature | Load/Save types | Description |
|------|---------|----------------|-------------|
| `PolarsCsvDataset` | `polars` | `DataFrame` / `DataFrame` | CSV files via Polars. Configurable separator, header, etc. |
| `PolarsParquetDataset` | `polars` | `DataFrame` / `DataFrame` | Parquet files via Polars. |
| `PolarsExcelDataset` | `polars` | `DataFrame` / — | Excel files via fastexcel. Read-only. |
| `JsonDataset` | `json` | `serde_json::Value` / `serde_json::Value` | JSON files. |
| `YamlDataset` | `yaml` | `Vec<Yaml>` / `Vec<Yaml>` | YAML files using `yaml_rust2`. |
| `PlotlyDataset` | `plotly` | `serde_json::Value` / `serde_json::Value` | Plotly charts. Saves `.json` + `.html`. Custom `html()` for viz. |
| `ImageDataset` | `image` | `DynamicImage` / `DynamicImage` | Image files via the `image` crate. |

## Partitioned datasets

| Type | Feature | Load/Save types | Description |
|------|---------|----------------|-------------|
| `PartitionedDataset<D>` | `polars` | `HashMap<String, D::LoadItem>` / `HashMap<String, D::SaveItem>` | Directory of files, eagerly loaded. |
| `LazyPartitionedDataset<D>` | `polars` | `HashMap<String, Lazy<D::LoadItem>>` / `HashMap<String, D::SaveItem>` | Directory of files, lazily loaded on demand. |

## Hardware datasets (no_std)

| Type | Feature | Load/Save types | Description |
|------|---------|----------------|-------------|
| `RegisterDataset<T>` | — | `T` / `T` | Volatile memory-mapped register. `T: Copy`. |
| `GpioDataset` | — | `bool` / `bool` | Single GPIO pin within a memory-mapped register. |

## Common traits

All file-backed datasets that support `PartitionedDataset` implement `FileDataset`:

```rust,ignore
pub trait FileDataset: Dataset + Clone {
    fn path(&self) -> &str;
    fn set_path(&mut self, path: &str);
}
```

Implementors: `PolarsCsvDataset`, `PolarsParquetDataset`, `PolarsExcelDataset`, `TextDataset`, `JsonDataset`.
