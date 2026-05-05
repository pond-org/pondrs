# Partitioned Dataset

`PartitionedDataset` represents a directory of files where each file is treated as a separate partition.

*Requires the `std` feature.*

## Definition

```rust,ignore
pub struct PartitionedDataset<D: FileDataset> {
    pub path: String,
    pub ext: String,
    pub dataset: D,
}
```

- **`path`** — the directory to read from / write to
- **`ext`** — file extension to filter by (e.g. `"csv"`, `"parquet"`, `"txt"`)
- **`dataset`** — a template dataset that is cloned and pointed at each file

## Loading

Returns `HashMap<String, D::LoadItem>` where keys are filename stems:

```text
data/partitions/
  january.csv
  february.csv
  march.csv
```

```rust,ignore
// loads as HashMap { "february" => ..., "january" => ..., "march" => ... }
```

The template dataset is cloned for each file, its path is set to the full file path, and `load()` is called on the clone.

## Saving

Accepts `HashMap<String, D::SaveItem>` and writes each entry as `{path}/{name}.{ext}`. Parent directories are created automatically.

When the inner dataset's `prefer_parallel()` returns `true` and the pipeline is running inside a rayon thread pool (e.g. via `ParallelRunner`), partition saves are distributed across threads. This is the default behavior for [`LazyDataset`](./lazy.md) wrappers.

```rust,ignore
Node {
    name: "split_by_month",
    func: |df: DataFrame| -> (HashMap<String, DataFrame>,) {
        // split DataFrame into partitions...
    },
    input: (&cat.all_data,),
    output: (&cat.monthly,),  // PartitionedDataset<PolarsCsvDataset>
}
```

## `FileDataset` requirement

The inner dataset type must implement `FileDataset`:

```rust,ignore
pub trait FileDataset: Dataset + Clone {
    fn path(&self) -> &str;
    fn set_path(&mut self, path: &str);
    fn prefer_parallel(&self) -> bool { false }
    fn ensure_parent_dir(&self) -> Result<(), std::io::Error> { ... }
    fn list_entries(&self, path: &str, ext: &str) -> Result<Vec<String>, PondError> { ... }
}
```

`list_entries` scans the directory for files matching `ext` and returns their stems, sorted. You can override it for non-filesystem storage.

Built-in types that implement `FileDataset`: `PolarsCsvDataset`, `PolarsParquetDataset`, `TextDataset`, `JsonDataset`, `ImageDataset`, `LazyDataset<D>` (for any `D: FileDataset`).

## YAML configuration

```yaml
monthly:
  path: data/partitions
  ext: csv
  dataset:
    separator: ","
    has_header: true
```

The `dataset` field configures the template dataset that is cloned for each partition file.

## Lazy partitions

For deferred, parallel partition processing, wrap the inner dataset in `LazyDataset`. See [Lazy Dataset](./lazy.md) for details on `LazyPartitionedDataset` and `PartitionedNode`.
