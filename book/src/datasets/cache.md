# Cache Dataset

`CacheDataset<D>` wraps any dataset and caches the loaded/saved value in memory. Subsequent loads return the cached value without hitting the underlying dataset.

*Requires the `std` feature.*

## Definition

```rust,ignore
pub struct CacheDataset<D: Dataset> {
    pub dataset: D,
    cache: Arc<Mutex<Option<D::LoadItem>>>,
}
```

## Usage

Wrap any dataset to add caching:

```rust,ignore
#[derive(Serialize, Deserialize)]
struct Catalog {
    readings: CacheDataset<PolarsCsvDataset>,
}
```

```yaml
readings:
  dataset:
    path: data/readings.csv
    separator: ","
```

## Behavior

- **First `load()`** — delegates to the inner dataset, caches the result, returns it
- **Subsequent `load()` calls** — returns the cached value without re-reading the file
- **`save()`** — writes to the inner dataset **and** updates the cache
- **`html()`** — delegates to the inner dataset

## When to use

Use `CacheDataset` when a dataset is read by multiple nodes and the underlying I/O is expensive:

```rust,ignore
(
    Node { name: "analyze", input: (&cat.readings,), .. },
    Node { name: "validate", input: (&cat.readings,), .. },
    Node { name: "summarize", input: (&cat.readings,), .. },
)
```

Without caching, `readings` would be loaded from disk three times. With `CacheDataset<PolarsCsvDataset>`, it's loaded once and served from memory for the remaining reads.

## Constraints

The inner dataset must satisfy:

- `D::LoadItem: Clone` — so the cached value can be cloned on each load
- `D::SaveItem: Clone + Into<D::LoadItem>` — so saves can update the cache
- `PondError: From<D::Error>` — for error conversion
