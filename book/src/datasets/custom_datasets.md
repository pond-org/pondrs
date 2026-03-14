# Custom Datasets

You can implement the `Dataset` trait for any type to integrate custom data sources into your pipeline.

## The `Dataset` trait

```rust,ignore
pub trait Dataset: serde::Serialize {
    type LoadItem;
    type SaveItem;
    type Error;

    fn load(&self) -> Result<Self::LoadItem, Self::Error>;
    fn save(&self, output: Self::SaveItem) -> Result<(), Self::Error>;
    fn is_param(&self) -> bool { false }

    #[cfg(feature = "std")]
    fn html(&self) -> Option<String> { None }
}
```

## Example: a plain text dataset

```rust,ignore
#[derive(Serialize, Deserialize, Clone)]
pub struct TextDataset {
    path: String,
}

impl Dataset for TextDataset {
    type LoadItem = String;
    type SaveItem = String;
    type Error = PondError;

    fn load(&self) -> Result<String, PondError> {
        Ok(std::fs::read_to_string(&self.path)?)
    }

    fn save(&self, text: String) -> Result<(), PondError> {
        std::fs::write(&self.path, text)?;
        Ok(())
    }
}
```

## Checklist

1. **Derive `Serialize`** (required by the supertrait) — and usually `Deserialize` too, so the catalog can be loaded from YAML.

2. **Name the type with a `Dataset` suffix** — the catalog indexer uses serde struct names ending in `"Dataset"` to identify leaf datasets. Without this suffix, nested struct fields may not be discovered correctly.

3. **Choose your error type** — use `PondError` for simplicity, or a custom error type if you want to preserve error detail (see [Dataset Errors](../error_handling/datasets.md)).

4. **Implement `html()`** (optional, `std` only) — return an HTML snippet for the viz dashboard. This is shown in the dataset detail panel.

## The `FileDataset` trait

If your dataset is backed by a file, implement `FileDataset` to enable use with `PartitionedDataset`:

```rust,ignore
pub trait FileDataset: Dataset + Clone {
    fn path(&self) -> &str;
    fn set_path(&mut self, path: &str);
}
```

This lets `PartitionedDataset` clone your dataset template and point each partition at a different file.

