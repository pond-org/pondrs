# Memory Dataset

`MemoryDataset<T>` is a thread-safe in-memory dataset for intermediate pipeline values.

*Requires the `std` feature.*

## Definition

```rust,ignore
#[derive(Debug, Serialize, Deserialize)]
pub struct MemoryDataset<T: Clone> {
    #[serde(skip)]
    value: Arc<Mutex<Option<T>>>,
}
```

- Starts empty (`None`)
- Loading before any save returns `PondError::DatasetNotLoaded`
- Thread-safe via `Arc<Mutex<_>>` — works with both `SequentialRunner` and `ParallelRunner`

## Usage

```rust,ignore
#[derive(Serialize, Deserialize)]
struct Catalog {
    intermediate: MemoryDataset<f64>,
}
```

```yaml
intermediate: {}
```

`MemoryDataset` has no persistent configuration — it always starts empty. In YAML, use an empty mapping `{}`.

## When to use

Use `MemoryDataset` for intermediate values that are computed by one node and consumed by another, without needing to persist to disk:

```rust,ignore
(
    Node {
        name: "compute",
        func: |input: DataFrame| {
            let mean = input.column("value").unwrap().mean().unwrap();
            (mean,)
        },
        input: (&cat.raw_data,),
        output: (&cat.mean_value,),  // MemoryDataset<f64>
    },
    Node {
        name: "use_result",
        func: |mean: f64| (format!("Mean: {mean}"),),
        input: (&cat.mean_value,),
        output: (&cat.report,),
    },
)
```

## Parallel safety

`MemoryDataset` is safe for use with the `ParallelRunner`. The `Mutex` ensures that concurrent reads and writes are properly synchronized. However, the parallel runner's dependency analysis ensures that a node won't try to read a `MemoryDataset` until the node that writes to it has completed.

## no_std alternative

In `no_std` environments, use [`CellDataset`](./cell.md) instead. It uses `Cell` instead of `Arc<Mutex<_>>`, but is limited to `Copy` types and single-threaded use.
