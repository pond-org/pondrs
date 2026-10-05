# Cache Dataset

`CacheDataset<D>` wraps any dataset and caches the loaded/saved value in memory. Subsequent loads return the cached value without hitting the underlying dataset.

*Requires the `std` feature.*

## Definition

```rust,ignore
pub struct CacheDataset<D: Dataset> {
    pub dataset: D,
    #[serde(default)]
    pub take: bool,
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
- **`remover()`** — clears the cache and removes the inner dataset's value (used by [`RetentionHook`](../hooks/builtin.md#retentionhook))

## Taking the value

With `take: true` (or `CacheDataset::new(ds).with_take(true)`), the first `load()` after a `save()` moves the value out of memory instead of cloning it, and later loads read the inner dataset again:

```yaml
checkpoint:
  dataset:
    path: ckpt/epoch_3.mpk
  take: true
```

This suits a value with a single consumer, such as one checkpoint in an epoch chain: the next epoch reads the weights straight from memory, skipping deserialization, and memory never holds more than the checkpoints not yet consumed. Without `take`, every epoch's `CacheDataset` would keep its weights for the rest of the run. A second reader still works — it loads from disk.

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

- `D::LoadItem: Clone + Send` — so the cached value can be cloned on each load
- `D::SaveItem: Clone + Into<D::LoadItem>` — so saves can update the cache
- `PondError: From<D::Error>` — for error conversion
