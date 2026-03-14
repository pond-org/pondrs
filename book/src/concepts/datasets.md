# Datasets

Datasets are the data abstraction in pondrs. Every piece of data flowing through a pipeline — whether it's a CSV file, an in-memory value, or a hardware register — is a dataset.

## The `Dataset` trait

```rust,ignore
pub trait Dataset: serde::Serialize {
    type LoadItem;
    type SaveItem;
    type Error;

    fn load(&self) -> Result<Self::LoadItem, Self::Error>;
    fn save(&self, output: Self::SaveItem) -> Result<(), Self::Error>;
    fn is_param(&self) -> bool { false }
}
```

- **`LoadItem`** — the type produced when loading (e.g. `DataFrame`, `String`, `f64`)
- **`SaveItem`** — the type accepted when saving (often the same as `LoadItem`)
- **`Error`** — the error type for I/O operations. Use `core::convert::Infallible` for datasets that never fail (like `Param`)
- **`is_param()`** — returns `true` for read-only parameter datasets. The pipeline validator uses this to prevent writing to params.
- **`Serialize` supertrait** — enables automatic YAML serialization of dataset configuration for the viz and catalog indexer.

## Datasets in the minimal example

The catalog uses three dataset types:

```rust,ignore
{{#include ../../../examples/minimal.rs:catalog}}
```

### `PolarsCsvDataset`

Reads and writes CSV files as Polars `DataFrame`s. Requires the `polars` feature. Configured with a file path and optional CSV options like separator:

```yaml
readings:
  path: data/readings.csv
  separator: ","
```

### `MemoryDataset<T>`

Thread-safe in-memory storage for intermediate values. Starts empty — loading before any save returns `DatasetNotLoaded`. Requires the `std` feature. Uses `Arc<Mutex<Option<T>>>` internally, so it works safely with the `ParallelRunner`.

```yaml
summary: {}
```

### `JsonDataset`

Reads and writes JSON files as `serde_json::Value`. Requires the `json` feature.

```yaml
report:
  path: data/report.json
```

## Further reading

- [Custom Datasets](../datasets/custom_datasets.md) — how to implement your own dataset type
- [List of Datasets](../datasets/other.md) — all built-in dataset types and their feature flags
- [Error handling](../error_handling/datasets.md) — how dataset errors are handled
- [no_std Datasets](../no_std/datasets.md) — datasets available without the standard library
