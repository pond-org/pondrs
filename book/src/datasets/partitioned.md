# Partitioned Dataset

`PartitionedDataset` and `LazyPartitionedDataset` represent a directory of files where each file is treated as a separate partition.

*Requires the `polars` feature.*

## `PartitionedDataset`

Eagerly loads all files in a directory into a `HashMap<String, D::LoadItem>`:

```rust,ignore
pub struct PartitionedDataset<D: FileDataset> {
    pub path: String,
    pub ext: String,
    pub dataset: D,
}
```

- **`path`** — the directory to read from / write to
- **`ext`** — file extension to filter by (e.g. `"csv"`, `"parquet"`)
- **`dataset`** — a template dataset that is cloned and pointed at each file

### Loading

Returns `HashMap<String, D::LoadItem>` where keys are filename stems:

```text
data/partitions/
  january.csv
  february.csv
  march.csv
```

```rust,ignore
// loads as HashMap { "january" => DataFrame, "february" => DataFrame, "march" => DataFrame }
```

### Saving

Accepts `HashMap<String, D::SaveItem>` and writes each entry as `{name}.{ext}`:

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

## `LazyPartitionedDataset`

Same as `PartitionedDataset` but returns `HashMap<String, Lazy<D::LoadItem>>` — each partition is loaded on demand:

```rust,ignore
Node {
    name: "process",
    func: |partitions: HashMap<String, Lazy<DataFrame>>| {
        // only load the partitions you need
        let jan = partitions["january"].load().unwrap();
        // ...
    },
    input: (&cat.monthly,),
    output: (&cat.result,),
}
```

`Lazy<T>` wraps a closure that calls `dataset.load()` when `.load()` is called on it.

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

## `FileDataset` requirement

The inner dataset type must implement `FileDataset`:

```rust,ignore
pub trait FileDataset: Dataset + Clone {
    fn path(&self) -> &str;
    fn set_path(&mut self, path: &str);
}
```

Built-in types that implement `FileDataset`: `PolarsCsvDataset`, `PolarsParquetDataset`, `TextDataset`, `JsonDataset`.
