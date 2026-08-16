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
    HookAbort(&'static str),
    RunnerNotFound,
    CheckFailed,
    Message(&'static str),
    #[cfg(feature = "std")]    LockPoisoned(String),
    #[cfg(feature = "std")]    Custom(String),
    #[cfg(feature = "std")]    Other(Box<dyn core::error::Error + Send + Sync>),
    // ...
}
```

Variants are feature-gated — only `DatasetNotLoaded`, `HookAbort`, `RunnerNotFound`, `CheckFailed`, and `Message` are available in `no_std` builds. The enum is `#[non_exhaustive]`, so match on it with a `_` arm.

`Custom(String)` flattens an error to its message. Prefer `PondError::other(e)`, which stores the error in the `Other` variant and keeps `Display`, the `source()` chain, and `downcast_ref` intact:

```rust,ignore
parse_my_format(&bytes).map_err(PondError::other)?;
```

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

The pipeline error type `E` must satisfy `E: From<PondError>`. This is the framework floor: it covers infrastructure failures that belong to no particular dataset — hook aborts, `RunnerNotFound`, `CheckFailed`, and adapter errors such as `EachField`'s `KeyMismatch` — as well as every built-in dataset, whose `Error` is already `PondError`.

It is *not* how custom dataset errors reach you. Those convert into `E` directly; see below.

## Adding variants for custom datasets

A dataset's `Error` converts straight into your pipeline error type. Give your enum an `#[error(transparent)] #[from]` variant per dataset error type, alongside the `PondError` variant:

```rust,ignore
impl Dataset for GpsDataset {
    type Error = GpsError;
    // ...
}

#[derive(Debug, thiserror::Error)]
enum MyError {
    #[error(transparent)]
    Pond(#[from] PondError),
    #[error(transparent)]
    Gps(#[from] GpsError),
}
```

That is all. **No `PondError: From<GpsError>` impl is required**, and you should not write one — `GpsError` reaches `MyError::Gps` with its type and `source()` chain intact, so you can match on it and recover.

The bound comes from the node input/output tuple impls, which require `E: From<D::Error>` for each dataset `D` in the tuple. A node may mix datasets with different error types freely. See [Dataset Errors](./datasets.md) for the full pattern.

If your dataset has no failure modes worth distinguishing, giving it `type Error = PondError` is simpler and needs no variant at all.
