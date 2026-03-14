# Dataset Errors

Each dataset declares its own `Error` associated type. The framework needs to convert these errors into the pipeline's error type `E` via `PondError`.

## The conversion chain

When a node loads or saves a dataset, errors flow through this chain:

```text
Dataset::Error → PondError → E (pipeline error type)
```

The first conversion (`Dataset::Error → PondError`) is required by the `NodeInput` and `NodeOutput` trait bounds: `PondError: From<D::Error>`. The second conversion (`PondError → E`) is required by the runner: `E: From<PondError>`.

## Built-in dataset errors

All built-in datasets use error types that already have `From` implementations into `PondError`:

| Dataset | Error type | PondError variant |
|---------|-----------|-------------------|
| `Param<T>` | `Infallible` | (never fails) |
| `CellDataset<T>` | `PondError` | direct |
| `MemoryDataset<T>` | `PondError` | direct |
| `PolarsCsvDataset` | `PondError` | `Polars`, `Io` |
| `JsonDataset` | `PondError` | `Json`, `Io` |
| `TextDataset` | `PondError` | `Io` |
| `YamlDataset` | `PondError` | `YamlScan`, `Io` |
| `CacheDataset<D>` | `PondError` | wraps inner |

## Custom datasets with `PondError`

The simplest approach for custom datasets is to use `PondError` directly as the error type:

```rust,ignore
impl Dataset for MyDataset {
    type LoadItem = MyData;
    type SaveItem = MyData;
    type Error = PondError;

    fn load(&self) -> Result<MyData, PondError> {
        let bytes = std::fs::read(&self.path)?;  // Io variant via From
        parse_my_format(&bytes)
            .map_err(|e| PondError::Custom(e.to_string()))
    }

    fn save(&self, data: MyData) -> Result<(), PondError> {
        let bytes = serialize_my_format(&data)
            .map_err(|e| PondError::Custom(e.to_string()))?;
        std::fs::write(&self.path, bytes)?;
        Ok(())
    }
}
```

## Custom datasets with their own error type

If you want to preserve error type information, your dataset can use its own error type. You then need `PondError: From<YourError>`:

```rust,ignore
#[derive(Debug, thiserror::Error)]
pub enum MyDatasetError {
    #[error("parse error at line {line}: {msg}")]
    Parse { line: usize, msg: String },
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

impl Dataset for MyDataset {
    type LoadItem = MyData;
    type SaveItem = MyData;
    type Error = MyDatasetError;
    // ...
}
```

Since `PondError` doesn't have a variant for every possible custom error, you typically handle this through your **pipeline error type**:

```rust,ignore
#[derive(Debug, thiserror::Error)]
enum PipelineError {
    #[error(transparent)]
    Pond(#[from] PondError),
    #[error(transparent)]
    MyDataset(#[from] MyDatasetError),
}
```

However, the `NodeInput`/`NodeOutput` bounds require `PondError: From<MyDatasetError>`. If this conversion doesn't exist, the dataset won't compile as a node input/output. The easiest solution is to use `PondError` as your dataset's error type and use `PondError::Custom` for the domain-specific cases, or to implement the `From` conversion yourself.

## Infallible datasets

Datasets that never fail (like `Param`) use `core::convert::Infallible`:

```rust,ignore
impl Dataset for Param<T> {
    type Error = Infallible;
    fn load(&self) -> Result<T, Infallible> { Ok(self.0.clone()) }
}
```

`PondError` implements `From<Infallible>` (via the `match x {}` pattern), so this works with any pipeline error type.
