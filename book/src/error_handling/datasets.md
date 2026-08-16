# Dataset Errors

Each dataset declares its own `Error` associated type. The framework converts these errors directly into the pipeline's error type `E`.

## The conversion chain

When a node loads or saves a dataset, errors flow straight through:

```text
Dataset::Error → E (pipeline error type)
```

There is no intermediate hop through `PondError`. The conversion is required by the `NodeInput<E>` and `NodeOutput<E>` trait bounds, which demand `E: From<D::Error>` for **each** dataset in a node's input or output tuple:

```rust,ignore
impl<E, T0, T1> NodeInput<E> for (T0, T1)
where
    E: From<T0::Error> + From<T1::Error> + From<PondError>,
{ /* ... */ }
```

`E: From<PondError>` stays as the floor. It is not there for dataset errors — it covers framework failures that belong to no dataset: hook aborts, and adapter errors such as the `PondError::KeyMismatch` that [`EachField`](../pipelines/split_join.md) raises when a `HashMap`'s keys do not match its catalog. It costs nothing, because the runner already requires `E: From<PondError>` of every pipeline.

## Built-in dataset errors

All built-in datasets use `PondError` as their error type, so a pipeline that only uses them needs nothing beyond `E: From<PondError>`:

| Dataset | Error type |
|---------|-----------|
| `Param<T>` | `PondError` (never returned) |
| `CellDataset<T>` | `PondError` |
| `MemoryDataset<T>` | `PondError` |
| `PolarsCsvDataset` | `PondError` |
| `JsonDataset` | `PondError` |
| `TextDataset` | `PondError` |
| `YamlDataset` | `PondError` |
| `CacheDataset<D>` | `PondError` |
| `LazyDataset<D>` | `D::Error` (delegates to the inner dataset) |

## Custom datasets with their own error type

This is the recommended path when your dataset has failure modes worth distinguishing. Keep your own error type on the dataset:

```rust,ignore
#[derive(Debug, thiserror::Error)]
pub enum GpsError {
    #[error("no fix ({satellites} satellites)")]
    NoFix { satellites: u8 },
    #[error("garbled sentence")]
    Garbled,
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

impl Dataset for GpsDataset {
    type LoadItem = Fix;
    type SaveItem = Fix;
    type Error = GpsError;
    // ...
}
```

Then give your pipeline error type a variant for it, alongside the `PondError` variant:

```rust,ignore
#[derive(Debug, thiserror::Error)]
enum AppError {
    #[error(transparent)]
    Pond(#[from] PondError),
    #[error(transparent)]
    Gps(#[from] GpsError),
}

fn pipeline<'a>(cat: &'a Catalog, params: &'a Params) -> impl Steps<AppError> + 'a {
    (/* nodes reading and writing GpsDataset */)
}
```

`GpsError` now arrives at your error handling as a `GpsError` — you can match on `AppError::Gps(GpsError::NoFix { satellites })` and recover, and the `source()` chain is intact. **No `PondError: From<GpsError>` impl is needed**, and none should be written.

A single node may freely mix datasets with different error types:

```rust,ignore
Node {
    name: "scale",
    input: (&cat.gps, &params.factor),   // GpsError and PondError
    output: (&cat.scaled,),
    func: |fix: Fix, factor: f64| (fix.scale(factor),),
}
```

`AppError` absorbs both, because the tuple impl requires `From` of each in turn.

## Custom datasets with `PondError`

If your dataset has no interesting failure modes of its own, using `PondError` directly is simpler and needs no variant in your error enum:

```rust,ignore
impl Dataset for MyDataset {
    type LoadItem = MyData;
    type SaveItem = MyData;
    type Error = PondError;

    fn load(&self) -> Result<MyData, PondError> {
        let bytes = std::fs::read(&self.path)?;      // Io variant via From
        parse_my_format(&bytes).map_err(PondError::other)
    }

    fn save(&self, data: MyData) -> Result<(), PondError> {
        let bytes = serialize_my_format(&data).map_err(PondError::other)?;
        std::fs::write(&self.path, bytes)?;
        Ok(())
    }
}
```

`PondError::other(e)` boxes a foreign error into the `Other` variant, preserving `Display`, the `source()` chain, and `downcast_ref`. Prefer it to `PondError::Custom(e.to_string())`, which discards all three.

## Avoid `Infallible`

A dataset that cannot fail should still declare a real error type. `core::convert::Infallible` would force every pipeline error type in reach of that dataset to implement `From<Infallible>`, and there is no way to blanket around it — any `impl<E> ... for E` overlaps the `E: From<X>` blanket and coherence rejects it.

This is why `Param<T>`, whose `load()` genuinely never fails, declares `type Error = PondError` and simply never returns `Err`.

## Where errors surface

The dataset-error bound is checked where the pipeline error type `E` is named — the pipeline function's return type, or a `Step<E>` annotation — not at the `Node { .. }` literal. A `Node` literal type-checks its closure's arguments and return type without knowing `E`. This mirrors how node function errors already behave; see [Compile errors](../pipelines/errors.md).
