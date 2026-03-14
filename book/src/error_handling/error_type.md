# Error Type

## `PondError`

`PondError` is the framework-level error type. It covers infrastructure failures like I/O errors, serialization errors, and dataset-not-loaded conditions:

```rust,ignore
pub enum PondError {
    #[cfg(feature = "std")]    Io(std::io::Error),
    #[cfg(feature = "polars")] Polars(polars::error::PolarsError),
    #[cfg(feature = "yaml")]   YamlScan(yaml_rust2::ScanError),
    #[cfg(feature = "yaml")]   YamlEmit(yaml_rust2::EmitError),
    #[cfg(feature = "std")]    SerdeYaml(serde_yaml::Error),
    #[cfg(any(feature = "json", feature = "plotly", feature = "viz"))]
                               Json(serde_json::Error),
    #[cfg(feature = "image")]  Image(image::ImageError),

    DatasetNotLoaded,          // always available (no_std)
    RunnerNotFound,
    CheckFailed,
    #[cfg(feature = "std")]    LockPoisoned(String),
    #[cfg(feature = "std")]    Custom(String),
}
```

Variants are feature-gated — only `DatasetNotLoaded`, `RunnerNotFound`, and `CheckFailed` are available in `no_std` builds.

## Using `PondError` directly

For simple pipelines, you can use `PondError` as your pipeline error type:

```rust,ignore
fn pipeline<'a>(cat: &'a Catalog, params: &'a Params) -> impl Steps<PondError> + 'a {
    (/* nodes */)
}
```

This works because `PondError` trivially satisfies `From<PondError>`.

## Custom error types

When you need domain-specific error variants, define your own error enum with a `From<PondError>` conversion:

```rust,ignore
#[derive(Debug, thiserror::Error)]
enum MyError {
    #[error(transparent)]
    Pond(#[from] PondError),

    #[error("validation failed: {0}")]
    Validation(String),

    #[error("threshold exceeded: {value} > {max}")]
    ThresholdExceeded { value: f64, max: f64 },
}
```

The `#[from]` attribute on the `PondError` variant provides the required `From<PondError>` implementation. Your pipeline function then uses `MyError` as its error type:

```rust,ignore
fn pipeline<'a>(cat: &'a Catalog, params: &'a Params) -> impl Steps<MyError> + 'a {
    (/* nodes that can return MyError */)
}
```

## The `From<PondError>` requirement

The pipeline error type `E` must satisfy `E: From<PondError>`. This is how the framework converts dataset I/O errors and infrastructure failures into your pipeline's error type. Without this conversion, dataset `load()` and `save()` calls couldn't propagate errors through your nodes.

## Adding variants for custom datasets

If you implement a custom dataset whose `Error` type is not already covered by `PondError`, you have two choices:

1. **Use `PondError::Custom`** (simplest): convert your error to a string.

   ```rust,ignore
   impl Dataset for MyDataset {
       type Error = PondError;
       fn load(&self) -> Result<Self::LoadItem, PondError> {
           do_something().map_err(|e| PondError::Custom(e.to_string()))
       }
   }
   ```

2. **Add a variant to your pipeline error type**: keep the original error type in your custom dataset and convert it in your pipeline error enum.

   ```rust,ignore
   impl Dataset for MyDataset {
       type Error = MyDatasetError;
       // ...
   }

   #[derive(Debug, thiserror::Error)]
   enum MyError {
       #[error(transparent)]
       Pond(#[from] PondError),
       #[error(transparent)]
       MyDataset(#[from] MyDatasetError),
   }
   ```

   This requires `PondError: From<MyDatasetError>` **or** using your custom pipeline error type `E` where `E: From<PondError> + From<MyDatasetError>`. See [Dataset Errors](./datasets.md) for the full pattern.
