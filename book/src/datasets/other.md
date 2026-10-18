# List of Datasets

All built-in dataset types, their feature flags, and typical use cases.

## Core datasets (no feature flags)

| Type | Load/Save types | Description |
|------|----------------|-------------|
| `Param<T>` | `T` / `Never` | Read-only parameter. Error: `PondError` (never returned). |
| `CellDataset<T>` | `T` / `T` | `no_std` intermediate storage. `T: Copy` only. |

## `std` datasets

| Type | Feature | Load/Save types | Description |
|------|---------|----------------|-------------|
| `MemoryDataset<T>` | `std` | `T` / `T` | Thread-safe in-memory storage via `Arc<Mutex<_>>`. |
| `TextDataset` | `std` | `String` / `String` | Reads/writes plain text files. |
| `LazyDataset<D>` | `std` | `Loader<D::LoadItem>` / `Lazy<D::SaveItem>` | Deferred wrapper. See [Lazy Dataset](./lazy.md). |
| `CacheDataset<D>` | `std` | `D::LoadItem` / `D::SaveItem` | Caching wrapper for any dataset. |

## File format datasets

| Type | Feature | Load/Save types | Description |
|------|---------|----------------|-------------|
| `PolarsCsvDataset` | `polars` | `DataFrame` / `DataFrame` | CSV files via Polars. Configurable separator, header, etc. |
| `PolarsParquetDataset` | `polars` | `DataFrame` / `DataFrame` | Parquet files via Polars. |
| `PolarsExcelDataset` | `polars` | `DataFrame` / — | Excel files via fastexcel. Read-only. |
| `JsonDataset` | `json` | `serde_json::Value` / `serde_json::Value` | JSON files. |
| `TypedJsonDataset<T>` | `json` | `T` / `T` | JSON files parsed straight into a serde type. See [below](#typed-json). |
| `YamlDataset` | `yaml` | `Vec<Yaml>` / `Vec<Yaml>` | YAML files using `yaml_rust2`. |
| `PlotlyDataset` | `plotly` | `serde_json::Value` / `serde_json::Value` | Plotly charts. Saves `.json` + `.html`. Custom `html()` for viz. |
| `ImageDataset` | `image` | `DynamicImage` / `DynamicImage` | Image files via the `image` crate. |

### Typed JSON

`JsonDataset` loads a `serde_json::Value`, which suits small or schemaless files. For large files, or ones with a known shape, use `TypedJsonDataset<T>`: the file is parsed directly into `T`, and fields `T` doesn't declare are skipped while parsing, so they never take up memory. A `Value` tree costs many times the file size (COCO's 448 MB `instances_train2017.json` takes about 4 GB), while a struct declaring only the fields a pipeline needs stays small. Peak memory during a load is the file size plus `T`. Nodes also get a typed value, with no `serde_json::from_value` step.

```rust,ignore
#[derive(Serialize, Deserialize)]
struct Catalog {
    annotations: TypedJsonDataset<CocoAnnotations>,
}
```

```yaml
annotations: { path: "data/coco/annotations/instances_train2017.json" }
```

`T` must be `Serialize + DeserializeOwned` for the dataset to load and save, but the catalog entry itself only holds a path. The dataset is `Send + Sync + Clone` whatever `T` is, and it implements `FileDataset`, so it works inside `PartitionedDataset`.

For both JSON datasets, viz shows only the path and size of files over 1 MiB, rather than reading and pretty-printing them.

## Partitioned datasets

| Type | Feature | Load/Save types | Description |
|------|---------|----------------|-------------|
| `PartitionedDataset<D>` | `std` | `BTreeMap<String, D::LoadItem>` / `BTreeMap<String, D::SaveItem>` | Directory of files, eagerly loaded. |
| `LazyPartitionedDataset<D>` | `std` | `BTreeMap<String, Loader<D::LoadItem>>` / `BTreeMap<String, Lazy<D::SaveItem>>` | Directory of files, lazily loaded on demand. |

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

Implementors: `PolarsCsvDataset`, `PolarsParquetDataset`, `PolarsExcelDataset`, `TextDataset`, `JsonDataset`, `TypedJsonDataset<T>`, `ImageDataset`, `LazyDataset<D>`.
